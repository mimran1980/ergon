/*
 * ergon-md-dpdk DPDK transport — internal shared definitions.
 *
 * Includes the opaque transport handle's full layout, the role-port state, and
 * the seams the runtime goes through for EAL initialization and port
 * operations. This header is internal to the native DPDK sources and the test
 * fakes; it is not part of the Rust/native ABI.
 */
#ifndef RUSTERON_DPDK_INTERNAL_H
#define RUSTERON_DPDK_INTERNAL_H

/* clock_gettime/CLOCK_MONOTONIC (runtime.c), inet_pton (transport.c) and
 * htonl/htons (packet.c) are POSIX-199309/200112 symbols glibc hides under
 * strict -std=c11 without a feature macro. Every native DPDK source includes
 * this header first, so declaring it here (before any system include) covers
 * them uniformly. The vendored Aeron util files compiled alongside declare
 * their own macro and must not receive a -D from the build script. */
#define _POSIX_C_SOURCE 200809L

/* x86_64 or aarch64 (build.rs refuses any other target): the clock's counter
 * and the spin hint below have one implementation for each. */
#if defined(__x86_64__)
#include <x86intrin.h>
#elif !defined(__aarch64__)
#error "the DPDK transport builds for x86_64 or aarch64 only"
#endif
#include <netinet/in.h>
#include <pthread.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "rusteron_dpdk_transport.h"
#include "media/aeron_udp_channel_transport.h" /* aeron_udp_channel_transport_params_t */
#include "rusteron_dpdk_port_ops.h"
#include "rusteron_dpdk_counters.h"
#include "rusteron_dpdk_arp.h"
#include "rusteron_dpdk_endpoint_map.h"
#include "rusteron_dpdk_events.h"

/* Aeron types for the poller callbacks and the shared receive loop (poller.c):
 * the affinity enum, the transport struct, the recv/recvmmsg function typedefs
 * and the (forward-declared) poller struct. transport.c already pulls these
 * transitively via media/aeron_udp_channel_transport.h; declaring them here
 * keeps poller.c self-contained. */
#include "media/aeron_udp_channel_transport_bindings.h"

#ifdef __cplusplus
extern "C" {
#endif

#define RUSTERON_DPDK_ROLE_SENDER 0
#define RUSTERON_DPDK_ROLE_RECEIVER 1

/* EAL modes selected by rusteron_dpdk_test_set_eal_mode (production = REAL). */
#define RUSTERON_DPDK_EAL_REAL 0    /* --huge-dir <configured path> */
#define RUSTERON_DPDK_EAL_NO_HUGE 1 /* --no-huge (tests without hugetlbfs) */
#define RUSTERON_DPDK_EAL_SKIP 2    /* skip the EAL seam entirely (tests) */

/* A canonical PCI BDF contains ':'; anything else is a virtual-device name
 * (test/TAP path, plan §11.2). Shared by the EAL argv builder and the port
 * bring-up checks so the two never disagree on the mode of a selector. */
static inline int rusteron_dpdk_selector_is_pci(const char *sel)
{
    return NULL != sel && NULL != strchr(sel, ':');
}

/* Transport-level offload bits passed to the port ops; the real port
 * implementation maps them onto the RTE_ETH_RX/TX_OFFLOAD_* masks. */
#define RUSTERON_DPDK_RX_OFFLOAD_IPV4_CKSUM (1ULL << 0)
#define RUSTERON_DPDK_RX_OFFLOAD_UDP_CKSUM (1ULL << 1)
#define RUSTERON_DPDK_RX_OFFLOAD_TIMESTAMP (1ULL << 2)
#define RUSTERON_DPDK_TX_OFFLOAD_IPV4_CKSUM (1ULL << 0)
#define RUSTERON_DPDK_TX_OFFLOAD_UDP_CKSUM (1ULL << 1)
/* A port's MBUF_FAST_FREE (port->tx_fast_free): not asked for, configured,
 * or asked for on a NIC that does not offer it (left off). */
#define RUSTERON_DPDK_FAST_FREE_OFF 0
#define RUSTERON_DPDK_FAST_FREE_ON 1
#define RUSTERON_DPDK_FAST_FREE_UNSUPPORTED 2
/* RTE_ETH_TX_OFFLOAD_MBUF_FAST_FREE: the PMD frees sent mbufs in bulk to one
 * pool, so every mbuf sent on the port must come from the port's own pool,
 * have a reference count of one and be a single segment (port_tx's are). */
#define RUSTERON_DPDK_TX_OFFLOAD_MBUF_FAST_FREE (1ULL << 2)

/* Error codes are defined in rusteron_dpdk_transport.h (public ABI). */

/* Receive counters (plan §7.6): one bucket per reject class, incremented in the
 * poll hot path, and surfaced as Aeron counters (rusteron_dpdk_counters.c). */
typedef struct rusteron_dpdk_rx_stats_stct
{
    uint64_t accepted;
    uint64_t arp;
    uint64_t ipv6;
    uint64_t multicast;
    uint64_t ethertype;
    uint64_t vlan;
    uint64_t ip_options;
    uint64_t fragment;
    uint64_t truncated;
    uint64_t protocol;
    uint64_t checksum;
    uint64_t multi_segment;
    uint64_t foreign_dst;
    uint64_t unknown_port;
    uint64_t not_connected_peer;  /* connected transport: datagram from another peer */
    uint64_t icmp;                /* ICMP handled (echo answered, errors recorded) */
} rusteron_dpdk_rx_stats_t;

#define RUSTERON_DPDK_MAX_EXTRA_IPS 8
#define RUSTERON_DPDK_MAX_KSOCKS 1024
#define RUSTERON_DPDK_MAX_TX_ROUTES 16
/* The most datagrams one send takes (Aeron offers at most 16): also how many
 * refused frames a NIC can hold queued, one vector's worth. */
#define RUSTERON_DPDK_TX_VECTOR_MAX 256
#define RUSTERON_DPDK_MAX_TX_LOW 16 /* RUSTERON_DPDK_TX_LOW_STREAMS ranges */

/* Aeron's threading modes (aeron_threading_mode_t, aeronmd.h: asserted equal
 * in rusteron_dpdk_transport.c), which decide which agent busy-spins on which
 * CPU: under DEDICATED the sender and the receiver, each on its own; under
 * SHARED_NETWORK one agent for both, on the sender's CPU; under SHARED one
 * agent for everything, on the conductor's CPU; under INVOKER the
 * application's thread that starts and runs the driver, which Aeron pins to
 * the conductor's CPU. */
#define RUSTERON_DPDK_THREADING_DEDICATED 0
#define RUSTERON_DPDK_THREADING_SHARED_NETWORK 1
#define RUSTERON_DPDK_THREADING_SHARED 2
#define RUSTERON_DPDK_THREADING_INVOKER 3

/* A transmit rate allowance: a token bucket, owned by one NIC's agent alone
 * (plain arithmetic, no atomics); it may go negative (debt). It counts
 * millibits: a megabit a second is a millibit a nanosecond, so a refill
 * credits all the time since the last, with no division and no fraction of a
 * byte dropped however often it runs. `rate_mbps` 0 is no allowance: the
 * helpers below are then all no-ops and the send path skips the block.
 * Aligned so it never straddles two cache lines: every send touches it. */
#define RUSTERON_DPDK_MILLIBITS_PER_BYTE 8000LL
typedef struct rusteron_dpdk_pacer_stct
{
    uint32_t rate_mbps;       /* megabits per second of frames, at most 400000; 0 = no allowance */
    int64_t bucket_millibits; /* the most it holds: the burst allowed at once */
    int64_t millibits;
    uint64_t last_ns;         /* last refill */
} __attribute__((aligned(32))) rusteron_dpdk_pacer_t;

_Static_assert(32 == sizeof(rusteron_dpdk_pacer_t), "a pacer fills half a cache line");

/* A kernel socket's readiness, raised by the local-I/O thread and cleared by
 * the agent that drains it; one per cache line, so flags of sockets drained by
 * different agents never share one. Adaptive local I/O also shares, on that
 * line, the agent's hot deadline and whether the socket is off the epoll. */
typedef struct rusteron_dpdk_ksock_stct
{
    volatile uint8_t ready;
    uint8_t used;
    volatile uint8_t unwatched;     /* adaptive: off the epoll; its agent reads it every cycle */
    uint8_t watched;                /* registered with the epoll (port_table_lock) */
    int fd;                         /* the socket, -1 when none (port_table_lock) */
    volatile uint64_t hot_until_ns; /* adaptive: its agent reads it every cycle until then */
} __attribute__((aligned(64))) rusteron_dpdk_ksock_t;

/* A receive-path error report (rusteron_dpdk_report_t.kind). */
#define RUSTERON_DPDK_REPORT_NONE 0
#define RUSTERON_DPDK_REPORT_PEER_MTU 1    /* a SETUP whose mtu is above max_aeron_mtu */
#define RUSTERON_DPDK_REPORT_FRAG_NEEDED 2 /* an ICMP fragmentation-needed for this NIC */
/* Its slot's state (rusteron_dpdk_report_t.state). */
#define RUSTERON_DPDK_REPORT_FREE 0
#define RUSTERON_DPDK_REPORT_WRITING 1
#define RUSTERON_DPDK_REPORT_READY 2

/* An error the NIC's agent found on the receive path, for housekeeping to
 * format and record: the agent fills it (FREE -> WRITING -> READY) and never
 * formats an address, builds a message or takes the distinct error log's
 * mutex; housekeeping takes it (READY -> FREE). One slot a NIC: the agent
 * reports at most once a second and housekeeping looks every 100 ms. */
typedef struct rusteron_dpdk_report_stct
{
    uint32_t state;    /* RUSTERON_DPDK_REPORT_FREE / _WRITING / _READY */
    uint32_t kind;     /* RUSTERON_DPDK_REPORT_* */
    uint32_t ip;       /* the peer (SETUP) or the router (ICMP), network order */
    uint32_t value;    /* the peer's mtu / the next-hop MTU */
    uint16_t udp_port; /* the peer's UDP port (SETUP), host order */
} rusteron_dpdk_report_t;

typedef struct rusteron_dpdk_port_stct
{
    uint8_t role;                 /* RUSTERON_DPDK_ROLE_* */
    char pci[16];                 /* dddd:bb:ss.f */
    int has_mac;                  /* selected by MAC on its PCI function (MANA) */
    uint8_t want_mac[RUSTERON_DPDK_ETH_ADDR_LEN]; /* that MAC, byte order */
    char mac_text[18];            /* that MAC as configured */
    char local_ipv4[16];          /* dotted quad */
    uint8_t prefix_len;           /* 1..=32 */
    char gateway_ipv4[16];        /* dotted quad */
    uint16_t port_id;             /* DPDK port id once probed */
    /* NUMA node the PMD reported, or -1 when it reported none. socket_known
     * distinguishes that read from a zeroed struct (node 0 is a real node). */
    int socket_id;
    int socket_known;
    void *mempool;                /* opaque rte_mempool* (or fake handle) */
    uint16_t rx_queue_id;
    uint16_t tx_queue_id;
    int configured;               /* dev_configure succeeded */
    int started;                  /* dev_start succeeded */
    int queues_ready;             /* both queues set up: only then may the port be closed */
    int link_up;                  /* link_wait_ready succeeded */
    int csum_offload_ok;          /* dev_info: IPv4/UDP checksum offloads */
    int ena_llq;                  /* dev_info: an ENA's LLQ BAR mapping (RUSTERON_DPDK_ENA_LLQ_*) */
    uint16_t rx_descriptors;      /* the receive ring's size: as configured, or the NIC's default */
    /* A poll draining this many frames came within a burst of a full ring
     * (rx-ring-full-suspected): the ring less a burst, or UINT32_MAX, never,
     * on an ENA, which counts a full ring's drops itself (nic-rx-missed). */
    uint32_t rx_ring_full_at;
    int rx_timestamp_ok;          /* dev_info: NIC RX hardware timestamps */
    int tx_fast_free_ok;          /* dev_info: TX MBUF_FAST_FREE offered */
    int tx_fast_free;             /* RUSTERON_DPDK_FAST_FREE_*: what became of RUSTERON_DPDK_TX_FAST_FREE */
    uint16_t rx_free_thresh;      /* the queues' free thresholds as set up (0: the PMD's) */
    uint16_t tx_free_thresh;
    uint16_t max_mtu;             /* dev_info: the NIC's largest L3 MTU (0: unknown) */
    uint8_t mac[RUSTERON_DPDK_ETH_ADDR_LEN]; /* dev_info MAC, byte order */
    uint32_t local_ip;            /* parsed local IPv4, network order */
    uint32_t gateway_ip;          /* parsed gateway IPv4, network order */
    char driver[64];              /* dev_info PMD name (labels, port-info) */
    uint64_t last_poll_ns;        /* clock at the end of the last receive poll, 0 before the first */
    /* Thread CPU time at that poll, 0 while RUSTERON_DPDK_GAP_ATTRIB is off or
     * before the first attributed poll: together with last_poll_ns it splits a
     * poll gap into work and descheduling. */
    uint64_t last_poll_cpu_ns;
    int tx_udp_csum_offload;      /* NIC completes the UDP checksum; else software */
    int open_transports;          /* DPDK transports open on this NIC (housekeeping_lock) */
    /* DPDK transports open on this NIC whose channel asks for media receive
     * timestamps (media-rcv-ts-offset): changed by the conductor at init and
     * close, read (relaxed) by the agent once a poll, so a NIC without one
     * never reads the clock for them. */
    int media_ts_transports;
    /* The driver's distinct error log, from the transports opened on this NIC:
     * read only while one is open (housekeeping_lock), as it goes with the
     * driver. */
    aeron_distinct_error_log_t *error_log;
    /* Device resets the PMD asked for (RTE_ETH_EVENT_INTR_RESET), counted
     * atomically by the port op's callback on whichever thread raises them;
     * housekeeping counts, logs and reports each new one (nic_resets_seen is
     * its own). */
    uint64_t nic_resets;
    uint64_t nic_resets_seen;
    int reset_watched;            /* watch_reset took: unwatch before the port closes */
    /* Link-status changes (RTE_ETH_EVENT_INTR_LSC), counted as nic_resets
     * are, where the NIC can raise them (lsc_on: RTE_ETH_DEV_INTR_LSC);
     * housekeeping keeps the counter and logs each new one. */
    int lsc_on;
    int lsc_watched;              /* watch_lsc took: unwatch before the port closes */
    uint64_t nic_link_changes;
    uint64_t nic_link_changes_seen;
    uint64_t icmp_window_ns;      /* ICMP error rate limit: current one-second window */
    uint32_t icmp_sent_in_window;
    uint64_t icmp_echo_window_ns; /* ICMP echo rate limit: current one-second window */
    uint32_t icmp_echo_sent;
    uint64_t last_report_ns;      /* rate limit for error reports (the agent's) */
    uint64_t arp_reply_window_ns; /* ARP reply rate limit: current one-second window */
    /* The bandwidth allowances: this NIC's (RUSTERON_DPDK_TX_RATE_MBPS, the
     * VM's allowance) and one per destination prefix
     * (RUSTERON_DPDK_TX_ROUTE_MBPS, a slower link), all owned by its agent. */
    rusteron_dpdk_pacer_t tx_pacer;
    rusteron_dpdk_pacer_t route_pacers[RUSTERON_DPDK_MAX_TX_ROUTES];
    uint32_t arp_replies_in_window;
    uint32_t extra_ips[RUSTERON_DPDK_MAX_EXTRA_IPS]; /* more local IPv4s (secondary IPs, VIPs) */
    uint32_t extra_ip_count;
    rusteron_dpdk_counters_t counters; /* Aeron counters (plan §9) */
    /* The agent's error report for housekeeping: a line of its own, which
     * housekeeping reads every 100 ms. */
    rusteron_dpdk_report_t report __attribute__((aligned(64)));
    /* The lab event log's ring for this NIC's agent, its one writer (NULL:
     * no log; RUSTERON_DPDK_EVENT_LOG). */
    rusteron_dpdk_event_ring_t *events;
    /* Owned by this NIC's agent thread alone (the sender agent for the sender
     * NIC, the receiver agent for the receiver NIC), so no lock is needed:
     * both its sends and its polls run there. */
    rusteron_dpdk_rx_stats_t rx_stats;
    rusteron_dpdk_arp_table_t arp;
    /* Frames the TX ring refused, built and waiting in order for this agent
     * to send them first, at its next send or poll of the NIC. A send is all
     * or nothing, as a kernel socket's queue: reporting the part of a vector
     * the ring took made Aeron send the whole vector again, that part twice.
     * At most one vector waits: while any frame does, a new vector is refused
     * whole before anything of it is built. */
    uint16_t tx_queued_count;
    /* Idle TX cleanup (RUSTERON_DPDK_TX_IDLE_CLEAN): the descriptors an idle
     * poll may reclaim (0: off, or the PMD lacks rte_eth_tx_done_cleanup),
     * the frames handed to the NIC since the last reclaim, and the clock at
     * the last data send. Written only while on. */
    uint32_t tx_idle_clean;
    uint32_t tx_unreaped;
    uint64_t last_tx_ns;
    uint32_t tx_idle_clean_asked; /* the budget asked for (port-info: unsupported when tx_idle_clean is 0) */
    rusteron_dpdk_mbuf_t tx_queued[RUSTERON_DPDK_TX_VECTOR_MAX];
} rusteron_dpdk_port_t;

/* An idle poll reclaims sent descriptors only once the last data send is
 * this old: the NIC has posted its completions by then. */
#define RUSTERON_DPDK_TX_IDLE_CLEAN_AGE_NS 5000ULL
/* The most descriptors RUSTERON_DPDK_TX_IDLE_CLEAN may ask a poll for (an
 * ENA's TX ring is at most 1024). */
#define RUSTERON_DPDK_TX_IDLE_CLEAN_MAX 1024


/* The UDP ports bound on one NIC, exclusively as a kernel socket would. Cold:
 * touched only at transport init/close, under the runtime's port_table_lock. */
typedef struct rusteron_dpdk_port_table_stct
{
    /* [0] the wildcard (0.0.0.0) binds, [1] the NIC's address, [2 + i] its
     * extra address i: as the kernel, a wildcard bind conflicts with any bind
     * of that port, a specific one only with the wildcard and itself. */
    uint64_t bound[2 + RUSTERON_DPDK_MAX_EXTRA_IPS][65536 / 64];
    uint16_t ephemeral_next;      /* where the next ephemeral search starts */
} rusteron_dpdk_port_table_t;

/* Per-channel state attached to an Aeron transport via bindings_clientd
 * (transport.c). One instance per initialized Aeron transport/endpoint. */
#define RUSTERON_DPDK_REPLY_IPS 64

/* The local address a peer last reached a wildcard transport on. */
typedef struct rusteron_dpdk_reply_ip_stct
{
    uint32_t peer_ip;
    uint32_t local_ip;
} rusteron_dpdk_reply_ip_t;

_Static_assert(64 == RUSTERON_DPDK_REPLY_IPS, "reply_slot takes 6 bits");

/* Fibonacci hashing: the top bits of the product depend on every bit of the
 * address, so neighbouring peers spread. */
static inline uint32_t rusteron_dpdk_reply_slot(uint32_t peer_ip)
{
    return (peer_ip * 0x9e3779b1u) >> (32 - 6);
}

#define RUSTERON_DPDK_LOCAL_IO_EPOLL 0
#define RUSTERON_DPDK_LOCAL_IO_SPIN 1
#define RUSTERON_DPDK_LOCAL_IO_ADAPTIVE 2

#define RUSTERON_DPDK_LAZY_NONE 0
#define RUSTERON_DPDK_LAZY_REQUESTED 1
#define RUSTERON_DPDK_LAZY_READY 2
#define RUSTERON_DPDK_LAZY_FAILED 3

typedef struct rusteron_dpdk_client_stct
{
    /* Hot: read on every send and dispatched datagram, in the first 64 bytes. */
    rusteron_dpdk_transport_t *runtime; /* owning native runtime (singleton) */
    rusteron_dpdk_port_t *port;         /* sender or receiver ENA by affinity */
    size_t mtu;                         /* per-channel Aeron MTU cap (bytes) */
    uint16_t local_udp_port;            /* local bind port, host order */
    uint16_t connected_port;            /* host order */
    uint32_t bound_ip;                  /* its bind address (network order), 0 for the wildcard */
    int connected;                      /* connect()ed: receive only from the peer */
    uint32_t connected_ip;              /* its address, network order */
    /* The kernel path (rusteron_dpdk_kernel.c): `delegated` sends and
     * receives everything through kernel_fd (a host-local channel); otherwise
     * kernel_fd, when open, carries only host-local destinations of a DPDK
     * transport, from the same port. -1 when unused. */
    int delegated;
    int kernel_fd;
    int ksock;                          /* its readiness slot, -1 when none */
    uint64_t local_hot_until_ns;        /* adaptive: read kernel_fd every cycle until then */

    struct rusteron_dpdk_poller_state_stct *poller; /* the poller it is added to */
    int kernel_polled;                  /* kernel_fd is on its poller's list */
    int affinity;                       /* AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_* */
    int bound_index;                    /* its row in the port table (0 wildcard, 1 NIC, 2+ extra) */
    uint32_t route_dst_ip;              /* the last destination looked up in the route allowances */
    int route_index;                    /* its route (-1 none, -2 not looked up yet) */
    struct sockaddr_storage connected_address; /* owned copy for NULL-address sends */
    aeron_udp_channel_transport_params_t params; /* socket buffers for a kernel socket */
    int media_ts;                       /* counted in its port's media_ts_transports */
    /* A DPDK transport's host-local send socket, opened by the local-I/O
     * thread on request (kernel.c), never by the agent: the agent writes
     * lazy_dst_ip, then publishes REQUESTED in lazy_state; the local-I/O
     * thread publishes READY with lazy_fd/lazy_ksock (or FAILED, retried each
     * second), and the agent adopts them as kernel_fd/ksock. */
    /* Its own cache line: the local-I/O thread writes it. */
    volatile int lazy_state __attribute__((aligned(64))); /* RUSTERON_DPDK_LAZY_* */
    uint32_t lazy_dst_ip;               /* the destination to bind on, network order */
    int lazy_fd;
    int lazy_ksock;
    int lazy_reported;                  /* the current FAILED episode was reported */
    uint64_t lazy_retry_ns;             /* local-I/O thread only */
    int lazy_linked;                    /* on the runtime's lazy_clients list */
    struct rusteron_dpdk_client_stct *lazy_next; /* port_table_lock */
    struct rusteron_dpdk_client_stct *lazy_prev;
    /* A wildcard transport answers each peer from the address it reached us
     * on, as the kernel picks the source for a reply: NICs with extra IPs only. */
    rusteron_dpdk_reply_ip_t reply[RUSTERON_DPDK_REPLY_IPS];
} __attribute__((aligned(64))) rusteron_dpdk_client_t;

_Static_assert(offsetof(rusteron_dpdk_client_t, local_hot_until_ns) + sizeof(uint64_t) <= 64,
               "the hot client fields share one cache line");
_Static_assert(offsetof(rusteron_dpdk_client_t, lazy_state) >= 64, "lazy_state, written off the agent, is not on it");

struct rusteron_dpdk_transport_stct
{
    rusteron_dpdk_config_t config;
    const rusteron_dpdk_port_ops_t *ops; /* active port ops (real or fake) */
    int eal_up;                          /* this transport owns EAL up */
    rusteron_dpdk_port_t sender;
    rusteron_dpdk_port_t receiver;

    /* Host-local IPv4s (127/8 implied): every address on a kernel interface
     * except the DPDK NICs'. Channels to or bound on these go through the
     * kernel (rusteron_dpdk_kernel.c), never DPDK. Read once at init. */
    uint32_t *host_ips;              /* sorted (host order values), owned */
    uint32_t host_ip_count;
    int wildcard_local;              /* a 0.0.0.0 receive bind also listens on the kernel */

    /* Housekeeping (runtime.c): the 1 Hz PMD statistics sample, on its own
     * thread so the agents' hot paths never do it. The lock serialises the
     * sample with counter registration and release, never the hot path. */
    pthread_mutex_t housekeeping_lock;
    int housekeeping_lock_ready;
    /* The driver's agents' CPUs (conductor, sender, receiver; -1 unpinned),
     * from its context at install: housekeeping keeps off them. */
    int agent_cpus[3];
    int agent_cpus_known;
    int conductor_spins;                 /* its idle strategy busy-spins */
    int threading_mode;                  /* RUSTERON_DPDK_THREADING_*, from the context at install */
    int local_io_mode;                   /* RUSTERON_DPDK_LOCAL_IO_* (RUSTERON_DPDK_LOCAL_IO) */
    uint64_t local_hot_ns;               /* adaptive: read every cycle this long after data */
    int local_unwatch;                   /* adaptive: hot sockets come off the epoll (RUSTERON_DPDK_LOCAL_UNWATCH) */
    uint64_t tx_retry_ns;                /* RUSTERON_DPDK_TX_RETRY_US: re-offer a refused tail this long */
    uint64_t nic_stats_ns;               /* RUSTERON_DPDK_NIC_STATS_MS: housekeeping's NIC statistics reads, 0 never */
    uint64_t nic_slow_stats_ns;          /* RUSTERON_DPDK_NIC_SLOW_STATS_MS: the full reads once split (UNSPLIT: not split), 0 never */
    uint32_t rx_max_bursts;              /* RUSTERON_DPDK_RX_MAX_BURSTS: bursts one poll drains while they come back full */
    uint32_t tx_idle_clean;              /* RUSTERON_DPDK_TX_IDLE_CLEAN: descriptors an idle poll reclaims, 0 off */
    int ena_llq_required;                /* RUSTERON_DPDK_ENA_LLQ_REQUIRED: an ENA without verified LLQ fails start */
    int vfio_wc;                         /* RUSTERON_DPDK_VFIO_WC=1: a vfio-pci port's unseen LLQ BAR is write-combined */
    uint64_t tx_pace_spin_ns;            /* RUSTERON_DPDK_TX_PACE_SPIN_US: longest wait to repay pacing debt */
    int tx_pace_spin_set;                /* RUSTERON_DPDK_TX_PACE_SPIN_US was given, not defaulted */
    /* RUSTERON_DPDK_TX_ROUTE_MBPS: destination prefixes with their own
     * allowance, longest first (network order); the pacers are per NIC. */
    uint32_t tx_route_net[RUSTERON_DPDK_MAX_TX_ROUTES];
    uint32_t tx_route_mask[RUSTERON_DPDK_MAX_TX_ROUTES];
    uint32_t tx_route_count;
    /* RUSTERON_DPDK_TX_LOW_STREAMS: stream ids (inclusive ranges) held below
     * the rest under the allowances, and the bucket bytes they leave them
     * (RUSTERON_DPDK_TX_LOW_RESERVE_BYTES). */
    uint32_t tx_low_count;
    int32_t tx_low_first[RUSTERON_DPDK_MAX_TX_LOW];
    int32_t tx_low_last[RUSTERON_DPDK_MAX_TX_LOW];
    int64_t tx_low_reserve;
    /* The CPUs the creating thread may run on, taken once at create (before
     * the EAL or any pinning): the pool housekeeping picks from. */
    uint64_t process_cpus[16];
    int process_cpus_known;
    pthread_t housekeeping_thread;
    int housekeeping_running;
    volatile int housekeeping_stop;
    /* A PMD asked for a device reset. The transport does not reset the NIC.
     * The media driver reads this, stops its agents, and exits. */
    int nic_reset_exit;
    uint64_t timer_service_ns;           /* the PMDs' timer service last ran (0: never; housekeeping) */
    int timer_service_failing;           /* it failed last time, which was reported (housekeeping) */
    /* The lab event log (RUSTERON_DPDK_EVENT_LOG, rusteron_dpdk_events.c):
     * the file, written by housekeeping and the close only, and the ring
     * housekeeping records its own work into. NULL when off. */
    FILE *event_log;
    rusteron_dpdk_event_ring_t *housekeeping_events;

    /* Cold: the per-NIC UDP port tables and the ephemeral range, resolved at
     * runtime init (RUSTERON_DPDK_EPHEMERAL_PORT_RANGE, else the netns's
     * ip_local_port_range, minus ip_local_reserved_ports). */
    pthread_mutex_t port_table_lock;
    uint16_t ephemeral_low;
    uint16_t ephemeral_high;
    uint64_t reserved_ports[65536 / 64];
    rusteron_dpdk_port_table_t sender_ports;
    rusteron_dpdk_port_table_t receiver_ports;

    /* The local-I/O thread (kernel.c) and the kernel-socket readiness slots
     * it raises; the slots are never freed (port_table_lock guards `used`). */
    int local_epfd;
    pthread_t local_io_thread;
    int local_io_running;
    volatile int local_io_stop;
    uint32_t local_unwatched;            /* sockets off the epoll (port_table_lock; read by the local-I/O thread) */
    /* adaptive: bumped by an agent when one of its sockets turns hot, so the
     * local-I/O thread takes it off the epoll (alone on its line) */
    struct
    {
        volatile uint32_t value;
    } __attribute__((aligned(64))) local_hot_epoch;
    uint8_t local_hot_epoch_pad[60];
    rusteron_dpdk_client_t *lazy_clients; /* DPDK transports (port_table_lock) */
    rusteron_dpdk_ksock_t ksocks[RUSTERON_DPDK_MAX_KSOCKS];
};

/* Every send reads both counts whenever an allowance is armed
 * (RUSTERON_DPDK_TX_RATE_MBPS or a route's; none unless set), so they share a
 * cache line: the transport is allocated 64-byte aligned. */
_Static_assert(offsetof(struct rusteron_dpdk_transport_stct, tx_route_count) / 64 ==
                   offsetof(struct rusteron_dpdk_transport_stct, tx_low_count) / 64,
               "the route and low-stream counts share one cache line");

/* The hint in a bounded spin-wait (the pacer's): pause on x86_64; isb on
 * aarch64, which AWS's Graviton guide gives as pause's drop-in and Rust's
 * spin_loop emits. yield, DPDK's rte_pause there, only yields to another
 * hardware thread, and Graviton's Neoverse cores have none, so it does
 * nothing; isb waits for the pipeline to drain, a short real delay as pause
 * is. */
static inline void rusteron_dpdk_spin_hint(void)
{
#if defined(__x86_64__)
    _mm_pause();
#else
    __asm__ __volatile__("isb" ::: "memory");
#endif
}

/* Credit an allowance's bucket for the time since its last refill. A read
 * behind the last refill (a counter read a little behind on another core, a
 * clock started afresh) credits nothing and leaves last_ns at the furthest time
 * credited, so no interval is ever credited twice; unguarded, the difference
 * would wrap and refill the bucket. */
static inline void rusteron_dpdk_pacer_refill(rusteron_dpdk_pacer_t *p, uint64_t now_ns)
{
    if (__builtin_expect(now_ns < p->last_ns, 0))
    {
        return;
    }
    const uint64_t elapsed = now_ns - p->last_ns;
    p->last_ns = now_ns;
    /* millibits = ns * Mbit/s; elapsed is capped so the product cannot overflow */
    const uint64_t capped = elapsed > 1000000000ULL ? 1000000000ULL : elapsed;
    const int64_t millibits = p->millibits + (int64_t)(capped * p->rate_mbps);
    p->millibits = millibits > p->bucket_millibits ? p->bucket_millibits : millibits;
}

/* Nanoseconds until an allowance's debt is repaid (0: none, or no allowance). */
static inline uint64_t rusteron_dpdk_pacer_wait(const rusteron_dpdk_pacer_t *p)
{
    return 0 == p->rate_mbps || p->millibits >= 0 ? 0 : (uint64_t)(-p->millibits) / p->rate_mbps;
}

/* Charge `wire` bytes on the wire to an allowance (negative: refund them). */
static inline void rusteron_dpdk_pacer_charge(rusteron_dpdk_pacer_t *p, int64_t wire)
{
    p->millibits -= wire * RUSTERON_DPDK_MILLIBITS_PER_BYTE;
}

/* Whether a low-priority send of `wire` bytes may draw on an allowance: it
 * leaves `reserve` bytes in the bucket, or the bucket is full (so a batch
 * larger than the bucket less the reserve is held back, never starved). It
 * never overdraws, so the others never wait for its debt. No allowance:
 * always. */
static inline int rusteron_dpdk_pacer_spare(const rusteron_dpdk_pacer_t *p, int64_t wire, int64_t reserve)
{
    return 0 == p->rate_mbps ||
        p->millibits - wire * RUSTERON_DPDK_MILLIBITS_PER_BYTE >= reserve * RUSTERON_DPDK_MILLIBITS_PER_BYTE ||
        p->millibits >= p->bucket_millibits;
}

/* `rate_mbps` at most 400000, 0 for no allowance; `bucket` in bytes, at most
 * 1 GiB: in millibits, far inside an int64. */
static inline void rusteron_dpdk_pacer_init(rusteron_dpdk_pacer_t *p, uint64_t rate_mbps, int64_t bucket,
                                            uint64_t now_ns)
{
    p->rate_mbps = (uint32_t)rate_mbps;
    p->bucket_millibits = bucket * RUSTERON_DPDK_MILLIBITS_PER_BYTE;
    p->millibits = p->bucket_millibits;
    p->last_ns = now_ns;
}

/* Whether `ip` (network order) is host-local: loopback, or a kernel
 * interface's address. Hot-path safe: a compare and a short scan. */
static inline int rusteron_dpdk_is_host_local(const rusteron_dpdk_transport_t *native, uint32_t ip)
{
    if (0x7f == (((const uint8_t *)&ip)[0]))
    {
        return 1;
    }
    /* Binary search: a node can carry hundreds of addresses (kube-proxy IPVS). */
    const uint32_t key = __builtin_bswap32(ip);
    uint32_t lo = 0, hi = native->host_ip_count;
    while (lo < hi)
    {
        const uint32_t mid = lo + (hi - lo) / 2;
        if (native->host_ips[mid] < key)
        {
            lo = mid + 1;
        }
        else
        {
            hi = mid;
        }
    }
    return lo < native->host_ip_count && native->host_ips[lo] == key;
}

/* Whether `ip` (network order) is one of the port's own addresses. */
static inline int rusteron_dpdk_port_owns_ip(const rusteron_dpdk_port_t *port, uint32_t ip)
{
    if (ip == port->local_ip)
    {
        return 1;
    }
    for (uint32_t i = 0; i < port->extra_ip_count; i++)
    {
        if (port->extra_ips[i] == ip)
        {
            return 1;
        }
    }
    return 0;
}

/* The kernel path for host-local traffic (rusteron_dpdk_kernel.c). Open a
 * non-blocking UDP socket bound to `bind_addr` (port 0: the kernel picks),
 * optionally connected; *port_out is the bound port. */
struct mmsghdr;
struct iovec;
int rusteron_dpdk_kernel_open(
    int *fd_out,
    const struct sockaddr_in *bind_addr,
    const struct sockaddr_in *connect_addr,
    const aeron_udp_channel_transport_params_t *params,
    uint16_t *port_out);
void rusteron_dpdk_kernel_close(int *fd);
/* Datagrams sent (a prefix of iov), 0 on EAGAIN-like pressure, -1 on error.
 * `address` NULL on a connected socket. */
int rusteron_dpdk_kernel_send(
    int fd, const struct sockaddr_storage *address, struct iovec *iov, size_t iov_length, int64_t *bytes_sent);
/* Receive up to vlen datagrams into msgvec's buffers and dispatch each. */
/* `dpdk_client` non-NULL (a DPDK transport's kernel socket): datagrams from
 * sources that are not host-local, or not its connected peer, are read and
 * discarded. Returns the datagrams read, -1 on an error, or
 * RUSTERON_DPDK_KERNEL_RECV_AGAIN when a transient error (a pending ICMP
 * error, EINTR) was consumed and datagrams may still wait. */
#define RUSTERON_DPDK_KERNEL_RECV_AGAIN (-2)
int rusteron_dpdk_kernel_recv(
    int fd,
    const rusteron_dpdk_client_t *dpdk_client,
    aeron_udp_channel_transport_t *transport,
    struct mmsghdr *msgvec,
    size_t vlen,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    void *clientd);

/* Poller (poller.c): put a transport's kernel socket on a poller's list;
 * receive for a transport through the poller it is registered with (every
 * endpoint on the NIC dispatched, none dropped). */
int rusteron_dpdk_poller_kernel_add(struct rusteron_dpdk_poller_state_stct *state, aeron_udp_channel_transport_t *transport);
int rusteron_dpdk_poller_receive_registered(
    rusteron_dpdk_client_t *client,
    struct mmsghdr *msgvec,
    size_t vlen,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    void *clientd);

/* The local-I/O thread and kernel-socket readiness (kernel.c). */
void rusteron_dpdk_local_io_stop(rusteron_dpdk_transport_t *native);
int rusteron_dpdk_ksock_register(rusteron_dpdk_transport_t *native, int fd);
void rusteron_dpdk_ksock_unregister(rusteron_dpdk_transport_t *native, int slot, int fd);
uint16_t rusteron_dpdk_port_tx(
    rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *port, rusteron_dpdk_mbuf_t **pkts, uint16_t nb);
/* Send the frames queued behind a full ring, first in, first out: 1 when none
 * are left, 0 while some still wait (the ring is full). Agent thread only. */
int rusteron_dpdk_port_tx_flush(rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *port);
/* Release the frames still queued: the NIC's last transport closed. */
void rusteron_dpdk_port_tx_drop_queued(rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *port);
/* The driver context is known (install): its threading mode
 * (RUSTERON_DPDK_THREADING_*) and its agents' CPUs. In a shared mode one
 * thread also receives, so a send never waits out a pacing debt: an unset
 * RUSTERON_DPDK_TX_PACE_SPIN_US is 0 there, and a non-zero one fails (-1). */
int rusteron_dpdk_runtime_agents_known(
    rusteron_dpdk_transport_t *native, int threading_mode, int conductor, int sender, int receiver,
    const char *conductor_idle);
void rusteron_dpdk_local_io_open_requested(rusteron_dpdk_transport_t *native, uint64_t now_ns);

/* One housekeeping pass: run the PMDs' due timers (ENA's watchdog; 1 Hz
 * gated), report the ports' new device-reset requests, and sample both ports'
 * statistics (1 Hz gated). */
void rusteron_dpdk_housekeeping_tick(rusteron_dpdk_transport_t *native, uint64_t now_ns);
/* Start/stop the housekeeping thread (runtime init/cleanup). */
int rusteron_dpdk_housekeeping_start(rusteron_dpdk_transport_t *native);
void rusteron_dpdk_housekeeping_stop(rusteron_dpdk_transport_t *native);
/* The CPU the EAL's main lcore runs on, or -1 (the fake EAL): where the
 * housekeeping thread goes unless RUSTERON_DPDK_HOUSEKEEPING_CPU says. */
int rusteron_dpdk_eal_main_cpu(void);
/* Where housekeeping and local I/O run: 1 with `set` filled, 0 unpinned, -1
 * bad env. */
#include <sched.h>
#ifdef CPU_SETSIZE /* cpu_set_t: only units built with _GNU_SOURCE + sched.h */
int rusteron_dpdk_housekeeping_affinity(const rusteron_dpdk_transport_t *native, cpu_set_t *set);
/* `cpus` NULL: unpinned. */
int rusteron_dpdk_local_io_start(rusteron_dpdk_transport_t *native, const cpu_set_t *cpus);
/* The CPUs in `set` as "0,3,5", into `buf`, which it returns. */
const char *rusteron_dpdk_cpu_set_list(const cpu_set_t *set, char *buf, size_t len);
#endif

/* EAL seam — implemented by rusteron_dpdk_eal.c (real rte_eal) in production
 * and by test/rusteron_dpdk_fake_eal.c in test builds. */
typedef struct rusteron_dpdk_eal_params_stct
{
    const rusteron_dpdk_config_t *config;
    int mode; /* RUSTERON_DPDK_EAL_* */
} rusteron_dpdk_eal_params_t;

int rusteron_dpdk_eal_init(const rusteron_dpdk_eal_params_t *params, char *errbuf, size_t errlen);
int rusteron_dpdk_eal_is_initialized(void);

/* Register/unregister the calling thread with DPDK (plan §7.2: "Register Aeron
 * network threads with DPDK"). Real impl in rusteron_dpdk_eal.c, no-op in
 * test/rusteron_dpdk_fake_eal.c. Both are safe on already-registered and EAL
 * threads. */
int rusteron_dpdk_eal_thread_register(void);
int rusteron_dpdk_eal_thread_unregister(void);

/* The DPDK the transport is built against (RTE_VER_YEAR, RTE_VER_MONTH): its
 * ENA PMD's device arguments changed in 24.11. Real in rusteron_dpdk_eal.c,
 * settable in the fake EAL. */
void rusteron_dpdk_eal_version(int *year, int *month);

/* MANA's per-queue ring limit (MAX_RECEIVE/SEND_BUFFERS_PER_QUEUE in the PMD). */
#define RUSTERON_DPDK_MANA_MAX_DESCRIPTORS 256

/* The receive ring a port takes when rx_descriptors is 0: this, or fewer
 * where the NIC's reported maximum (MANA's 256) is lower. */
#define RUSTERON_DPDK_RX_DESCRIPTORS_DEFAULT 4096

/* RUSTERON_DPDK_ENA_DEVARGS: comma-separated key=value device arguments
 * (keys and values of [A-Za-z0-9_.:-]), at most this long. */
#define RUSTERON_DPDK_ENA_DEVARGS_MAX 200

/* An ENA port whose LLQ the operator's device arguments turn off (the port's
 * ena_llq; beyond what dev_info reports, RUSTERON_DPDK_ENA_LLQ_*). */
#define RUSTERON_DPDK_ENA_LLQ_DISABLED 5

/* The ENA device arguments every ENA port gets (`-a <bdf>,<these>`): the
 * operator's RUSTERON_DPDK_ENA_DEVARGS, and the PMD's LLQ setting unless
 * those name it (`llq_policy=3`, large 256-byte entries, from DPDK 24.11,
 * whose 24.11.0 and .1 otherwise disable LLQ; `enable_llq=1`
 * before). *llq_off is set when the operator's arguments turn LLQ off. 0 on
 * success; -1 (error set, naming the variable) when they are malformed,
 * the LLQ key's value included unless a decimal in the PMD's range. */
int rusteron_dpdk_ena_devargs(char *out, size_t len, int *llq_off);

/* How `bdf`'s BAR 2 (an ENA's LLQ BAR) is mapped in the memory map at
 * `maps_path` (a /proc/<pid>/maps): RUSTERON_DPDK_ENA_LLQ_WC, _UNCACHED,
 * _UNSEEN, or _UNKNOWN when it cannot be read. DPDK-free (runtime.c). */
int rusteron_dpdk_ena_llq_mapping(const char *maps_path, const char *bdf);

/* Parse aa:bb:cc:dd:ee:ff (either case) into bytes. 0 on success. */
int rusteron_dpdk_parse_mac(const char *text, uint8_t mac[RUSTERON_DPDK_ETH_ADDR_LEN]);

/* The EAL device arguments for the configured ports, space-separated, into
 * `out`: `-a <bdf>` per ENA, `-a <bdf>,mac=<m>[,mac=<m>]` once per MANA
 * function, `--vdev=<name>` per test vdev. DPDK-free so tests can assert it;
 * rusteron_dpdk_eal.c splits it into argv. 0 on success. */
int rusteron_dpdk_eal_allow_args(const rusteron_dpdk_config_t *cfg, char *out, size_t len);

/* RUSTERON_DPDK_EAL_IN_MEMORY (0/1/false/true, unset 0) into *in_memory:
 * --in-memory instead of hugepage files, so no multi-process channel and no
 * files left in hugetlbfs after a crash. 0 on success; -1 (error code
 * RUSTERON_DPDK_ERR_INVALID_ENV, naming the variables) when it is malformed
 * or RUSTERON_DPDK_EAL_ARGS contradicts it. DPDK-free (runtime.c). */
int rusteron_dpdk_eal_in_memory(int *in_memory);

/* The EAL's memory options for `cfg` in EAL mode `mode`, space-separated,
 * into `out`: `--huge-dir <dir>`, `--in-memory`, `--no-huge` or
 * `--no-huge --in-memory`, as rusteron_dpdk_eal.c passes them. 0 on success,
 * -1 as rusteron_dpdk_eal_in_memory. */
int rusteron_dpdk_eal_mem_args(const rusteron_dpdk_config_t *cfg, int mode, char *out, size_t len);

/* Runtime orchestration (rusteron_dpdk_runtime.c). */
int rusteron_dpdk_runtime_init(rusteron_dpdk_transport_t *native);
void rusteron_dpdk_runtime_cleanup(rusteron_dpdk_transport_t *native);

/* Resolve a device BDF against the allow-list. Only the two configured ENA
 * devices may be opened; anything else is rejected (plan §7.2). */
int rusteron_dpdk_runtime_probe_device(
    rusteron_dpdk_transport_t *native, const char *pci_bdf, uint16_t *port_id);

/* Test hooks (runtime.c). Production behaviour is the default (REAL, no
 * override, reset is a no-op for a fresh process). */
void rusteron_dpdk_test_reset(void);
/* Take `port`'s queued error report, if any (housekeeping): its message into
 * `message` and the fixed description for the driver's distinct error log
 * (one entry per kind, not per peer) into *description. Returns its kind, or
 * RUSTERON_DPDK_REPORT_NONE. poller.c. */
uint32_t rusteron_dpdk_report_take(
    const rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *port, char *message, size_t len,
    const char **description);
/* How many receive-path error reports were formatted (poller.c), and its
 * reset. */
int rusteron_dpdk_test_reports_formatted(void);
void rusteron_dpdk_reports_test_reset(void);
void rusteron_dpdk_test_set_eal_mode(int mode);
void rusteron_dpdk_set_port_ops(const rusteron_dpdk_port_ops_t *ops);
/* The kernel driver every PCI port reads as bound to, instead of its sysfs
 * driver link ("" or NULL: sysfs again; reset clears it). */
void rusteron_dpdk_test_set_pci_kernel_driver(const char *driver);

/* The local-I/O mode a created runtime selected (RUSTERON_DPDK_LOCAL_IO_*,
 * -1 for no runtime). Read back to prove which mode an environment value
 * actually selected. */
int rusteron_dpdk_test_local_io_mode(const rusteron_dpdk_transport_t *native);

/* Test reset for the transport.c singleton (cleared by rusteron_dpdk_test_reset). */
void rusteron_dpdk_transport_test_reset(void);

/* Monotonic nanosecond clock (runtime.c), started at create
 * (rusteron_dpdk_clock_start, with the config's choice): the CPU counter (the
 * TSC on x86_64, the generic timer's virtual counter CNTVCT_EL0 on aarch64) at
 * the rate DPDK's EAL measured or read (rte_get_tsc_hz), or at the clock
 * start's own measurement where the EAL's is more than 100 ppm off it,
 * converted with a 32.32 factor; or CLOCK_MONOTONIC through the same formula
 * with a factor of one. The counter is used only where the EAL's rate can be
 * trusted (rusteron_dpdk_clock_rule); otherwise, and until a transport starts
 * it, the clock is CLOCK_MONOTONIC. Either way, nanoseconds since boot. Tests
 * pin it with the setter so ARP rate limiting, expiry and the counter sample
 * gate are deterministic; reset clears the pin and returns the clock to its
 * boot default. The source values are what counter type 80 reports, so they
 * never change: 0 CLOCK_MONOTONIC, 1 the TSC, 2 CNTVCT_EL0. */
#define RUSTERON_DPDK_CLOCK_MONOTONIC 0
#define RUSTERON_DPDK_CLOCK_TSC 1
#define RUSTERON_DPDK_CLOCK_CNTVCT 2
#define RUSTERON_DPDK_CLOCK_PINNED 3
/* This architecture's counter source. */
#if defined(__x86_64__)
#define RUSTERON_DPDK_CLOCK_COUNTER RUSTERON_DPDK_CLOCK_TSC
#else
#define RUSTERON_DPDK_CLOCK_COUNTER RUSTERON_DPDK_CLOCK_CNTVCT
#endif

/* The clock's state (runtime.c). Hidden, so the inlined fast path below reads
 * it PC-relative rather than through the GOT. */
typedef struct rusteron_dpdk_clock_stct
{
    int mode;            /* RUSTERON_DPDK_CLOCK_{COUNTER,MONOTONIC,PINNED} */
    uint64_t base_ticks; /* the ticks at the base */
    uint64_t base_ns;    /* the time, ns, at the base (monotonic domain) */
    uint64_t mult;       /* ns per tick, 32.32 fixed point, in (0, 2^63) */
} rusteron_dpdk_clock_t;
extern rusteron_dpdk_clock_t rusteron_dpdk_clock __attribute__((visibility("hidden")));
/* Out of line and cold, also where it could be inlined (runtime.c), so the
 * fast path below takes no stack frame for the fallback's timespec. */
__attribute__((noinline, cold)) uint64_t rusteron_dpdk_clock_ns_slow(void);

/* One read of the hardware counter, as fast as it can be read: rdtsc on
 * x86_64, mrs CNTVCT_EL0 on aarch64 (DPDK's rte_rdtsc on each). Neither waits
 * for the instructions before it, which a few ns of skew in a freshness,
 * pacing or loop-gap time never matters to. */
static inline uint64_t rusteron_dpdk_counter(void)
{
#if defined(__x86_64__)
    return __rdtsc();
#else
    uint64_t ticks;
    __asm__ __volatile__("mrs %0, cntvct_el0" : "=r"(ticks));
    return ticks;
#endif
}

/* `ticks` in ns through the started conversion:
 * base_ns + ((ticks - base_ticks) * mult) >> 32. The difference is signed, so
 * a tick a little behind the base (an unordered read on another core, just
 * after the start) is a little before base_ns, never 2^64 ns later; one
 * widening signed multiply (x86 imul, Arm mul and smulh). */
static inline uint64_t rusteron_dpdk_clock_ns_at(uint64_t ticks)
{
    const int64_t delta = (int64_t)(ticks - rusteron_dpdk_clock.base_ticks);
    return rusteron_dpdk_clock.base_ns +
        (uint64_t)(int64_t)(((__int128)delta * (int64_t)rusteron_dpdk_clock.mult) >> 32);
}

/* The transport's clock read, inlined into the hot paths: the counter
 * converted (a read, a subtraction, a multiply and an add), or the slow path
 * (the CLOCK_MONOTONIC fallback, a test pin). */
static inline uint64_t rusteron_dpdk_clock_now(void)
{
    if (__builtin_expect(RUSTERON_DPDK_CLOCK_COUNTER == rusteron_dpdk_clock.mode, 1))
    {
        return rusteron_dpdk_clock_ns_at(rusteron_dpdk_counter());
    }
    return rusteron_dpdk_clock_ns_slow();
}

/* rusteron_dpdk_clock_now as an exported function (tests, benchmarks). */
uint64_t rusteron_dpdk_clock_ns(void);
/* What the EAL says about the counter (rusteron_dpdk_eal_counter_rate): all
 * zero before the EAL is up. */
typedef struct rusteron_dpdk_counter_rate_stct
{
    uint64_t eal_hz; /* rte_get_tsc_hz() */
    uint64_t cntfrq; /* aarch64: CNTFRQ_EL0, the generic timer's rate; 0 on x86_64 */
    int invariant;   /* x86_64: CPUID 0x80000007 EDX bit 8, an invariant TSC; 0 on aarch64 */
} rusteron_dpdk_counter_rate_t;
/* EAL seam (rusteron_dpdk_eal.c, or the fake EAL's settable values). */
void rusteron_dpdk_eal_counter_rate(rusteron_dpdk_counter_rate_t *rate);
/* The rate range the transport converts: mult stays in (0, 2^63). */
#define RUSTERON_DPDK_CLOCK_MIN_HZ 1000000ULL
#define RUSTERON_DPDK_CLOCK_MAX_HZ 20000000000ULL
/* Whether the counter `source` (RUSTERON_DPDK_CLOCK_TSC or _CNTVCT) can run at
 * `rate->eal_hz`: RUSTERON_DPDK_CLOCK_WHY_COUNTER, or why not (_NO_RATE: none,
 * or outside the range above; _NOT_INVARIANT: a TSC CPUID does not call
 * invariant; _NOT_CNTFRQ: an EAL rate that is not CNTFRQ_EL0, a DPDK built to
 * time with the PMU's cycle counter, RTE_ARM_EAL_RDTSC_USE_PMU). Pure. */
int rusteron_dpdk_clock_rule(int source, const rusteron_dpdk_counter_rate_t *rate);
/* How far, in parts per million of the measurement, DPDK's rate may be from
 * the one the clock start measures before the measurement replaces it. */
#define RUSTERON_DPDK_CLOCK_RATE_TOLERANCE_PPM 100u
/* The rate the counter is converted at, into *hz: DPDK's (`eal_hz`, in the
 * rule's range) where the start-up measurement (`measured_hz`) agrees with it
 * within the tolerance above, or is no rate the transport converts (0: none);
 * else the measurement. RUSTERON_DPDK_CLOCK_RATE_EAL or _MEASURED. Pure. */
int rusteron_dpdk_clock_choose_rate(uint64_t eal_hz, uint64_t measured_hz, uint64_t *hz);
/* How long the clock start measures the counter against CLOCK_MONOTONIC_RAW:
 * each end the tightest of 16 bracketed reads, so a bracket of 100 ns is
 * 4 ppm of it. */
#define RUSTERON_DPDK_CLOCK_MEASURE_NS 25000000ULL
/* Start the transport's clock for the process with the config's `choice`
 * (RUSTERON_DPDK_CLOCK_CHOICE_*): where the rule allows the counter, measure
 * its rate (above), choose the rate (rusteron_dpdk_clock_choose_rate) and
 * anchor the counter on CLOCK_MONOTONIC; else CLOCK_MONOTONIC with a factor of
 * one. A test pin stays in force. Create calls it once the EAL is up, before
 * the ports and threads. */
void rusteron_dpdk_clock_start(int choice);
/* Tests: the rate the clock start takes as measured, in place of measuring
 * (0: measure); the reset clears it. */
void rusteron_dpdk_test_set_measured_counter_hz(uint64_t hz);
/* RUSTERON_DPDK_CLOCK_COUNTER or RUSTERON_DPDK_CLOCK_MONOTONIC: the started
 * clock's (a pin aside). */
int rusteron_dpdk_clock_source(void);
/* The started conversion applied to `ticks` (tests). */
uint64_t rusteron_dpdk_test_clock_ns_at(uint64_t ticks);
/* The started conversion's base ticks, base ns and factor (tests). */
void rusteron_dpdk_test_clock_params(uint64_t *base_ticks, uint64_t *base_ns, uint64_t *mult);
/* One read of the counter the clock converts (tests, the benchmark). */
uint64_t rusteron_dpdk_test_counter(void);
/* Pin the clock at `ns`. Pinning a clock not pinned yet also moves the active
 * transport's allowances onto it (their last refill becomes `ns`). */
void rusteron_dpdk_test_set_clock_ns(uint64_t ns);
/* Move the pinned clock on by `ns` at every read from the calling thread (0:
 * it stays put), so a wait spun on it ends; reset clears it. */
void rusteron_dpdk_test_set_clock_step_ns(uint64_t ns);
/* Move the pinned clock on by `ns` without reading it (a fake PMD call that
 * takes that long); nothing when the clock is not pinned. */
void rusteron_dpdk_test_advance_clock_ns(uint64_t ns);
/* Reads of the pinned clock since the reset (tests: what a path costs). */
uint64_t rusteron_dpdk_test_clock_reads(void);
/* CLOCK_THREAD_CPUTIME_ID in ns, the clock gap attribution splits a poll gap
 * with; pinned by the test hook below. */
uint64_t rusteron_dpdk_thread_cpu_ns(void);
/* Pin the thread CPU clock at `ns`; reset clears the pin. */
void rusteron_dpdk_test_set_thread_cpu_ns(uint64_t ns);
/* RUSTERON_DPDK_GAP_ATTRIB's two parts: time the PMD bursts (1 or `burst`),
 * and split poll gaps by the thread CPU clock (1 only). */
#define RUSTERON_DPDK_GAP_ATTRIB_BURST 1
#define RUSTERON_DPDK_GAP_ATTRIB_CPU 2
/* Which of them RUSTERON_DPDK_GAP_ATTRIB turned on (0 default). */
int rusteron_dpdk_gap_attributing(void);
/* The same, read inline on the agents' paths (runtime.c sets it at create). */
extern int rusteron_dpdk_gap_attrib __attribute__((visibility("hidden")));
/* PMD burst timing (two transport clock reads a burst). */
static inline int rusteron_dpdk_gap_attrib_on(void)
{
    return __builtin_expect(0 != (rusteron_dpdk_gap_attrib & RUSTERON_DPDK_GAP_ATTRIB_BURST), 0);
}
/* The work/descheduling split (a thread CPU clock read, a system call, a poll). */
static inline int rusteron_dpdk_gap_cpu_on(void)
{
    return __builtin_expect(0 != (rusteron_dpdk_gap_attrib & RUSTERON_DPDK_GAP_ATTRIB_CPU), 0);
}

/* The lab event log (rusteron_dpdk_events.c). Open it when
 * RUSTERON_DPDK_EVENT_LOG names a file (0; -1 with the error set when it
 * cannot be written), drain every ring into it (housekeeping), and drain and
 * close it (cleanup, after every writer stopped). */
int rusteron_dpdk_events_open(rusteron_dpdk_transport_t *native);
void rusteron_dpdk_events_drain(rusteron_dpdk_transport_t *native);
void rusteron_dpdk_events_close(rusteron_dpdk_transport_t *native);
/* Tests: the events each ring holds (a power of two; 0 the default), and the
 * reset of that and the gap floor. */
void rusteron_dpdk_test_set_event_ring_events(uint32_t events);
void rusteron_dpdk_events_test_reset(void);

/* CLOCK_REALTIME in ns, the clock NIC RX timestamps are compared against
 * (chrony disciplines it to the NIC's PTP hardware clock), and the media
 * receive timestamp of a channel asking for one (once a receive burst). Read
 * only when a timestamped frame arrives or such a channel is open. Tests pin
 * it; reset clears the pin. */
uint64_t rusteron_dpdk_realtime_ns(void);
void rusteron_dpdk_test_set_realtime_ns(uint64_t ns);
/* Each read of the pinned realtime clock then advances it by `ns` (tests: a
 * clock that moves between reads; 0 after a reset). */
void rusteron_dpdk_test_set_realtime_step_ns(uint64_t ns);
/* Reads of the pinned realtime clock since the pin (tests: what a poll costs). */
uint64_t rusteron_dpdk_test_realtime_reads(void);

/* The process-lifetime native runtime singleton (transport.c), or NULL when no
 * transport has been created. EAL can be initialized once, so there is at most
 * one. */
rusteron_dpdk_transport_t *rusteron_dpdk_active_runtime(void);

/* Test inspection of the two role ports (distinct ports / mempools). */
void rusteron_dpdk_transport_test_dump(
    const rusteron_dpdk_transport_t *native,
    uint16_t *sender_port, uintptr_t *sender_pool,
    uint16_t *receiver_port, uintptr_t *receiver_pool);

/* Test-only ARP helpers (runtime.c) so the tx tests can drive the next-hop
 * cache directly: seed a reachable entry, or feed an incoming ARP frame through
 * the handler as if it arrived on the given role (0 = sender, 1 = receiver). */
int rusteron_dpdk_transport_test_arp_seed(
    rusteron_dpdk_transport_t *native,
    const char *ip_str, const uint8_t mac[6]);
int rusteron_dpdk_transport_test_arp_rx(
    rusteron_dpdk_transport_t *native,
    int role, const uint8_t *frame, size_t frame_len);

/* Thread-local error buffer (defined in rusteron_dpdk_transport.c). */
void rusteron_dpdk_set_error(const char *message);
void rusteron_dpdk_set_error_code(const char *message, int code);

/* Aeron poller callbacks (rusteron_dpdk_poller.c, plan §7.6/§7.7). */
int rusteron_dpdk_poller_init(
    aeron_udp_transport_poller_t *poller,
    aeron_driver_context_t *context,
    aeron_udp_channel_transport_affinity_t affinity);
int rusteron_dpdk_poller_close(aeron_udp_transport_poller_t *poller);
int rusteron_dpdk_poller_add(
    aeron_udp_transport_poller_t *poller, aeron_udp_channel_transport_t *transport);
int rusteron_dpdk_poller_remove(
    aeron_udp_transport_poller_t *poller, aeron_udp_channel_transport_t *transport);
int rusteron_dpdk_poller_poll(
    aeron_udp_transport_poller_t *poller,
    struct mmsghdr *msgvec,
    size_t vlen,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    aeron_udp_channel_transport_recvmmsg_func_t recvmmsg_func,
    void *clientd);

/* Shared receive loop (rusteron_dpdk_poller.c): poll `client`'s port and
 * dispatch each valid frame to recv_func. When `endpoints` is non-NULL the
 * target transport is chosen by the map; otherwise only frames addressed to
 * `only_transport`'s own local endpoint are dispatched. Returns the number of
 * datagrams dispatched; accumulates payload bytes in *bytes_rcved. */
int rusteron_dpdk_poller_receive(
    rusteron_dpdk_client_t *client,
    aeron_udp_channel_transport_t *only_transport,
    const rusteron_dpdk_endpoint_map_t *endpoints,
    struct mmsghdr *msgvec,
    size_t vlen,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    void *clientd);

/* Test inspection of the receive counters (runtime.c). */
void rusteron_dpdk_transport_test_local_watch(const void *transport, int *watched, int *unwatched);
void rusteron_dpdk_transport_test_rx_stats(
    const rusteron_dpdk_transport_t *native, rusteron_dpdk_rx_stats_t *out);

#ifdef __cplusplus
}
#endif

#endif /* RUSTERON_DPDK_INTERNAL_H */
