/*
 * ergon-md-dpdk — the lab's per-stage trace and in-process packet capture
 * (rusteron_dpdk_stages.h).
 *
 * The recording threads only fill preallocated slots of rings of their own;
 * the writer thread (or the close) writes the files.
 */
#define _GNU_SOURCE /* syscall(SYS_gettid) */
#include "rusteron_dpdk_internal.h"

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/syscall.h>
#include <time.h>
#include <unistd.h>

#include "aeronmd.h"

uint32_t rusteron_dpdk_lab_hooks = 0;
uint64_t rusteron_dpdk_stage_mask = 0;
_Thread_local uint64_t rusteron_dpdk_stage_send_tsc = 0;

/* ---- rings ---- */

typedef struct rusteron_dpdk_stage_ring_stct
{
    uint64_t head __attribute__((aligned(64))); /* writer: records written */
    uint64_t drops;                              /* writer: records dropped, full */
    uint64_t tail __attribute__((aligned(64))); /* reader: records drained */
    uint64_t drops_logged;                       /* reader: drops the file has */
    rusteron_dpdk_stage_record_t *records __attribute__((aligned(64)));
    uint64_t mask;
    uint32_t tid;       /* the writer's kernel id, published by `used` */
    uint32_t used;      /* claimed (release) */
    uint32_t announced; /* reader: its thread is in the file */
} rusteron_dpdk_stage_ring_t;

typedef struct rusteron_dpdk_capture_slot_stct
{
    uint64_t tsc;
    uint32_t orig_len;
    uint16_t caplen;
    uint8_t port;
    uint8_t outbound;
    uint8_t data[RUSTERON_DPDK_CAPTURE_SNAP];
} rusteron_dpdk_capture_slot_t;

typedef struct rusteron_dpdk_capture_ring_stct
{
    uint64_t head __attribute__((aligned(64)));
    uint64_t drops;
    uint64_t tail __attribute__((aligned(64)));
    uint64_t drops_logged;
    rusteron_dpdk_capture_slot_t *slots __attribute__((aligned(64)));
    uint64_t mask;
    uint32_t used;
} rusteron_dpdk_capture_ring_t;

typedef struct rusteron_dpdk_lab_stct
{
    int open;
    int kernel;
    uint64_t generation; /* bumped at each open: threads claim anew */
    FILE *stage_file;
    FILE *capture_file;
    rusteron_dpdk_stage_ring_t stage_rings[RUSTERON_DPDK_STAGE_RINGS];
    uint32_t stage_claimed;
    uint64_t stage_threads_lost;
    rusteron_dpdk_capture_ring_t capture_rings[RUSTERON_DPDK_CAPTURE_RINGS];
    uint32_t capture_claimed;
    uint64_t capture_threads_lost;
    uint64_t capture_start_ns; /* the window, from open (CLOCK_MONOTONIC) */
    uint64_t capture_end_ns;
    int capture_done;          /* the window has closed */
    uint64_t capture_written;
    /* the clock at open: counter, CLOCK_MONOTONIC_RAW, CLOCK_MONOTONIC */
    uint64_t open_tsc;
    uint64_t open_raw_ns;
    uint64_t open_mono_ns;
    pthread_t writer;
    int writer_running;
    volatile int writer_stop;
} rusteron_dpdk_lab_t;

static rusteron_dpdk_lab_t rusteron_dpdk_lab;
static int rusteron_dpdk_lab_no_thread = 0;
static uint32_t rusteron_dpdk_lab_ring_records = 0;

static _Thread_local rusteron_dpdk_stage_ring_t *rusteron_dpdk_stage_tls_ring = NULL;
static _Thread_local uint64_t rusteron_dpdk_stage_tls_generation = 0;
static _Thread_local rusteron_dpdk_capture_ring_t *rusteron_dpdk_capture_tls_ring = NULL;
static _Thread_local uint64_t rusteron_dpdk_capture_tls_generation = 0;

void rusteron_dpdk_lab_test_set_no_thread(int no_thread)
{
    rusteron_dpdk_lab_no_thread = no_thread;
}

void rusteron_dpdk_lab_test_set_ring_records(uint32_t records)
{
    rusteron_dpdk_lab_ring_records = records;
}

void rusteron_dpdk_lab_test_reset(void)
{
    rusteron_dpdk_lab_close();
    rusteron_dpdk_lab_no_thread = 1;
    rusteron_dpdk_lab_ring_records = 0;
}

static uint64_t rusteron_dpdk_lab_clock(clockid_t id)
{
    struct timespec ts;
    clock_gettime(id, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
}

static uint32_t rusteron_dpdk_lab_tid(void)
{
    return (uint32_t)syscall(SYS_gettid);
}

static inline uint32_t rusteron_dpdk_le32_at(const uint8_t *p)
{
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static inline uint64_t rusteron_dpdk_le64_at(const uint8_t *p)
{
    return (uint64_t)rusteron_dpdk_le32_at(p) | ((uint64_t)rusteron_dpdk_le32_at(p + 4) << 32);
}

/* The calling thread's stage ring, claimed at its first record of this
 * open; NULL when every ring is taken (counted). */
static rusteron_dpdk_stage_ring_t *rusteron_dpdk_stage_ring(void)
{
    const uint64_t generation = __atomic_load_n(&rusteron_dpdk_lab.generation, __ATOMIC_ACQUIRE);
    if (rusteron_dpdk_stage_tls_generation == generation)
    {
        return rusteron_dpdk_stage_tls_ring;
    }
    rusteron_dpdk_stage_tls_generation = generation;
    rusteron_dpdk_stage_tls_ring = NULL;
    const uint32_t i = __atomic_fetch_add(&rusteron_dpdk_lab.stage_claimed, 1, __ATOMIC_RELAXED);
    if (i >= RUSTERON_DPDK_STAGE_RINGS)
    {
        __atomic_fetch_add(&rusteron_dpdk_lab.stage_threads_lost, 1, __ATOMIC_RELAXED);
        return NULL;
    }
    rusteron_dpdk_stage_ring_t *ring = &rusteron_dpdk_lab.stage_rings[i];
    ring->tid = rusteron_dpdk_lab_tid();
    __atomic_store_n(&ring->used, 1, __ATOMIC_RELEASE);
    rusteron_dpdk_stage_tls_ring = ring;
    return ring;
}

static rusteron_dpdk_capture_ring_t *rusteron_dpdk_capture_ring(void)
{
    const uint64_t generation = __atomic_load_n(&rusteron_dpdk_lab.generation, __ATOMIC_ACQUIRE);
    if (rusteron_dpdk_capture_tls_generation == generation)
    {
        return rusteron_dpdk_capture_tls_ring;
    }
    rusteron_dpdk_capture_tls_generation = generation;
    rusteron_dpdk_capture_tls_ring = NULL;
    const uint32_t i = __atomic_fetch_add(&rusteron_dpdk_lab.capture_claimed, 1, __ATOMIC_RELAXED);
    if (i >= RUSTERON_DPDK_CAPTURE_RINGS)
    {
        __atomic_fetch_add(&rusteron_dpdk_lab.capture_threads_lost, 1, __ATOMIC_RELAXED);
        return NULL;
    }
    rusteron_dpdk_capture_ring_t *ring = &rusteron_dpdk_lab.capture_rings[i];
    __atomic_store_n(&ring->used, 1, __ATOMIC_RELEASE);
    rusteron_dpdk_capture_tls_ring = ring;
    return ring;
}

static inline void rusteron_dpdk_stage_put(
    rusteron_dpdk_stage_ring_t *ring, uint8_t stage, uint64_t tsc, uint64_t seq, int32_t stream_id,
    int32_t term_offset, uint32_t value)
{
    const uint64_t head = ring->head; /* this thread's alone */
    if (head - __atomic_load_n(&ring->tail, __ATOMIC_ACQUIRE) > ring->mask)
    {
        __atomic_store_n(&ring->drops, ring->drops + 1, __ATOMIC_RELAXED);
        return;
    }
    rusteron_dpdk_stage_record_t *r = &ring->records[head & ring->mask];
    r->tsc = tsc;
    r->seq = seq;
    r->stream_id = stream_id;
    r->term_offset = term_offset;
    r->value = value;
    r->stage = stage;
    __atomic_store_n(&ring->head, head + 1, __ATOMIC_RELEASE);
}

/* Aeron's frame header: frame_length (0), type (6), term_offset (8),
 * stream_id (16); the payload at 32. aeron_lat's message: seq (0), kind
 * (28). */
#define RUSTERON_DPDK_STAGE_HDR 32u
#define RUSTERON_DPDK_STAGE_MSG 32u

/* Each sampled aeron_lat message among the Aeron frames of datagram `d`
 * (`len` bytes), into `keys` (`n` held, at most `max`), each with `frame`;
 * returns the new count. */
static int rusteron_dpdk_stage_walk(
    const uint8_t *d, size_t len, uint16_t frame, rusteron_dpdk_stage_key_t *keys, int n, int max)
{
    const uint64_t mask = rusteron_dpdk_stage_mask;
    size_t off = 0;
    while (off + RUSTERON_DPDK_STAGE_HDR <= len && n < max)
    {
        const int32_t frame_length = (int32_t)rusteron_dpdk_le32_at(d + off);
        const uint16_t type = (uint16_t)(d[off + 6] | (d[off + 7] << 8));
        if (0x01 != type && 0x00 != type)
        {
            break; /* a control frame: alone in its datagram */
        }
        if (frame_length <= 0)
        {
            break; /* a heartbeat: the datagram's last */
        }
        if (0x01 == type && (size_t)frame_length >= RUSTERON_DPDK_STAGE_HDR + RUSTERON_DPDK_STAGE_MSG &&
            off + RUSTERON_DPDK_STAGE_HDR + RUSTERON_DPDK_STAGE_MSG <= len)
        {
            const uint8_t *p = d + off + RUSTERON_DPDK_STAGE_HDR;
            const uint32_t kind = rusteron_dpdk_le32_at(p + 28);
            const uint64_t seq = rusteron_dpdk_le64_at(p);
            if (kind >= 1 && kind <= 3 && 0 == (seq & mask))
            {
                keys[n].seq = seq;
                keys[n].stream_id = (int32_t)rusteron_dpdk_le32_at(d + off + 16);
                keys[n].term_offset = (int32_t)rusteron_dpdk_le32_at(d + off + 8);
                keys[n].frame = frame;
                n++;
            }
        }
        off += ((size_t)frame_length + 31u) & ~(size_t)31u;
    }
    return n;
}

void rusteron_dpdk_stage_put_keys(
    const rusteron_dpdk_stage_key_t *keys, int n, uint16_t below, const uint8_t *stages, const uint64_t *tscs,
    int stamps, uint32_t value)
{
    rusteron_dpdk_stage_ring_t *ring = NULL;
    for (int k = 0; k < n; k++)
    {
        if (keys[k].frame >= below)
        {
            continue;
        }
        if (NULL == ring && NULL == (ring = rusteron_dpdk_stage_ring()))
        {
            return;
        }
        for (int i = 0; i < stamps; i++)
        {
            rusteron_dpdk_stage_put(ring, stages[i], tscs[i], keys[k].seq, keys[k].stream_id, keys[k].term_offset, value);
        }
    }
}

void rusteron_dpdk_stage_datagram(
    const uint8_t *d, size_t len, const uint8_t *stages, const uint64_t *tscs, int n, uint32_t value)
{
    rusteron_dpdk_stage_key_t keys[RUSTERON_DPDK_STAGE_KEYS_MAX];
    const int found = rusteron_dpdk_stage_walk(d, len, 0, keys, 0, RUSTERON_DPDK_STAGE_KEYS_MAX);
    rusteron_dpdk_stage_put_keys(keys, found, 1, stages, tscs, n, value);
}

int rusteron_dpdk_stage_frame_keys(
    const uint8_t *frame, size_t len, uint16_t index, rusteron_dpdk_stage_key_t *keys, int n, int max)
{
    /* Ethernet II, IPv4, UDP: the frames the transport builds */
    if (len < 14 + 20 + 8 || 0x08 != frame[12] || 0x00 != frame[13] || 17 != frame[14 + 9])
    {
        return n;
    }
    const size_t ihl = (size_t)(frame[14] & 0x0f) * 4u;
    const size_t at = 14 + ihl + 8;
    if (ihl < 20 || at > len)
    {
        return n;
    }
    return rusteron_dpdk_stage_walk(frame + at, len - at, index, keys, n, max);
}

static void rusteron_dpdk_capture_put(
    rusteron_dpdk_capture_ring_t *ring, int port, int outbound, const uint8_t *data, uint32_t len, uint64_t tsc)
{
    const uint64_t head = ring->head;
    if (head - __atomic_load_n(&ring->tail, __ATOMIC_ACQUIRE) > ring->mask)
    {
        __atomic_store_n(&ring->drops, ring->drops + 1, __ATOMIC_RELAXED);
        return;
    }
    rusteron_dpdk_capture_slot_t *s = &ring->slots[head & ring->mask];
    const uint32_t cap = len < RUSTERON_DPDK_CAPTURE_SNAP ? len : RUSTERON_DPDK_CAPTURE_SNAP;
    s->tsc = tsc;
    s->orig_len = len;
    s->caplen = (uint16_t)cap;
    s->port = (uint8_t)port;
    s->outbound = (uint8_t)outbound;
    memcpy(s->data, data, cap);
    __atomic_store_n(&ring->head, head + 1, __ATOMIC_RELEASE);
}

uint16_t rusteron_dpdk_capture_stage(int port, struct rusteron_dpdk_mbuf_stct *const *frames, uint16_t n)
{
    rusteron_dpdk_capture_ring_t *ring = rusteron_dpdk_capture_ring();
    if (NULL == ring)
    {
        return 0;
    }
    const uint64_t head = ring->head;
    const uint64_t room = ring->mask + 1 - (head - __atomic_load_n(&ring->tail, __ATOMIC_ACQUIRE));
    const uint16_t staged = room < n ? (uint16_t)room : n;
    for (uint16_t i = 0; i < staged; i++)
    {
        rusteron_dpdk_capture_slot_t *s = &ring->slots[(head + i) & ring->mask];
        const uint32_t len = frames[i]->frame_len;
        const uint32_t cap = len < RUSTERON_DPDK_CAPTURE_SNAP ? len : RUSTERON_DPDK_CAPTURE_SNAP;
        s->orig_len = len;
        s->caplen = (uint16_t)cap;
        s->port = (uint8_t)port;
        s->outbound = 1;
        memcpy(s->data, frames[i]->data, cap);
    }
    return staged;
}

void rusteron_dpdk_capture_publish(uint16_t staged, uint16_t sent, uint64_t tsc)
{
    rusteron_dpdk_capture_ring_t *ring = rusteron_dpdk_capture_ring();
    if (NULL == ring)
    {
        return;
    }
    const uint16_t n = sent < staged ? sent : staged;
    const uint64_t head = ring->head;
    for (uint16_t i = 0; i < n; i++)
    {
        ring->slots[(head + i) & ring->mask].tsc = tsc;
    }
    if (sent > staged)
    {
        __atomic_store_n(&ring->drops, ring->drops + (uint64_t)(sent - staged), __ATOMIC_RELAXED);
    }
    __atomic_store_n(&ring->head, head + n, __ATOMIC_RELEASE);
}

void rusteron_dpdk_capture_views(
    int port, int outbound, const struct rusteron_dpdk_mbuf_stct *views, uint16_t n, uint64_t tsc)
{
    rusteron_dpdk_capture_ring_t *ring = rusteron_dpdk_capture_ring();
    if (NULL == ring)
    {
        return;
    }
    for (uint16_t i = 0; i < n; i++)
    {
        rusteron_dpdk_capture_put(ring, port, outbound, views[i].data, views[i].frame_len, tsc);
    }
}

/* ---- the writer ---- */

static const char *rusteron_dpdk_stage_name(uint8_t stage)
{
    switch (stage)
    {
        case RUSTERON_DPDK_STAGE_TX_SEND: return "tx-send";
        case RUSTERON_DPDK_STAGE_TX_CALL: return "tx-call";
        case RUSTERON_DPDK_STAGE_TX_DONE: return "tx-done";
        case RUSTERON_DPDK_STAGE_RX_BURST: return "rx-burst";
        case RUSTERON_DPDK_STAGE_RX_HW: return "rx-hw";
        case RUSTERON_DPDK_STAGE_RX_DISPATCH: return "rx-dispatch";
        case RUSTERON_DPDK_STAGE_RX_INSERTED: return "rx-inserted";
        default: return "unknown";
    }
}

static void rusteron_dpdk_lab_clock_line(FILE *f, const char *when)
{
    const uint64_t tsc = rusteron_dpdk_counter();
    const uint64_t raw = rusteron_dpdk_lab_clock(CLOCK_MONOTONIC_RAW);
    const uint64_t rt = rusteron_dpdk_lab_clock(CLOCK_REALTIME);
    fprintf(f, "# clock %s tsc=%llu mono_raw_ns=%llu realtime_ns=%llu\n", when, (unsigned long long)tsc,
            (unsigned long long)raw, (unsigned long long)rt);
}

static void rusteron_dpdk_stage_drain_ring(FILE *f, rusteron_dpdk_stage_ring_t *ring, int index)
{
    if (!__atomic_load_n(&ring->used, __ATOMIC_ACQUIRE))
    {
        return;
    }
    if (!ring->announced)
    {
        char path[64], comm[32] = "?";
        snprintf(path, sizeof(path), "/proc/self/task/%u/comm", (unsigned)ring->tid);
        FILE *c = fopen(path, "r");
        if (NULL != c)
        {
            if (NULL != fgets(comm, sizeof(comm), c))
            {
                comm[strcspn(comm, "\n")] = '\0';
            }
            fclose(c);
        }
        fprintf(f, "# ring %d tid %u comm %s\n", index, (unsigned)ring->tid, comm);
        ring->announced = 1;
    }
    const uint64_t head = __atomic_load_n(&ring->head, __ATOMIC_ACQUIRE);
    for (uint64_t t = ring->tail; t != head; t++)
    {
        const rusteron_dpdk_stage_record_t *r = &ring->records[t & ring->mask];
        fprintf(f, "%s,%llu,%d,%llu,%d,%u,%d\n", rusteron_dpdk_stage_name(r->stage), (unsigned long long)r->tsc,
                r->stream_id, (unsigned long long)r->seq, r->term_offset, (unsigned)r->value, index);
    }
    __atomic_store_n(&ring->tail, head, __ATOMIC_RELEASE);
    const uint64_t drops = __atomic_load_n(&ring->drops, __ATOMIC_RELAXED);
    if (drops != ring->drops_logged)
    {
        fprintf(f, "# drops ring %d %llu\n", index, (unsigned long long)(drops - ring->drops_logged));
        ring->drops_logged = drops;
    }
}

/* pcapng: the section header, then one interface a DPDK port (Ethernet,
 * the snap length, ns stamps), each packet an enhanced packet block with
 * its direction in epb_flags. */
static void rusteron_dpdk_pcapng_u32(FILE *f, uint32_t v)
{
    fwrite(&v, sizeof(v), 1, f);
}

static void rusteron_dpdk_pcapng_header(FILE *f)
{
    /* SHB: type, length, byte-order magic, version 1.0, section length -1 */
    rusteron_dpdk_pcapng_u32(f, 0x0A0D0D0A);
    rusteron_dpdk_pcapng_u32(f, 28);
    rusteron_dpdk_pcapng_u32(f, 0x1A2B3C4D);
    rusteron_dpdk_pcapng_u32(f, 0x00000001); /* major 1, minor 0 */
    rusteron_dpdk_pcapng_u32(f, 0xFFFFFFFF);
    rusteron_dpdk_pcapng_u32(f, 0xFFFFFFFF);
    rusteron_dpdk_pcapng_u32(f, 28);
    static const char *names[2] = {"sender", "receiver"};
    for (int i = 0; i < 2; i++)
    {
        /* IDB: linktype 1 (Ethernet), snaplen; if_name (2, padded to 8),
         * if_tsresol (9: 10^-9), end */
        const uint32_t length = 20 + 12 + 8 + 4;
        rusteron_dpdk_pcapng_u32(f, 0x00000001);
        rusteron_dpdk_pcapng_u32(f, length);
        rusteron_dpdk_pcapng_u32(f, 0x00000001); /* linktype 1, reserved 0 */
        rusteron_dpdk_pcapng_u32(f, RUSTERON_DPDK_CAPTURE_SNAP);
        uint8_t name[12] = {2, 0, 8, 0};
        memcpy(name + 4, names[i], strlen(names[i]));
        fwrite(name, sizeof(name), 1, f);
        const uint8_t tsresol[8] = {9, 0, 1, 0, 9, 0, 0, 0};
        fwrite(tsresol, sizeof(tsresol), 1, f);
        rusteron_dpdk_pcapng_u32(f, 0); /* opt_endofopt */
        rusteron_dpdk_pcapng_u32(f, length);
    }
}

static void rusteron_dpdk_pcapng_packet(FILE *f, const rusteron_dpdk_capture_slot_t *s, uint64_t ns)
{
    const uint32_t padded = ((uint32_t)s->caplen + 3u) & ~3u;
    const uint32_t length = 28 + padded + 12 + 4;
    rusteron_dpdk_pcapng_u32(f, 0x00000006);
    rusteron_dpdk_pcapng_u32(f, length);
    rusteron_dpdk_pcapng_u32(f, s->port);
    rusteron_dpdk_pcapng_u32(f, (uint32_t)(ns >> 32));
    rusteron_dpdk_pcapng_u32(f, (uint32_t)ns);
    rusteron_dpdk_pcapng_u32(f, s->caplen);
    rusteron_dpdk_pcapng_u32(f, s->orig_len);
    fwrite(s->data, s->caplen, 1, f);
    static const uint8_t zero[4] = {0, 0, 0, 0};
    fwrite(zero, padded - s->caplen, 1, f);
    /* epb_flags (2): bits 0-1 the direction, 1 inbound, 2 outbound */
    const uint8_t flags[8] = {2, 0, 4, 0, (uint8_t)(s->outbound ? 2 : 1), 0, 0, 0};
    fwrite(flags, sizeof(flags), 1, f);
    rusteron_dpdk_pcapng_u32(f, 0);
    rusteron_dpdk_pcapng_u32(f, length);
}

/* The counter's ns per tick now: measured against CLOCK_MONOTONIC_RAW since
 * the open, once a millisecond has passed; else the transport clock's
 * factor, or 1. */
static double rusteron_dpdk_lab_ns_per_tick(uint64_t tsc, uint64_t raw_ns)
{
    const rusteron_dpdk_lab_t *lab = &rusteron_dpdk_lab;
    if (raw_ns > lab->open_raw_ns + 1000000ULL && tsc > lab->open_tsc)
    {
        return (double)(raw_ns - lab->open_raw_ns) / (double)(tsc - lab->open_tsc);
    }
    if (RUSTERON_DPDK_CLOCK_COUNTER == rusteron_dpdk_clock.mode && 0 != rusteron_dpdk_clock.mult)
    {
        return (double)rusteron_dpdk_clock.mult / 4294967296.0;
    }
    return 1.0;
}

static void rusteron_dpdk_capture_drain_ring(FILE *f, rusteron_dpdk_capture_ring_t *ring, uint64_t tsc_now,
                                             uint64_t rt_now, double ns_per_tick)
{
    if (!__atomic_load_n(&ring->used, __ATOMIC_ACQUIRE))
    {
        return;
    }
    const uint64_t head = __atomic_load_n(&ring->head, __ATOMIC_ACQUIRE);
    for (uint64_t t = ring->tail; t != head; t++)
    {
        const rusteron_dpdk_capture_slot_t *s = &ring->slots[t & ring->mask];
        const double back = (double)(int64_t)(tsc_now - s->tsc) * ns_per_tick;
        rusteron_dpdk_pcapng_packet(f, s, rt_now - (uint64_t)(int64_t)back);
        rusteron_dpdk_lab.capture_written++;
    }
    __atomic_store_n(&ring->tail, head, __ATOMIC_RELEASE);
    const uint64_t drops = __atomic_load_n(&ring->drops, __ATOMIC_RELAXED);
    if (drops != ring->drops_logged)
    {
        /* the count goes to stderr (the driver log): pcapng has no row for it */
        fprintf(stderr, "md-dpdk capture: a full ring dropped %llu frames\n",
                (unsigned long long)(drops - ring->drops_logged));
        ring->drops_logged = drops;
    }
}

void rusteron_dpdk_lab_drain(void)
{
    rusteron_dpdk_lab_t *lab = &rusteron_dpdk_lab;
    if (!lab->open)
    {
        return;
    }
    if (NULL != lab->capture_file && !lab->capture_done)
    {
        const uint64_t elapsed = rusteron_dpdk_lab_clock(CLOCK_MONOTONIC) - lab->open_mono_ns;
        if (elapsed >= lab->capture_end_ns)
        {
            __atomic_fetch_and(&rusteron_dpdk_lab_hooks, ~RUSTERON_DPDK_LAB_CAPTURE, __ATOMIC_RELAXED);
            lab->capture_done = 1;
        }
        else if (elapsed >= lab->capture_start_ns)
        {
            __atomic_fetch_or(&rusteron_dpdk_lab_hooks, RUSTERON_DPDK_LAB_CAPTURE, __ATOMIC_RELAXED);
        }
    }
    if (NULL != lab->stage_file)
    {
        for (int i = 0; i < RUSTERON_DPDK_STAGE_RINGS; i++)
        {
            rusteron_dpdk_stage_drain_ring(lab->stage_file, &lab->stage_rings[i], i);
        }
        fflush(lab->stage_file);
    }
    if (NULL != lab->capture_file)
    {
        const uint64_t tsc = rusteron_dpdk_counter();
        const uint64_t raw = rusteron_dpdk_lab_clock(CLOCK_MONOTONIC_RAW);
        const uint64_t rt = rusteron_dpdk_lab_clock(CLOCK_REALTIME);
        const double ns_per_tick = rusteron_dpdk_lab_ns_per_tick(tsc, raw);
        for (int i = 0; i < RUSTERON_DPDK_CAPTURE_RINGS; i++)
        {
            rusteron_dpdk_capture_drain_ring(lab->capture_file, &lab->capture_rings[i], tsc, rt, ns_per_tick);
        }
        fflush(lab->capture_file);
    }
}

static void *rusteron_dpdk_lab_writer(void *arg)
{
    (void)arg;
    while (!__atomic_load_n(&rusteron_dpdk_lab.writer_stop, __ATOMIC_ACQUIRE))
    {
        const struct timespec pass = {0, (long)RUSTERON_DPDK_LAB_PASS_NS};
        nanosleep(&pass, NULL);
        rusteron_dpdk_lab_drain();
    }
    return NULL;
}

/* ---- open and close ---- */

static int rusteron_dpdk_lab_fail(const char *message)
{
    rusteron_dpdk_set_error_code(message, RUSTERON_DPDK_ERR_INVALID_ENV);
    return -1;
}

static void rusteron_dpdk_lab_free(void)
{
    rusteron_dpdk_lab_t *lab = &rusteron_dpdk_lab;
    for (int i = 0; i < RUSTERON_DPDK_STAGE_RINGS; i++)
    {
        free(lab->stage_rings[i].records);
    }
    for (int i = 0; i < RUSTERON_DPDK_CAPTURE_RINGS; i++)
    {
        free(lab->capture_rings[i].slots);
    }
    if (NULL != lab->stage_file)
    {
        fclose(lab->stage_file);
    }
    if (NULL != lab->capture_file)
    {
        fclose(lab->capture_file);
    }
    const uint64_t generation = lab->generation;
    memset(lab, 0, sizeof(*lab));
    lab->generation = generation;
}

/* Every page of `bytes` at `p` written now, so no record faults one in. */
static void *rusteron_dpdk_lab_alloc(size_t bytes)
{
    void *p = aligned_alloc(64, (bytes + 63u) & ~(size_t)63u);
    if (NULL != p)
    {
        memset(p, 0, bytes);
    }
    return p;
}

static int rusteron_dpdk_lab_parse_window(const char *text, uint64_t *start_ms, uint64_t *length_ms)
{
    char *end = NULL;
    errno = 0;
    const unsigned long long start = strtoull(text, &end, 10);
    if (errno != 0 || end == text || ':' != *end)
    {
        return -1;
    }
    const char *rest = end + 1;
    const unsigned long long length = strtoull(rest, &end, 10);
    if (errno != 0 || end == rest || '\0' != *end || 0 == length || start > 86400000ULL || length > 600000ULL)
    {
        return -1;
    }
    *start_ms = start;
    *length_ms = length;
    return 0;
}

int rusteron_dpdk_lab_open(int kernel)
{
    rusteron_dpdk_lab_t *lab = &rusteron_dpdk_lab;
    if (lab->open)
    {
        return 0;
    }
    char message[512];
    const char *every_text = getenv("RUSTERON_DPDK_STAGE_TRACE");
    const char *stage_path = getenv("RUSTERON_DPDK_STAGE_TRACE_FILE");
    const char *capture_path = getenv("RUSTERON_DPDK_CAPTURE_FILE");
    const char *window_text = getenv("RUSTERON_DPDK_CAPTURE_WINDOW_MS");
    const int stages = NULL != every_text && '\0' != every_text[0];
    const int capture = NULL != capture_path && '\0' != capture_path[0];
    if (!stages && NULL != stage_path && '\0' != stage_path[0])
    {
        return rusteron_dpdk_lab_fail("RUSTERON_DPDK_STAGE_TRACE_FILE without RUSTERON_DPDK_STAGE_TRACE (<N>, a power of two)");
    }
    if (!capture && NULL != window_text && '\0' != window_text[0])
    {
        return rusteron_dpdk_lab_fail("RUSTERON_DPDK_CAPTURE_WINDOW_MS without RUSTERON_DPDK_CAPTURE_FILE");
    }
    if (!stages && !capture)
    {
        return 0;
    }
    uint64_t every = 0;
    if (stages)
    {
        char *end = NULL;
        errno = 0;
        every = strtoull(every_text, &end, 10);
        if (errno != 0 || end == every_text || '\0' != *end || 0 == every || 0 != (every & (every - 1)) ||
            every > RUSTERON_DPDK_STAGE_EVERY_MAX)
        {
            snprintf(message, sizeof(message),
                     "RUSTERON_DPDK_STAGE_TRACE=%s: a power of two from 1 to %u (every Nth message by its seq)",
                     every_text, RUSTERON_DPDK_STAGE_EVERY_MAX);
            return rusteron_dpdk_lab_fail(message);
        }
        if (NULL == stage_path || '\0' == stage_path[0])
        {
            return rusteron_dpdk_lab_fail("RUSTERON_DPDK_STAGE_TRACE needs RUSTERON_DPDK_STAGE_TRACE_FILE (its file)");
        }
    }
    uint64_t start_ms = 0, length_ms = 1000;
    if (capture)
    {
        if (kernel)
        {
            return rusteron_dpdk_lab_fail(
                "RUSTERON_DPDK_CAPTURE_FILE is the DPDK transport's (the socket transport has tcpdump)");
        }
        if (NULL != window_text && '\0' != window_text[0] &&
            rusteron_dpdk_lab_parse_window(window_text, &start_ms, &length_ms) < 0)
        {
            snprintf(message, sizeof(message),
                     "RUSTERON_DPDK_CAPTURE_WINDOW_MS=%s: <start>:<length> in ms from the transport's start "
                     "(start at most a day, length 1 to 600000)",
                     window_text);
            return rusteron_dpdk_lab_fail(message);
        }
    }

    const uint64_t generation = lab->generation;
    memset(lab, 0, sizeof(*lab));
    lab->generation = generation;
    lab->kernel = kernel;
    if (stages)
    {
        const uint32_t records =
            0 != rusteron_dpdk_lab_ring_records ? rusteron_dpdk_lab_ring_records : RUSTERON_DPDK_STAGE_RING_RECORDS;
        for (int i = 0; i < RUSTERON_DPDK_STAGE_RINGS; i++)
        {
            rusteron_dpdk_stage_ring_t *r = &lab->stage_rings[i];
            r->records = rusteron_dpdk_lab_alloc((size_t)records * sizeof(rusteron_dpdk_stage_record_t));
            r->mask = records - 1;
            if (NULL == r->records)
            {
                rusteron_dpdk_lab_free();
                return rusteron_dpdk_lab_fail("RUSTERON_DPDK_STAGE_TRACE: out of memory for its rings");
            }
        }
        lab->stage_file = fopen(stage_path, "w");
        if (NULL == lab->stage_file)
        {
            snprintf(message, sizeof(message), "RUSTERON_DPDK_STAGE_TRACE_FILE=%s cannot be opened for writing: %s",
                     stage_path, strerror(errno));
            rusteron_dpdk_lab_free();
            return rusteron_dpdk_lab_fail(message);
        }
    }
    if (capture)
    {
        const uint32_t slots = 0 != rusteron_dpdk_lab_ring_records ? rusteron_dpdk_lab_ring_records
                                                                   : RUSTERON_DPDK_CAPTURE_RING_SLOTS;
        for (int i = 0; i < RUSTERON_DPDK_CAPTURE_RINGS; i++)
        {
            rusteron_dpdk_capture_ring_t *r = &lab->capture_rings[i];
            r->slots = rusteron_dpdk_lab_alloc((size_t)slots * sizeof(rusteron_dpdk_capture_slot_t));
            r->mask = slots - 1;
            if (NULL == r->slots)
            {
                rusteron_dpdk_lab_free();
                return rusteron_dpdk_lab_fail("RUSTERON_DPDK_CAPTURE_FILE: out of memory for its rings");
            }
        }
        lab->capture_file = fopen(capture_path, "wb");
        if (NULL == lab->capture_file)
        {
            snprintf(message, sizeof(message), "RUSTERON_DPDK_CAPTURE_FILE=%s cannot be opened for writing: %s",
                     capture_path, strerror(errno));
            rusteron_dpdk_lab_free();
            return rusteron_dpdk_lab_fail(message);
        }
        rusteron_dpdk_pcapng_header(lab->capture_file);
        fflush(lab->capture_file);
        lab->capture_start_ns = start_ms * 1000000ULL;
        lab->capture_end_ns = (start_ms + length_ms) * 1000000ULL;
    }
    lab->open_tsc = rusteron_dpdk_counter();
    lab->open_raw_ns = rusteron_dpdk_lab_clock(CLOCK_MONOTONIC_RAW);
    lab->open_mono_ns = rusteron_dpdk_lab_clock(CLOCK_MONOTONIC);
    if (NULL != lab->stage_file)
    {
        fprintf(lab->stage_file, "# ergon md-dpdk stage trace v1\n");
        fprintf(lab->stage_file, "# every=%llu pid=%d transport=%s\n", (unsigned long long)every, (int)getpid(),
                kernel ? "kernel" : "dpdk");
        rusteron_dpdk_lab_clock_line(lab->stage_file, "open");
        fprintf(lab->stage_file, "stage,tsc,stream_id,seq,term_offset,value,ring\n");
        fflush(lab->stage_file);
    }
    rusteron_dpdk_stage_mask = every > 0 ? every - 1 : 0;
    lab->open = 1;
    /* the threads claim anew: a ring of an earlier open is gone */
    __atomic_store_n(&lab->generation, generation + 1, __ATOMIC_RELEASE);
    uint32_t hooks = stages ? RUSTERON_DPDK_LAB_STAGES : 0;
    if (capture && 0 == lab->capture_start_ns)
    {
        hooks |= RUSTERON_DPDK_LAB_CAPTURE;
    }
    __atomic_store_n(&rusteron_dpdk_lab_hooks, hooks, __ATOMIC_RELEASE);
    if (!rusteron_dpdk_lab_no_thread)
    {
        lab->writer_stop = 0;
        if (0 != pthread_create(&lab->writer, NULL, rusteron_dpdk_lab_writer, NULL))
        {
            __atomic_store_n(&rusteron_dpdk_lab_hooks, 0, __ATOMIC_RELEASE);
            rusteron_dpdk_lab_free();
            rusteron_dpdk_set_error("RUSTERON_DPDK_STAGE_TRACE / RUSTERON_DPDK_CAPTURE_FILE: cannot start the writer thread");
            return -1;
        }
        lab->writer_running = 1;
    }
    return 0;
}

void rusteron_dpdk_lab_close(void)
{
    rusteron_dpdk_lab_t *lab = &rusteron_dpdk_lab;
    if (!lab->open)
    {
        return;
    }
    __atomic_store_n(&rusteron_dpdk_lab_hooks, 0, __ATOMIC_RELEASE);
    if (lab->writer_running)
    {
        __atomic_store_n(&lab->writer_stop, 1, __ATOMIC_RELEASE);
        pthread_join(lab->writer, NULL);
        lab->writer_running = 0;
    }
    lab->capture_done = 1; /* the window ends with the transport */
    rusteron_dpdk_lab_drain();
    if (NULL != lab->stage_file)
    {
        const uint64_t lost = __atomic_load_n(&lab->stage_threads_lost, __ATOMIC_RELAXED);
        if (0 != lost)
        {
            fprintf(lab->stage_file, "# threads-without-a-ring %llu\n", (unsigned long long)lost);
        }
        rusteron_dpdk_lab_clock_line(lab->stage_file, "close");
    }
    rusteron_dpdk_lab_free();
}

/* ---- the socket transport, traced ---- */

static aeron_udp_channel_transport_bindings_t rusteron_dpdk_stage_inner;
static aeron_udp_channel_transport_bindings_t rusteron_dpdk_stage_kernel;
static _Thread_local aeron_udp_transport_recv_func_t rusteron_dpdk_stage_recv_inner = NULL;

static int rusteron_dpdk_stage_send(
    aeron_udp_channel_data_paths_t *data_paths,
    aeron_udp_channel_transport_t *transport,
    struct sockaddr_storage *address,
    struct iovec *iov,
    size_t iov_length,
    int64_t *bytes_sent)
{
    if (!(rusteron_dpdk_lab_on() & RUSTERON_DPDK_LAB_STAGES))
    {
        return rusteron_dpdk_stage_inner.send_func(data_paths, transport, address, iov, iov_length, bytes_sent);
    }
    const uint64_t call = rusteron_dpdk_counter();
    const int rc = rusteron_dpdk_stage_inner.send_func(data_paths, transport, address, iov, iov_length, bytes_sent);
    const uint64_t done = rusteron_dpdk_counter();
    /* the kernel's send is both the pickup and the hand-over */
    static const uint8_t stages[3] = {
        RUSTERON_DPDK_STAGE_TX_SEND, RUSTERON_DPDK_STAGE_TX_CALL, RUSTERON_DPDK_STAGE_TX_DONE};
    const uint64_t tscs[3] = {call, call, done};
    for (int i = 0; i < rc && (size_t)i < iov_length; i++)
    {
        rusteron_dpdk_stage_datagram((const uint8_t *)iov[i].iov_base, iov[i].iov_len, stages, tscs, 3, 0);
    }
    return rc;
}

static void rusteron_dpdk_stage_recv(
    aeron_udp_channel_data_paths_t *data_paths,
    aeron_udp_channel_transport_t *transport,
    void *receiver_clientd,
    void *endpoint_clientd,
    void *destination_clientd,
    uint8_t *buffer,
    size_t length,
    struct sockaddr_storage *addr,
    struct timespec *media_timestamp)
{
    const uint64_t rx = rusteron_dpdk_counter();
    rusteron_dpdk_stage_recv_inner(
        data_paths, transport, receiver_clientd, endpoint_clientd, destination_clientd, buffer, length, addr,
        media_timestamp);
    const uint64_t inserted = rusteron_dpdk_counter();
    static const uint8_t stages[3] = {
        RUSTERON_DPDK_STAGE_RX_BURST, RUSTERON_DPDK_STAGE_RX_DISPATCH, RUSTERON_DPDK_STAGE_RX_INSERTED};
    const uint64_t tscs[3] = {rx, rx, inserted};
    rusteron_dpdk_stage_datagram(buffer, length, stages, tscs, 3, 0);
}

static int rusteron_dpdk_stage_poll(
    aeron_udp_transport_poller_t *poller,
    struct mmsghdr *msgvec,
    size_t vlen,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    aeron_udp_channel_transport_recvmmsg_func_t recvmmsg_func,
    void *clientd)
{
    if (!(rusteron_dpdk_lab_on() & RUSTERON_DPDK_LAB_STAGES) || NULL == recv_func)
    {
        return rusteron_dpdk_stage_inner.poller_poll_func(
            poller, msgvec, vlen, bytes_rcved, recv_func, recvmmsg_func, clientd);
    }
    const aeron_udp_transport_recv_func_t saved = rusteron_dpdk_stage_recv_inner;
    rusteron_dpdk_stage_recv_inner = recv_func;
    const int rc = rusteron_dpdk_stage_inner.poller_poll_func(
        poller, msgvec, vlen, bytes_rcved, rusteron_dpdk_stage_recv, recvmmsg_func, clientd);
    rusteron_dpdk_stage_recv_inner = saved;
    return rc;
}

static int rusteron_dpdk_stage_recvmmsg(
    aeron_udp_channel_transport_t *transport,
    struct mmsghdr *msgvec,
    size_t vlen,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    void *clientd)
{
    /* Aeron's poller hands each transport the recvmmsg it was given, this
     * wrapper, with the receive callback the poll wrapper already stamps:
     * that one passes through, so no datagram is stamped twice. */
    if (!(rusteron_dpdk_lab_on() & RUSTERON_DPDK_LAB_STAGES) || NULL == recv_func ||
        rusteron_dpdk_stage_recv == recv_func)
    {
        return rusteron_dpdk_stage_inner.recvmmsg_func(transport, msgvec, vlen, bytes_rcved, recv_func, clientd);
    }
    const aeron_udp_transport_recv_func_t saved = rusteron_dpdk_stage_recv_inner;
    rusteron_dpdk_stage_recv_inner = recv_func;
    const int rc = rusteron_dpdk_stage_inner.recvmmsg_func(
        transport, msgvec, vlen, bytes_rcved, rusteron_dpdk_stage_recv, clientd);
    rusteron_dpdk_stage_recv_inner = saved;
    return rc;
}

void rusteron_dpdk_stage_wrap_bindings(
    const aeron_udp_channel_transport_bindings_t *inner, aeron_udp_channel_transport_bindings_t *out)
{
    rusteron_dpdk_stage_inner = *inner;
    *out = *inner;
    out->send_func = rusteron_dpdk_stage_send;
    out->poller_poll_func = rusteron_dpdk_stage_poll;
    out->recvmmsg_func = rusteron_dpdk_stage_recvmmsg;
}

int rusteron_dpdk_stage_kernel_install(aeron_driver_context_t *context)
{
    if (NULL == context)
    {
        rusteron_dpdk_set_error("stage trace: no driver context");
        return -1;
    }
    if (rusteron_dpdk_lab_open(1) < 0)
    {
        return -1;
    }
    aeron_udp_channel_transport_bindings_t *current = aeron_driver_context_get_udp_channel_transport_bindings(context);
    if (NULL == current || NULL == current->send_func || NULL == current->poller_poll_func)
    {
        rusteron_dpdk_lab_close();
        rusteron_dpdk_set_error("stage trace: the driver context has no UDP channel bindings to wrap");
        return -1;
    }
    rusteron_dpdk_stage_wrap_bindings(current, &rusteron_dpdk_stage_kernel);
    if (aeron_driver_context_set_udp_channel_transport_bindings(context, &rusteron_dpdk_stage_kernel) < 0)
    {
        rusteron_dpdk_lab_close();
        rusteron_dpdk_set_error("stage trace: the wrapped UDP channel bindings were refused");
        return -1;
    }
    return 0;
}
