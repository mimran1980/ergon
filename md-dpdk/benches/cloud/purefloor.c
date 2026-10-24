/* purefloor: the pure floor (modes.sh dpdk-pure). rtt's round trip over DPDK
 * alone: none of ergon-md-dpdk and no Aeron, so an idle round trip's tail
 * that the raw floor (dpdk-raw: ergon-md-dpdk's packet path) shows can be
 * put on the transport or beneath it (the NIC, its PMD, the host, the
 * fabric). vm.sh build compiles it on the host against pkg-config's libdpdk,
 * as gapspin.
 *
 *   purefloor xpong <ping endpoint> <pong endpoint> <pong cpu|->
 *   purefloor xping <ping endpoint> <pong endpoint> <messages> <warmup> <ping cpu|->
 *
 * The arguments, the pacing (each ping sent once the last one's echo is
 * back), the warm-up, the 64-byte datagrams (the send stamp at byte 32, the
 * sequence at 40), the lost-ping timeout, the connect probe and every output
 * are the floor client's (harness/src/bin/floor.rs, roundtrip.rs): the
 * rtt,... line, RTT_HISTOGRAM (HdrHistogram's 3-significant-digit buckets),
 * LOAD_SPIKES, LOOP_STALLS_DIR, under LABEL, RTT_SECONDS, RTT_LOST_TIMEOUT_US,
 * LOAD_SPIKE_NS, LOOP_STALL_NS, FLOOR_CONNECT_SECS.
 *
 * One port, the driver's receiver port (driver.env: RUSTERON_DPDK_RECEIVER_PCI,
 * _IPV4_CIDR, _GATEWAY), one RX and one TX queue, polled by rte_eth_rx_burst
 * and sent on by rte_eth_tx_burst from this one thread. The pong is a
 * reflector: each UDP datagram to its endpoint goes back out with its MAC, IP
 * addresses and UDP ports swapped (testpmd's 5tswap), its payload untouched.
 * The ping resolves its next hop (the peer, or the gateway off the subnet) by
 * ARP; both answer ARP for their own address. The ping times each round trip
 * by the CPU's counter: x86 the TSC (calibrated against
 * CLOCK_MONOTONIC_RAW), arm64 the generic timer's virtual count, CNTVCT_EL0,
 * at CNTFRQ_EL0.
 *
 * The EAL and the port as the transport sets them up: RUSTERON_DPDK_EAL_ARGS,
 * _FILE_PREFIX, _HUGE_DIR, _EAL_IN_MEMORY; on a PCI device ENA's large LLQ
 * entries (llq_policy=3; enable_llq=1 before DPDK 24.11) and
 * RUSTERON_DPDK_ENA_DEVARGS, a name starting net_ taken as a --vdev (the
 * tests'); RUSTERON_DPDK_RX_DESCRIPTORS (min(4096, the NIC's) unset),
 * _TX_DESCRIPTORS (256), _MBUFS_PER_PORT (16384), _MEMPOOL_CACHE (256),
 * _BURST_SIZE (32), _RX_HW_TIMESTAMP (the offload on, its stamps unread: the
 * ENA's RX descriptors as the transport's). An ENA port whose LLQ BAR is not
 * mapped write-combined stops it. No checksum offloads: the IPv4 checksum in
 * software, the UDP checksum 0. */
#define _GNU_SOURCE
#include <arpa/inet.h>
#include <inttypes.h>
#include <sched.h>
#include <signal.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <time.h>
#include <unistd.h>

#include <rte_arp.h>
#include <rte_eal.h>
#include <rte_ethdev.h>
#include <rte_ether.h>
#include <rte_ip.h>
#include <rte_mbuf.h>
#include <rte_mempool.h>
#include <rte_udp.h>
#include <rte_version.h>

#if defined(__x86_64__)
#include <x86intrin.h>
/* ordered: no earlier load or later instruction crosses the read */
static inline uint64_t ticks(void)
{
    _mm_lfence();
    const uint64_t t = __rdtsc();
    _mm_lfence();
    return t;
}
#elif defined(__aarch64__)
/* isb: the read is not taken ahead of the earlier instructions */
static inline uint64_t ticks(void)
{
    uint64_t v;
    __asm__ __volatile__("isb\n\tmrs %0, cntvct_el0" : "=r"(v) : : "memory");
    return v;
}
#else
#error "purefloor: x86_64 or aarch64"
#endif

#define DATAGRAM 64
#define STAMP 32
#define SEQ 40
#define PING_STREAM_ID 1002
#define SPIKE_CAPACITY 100000
#define STALL_CAPACITY 1000000
#define HDRS (sizeof(struct rte_ether_hdr) + sizeof(struct rte_ipv4_hdr) + sizeof(struct rte_udp_hdr))
#define FRAME (HDRS + DATAGRAM)
#define ARP_FRAME 60
#define MAX_BURST 256
#define HISTOGRAM_MAX_NS 60000000000ULL

static volatile sig_atomic_t stop;
static double ns_per_tick;

static void die(const char *fmt, ...) __attribute__((format(printf, 1, 2), noreturn));
static void die(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    fputs("purefloor: ", stderr);
    vfprintf(stderr, fmt, ap);
    fputc('\n', stderr);
    va_end(ap);
    exit(1);
}

static void on_signal(int sig)
{
    (void)sig;
    stop = 1;
}

static uint64_t ns_of(uint64_t t)
{
    return (uint64_t)((double)t * ns_per_tick);
}

static uint64_t ticks_of(uint64_t ns)
{
    return (uint64_t)((double)ns / ns_per_tick);
}

static void calibrate(void)
{
#if defined(__aarch64__)
    uint64_t hz;
    __asm__ __volatile__("mrs %0, cntfrq_el0" : "=r"(hz));
    if (0 == hz)
    {
        die("CNTFRQ_EL0 reads 0");
    }
    ns_per_tick = 1e9 / (double)hz;
#else
    struct timespec a, b;
    clock_gettime(CLOCK_MONOTONIC_RAW, &a);
    const uint64_t t0 = ticks();
    do
    {
        clock_gettime(CLOCK_MONOTONIC_RAW, &b);
    } while ((b.tv_sec - a.tv_sec) * 1000000000L + (b.tv_nsec - a.tv_nsec) < 200000000L);
    const uint64_t t1 = ticks();
    ns_per_tick = ((double)(b.tv_sec - a.tv_sec) * 1e9 + (double)(b.tv_nsec - a.tv_nsec)) / (double)(t1 - t0);
#endif
}

static uint64_t realtime_ns(void)
{
    struct timespec t;
    clock_gettime(CLOCK_REALTIME, &t);
    return (uint64_t)t.tv_sec * 1000000000ULL + (uint64_t)t.tv_nsec;
}

/* <name> as a whole number from <least> to <most>, else <fallback> when
 * unset or empty; anything else stops it, naming the variable */
static uint64_t env_u64(const char *name, uint64_t fallback, uint64_t least, uint64_t most)
{
    const char *v = getenv(name);
    if (NULL == v || '\0' == *v)
    {
        return fallback;
    }
    char *end;
    const unsigned long long n = strtoull(v, &end, 10);
    if ('\0' != *end || '-' == *v || n < least || n > most)
    {
        die("%s=%s: a whole number from %" PRIu64 " to %" PRIu64, name, v, least, most);
    }
    return n;
}

static int env_set(const char *name)
{
    const char *v = getenv(name);
    return NULL != v && '\0' != *v;
}

static int env_true(const char *name)
{
    const char *v = getenv(name);
    if (NULL == v || '\0' == *v || 0 == strcmp(v, "0") || 0 == strcasecmp(v, "false"))
    {
        return 0;
    }
    if (0 == strcmp(v, "1") || 0 == strcasecmp(v, "true"))
    {
        return 1;
    }
    die("%s=%s: 0, 1, true or false", name, v);
}

static const char *env_required(const char *name)
{
    const char *v = getenv(name);
    if (NULL == v || '\0' == *v)
    {
        die("%s is not set (vm.sh driver dpdk-pure writes it in driver.env)", name);
    }
    return v;
}

struct endpoint
{
    uint32_t ip; /* network order */
    uint16_t port; /* network order */
};

static struct endpoint endpoint(const char *arg)
{
    char host[64];
    const char *colon = strrchr(arg, ':');
    if (NULL == colon || (size_t)(colon - arg) >= sizeof(host))
    {
        die("endpoint %s: an IPv4 host:port", arg);
    }
    memcpy(host, arg, (size_t)(colon - arg));
    host[colon - arg] = '\0';
    struct in_addr a;
    char *end;
    const unsigned long port = strtoul(colon + 1, &end, 10);
    if (1 != inet_pton(AF_INET, host, &a) || '\0' != *end || 0 == port || port > 65535)
    {
        die("endpoint %s: an IPv4 host:port", arg);
    }
    return (struct endpoint){.ip = a.s_addr, .port = htons((uint16_t)port)};
}

static int cpu_arg(const char *arg)
{
    if (0 == strcmp(arg, "-"))
    {
        return -1;
    }
    char *end;
    const long cpu = strtol(arg, &end, 10);
    if ('\0' != *end || cpu < 0 || cpu >= CPU_SETSIZE)
    {
        die("cpu %s: a CPU number or -", arg);
    }
    return (int)cpu;
}

static void pin(int cpu)
{
    if (cpu < 0)
    {
        return;
    }
    cpu_set_t set;
    CPU_ZERO(&set);
    CPU_SET(cpu, &set);
    if (0 != sched_setaffinity(0, sizeof(set), &set))
    {
        die("pinning to CPU %d failed", cpu);
    }
}

/* Gaps of at least a threshold in the calling loop, each with when it ended
 * (the floor client's Stalls): preallocated, so recording never allocates */
struct stalls
{
    uint64_t threshold, last, t0, t0_realtime;
    uint64_t *end, *gap;
    size_t n;
};

static struct stalls *stalls_new(void)
{
    if (!env_set("LOOP_STALLS_DIR"))
    {
        return NULL;
    }
    struct stalls *s = calloc(1, sizeof(*s));
    if (NULL == s || NULL == (s->end = calloc(STALL_CAPACITY, sizeof(uint64_t))) ||
        NULL == (s->gap = calloc(STALL_CAPACITY, sizeof(uint64_t))))
    {
        die("no memory for the stalls");
    }
    s->threshold = ticks_of(env_u64("LOOP_STALL_NS", 20000, 0, UINT64_MAX / 2));
    s->t0 = s->last = ticks();
    s->t0_realtime = realtime_ns();
    return s;
}

static inline void stalls_pass(struct stalls *s, uint64_t now)
{
    if (NULL == s)
    {
        return;
    }
    const uint64_t gap = now - s->last;
    if (gap >= s->threshold && s->n < STALL_CAPACITY)
    {
        s->end[s->n] = now - s->t0;
        s->gap[s->n] = gap;
        s->n++;
    }
    s->last = now;
}

static void stalls_write(const struct stalls *s, const char *name)
{
    if (NULL == s)
    {
        return;
    }
    char path[4096];
    snprintf(path, sizeof(path), "%s/%s", getenv("LOOP_STALLS_DIR"), name);
    FILE *f = fopen(path, "w");
    if (NULL == f)
    {
        die("LOOP_STALLS_DIR %s: cannot write", path);
    }
    fprintf(f, "# start_realtime_ns=%" PRIu64 "\nend_ns,gap_ns\n", s->t0_realtime);
    for (size_t i = 0; i < s->n; i++)
    {
        fprintf(f, "%" PRIu64 ",%" PRIu64 "\n", ns_of(s->end[i]), ns_of(s->gap[i]));
    }
    fclose(f);
}

/* HdrHistogram's buckets at 3 significant digits from 1 (roundtrip.rs's
 * Histogram::new_with_bounds(1, 60 s, 3)): 2048 sub-buckets, so a value's
 * bucket is 53 less the leading zeros of value | 2047 */
static inline unsigned hdr_bucket(uint64_t v)
{
    return 53U - (unsigned)__builtin_clzll(v | 2047U);
}

static inline uint64_t hdr_lowest(uint64_t v)
{
    const unsigned b = hdr_bucket(v);
    return (v >> b) << b;
}

static inline uint64_t hdr_highest(uint64_t v)
{
    return hdr_lowest(v) + (1ULL << hdr_bucket(v)) - 1;
}

static inline uint64_t hdr_median(uint64_t v)
{
    return hdr_lowest(v) + ((1ULL << hdr_bucket(v)) >> 1);
}

/* The measured run's round trips (roundtrip.rs Results) */
struct results
{
    uint64_t *samples; /* ns */
    size_t n, capacity;
    uint64_t spike_ns, spikes_total, t0, t0_realtime;
    uint64_t *due, *latency, *ping;
    size_t spikes, spike_capacity;
};

static void results_new(struct results *r, size_t messages)
{
    memset(r, 0, sizeof(*r));
    r->capacity = messages;
    r->samples = calloc(messages > 0 ? messages : 1, sizeof(uint64_t));
    r->spike_ns = env_u64("LOAD_SPIKE_NS", 100000, 0, UINT64_MAX / 2);
    r->spike_capacity = env_set("LOAD_SPIKES") ? SPIKE_CAPACITY : 0;
    r->due = calloc(SPIKE_CAPACITY, sizeof(uint64_t));
    r->latency = calloc(SPIKE_CAPACITY, sizeof(uint64_t));
    r->ping = calloc(SPIKE_CAPACITY, sizeof(uint64_t));
    if (NULL == r->samples || NULL == r->due || NULL == r->latency || NULL == r->ping)
    {
        die("no memory for %zu samples", messages);
    }
}

static inline void results_record(struct results *r, uint64_t rtt_ticks, size_t ping)
{
    uint64_t rtt = ns_of(rtt_ticks);
    if (rtt > HISTOGRAM_MAX_NS)
    {
        rtt = HISTOGRAM_MAX_NS;
    }
    if (r->n < r->capacity)
    {
        r->samples[r->n++] = rtt;
    }
    if (rtt >= r->spike_ns)
    {
        r->spikes_total++;
        if (r->spikes < r->spike_capacity)
        {
            const uint64_t since = ns_of(ticks() - r->t0);
            r->due[r->spikes] = since > rtt ? since - rtt : 0;
            r->latency[r->spikes] = rtt;
            r->ping[r->spikes] = ping;
            r->spikes++;
        }
    }
}

static int cmp_u64(const void *a, const void *b)
{
    const uint64_t x = *(const uint64_t *)a, y = *(const uint64_t *)b;
    return x < y ? -1 : x > y;
}

/* hdrhistogram's value_at_quantile: the highest equivalent of the value at
 * rank ceil(q * n), at least 1 */
static uint64_t quantile(const struct results *r, double q)
{
    if (0 == r->n)
    {
        return 0;
    }
    const double fractional = q * (double)r->n;
    uint64_t rank = (uint64_t)fractional;
    if ((double)rank < fractional)
    {
        rank++;
    }
    if (rank < 1)
    {
        rank = 1;
    }
    if (rank > r->n)
    {
        rank = r->n;
    }
    return hdr_highest(r->samples[rank - 1]);
}

static void results_report(struct results *r, uint64_t lost)
{
    qsort(r->samples, r->n, sizeof(uint64_t), cmp_u64);
    double sum = 0;
    for (size_t i = 0; i < r->n; i++)
    {
        sum += (double)hdr_median(r->samples[i]);
    }
    const char *label = getenv("LABEL");
    printf("rtt,%s,udp,%" PRIu64 ",%" PRIu64 ",%" PRIu64 ",%" PRIu64 ",%" PRIu64 ",%" PRIu64 ",%.0f,%zu,%" PRIu64 "\n",
           NULL != label ? label : "-", quantile(r, 0.5), quantile(r, 0.9), quantile(r, 0.99), quantile(r, 0.999),
           quantile(r, 0.9999), r->n ? hdr_highest(r->samples[r->n - 1]) : 0, r->n ? sum / (double)r->n : 0.0, r->n,
           lost);
    fflush(stdout);
    const char *path = getenv("RTT_HISTOGRAM");
    if (NULL != path && '\0' != *path)
    {
        FILE *f = fopen(path, "w");
        if (NULL == f)
        {
            die("RTT_HISTOGRAM %s: cannot write", path);
        }
        fputs("value_ns,count\n", f);
        for (size_t i = 0; i < r->n;)
        {
            const uint64_t v = hdr_highest(r->samples[i]);
            size_t j = i;
            while (j < r->n && hdr_highest(r->samples[j]) == v)
            {
                j++;
            }
            fprintf(f, "%" PRIu64 ",%zu\n", v, j - i);
            i = j;
        }
        fclose(f);
    }
    path = getenv("LOAD_SPIKES");
    if (NULL != path && '\0' != *path)
    {
        FILE *f = fopen(path, "w");
        if (NULL == f)
        {
            die("LOAD_SPIKES %s: cannot write", path);
        }
        fprintf(f, "# start_realtime_ns=%" PRIu64 "\n# spikes_total=%" PRIu64 "\nstream,due_ns,latency_ns,ping\n",
                r->t0_realtime, r->spikes_total);
        for (size_t i = 0; i < r->spikes; i++)
        {
            fprintf(f, "%d,%" PRIu64 ",%" PRIu64 ",%" PRIu64 "\n", PING_STREAM_ID, r->due[i], r->latency[i], r->ping[i]);
        }
        fclose(f);
    }
}

/* The one port and what this side is on it */
struct wire
{
    uint16_t port;
    uint16_t burst;
    struct rte_mempool *pool;
    struct rte_ether_addr mac, next_mac;
    struct endpoint listen, peer;
    uint32_t next_hop; /* network order */
    uint8_t template[FRAME];
};

/* The EAL's arguments as the transport builds them (rusteron_dpdk_eal.c,
 * rusteron_dpdk_runtime.c's ENA device arguments) */
static int eal_init(const char *device)
{
    static char *argv[96];
    static char allow[1024];
    int argc = 0;
    argv[argc++] = "purefloor";
    char *eal = strdup(getenv("RUSTERON_DPDK_EAL_ARGS") ? getenv("RUSTERON_DPDK_EAL_ARGS") : "");
    if (NULL == eal)
    {
        die("no memory");
    }
    int no_huge = 0;
    char *save = NULL;
    for (char *tok = strtok_r(eal, " ", &save); NULL != tok && argc < 80; tok = strtok_r(NULL, " ", &save))
    {
        no_huge |= 0 == strcmp(tok, "--no-huge");
        argv[argc++] = tok;
    }
    if (0 == strncmp(device, "net_", 4))
    {
        snprintf(allow, sizeof(allow), "--vdev=%s", device);
        argv[argc++] = allow;
    }
    else
    {
        const char *extra = getenv("RUSTERON_DPDK_ENA_DEVARGS");
        extra = NULL != extra ? extra : "";
#if RTE_VERSION >= RTE_VERSION_NUM(24, 11, 0, 0)
        const char *key = "llq_policy", *policy = "llq_policy=3";
#else
        const char *key = "enable_llq", *policy = "enable_llq=1";
#endif
        const int named = NULL != strstr(extra, key);
        const int n = snprintf(allow, sizeof(allow), "%s%s%s%s%s", device, named ? "" : ",", named ? "" : policy,
                               '\0' != *extra ? "," : "", extra);
        if (n < 0 || (size_t)n >= sizeof(allow))
        {
            die("RUSTERON_DPDK_ENA_DEVARGS too long");
        }
        argv[argc++] = "-a";
        argv[argc++] = allow;
    }
    argv[argc++] = "--file-prefix";
    argv[argc++] = (char *)env_required("RUSTERON_DPDK_FILE_PREFIX");
    argv[argc++] = "--proc-type";
    argv[argc++] = "primary";
    argv[argc++] = "--no-telemetry";
    if (env_true("RUSTERON_DPDK_EAL_IN_MEMORY"))
    {
        argv[argc++] = "--in-memory";
    }
    else if (!no_huge)
    {
        argv[argc++] = "--huge-dir";
        argv[argc++] = getenv("RUSTERON_DPDK_HUGE_DIR") && *getenv("RUSTERON_DPDK_HUGE_DIR") ? getenv("RUSTERON_DPDK_HUGE_DIR")
                                                                                              : "/dev/hugepages";
    }
    fprintf(stderr, "purefloor: EAL");
    for (int i = 1; i < argc; i++)
    {
        fprintf(stderr, " %s", argv[i]);
    }
    fputc('\n', stderr);
    return rte_eal_init(argc, argv);
}

/* ENA: the low-latency queue BAR (BAR 2) mapped write-combined in this
 * process, as host.sh ena_llq_check and the transport require */
static void ena_wc_check(const char *pci)
{
    char want[128];
    snprintf(want, sizeof(want), "/%s/resource2_wc", pci);
    FILE *f = fopen("/proc/self/maps", "r");
    if (NULL == f)
    {
        die("/proc/self/maps unreadable: the ENA's LLQ BAR mapping not confirmed");
    }
    char line[4096];
    int found = 0;
    while (!found && NULL != fgets(line, sizeof(line), f))
    {
        found = NULL != strstr(line, want);
    }
    fclose(f);
    if (!found)
    {
        die("%s: its low-latency queue BAR is not mapped write-combined (no %s in /proc/self/maps): load igb_uio with "
            "wc_activate=1",
            pci, want);
    }
}

static void port_open(struct wire *w, const char *pci)
{
    uint16_t port = RTE_MAX_ETHPORTS, count = 0, id;
    RTE_ETH_FOREACH_DEV(id)
    {
        port = 0 == count ? id : port;
        count++;
    }
    if (1 != count)
    {
        die("%u ports probed for %s: one wanted", count, pci);
    }
    w->port = port;
    struct rte_eth_dev_info info;
    if (0 != rte_eth_dev_info_get(port, &info))
    {
        die("rte_eth_dev_info_get failed");
    }
    const uint64_t rx_most = info.rx_desc_lim.nb_max ? info.rx_desc_lim.nb_max : 4096;
    uint16_t rx = (uint16_t)env_u64("RUSTERON_DPDK_RX_DESCRIPTORS", rx_most < 4096 ? rx_most : 4096, 64, 8192);
    uint16_t tx = (uint16_t)env_u64("RUSTERON_DPDK_TX_DESCRIPTORS", 256, 64, 8192);
    const uint16_t rx_asked = rx, tx_asked = tx;
    const unsigned mbufs = (unsigned)env_u64("RUSTERON_DPDK_MBUFS_PER_PORT", 16384, 1, UINT32_MAX);
    const unsigned cache = (unsigned)env_u64("RUSTERON_DPDK_MEMPOOL_CACHE", 256, 0, 512);
    w->burst = (uint16_t)env_u64("RUSTERON_DPDK_BURST_SIZE", 32, 1, MAX_BURST);
    struct rte_eth_conf conf;
    memset(&conf, 0, sizeof(conf));
    const int stamps = env_true("RUSTERON_DPDK_RX_HW_TIMESTAMP");
    if (stamps)
    {
        if (0 == (info.rx_offload_capa & RTE_ETH_RX_OFFLOAD_TIMESTAMP))
        {
            die("RUSTERON_DPDK_RX_HW_TIMESTAMP=1, but %s (%s) offers no RX timestamps", pci, info.driver_name);
        }
        conf.rxmode.offloads |= RTE_ETH_RX_OFFLOAD_TIMESTAMP;
    }
    const int socket = rte_eth_dev_socket_id(port);
    w->pool = rte_pktmbuf_pool_create("purefloor", mbufs, cache, 0, RTE_MBUF_DEFAULT_BUF_SIZE, socket);
    if (NULL == w->pool)
    {
        die("rte_pktmbuf_pool_create(%u mbufs, cache %u) failed: %s", mbufs, cache, rte_strerror(rte_errno));
    }
    int rc = rte_eth_dev_configure(port, 1, 1, &conf);
    if (0 != rc)
    {
        die("rte_eth_dev_configure failed: %s", rte_strerror(-rc));
    }
    if (0 != rte_eth_dev_adjust_nb_rx_tx_desc(port, &rx, &tx))
    {
        die("rte_eth_dev_adjust_nb_rx_tx_desc failed");
    }
    if ((env_set("RUSTERON_DPDK_RX_DESCRIPTORS") && rx != rx_asked) ||
        (env_set("RUSTERON_DPDK_TX_DESCRIPTORS") && tx != tx_asked))
    {
        die("%s takes %u RX and %u TX descriptors, not the %u and %u asked", info.driver_name, rx, tx, rx_asked,
            tx_asked);
    }
    if (0 != (rc = rte_eth_rx_queue_setup(port, 0, rx, socket, NULL, w->pool)))
    {
        die("rte_eth_rx_queue_setup(%u) failed: %s", rx, rte_strerror(-rc));
    }
    if (0 != (rc = rte_eth_tx_queue_setup(port, 0, tx, socket, NULL)))
    {
        die("rte_eth_tx_queue_setup(%u) failed: %s", tx, rte_strerror(-rc));
    }
    if (0 != (rc = rte_eth_dev_start(port)))
    {
        die("rte_eth_dev_start failed: %s", rte_strerror(-rc));
    }
    if (0 == strcmp(info.driver_name, "net_ena"))
    {
        ena_wc_check(pci);
    }
    if (0 != rte_eth_macaddr_get(port, &w->mac))
    {
        die("rte_eth_macaddr_get failed");
    }
    struct rte_eth_link link;
    memset(&link, 0, sizeof(link));
    for (int i = 0; i < 100; i++)
    {
        if (0 == rte_eth_link_get_nowait(port, &link) && link.link_status)
        {
            break;
        }
        usleep(100000);
    }
    char mac[RTE_ETHER_ADDR_FMT_SIZE];
    rte_ether_format_addr(mac, sizeof(mac), &w->mac);
    fprintf(stderr,
            "purefloor: %s alone, no ergon-md-dpdk or Aeron: port %s (%s, %s), %u RX and %u TX descriptors, "
            "%u mbufs (cache %u), bursts of %u, RX timestamps %s, link %s\n",
            rte_version(), pci, info.driver_name, mac, rx, tx, mbufs, cache, w->burst, stamps ? "on" : "off",
            link.link_status ? "up" : "down at start");
}

static int is_arp(const struct rte_mbuf *m)
{
    const struct rte_ether_hdr *eth = rte_pktmbuf_mtod(m, const struct rte_ether_hdr *);
    return m->data_len >= sizeof(*eth) + sizeof(struct rte_arp_hdr) &&
           rte_cpu_to_be_16(RTE_ETHER_TYPE_ARP) == eth->ether_type;
}

/* An ARP request for this side's address answered in place (sent back);
 * any other frame freed. 1 if it was an ARP reply from <from> (its sender's
 * MAC in <mac>) */
static int other_frame(struct wire *w, struct rte_mbuf *m, uint32_t from, struct rte_ether_addr *mac)
{
    if (!is_arp(m))
    {
        rte_pktmbuf_free(m);
        return 0;
    }
    struct rte_ether_hdr *eth = rte_pktmbuf_mtod(m, struct rte_ether_hdr *);
    struct rte_arp_hdr *arp = (struct rte_arp_hdr *)(eth + 1);
    int replied = 0;
    if (rte_cpu_to_be_16(RTE_ARP_OP_REPLY) == arp->arp_opcode && 0 != from && arp->arp_data.arp_sip == from)
    {
        rte_ether_addr_copy(&arp->arp_data.arp_sha, mac);
        replied = 1;
    }
    else if (rte_cpu_to_be_16(RTE_ARP_OP_REQUEST) == arp->arp_opcode && arp->arp_data.arp_tip == w->listen.ip)
    {
        rte_ether_addr_copy(&eth->src_addr, &eth->dst_addr);
        rte_ether_addr_copy(&w->mac, &eth->src_addr);
        arp->arp_opcode = rte_cpu_to_be_16(RTE_ARP_OP_REPLY);
        rte_ether_addr_copy(&arp->arp_data.arp_sha, &arp->arp_data.arp_tha);
        arp->arp_data.arp_tip = arp->arp_data.arp_sip;
        rte_ether_addr_copy(&w->mac, &arp->arp_data.arp_sha);
        arp->arp_data.arp_sip = w->listen.ip;
        if (1 == rte_eth_tx_burst(w->port, 0, &m, 1))
        {
            return 0;
        }
    }
    rte_pktmbuf_free(m);
    return replied;
}

/* A UDP datagram of at least DATAGRAM bytes to this side's endpoint: its payload */
static uint8_t *datagram(const struct wire *w, struct rte_mbuf *m)
{
    if (m->data_len < FRAME)
    {
        return NULL;
    }
    struct rte_ether_hdr *eth = rte_pktmbuf_mtod(m, struct rte_ether_hdr *);
    const struct rte_ipv4_hdr *ip = (const struct rte_ipv4_hdr *)(eth + 1);
    const struct rte_udp_hdr *udp = (const struct rte_udp_hdr *)(ip + 1);
    if (rte_cpu_to_be_16(RTE_ETHER_TYPE_IPV4) != eth->ether_type || 0x45 != ip->version_ihl ||
        IPPROTO_UDP != ip->next_proto_id || ip->dst_addr != w->listen.ip || udp->dst_port != w->listen.port)
    {
        return NULL;
    }
    return (uint8_t *)(udp + 1);
}

static void send_arp_request(struct wire *w)
{
    struct rte_mbuf *m = rte_pktmbuf_alloc(w->pool);
    if (NULL == m)
    {
        die("the mbuf pool is empty");
    }
    uint8_t *p = (uint8_t *)rte_pktmbuf_append(m, ARP_FRAME);
    if (NULL == p)
    {
        die("an mbuf too small for an ARP frame");
    }
    memset(p, 0, ARP_FRAME);
    struct rte_ether_hdr *eth = (struct rte_ether_hdr *)p;
    struct rte_arp_hdr *arp = (struct rte_arp_hdr *)(eth + 1);
    memset(&eth->dst_addr, 0xff, sizeof(eth->dst_addr));
    rte_ether_addr_copy(&w->mac, &eth->src_addr);
    eth->ether_type = rte_cpu_to_be_16(RTE_ETHER_TYPE_ARP);
    arp->arp_hardware = rte_cpu_to_be_16(RTE_ARP_HRD_ETHER);
    arp->arp_protocol = rte_cpu_to_be_16(RTE_ETHER_TYPE_IPV4);
    arp->arp_hlen = RTE_ETHER_ADDR_LEN;
    arp->arp_plen = 4;
    arp->arp_opcode = rte_cpu_to_be_16(RTE_ARP_OP_REQUEST);
    rte_ether_addr_copy(&w->mac, &arp->arp_data.arp_sha);
    arp->arp_data.arp_sip = w->listen.ip;
    arp->arp_data.arp_tip = w->next_hop;
    if (1 != rte_eth_tx_burst(w->port, 0, &m, 1))
    {
        rte_pktmbuf_free(m);
    }
}

/* The ping's frame: this side's endpoint to the peer's, by the next hop */
static void build_template(struct wire *w)
{
    memset(w->template, 0, sizeof(w->template));
    struct rte_ether_hdr *eth = (struct rte_ether_hdr *)w->template;
    struct rte_ipv4_hdr *ip = (struct rte_ipv4_hdr *)(eth + 1);
    struct rte_udp_hdr *udp = (struct rte_udp_hdr *)(ip + 1);
    rte_ether_addr_copy(&w->next_mac, &eth->dst_addr);
    rte_ether_addr_copy(&w->mac, &eth->src_addr);
    eth->ether_type = rte_cpu_to_be_16(RTE_ETHER_TYPE_IPV4);
    ip->version_ihl = 0x45;
    ip->total_length = rte_cpu_to_be_16(sizeof(*ip) + sizeof(*udp) + DATAGRAM);
    ip->fragment_offset = rte_cpu_to_be_16(RTE_IPV4_HDR_DF_FLAG);
    ip->time_to_live = 64;
    ip->next_proto_id = IPPROTO_UDP;
    ip->src_addr = w->listen.ip;
    ip->dst_addr = w->peer.ip;
    ip->hdr_checksum = rte_ipv4_cksum(ip);
    udp->src_port = w->listen.port;
    udp->dst_port = w->peer.port;
    udp->dgram_len = rte_cpu_to_be_16(sizeof(*udp) + DATAGRAM);
    udp->dgram_cksum = 0;
}

/* Sends datagram <seq> stamped <stamp> now: 0 when the TX ring is full */
static inline int send_ping(struct wire *w, uint64_t seq, uint64_t stamp)
{
    struct rte_mbuf *m = rte_pktmbuf_alloc(w->pool);
    if (NULL == m)
    {
        die("the mbuf pool is empty");
    }
    uint8_t *p = (uint8_t *)rte_pktmbuf_append(m, FRAME);
    if (NULL == p)
    {
        die("an mbuf too small for a %zu-byte frame", FRAME);
    }
    memcpy(p, w->template, FRAME);
    memcpy(p + HDRS + STAMP, &stamp, sizeof(stamp));
    memcpy(p + HDRS + SEQ, &seq, sizeof(seq));
    if (1 == rte_eth_tx_burst(w->port, 0, &m, 1))
    {
        return 1;
    }
    rte_pktmbuf_free(m);
    return 0;
}

static inline void drain(struct wire *w)
{
    struct rte_mbuf *bufs[MAX_BURST];
    const uint16_t n = rte_eth_rx_burst(w->port, 0, bufs, w->burst);
    for (uint16_t i = 0; i < n; i++)
    {
        other_frame(w, bufs[i], 0, NULL);
    }
}

static inline uint64_t seq_of(const uint8_t *payload)
{
    uint64_t seq;
    memcpy(&seq, payload + SEQ, sizeof(seq));
    return seq;
}

/* One round trip of datagram <seq> (floor.rs round_trip): 1 once its echo is
 * back, recorded when <r> is given; 0 if none came within <timeout> ticks */
static int round_trip(struct wire *w, struct results *r, struct stalls *s, uint64_t seq, uint64_t timeout, size_t i)
{
    stalls_pass(s, ticks());
    uint64_t started;
    for (;;)
    {
        started = ticks();
        if (send_ping(w, seq, started))
        {
            break;
        }
        drain(w);
    }
    for (;;)
    {
        stalls_pass(s, ticks());
        struct rte_mbuf *bufs[MAX_BURST];
        const uint16_t n = rte_eth_rx_burst(w->port, 0, bufs, w->burst);
        int echoed = 0;
        for (uint16_t k = 0; k < n; k++)
        {
            const uint8_t *payload = datagram(w, bufs[k]);
            if (NULL == payload)
            {
                other_frame(w, bufs[k], 0, NULL);
                continue;
            }
            if (seq_of(payload) == seq)
            {
                uint64_t sent;
                memcpy(&sent, payload + STAMP, sizeof(sent));
                const uint64_t now = ticks();
                if (NULL != r)
                {
                    results_record(r, now - sent, i);
                }
                echoed = 1;
            }
            rte_pktmbuf_free(bufs[k]);
        }
        if (echoed)
        {
            return 1;
        }
        if (0 == n && ticks() - started > timeout)
        {
            return 0;
        }
    }
}

/* The next hop's MAC by ARP, then probes (sequence 0) every 10 ms until the
 * pong echoes one, within FLOOR_CONNECT_SECS (30) */
static void connect_peer(struct wire *w)
{
    const uint64_t secs = env_u64("FLOOR_CONNECT_SECS", 30, 1, 86400);
    const uint64_t deadline = ticks() + ticks_of(secs * 1000000000ULL);
    const uint64_t ten_ms = ticks_of(10000000ULL);
    int resolved = 0;
    while (!resolved && ticks() < deadline)
    {
        send_arp_request(w);
        const uint64_t until = ticks() + ten_ms;
        while (!resolved && ticks() < until)
        {
            struct rte_mbuf *bufs[MAX_BURST];
            const uint16_t n = rte_eth_rx_burst(w->port, 0, bufs, w->burst);
            for (uint16_t k = 0; k < n; k++)
            {
                resolved |= other_frame(w, bufs[k], w->next_hop, &w->next_mac);
            }
        }
    }
    char hop[INET_ADDRSTRLEN], peer[INET_ADDRSTRLEN];
    inet_ntop(AF_INET, &w->next_hop, hop, sizeof(hop));
    inet_ntop(AF_INET, &w->peer.ip, peer, sizeof(peer));
    if (!resolved)
    {
        die("no ARP answer from the next hop %s within %" PRIu64 " s", hop, secs);
    }
    char mac[RTE_ETHER_ADDR_FMT_SIZE];
    rte_ether_format_addr(mac, sizeof(mac), &w->next_mac);
    fprintf(stderr, "purefloor: next hop %s is %s\n", hop, mac);
    build_template(w);
    while (ticks() < deadline)
    {
        if (round_trip(w, NULL, NULL, 0, ten_ms, 0))
        {
            return;
        }
    }
    die("no echo from the pong at %s:%u within %" PRIu64 " s", peer, ntohs(w->peer.port), secs);
}

static void run_ping(struct wire *w, size_t messages, size_t warmup, int cpu)
{
    connect_peer(w);
    pin(cpu);
    const uint64_t budget = ticks_of(env_u64("RTT_SECONDS", 20, 0, 86400) * 1000000000ULL);
    const uint64_t lost_us = env_u64("RTT_LOST_TIMEOUT_US", 0, 0, 86400000000ULL);
    const uint64_t timeout = ticks_of((lost_us > 0 ? lost_us : 50000) * 1000ULL);
    struct results r;
    results_new(&r, messages);
    uint64_t seq = 0;
    uint64_t start = ticks();
    for (size_t i = 0; i < warmup; i++)
    {
        round_trip(w, NULL, NULL, ++seq, timeout, i);
        if (0 == i % 64 && ticks() - start > budget / 4)
        {
            break;
        }
    }
    start = ticks();
    r.t0 = start;
    r.t0_realtime = realtime_ns();
    struct stalls *s = stalls_new();
    uint64_t lost = 0;
    for (size_t i = 0; i < messages; i++)
    {
        if (!round_trip(w, &r, s, ++seq, timeout, i))
        {
            lost++;
        }
        if (0 == i % 64 && ticks() - start > budget)
        {
            break;
        }
    }
    results_report(&r, lost);
    stalls_write(s, "stalls-ping.csv");
}

/* The reflector: each datagram to this side's endpoint back to its sender,
 * its MAC, IP addresses and UDP ports swapped (5tswap), until SIGTERM */
static void run_pong(struct wire *w, int cpu)
{
    pin(cpu);
    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = on_signal;
    sigaction(SIGTERM, &sa, NULL);
    sigaction(SIGINT, &sa, NULL);
    struct stalls *s = stalls_new();
    uint64_t echoed = 0;
    while (!stop)
    {
        stalls_pass(s, ticks());
        struct rte_mbuf *bufs[MAX_BURST], *out[MAX_BURST];
        const uint16_t n = rte_eth_rx_burst(w->port, 0, bufs, w->burst);
        uint16_t k = 0;
        for (uint16_t i = 0; i < n; i++)
        {
            if (NULL == datagram(w, bufs[i]))
            {
                other_frame(w, bufs[i], 0, NULL);
                continue;
            }
            struct rte_ether_hdr *eth = rte_pktmbuf_mtod(bufs[i], struct rte_ether_hdr *);
            struct rte_ipv4_hdr *ip = (struct rte_ipv4_hdr *)(eth + 1);
            struct rte_udp_hdr *udp = (struct rte_udp_hdr *)(ip + 1);
            struct rte_ether_addr mac;
            rte_ether_addr_copy(&eth->dst_addr, &mac);
            rte_ether_addr_copy(&eth->src_addr, &eth->dst_addr);
            rte_ether_addr_copy(&mac, &eth->src_addr);
            const uint32_t addr = ip->src_addr;
            ip->src_addr = ip->dst_addr;
            ip->dst_addr = addr;
            const uint16_t port = udp->src_port;
            udp->src_port = udp->dst_port;
            udp->dst_port = port;
            out[k++] = bufs[i];
        }
        uint16_t sent = 0;
        while (sent < k)
        {
            sent += rte_eth_tx_burst(w->port, 0, out + sent, k - sent);
            if (stop)
            {
                break;
            }
        }
        for (uint16_t i = sent; i < k; i++)
        {
            rte_pktmbuf_free(out[i]);
        }
        echoed += sent;
    }
    stalls_write(s, "stalls-pong.csv");
    fprintf(stderr, "purefloor: pong ended: SIGTERM (%" PRIu64 " echoed)\n", echoed);
}

int main(int argc, char **argv)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    const int ping = argc > 1 && 0 == strcmp(argv[1], "xping");
    const int pong = argc > 1 && 0 == strcmp(argv[1], "xpong");
    if (!((ping && 7 == argc) || (pong && 5 == argc)))
    {
        fprintf(stderr, "purefloor xpong <ping endpoint> <pong endpoint> <pong cpu|->\n"
                        "purefloor xping <ping endpoint> <pong endpoint> <messages> <warmup> <ping cpu|->\n");
        return 2;
    }
    if (0 != mlockall(MCL_CURRENT | MCL_FUTURE))
    {
        fprintf(stderr, "purefloor: mlockall failed: memory not locked\n");
    }
    calibrate();
    struct wire w;
    memset(&w, 0, sizeof(w));
    const struct endpoint ping_ep = endpoint(argv[2]), pong_ep = endpoint(argv[3]);
    /* each side listens on its own endpoint and sends to the other's */
    w.listen = pong ? ping_ep : pong_ep;
    w.peer = pong ? pong_ep : ping_ep;
    const size_t messages = ping ? (size_t)strtoull(argv[4], NULL, 10) : 0;
    const size_t warmup = ping ? (size_t)strtoull(argv[5], NULL, 10) : 0;
    const int cpu = cpu_arg(argv[argc - 1]);

    const char *pci = env_required("RUSTERON_DPDK_RECEIVER_PCI");
    const char *cidr = env_required("RUSTERON_DPDK_RECEIVER_IPV4_CIDR");
    char ip[64];
    const char *slash = strchr(cidr, '/');
    char *end;
    const long prefix = NULL != slash ? strtol(slash + 1, &end, 10) : -1;
    struct in_addr own;
    if (NULL == slash || (size_t)(slash - cidr) >= sizeof(ip) || '\0' != *end || prefix < 1 || prefix > 32)
    {
        die("RUSTERON_DPDK_RECEIVER_IPV4_CIDR=%s: <IPv4>/<prefix>", cidr);
    }
    memcpy(ip, cidr, (size_t)(slash - cidr));
    ip[slash - cidr] = '\0';
    if (1 != inet_pton(AF_INET, ip, &own) || own.s_addr != w.listen.ip)
    {
        die("this side's endpoint %s is not the receiver port's address (RUSTERON_DPDK_RECEIVER_IPV4_CIDR=%s)",
            pong ? argv[2] : argv[3], cidr);
    }
    const uint32_t mask = htonl(prefix == 32 ? 0xffffffffU : ~(0xffffffffU >> prefix));
    if ((w.peer.ip & mask) == (own.s_addr & mask))
    {
        w.next_hop = w.peer.ip;
    }
    else
    {
        struct in_addr gw;
        const char *gateway = getenv("RUSTERON_DPDK_RECEIVER_GATEWAY");
        if (NULL == gateway || 1 != inet_pton(AF_INET, gateway, &gw))
        {
            die("the peer is off the subnet %s, and RUSTERON_DPDK_RECEIVER_GATEWAY (%s) is no IPv4 address", cidr,
                gateway ? gateway : "unset");
        }
        w.next_hop = gw.s_addr;
    }

    if (eal_init(pci) < 0)
    {
        die("rte_eal_init failed: %s", rte_strerror(rte_errno));
    }
    port_open(&w, pci);
    fprintf(stderr, "purefloor: %s, listen %s, peer %s, clock %s (%.6f ns a tick)\n", ping ? "ping" : "pong",
            pong ? argv[2] : argv[3], pong ? argv[3] : argv[2],
#if defined(__aarch64__)
            "CNTVCT_EL0",
#else
            "TSC",
#endif
            ns_per_tick);
    if (ping)
    {
        run_ping(&w, messages, warmup, cpu);
    }
    else
    {
        run_pong(&w, cpu);
    }
    rte_eth_dev_stop(w.port);
    rte_eth_dev_close(w.port);
    rte_eal_cleanup();
    return 0;
}
