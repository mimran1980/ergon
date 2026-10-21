/*
 * ergon-md-dpdk DPDK transport — real ENA port operations (plan §7.2).
 *
 * Implements the port-ops table with libdpdk: resolves a PCI BDF to a port id,
 * verifies the net_ena PMD and IPv4/UDP checksum offloads, configures a single
 * RX/TX queue pair, creates a per-port mbuf pool, applies the L3 MTU, starts
 * the device and waits for the link; runs the PMDs' timers (ENA's watchdog)
 * and counts device-reset requests for the housekeeping thread.
 *
 * This is the only translation unit (besides the EAL seam) that references
 * libdpdk; test builds replace the whole table with a fake via
 * test/rusteron_dpdk_fake_port.c, so this file must not be linked there.
 */
#include "rusteron_dpdk_internal.h"
#include "rusteron_dpdk_port_ops.h"

#include <rte_cycles.h>
#include <rte_eal.h>
#include <rte_errno.h>
#include <rte_ethdev.h>
#include <rte_ether.h>
#include <rte_lcore.h>
#include <rte_mbuf.h>
#include <rte_mbuf_dyn.h>
#include <rte_mbuf_pool_ops.h>
#include <rte_memory.h>
#include <rte_mempool.h>
#include <rte_timer.h>

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define RUSTERON_DPDK_MBUF_DESC_COUNT 2048

static int rusteron_dpdk_port_probe_port(const char *pci_bdf, const uint8_t *mac, uint16_t *port_id)
{
    uint16_t p;
    RTE_ETH_FOREACH_DEV(p)
    {
        char name[RTE_ETH_NAME_MAX_LEN];
        if (0 != rte_eth_dev_get_name_by_port(p, name))
        {
            continue;
        }
        if (NULL == mac)
        {
            if (0 == strcmp(name, pci_bdf))
            {
                *port_id = p;
                return 0;
            }
            continue;
        }
        /* MAC selection (Azure). The MANA PMD names its ports after the
         * function (`<bdf>_port<n>`), netvsc after the NIC's VMBus UUID, and a
         * netvsc port owns its MANA VF's port, which the iteration then skips.
         * Either way the port to use is the one reporting the configured MAC. */
        struct rte_ether_addr addr;
        if (0 == rte_eth_macaddr_get(p, &addr) &&
            0 == memcmp(addr.addr_bytes, mac, RUSTERON_DPDK_ETH_ADDR_LEN))
        {
            *port_id = p;
            return 0;
        }
    }
    return -1;
}

static int rusteron_dpdk_port_dev_info(
    uint16_t port_id, const char *pci_bdf,
    uint8_t mac[RUSTERON_DPDK_ETH_ADDR_LEN],
    char *driver_name, size_t driver_name_len,
    int *csum_offload_ok, int *ena_llq, int *rx_timestamp_ok, uint16_t *max_mtu,
    uint16_t desc_lim[4])
{
    /* capabilities are read by port id once probed; pci_bdf names the device
     * in sysfs, for the LLQ BAR's mapping */
    struct rte_eth_dev_info info;
    memset(&info, 0, sizeof(info));
    if (rte_eth_dev_info_get(port_id, &info) < 0)
    {
        return -1;
    }

    struct rte_ether_addr mac_addr;
    memset(&mac_addr, 0, sizeof(mac_addr));
    if (rte_eth_macaddr_get(port_id, &mac_addr) < 0)
    {
        return -1;
    }
    memcpy(mac, mac_addr.addr_bytes, RUSTERON_DPDK_ETH_ADDR_LEN);

    const char *driver = info.driver_name != NULL ? info.driver_name : "";
    snprintf(driver_name, driver_name_len, "%s", driver);

    /* IPv4 and UDP checksum insertion in the transmit path and receive
     * offloads must both be advertised (plan §7.2). */
    const uint64_t tx_need = RTE_ETH_TX_OFFLOAD_IPV4_CKSUM | RTE_ETH_TX_OFFLOAD_UDP_CKSUM;
    const uint64_t rx_need = RTE_ETH_RX_OFFLOAD_IPV4_CKSUM | RTE_ETH_RX_OFFLOAD_UDP_CKSUM;
    int tx_ok = tx_need == (info.tx_offload_capa & tx_need);
    int rx_ok = rx_need == (info.rx_offload_capa & rx_need);
    *csum_offload_ok = (tx_ok && rx_ok) ? 1 : 0;

    /* An ENA's low-latency queue placement is the PMD's own (no ethdev API or
     * xstat reports it); what the process can see is how BAR 2, the LLQ BAR,
     * is mapped: write-combined (resource2_wc), which LLQ needs, or not.
     * Mapped at probe, before this. */
    *ena_llq = 0 == strcmp(driver, "net_ena") ? rusteron_dpdk_ena_llq_mapping("/proc/self/maps", pci_bdf)
                                              : RUSTERON_DPDK_ENA_LLQ_NA;

    /* ENA delivers the Nitro PHC's RX timestamps from DPDK 25.11, in a
     * precision-time placement group on supporting instances. */
    *rx_timestamp_ok = 0 != (info.rx_offload_capa & RTE_ETH_RX_OFFLOAD_TIMESTAMP);
    *max_mtu = info.max_mtu;
    desc_lim[0] = info.rx_desc_lim.nb_min;
    desc_lim[1] = info.rx_desc_lim.nb_max;
    desc_lim[2] = info.tx_desc_lim.nb_min;
    desc_lim[3] = info.tx_desc_lim.nb_max;

    return 0;
}

/* The mbuf RX timestamp dynamic field and flag, registered process-wide the
 * first time a port enables the offload; -1 / 0 until then. */
static int rusteron_dpdk_port_ts_offset = -1;
static uint64_t rusteron_dpdk_port_ts_flag = 0;

static int rusteron_dpdk_port_dev_configure(
    uint16_t port_id, uint16_t rx_rings, uint16_t tx_rings,
    uint64_t rx_offloads, uint64_t tx_offloads)
{
    struct rte_eth_conf conf;
    memset(&conf, 0, sizeof(conf));

    if (0 != (rx_offloads & RUSTERON_DPDK_RX_OFFLOAD_IPV4_CKSUM))
    {
        conf.rxmode.offloads |= RTE_ETH_RX_OFFLOAD_IPV4_CKSUM;
    }
    if (0 != (rx_offloads & RUSTERON_DPDK_RX_OFFLOAD_UDP_CKSUM))
    {
        conf.rxmode.offloads |= RTE_ETH_RX_OFFLOAD_UDP_CKSUM;
    }
    if (0 != (rx_offloads & RUSTERON_DPDK_RX_OFFLOAD_TIMESTAMP))
    {
        conf.rxmode.offloads |= RTE_ETH_RX_OFFLOAD_TIMESTAMP;
    }
    if (0 != (tx_offloads & RUSTERON_DPDK_TX_OFFLOAD_IPV4_CKSUM))
    {
        conf.txmode.offloads |= RTE_ETH_TX_OFFLOAD_IPV4_CKSUM;
    }
    if (0 != (tx_offloads & RUSTERON_DPDK_TX_OFFLOAD_UDP_CKSUM))
    {
        conf.txmode.offloads |= RTE_ETH_TX_OFFLOAD_UDP_CKSUM;
    }

    int rc = rte_eth_dev_configure(port_id, rx_rings, tx_rings, &conf);
    if (0 == rc && 0 != (rx_offloads & RUSTERON_DPDK_RX_OFFLOAD_TIMESTAMP) && rusteron_dpdk_port_ts_offset < 0)
    {
        if (rte_mbuf_dyn_rx_timestamp_register(&rusteron_dpdk_port_ts_offset, &rusteron_dpdk_port_ts_flag) < 0)
        {
            char detail[256];
            snprintf(detail, sizeof(detail), "port %u: cannot register the mbuf RX timestamp field: %s",
                     port_id, rte_strerror(rte_errno));
            rusteron_dpdk_set_error(detail);
            rusteron_dpdk_port_ts_offset = -1;
            return -1;
        }
    }
    return rc;
}

/* `cache_size` as configured: 0 is no per-core cache, a valid choice. */
static void *rusteron_dpdk_port_mempool_create(const char *name, uint32_t n, uint16_t cache_size, uint16_t data_room)
{
    void *pool = rte_pktmbuf_pool_create(
        name, n, cache_size, 0, (uint16_t)(RTE_PKTMBUF_HEADROOM + data_room), SOCKET_ID_ANY);
    if (NULL == pool)
    {
        /* Surface the underlying rte error (e.g. ENOMEM on hugepage
         * exhaustion) — runtime.c only records a generic fallback if no
         * error was already set here. What it asked for says how many
         * hugepages it needs: jumbo frames make each mbuf about 9 KiB. */
        char detail[512];
        snprintf(detail, sizeof(detail), "mempool %s (%u mbufs of %u bytes of data room): %s", name, n,
                 (unsigned)data_room, rte_strerror(rte_errno));
        rusteron_dpdk_set_error_code(detail, rte_errno);
    }
    return pool;
}

/* What rte_pktmbuf_init gives each mbuf after the headroom: the pool's data
 * room less RTE_PKTMBUF_HEADROOM, the tailroom of a fresh one. */
static uint32_t rusteron_dpdk_port_mbuf_room(void *mempool)
{
    const uint16_t room = rte_pktmbuf_data_room_size(mempool);
    return room > RTE_PKTMBUF_HEADROOM ? (uint32_t)(room - RTE_PKTMBUF_HEADROOM) : 0;
}

static int rusteron_dpdk_port_rx_queue_setup(
    uint16_t port_id, uint16_t queue_id, uint16_t nb_desc, void *mempool)
{
    return rte_eth_rx_queue_setup(port_id, queue_id, nb_desc, 0, NULL, mempool);
}

static int rusteron_dpdk_port_tx_queue_setup(
    uint16_t port_id, uint16_t queue_id, uint16_t nb_desc)
{
    return rte_eth_tx_queue_setup(port_id, queue_id, nb_desc, 0, NULL);
}

static int rusteron_dpdk_port_dev_set_mtu(uint16_t port_id, uint16_t mtu)
{
    return rte_eth_dev_set_mtu(port_id, mtu);
}

static int rusteron_dpdk_port_dev_start(uint16_t port_id)
{
    return rte_eth_dev_start(port_id);
}

static int rusteron_dpdk_port_link_wait_ready(uint16_t port_id, uint32_t timeout_ms)
{
    uint32_t waited = 0;
    const uint32_t step_ms = 10;
    for (;;)
    {
        struct rte_eth_link link;
        memset(&link, 0, sizeof(link));
        rte_eth_link_get(port_id, &link);
        if (RTE_ETH_LINK_UP == link.link_status)
        {
            return 0;
        }
        if (waited >= timeout_ms)
        {
            return -1;
        }
        rte_delay_ms(step_ms);
        waited += step_ms;
    }
}

static int rusteron_dpdk_port_dev_stop(uint16_t port_id)
{
    rte_eth_dev_stop(port_id);
    return 0;
}

static int rusteron_dpdk_port_dev_close(uint16_t port_id)
{
    rte_eth_dev_close(port_id);
    return 0;
}

static void rusteron_dpdk_port_mempool_free(void *mempool)
{
    rte_mempool_free(mempool);
}

/* Data path (plan §7.4). */

static int rusteron_dpdk_port_mbuf_alloc(void *mempool, rusteron_dpdk_mbuf_t *m)
{
    struct rte_mbuf *mbuf = rte_pktmbuf_alloc(mempool);
    if (NULL == mbuf)
    {
        return -1;
    }

    m->opaque = mbuf;
    m->data = rte_pktmbuf_mtod(mbuf, uint8_t *);
    m->capacity = (uint32_t)rte_pktmbuf_tailroom(mbuf);
    m->frame_len = 0;
    m->ol_flags = 0;
    m->l2_len = 0;
    m->l3_len = 0;
    m->l4_len = 0;
    m->udp_pseudo_csum = 0;
    return 0;
}

static void rusteron_dpdk_port_mbuf_release(rusteron_dpdk_mbuf_t *m)
{
    if (NULL != m && NULL != m->opaque)
    {
        rte_pktmbuf_free(m->opaque);
        m->opaque = NULL;
    }
}

/* Map the transport-level offload flags onto the DPDK TX offload bits. */
static uint64_t rusteron_dpdk_port_ol_flags(const rusteron_dpdk_mbuf_t *m)
{
    uint64_t flags = 0;
    if (0 != (m->ol_flags & RUSTERON_DPDK_MBUF_F_TX_IPV4))
    {
        flags |= RTE_MBUF_F_TX_IPV4;
    }
    if (0 != (m->ol_flags & RUSTERON_DPDK_MBUF_F_TX_IP_CKSUM))
    {
        flags |= RTE_MBUF_F_TX_IP_CKSUM;
    }
    if (0 != (m->ol_flags & RUSTERON_DPDK_MBUF_F_TX_UDP_CKSUM))
    {
        flags |= RTE_MBUF_F_TX_UDP_CKSUM;
    }
    return flags;
}

static uint16_t rusteron_dpdk_port_tx_burst(
    uint16_t port_id, uint16_t tx_queue_id,
    rusteron_dpdk_mbuf_t **pkts, uint16_t nb)
{
    struct rte_mbuf *mbufs[256];
    uint16_t i;

    for (i = 0; i < nb; i++)
    {
        struct rte_mbuf *mbuf = pkts[i]->opaque;
        /* pkt_len/data_len setters were removed from DPDK 23.11; the fields are
         * public and assigned directly. */
        mbuf->pkt_len = pkts[i]->frame_len;
        mbuf->data_len = (uint16_t)pkts[i]->frame_len;
        mbuf->ol_flags = rusteron_dpdk_port_ol_flags(pkts[i]);
        mbuf->l2_len = pkts[i]->l2_len;
        mbuf->l3_len = pkts[i]->l3_len;
        mbuf->l4_len = pkts[i]->l4_len;
        /* The UDP pseudo-header checksum seed already lives in the packet's UDP
         * checksum field (packet.c writes it), which is the DPDK TX-offload
         * contract; the mbuf carries only ol_flags + the L2/L3/L4 lengths. */
        mbufs[i] = mbuf;
    }

    /* The accepted prefix belongs to the NIC; the rejected tail stays the
     * caller's (rusteron_dpdk_port_tx offers it again or releases it). */
    return rte_eth_tx_burst(port_id, tx_queue_id, mbufs, nb);
}

/* The decode of those verdicts is pure and lives in rusteron_dpdk_packet.c so
 * the test archive can exercise it; only the bit masks come from DPDK. Assert
 * here that they still have the shape the decode relies on: BAD and GOOD are
 * one bit each and NONE is both. The positions themselves are free to move —
 * DPDK 23.11 put the IP pair at bits 4 and 7 and the L4 pair at 3 and 8,
 * where older releases had two adjacent pairs at 5/6 and 7/8. */
_Static_assert(
    0 == (RTE_MBUF_F_RX_IP_CKSUM_BAD & RTE_MBUF_F_RX_IP_CKSUM_GOOD) &&
        RTE_MBUF_F_RX_IP_CKSUM_NONE == (RTE_MBUF_F_RX_IP_CKSUM_BAD | RTE_MBUF_F_RX_IP_CKSUM_GOOD) &&
        0 == (RTE_MBUF_F_RX_L4_CKSUM_BAD & RTE_MBUF_F_RX_L4_CKSUM_GOOD) &&
        RTE_MBUF_F_RX_L4_CKSUM_NONE == (RTE_MBUF_F_RX_L4_CKSUM_BAD | RTE_MBUF_F_RX_L4_CKSUM_GOOD),
    "the DPDK checksum verdicts are no longer a BAD bit, a GOOD bit and NONE as both");

/* Map the DPDK RX offload verdicts onto the view flags (plan §7.6). */
static uint32_t rusteron_dpdk_port_rx_ol_flags(const struct rte_mbuf *mbuf)
{
    return rusteron_dpdk_rx_ol_flags_from_dpdk(
        mbuf->ol_flags, RTE_MBUF_F_RX_IP_CKSUM_BAD, RTE_MBUF_F_RX_IP_CKSUM_GOOD, RTE_MBUF_F_RX_IP_CKSUM_NONE,
        RTE_MBUF_F_RX_L4_CKSUM_BAD, RTE_MBUF_F_RX_L4_CKSUM_GOOD, RTE_MBUF_F_RX_L4_CKSUM_NONE);
}

static uint16_t rusteron_dpdk_port_rx_burst(
    uint16_t port_id, uint16_t rx_queue_id,
    rusteron_dpdk_mbuf_t **pkts, uint16_t nb)
{
    struct rte_mbuf *mbufs[256];
    uint16_t i;
    uint16_t received = rte_eth_rx_burst(port_id, rx_queue_id, mbufs, nb);

    for (i = 0; i < received; i++)
    {
        struct rte_mbuf *mbuf = mbufs[i];
        rusteron_dpdk_mbuf_t *m = pkts[i];
        m->opaque = mbuf;
        m->data = rte_pktmbuf_mtod(mbuf, uint8_t *);
        m->capacity = (uint32_t)rte_pktmbuf_tailroom(mbuf);
        m->frame_len = rte_pktmbuf_pkt_len(mbuf);
        m->ol_flags = 0; /* TX offload flags are not meaningful on RX */
        m->l2_len = 0;
        m->l3_len = 0;
        m->l4_len = 0;
        m->udp_pseudo_csum = 0;
        m->nb_segs = (uint32_t)mbuf->nb_segs;
        m->rx_ol_flags = rusteron_dpdk_port_rx_ol_flags(mbuf);
        m->hw_rx_ns = rusteron_dpdk_port_ts_offset >= 0 && 0 != (mbuf->ol_flags & rusteron_dpdk_port_ts_flag)
            ? *RTE_MBUF_DYNFIELD(mbuf, rusteron_dpdk_port_ts_offset, rte_mbuf_timestamp_t *)
            : 0;
    }
    return received;
}

/* Extended statistics (plan §9). DPDK exposes the count via a NULL-array call,
 * and the values/names in id order (ENA reports sequential ids). */

static uint32_t rusteron_dpdk_port_xstats_count(uint16_t port_id)
{
    int n = rte_eth_xstats_get_names(port_id, NULL, 0);
    return n > 0 ? (uint32_t)n : 0;
}

static int rusteron_dpdk_port_xstats_names(uint16_t port_id, char *names, uint32_t count)
{
    if (count == 0)
    {
        return 0;
    }
    struct rte_eth_xstat_name *buf = calloc(count, sizeof(*buf));
    if (NULL == buf)
    {
        return -1;
    }
    int got = rte_eth_xstats_get_names(port_id, buf, count);
    if (got != (int)count)
    {
        free(buf);
        return -1;
    }
    for (uint32_t i = 0; i < count; i++)
    {
        memcpy(names + (i * RUSTERON_DPDK_XSTAT_NAME_LEN), buf[i].name, RUSTERON_DPDK_XSTAT_NAME_LEN);
    }
    free(buf);
    return 0;
}

static int rusteron_dpdk_port_xstats_get(uint16_t port_id, uint64_t *values, uint32_t count)
{
    if (count == 0)
    {
        return 0;
    }
    struct rte_eth_xstat *buf = calloc(count, sizeof(*buf));
    if (NULL == buf)
    {
        return -1;
    }
    int got = rte_eth_xstats_get(port_id, buf, count);
    if (got != (int)count)
    {
        free(buf);
        return -1;
    }
    for (uint32_t i = 0; i < count; i++)
    {
        values[i] = buf[i].value;
    }
    free(buf);
    return 0;
}

static uint32_t rusteron_dpdk_port_mempool_avail(void *mempool)
{
    return rte_mempool_avail_count(mempool);
}

static int rusteron_dpdk_port_basic_stats(uint16_t port_id, uint64_t *ierrors, uint64_t *imissed, uint64_t *rx_nombuf)
{
    struct rte_eth_stats stats;
    if (0 != rte_eth_stats_get(port_id, &stats))
    {
        return -1;
    }
    *ierrors = stats.ierrors;
    *imissed = stats.imissed;
    *rx_nombuf = stats.rx_nombuf;
    return 0;
}

/* rte_timer_alt_manage's callback: the timer's own. */
static void rusteron_dpdk_port_timer_run(struct rte_timer *tim)
{
    tim->f(tim, tim->arg);
}

/* The PMDs' due timers. rte_timer_manage runs only the calling lcore's, and
 * the ENA PMD arms its watchdog (ena_start: rte_timer_reset on rte_lcore_id())
 * on the lcore that started the port: the main lcore, the thread that created
 * the transport and initialised the EAL, which never comes back to run it.
 * rte_timer_alt_manage runs another lcore's list from this one, and re-arms a
 * periodic timer on this one's, so both lists are run. Timers armed by
 * rte_timer_reset are in the default timer data, id 0. The calling thread
 * must have an lcore id (registered); the call never blocks. */
static int rusteron_dpdk_port_timer_service(void)
{
    const unsigned int self = rte_lcore_id();
    if (self >= RTE_MAX_LCORE)
    {
        return -1; /* no lcore: the timer lists would be indexed past their end */
    }
    unsigned int lcores[2] = { rte_get_main_lcore(), self };
    const int rc = rte_timer_alt_manage(0, lcores, self == lcores[0] ? 1 : 2, rusteron_dpdk_port_timer_run);
    /* -EINVAL: no PMD initialised the timer subsystem (no ENA), so no timers */
    return -EINVAL == rc ? 0 : rc;
}

/* RTE_ETH_EVENT_INTR_RESET: the PMD asks for a device reset (ENA, from its
 * watchdog). Raised on the thread that found it (the housekeeping thread's
 * timer service for ENA, an interrupt thread for others), so only an atomic
 * count; housekeeping reports it. */
static int rusteron_dpdk_port_on_reset(uint16_t port_id, enum rte_eth_event_type event, void *cb_arg, void *ret_param)
{
    (void)port_id;
    (void)event;
    (void)ret_param;
    __atomic_add_fetch((uint64_t *)cb_arg, 1, __ATOMIC_RELAXED);
    return 0;
}

static int rusteron_dpdk_port_watch_reset(uint16_t port_id, uint64_t *resets)
{
    return rte_eth_dev_callback_register(port_id, RTE_ETH_EVENT_INTR_RESET, rusteron_dpdk_port_on_reset, resets);
}

/* -EAGAIN while the callback runs on another thread: wait it out (a second
 * at most), as `resets` goes with the transport. */
static void rusteron_dpdk_port_unwatch_reset(uint16_t port_id, uint64_t *resets)
{
    for (int tries = 0; tries < 1000; tries++)
    {
        if (-EAGAIN != rte_eth_dev_callback_unregister(
                           port_id, RTE_ETH_EVENT_INTR_RESET, rusteron_dpdk_port_on_reset, resets))
        {
            return;
        }
        rte_delay_ms(1);
    }
}

rusteron_dpdk_port_ops_t *rusteron_dpdk_port_ops_real(void)
{
    static rusteron_dpdk_port_ops_t ops = {
        .probe_port = rusteron_dpdk_port_probe_port,
        .dev_info = rusteron_dpdk_port_dev_info,
        .dev_configure = rusteron_dpdk_port_dev_configure,
        .mempool_create = rusteron_dpdk_port_mempool_create,
        .mbuf_room = rusteron_dpdk_port_mbuf_room,
        .rx_queue_setup = rusteron_dpdk_port_rx_queue_setup,
        .tx_queue_setup = rusteron_dpdk_port_tx_queue_setup,
        .dev_set_mtu = rusteron_dpdk_port_dev_set_mtu,
        .dev_start = rusteron_dpdk_port_dev_start,
        .link_wait_ready = rusteron_dpdk_port_link_wait_ready,
        .dev_stop = rusteron_dpdk_port_dev_stop,
        .dev_close = rusteron_dpdk_port_dev_close,
        .mempool_free = rusteron_dpdk_port_mempool_free,
        .mbuf_alloc = rusteron_dpdk_port_mbuf_alloc,
        .mbuf_release = rusteron_dpdk_port_mbuf_release,
        .tx_burst = rusteron_dpdk_port_tx_burst,
        .rx_burst = rusteron_dpdk_port_rx_burst,
        .xstats_count = rusteron_dpdk_port_xstats_count,
        .xstats_names = rusteron_dpdk_port_xstats_names,
        .xstats_get = rusteron_dpdk_port_xstats_get,
        .mempool_avail = rusteron_dpdk_port_mempool_avail,
        .basic_stats = rusteron_dpdk_port_basic_stats,
        .timer_service = rusteron_dpdk_port_timer_service,
        .watch_reset = rusteron_dpdk_port_watch_reset,
        .unwatch_reset = rusteron_dpdk_port_unwatch_reset,
    };
    return &ops;
}
