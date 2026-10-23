/*
 * ergon-md-dpdk DPDK transport — Aeron counters (plan §9).
 *
 * Registration resolves every counter's value address up front; the hot path
 * (transport.c / poller.c) only ever calls the inline `rusteron_dpdk_counters_add`.
 * Extended statistics are cached at registration (names for the labels, ids for
 * the sample) and refreshed at most once per second by
 * `rusteron_dpdk_counters_sample`, which the housekeeping thread calls, never
 * an agent.
 */
#include "rusteron_dpdk_internal.h" /* port struct + rusteron_dpdk_clock_ns */
#include "rusteron_dpdk_counters.h"

#include <inttypes.h>
#include <sys/random.h>
#include <stdio.h>
#include <string.h>

#include "concurrent/aeron_counters_manager.h"
#include "aeron_alloc.h"

/* Fixed-counter metadata: type ID, base name and direction for the label. The
 * discard counter's type ID is role-dependent (86 sender / 95 receiver). */
typedef struct rusteron_dpdk_counter_spec_stct
{
    int32_t type_id;
    const char *name;
    const char *dir;
} rusteron_dpdk_counter_spec_t;

static const rusteron_dpdk_counter_spec_t rusteron_dpdk_counter_specs[RD_COUNTER_COUNT] =
{
    [RD_COUNTER_PORT_INFO] = { RUSTERON_DPDK_TYPE_PORT_INFO, "port-info", "-" },
    [RD_COUNTER_TRANSPORT] = { RUSTERON_DPDK_TYPE_TRANSPORT, "transport", "-" },
    [RD_COUNTER_TX_PKTS] = { RUSTERON_DPDK_TYPE_PKTS, "pkts", "tx" },
    [RD_COUNTER_TX_BYTES] = { RUSTERON_DPDK_TYPE_BYTES, "bytes", "tx" },
    [RD_COUNTER_RX_PKTS] = { RUSTERON_DPDK_TYPE_PKTS, "pkts", "rx" },
    [RD_COUNTER_RX_BYTES] = { RUSTERON_DPDK_TYPE_BYTES, "bytes", "rx" },
    [RD_COUNTER_NOBUFS] = { RUSTERON_DPDK_TYPE_NOBUFS, "nobufs", "tx" },
    [RD_COUNTER_TX_EAGAIN] = { RUSTERON_DPDK_TYPE_TX_EAGAIN, "tx-eagain", "tx" },
    [RD_COUNTER_ERROR] = { RUSTERON_DPDK_TYPE_ERROR, "error", "-" },
    [RD_COUNTER_ARP_MISS] = { RUSTERON_DPDK_TYPE_ARP_MISS, "arp-miss", "tx" },
    [RD_COUNTER_DISCARD] = { RUSTERON_DPDK_TYPE_RX_RECEIVER_DISCARD, "discard", "rx" },
    [RD_COUNTER_QUEUE_DROP] = { RUSTERON_DPDK_TYPE_QUEUE_DROP, "queue-drop", "rx" },
    [RD_COUNTER_CHECKSUM] = { RUSTERON_DPDK_TYPE_CHECKSUM_FAILURE, "checksum-failure", "rx" },
    [RD_COUNTER_FRAGMENTED] = { RUSTERON_DPDK_TYPE_FRAGMENTED, "fragmented", "rx" },
    [RD_COUNTER_MISSED_PACKETS] = { RUSTERON_DPDK_TYPE_MISSED_PACKETS, "missed-packets", "rx" },
    [RD_COUNTER_UNSUPPORTED_ETHERTYPE] =
        { RUSTERON_DPDK_TYPE_RX_UNSUPPORTED_ETHERTYPE, "unsupported-ethertype", "rx" },
    [RD_COUNTER_UNSUPPORTED_PROTOCOL] =
        { RUSTERON_DPDK_TYPE_RX_UNSUPPORTED_PROTOCOL, "unsupported-protocol", "rx" },
    [RD_COUNTER_POLLER] = { RUSTERON_DPDK_TYPE_POLLER, "poller", "rx" },
    [RD_COUNTER_MEMPOOL_AVAILABLE] =
        { RUSTERON_DPDK_TYPE_MEMPOOL_AVAILABLE, "mempool-available", "-" },
    [RD_COUNTER_CLOCK] = { RUSTERON_DPDK_TYPE_CLOCK, "clock", "-" },
    [RD_COUNTER_RX_HW_DELAY_MAX] = { RUSTERON_DPDK_TYPE_RX_HW_DELAY, "nic-arrival-to-rx-burst-max-ns", "rx" },
    [RD_COUNTER_RX_HW_DELAY_SUM] = { RUSTERON_DPDK_TYPE_RX_HW_DELAY, "nic-arrival-to-rx-burst-sum-ns", "rx" },
    [RD_COUNTER_RX_HW_TIMESTAMPED] = { RUSTERON_DPDK_TYPE_RX_HW_DELAY, "nic-arrival-to-rx-burst-count", "rx" },
    [RD_COUNTER_RX_POLL_GAP_MAX] = { RUSTERON_DPDK_TYPE_POLLER, "rx-poll-gap-max-ns", "rx" },
    [RD_COUNTER_RX_POLL_GAPS_100US] = { RUSTERON_DPDK_TYPE_POLLER, "rx-poll-gaps-over-100us", "rx" },
    [RD_COUNTER_RX_BURST_FULL] = { RUSTERON_DPDK_TYPE_POLLER, "rx-burst-full", "rx" },
    [RD_COUNTER_MULTI_SEGMENT] = { RUSTERON_DPDK_TYPE_FRAGMENTED, "multi-segment", "rx" },
    [RD_COUNTER_PEER_MTU_TOO_LARGE] = { RUSTERON_DPDK_TYPE_ERROR, "peer-mtu-too-large", "rx" },
    [RD_COUNTER_NOT_CONNECTED_PEER] = { RUSTERON_DPDK_TYPE_QUEUE_DROP, "not-connected-peer", "rx" },
    [RD_COUNTER_ICMP_ECHO] = { RUSTERON_DPDK_TYPE_RX_UNSUPPORTED_PROTOCOL, "icmp-echo", "rx" },
    [RD_COUNTER_ICMP_UNREACH_SENT] = { RUSTERON_DPDK_TYPE_RX_UNSUPPORTED_PROTOCOL, "icmp-port-unreachable-sent", "tx" },
    [RD_COUNTER_ICMP_FRAG_NEEDED] = { RUSTERON_DPDK_TYPE_RX_UNSUPPORTED_PROTOCOL, "icmp-frag-needed", "rx" },
    [RD_COUNTER_ICMP_OTHER] = { RUSTERON_DPDK_TYPE_RX_UNSUPPORTED_PROTOCOL, "icmp-other", "rx" },
    [RD_COUNTER_NIC_RX_ERRORS] = { RUSTERON_DPDK_TYPE_MISSED_PACKETS, "nic-rx-errors", "rx" },
    [RD_COUNTER_NIC_RX_MISSED] = { RUSTERON_DPDK_TYPE_MISSED_PACKETS, "nic-rx-missed", "rx" },
    [RD_COUNTER_NIC_RX_NOMBUF] = { RUSTERON_DPDK_TYPE_MISSED_PACKETS, "nic-rx-nombuf", "rx" },
    [RD_COUNTER_RX_SW_CHECKSUM] = { RUSTERON_DPDK_TYPE_CHECKSUM_FAILURE, "sw-checksum-verified", "rx" },
    [RD_COUNTER_TX_PACED] = { RUSTERON_DPDK_TYPE_TX_EAGAIN, "tx-paced", "tx" },
    [RD_COUNTER_POLL_CYCLE_GAP_MAX] = { RUSTERON_DPDK_TYPE_POLLER, "poll-cycle-gap-max-ns", "rx" },
    [RD_COUNTER_POLL_CYCLE_GAPS_20US] = { RUSTERON_DPDK_TYPE_POLLER, "poll-cycle-gaps-20-50us", "rx" },
    [RD_COUNTER_POLL_CYCLE_GAPS_50US] = { RUSTERON_DPDK_TYPE_POLLER, "poll-cycle-gaps-50-100us", "rx" },
    [RD_COUNTER_POLL_CYCLE_GAPS_100US] = { RUSTERON_DPDK_TYPE_POLLER, "poll-cycle-gaps-over-100us", "rx" },
    [RD_COUNTER_POLL_CYCLE_GAPS_WORK] = { RUSTERON_DPDK_TYPE_POLLER, "poll-cycle-gaps-work", "rx" },
    [RD_COUNTER_POLL_CYCLE_GAPS_DESCHED] = { RUSTERON_DPDK_TYPE_POLLER, "poll-cycle-gaps-desched", "rx" },
    [RD_COUNTER_POLL_CYCLE_GAP_WORK_MAX] = { RUSTERON_DPDK_TYPE_POLLER, "poll-cycle-gap-work-max-ns", "rx" },
    [RD_COUNTER_POLL_CYCLE_GAP_DESCHED_MAX] = { RUSTERON_DPDK_TYPE_POLLER, "poll-cycle-gap-desched-max-ns", "rx" },
    [RD_COUNTER_TX_QUEUED] = { RUSTERON_DPDK_TYPE_TX_EAGAIN, "tx-queued", "tx" },
    [RD_COUNTER_RX_BACKLOG_MAX] = { RUSTERON_DPDK_TYPE_POLLER, "rx-backlog-max", "rx" },
    [RD_COUNTER_RX_RING_FULL_SUSPECTED] = { RUSTERON_DPDK_TYPE_POLLER, "rx-ring-full-suspected", "rx" },
    [RD_COUNTER_NIC_RESET_REQUESTED] = { RUSTERON_DPDK_TYPE_ERROR, "nic-reset-requested", "-" },
    [RD_COUNTER_RX_HW_DELAY_MIN] = { RUSTERON_DPDK_TYPE_RX_HW_DELAY, "nic-arrival-to-rx-burst-min-ns", "rx" },
    [RD_COUNTER_RX_HW_DELAY_NEGATIVE] = { RUSTERON_DPDK_TYPE_RX_HW_DELAY, "nic-arrival-to-rx-burst-negative", "rx" },
    [RD_COUNTER_RX_BURST_MAX] = { RUSTERON_DPDK_TYPE_POLLER, "rx-burst-max-ns", "rx" },
    [RD_COUNTER_RX_BURSTS_1US] = { RUSTERON_DPDK_TYPE_POLLER, "rx-bursts-over-1us", "rx" },
    [RD_COUNTER_RX_BURSTS_5US] = { RUSTERON_DPDK_TYPE_POLLER, "rx-bursts-over-5us", "rx" },
    [RD_COUNTER_TX_BURST_MAX] = { RUSTERON_DPDK_TYPE_POLLER, "tx-burst-max-ns", "tx" },
    [RD_COUNTER_TX_BURSTS_1US] = { RUSTERON_DPDK_TYPE_POLLER, "tx-bursts-over-1us", "tx" },
    [RD_COUNTER_TX_BURSTS_5US] = { RUSTERON_DPDK_TYPE_POLLER, "tx-bursts-over-5us", "tx" },
    [RD_COUNTER_NIC_LINK_CHANGES] = { RUSTERON_DPDK_TYPE_ERROR, "nic-link-changes", "-" },
    [RD_COUNTER_TX_IDLE_CLEANED] = { RUSTERON_DPDK_TYPE_POLLER, "tx-idle-cleaned", "tx" },
};
_Static_assert(RD_COUNTER_RX_BURSTS_1US == RD_COUNTER_RX_BURST_MAX + 1 &&
                   RD_COUNTER_RX_BURSTS_5US == RD_COUNTER_RX_BURST_MAX + 2 &&
                   RD_COUNTER_TX_BURSTS_1US == RD_COUNTER_TX_BURST_MAX + 1 &&
                   RD_COUNTER_TX_BURSTS_5US == RD_COUNTER_TX_BURST_MAX + 2,
               "each burst max is followed by its two counts (rusteron_dpdk_counters_burst)");

static void rusteron_dpdk_counters_label(
    char *label, size_t label_len, const rusteron_dpdk_port_t *port,
    const char *name, const char *dir)
{
    const char *role = port->role == RUSTERON_DPDK_ROLE_SENDER ? "sender" : "receiver";
    int32_t queue = (dir[0] == 't') ? port->tx_queue_id : port->rx_queue_id;
    snprintf(
        label, label_len, "rusteron-dpdk role=%s bdf=%s port=%u q=%" PRId32 " dir=%s %s",
        role, port->pci, (unsigned)port->port_id, queue, dir, name);
}

/* One of ethdev's basic statistics as xstats (rte_eth_stats_get, on ENA an
 * admin command): the eight fixed ones and the per-queue rx_q<n>_packets,
 * _bytes, _errors and tx_q<n>_packets, _bytes. */
static int rusteron_dpdk_xstat_is_basic(const char *name)
{
    static const char *const fixed[] = {
        "rx_good_packets", "tx_good_packets", "rx_good_bytes", "tx_good_bytes",
        "rx_missed_errors", "rx_errors", "tx_errors", "rx_mbuf_allocation_errors",
    };
    for (size_t i = 0; i < sizeof(fixed) / sizeof(fixed[0]); i++)
    {
        if (0 == strcmp(name, fixed[i]))
        {
            return 1;
        }
    }
    const int rx = 0 == strncmp(name, "rx_q", 4);
    if (!rx && 0 != strncmp(name, "tx_q", 4))
    {
        return 0;
    }
    const char *p = name + 4;
    if (*p < '0' || *p > '9')
    {
        return 0;
    }
    while (*p >= '0' && *p <= '9')
    {
        p++;
    }
    return 0 == strcmp(p, "_packets") || 0 == strcmp(p, "_bytes") || (rx && 0 == strcmp(p, "_errors"));
}

/* A PMD xstat kept in software, read without a device command: ENA's queue
 * stats (rx_q<n>_*, tx_q<n>_*) and its global ones. Its ENI allowances, SRD
 * and customer metrics are admin commands. */
static int rusteron_dpdk_xstat_is_software(const char *name)
{
    static const char *const global[] = { "wd_expired", "dev_start", "dev_stop", "tx_drops" };
    for (size_t i = 0; i < sizeof(global) / sizeof(global[0]); i++)
    {
        if (0 == strcmp(name, global[i]))
        {
            return 1;
        }
    }
    if (0 != strncmp(name, "rx_q", 4) && 0 != strncmp(name, "tx_q", 4))
    {
        return 0;
    }
    const char *p = name + 4;
    if (*p < '0' || *p > '9')
    {
        return 0;
    }
    while (*p >= '0' && *p <= '9')
    {
        p++;
    }
    return '_' == *p;
}

/* The software xstats among `n` names: past ethdev's basic block, which
 * leads (rte_eth_xstats_get_names lists it first), so its per-queue names,
 * which look like a PMD's, are never taken for one. */
static void rusteron_dpdk_counters_software_xstats(rusteron_dpdk_counters_t *c, const char *names, uint32_t n)
{
    uint32_t i = 0;
    while (i < n && rusteron_dpdk_xstat_is_basic(names + ((size_t)i * RUSTERON_DPDK_XSTAT_NAME_LEN)))
    {
        i++;
    }
    c->software_xstat_count = 0;
    if (NULL == c->software_xstats)
    {
        return;
    }
    for (; i < n; i++)
    {
        if (rusteron_dpdk_xstat_is_software(names + ((size_t)i * RUSTERON_DPDK_XSTAT_NAME_LEN)))
        {
            c->software_xstats[c->software_xstat_count++] = (uint16_t)i;
        }
    }
}

/* Cache the PMD's extended stats: one Aeron counter (type 92) per xstat, names
 * copied for the labels, plus an initial sample so the counters read a value
 * before the first 1 Hz tick. */
static void rusteron_dpdk_counters_cache_xstats(
    rusteron_dpdk_counters_t *c, const rusteron_dpdk_port_t *port,
    const rusteron_dpdk_port_ops_t *ops)
{
    if (NULL == ops || NULL == ops->xstats_count || NULL == ops->xstats_names ||
        NULL == ops->xstats_get)
    {
        return;
    }

    const uint32_t n = ops->xstats_count(port->port_id);
    if (n == 0 || n > RUSTERON_DPDK_MAX_XSTATS)
    {
        return;
    }

    const size_t name_bytes = (size_t)n * RUSTERON_DPDK_XSTAT_NAME_LEN;
    char *names = NULL;
    rusteron_dpdk_xstat_t *xstats = NULL;
    if (aeron_alloc((void **)&names, name_bytes) < 0 ||
        aeron_alloc((void **)&xstats, (size_t)n * sizeof(rusteron_dpdk_xstat_t)) < 0)
    {
        aeron_free(names);
        aeron_free(xstats);
        return;
    }
    memset(names, 0, name_bytes);
    memset(xstats, 0, (size_t)n * sizeof(rusteron_dpdk_xstat_t));

    if (ops->xstats_names(port->port_id, names, n) < 0)
    {
        aeron_free(names);
        aeron_free(xstats);
        return;
    }

    uint64_t values[RUSTERON_DPDK_MAX_XSTATS];
    if (ops->xstats_get(port->port_id, values, n) != 0)
    {
        aeron_free(names);
        aeron_free(xstats);
        return;
    }

    for (uint32_t i = 0; i < n; i++)
    {
        xstats[i].name = names + ((size_t)i * RUSTERON_DPDK_XSTAT_NAME_LEN);
        xstats[i].id = aeron_counters_manager_allocate(
            c->manager, RUSTERON_DPDK_TYPE_EXTENDED_STATS, NULL, 0,
            xstats[i].name, strlen(xstats[i].name));
        if (xstats[i].id >= 0)
        {
            xstats[i].addr = aeron_counters_manager_addr(c->manager, xstats[i].id);
            __atomic_store_n(xstats[i].addr, (int64_t)values[i], __ATOMIC_RELAXED);
        }
        /* ENA missed RX packets (type 84): mirror whichever xstat names itself
         * "missed" (the ENA PMD exposes rx_missed / ena_*_rx_missed variants). */
        if (UINT32_MAX == c->missed_xstat_index &&
            NULL != strstr(xstats[i].name, "missed"))
        {
            c->missed_xstat_index = i;
        }
    }

    c->xstat_count = n;
    c->xstats = xstats;
    c->xstat_names = names;
    /* best effort: without it the reads are never split (all full) */
    if (aeron_alloc((void **)&c->software_xstats, (size_t)n * sizeof(uint16_t)) < 0)
    {
        c->software_xstats = NULL;
    }
    rusteron_dpdk_counters_software_xstats(c, names, n);
}

/* Registration ids for this process's counter sets: unique across drivers
 * and processes (a random start), so a set is recognised by its id alone. */
static int64_t rusteron_dpdk_counters_next_token = 0;

/* Whether `manager` still holds this set as allocated: its first counter
 * carries our token. Only the live manager passed in is read, so a manager
 * since closed, even one replaced at the same address, is never mistaken
 * for ours. */
static int rusteron_dpdk_counters_still_held(const rusteron_dpdk_counters_t *c, const aeron_counters_manager_t *manager)
{
    const int32_t id = c->id[0];
    if (id < 0 || id > manager->max_counter_id ||
        (size_t)(id + 1) * AERON_COUNTERS_MANAGER_METADATA_LENGTH > manager->metadata_length ||
        (size_t)(id + 1) * AERON_COUNTERS_MANAGER_VALUE_LENGTH > manager->values_length)
    {
        return 0;
    }
    const aeron_counter_metadata_descriptor_t *md =
        (const aeron_counter_metadata_descriptor_t *)(manager->metadata + (size_t)id * AERON_COUNTERS_MANAGER_METADATA_LENGTH);
    const aeron_counter_value_descriptor_t *vd =
        (const aeron_counter_value_descriptor_t *)(manager->values + (size_t)id * AERON_COUNTERS_MANAGER_VALUE_LENGTH);
    return AERON_COUNTER_RECORD_ALLOCATED == __atomic_load_n(&md->state, __ATOMIC_ACQUIRE) &&
        c->token == __atomic_load_n(&vd->registration_id, __ATOMIC_ACQUIRE);
}

int rusteron_dpdk_counters_register(
    rusteron_dpdk_counters_t *c, aeron_counters_manager_t *manager,
    const rusteron_dpdk_port_t *port, const rusteron_dpdk_port_ops_t *ops)
{
    char label[256];

    if (NULL == c || NULL == port)
    {
        return -1;
    }
    if (NULL == manager)
    {
        return 0;
    }
    if (c->registered)
    {
        /* Registered by an open transport, or left dormant by a last close:
         * reuse them while this manager still holds them; otherwise they
         * belonged to a manager since closed. */
        if (c->manager == manager && rusteron_dpdk_counters_still_held(c, manager))
        {
            c->dormant = 0;
            return 0;
        }
        rusteron_dpdk_counters_forget(c);
    }
    if (0 == __atomic_load_n(&rusteron_dpdk_counters_next_token, __ATOMIC_RELAXED))
    {
        int64_t expected = 0;
        int64_t start = 0;
        if (sizeof(start) != getrandom(&start, sizeof(start), 0))
        {
            start = (int64_t)rusteron_dpdk_clock_now();
        }
        start = (start & INT64_C(0x3fffffffffffffff)) | INT64_C(0x4000000000000000); /* never 0 or negative */
        __atomic_compare_exchange_n(
            &rusteron_dpdk_counters_next_token, &expected, start, 0, __ATOMIC_RELAXED, __ATOMIC_RELAXED);
    }
    c->token = __atomic_add_fetch(&rusteron_dpdk_counters_next_token, 1, __ATOMIC_RELAXED);

    c->manager = manager;
    c->missed_xstat_index = UINT32_MAX;

    for (int32_t kind = 0; kind < RD_COUNTER_COUNT; kind++)
    {
        int32_t type_id = rusteron_dpdk_counter_specs[kind].type_id;
        const char *name = rusteron_dpdk_counter_specs[kind].name;
        const char *dir = rusteron_dpdk_counter_specs[kind].dir;

        if (RD_COUNTER_RX_RING_FULL_SUSPECTED == kind && UINT32_MAX == port->rx_ring_full_at)
        {
            /* an ENA counts a full ring's drops itself (nic-rx-missed) */
            c->id[kind] = -1;
            c->addr[kind] = NULL;
            continue;
        }

        if (RD_COUNTER_DISCARD == kind)
        {
            type_id = (port->role == RUSTERON_DPDK_ROLE_SENDER)
                ? RUSTERON_DPDK_TYPE_RX_SENDER_DISCARD
                : RUSTERON_DPDK_TYPE_RX_RECEIVER_DISCARD;
        }

        if (RD_COUNTER_PORT_INFO == kind)
        {
            /* port-info carries the driver + MAC in its label, the receive
             * ring it took (the NIC's default unless configured) and, on an
             * ENA, whether its LLQ BAR was seen write-combined. */
            static const char *const llq[] = { "", " llq=wc", " llq=uncached", " llq=unseen", " llq=unknown",
                                               " llq=off" };
            rusteron_dpdk_counters_label(label, sizeof(label), port, name, dir);
            size_t off = strlen(label);
            snprintf(
                label + off, sizeof(label) - off,
                " driver=%s mac=%02x:%02x:%02x:%02x:%02x:%02x rx-desc=%u%s",
                port->driver[0] != '\0' ? port->driver : "-",
                port->mac[0], port->mac[1], port->mac[2],
                port->mac[3], port->mac[4], port->mac[5], (unsigned)port->rx_descriptors,
                port->ena_llq >= 0 && port->ena_llq <= RUSTERON_DPDK_ENA_LLQ_DISABLED ? llq[port->ena_llq] : "");
            /* The queue tuning, where it is not the PMD's default. */
            static const char *const fast_free[] = { "", " tx-fast-free=on", " tx-fast-free=unsupported" };
            off = strlen(label);
            snprintf(label + off, sizeof(label) - off, "%s",
                     port->tx_fast_free >= 0 && port->tx_fast_free <= RUSTERON_DPDK_FAST_FREE_UNSUPPORTED
                         ? fast_free[port->tx_fast_free]
                         : "");
            if (0 != port->rx_free_thresh || 0 != port->tx_free_thresh)
            {
                off = strlen(label);
                snprintf(label + off, sizeof(label) - off, " rx-free-thresh=%u tx-free-thresh=%u",
                         (unsigned)port->rx_free_thresh, (unsigned)port->tx_free_thresh);
            }
            /* idle TX cleanup: its budget, or that the PMD lacks the op */
            if (0 != port->tx_idle_clean_asked)
            {
                off = strlen(label);
                if (0 != port->tx_idle_clean)
                {
                    snprintf(label + off, sizeof(label) - off, " tx-idle-clean=%u", (unsigned)port->tx_idle_clean);
                }
                else
                {
                    snprintf(label + off, sizeof(label) - off, " tx-idle-clean=unsupported");
                }
            }
            /* nic-link-changes stays 0 on a NIC that cannot raise them */
            if (!port->lsc_on)
            {
                off = strlen(label);
                snprintf(label + off, sizeof(label) - off, " lsc=off");
            }
        }
        else
        {
            rusteron_dpdk_counters_label(label, sizeof(label), port, name, dir);
        }

        c->id[kind] = aeron_counters_manager_allocate(
            c->manager, type_id, NULL, 0, label, strlen(label));
        c->addr[kind] = c->id[kind] >= 0
            ? aeron_counters_manager_addr(c->manager, c->id[kind])
            : NULL;
        if (c->id[kind] >= 0)
        {
            aeron_counters_manager_counter_registration_id(c->manager, c->id[kind], c->token);
        }
    }

    rusteron_dpdk_counters_cache_xstats(c, port, ops);

    if (NULL != ops && NULL != ops->mempool_avail && NULL != port->mempool &&
        NULL != c->addr[RD_COUNTER_MEMPOOL_AVAILABLE])
    {
        __atomic_store_n(
            c->addr[RD_COUNTER_MEMPOOL_AVAILABLE],
            (int64_t)ops->mempool_avail(port->mempool), __ATOMIC_RELAXED);
    }
    if (NULL != c->addr[RD_COUNTER_PORT_INFO])
    {
        __atomic_store_n(c->addr[RD_COUNTER_PORT_INFO], 1, __ATOMIC_RELAXED);
    }

    if (NULL != c->addr[RD_COUNTER_CLOCK])
    {
        __atomic_store_n(c->addr[RD_COUNTER_CLOCK], (int64_t)rusteron_dpdk_clock_source(), __ATOMIC_RELAXED);
    }
    /* Reset requests and link changes the port saw before this set existed
     * (housekeeping keeps them current while a transport is open). */
    if (NULL != c->addr[RD_COUNTER_NIC_RESET_REQUESTED])
    {
        __atomic_store_n(
            c->addr[RD_COUNTER_NIC_RESET_REQUESTED],
            (int64_t)__atomic_load_n(&port->nic_resets, __ATOMIC_RELAXED), __ATOMIC_RELAXED);
    }
    if (NULL != c->addr[RD_COUNTER_NIC_LINK_CHANGES])
    {
        __atomic_store_n(
            c->addr[RD_COUNTER_NIC_LINK_CHANGES],
            (int64_t)__atomic_load_n(&port->nic_link_changes, __ATOMIC_RELAXED), __ATOMIC_RELAXED);
    }

    c->last_sample_ns = rusteron_dpdk_clock_now();
    c->last_nic_sample_ns = c->last_sample_ns;
    c->last_slow_sample_ns = c->last_sample_ns;
    c->registered = 1;
    c->dormant = 0;
    return 0;
}

void rusteron_dpdk_counters_sample(
    rusteron_dpdk_counters_t *c, const rusteron_dpdk_port_t *port,
    const rusteron_dpdk_port_ops_t *ops, uint64_t now_ns, uint64_t nic_stats_ns, uint64_t nic_slow_ns,
    rusteron_dpdk_event_ring_t *events, rusteron_dpdk_nic_sample_t *nic)
{
    nic->due = 0;
    if (NULL == c || !c->registered || c->dormant || NULL == port || NULL == ops)
    {
        return;
    }

    if (now_ns < c->last_sample_ns || now_ns - c->last_sample_ns < RUSTERON_DPDK_XSTAT_SAMPLE_NS)
    {
        return; /* at most once per second (plan §9) */
    }
    c->last_sample_ns = now_ns;

    const uint64_t start_ns = NULL != events ? rusteron_dpdk_clock_now() : 0;
    if (NULL != ops->mempool_avail && NULL != port->mempool &&
        NULL != c->addr[RD_COUNTER_MEMPOOL_AVAILABLE])
    {
        __atomic_store_n(
            c->addr[RD_COUNTER_MEMPOOL_AVAILABLE],
            (int64_t)ops->mempool_avail(port->mempool), __ATOMIC_RELAXED);
    }
    if (NULL != events)
    {
        rusteron_dpdk_event_record(events, RUSTERON_DPDK_EVENT_SAMPLE, port->port_id, rusteron_dpdk_clock_now() - start_ns);
    }

    /* The NIC's own statistics: on ENA each read is an admin command to the
     * device, so as often as RUSTERON_DPDK_NIC_STATS_MS says (0: never). Split
     * (RUSTERON_DPDK_NIC_SLOW_STATS_MS), those reads take only the PMD's
     * software xstats, and the full read comes as often as it says. */
    const int fast_due =
        0 != nic_stats_ns && now_ns >= c->last_nic_sample_ns && now_ns - c->last_nic_sample_ns >= nic_stats_ns;
    const int split = RUSTERON_DPDK_NIC_STATS_UNSPLIT != nic_slow_ns;
    const int slow_due = split && 0 != nic_slow_ns && now_ns >= c->last_slow_sample_ns &&
        now_ns - c->last_slow_sample_ns >= nic_slow_ns;
    if (!fast_due && !slow_due)
    {
        return;
    }
    if (fast_due)
    {
        c->last_nic_sample_ns = now_ns;
    }
    nic->full = !split || slow_due;
    if (slow_due)
    {
        c->last_slow_sample_ns = now_ns;
    }
    nic->id_count = 0;
    if (!nic->full)
    {
        if (NULL == ops->xstats_get_by_id || 0 == c->software_xstat_count)
        {
            return; /* nothing a split read could take */
        }
        nic->id_count = c->software_xstat_count;
        for (uint32_t i = 0; i < c->software_xstat_count; i++)
        {
            nic->ids[i] = c->software_xstats[i];
        }
    }
    nic->due = 1;
    nic->port_id = port->port_id;
    nic->xstat_count = NULL != ops->xstats_get ? c->xstat_count : 0;
    nic->generation = c->generation;
}

void rusteron_dpdk_counters_read_nic(
    const rusteron_dpdk_port_ops_t *ops, rusteron_dpdk_nic_sample_t *nic, rusteron_dpdk_event_ring_t *events)
{
    if (!nic->due)
    {
        return;
    }
    const uint64_t start_ns = NULL != events ? rusteron_dpdk_clock_now() : 0;
    if (!nic->full)
    {
        /* Split: the software xstats alone, by id, scattered to their slots. */
        uint64_t got[RUSTERON_DPDK_MAX_XSTATS];
        nic->basic_ok = 0;
        nic->xstats_ok = nic->xstat_count > 0 &&
            0 == ops->xstats_get_by_id(nic->port_id, nic->ids, got, nic->id_count);
        for (uint32_t i = 0; nic->xstats_ok && i < nic->id_count; i++)
        {
            nic->values[nic->ids[i]] = got[i];
        }
    }
    else
    {
        nic->basic_ok = NULL != ops->basic_stats &&
            0 == ops->basic_stats(nic->port_id, &nic->ierrors, &nic->imissed, &nic->rx_nombuf);
        nic->xstats_ok =
            nic->xstat_count > 0 && 0 == ops->xstats_get(nic->port_id, nic->values, nic->xstat_count);
    }
    if (NULL != events)
    {
        rusteron_dpdk_event_record(
            events, RUSTERON_DPDK_EVENT_NIC_STATS, nic->port_id, rusteron_dpdk_clock_now() - start_ns);
    }
}

void rusteron_dpdk_counters_store_nic(rusteron_dpdk_counters_t *c, const rusteron_dpdk_nic_sample_t *nic)
{
    if (!nic->due || NULL == c || !c->registered || c->dormant || c->generation != nic->generation)
    {
        return;
    }
    if (nic->basic_ok)
    {
        if (NULL != c->addr[RD_COUNTER_NIC_RX_ERRORS])
        {
            __atomic_store_n(c->addr[RD_COUNTER_NIC_RX_ERRORS], (int64_t)nic->ierrors, __ATOMIC_RELAXED);
        }
        if (NULL != c->addr[RD_COUNTER_NIC_RX_MISSED])
        {
            __atomic_store_n(c->addr[RD_COUNTER_NIC_RX_MISSED], (int64_t)nic->imissed, __ATOMIC_RELAXED);
        }
        if (NULL != c->addr[RD_COUNTER_NIC_RX_NOMBUF])
        {
            __atomic_store_n(c->addr[RD_COUNTER_NIC_RX_NOMBUF], (int64_t)nic->rx_nombuf, __ATOMIC_RELAXED);
        }
    }
    if (nic->xstats_ok && nic->xstat_count == c->xstat_count && !nic->full)
    {
        for (uint32_t i = 0; i < nic->id_count; i++)
        {
            const uint64_t id = nic->ids[i];
            if (NULL != c->xstats[id].addr)
            {
                __atomic_store_n(c->xstats[id].addr, (int64_t)nic->values[id], __ATOMIC_RELAXED);
            }
            if (id == c->missed_xstat_index && NULL != c->addr[RD_COUNTER_MISSED_PACKETS])
            {
                __atomic_store_n(c->addr[RD_COUNTER_MISSED_PACKETS], (int64_t)nic->values[id], __ATOMIC_RELAXED);
            }
        }
    }
    else if (nic->xstats_ok && nic->xstat_count == c->xstat_count)
    {
        for (uint32_t i = 0; i < c->xstat_count; i++)
        {
            if (NULL != c->xstats[i].addr)
            {
                __atomic_store_n(c->xstats[i].addr, (int64_t)nic->values[i], __ATOMIC_RELAXED);
            }
        }
        if (UINT32_MAX != c->missed_xstat_index && NULL != c->addr[RD_COUNTER_MISSED_PACKETS])
        {
            __atomic_store_n(
                c->addr[RD_COUNTER_MISSED_PACKETS], (int64_t)nic->values[c->missed_xstat_index], __ATOMIC_RELAXED);
        }
    }
}

void rusteron_dpdk_counters_forget(rusteron_dpdk_counters_t *c)
{
    if (NULL == c)
    {
        return;
    }
    /* The manager may be closed already, so its slots are never read here. */
    for (int32_t kind = 0; kind < RD_COUNTER_COUNT; kind++)
    {
        c->id[kind] = -1;
        c->addr[kind] = NULL;
    }
    aeron_free(c->xstats);
    aeron_free(c->xstat_names);
    aeron_free(c->software_xstats);
    c->xstats = NULL;
    c->xstat_names = NULL;
    c->software_xstats = NULL;
    c->xstat_count = 0;
    c->software_xstat_count = 0;
    c->missed_xstat_index = UINT32_MAX;
    c->registered = 0;
    c->dormant = 0;
    c->generation++;
    c->manager = NULL;
}
