/*
 * ergon-io's DPDK port shim for the smoltcp-spin backend
 * (src/netstack/smoltcp/port.rs declares these functions).
 *
 * DPDK's burst and mbuf calls are static inline in its headers, so Rust
 * reaches them through this file. One port, one RX and one TX queue (queue 0),
 * one mbuf pool, driven by the thread that owns the smoltcp stack.
 *
 * Hot path: eio_rx_burst, eio_tx_alloc, eio_tx_offload, eio_tx_burst,
 * eio_mbuf_free. Set-up: eio_eal_init, eio_port_start, eio_thread_attach.
 * Housekeeping: eio_timer_service (the PMDs' timers: ENA's watchdog),
 * eio_port_stats.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <rte_cycles.h>
#include <rte_eal.h>
#include <rte_errno.h>
#include <rte_ether.h>
#include <rte_ethdev.h>
#include <rte_ip.h>
#include <rte_lcore.h>
#include <rte_mbuf.h>
#include <rte_mempool.h>
#include <rte_tcp.h>
#include <rte_timer.h>
#include <rte_udp.h>
#include <rte_version.h>

/* The last set-up error, for eio_last_error. Set-up runs on one thread at a
 * time (the Rust side holds a process-wide lock around it). */
static char eio_err[512];

static void eio_set_error(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(eio_err, sizeof(eio_err), fmt, ap);
    va_end(ap);
}

const char *eio_last_error(void)
{
    return eio_err;
}

static int eio_eal_up = 0;

/* rte_eal_init once per process. Returns 0 when it ran now, 1 when it had
 * already run, -1 on failure (eio_last_error). The calling thread's CPU
 * affinity is restored afterwards: rte_eal_init pins it to the main lcore. */
int eio_eal_init(int argc, char **argv)
{
    if (eio_eal_up)
    {
        return 1;
    }
    cpu_set_t caller;
    int have = 0 == pthread_getaffinity_np(pthread_self(), sizeof(caller), &caller);
    int rc = rte_eal_init(argc, argv);
    if (have)
    {
        (void)pthread_setaffinity_np(pthread_self(), sizeof(caller), &caller);
    }
    if (rc < 0)
    {
        eio_set_error("rte_eal_init failed (DPDK %s): %s", rte_version(), rte_strerror(rte_errno));
        return -1;
    }
    eio_eal_up = 1;
    return 0;
}

/* The CPUs the calling thread may run on, as an EAL --lcores value that maps
 * lcore 0 onto exactly them: "0@(1,3,5)". So rte_eal_init starts no worker
 * lcore threads and leaves the thread where its caller put it. */
int eio_lcores_arg(char *out, size_t len)
{
    cpu_set_t set;
    if (0 != pthread_getaffinity_np(pthread_self(), sizeof(set), &set))
    {
        return -1;
    }
    size_t n = (size_t)snprintf(out, len, "0@(");
    int first = 1;
    for (int cpu = 0; cpu < CPU_SETSIZE && n < len; cpu++)
    {
        if (CPU_ISSET(cpu, &set))
        {
            n += (size_t)snprintf(out + n, len - n, first ? "%d" : ",%d", cpu);
            first = 0;
        }
    }
    if (first || n + 2 > len)
    {
        return -1;
    }
    snprintf(out + n, len - n, ")");
    return 0;
}

/* Gives a thread that is not the EAL's main lcore an lcore id (mempool caches
 * and the timer lists are per lcore). 0 when it has one now. */
int eio_thread_attach(void)
{
    if (LCORE_ID_ANY != rte_lcore_id())
    {
        return 0;
    }
    return rte_thread_register() < 0 ? -1 : 0;
}

/* Gives back an lcore id eio_thread_attach took (never the main lcore's). */
void eio_thread_detach(void)
{
    unsigned int id = rte_lcore_id();
    if (LCORE_ID_ANY != id && rte_lcore_has_role(id, ROLE_NON_EAL))
    {
        rte_thread_unregister();
    }
}

typedef struct
{
    uint16_t port_id;
    uint8_t mac[6];
    uint16_t mtu;
    int tx_csum;
    uint32_t data_room;
    uint16_t rx_desc;
    uint16_t tx_desc;
    char driver[32];
    void *pool;
} eio_port_t;

#define EIO_MAX_PORTS 32
static eio_port_t eio_ports[EIO_MAX_PORTS];
static int eio_started[EIO_MAX_PORTS];

static int eio_find_port(const char *name, uint16_t *pid)
{
    if (0 == rte_eth_dev_get_port_by_name(name, pid))
    {
        return 0;
    }
    char *end = NULL;
    long idx = strtol(name, &end, 10);
    if (end != name && '\0' == *end && idx >= 0 && idx < RTE_MAX_ETHPORTS && rte_eth_dev_is_valid_port((uint16_t)idx))
    {
        *pid = (uint16_t)idx;
        return 0;
    }
    return -1;
}

/* Configures and starts `name` (a device name such as a PCI address or
 * net_af_packet0, or a port index) once per process: one RX and one TX
 * queue, a pool of `nb_mbufs` mbufs each holding a whole frame of `ip_mtu`,
 * and the IPv4, TCP and UDP transmit checksum offloads when `want_tx_csum`
 * and the port has all three. A second call for a started port fills `out`
 * and starts nothing. 0 on success, -1 with eio_last_error. */
int eio_port_start(const char *name, uint16_t ip_mtu, uint16_t rx_desc, uint16_t tx_desc, uint32_t nb_mbufs,
                   int want_tx_csum, uint32_t link_wait_ms, eio_port_t *out)
{
    uint16_t pid;
    if (eio_find_port(name, &pid) < 0)
    {
        eio_set_error("no DPDK port named %s (%u ports probed; pass the device to EAL: -a <pci> or --vdev=...)",
                      name, (unsigned)rte_eth_dev_count_avail());
        return -1;
    }
    if (pid >= EIO_MAX_PORTS)
    {
        eio_set_error("port %u: past the %d this shim keeps", pid, EIO_MAX_PORTS);
        return -1;
    }
    if (eio_started[pid])
    {
        *out = eio_ports[pid];
        return 0;
    }
    struct rte_eth_dev_info info;
    memset(&info, 0, sizeof(info));
    int rc = rte_eth_dev_info_get(pid, &info);
    if (rc < 0)
    {
        eio_set_error("port %s: rte_eth_dev_info_get: %s", name, rte_strerror(-rc));
        return -1;
    }
    if (ip_mtu > info.max_mtu)
    {
        eio_set_error("port %s (%s): MTU %u is over the port's maximum %u", name, info.driver_name, ip_mtu,
                      info.max_mtu);
        return -1;
    }
    const uint64_t csum = RTE_ETH_TX_OFFLOAD_IPV4_CKSUM | RTE_ETH_TX_OFFLOAD_TCP_CKSUM | RTE_ETH_TX_OFFLOAD_UDP_CKSUM;
    int tx_csum = want_tx_csum && csum == (info.tx_offload_capa & csum);

    struct rte_eth_conf conf;
    memset(&conf, 0, sizeof(conf));
    conf.rxmode.mtu = ip_mtu;
    conf.txmode.offloads = tx_csum ? csum : 0;
    rc = rte_eth_dev_configure(pid, 1, 1, &conf);
    if (rc < 0)
    {
        eio_set_error("port %s (%s): rte_eth_dev_configure: %s", name, info.driver_name, rte_strerror(-rc));
        return -1;
    }
    rc = rte_eth_dev_adjust_nb_rx_tx_desc(pid, &rx_desc, &tx_desc);
    if (rc < 0)
    {
        eio_set_error("port %s: rte_eth_dev_adjust_nb_rx_tx_desc: %s", name, rte_strerror(-rc));
        return -1;
    }
    /* A whole Ethernet frame (header, a VLAN tag's room, the IP MTU) in one
     * mbuf: the RX path drops chained mbufs. */
    uint32_t frame = (uint32_t)ip_mtu + RTE_ETHER_HDR_LEN + 4 + RTE_ETHER_CRC_LEN;
    uint32_t room = frame < RTE_MBUF_DEFAULT_DATAROOM ? RTE_MBUF_DEFAULT_DATAROOM : frame;
    int socket = rte_eth_dev_socket_id(pid);
    if (socket < 0)
    {
        socket = (int)SOCKET_ID_ANY;
    }
    char pool_name[RTE_MEMPOOL_NAMESIZE];
    snprintf(pool_name, sizeof(pool_name), "eio_pool_%u", pid);
    struct rte_mempool *pool =
        rte_pktmbuf_pool_create(pool_name, nb_mbufs, 256, 0, (uint16_t)(RTE_PKTMBUF_HEADROOM + room), socket);
    if (NULL == pool)
    {
        eio_set_error("port %s: mbuf pool of %u x %u bytes: %s", name, nb_mbufs, room, rte_strerror(rte_errno));
        return -1;
    }
    rc = rte_eth_rx_queue_setup(pid, 0, rx_desc, (unsigned int)socket, NULL, pool);
    if (rc < 0)
    {
        eio_set_error("port %s: rte_eth_rx_queue_setup: %s", name, rte_strerror(-rc));
        return -1;
    }
    rc = rte_eth_tx_queue_setup(pid, 0, tx_desc, (unsigned int)socket, NULL);
    if (rc < 0)
    {
        eio_set_error("port %s: rte_eth_tx_queue_setup: %s", name, rte_strerror(-rc));
        return -1;
    }
    rc = rte_eth_dev_start(pid);
    if (rc < 0)
    {
        eio_set_error("port %s: rte_eth_dev_start: %s", name, rte_strerror(-rc));
        return -1;
    }
    for (uint32_t waited = 0;; waited += 10)
    {
        struct rte_eth_link link;
        memset(&link, 0, sizeof(link));
        if (0 == rte_eth_link_get_nowait(pid, &link) && RTE_ETH_LINK_UP == link.link_status)
        {
            break;
        }
        if (waited >= link_wait_ms)
        {
            eio_set_error("port %s: link still down after %u ms", name, link_wait_ms);
            return -1;
        }
        rte_delay_ms(10);
    }
    struct rte_ether_addr mac;
    rc = rte_eth_macaddr_get(pid, &mac);
    if (rc < 0)
    {
        eio_set_error("port %s: rte_eth_macaddr_get: %s", name, rte_strerror(-rc));
        return -1;
    }
    eio_port_t *p = &eio_ports[pid];
    memset(p, 0, sizeof(*p));
    p->port_id = pid;
    memcpy(p->mac, mac.addr_bytes, 6);
    p->mtu = ip_mtu;
    p->tx_csum = tx_csum;
    p->data_room = room;
    p->rx_desc = rx_desc;
    p->tx_desc = tx_desc;
    snprintf(p->driver, sizeof(p->driver), "%s", NULL != info.driver_name ? info.driver_name : "");
    p->pool = pool;
    eio_started[pid] = 1;
    *out = *p;
    return 0;
}

/* One received frame: its mbuf, the frame's bytes in the first segment, and
 * how many segments the mbuf has (the Rust side drops chained ones). */
typedef struct
{
    void *mbuf;
    const uint8_t *data;
    uint32_t len;
    uint32_t segs;
} eio_rx_t;

#define EIO_RX_MAX 64

uint16_t eio_rx_burst(uint16_t pid, eio_rx_t *out, uint16_t n)
{
    struct rte_mbuf *mb[EIO_RX_MAX];
    if (n > EIO_RX_MAX)
    {
        n = EIO_RX_MAX;
    }
    uint16_t k = rte_eth_rx_burst(pid, 0, mb, n);
    for (uint16_t i = 0; i < k; i++)
    {
        out[i].mbuf = mb[i];
        out[i].data = rte_pktmbuf_mtod(mb[i], const uint8_t *);
        out[i].len = rte_pktmbuf_data_len(mb[i]);
        out[i].segs = mb[i]->nb_segs;
    }
    return k;
}

void eio_mbuf_free(void *m)
{
    rte_pktmbuf_free((struct rte_mbuf *)m);
}

/* A fresh mbuf from `pool` holding `len` bytes, its address in *mbuf; the
 * bytes' address, or NULL when the pool is empty. */
uint8_t *eio_tx_alloc(void *pool, uint16_t len, void **mbuf)
{
    struct rte_mbuf *m = rte_pktmbuf_alloc((struct rte_mempool *)pool);
    if (NULL == m)
    {
        return NULL;
    }
    char *p = rte_pktmbuf_append(m, len);
    if (NULL == p)
    {
        rte_pktmbuf_free(m);
        return NULL;
    }
    *mbuf = m;
    return (uint8_t *)p;
}

/* Hands the IPv4 header checksum and the TCP (proto 6) or UDP (17) checksum
 * of the frame in `m` to the NIC: the offload flags and lengths, a zero IP
 * checksum and the pseudo-header sum in the L4 checksum field, which is what
 * the offload expects. `l3` is the IPv4 header's length. */
void eio_tx_offload(void *mv, uint16_t l3, uint8_t proto)
{
    struct rte_mbuf *m = (struct rte_mbuf *)mv;
    struct rte_ipv4_hdr *ip = rte_pktmbuf_mtod_offset(m, struct rte_ipv4_hdr *, RTE_ETHER_HDR_LEN);
    uint64_t ol = RTE_MBUF_F_TX_IPV4 | RTE_MBUF_F_TX_IP_CKSUM;
    m->l2_len = RTE_ETHER_HDR_LEN;
    m->l3_len = l3;
    ip->hdr_checksum = 0;
    if (IPPROTO_TCP == proto)
    {
        ol |= RTE_MBUF_F_TX_TCP_CKSUM;
        struct rte_tcp_hdr *tcp = (struct rte_tcp_hdr *)((uint8_t *)ip + l3);
        tcp->cksum = rte_ipv4_phdr_cksum(ip, ol);
    }
    else if (IPPROTO_UDP == proto)
    {
        ol |= RTE_MBUF_F_TX_UDP_CKSUM;
        struct rte_udp_hdr *udp = (struct rte_udp_hdr *)((uint8_t *)ip + l3);
        udp->dgram_cksum = rte_ipv4_phdr_cksum(ip, ol);
    }
    m->ol_flags = ol;
}

/* The prefix of `mbufs` the NIC took; the rest stay the caller's. */
uint16_t eio_tx_burst(uint16_t pid, void **mbufs, uint16_t n)
{
    return rte_eth_tx_burst(pid, 0, (struct rte_mbuf **)mbufs, n);
}

/* rte_timer_alt_manage's callback: the timer's own. */
static void eio_timer_run(struct rte_timer *tim)
{
    tim->f(tim, tim->arg);
}

/* The PMDs' due timers (ENA's watchdog), on the main lcore's list (where the
 * PMD armed it, at port start) and this thread's. Never blocks. 0 when none
 * are kept (no PMD initialised the timer subsystem). */
int eio_timer_service(void)
{
    const unsigned int self = rte_lcore_id();
    if (self >= RTE_MAX_LCORE)
    {
        return -1;
    }
    unsigned int lcores[2] = {rte_get_main_lcore(), self};
    int rc = rte_timer_alt_manage(0, lcores, self == lcores[0] ? 1 : 2, eio_timer_run);
    return -EINVAL == rc ? 0 : rc;
}

/* ipackets, opackets, ierrors, oerrors, imissed, rx_nombuf. */
int eio_port_stats(uint16_t pid, uint64_t out[6])
{
    struct rte_eth_stats s;
    if (0 != rte_eth_stats_get(pid, &s))
    {
        return -1;
    }
    out[0] = s.ipackets;
    out[1] = s.opackets;
    out[2] = s.ierrors;
    out[3] = s.oerrors;
    out[4] = s.imissed;
    out[5] = s.rx_nombuf;
    return 0;
}
