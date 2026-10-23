/*
 * ergon-md-dpdk DPDK transport — fake port ops for tests.
 *
 * Implements the same symbol rusteron_dpdk_port_ops_real() that the production
 * rusteron_dpdk_port.c defines, so the linker resolves the runtime's ops lookup
 * to this table in test binaries. Drives the full init/teardown sequence with
 * a call log (for reverse-order teardown assertions) and a failure-step counter
 * (1..=18: 1-9 sender steps, 10-18 receiver steps) so every failure path in the
 * runtime is reachable deterministically without libdpdk.
 */
/* clock_gettime under the strict -std=c11 build (the legacy clock below). */
#define _POSIX_C_SOURCE 200809L

#include "rusteron_dpdk_fake.h"

#include <pthread.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

#define RUSTERON_DPDK_FAKE_LOG_MAX 64
#define RUSTERON_DPDK_FAKE_LOG_ENTRY_LEN 128

#define RUSTERON_DPDK_FAKE_BUFFER_COUNT 256
/* Room for a jumbo frame (a 9001-byte MTU's and more); each mbuf offers only
 * its pool's data room of it. */
#define RUSTERON_DPDK_FAKE_BUFFER_CAPACITY RUSTERON_DPDK_FAKE_FRAME_MAX
/* The data room of an mbuf from no pool the fake created: DPDK's default. */
#define RUSTERON_DPDK_FAKE_DEFAULT_ROOM 2048
#define RUSTERON_DPDK_FAKE_POOLS 16
#define RUSTERON_DPDK_FAKE_POOL_AVAIL_DEFAULT 64
#define RUSTERON_DPDK_FAKE_TX_BURST_CAP_DEFAULT 64
#define RUSTERON_DPDK_FAKE_CAPTURE_MAX 1024
#define RUSTERON_DPDK_FAKE_PORTS 4
#define RUSTERON_DPDK_FAKE_RX_DEPTH 256
#define RUSTERON_DPDK_FAKE_XSTAT_MAX 256

typedef struct rusteron_dpdk_fake_buffer_stct
{
    uint8_t storage[RUSTERON_DPDK_FAKE_BUFFER_CAPACITY];
    int used;
    const void *pool; /* the pool it was allocated from */
} rusteron_dpdk_fake_buffer_t;

/* The pinned transport clock moved on by a fake PMD call (runtime.c). */
void rusteron_dpdk_test_advance_clock_ns(uint64_t ns);

static int rusteron_dpdk_fake_failure_step = 0;
static int rusteron_dpdk_fake_call_count = 0;
static char rusteron_dpdk_fake_driver[64] = "net_ena";
static int rusteron_dpdk_fake_csum_ok = 1;
static int rusteron_dpdk_fake_rx_timestamp_ok = 0;
/* Whether dev_info advertises TX MBUF_FAST_FREE: as ENA does, until set. */
static int rusteron_dpdk_fake_tx_fast_free_capa = 1;
/* How long a burst that returns or takes frames holds the pinned clock: it
 * moves on by this much inside the call (0: it does not). */
static uint64_t rusteron_dpdk_fake_rx_burst_ns = 0;
static uint64_t rusteron_dpdk_fake_tx_burst_ns = 0;
/* What each port was set up with: the TX offloads it was configured with and
 * each queue's free threshold (0: the PMD's). */
static uint64_t rusteron_dpdk_fake_tx_offloads_of[4];
static uint16_t rusteron_dpdk_fake_rx_free_of[4];
static uint16_t rusteron_dpdk_fake_tx_free_of[4];
/* Sends that broke MBUF_FAST_FREE's preconditions on a port configured with
 * it: an mbuf from another pool, one not held (freed, or sent twice), or a
 * frame that does not fit one segment. */
static int rusteron_dpdk_fake_fast_free_bad = 0;
/* How the fake reports an ENA's LLQ BAR mapped (RUSTERON_DPDK_ENA_LLQ_*):
 * write-combined until set. */
static int rusteron_dpdk_fake_ena_llq = RUSTERON_DPDK_ENA_LLQ_WC;
static uint16_t rusteron_dpdk_fake_max_mtu = 9001;
/* Descriptor limits the fake ports report: min/max per direction (0: none). */
static uint16_t rusteron_dpdk_fake_desc_lim[4];
static uint64_t rusteron_dpdk_fake_basic[3];
static uint16_t rusteron_dpdk_fake_next_port = 0;
static uintptr_t rusteron_dpdk_fake_next_pool = 0x1000;
/* Each pool's mbuf data room, as asked unless capped (0: no cap), and the
 * pool each port receives into. */
static uint32_t rusteron_dpdk_fake_pool_room[RUSTERON_DPDK_FAKE_POOLS];
static uint32_t rusteron_dpdk_fake_room_cap = 0;
static void *rusteron_dpdk_fake_rx_pool[4];
/* The timer service: calls, those from a thread not registered with DPDK, the
 * last caller and what it returns. */
static int rusteron_dpdk_fake_timer_calls = 0;
static int rusteron_dpdk_fake_timer_unregistered = 0;
static pthread_t rusteron_dpdk_fake_timer_caller;
static int rusteron_dpdk_fake_timer_rc = 0;
/* Each port's reset-request count, as registered (watch_reset). */
static uint64_t *rusteron_dpdk_fake_resets[4];
/* NUMA node a test assigns to a port id (-1 until set: the PMD reports none). */
static int rusteron_dpdk_fake_socket_of[4];
static int rusteron_dpdk_fake_socket_known[4];
/* Socket arguments of the pool and queues for the port currently being set up. */
static int rusteron_dpdk_fake_last_pool_socket = -1;
static int rusteron_dpdk_fake_last_rx_socket = -1;
static int rusteron_dpdk_fake_log_len = 0;
static char rusteron_dpdk_fake_log[RUSTERON_DPDK_FAKE_LOG_MAX][RUSTERON_DPDK_FAKE_LOG_ENTRY_LEN];

/* Data path (plan §7.4): a pool of fixed-capacity buffers, a tx-burst accept
 * cap, live-mbuf accounting, and a frame capture for golden-vector asserts. */
static rusteron_dpdk_fake_buffer_t rusteron_dpdk_fake_buffers[RUSTERON_DPDK_FAKE_BUFFER_COUNT];
static int rusteron_dpdk_fake_pool_avail = RUSTERON_DPDK_FAKE_POOL_AVAIL_DEFAULT;
static uint16_t rusteron_dpdk_fake_tx_burst_cap = RUSTERON_DPDK_FAKE_TX_BURST_CAP_DEFAULT;
static int rusteron_dpdk_fake_allocated_count = 0;
static int rusteron_dpdk_fake_released_count = 0;
static int rusteron_dpdk_fake_capture_len = 0;
static rusteron_dpdk_fake_capture_t rusteron_dpdk_fake_capture[RUSTERON_DPDK_FAKE_CAPTURE_MAX];

/* RX path (plan §7.6): a per-port ring of injected frames that rx_burst drains
 * into freshly allocated mbuf views, so receive traffic flows through the same
 * pool and leak accounting as transmit. */
typedef struct rusteron_dpdk_fake_rx_entry_stct
{
    uint8_t data[RUSTERON_DPDK_FAKE_BUFFER_CAPACITY];
    uint32_t len;
    uint32_t rx_ol_flags;
    uint32_t nb_segs;
    uint64_t hw_rx_ns;
} rusteron_dpdk_fake_rx_entry_t;

static int rusteron_dpdk_fake_rx_head[RUSTERON_DPDK_FAKE_PORTS];
static int rusteron_dpdk_fake_rx_tail[RUSTERON_DPDK_FAKE_PORTS];
static rusteron_dpdk_fake_rx_entry_t rusteron_dpdk_fake_rx_q[RUSTERON_DPDK_FAKE_PORTS][RUSTERON_DPDK_FAKE_RX_DEPTH];

/* Extended statistics (plan §9): a fake PMD xstat table installed via
 * rusteron_dpdk_fake_set_xstats, so the counters test can assert the type-92
 * mirrors (including an ENA-style *missed* entry) without libdpdk. */
static uint32_t rusteron_dpdk_fake_xstat_count = 0;
static char rusteron_dpdk_fake_xstat_names[RUSTERON_DPDK_FAKE_XSTAT_MAX][RUSTERON_DPDK_XSTAT_NAME_LEN];
static uint64_t rusteron_dpdk_fake_xstat_values[RUSTERON_DPDK_FAKE_XSTAT_MAX];

/* Link-status-change interrupts: whether the ports can raise them (1, as
 * ENA, after the reset), whether each was configured to, and where its
 * changes are counted while watched. A gateway that answers ARP on each port
 * (rusteron_dpdk_fake_set_arp_responder). */
static int rusteron_dpdk_fake_lsc_capa = 1;
static int rusteron_dpdk_fake_lsc_of[4];
static uint64_t *rusteron_dpdk_fake_lsc_watch[4];
static int rusteron_dpdk_fake_arp_responder[4];
/* A NIC statistics read that takes this long, and how many have begun. */
static uint32_t rusteron_dpdk_fake_stats_delay = 0;
static int rusteron_dpdk_fake_stats_started = 0;

static void rusteron_dpdk_fake_logf(const char *fmt, ...)
{
    if (rusteron_dpdk_fake_log_len >= RUSTERON_DPDK_FAKE_LOG_MAX)
    {
        return;
    }
    va_list ap;
    va_start(ap, fmt);
    vsnprintf(rusteron_dpdk_fake_log[rusteron_dpdk_fake_log_len++],
              RUSTERON_DPDK_FAKE_LOG_ENTRY_LEN, fmt, ap);
    va_end(ap);
}

/* Advance the call counter; fail when the configured step is reached. */
static int rusteron_dpdk_fake_step(void)
{
    rusteron_dpdk_fake_call_count++;
    if (rusteron_dpdk_fake_failure_step != 0 &&
        rusteron_dpdk_fake_call_count == rusteron_dpdk_fake_failure_step)
    {
        return -1;
    }
    return 0;
}

void rusteron_dpdk_fake_reset(void)
{
    rusteron_dpdk_fake_failure_step = 0;
    rusteron_dpdk_fake_call_count = 0;
    strcpy(rusteron_dpdk_fake_driver, "net_ena");
    rusteron_dpdk_fake_csum_ok = 1;
    rusteron_dpdk_fake_rx_timestamp_ok = 0;
    rusteron_dpdk_fake_tx_fast_free_capa = 1;
    rusteron_dpdk_fake_rx_burst_ns = 0;
    rusteron_dpdk_fake_tx_burst_ns = 0;
    memset(rusteron_dpdk_fake_tx_offloads_of, 0, sizeof(rusteron_dpdk_fake_tx_offloads_of));
    memset(rusteron_dpdk_fake_rx_free_of, 0, sizeof(rusteron_dpdk_fake_rx_free_of));
    memset(rusteron_dpdk_fake_tx_free_of, 0, sizeof(rusteron_dpdk_fake_tx_free_of));
    rusteron_dpdk_fake_fast_free_bad = 0;
    rusteron_dpdk_fake_ena_llq = RUSTERON_DPDK_ENA_LLQ_WC;
    rusteron_dpdk_fake_max_mtu = 9001;
    memset(rusteron_dpdk_fake_desc_lim, 0, sizeof(rusteron_dpdk_fake_desc_lim));
    memset(rusteron_dpdk_fake_basic, 0, sizeof(rusteron_dpdk_fake_basic));
    rusteron_dpdk_fake_stats_delay = 0;
    rusteron_dpdk_fake_lsc_capa = 1;
    memset(rusteron_dpdk_fake_lsc_of, 0, sizeof(rusteron_dpdk_fake_lsc_of));
    memset(rusteron_dpdk_fake_lsc_watch, 0, sizeof(rusteron_dpdk_fake_lsc_watch));
    memset(rusteron_dpdk_fake_arp_responder, 0, sizeof(rusteron_dpdk_fake_arp_responder));
    __atomic_store_n(&rusteron_dpdk_fake_stats_started, 0, __ATOMIC_RELEASE);
    rusteron_dpdk_fake_next_port = 0;
    rusteron_dpdk_fake_next_pool = 0x1000;
    rusteron_dpdk_fake_log_len = 0;
    memset(rusteron_dpdk_fake_pool_room, 0, sizeof(rusteron_dpdk_fake_pool_room));
    rusteron_dpdk_fake_room_cap = 0;
    memset(rusteron_dpdk_fake_rx_pool, 0, sizeof(rusteron_dpdk_fake_rx_pool));
    rusteron_dpdk_fake_timer_calls = 0;
    rusteron_dpdk_fake_timer_unregistered = 0;
    rusteron_dpdk_fake_timer_rc = 0;
    memset(rusteron_dpdk_fake_resets, 0, sizeof(rusteron_dpdk_fake_resets));
    memset(rusteron_dpdk_fake_socket_of, 0, sizeof(rusteron_dpdk_fake_socket_of));
    memset(rusteron_dpdk_fake_socket_known, 0, sizeof(rusteron_dpdk_fake_socket_known));
    rusteron_dpdk_fake_last_pool_socket = -1;
    rusteron_dpdk_fake_last_rx_socket = -1;

    memset(rusteron_dpdk_fake_buffers, 0, sizeof(rusteron_dpdk_fake_buffers));
    rusteron_dpdk_fake_pool_avail = RUSTERON_DPDK_FAKE_POOL_AVAIL_DEFAULT;
    rusteron_dpdk_fake_tx_burst_cap = RUSTERON_DPDK_FAKE_TX_BURST_CAP_DEFAULT;
    rusteron_dpdk_fake_allocated_count = 0;
    rusteron_dpdk_fake_released_count = 0;
    rusteron_dpdk_fake_capture_len = 0;
    memset(rusteron_dpdk_fake_rx_head, 0, sizeof(rusteron_dpdk_fake_rx_head));
    memset(rusteron_dpdk_fake_rx_tail, 0, sizeof(rusteron_dpdk_fake_rx_tail));
    memset(rusteron_dpdk_fake_rx_q, 0, sizeof(rusteron_dpdk_fake_rx_q));
    rusteron_dpdk_fake_xstat_count = 0;
}

void rusteron_dpdk_fake_set_tx_burst_cap(uint16_t n)
{
    rusteron_dpdk_fake_tx_burst_cap = n;
}

void rusteron_dpdk_fake_set_pool_avail(int n)
{
    rusteron_dpdk_fake_pool_avail = n;
}

void rusteron_dpdk_fake_set_xstats(const char *const *names, const uint64_t *values, uint32_t count)
{
    if (count > RUSTERON_DPDK_FAKE_XSTAT_MAX)
    {
        count = RUSTERON_DPDK_FAKE_XSTAT_MAX;
    }
    rusteron_dpdk_fake_xstat_count = count;
    for (uint32_t i = 0; i < count; i++)
    {
        memset(rusteron_dpdk_fake_xstat_names[i], 0, RUSTERON_DPDK_XSTAT_NAME_LEN);
        if (NULL != names && NULL != names[i])
        {
            snprintf(rusteron_dpdk_fake_xstat_names[i], RUSTERON_DPDK_XSTAT_NAME_LEN, "%s", names[i]);
        }
        rusteron_dpdk_fake_xstat_values[i] = (NULL != values) ? values[i] : 0;
    }
}

int rusteron_dpdk_fake_capture_count(void)
{
    return rusteron_dpdk_fake_capture_len;
}

int rusteron_dpdk_fake_capture_at(int index, rusteron_dpdk_fake_capture_t *out)
{
    if (NULL == out || index < 0 || index >= rusteron_dpdk_fake_capture_len)
    {
        return -1;
    }
    *out = rusteron_dpdk_fake_capture[index];
    return 0;
}

int rusteron_dpdk_fake_allocated(void)
{
    return rusteron_dpdk_fake_allocated_count;
}

int rusteron_dpdk_fake_released(void)
{
    return rusteron_dpdk_fake_released_count;
}

void rusteron_dpdk_fake_set_failure(int step)
{
    rusteron_dpdk_fake_failure_step = step;
}

void rusteron_dpdk_fake_set_driver(const char *driver)
{
    snprintf(rusteron_dpdk_fake_driver, sizeof(rusteron_dpdk_fake_driver), "%s", driver);
}

void rusteron_dpdk_fake_set_csum_ok(int ok)
{
    rusteron_dpdk_fake_csum_ok = ok;
}

void rusteron_dpdk_fake_set_rx_timestamp_ok(int ok)
{
    rusteron_dpdk_fake_rx_timestamp_ok = ok;
}

void rusteron_dpdk_fake_set_ena_llq(int state)
{
    rusteron_dpdk_fake_ena_llq = state;
}

void rusteron_dpdk_fake_set_tx_fast_free_capa(int ok)
{
    rusteron_dpdk_fake_tx_fast_free_capa = ok;
}

void rusteron_dpdk_fake_set_burst_ns(uint64_t rx_ns, uint64_t tx_ns)
{
    rusteron_dpdk_fake_rx_burst_ns = rx_ns;
    rusteron_dpdk_fake_tx_burst_ns = tx_ns;
}

int rusteron_dpdk_fake_queue_conf(uint16_t port_id, uint16_t *rx_free_thresh, uint16_t *tx_free_thresh,
                                  uint64_t *tx_offloads)
{
    if (port_id >= 4)
    {
        return -1;
    }
    *rx_free_thresh = rusteron_dpdk_fake_rx_free_of[port_id];
    *tx_free_thresh = rusteron_dpdk_fake_tx_free_of[port_id];
    *tx_offloads = rusteron_dpdk_fake_tx_offloads_of[port_id];
    return 0;
}

int rusteron_dpdk_fake_fast_free_violations(void)
{
    return rusteron_dpdk_fake_fast_free_bad;
}

void rusteron_dpdk_fake_set_desc_limits(uint16_t rx_min, uint16_t rx_max, uint16_t tx_min, uint16_t tx_max)
{
    rusteron_dpdk_fake_desc_lim[0] = rx_min;
    rusteron_dpdk_fake_desc_lim[1] = rx_max;
    rusteron_dpdk_fake_desc_lim[2] = tx_min;
    rusteron_dpdk_fake_desc_lim[3] = tx_max;
}

void rusteron_dpdk_fake_set_max_mtu(uint16_t mtu)
{
    rusteron_dpdk_fake_max_mtu = mtu;
}

void rusteron_dpdk_fake_set_mbuf_room_cap(uint32_t room)
{
    rusteron_dpdk_fake_room_cap = room;
}

/* The pool slot of a fake pool handle, or -1. */
static int rusteron_dpdk_fake_pool_slot(const void *mempool)
{
    const uintptr_t h = (uintptr_t)mempool;
    if (h <= 0x1000 || 0 != (h - 0x1000) % 0x10)
    {
        return -1;
    }
    const uintptr_t slot = (h - 0x1000) / 0x10 - 1;
    return slot < RUSTERON_DPDK_FAKE_POOLS ? (int)slot : -1;
}

/* The data room of an mbuf from `mempool`. */
static uint32_t rusteron_dpdk_fake_room(const void *mempool)
{
    const int slot = rusteron_dpdk_fake_pool_slot(mempool);
    return slot >= 0 && 0 != rusteron_dpdk_fake_pool_room[slot] ? rusteron_dpdk_fake_pool_room[slot]
                                                                  : RUSTERON_DPDK_FAKE_DEFAULT_ROOM;
}

/* Atomic: the housekeeping thread may be calling the service meanwhile. */
int rusteron_dpdk_fake_timer_service_calls(void)
{
    return __atomic_load_n(&rusteron_dpdk_fake_timer_calls, __ATOMIC_RELAXED);
}

int rusteron_dpdk_fake_timer_service_unregistered(void)
{
    return __atomic_load_n(&rusteron_dpdk_fake_timer_unregistered, __ATOMIC_RELAXED);
}

int rusteron_dpdk_fake_timer_service_on_caller(void)
{
    return rusteron_dpdk_fake_timer_calls > 0 && pthread_equal(rusteron_dpdk_fake_timer_caller, pthread_self());
}

void rusteron_dpdk_fake_set_timer_service_rc(int rc)
{
    rusteron_dpdk_fake_timer_rc = rc;
}

void rusteron_dpdk_fake_raise_reset(uint16_t port_id)
{
    /* As the real port's RTE_ETH_EVENT_INTR_RESET callback does. */
    if (port_id < 4 && NULL != rusteron_dpdk_fake_resets[port_id])
    {
        __atomic_add_fetch(rusteron_dpdk_fake_resets[port_id], 1, __ATOMIC_RELAXED);
    }
}

static int rusteron_dpdk_fake_timer_service(void)
{
    __atomic_add_fetch(&rusteron_dpdk_fake_timer_calls, 1, __ATOMIC_RELAXED);
    if (!rusteron_dpdk_fake_eal_thread_registered())
    {
        __atomic_add_fetch(&rusteron_dpdk_fake_timer_unregistered, 1, __ATOMIC_RELAXED);
    }
    rusteron_dpdk_fake_timer_caller = pthread_self();
    return rusteron_dpdk_fake_timer_rc;
}

static int rusteron_dpdk_fake_watch_reset(uint16_t port_id, uint64_t *resets)
{
    rusteron_dpdk_fake_logf("watch-reset %u", port_id);
    if (port_id < 4)
    {
        rusteron_dpdk_fake_resets[port_id] = resets;
    }
    return 0;
}

static void rusteron_dpdk_fake_unwatch_reset(uint16_t port_id, uint64_t *resets)
{
    (void)resets;
    rusteron_dpdk_fake_logf("unwatch-reset %u", port_id);
    if (port_id < 4)
    {
        rusteron_dpdk_fake_resets[port_id] = NULL;
    }
}

/* Link-status-change interrupts: whether the ports can raise them (1, as
 * ENA, after the reset), whether each was configured to, and where its
 * changes are counted while watched. */

void rusteron_dpdk_fake_set_lsc_capa(int ok)
{
    rusteron_dpdk_fake_lsc_capa = ok;
}

void rusteron_dpdk_fake_raise_lsc(uint16_t port_id)
{
    /* As the real port's RTE_ETH_EVENT_INTR_LSC callback does. */
    if (port_id < 4 && NULL != rusteron_dpdk_fake_lsc_watch[port_id])
    {
        __atomic_add_fetch(rusteron_dpdk_fake_lsc_watch[port_id], 1, __ATOMIC_RELAXED);
    }
}

int rusteron_dpdk_fake_lsc_configured(uint16_t port_id)
{
    return port_id < 4 ? rusteron_dpdk_fake_lsc_of[port_id] : 0;
}

static int rusteron_dpdk_fake_lsc_capable(uint16_t port_id)
{
    (void)port_id;
    return rusteron_dpdk_fake_lsc_capa;
}

static int rusteron_dpdk_fake_watch_lsc(uint16_t port_id, uint64_t *changes)
{
    if (port_id < 4)
    {
        rusteron_dpdk_fake_lsc_watch[port_id] = changes;
    }
    return 0;
}

static void rusteron_dpdk_fake_unwatch_lsc(uint16_t port_id, uint64_t *changes)
{
    (void)changes;
    if (port_id < 4)
    {
        rusteron_dpdk_fake_lsc_watch[port_id] = NULL;
    }
}

void rusteron_dpdk_fake_set_arp_responder(uint16_t port_id, int on)
{
    if (port_id < 4)
    {
        rusteron_dpdk_fake_arp_responder[port_id] = on;
    }
}

/* A NIC statistics read that takes this long (an ENA admin command can),
 * and how many have begun: the test that a close never waits behind one. */

void rusteron_dpdk_fake_set_stats_delay_ms(uint32_t ms)
{
    rusteron_dpdk_fake_stats_delay = ms;
}

int rusteron_dpdk_fake_stats_reads_started(void)
{
    return __atomic_load_n(&rusteron_dpdk_fake_stats_started, __ATOMIC_ACQUIRE);
}

void rusteron_dpdk_fake_set_basic_stats(uint64_t ierrors, uint64_t imissed, uint64_t rx_nombuf)
{
    rusteron_dpdk_fake_basic[0] = ierrors;
    rusteron_dpdk_fake_basic[1] = imissed;
    rusteron_dpdk_fake_basic[2] = rx_nombuf;
}

static int rusteron_dpdk_fake_basic_stats(uint16_t port_id, uint64_t *ierrors, uint64_t *imissed, uint64_t *rx_nombuf)
{
    (void)port_id;
    __atomic_add_fetch(&rusteron_dpdk_fake_stats_started, 1, __ATOMIC_RELEASE);
    if (0 != rusteron_dpdk_fake_stats_delay)
    {
        const struct timespec delay = { .tv_sec = rusteron_dpdk_fake_stats_delay / 1000,
                                        .tv_nsec = (long)(rusteron_dpdk_fake_stats_delay % 1000) * 1000000L };
        nanosleep(&delay, NULL);
    }
    *ierrors = rusteron_dpdk_fake_basic[0];
    *imissed = rusteron_dpdk_fake_basic[1];
    *rx_nombuf = rusteron_dpdk_fake_basic[2];
    return 0;
}

int rusteron_dpdk_fake_log_count(void)
{
    return rusteron_dpdk_fake_log_len;
}

void rusteron_dpdk_fake_log_at(int index, char *buf, size_t buflen)
{
    if (index >= 0 && index < rusteron_dpdk_fake_log_len)
    {
        snprintf(buf, buflen, "%s", rusteron_dpdk_fake_log[index]);
    }
    else if (buflen > 0)
    {
        buf[0] = '\0';
    }
}

static int rusteron_dpdk_fake_probe_port(const char *pci_bdf, const uint8_t *mac, uint16_t *port_id)
{
    if (NULL == mac)
    {
        rusteron_dpdk_fake_logf("probe %s", pci_bdf);
    }
    else
    {
        rusteron_dpdk_fake_logf("probe %s mac=%02x:%02x:%02x:%02x:%02x:%02x",
                                pci_bdf, mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]);
    }
    if (rusteron_dpdk_fake_step() < 0)
    {
        return -1;
    }
    if (NULL != mac)
    {
        /* MAC selection (MANA): the fake port n reports 02:00:00:00:00:<n+1>
         * (dev_info), so the selected port is the one whose MAC that is. */
        static const uint8_t prefix[5] = { 0x02, 0x00, 0x00, 0x00, 0x00 };
        if (0 != memcmp(mac, prefix, sizeof(prefix)) || 0 == mac[5] || mac[5] > RUSTERON_DPDK_FAKE_PORTS)
        {
            return -1;
        }
        *port_id = (uint16_t)(mac[5] - 1);
        return 0;
    }
    *port_id = rusteron_dpdk_fake_next_port++;
    return 0;
}

static int rusteron_dpdk_fake_dev_info(
    uint16_t port_id, const char *pci_bdf,
    uint8_t mac[RUSTERON_DPDK_ETH_ADDR_LEN],
    char *driver_name, size_t driver_name_len,
    int *csum_offload_ok, int *ena_llq, int *rx_timestamp_ok, int *tx_fast_free_ok, uint16_t *max_mtu,
    uint16_t desc_lim[4])
{
    rusteron_dpdk_fake_logf("info %s (port %u)", pci_bdf, port_id);
    if (rusteron_dpdk_fake_step() < 0)
    {
        return -1;
    }
    /* Deterministic, distinct locally-administered MACs: 02:00:00:00:00:01 for
     * the sender (port 0) and :02 for the receiver (port 1). */
    memset(mac, 0, RUSTERON_DPDK_ETH_ADDR_LEN);
    mac[0] = 0x02;
    mac[5] = (uint8_t)(port_id + 1);
    snprintf(driver_name, driver_name_len, "%s", rusteron_dpdk_fake_driver);
    *csum_offload_ok = rusteron_dpdk_fake_csum_ok;
    *ena_llq = 0 == strcmp(rusteron_dpdk_fake_driver, "net_ena") ? rusteron_dpdk_fake_ena_llq : RUSTERON_DPDK_ENA_LLQ_NA;
    *rx_timestamp_ok = rusteron_dpdk_fake_rx_timestamp_ok;
    *tx_fast_free_ok = rusteron_dpdk_fake_tx_fast_free_capa;
    *max_mtu = rusteron_dpdk_fake_max_mtu;
    memcpy(desc_lim, rusteron_dpdk_fake_desc_lim, sizeof(rusteron_dpdk_fake_desc_lim));
    return 0;
}

static int rusteron_dpdk_fake_dev_configure(
    uint16_t port_id, uint16_t rx_rings, uint16_t tx_rings,
    uint64_t rx_offloads, uint64_t tx_offloads, int lsc)
{
    if (port_id < 4)
    {
        rusteron_dpdk_fake_tx_offloads_of[port_id] = tx_offloads;
        rusteron_dpdk_fake_lsc_of[port_id] = lsc;
    }
    /* 1ULL << 2 is RUSTERON_DPDK_RX_OFFLOAD_TIMESTAMP (internal.h). */
    rusteron_dpdk_fake_logf("configure %u (rxr=%u txr=%u)%s", port_id, rx_rings, tx_rings,
                            0 != (rx_offloads & (1ULL << 2)) ? " rx-timestamp" : "");
    return rusteron_dpdk_fake_step();
}

static int rusteron_dpdk_fake_socket_id(uint16_t port_id)
{
    if (port_id < 4 && rusteron_dpdk_fake_socket_known[port_id])
    {
        return rusteron_dpdk_fake_socket_of[port_id];
    }
    return -1;
}

void rusteron_dpdk_fake_set_socket(uint16_t port_id, int socket_id)
{
    if (port_id < 4)
    {
        rusteron_dpdk_fake_socket_of[port_id] = socket_id;
        rusteron_dpdk_fake_socket_known[port_id] = 1;
    }
}

static void *rusteron_dpdk_fake_mempool_create(
    const char *name, uint32_t n, uint16_t cache_size, uint16_t data_room, int socket_id)
{
    (void)cache_size;
    rusteron_dpdk_fake_last_pool_socket = socket_id;
    rusteron_dpdk_fake_logf("pool %s (%u, room %u)", name, n, data_room);
    if (rusteron_dpdk_fake_step() < 0)
    {
        return NULL;
    }
    rusteron_dpdk_fake_next_pool += 0x10;
    /* Its mbufs hold the room asked for, unless capped (a pool, or a PMD,
     * that gives less), within the fake's buffers. */
    uint32_t room = data_room;
    if (0 != rusteron_dpdk_fake_room_cap && room > rusteron_dpdk_fake_room_cap)
    {
        room = rusteron_dpdk_fake_room_cap;
    }
    if (room > RUSTERON_DPDK_FAKE_BUFFER_CAPACITY)
    {
        room = RUSTERON_DPDK_FAKE_BUFFER_CAPACITY;
    }
    const int slot = rusteron_dpdk_fake_pool_slot((void *)rusteron_dpdk_fake_next_pool);
    if (slot >= 0)
    {
        rusteron_dpdk_fake_pool_room[slot] = room;
    }
    return (void *)rusteron_dpdk_fake_next_pool;
}

static uint32_t rusteron_dpdk_fake_mbuf_room(void *mempool)
{
    return rusteron_dpdk_fake_room(mempool);
}

static int rusteron_dpdk_fake_rx_queue_setup(
    uint16_t port_id, uint16_t queue_id, uint16_t nb_desc, void *mempool, int socket_id, uint16_t free_thresh)
{
    rusteron_dpdk_fake_last_rx_socket = socket_id;
    rusteron_dpdk_fake_logf("rx %u (q=%u desc=%u)", port_id, queue_id, nb_desc);
    if (port_id < 4)
    {
        rusteron_dpdk_fake_rx_pool[port_id] = mempool;
        rusteron_dpdk_fake_rx_free_of[port_id] = free_thresh;
    }
    return rusteron_dpdk_fake_step();
}

static int rusteron_dpdk_fake_tx_queue_setup(
    uint16_t port_id, uint16_t queue_id, uint16_t nb_desc, int socket_id, uint16_t free_thresh)
{
    if (port_id < 4)
    {
        rusteron_dpdk_fake_tx_free_of[port_id] = free_thresh;
    }
    rusteron_dpdk_fake_logf("tx %u (q=%u desc=%u)", port_id, queue_id, nb_desc);
    rusteron_dpdk_fake_logf("numa-socket port %u pool %d rx %d tx %d", port_id, rusteron_dpdk_fake_last_pool_socket,
                            rusteron_dpdk_fake_last_rx_socket, socket_id);
    return rusteron_dpdk_fake_step();
}

static int rusteron_dpdk_fake_dev_set_mtu(uint16_t port_id, uint16_t mtu)
{
    rusteron_dpdk_fake_logf("mtu %u (%u)", port_id, mtu);
    return rusteron_dpdk_fake_step();
}

static int rusteron_dpdk_fake_dev_start(uint16_t port_id)
{
    rusteron_dpdk_fake_logf("start %u", port_id);
    return rusteron_dpdk_fake_step();
}

static int rusteron_dpdk_fake_link_wait_ready(uint16_t port_id, uint32_t timeout_ms)
{
    (void)timeout_ms;
    rusteron_dpdk_fake_logf("link %u", port_id);
    return rusteron_dpdk_fake_step();
}

static int rusteron_dpdk_fake_dev_stop(uint16_t port_id)
{
    rusteron_dpdk_fake_logf("stop %u", port_id);
    return 0;
}

static int rusteron_dpdk_fake_dev_close(uint16_t port_id)
{
    rusteron_dpdk_fake_logf("close %u", port_id);
    return 0;
}

static void rusteron_dpdk_fake_mempool_free(void *mempool)
{
    rusteron_dpdk_fake_logf("free %p", mempool);
}

/* Data path (plan §7.4). */

static void rusteron_dpdk_fake_release_buffer(rusteron_dpdk_mbuf_t *m)
{
    if (NULL != m && NULL != m->opaque)
    {
        /* opaque is a 1-based buffer index (see mbuf_alloc): 0 would alias NULL
         * and this guard would silently skip the release/count. */
        rusteron_dpdk_fake_buffers[(uintptr_t)m->opaque - 1].used = 0;
        m->opaque = NULL;
        rusteron_dpdk_fake_released_count++;
    }
}

static int rusteron_dpdk_fake_mbuf_alloc(void *mempool, rusteron_dpdk_mbuf_t *m)
{
    /* One shared set of buffers backs every role mempool in the fake; each
     * mbuf offers its pool's data room of one. */
    int live = rusteron_dpdk_fake_allocated_count - rusteron_dpdk_fake_released_count;
    if (live >= rusteron_dpdk_fake_pool_avail)
    {
        return -1; /* pool exhaustion */
    }

    for (int i = 0; i < RUSTERON_DPDK_FAKE_BUFFER_COUNT; i++)
    {
        if (!rusteron_dpdk_fake_buffers[i].used)
        {
            rusteron_dpdk_fake_buffers[i].used = 1;
            rusteron_dpdk_fake_buffers[i].pool = mempool;
            /* 1-based handle: index 0 would cast to NULL and defeat the fake's
             * own NULL-opaque release guard (and leak-accounting asserts). */
            m->opaque = (void *)(uintptr_t)(i + 1);
            m->data = rusteron_dpdk_fake_buffers[i].storage;
            m->capacity = rusteron_dpdk_fake_room(mempool);
            m->frame_len = 0;
            m->ol_flags = 0;
            m->l2_len = 0;
            m->l3_len = 0;
            m->l4_len = 0;
            m->udp_pseudo_csum = 0;
            rusteron_dpdk_fake_allocated_count++;
            return 0;
        }
    }
    return -1;
}

static void rusteron_dpdk_fake_mbuf_release(rusteron_dpdk_mbuf_t *m)
{
    rusteron_dpdk_fake_release_buffer(m);
}

/* The port's gateway (rusteron_dpdk_fake_set_arp_responder): an ARP
 * request sent on it is answered from 02:00:00:00:00:f<port>, queued for the
 * port's next rx_burst. */
static void rusteron_dpdk_fake_answer_arp(uint16_t port_id, const rusteron_dpdk_mbuf_t *m)
{
    const uint8_t *f = m->data;
    if (m->frame_len < 42 || 0x08 != f[12] || 0x06 != f[13] || 0x00 != f[20] || 0x01 != f[21])
    {
        return;
    }
    uint8_t r[42];
    const uint8_t mac[6] = { 0x02, 0x00, 0x00, 0x00, 0x00, (uint8_t)(0xf0 | port_id) };
    memcpy(r, f + 22, 6);       /* to the requester */
    memcpy(r + 6, mac, 6);
    memcpy(r + 12, f + 12, 8);  /* ARP, Ethernet/IPv4, 6, 4 */
    r[20] = 0x00;
    r[21] = 0x02;               /* reply */
    memcpy(r + 22, mac, 6);     /* sender: the gateway */
    memcpy(r + 28, f + 38, 4);  /* at the address asked for */
    memcpy(r + 32, f + 22, 10); /* target: the requester */
    (void)rusteron_dpdk_fake_rx_inject(port_id, r, sizeof(r), 0, 1);
}

static uint16_t rusteron_dpdk_fake_tx_burst(
    uint16_t port_id, uint16_t tx_queue_id,
    rusteron_dpdk_mbuf_t **pkts, uint16_t nb)
{
    (void)tx_queue_id;
    uint16_t cap = rusteron_dpdk_fake_tx_burst_cap;
    uint16_t accepted = nb < cap ? nb : cap;
    if (accepted > 0 && 0 != rusteron_dpdk_fake_tx_burst_ns)
    {
        rusteron_dpdk_test_advance_clock_ns(rusteron_dpdk_fake_tx_burst_ns);
    }
    /* MBUF_FAST_FREE (1ULL << 2, internal.h) frees what the NIC sent in bulk
     * to one pool: each mbuf must be the port's pool's, held by the sender
     * alone (allocated, not freed, not twice in the burst) and one segment. */
    if (port_id < 4 && 0 != (rusteron_dpdk_fake_tx_offloads_of[port_id] & (1ULL << 2)))
    {
        for (uint16_t i = 0; i < accepted; i++)
        {
            const rusteron_dpdk_mbuf_t *m = pkts[i];
            const uintptr_t h = (uintptr_t)m->opaque;
            int bad = 0 == h || h > RUSTERON_DPDK_FAKE_BUFFER_COUNT || !rusteron_dpdk_fake_buffers[h - 1].used ||
                rusteron_dpdk_fake_buffers[h - 1].pool != rusteron_dpdk_fake_rx_pool[port_id] ||
                m->frame_len > m->capacity;
            for (uint16_t j = 0; j < i && !bad; j++)
            {
                bad = pkts[j]->opaque == m->opaque;
            }
            rusteron_dpdk_fake_fast_free_bad += bad;
        }
    }

    /* Accepted prefix: the "NIC" owns these — record them and recycle the
     * buffers. The rejected tail stays the caller's, as rte_eth_tx_burst's. */
    for (uint16_t i = 0; i < accepted; i++)
    {
        rusteron_dpdk_mbuf_t *m = pkts[i];
        if (port_id < 4 && rusteron_dpdk_fake_arp_responder[port_id])
        {
            rusteron_dpdk_fake_answer_arp(port_id, m);
        }
        if (rusteron_dpdk_fake_capture_len < RUSTERON_DPDK_FAKE_CAPTURE_MAX)
        {
            rusteron_dpdk_fake_capture_t *c = &rusteron_dpdk_fake_capture[rusteron_dpdk_fake_capture_len++];
            uint32_t len = m->frame_len;
            memset(c, 0, sizeof(*c));
            memcpy(c->data, m->data, len);
            c->len = len;
            c->ol_flags = m->ol_flags;
            c->l2_len = m->l2_len;
            c->l3_len = m->l3_len;
            c->l4_len = m->l4_len;
            c->udp_pseudo_csum = m->udp_pseudo_csum;
            c->port_id = port_id;
        }
        rusteron_dpdk_fake_release_buffer(m);
    }
    return accepted;
}

int rusteron_dpdk_fake_rx_inject_hw_ts(
    uint16_t port_id, const uint8_t *frame, size_t len,
    uint32_t rx_ol_flags, uint32_t nb_segs, uint64_t hw_rx_ns);

int rusteron_dpdk_fake_rx_inject(
    uint16_t port_id, const uint8_t *frame, size_t len,
    uint32_t rx_ol_flags, uint32_t nb_segs)
{
    return rusteron_dpdk_fake_rx_inject_hw_ts(port_id, frame, len, rx_ol_flags, nb_segs, 0);
}

int rusteron_dpdk_fake_rx_inject_hw_ts(
    uint16_t port_id, const uint8_t *frame, size_t len,
    uint32_t rx_ol_flags, uint32_t nb_segs, uint64_t hw_rx_ns)
{
    if (port_id >= RUSTERON_DPDK_FAKE_PORTS || NULL == frame ||
        len > RUSTERON_DPDK_FAKE_BUFFER_CAPACITY)
    {
        return -1;
    }
    int tail = rusteron_dpdk_fake_rx_tail[port_id];
    if ((tail + 1) % RUSTERON_DPDK_FAKE_RX_DEPTH == rusteron_dpdk_fake_rx_head[port_id])
    {
        return -1; /* queue full */
    }
    rusteron_dpdk_fake_rx_entry_t *entry = &rusteron_dpdk_fake_rx_q[port_id][tail];
    memcpy(entry->data, frame, len);
    entry->len = (uint32_t)len;
    entry->rx_ol_flags = rx_ol_flags;
    entry->nb_segs = nb_segs;
    entry->hw_rx_ns = hw_rx_ns;
    rusteron_dpdk_fake_rx_tail[port_id] = (tail + 1) % RUSTERON_DPDK_FAKE_RX_DEPTH;
    return 0;
}

int rusteron_dpdk_fake_rx_queued(uint16_t port_id)
{
    if (port_id >= RUSTERON_DPDK_FAKE_PORTS)
    {
        return 0;
    }
    int head = rusteron_dpdk_fake_rx_head[port_id];
    int tail = rusteron_dpdk_fake_rx_tail[port_id];
    return (tail - head + RUSTERON_DPDK_FAKE_RX_DEPTH) % RUSTERON_DPDK_FAKE_RX_DEPTH;
}

static uint16_t rusteron_dpdk_fake_rx_burst(
    uint16_t port_id, uint16_t rx_queue_id,
    rusteron_dpdk_mbuf_t **pkts, uint16_t nb)
{
    (void)rx_queue_id;
    if (port_id >= RUSTERON_DPDK_FAKE_PORTS)
    {
        return 0;
    }

    uint16_t received = 0;
    while (received < nb &&
           rusteron_dpdk_fake_rx_head[port_id] != rusteron_dpdk_fake_rx_tail[port_id])
    {
        rusteron_dpdk_fake_rx_entry_t *entry = &rusteron_dpdk_fake_rx_q[port_id][rusteron_dpdk_fake_rx_head[port_id]];
        rusteron_dpdk_mbuf_t *m = pkts[received];
        if (rusteron_dpdk_fake_mbuf_alloc(rusteron_dpdk_fake_rx_pool[port_id], m) < 0)
        {
            break; /* pool exhausted: deliver what we have */
        }
        /* A frame larger than the queue's mbufs arrives in several of them,
         * as ENA's always scattered receive delivers it: the first one full,
         * the frame's length and segment count in it. */
        const uint32_t segs = (entry->len + m->capacity - 1) / m->capacity;
        memcpy(m->data, entry->data, entry->len < m->capacity ? entry->len : m->capacity);
        m->frame_len = entry->len;
        m->nb_segs = segs > entry->nb_segs ? segs : entry->nb_segs;
        m->rx_ol_flags = entry->rx_ol_flags;
        m->hw_rx_ns = entry->hw_rx_ns;
        rusteron_dpdk_fake_rx_head[port_id] =
            (rusteron_dpdk_fake_rx_head[port_id] + 1) % RUSTERON_DPDK_FAKE_RX_DEPTH;
        received++;
    }
    if (received > 0 && 0 != rusteron_dpdk_fake_rx_burst_ns)
    {
        rusteron_dpdk_test_advance_clock_ns(rusteron_dpdk_fake_rx_burst_ns);
    }
    return received;
}

int rusteron_dpdk_fake_tx_foreign(uint16_t port_id)
{
    /* An mbuf of the other role's pool (port 0 is the sender, 1 the
     * receiver), sent on `port_id`: what MBUF_FAST_FREE forbids. */
    if (port_id >= 2)
    {
        return -1;
    }
    rusteron_dpdk_mbuf_t m;
    if (rusteron_dpdk_fake_mbuf_alloc(rusteron_dpdk_fake_rx_pool[1 - port_id], &m) < 0)
    {
        return -1;
    }
    m.frame_len = 60;
    memset(m.data, 0, m.frame_len);
    rusteron_dpdk_mbuf_t *burst[1] = { &m };
    return 1 == rusteron_dpdk_fake_tx_burst(port_id, 0, burst, 1) ? 0 : -1;
}

/* Extended statistics (plan §9): mirror the installed fake xstat table. */
static uint32_t rusteron_dpdk_fake_xstats_count(uint16_t port_id)
{
    (void)port_id;
    return rusteron_dpdk_fake_xstat_count;
}

static int rusteron_dpdk_fake_xstats_names(uint16_t port_id, char *names, uint32_t count)
{
    (void)port_id;
    if (count > rusteron_dpdk_fake_xstat_count)
    {
        return -1;
    }
    for (uint32_t i = 0; i < count; i++)
    {
        memcpy(
            names + (i * RUSTERON_DPDK_XSTAT_NAME_LEN),
            rusteron_dpdk_fake_xstat_names[i], RUSTERON_DPDK_XSTAT_NAME_LEN);
    }
    return 0;
}

static int rusteron_dpdk_fake_xstats_get(uint16_t port_id, uint64_t *values, uint32_t count)
{
    (void)port_id;
    if (count > rusteron_dpdk_fake_xstat_count)
    {
        return -1;
    }
    for (uint32_t i = 0; i < count; i++)
    {
        values[i] = rusteron_dpdk_fake_xstat_values[i];
    }
    return 0;
}

/* Free mbufs = configured pool capacity minus live allocations. */
static uint32_t rusteron_dpdk_fake_mempool_avail(void *mempool)
{
    (void)mempool;
    int live = rusteron_dpdk_fake_allocated_count - rusteron_dpdk_fake_released_count;
    int avail = rusteron_dpdk_fake_pool_avail - live;
    return avail > 0 ? (uint32_t)avail : 0;
}

rusteron_dpdk_port_ops_t *rusteron_dpdk_port_ops_real(void)
{
    static rusteron_dpdk_port_ops_t ops = {
        .probe_port = rusteron_dpdk_fake_probe_port,
        .dev_info = rusteron_dpdk_fake_dev_info,
        .dev_configure = rusteron_dpdk_fake_dev_configure,
        .socket_id = rusteron_dpdk_fake_socket_id,
        .mempool_create = rusteron_dpdk_fake_mempool_create,
        .mbuf_room = rusteron_dpdk_fake_mbuf_room,
        .rx_queue_setup = rusteron_dpdk_fake_rx_queue_setup,
        .tx_queue_setup = rusteron_dpdk_fake_tx_queue_setup,
        .dev_set_mtu = rusteron_dpdk_fake_dev_set_mtu,
        .dev_start = rusteron_dpdk_fake_dev_start,
        .link_wait_ready = rusteron_dpdk_fake_link_wait_ready,
        .dev_stop = rusteron_dpdk_fake_dev_stop,
        .dev_close = rusteron_dpdk_fake_dev_close,
        .mempool_free = rusteron_dpdk_fake_mempool_free,
        .mbuf_alloc = rusteron_dpdk_fake_mbuf_alloc,
        .mbuf_release = rusteron_dpdk_fake_mbuf_release,
        .tx_burst = rusteron_dpdk_fake_tx_burst,
        .rx_burst = rusteron_dpdk_fake_rx_burst,
        .xstats_count = rusteron_dpdk_fake_xstats_count,
        .xstats_names = rusteron_dpdk_fake_xstats_names,
        .xstats_get = rusteron_dpdk_fake_xstats_get,
        .mempool_avail = rusteron_dpdk_fake_mempool_avail,
        .basic_stats = rusteron_dpdk_fake_basic_stats,
        .timer_service = rusteron_dpdk_fake_timer_service,
        .watch_reset = rusteron_dpdk_fake_watch_reset,
        .unwatch_reset = rusteron_dpdk_fake_unwatch_reset,
        .lsc_capable = rusteron_dpdk_fake_lsc_capable,
        .watch_lsc = rusteron_dpdk_fake_watch_lsc,
        .unwatch_lsc = rusteron_dpdk_fake_unwatch_lsc,
    };
    return &ops;
}

/* The transport clock as it was before the counter clock (runtime.c), for the
 * clock benchmark's before/after comparison: CLOCK_MONOTONIC in milliseconds. */
uint64_t rusteron_dpdk_fake_legacy_clock_ms(void)
{
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000u + (uint64_t)ts.tv_nsec / 1000000u;
}
