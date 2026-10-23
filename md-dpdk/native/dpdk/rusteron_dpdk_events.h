/*
 * ergon-md-dpdk DPDK transport — the lab event log (RUSTERON_DPDK_EVENT_LOG).
 *
 * Off unless the variable names a file. Then each port's agent records its
 * loop gaps of RUSTERON_DPDK_EVENT_GAP_NS or more and its ARP traffic into a
 * ring of its own, and the housekeeping thread records its own work (the
 * PMDs' timer service, each port's 1 Hz sample and NIC statistics read) into
 * another. Each ring has one writer and one reader, is allocated and touched
 * at open, and never blocks its writer: a full ring drops the newest event and
 * counts it. Housekeeping drains every ring into the file each pass (100 ms),
 * and the close drains what is left, so the agents never write the file.
 *
 * File rows: `kind,tsc,realtime_ns,value_ns,port,agent` after a `#` header.
 * tsc is the CPU counter (rdtsc, CNTVCT_EL0) and realtime_ns CLOCK_REALTIME
 * when the event ended; value_ns its length (a gap, a call's duration; 0 for
 * an instant, the count for `drops`); port the DPDK port id (`-` for the timer
 * service); agent the recording thread's kernel id.
 */
#ifndef RUSTERON_DPDK_EVENTS_H
#define RUSTERON_DPDK_EVENTS_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Event kinds (the file names them). */
#define RUSTERON_DPDK_EVENT_GAP 1         /* an agent's loop gap */
#define RUSTERON_DPDK_EVENT_ARP_REQUEST 2 /* an ARP request sent (resolve, refresh or retry) */
#define RUSTERON_DPDK_EVENT_ARP_REPLY 3   /* an ARP reply sent for one of the NIC's addresses */
#define RUSTERON_DPDK_EVENT_ARP_LEARNED 4 /* an ARP reply that resolved or refreshed a next hop */
#define RUSTERON_DPDK_EVENT_TIMERS 5      /* the PMDs' timer service (ENA's watchdog) */
#define RUSTERON_DPDK_EVENT_SAMPLE 6      /* a port's 1 Hz sample (mempool availability) */
#define RUSTERON_DPDK_EVENT_NIC_STATS 7   /* a port's NIC statistics read (ENA: admin commands) */

/* The agents' loop-gap counters (poll-cycle-gaps-*) count from this. */
#define RUSTERON_DPDK_POLL_GAP_NS 20000ULL
/* No port (the timer service). */
#define RUSTERON_DPDK_EVENT_NO_PORT UINT16_MAX
/* The shortest loop gap logged: the 20 µs counters' floor is too coarse to
 * attribute a 72 µs round trip's excess. */
#define RUSTERON_DPDK_EVENT_GAP_NS 10000ULL
/* Events each ring holds (a power of two): 100 ms of a gap every 25 µs. */
#define RUSTERON_DPDK_EVENT_RING_EVENTS 4096u

typedef struct rusteron_dpdk_event_stct
{
    uint64_t tsc;
    uint64_t realtime_ns;
    uint64_t value_ns;
    uint32_t agent;
    uint16_t port;
    uint8_t kind;
    uint8_t pad;
} rusteron_dpdk_event_t;

_Static_assert(32 == sizeof(rusteron_dpdk_event_t), "two events a cache line");

/* One writer, one reader: the writer's and the reader's indexes on lines of
 * their own, so neither's stores touch the other's line. */
typedef struct rusteron_dpdk_event_ring_stct
{
    uint64_t head __attribute__((aligned(64))); /* writer: events written */
    uint64_t drops;                              /* writer: events dropped, full */
    uint64_t tail __attribute__((aligned(64))); /* reader: events drained */
    uint64_t drops_logged;                       /* reader: drops the file has */
    rusteron_dpdk_event_t *events __attribute__((aligned(64)));
    uint64_t mask;
} rusteron_dpdk_event_ring_t;

/* The loop gap the agents' poll timing first looks at: the 20 µs counters',
 * or RUSTERON_DPDK_EVENT_GAP_NS while the log is open (runtime.c). Read
 * inline on every poll. */
extern uint64_t rusteron_dpdk_gap_floor_ns __attribute__((visibility("hidden")));

/* Record an event into `ring` (its one writer), ended now. Out of line and
 * cold: called only for an event, never on an ordinary poll or send. */
__attribute__((cold, noinline)) void rusteron_dpdk_event_record(
    rusteron_dpdk_event_ring_t *ring, int kind, uint16_t port, uint64_t value_ns);

#ifdef __cplusplus
}
#endif

#endif /* RUSTERON_DPDK_EVENTS_H */
