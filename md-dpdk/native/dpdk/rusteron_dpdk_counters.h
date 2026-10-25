/*
 * ergon-md-dpdk DPDK transport — Aeron counters (plan §9).
 *
 * A per-role-port counter set surfaced through the driver's counters manager.
 * The type IDs (75..=95) are the driver's reserved range; no other media-driver
 * code allocates in it. Every counter carries a label with the role, PCI BDF,
 * DPDK port, queue and direction so a cluster-admin dashboard can attribute a
 * value to a specific ENA without extra state.
 *
 * Hot path: `rusteron_dpdk_counters_add` is one add into a value address
 * resolved at registration — no lookup, no allocation, no syscall: a relaxed
 * load and a release store for a kind only its port's agent writes, a relaxed
 * atomic add for the few others write too (rusteron_dpdk_counter_shared).
 * Allocations happen only at registration (one pass over the PMD xstats) and on
 * the 1 Hz extended-statistics sample, which reads the PMD's values into the
 * pre-resolved slots.
 */
#ifndef RUSTERON_DPDK_COUNTERS_H
#define RUSTERON_DPDK_COUNTERS_H

#include <stddef.h>
#include <stdint.h>
#include <string.h>

#include "rusteron_dpdk_port_ops.h" /* RUSTERON_DPDK_XSTAT_NAME_LEN */

/* Forward-declared; counters.c includes the real header for the manager
 * functions. The hot-path helpers only touch int64_t* addresses. */
typedef struct aeron_counters_manager_stct aeron_counters_manager_t;
struct rusteron_dpdk_port_stct; /* defined in rusteron_dpdk_internal.h, which
                                   includes this header first */
struct rusteron_dpdk_event_ring_stct; /* rusteron_dpdk_events.h */

#ifdef __cplusplus
extern "C" {
#endif

/* Aeron counter type IDs (plan §9). */
#define RUSTERON_DPDK_TYPE_PORT_INFO                75
#define RUSTERON_DPDK_TYPE_TRANSPORT                76
#define RUSTERON_DPDK_TYPE_NOBUFS                   77
#define RUSTERON_DPDK_TYPE_TX_EAGAIN                78
#define RUSTERON_DPDK_TYPE_ERROR                    79
#define RUSTERON_DPDK_TYPE_CLOCK                    80
#define RUSTERON_DPDK_TYPE_RX_HW_DELAY              81
#define RUSTERON_DPDK_TYPE_PKTS                     82
#define RUSTERON_DPDK_TYPE_BYTES                    83
#define RUSTERON_DPDK_TYPE_MISSED_PACKETS           84
#define RUSTERON_DPDK_TYPE_ARP_MISS                 85
#define RUSTERON_DPDK_TYPE_RX_SENDER_DISCARD        86
#define RUSTERON_DPDK_TYPE_POLLER                   87
#define RUSTERON_DPDK_TYPE_QUEUE_DROP               88
#define RUSTERON_DPDK_TYPE_CHECKSUM_FAILURE         89
#define RUSTERON_DPDK_TYPE_FRAGMENTED               90
#define RUSTERON_DPDK_TYPE_MEMPOOL_AVAILABLE        91
#define RUSTERON_DPDK_TYPE_EXTENDED_STATS           92
#define RUSTERON_DPDK_TYPE_RX_UNSUPPORTED_ETHERTYPE 93
#define RUSTERON_DPDK_TYPE_RX_UNSUPPORTED_PROTOCOL  94
#define RUSTERON_DPDK_TYPE_RX_RECEIVER_DISCARD      95

/* A port with no configured extended stats is a no-op for the 1 Hz sample. */
#define RUSTERON_DPDK_MAX_XSTATS 256
#define RUSTERON_DPDK_XSTAT_SAMPLE_NS 1000000000ULL /* 1 s */

/* Fixed-counter kinds. Indexes into rusteron_dpdk_counters_t.id[]/addr[];
 * the type IDs, names and directions live in the table in counters.c. */
enum rusteron_dpdk_counter_kind
{
    RD_COUNTER_PORT_INFO = 0,        /* 75   port info (driver + MAC in label) */
    RD_COUNTER_TRANSPORT,            /* 76   open transports on this port */
    RD_COUNTER_TX_PKTS,              /* 82   datagrams sent */
    RD_COUNTER_TX_BYTES,             /* 83   payload bytes sent */
    RD_COUNTER_RX_PKTS,              /* 82   datagrams dispatched */
    RD_COUNTER_RX_BYTES,             /* 83   payload bytes dispatched */
    RD_COUNTER_NOBUFS,               /* 77   TX mbuf pool exhausted */
    RD_COUNTER_TX_EAGAIN,            /* 78   TX bursts truncated by NIC */
    RD_COUNTER_ERROR,                /* 79   rejected datagrams */
    RD_COUNTER_ARP_MISS,             /* 85   unresolved next hop */
    RD_COUNTER_DISCARD,              /* 86/95 role discard */
    RD_COUNTER_QUEUE_DROP,           /* 88   endpoint-map miss */
    RD_COUNTER_CHECKSUM,             /* 89   RX checksum failures */
    RD_COUNTER_FRAGMENTED,           /* 90   RX IP fragments */
    RD_COUNTER_MISSED_PACKETS,       /* 84   ENA missed RX (mirrored xstat) */
    RD_COUNTER_UNSUPPORTED_ETHERTYPE, /* 93   RX unsupported ethertype */
    RD_COUNTER_UNSUPPORTED_PROTOCOL, /* 94   RX unsupported IP protocol */
    RD_COUNTER_POLLER,               /* 87   frames processed per poll */
    RD_COUNTER_MEMPOOL_AVAILABLE,    /* 91   free mbufs (1 Hz) */
    RD_COUNTER_CLOCK,                /* 80   transport clock: 1 TSC, 2 CNTVCT_EL0, 0 monotonic */
    RD_COUNTER_RX_HW_DELAY_MAX,      /* 81   max NIC arrival -> the frame's dispatch, ns */
    RD_COUNTER_RX_HW_DELAY_SUM,      /* 81   sum of those delays, ns */
    RD_COUNTER_RX_HW_TIMESTAMPED,    /* 81   frames in that sum (NIC-stamped, not after dispatch) */
    RD_COUNTER_RX_POLL_GAP_MAX,      /* 87   longest gap before a poll that found frames, ns */
    RD_COUNTER_RX_POLL_GAPS_100US,   /* 87   such gaps of 100 µs or more */
    RD_COUNTER_RX_BURST_FULL,        /* 87   polls that drained a full burst (a backlog) */
    RD_COUNTER_MULTI_SEGMENT,        /* 90   RX frames larger than one mbuf */
    RD_COUNTER_PEER_MTU_TOO_LARGE,   /* 79   SETUPs from peers whose MTU exceeds ours, dropped */
    RD_COUNTER_NOT_CONNECTED_PEER,   /* 88   connected transport: datagram from another peer */
    RD_COUNTER_ICMP_ECHO,            /* 94   ICMP echo requests answered */
    RD_COUNTER_ICMP_UNREACH_SENT,    /* 94   ICMP port unreachable sent (rate-limited) */
    RD_COUNTER_ICMP_FRAG_NEEDED,     /* 94   ICMP fragmentation needed received (path MTU) */
    RD_COUNTER_ICMP_OTHER,           /* 94   other ICMP received */
    RD_COUNTER_NIC_RX_ERRORS,        /* 84   NIC ierrors (1 Hz, housekeeping) */
    RD_COUNTER_NIC_RX_MISSED,        /* 84   NIC imissed: ring full (1 Hz, housekeeping) */
    RD_COUNTER_NIC_RX_NOMBUF,        /* 84   NIC rx_nombuf: mempool empty (1 Hz, housekeeping) */
    RD_COUNTER_RX_SW_CHECKSUM,       /* 89   datagrams whose UDP checksum was verified in software */
    RD_COUNTER_TX_PACED,             /* 78   sends held to the bandwidth allowance (waited or deferred) */
    RD_COUNTER_POLL_CYCLE_GAP_MAX,   /* 87   longest gap between the port's polls, frames or not: the agent's loop, ns */
    RD_COUNTER_POLL_CYCLE_GAPS_20US, /* 87   such gaps of 20 to 50 µs */
    RD_COUNTER_POLL_CYCLE_GAPS_50US, /* 87   such gaps of 50 to 100 µs */
    RD_COUNTER_POLL_CYCLE_GAPS_100US, /* 87  such gaps of 100 µs or more */
    /* RUSTERON_DPDK_GAP_ATTRIB=1: the same gaps split by whether the thread
     * spent at least half of one on the CPU (work) or not (descheduling). 0
     * while attribution is off. */
    RD_COUNTER_POLL_CYCLE_GAPS_WORK,   /* 87   gaps of 20 µs or more the thread spent at least half of on the CPU */
    RD_COUNTER_POLL_CYCLE_GAPS_DESCHED, /* 87  gaps of 20 µs or more the thread spent less than half of on the CPU */
    RD_COUNTER_POLL_CYCLE_GAP_WORK_MAX,   /* 87 longest such gap that was work, ns */
    RD_COUNTER_POLL_CYCLE_GAP_DESCHED_MAX, /* 87 longest such gap that was descheduling, ns */
    RD_COUNTER_TX_QUEUED,            /* 78   frames the TX ring refused, queued to go first next */
    RD_COUNTER_RX_BACKLOG_MAX,       /* 87   most frames one poll drained */
    RD_COUNTER_RX_RING_FULL_SUSPECTED, /* 87 polls that drained within a burst of a full ring */
    RD_COUNTER_NIC_RESET_REQUESTED,  /* 79   device resets the PMD asked for (RTE_ETH_EVENT_INTR_RESET; housekeeping) */
    RD_COUNTER_RX_HW_DELAY_MIN,      /* 81   min NIC arrival -> the frame's dispatch, ns (meaningless while the count is 0) */
    RD_COUNTER_RX_HW_DELAY_NEGATIVE, /* 81   NIC stamps later than the host clock read at that dispatch: PHC/host clock skew */
    /* RUSTERON_DPDK_GAP_ATTRIB=1: how long the PMD's burst calls took (an
     * rx_burst that returned frames, every tx_burst). ENA refills its receive
     * ring and reclaims sent descriptors inside them. 0 while off. Each
     * direction's two counts follow its max (rusteron_dpdk_counters_burst). */
    RD_COUNTER_RX_BURST_MAX,         /* 87   longest rx_burst that returned frames, ns */
    RD_COUNTER_RX_BURSTS_1US,        /* 87   such bursts of 1 µs or more */
    RD_COUNTER_RX_BURSTS_5US,        /* 87   such bursts of 5 µs or more */
    RD_COUNTER_TX_BURST_MAX,         /* 87   longest tx_burst, ns */
    RD_COUNTER_TX_BURSTS_1US,        /* 87   tx_bursts of 1 µs or more */
    RD_COUNTER_TX_BURSTS_5US,        /* 87   tx_bursts of 5 µs or more */
    RD_COUNTER_NIC_LINK_CHANGES,     /* 79   link-status changes (RTE_ETH_EVENT_INTR_LSC, where the NIC raises it; housekeeping) */
    RD_COUNTER_TX_IDLE_CLEANED,      /* 87   sent descriptors reclaimed on idle polls (RUSTERON_DPDK_TX_IDLE_CLEAN) */
    RD_COUNTER_COUNT
};

/* One mirrored PMD extended stat (type 92). */
typedef struct rusteron_dpdk_xstat_stct
{
    int32_t id;      /* aeron counter id, or -1 */
    int64_t *addr;   /* resolved value slot, or NULL */
    char *name;      /* cached PMD name (points into xstat_names) */
} rusteron_dpdk_xstat_t;

/* Per-port counter set, embedded in rusteron_dpdk_port_t. */
typedef struct rusteron_dpdk_counters_stct
{
    int registered;              /* counters allocated for this port */
    int64_t token;               /* process-unique registration id stamped on them */
    int dormant;                 /* the port's last transport closed: kept, never written */
    /* Moves on whenever the set goes dormant or is forgotten (housekeeping_lock):
     * a NIC read begun before cannot be stored into what it has become. */
    uint32_t generation;
    aeron_counters_manager_t *manager; /* borrowed; read only to check a set is still held */
    int32_t id[RD_COUNTER_COUNT];
    /* Read by the agents on every bump: on lines of their own, away from the
     * fields the conductor writes at registration and close above. */
    int64_t *addr[RD_COUNTER_COUNT] __attribute__((aligned(64)));

    /* Extended statistics (type 92): one Aeron counter per PMD xstat, cached at
     * registration so the 1 Hz sample is a plain store into pre-resolved
     * addresses (plan §9). */
    uint32_t xstat_count;
    rusteron_dpdk_xstat_t *xstats;
    char *xstat_names;           /* backing store for name pointers */
    uint32_t missed_xstat_index; /* xstat whose name contains "missed" (UINT32_MAX = none) */
    /* Written by housekeeping once a second: a line of its own (the set's
     * size rounds up to whole lines, so nothing after it shares it either). */
    uint64_t last_sample_ns __attribute__((aligned(64)));
    uint64_t last_nic_sample_ns; /* the last read of the NIC's own statistics */
    uint64_t last_slow_sample_ns; /* the last full read, once RUSTERON_DPDK_NIC_SLOW_STATS_MS splits them */
    /* The xstats the PMD keeps in software (ENA: its queue stats, wd_expired,
     * dev_start, dev_stop, tx_drops), by id: what a split read takes. */
    uint32_t software_xstat_count;
    uint16_t *software_xstats;    /* allocated with xstats; housekeeping only */
} rusteron_dpdk_counters_t;

/* RUSTERON_DPDK_NIC_SLOW_STATS_MS unset: every NIC read is a full one. */
#define RUSTERON_DPDK_NIC_STATS_UNSPLIT UINT64_MAX

/* Housekeeping writes last_sample_ns and last_nic_sample_ns once a second;
 * the agents read addr[] on every counter bump. On one cache line, each of
 * those writes would cost the agents a miss: the two never share one, the set
 * aligned so the offsets are the lines. */
_Static_assert(_Alignof(rusteron_dpdk_counters_t) >= 64, "the counter set starts a cache line");
_Static_assert(offsetof(rusteron_dpdk_counters_t, last_sample_ns) / 64 >
                   (offsetof(rusteron_dpdk_counters_t, addr) + sizeof(((rusteron_dpdk_counters_t *)0)->addr) - 1) / 64,
               "housekeeping's sample times share no cache line with the agents' addr[]");

/* Whether a thread other than the port's agent also bumps `kind`, so its add
 * must stay an atomic read-modify-write. Each port's poller and TX queue are
 * one agent's (DEDICATED's sender or receiver; the one network agent
 * otherwise), so every kind bumped on the send and receive paths has that one
 * writer. tests/dpdk_counter_writers.rs derives each bump's threads from these
 * sources and fails unless this lists exactly the kinds another thread writes:
 *   - the open transports, which the conductor's init and close move. */
static inline int rusteron_dpdk_counter_shared(int kind)
{
    switch (kind)
    {
        case RD_COUNTER_TRANSPORT:
            return 1;
        default:
            return 0;
    }
}

/* Hot-path bump into a pre-resolved value slot: for a kind with one writer
 * (rusteron_dpdk_counter_shared), a relaxed load and a release store, which no
 * reader can see torn; for a shared one, a relaxed atomic add. `kind` is a
 * constant at every call, so the choice folds away. NULL-safe so a port
 * without a counters manager (e.g. a NULL-context test) is a no-op. */
static inline void rusteron_dpdk_counters_add(rusteron_dpdk_counters_t *c, int kind, int64_t n)
{
    if (NULL != c && NULL != c->addr[kind])
    {
        if (rusteron_dpdk_counter_shared(kind))
        {
            __atomic_add_fetch(c->addr[kind], n, __ATOMIC_RELAXED);
        }
        else
        {
            __atomic_store_n(c->addr[kind], __atomic_load_n(c->addr[kind], __ATOMIC_RELAXED) + n, __ATOMIC_RELEASE);
        }
    }
}

/* Raise a counter to `v` if it is larger. Each port's counters have a single
 * writer (the agent polling it), so a relaxed load and store suffice. */
static inline void rusteron_dpdk_counters_max(rusteron_dpdk_counters_t *c, int kind, int64_t v)
{
    if (NULL != c && NULL != c->addr[kind] && v > __atomic_load_n(c->addr[kind], __ATOMIC_RELAXED))
    {
        __atomic_store_n(c->addr[kind], v, __ATOMIC_RELAXED);
    }
}

/* A PMD burst call of `ns` (RUSTERON_DPDK_GAP_ATTRIB=1): the direction's
 * longest, and its counts of 1 µs or more and 5 µs or more, the two kinds
 * after `max_kind`. */
static inline void rusteron_dpdk_counters_burst(rusteron_dpdk_counters_t *c, int max_kind, uint64_t ns)
{
    rusteron_dpdk_counters_max(c, max_kind, (int64_t)ns);
    if (ns >= 1000)
    {
        rusteron_dpdk_counters_add(c, max_kind + 1, 1);
        if (ns >= 5000)
        {
            rusteron_dpdk_counters_add(c, max_kind + 2, 1);
        }
    }
}

/* Lower a counter to `v` if it is smaller, or set it when the counter `count`
 * (the samples behind it) is still 0, so a fresh minimum needs no sentinel:
 * it reads 0 until its first sample. Single writer, as for the max. */
static inline void rusteron_dpdk_counters_min_counted(rusteron_dpdk_counters_t *c, int kind, int64_t v, int count)
{
    if (NULL == c || NULL == c->addr[kind])
    {
        return;
    }
    const int first = NULL == c->addr[count] || 0 == __atomic_load_n(c->addr[count], __ATOMIC_RELAXED);
    if (first || v < __atomic_load_n(c->addr[kind], __ATOMIC_RELAXED))
    {
        __atomic_store_n(c->addr[kind], v, __ATOMIC_RELAXED);
    }
}

/* Allocate every fixed counter plus the PMD xstat mirrors for `port` from
 * `manager`. Best effort: a missing counters manager (or a NULL context) is a
 * no-op returning 0, and a failed single allocation leaves that counter at -1
 * (its bumps are then no-ops) without failing the transport. */
int rusteron_dpdk_counters_register(
    rusteron_dpdk_counters_t *c, aeron_counters_manager_t *manager,
    const struct rusteron_dpdk_port_stct *port, const struct rusteron_dpdk_port_ops_stct *ops);

/* Drop the local view of the counters without touching the manager (it may
 * already be gone with the driver): every later bump and sample is a no-op. */
void rusteron_dpdk_counters_forget(rusteron_dpdk_counters_t *c);

/* One read of a NIC's own statistics, carried between housekeeping's two
 * short holds of its lock: the read itself, on ENA admin commands to the
 * device, runs with the lock released, so closing a transport never waits
 * behind it. */
typedef struct rusteron_dpdk_nic_sample_stct
{
    int due;                     /* a read is due: the rest is set */
    uint16_t port_id;
    uint32_t xstat_count;        /* the set's PMD xstats when it was copied */
    uint32_t generation;         /* the set's generation then */
    /* A full read (basic statistics and every xstat), or, split, only the
     * `id_count` software xstats `ids` names, by id. */
    int full;
    uint32_t id_count;
    uint64_t ids[RUSTERON_DPDK_MAX_XSTATS];
    int basic_ok;                /* basic_stats answered */
    uint64_t ierrors, imissed, rx_nombuf;
    int xstats_ok;               /* xstats_get answered (split: xstats_get_by_id, into values[ids[i]]) */
    uint64_t values[RUSTERON_DPDK_MAX_XSTATS];
} rusteron_dpdk_nic_sample_t;

/* Refresh the mempool-availability counter at most once per second, and say
 * in `nic` whether the NIC's own statistics (basic stats and the PMD xstat
 * mirrors) are due, at most once every `nic_stats_ns` (0: never; on ENA each
 * read is an admin command to the device), copying what the read needs.
 * With `nic_slow_ns` other than RUSTERON_DPDK_NIC_STATS_UNSPLIT the reads
 * split: every `nic_stats_ns` only the PMD's software xstats, by id; every
 * `nic_slow_ns` (0: never) everything.
 * Called by the housekeeping thread under its lock, never the agents' hot
 * path; with the event log open (`events`, housekeeping's ring; else NULL)
 * the mempool read is logged with how long it took. */
void rusteron_dpdk_counters_sample(
    rusteron_dpdk_counters_t *c, const struct rusteron_dpdk_port_stct *port,
    const struct rusteron_dpdk_port_ops_stct *ops, uint64_t now_ns, uint64_t nic_stats_ns, uint64_t nic_slow_ns,
    struct rusteron_dpdk_event_ring_stct *events, rusteron_dpdk_nic_sample_t *nic);

/* Read the NIC statistics `nic` says are due, into `nic`: housekeeping,
 * without its lock. Logged with how long it took when `events` is open. */
void rusteron_dpdk_counters_read_nic(
    const struct rusteron_dpdk_port_ops_stct *ops, rusteron_dpdk_nic_sample_t *nic,
    struct rusteron_dpdk_event_ring_stct *events);

/* Store what `nic` read into the set, under housekeeping's lock, unless the
 * set went dormant or was replaced since it was copied (its generation). */
void rusteron_dpdk_counters_store_nic(rusteron_dpdk_counters_t *c, const rusteron_dpdk_nic_sample_t *nic);

#ifdef __cplusplus
}
#endif

#endif /* RUSTERON_DPDK_COUNTERS_H */
