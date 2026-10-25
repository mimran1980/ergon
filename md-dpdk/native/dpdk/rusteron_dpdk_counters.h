/*
 * ergon-md-dpdk DPDK transport — Aeron counters (plan §9).
 *
 * A per-role-port counter set surfaced through the driver's counters manager.
 * The type IDs (75..=95) are the driver's reserved range; no other media-driver
 * code allocates in it. Every counter carries a label with the role, PCI BDF,
 * DPDK port, queue and direction so a cluster-admin dashboard can attribute a
 * value to a specific ENA without extra state.
 *
 * Hot path: `rusteron_dpdk_counters_add` is a single relaxed atomic add into a
 * value address resolved at registration — no lookup, no allocation, no syscall.
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
    aeron_counters_manager_t *manager; /* borrowed; read only to check a set is still held */
    int32_t id[RD_COUNTER_COUNT];
    int64_t *addr[RD_COUNTER_COUNT];

    /* Extended statistics (type 92): one Aeron counter per PMD xstat, cached at
     * registration so the 1 Hz sample is a plain store into pre-resolved
     * addresses (plan §9). */
    uint32_t xstat_count;
    rusteron_dpdk_xstat_t *xstats;
    char *xstat_names;           /* backing store for name pointers */
    uint32_t missed_xstat_index; /* xstat whose name contains "missed" (UINT32_MAX = none) */
    uint64_t last_sample_ns;
    uint64_t last_nic_sample_ns; /* the last read of the NIC's own statistics */
} rusteron_dpdk_counters_t;

/* Hot-path bump: a single relaxed atomic add into a pre-resolved value slot.
 * NULL-safe so a port without a counters manager (e.g. a NULL-context test) is
 * a no-op. */
static inline void rusteron_dpdk_counters_add(rusteron_dpdk_counters_t *c, int kind, int64_t n)
{
    if (NULL != c && NULL != c->addr[kind])
    {
        __atomic_add_fetch(c->addr[kind], n, __ATOMIC_RELAXED);
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

/* Refresh the mempool-availability counter at most once per second, and the
 * NIC's own statistics (basic stats and the PMD xstat mirrors) at most once
 * every `nic_stats_ns` (0: never; on ENA each read is an admin command to the
 * device). Called by the housekeeping thread, never the agents' hot path. */
void rusteron_dpdk_counters_sample(
    rusteron_dpdk_counters_t *c, const struct rusteron_dpdk_port_stct *port,
    const struct rusteron_dpdk_port_ops_stct *ops, uint64_t now_ns, uint64_t nic_stats_ns);

#ifdef __cplusplus
}
#endif

#endif /* RUSTERON_DPDK_COUNTERS_H */
