/*
 * ergon-md-dpdk DPDK transport — the lab event log (RUSTERON_DPDK_EVENT_LOG;
 * see rusteron_dpdk_events.h).
 *
 * The writers (each port's agent, the housekeeping thread) only fill
 * preallocated slots of their own ring; the file is written by housekeeping
 * and the close.
 */
#define _GNU_SOURCE /* syscall(SYS_gettid): the agent's kernel id */
#define _POSIX_C_SOURCE 200809L

#include "rusteron_dpdk_internal.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/syscall.h>
#include <unistd.h>

uint64_t rusteron_dpdk_gap_floor_ns = RUSTERON_DPDK_POLL_GAP_NS;

/* Events each ring holds when a test sets it (a power of two; 0: the
 * default). */
static uint32_t rusteron_dpdk_test_event_ring_events = 0;

void rusteron_dpdk_test_set_event_ring_events(uint32_t events)
{
    rusteron_dpdk_test_event_ring_events = events;
}

void rusteron_dpdk_events_test_reset(void)
{
    rusteron_dpdk_test_event_ring_events = 0;
    rusteron_dpdk_gap_floor_ns = RUSTERON_DPDK_POLL_GAP_NS;
}

/* The calling thread's kernel id, read once per thread. */
static _Thread_local uint32_t rusteron_dpdk_event_agent = 0;

static uint32_t rusteron_dpdk_event_thread(void)
{
    if (0 == rusteron_dpdk_event_agent)
    {
        rusteron_dpdk_event_agent = (uint32_t)syscall(SYS_gettid);
    }
    return rusteron_dpdk_event_agent;
}

void rusteron_dpdk_event_record(rusteron_dpdk_event_ring_t *ring, int kind, uint16_t port, uint64_t value_ns)
{
    const uint64_t head = ring->head; /* this thread's alone */
    if (head - __atomic_load_n(&ring->tail, __ATOMIC_ACQUIRE) > ring->mask)
    {
        __atomic_store_n(&ring->drops, ring->drops + 1, __ATOMIC_RELAXED);
        return;
    }
    rusteron_dpdk_event_t *e = &ring->events[head & ring->mask];
    e->tsc = rusteron_dpdk_counter();
    e->realtime_ns = rusteron_dpdk_realtime_ns();
    e->value_ns = value_ns;
    e->agent = rusteron_dpdk_event_thread();
    e->port = port;
    e->kind = (uint8_t)kind;
    __atomic_store_n(&ring->head, head + 1, __ATOMIC_RELEASE);
}

static const char *rusteron_dpdk_event_name(int kind)
{
    switch (kind)
    {
        case RUSTERON_DPDK_EVENT_GAP: return "gap";
        case RUSTERON_DPDK_EVENT_ARP_REQUEST: return "arp-request";
        case RUSTERON_DPDK_EVENT_ARP_REPLY: return "arp-reply";
        case RUSTERON_DPDK_EVENT_ARP_LEARNED: return "arp-learned";
        case RUSTERON_DPDK_EVENT_TIMERS: return "timers";
        case RUSTERON_DPDK_EVENT_SAMPLE: return "sample";
        case RUSTERON_DPDK_EVENT_NIC_STATS: return "nic-stats";
        default: return "unknown";
    }
}

static void rusteron_dpdk_event_row(
    FILE *f, const char *kind, uint64_t tsc, uint64_t realtime_ns, uint64_t value_ns, uint16_t port, uint32_t agent)
{
    char port_text[8] = "-";
    if (RUSTERON_DPDK_EVENT_NO_PORT != port)
    {
        snprintf(port_text, sizeof(port_text), "%u", (unsigned)port);
    }
    fprintf(f, "%s,%llu,%llu,%llu,%s,%u\n", kind, (unsigned long long)tsc, (unsigned long long)realtime_ns,
            (unsigned long long)value_ns, port_text, (unsigned)agent);
}

/* Everything `ring` holds into the file, then how many it dropped since the
 * last drain, on `port`'s behalf. Its one reader. */
static void rusteron_dpdk_events_drain_ring(FILE *f, rusteron_dpdk_event_ring_t *ring, uint16_t port)
{
    if (NULL == ring)
    {
        return;
    }
    const uint64_t head = __atomic_load_n(&ring->head, __ATOMIC_ACQUIRE);
    for (uint64_t t = ring->tail; t != head; t++)
    {
        const rusteron_dpdk_event_t *e = &ring->events[t & ring->mask];
        rusteron_dpdk_event_row(
            f, rusteron_dpdk_event_name(e->kind), e->tsc, e->realtime_ns, e->value_ns, e->port, e->agent);
    }
    __atomic_store_n(&ring->tail, head, __ATOMIC_RELEASE);
    const uint64_t drops = __atomic_load_n(&ring->drops, __ATOMIC_RELAXED);
    if (drops != ring->drops_logged)
    {
        rusteron_dpdk_event_row(
            f, "drops", rusteron_dpdk_counter(), rusteron_dpdk_realtime_ns(), drops - ring->drops_logged, port,
            rusteron_dpdk_event_thread());
        ring->drops_logged = drops;
    }
}

void rusteron_dpdk_events_drain(rusteron_dpdk_transport_t *native)
{
    if (NULL == native->event_log)
    {
        return;
    }
    rusteron_dpdk_events_drain_ring(native->event_log, native->sender.events, native->sender.port_id);
    rusteron_dpdk_events_drain_ring(native->event_log, native->receiver.events, native->receiver.port_id);
    rusteron_dpdk_events_drain_ring(native->event_log, native->housekeeping_events, RUSTERON_DPDK_EVENT_NO_PORT);
    fflush(native->event_log);
}

static void rusteron_dpdk_event_ring_free(rusteron_dpdk_event_ring_t **ring)
{
    if (NULL != *ring)
    {
        free((*ring)->events);
        free(*ring);
        *ring = NULL;
    }
}

/* A ring of `events` (a power of two), every page touched now so no write
 * on an agent faults one in. NULL when out of memory. */
static rusteron_dpdk_event_ring_t *rusteron_dpdk_event_ring_new(uint32_t events)
{
    rusteron_dpdk_event_ring_t *ring = aligned_alloc(64, sizeof(rusteron_dpdk_event_ring_t));
    if (NULL == ring)
    {
        return NULL;
    }
    memset(ring, 0, sizeof(*ring));
    ring->events = aligned_alloc(64, (size_t)events * sizeof(rusteron_dpdk_event_t));
    if (NULL == ring->events)
    {
        free(ring);
        return NULL;
    }
    memset(ring->events, 0, (size_t)events * sizeof(rusteron_dpdk_event_t));
    ring->mask = events - 1;
    return ring;
}

int rusteron_dpdk_events_open(rusteron_dpdk_transport_t *native)
{
    const char *path = getenv("RUSTERON_DPDK_EVENT_LOG");
    if (NULL == path || '\0' == path[0])
    {
        return 0;
    }
    char message[512];
    FILE *f = fopen(path, "w");
    if (NULL == f)
    {
        snprintf(message, sizeof(message), "RUSTERON_DPDK_EVENT_LOG=%s cannot be opened for writing: %s", path,
                 strerror(errno));
        rusteron_dpdk_set_error(message);
        return -1;
    }
    const uint32_t events = 0 != rusteron_dpdk_test_event_ring_events ? rusteron_dpdk_test_event_ring_events
                                                                      : RUSTERON_DPDK_EVENT_RING_EVENTS;
    native->sender.events = rusteron_dpdk_event_ring_new(events);
    native->receiver.events = rusteron_dpdk_event_ring_new(events);
    native->housekeeping_events = rusteron_dpdk_event_ring_new(events);
    if (NULL == native->sender.events || NULL == native->receiver.events || NULL == native->housekeeping_events)
    {
        rusteron_dpdk_event_ring_free(&native->sender.events);
        rusteron_dpdk_event_ring_free(&native->receiver.events);
        rusteron_dpdk_event_ring_free(&native->housekeeping_events);
        fclose(f);
        snprintf(message, sizeof(message), "RUSTERON_DPDK_EVENT_LOG=%s: out of memory for its rings", path);
        rusteron_dpdk_set_error(message);
        return -1;
    }
    fprintf(f, "# kind,tsc,realtime_ns,value_ns,port,agent\n");
    fflush(f);
    native->event_log = f;
    rusteron_dpdk_gap_floor_ns = RUSTERON_DPDK_EVENT_GAP_NS;
    return 0;
}

void rusteron_dpdk_events_close(rusteron_dpdk_transport_t *native)
{
    if (NULL == native->event_log)
    {
        return;
    }
    rusteron_dpdk_events_drain(native);
    fclose(native->event_log);
    native->event_log = NULL;
    rusteron_dpdk_gap_floor_ns = RUSTERON_DPDK_POLL_GAP_NS;
    rusteron_dpdk_event_ring_free(&native->sender.events);
    rusteron_dpdk_event_ring_free(&native->receiver.events);
    rusteron_dpdk_event_ring_free(&native->housekeeping_events);
}
