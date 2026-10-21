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
} rusteron_dpdk_fake_buffer_t;

static int rusteron_dpdk_fake_failure_step = 0;
static int rusteron_dpdk_fake_call_count = 0;
static char rusteron_dpdk_fake_driver[64] = "net_ena";
static int rusteron_dpdk_fake_csum_ok = 1;
static int rusteron_dpdk_fake_rx_timestamp_ok = 0;
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
    rusteron_dpdk_fake_ena_llq = RUSTERON_DPDK_ENA_LLQ_WC;
    rusteron_dpdk_fake_max_mtu = 9001;
    memset(rusteron_dpdk_fake_desc_lim, 0, sizeof(rusteron_dpdk_fake_desc_lim));
    memset(rusteron_dpdk_fake_basic, 0, sizeof(rusteron_dpdk_fake_basic));
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

void rusteron_dpdk_fake_set_basic_stats(uint64_t ierrors, uint64_t imissed, uint64_t rx_nombuf)
{
    rusteron_dpdk_fake_basic[0] = ierrors;
    rusteron_dpdk_fake_basic[1] = imissed;
    rusteron_dpdk_fake_basic[2] = rx_nombuf;
}

static int rusteron_dpdk_fake_basic_stats(uint16_t port_id, uint64_t *ierrors, uint64_t *imissed, uint64_t *rx_nombuf)
{
    (void)port_id;
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
    int *csum_offload_ok, int *ena_llq, int *rx_timestamp_ok, uint16_t *max_mtu,
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
    *max_mtu = rusteron_dpdk_fake_max_mtu;
    memcpy(desc_lim, rusteron_dpdk_fake_desc_lim, sizeof(rusteron_dpdk_fake_desc_lim));
    return 0;
}

static int rusteron_dpdk_fake_dev_configure(
    uint16_t port_id, uint16_t rx_rings, uint16_t tx_rings,
    uint64_t rx_offloads, uint64_t tx_offloads)
{
    (void)tx_offloads;
    /* 1ULL << 2 is RUSTERON_DPDK_RX_OFFLOAD_TIMESTAMP (internal.h). */
    rusteron_dpdk_fake_logf("configure %u (rxr=%u txr=%u)%s", port_id, rx_rings, tx_rings,
                            0 != (rx_offloads & (1ULL << 2)) ? " rx-timestamp" : "");
    return rusteron_dpdk_fake_step();
}

static void *rusteron_dpdk_fake_mempool_create(const char *name, uint32_t n, uint16_t cache_size, uint16_t data_room)
{
    (void)cache_size;
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
    uint16_t port_id, uint16_t queue_id, uint16_t nb_desc, void *mempool)
{
    rusteron_dpdk_fake_logf("rx %u (q=%u desc=%u)", port_id, queue_id, nb_desc);
    if (port_id < 4)
    {
        rusteron_dpdk_fake_rx_pool[port_id] = mempool;
    }
    return rusteron_dpdk_fake_step();
}

static int rusteron_dpdk_fake_tx_queue_setup(
    uint16_t port_id, uint16_t queue_id, uint16_t nb_desc)
{
    rusteron_dpdk_fake_logf("tx %u (q=%u desc=%u)", port_id, queue_id, nb_desc);
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

static uint16_t rusteron_dpdk_fake_tx_burst(
    uint16_t port_id, uint16_t tx_queue_id,
    rusteron_dpdk_mbuf_t **pkts, uint16_t nb)
{
    (void)tx_queue_id;
    uint16_t cap = rusteron_dpdk_fake_tx_burst_cap;
    uint16_t accepted = nb < cap ? nb : cap;

    /* Accepted prefix: the "NIC" owns these — record them and recycle the
     * buffers. The rejected tail stays the caller's, as rte_eth_tx_burst's. */
    for (uint16_t i = 0; i < accepted; i++)
    {
        rusteron_dpdk_mbuf_t *m = pkts[i];
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
    return received;
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
