/*
 * ergon-md-dpdk DPDK transport — test fakes, shared header.
 *
 * Only test binaries link these archives (rusteron_dpdk_fake.a for the port
 * ops, rusteron_dpdk_fake_eal.a for the EAL seam). Both are DPDK-free so the
 * integration tests run on any host without libdpdk.
 */
#ifndef RUSTERON_DPDK_FAKE_H
#define RUSTERON_DPDK_FAKE_H

#include <stddef.h>
#include <stdint.h>

#include "rusteron_dpdk_port_ops.h"

#ifdef __cplusplus
extern "C" {
#endif

/* The largest frame the fake carries, sent or received: a jumbo frame (a
 * 9001-byte MTU's) and more. */
#define RUSTERON_DPDK_FAKE_FRAME_MAX 9216

/* Reset all fake state (call via the Rust test setup between tests). */
void rusteron_dpdk_fake_reset(void);
void rusteron_dpdk_fake_set_desc_limits(uint16_t rx_min, uint16_t rx_max, uint16_t tx_min, uint16_t tx_max);

/* Fail the n-th init step (1..=18; 1-9 hit the sender, 10-18 the receiver). */
void rusteron_dpdk_fake_set_failure(int step);

/* Override the reported driver name and checksum capability (0 clears). */
void rusteron_dpdk_fake_set_driver(const char *driver);
void rusteron_dpdk_fake_set_csum_ok(int ok);
/* How an ENA port's LLQ BAR is mapped (RUSTERON_DPDK_ENA_LLQ_*, dev_info's
 * ena_llq for net_ena): RUSTERON_DPDK_ENA_LLQ_WC after the reset. */
void rusteron_dpdk_fake_set_ena_llq(int state);

/* Whether dev_info advertises TX MBUF_FAST_FREE (1, as ENA, after the reset). */
void rusteron_dpdk_fake_set_tx_fast_free_capa(int ok);
/* Each rx_burst that returns frames, and each tx_burst that takes some, moves
 * the pinned transport clock on by this much inside the call (0 after the
 * reset): a PMD call that took that long. */
void rusteron_dpdk_fake_set_burst_ns(uint64_t rx_ns, uint64_t tx_ns);
/* What `port_id` was set up with: its queues' free thresholds (0: the PMD's)
 * and the TX offloads it was configured with. 0 on success. */
int rusteron_dpdk_fake_queue_conf(uint16_t port_id, uint16_t *rx_free_thresh, uint16_t *tx_free_thresh,
                                  uint64_t *tx_offloads);
/* Mbufs sent on a port configured with MBUF_FAST_FREE that break its
 * preconditions: another pool's, not held by the sender alone, or more than
 * one segment. */
int rusteron_dpdk_fake_fast_free_violations(void);
/* Send one mbuf of the other role's pool on `port_id` (0 or 1), as nothing in
 * the transport does: the check above sees it. 0 when the fake took it. */
int rusteron_dpdk_fake_tx_foreign(uint16_t port_id);

/* The NIC statistics read (basic_stats) sleeps `ms` inside the call, as an
 * ENA admin command may take (0 after the reset); and how many such reads
 * have begun, on any thread. */
void rusteron_dpdk_fake_set_stats_delay_ms(uint32_t ms);
int rusteron_dpdk_fake_stats_reads_started(void);

/* Link-status-change interrupts: whether the ports can raise them
 * (RTE_ETH_DEV_INTR_LSC; 1, as ENA, after the reset), whether `port_id` was
 * configured to, and one raised on it, as its event callback would count it. */
void rusteron_dpdk_fake_set_lsc_capa(int ok);
int rusteron_dpdk_fake_lsc_configured(uint16_t port_id);
void rusteron_dpdk_fake_raise_lsc(uint16_t port_id);

/* A gateway on `port_id` that answers every ARP request sent on it (off
 * after the reset): the reply is queued for the port's next rx_burst. */
void rusteron_dpdk_fake_set_arp_responder(uint16_t port_id, int on);

/* tx_done_cleanup: the cleanups asked for on `port_id`, the budget of the
 * last, the frames the port sent since its last cleanup, and whether the PMD
 * lacks the op (-ENOTSUP; 0 after the reset). */
int rusteron_dpdk_fake_tx_cleanup_calls(uint16_t port_id);
uint32_t rusteron_dpdk_fake_tx_cleanup_last_budget(void);
uint32_t rusteron_dpdk_fake_tx_unreaped_count(uint16_t port_id);
void rusteron_dpdk_fake_set_tx_cleanup_unsupported(int unsupported);
/* Make tx_done_cleanup return `rc` (a negative errno other than -ENOTSUP),
 * 0 to stop; 0 after the reset. */
void rusteron_dpdk_fake_set_tx_cleanup_error(int rc);

/* Extended-statistics reads since the reset: of every xstat and by id; and
 * the ids the last by-id read asked for (its count returned, at most `max`
 * copied). */
void rusteron_dpdk_fake_xstats_reads(int *full, int *by_id);
uint32_t rusteron_dpdk_fake_xstats_last_ids(uint64_t *ids, uint32_t max);

/* The fake's call log, for reverse-teardown ordering assertions. */
int rusteron_dpdk_fake_log_count(void);
void rusteron_dpdk_fake_log_at(int index, char *buf, size_t buflen);

/* Data-path knobs (plan §7.4): cap the frames accepted per tx_burst, and cap
 * the number of mbufs that may be live at once (pool exhaustion). */
void rusteron_dpdk_fake_set_tx_burst_cap(uint16_t n);
void rusteron_dpdk_fake_set_pool_avail(int n);

/* Extended statistics (plan §9): install the fake PMD's xstat table. `names`
 * points to `count` NUL-terminated strings, each copied at the fixed
 * RUSTERON_DPDK_XSTAT_NAME_LEN stride; `values` are copied verbatim. Call again
 * to change the values between 1 Hz samples (the sample must re-read them). */
void rusteron_dpdk_fake_set_xstats(const char *const *names, const uint64_t *values, uint32_t count);
/* The next `reads` xstats counts report none, whatever is installed (0 after
 * the reset): a netvsc VF that attaches after the port starts. */
void rusteron_dpdk_fake_set_xstats_hidden_reads(uint32_t reads);

/* A frame accepted by the fake tx_burst (the mbuf is owned by the "NIC"). */
typedef struct rusteron_dpdk_fake_capture_stct
{
    uint8_t data[RUSTERON_DPDK_FAKE_FRAME_MAX];
    uint32_t len;
    uint32_t ol_flags;
    uint16_t l2_len, l3_len, l4_len;
    uint16_t udp_pseudo_csum;
    uint16_t port_id;
} rusteron_dpdk_fake_capture_t;

int rusteron_dpdk_fake_capture_count(void);
int rusteron_dpdk_fake_capture_at(int index, rusteron_dpdk_fake_capture_t *out);

/* Live-mbuf accounting for leak assertions: after any send, every allocated
 * mbuf has been released (sent or unsent), so allocated() == released(). */
int rusteron_dpdk_fake_allocated(void);
int rusteron_dpdk_fake_released(void);

/* RX injector (plan §7.6): queue a raw frame for the next rx_burst on the
 * given port. `rx_ol_flags`/`nb_segs` seed the delivered view so tests can
 * exercise NIC-reported checksum verdicts and multi-segment rejection.
 * Returns 0 on success, -1 when the queue is full or the port is out of range. */
int rusteron_dpdk_fake_rx_inject(
    uint16_t port_id, const uint8_t *frame, size_t len,
    uint32_t rx_ol_flags, uint32_t nb_segs);

/* Number of frames still queued for the port's rx_burst (test assertion). */
int rusteron_dpdk_fake_rx_queued(uint16_t port_id);

/* Each pool's mbufs hold the data room the transport asks for (the fake logs
 * it as "pool <name> (<n>, room <bytes>)"), at most `room` once this is set,
 * as a pool or PMD that gives less would (0: as asked). A frame injected for
 * a port larger than its receive pool's room arrives in several segments. */
void rusteron_dpdk_fake_set_mbuf_room_cap(uint32_t room);

/* The timer service (the PMDs' timers: ENA's health checks): how often it
 * ran, how often from a thread the fake EAL never registered, whether the
 * calling thread ran it last, and what it returns (0 until set). */
int rusteron_dpdk_fake_timer_service_calls(void);
int rusteron_dpdk_fake_timer_service_unregistered(void);
int rusteron_dpdk_fake_timer_service_on_caller(void);
void rusteron_dpdk_fake_set_timer_service_rc(int rc);

/* A device-reset request (RTE_ETH_EVENT_INTR_RESET) from the port's PMD, as
 * its event callback raises it: the transport's count for the port goes up. */
void rusteron_dpdk_fake_raise_reset(uint16_t port_id);

/* The NUMA node `port_id` reports (rte_eth_dev_socket_id). Unset: -1, the
 * PMD reports none. The sender is port 0 and the receiver port 1. */
void rusteron_dpdk_fake_set_socket(uint16_t port_id, int socket_id);

/* Fake EAL seam reset (rusteron_dpdk_fake_eal.c). */
void rusteron_dpdk_fake_eal_reset(void);
/* The DPDK version the fake EAL reports (rusteron_dpdk_eal_version), which
 * picks ENA's LLQ device argument: 25.11 after the reset. */
void rusteron_dpdk_fake_set_eal_version(int year, int month);
/* The CPU the fake EAL reports its main lcore on (rusteron_dpdk_eal_main_cpu),
 * where the real one confines its own threads; -1 (none) until set, and
 * again after the reset. */
void rusteron_dpdk_fake_set_eal_main_cpu(int cpu);
/* What the fake EAL reports about the counter (rusteron_dpdk_eal_counter_rate),
 * whether or not it ran: nothing after the reset (the transport's clock is then
 * CLOCK_MONOTONIC). `hz` as rte_get_tsc_hz() on a host whose counter it trusts
 * (an invariant TSC; CNTFRQ_EL0 the same); or each fact on its own. */
void rusteron_dpdk_fake_set_eal_tsc_hz(uint64_t hz);
void rusteron_dpdk_fake_set_eal_counter_facts(uint64_t eal_hz, int invariant, uint64_t cntfrq);
/* Whether the calling thread registered with the fake EAL
 * (rusteron_dpdk_eal_thread_register) and has not unregistered since. */
int rusteron_dpdk_fake_eal_thread_registered(void);

#ifdef __cplusplus
}
#endif

#endif /* RUSTERON_DPDK_FAKE_H */
