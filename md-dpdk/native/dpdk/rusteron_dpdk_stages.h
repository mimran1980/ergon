/*
 * ergon-md-dpdk — the lab's per-stage trace and in-process packet capture.
 *
 * Both are off unless their variables are set, and lab only. Each hot-path
 * hook is one relaxed load of rusteron_dpdk_lab_hooks and a branch while off.
 *
 * Stage trace (RUSTERON_DPDK_STAGE_TRACE=<N>, a power of two, and
 * RUSTERON_DPDK_STAGE_TRACE_FILE=<file>): every Aeron data frame whose
 * payload is an aeron_lat message (io/benches/core message.rs: seq u64 LE at
 * payload byte 0, kind 1..=3 u32 LE at byte 28) with seq % N == 0 gets a CPU
 * counter stamp (rdtsc, CNTVCT_EL0) at each stage it passes on this host:
 *
 *   tx-send      the transport's send entry: the driver's sender agent has
 *                picked the frame up (DPDK: send_func; kernel: send_func)
 *   tx-call      just before the burst that hands it to the NIC
 *                (rte_eth_tx_burst; kernel: the sendmmsg/sendmsg call)
 *   tx-done      that call returned (value 1: the ring refused it, queued)
 *   rx-burst     the receive burst holding it returned (rte_eth_rx_burst;
 *                kernel: the receive callback's entry, after recvmmsg)
 *   rx-hw        DPDK with NIC RX timestamps: value is the ns from the NIC's
 *                arrival stamp to the CLOCK_REALTIME read just before the
 *                frame's dispatch (its tsc that read's)
 *   rx-dispatch  just before Aeron's receive callback
 *   rx-inserted  that callback returned: the frame is in the image
 *
 * The decision is the payload's alone, so every stage on both hosts samples
 * the same messages, and a message is keyed by (stream id, seq). Records go
 * into a preallocated, pre-touched ring of the writing thread's own (one
 * writer, one reader; claimed at the thread's first record), never a lock,
 * an allocation or a file write on the agent; a full ring drops the newest
 * and counts it. A writer thread of its own drains every ring to the file
 * every 10 ms, and the close drains the rest.
 *
 * File: `#` lines (the clock at open and close: tsc, CLOCK_MONOTONIC_RAW and
 * CLOCK_REALTIME ns; each ring's thread; drops), then
 * `stage,tsc,stream_id,seq,term_offset,value,ring` rows.
 *
 * Capture (DPDK only; RUSTERON_DPDK_CAPTURE_FILE=<file>,
 * RUSTERON_DPDK_CAPTURE_WINDOW_MS=<start>:<length>, from the open): every
 * frame each port receives and transmits in the window, its first
 * RUSTERON_DPDK_CAPTURE_SNAP bytes copied into the agent's own capture ring;
 * the writer thread opens and closes the window and writes pcapng (one
 * interface per port, each packet's direction in its flags, ns stamps on
 * CLOCK_REALTIME converted from the counter). Outside the window the hook
 * is off.
 */
#ifndef RUSTERON_DPDK_STAGES_H
#define RUSTERON_DPDK_STAGES_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define RUSTERON_DPDK_STAGE_TX_SEND 1
#define RUSTERON_DPDK_STAGE_TX_CALL 2
#define RUSTERON_DPDK_STAGE_TX_DONE 3
#define RUSTERON_DPDK_STAGE_RX_BURST 4
#define RUSTERON_DPDK_STAGE_RX_HW 5
#define RUSTERON_DPDK_STAGE_RX_DISPATCH 6
#define RUSTERON_DPDK_STAGE_RX_INSERTED 7

/* rusteron_dpdk_lab_hooks' bits. */
#define RUSTERON_DPDK_LAB_STAGES 1u
#define RUSTERON_DPDK_LAB_CAPTURE 2u

/* The largest sampling interval RUSTERON_DPDK_STAGE_TRACE takes. */
#define RUSTERON_DPDK_STAGE_EVERY_MAX (1u << 20)
/* Rings: at most this many threads record (the agents of any threading
 * mode, an embedded or invoked driver's included). */
#define RUSTERON_DPDK_STAGE_RINGS 8
/* Records a ring holds (a power of two): 2 MiB, half a second of every
 * frame sampled at 32 stamps a microsecond, drained every 10 ms. */
#define RUSTERON_DPDK_STAGE_RING_RECORDS 65536u
/* Bytes of each frame a capture keeps: a 64-byte message's whole frame
 * (Ethernet 14, IPv4 20, UDP 8, Aeron header 32, payload 64). */
#define RUSTERON_DPDK_CAPTURE_SNAP 160u
/* Frames a capture ring holds (a power of two). */
#define RUSTERON_DPDK_CAPTURE_RING_SLOTS 16384u
#define RUSTERON_DPDK_CAPTURE_RINGS 4
/* The writer thread's pass. */
#define RUSTERON_DPDK_LAB_PASS_NS 10000000ULL

typedef struct rusteron_dpdk_stage_record_stct
{
    uint64_t tsc;
    uint64_t seq;
    int32_t stream_id;
    int32_t term_offset;
    uint32_t value;
    uint8_t stage;
    uint8_t pad[3];
} rusteron_dpdk_stage_record_t;

_Static_assert(32 == sizeof(rusteron_dpdk_stage_record_t), "two records a cache line");

/* Which hooks are on (RUSTERON_DPDK_LAB_*): read relaxed, once a send or a
 * poll; the capture bit is set by the writer thread for the window alone. */
extern uint32_t rusteron_dpdk_lab_hooks __attribute__((visibility("hidden")));
/* N - 1: a message is sampled when (seq & mask) == 0. */
extern uint64_t rusteron_dpdk_stage_mask __attribute__((visibility("hidden")));

static inline uint32_t rusteron_dpdk_lab_on(void)
{
    return __atomic_load_n(&rusteron_dpdk_lab_hooks, __ATOMIC_RELAXED);
}

/* A sampled message found in a frame: its key, and the frame's index in
 * its burst. */
typedef struct rusteron_dpdk_stage_key_stct
{
    uint64_t seq;
    int32_t stream_id;
    int32_t term_offset;
    uint16_t frame;
} rusteron_dpdk_stage_key_t;
/* The most sampled messages one burst or datagram records. */
#define RUSTERON_DPDK_STAGE_KEYS_MAX 256

/* Record `n` stamps (stages[i] at tscs[i], each with `value`) for every
 * sampled message among the Aeron frames of datagram `d` (`len` bytes), into
 * the calling thread's ring. Out of line: tracing only. Read `d` only while
 * it is the caller's (before a transmit hands it over). */
__attribute__((noinline)) void rusteron_dpdk_stage_datagram(
    const uint8_t *d, size_t len, const uint8_t *stages, const uint64_t *tscs, int n, uint32_t value);

/* The sampled messages of Ethernet frame `frame` (`len` bytes; IPv4 UDP,
 * else none), each with `index`, appended to `keys` (`n` held, at most
 * `max`): the new count. For a transmit, before its tx_burst, while the
 * frames are still the caller's. */
int rusteron_dpdk_stage_frame_keys(
    const uint8_t *frame, size_t len, uint16_t index, rusteron_dpdk_stage_key_t *keys, int n, int max);
/* Record `stamps` stamps for each of the `n` keys whose frame index is below
 * `below` (the frames the NIC took). */
void rusteron_dpdk_stage_put_keys(
    const rusteron_dpdk_stage_key_t *keys, int n, uint16_t below, const uint8_t *stages, const uint64_t *tscs,
    int stamps, uint32_t value);

/* The counter at the DPDK transport's send entry on this thread, for the
 * tx-send stamp of the frames its tx_burst hands over (0: none pending). */
extern _Thread_local uint64_t rusteron_dpdk_stage_send_tsc;

/* A transmit's capture, in two steps around its tx_burst, so the frames are
 * read only while they are the caller's: copy `n` frames of port `port` (0
 * sender, 1 receiver) into the calling thread's capture ring, unpublished
 * (returns how many fit), then publish the first `sent` of them stamped
 * `tsc` (the rest the NIC refused are left: their send captures them). */
struct rusteron_dpdk_mbuf_stct;
uint16_t rusteron_dpdk_capture_stage(int port, struct rusteron_dpdk_mbuf_stct *const *frames, uint16_t n);
void rusteron_dpdk_capture_publish(uint16_t staged, uint16_t sent, uint64_t tsc);
/* Capture `n` received frames of port `port` at counter `tsc`, inbound
 * (`outbound` 0) or outbound, into the calling thread's capture ring. Out of
 * line: the window only. */
__attribute__((noinline)) void rusteron_dpdk_capture_views(
    int port, int outbound, const struct rusteron_dpdk_mbuf_stct *views, uint16_t n, uint64_t tsc);

/* Open both from the environment, once (0 when neither is set, or already
 * open); -1 with the error set (naming the variable) when one is malformed
 * or its file cannot be written. `kernel`: the socket transport's (no
 * capture: RUSTERON_DPDK_CAPTURE_FILE is refused). Starts the writer thread
 * unless a test turned it off. */
int rusteron_dpdk_lab_open(int kernel);
/* One writer pass now (the window, then every ring to its file): tests, with
 * the writer thread off. */
void rusteron_dpdk_lab_drain(void);
/* Stop the writer, drain everything left, close the files, free the rings:
 * after every recording thread stopped. A no-op when not open. */
void rusteron_dpdk_lab_close(void);

/* The socket transport traced: `context`'s UDP channel bindings (Aeron's
 * default) wrapped so sends and receives are stamped as above, after
 * rusteron_dpdk_lab_open(1). 0, or -1 with the error set. */
struct aeron_driver_context_stct;
int rusteron_dpdk_stage_kernel_install(struct aeron_driver_context_stct *context);
/* Wrap `inner` into `out` (tests: an inner pair of fakes). */
struct aeron_udp_channel_transport_bindings_stct;
void rusteron_dpdk_stage_wrap_bindings(
    const struct aeron_udp_channel_transport_bindings_stct *inner, struct aeron_udp_channel_transport_bindings_stct *out);

/* Tests: no writer thread (1; 0 the default), records a ring holds (a power
 * of two; 0 the default), and the reset of both. */
void rusteron_dpdk_lab_test_set_no_thread(int no_thread);
void rusteron_dpdk_lab_test_set_ring_records(uint32_t records);
void rusteron_dpdk_lab_test_reset(void);

#ifdef __cplusplus
}
#endif

#endif /* RUSTERON_DPDK_STAGES_H */
