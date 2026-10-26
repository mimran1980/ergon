/*
 * ergon-md-dpdk DPDK ENA kernel-bypass transport — stable ABI.
 *
 * This header is the Rust/native boundary for the DPDK transport. It is a
 * Rusteron-owned ABI: it references Aeron C types (aeron_driver_context_t,
 * aeron_udp_channel_transport_bindings_t, ...) and never exposes DPDK types.
 *
 * The transport handle is opaque. The configuration struct is a shared
 * fixed-layout value type so Rust can construct it directly with #[repr(C)]
 * without a battery of native setters. Both the C side and the Rust ffi module
 * must stay in agreement; the struct carries a size field so a layout mismatch
 * fails loudly instead of corrupting state.
 */
#ifndef RUSTERON_DPDK_TRANSPORT_H
#define RUSTERON_DPDK_TRANSPORT_H

#include <stddef.h>
#include <stdint.h>

#include "aeron_driver_context.h"
#include "media/aeron_udp_channel_transport_bindings.h"

#ifdef __cplusplus
extern "C" {
#endif

#define RUSTERON_DPDK_CONFIG_STRUCT_SIZE (sizeof(struct rusteron_dpdk_config_stct))

/*
 * Fixed-layout configuration shared with Rust (see ergon-md-dpdk
 * src/ffi.rs for the matching #[repr(C)] definition).
 *
 * Strings are NUL-terminated fixed buffers sized generously; IPv4 addresses
 * and gateways use dotted-quad text, PCI addresses use canonical dddd:bb:ss.f.
 */
/* Jumbo frames: an AWS 9001-byte VPC MTU less IPv4+UDP, 32-aligned. */
#define RUSTERON_DPDK_MAX_AERON_MTU 8960

/*
 * The clock the transport reads (the config's `clock`): the CPU counter (rdtsc
 * on x86_64, CNTVCT_EL0 on aarch64) at the rate DPDK's EAL gives
 * (rte_get_tsc_hz), or at the rate measured at start where DPDK's is more
 * than 100 ppm off; or CLOCK_MONOTONIC. COUNTER falls back to CLOCK_MONOTONIC
 * where the EAL gives no rate the transport can trust; the clock report says
 * why, and which rate.
 */
#define RUSTERON_DPDK_CLOCK_CHOICE_COUNTER 0
#define RUSTERON_DPDK_CLOCK_CHOICE_MONOTONIC 1

typedef struct rusteron_dpdk_config_stct
{
    /* defensive layout guard — must equal sizeof of this struct */
    uint32_t struct_size;

    /* DPDK EAL */
    char file_prefix[65];      /* [A-Za-z0-9_-]{1,64} */
    char hugepage_dir[4096];   /* absolute path on hugetlbfs */

    /* sender ENA */
    char sender_pci[16];       /* dddd:bb:ss.f */
    char sender_ipv4[16];      /* dotted quad */
    uint8_t sender_prefix_len; /* 1..=32 */
    char sender_gateway[16];   /* dotted quad */

    /* receiver ENA */
    char receiver_pci[16];
    char receiver_ipv4[16];
    uint8_t receiver_prefix_len;
    char receiver_gateway[16];

    /* device tuning */
    uint16_t rx_descriptors;   /* 64..=8192, or 0: min(4096, the NIC's maximum), per port at start */
    uint16_t tx_descriptors;   /* 64..=8192 */
    uint32_t mbufs_per_port;
    uint16_t mempool_cache;
    uint16_t burst_size;       /* 1..=256 */
    size_t   max_aeron_mtu;    /* 32-aligned, <= 8960 and the NIC's max MTU - 28 */

    /* Optional port MACs, aa:bb:cc:dd:ee:ff or empty. Azure MANA exposes every
     * accelerated NIC on one PCI function, so a MANA port is selected by its
     * MAC (`-a <bdf>,mac=<mac>`); an ENA leaves these empty. */
    char sender_mac[18];
    char receiver_mac[18];

    /* 1 requests the NIC's hardware receive timestamps (RTE_ETH_RX_OFFLOAD_TIMESTAMP,
     * AWS ENA in a precision-time placement group); init fails on a port
     * without them. 0 leaves them off. */
    uint8_t rx_hw_timestamp;

    /* The clock the transport reads (ARP freshness, the pacer, poll gaps):
     * RUSTERON_DPDK_CLOCK_CHOICE_{COUNTER,MONOTONIC}, started at create for
     * the process. */
    uint8_t clock;

    /* ENA queue tuning (RUSTERON_DPDK_TX_FAST_FREE, RUSTERON_DPDK_RX_FREE_THRESH,
     * RUSTERON_DPDK_TX_FREE_THRESH). tx_fast_free 1 asks for
     * RTE_ETH_TX_OFFLOAD_MBUF_FAST_FREE, configured on a port whose NIC offers
     * it; 0 leaves it off. A free threshold of 0 keeps the PMD's default; else
     * it must be below its ring (the receive ring as each port takes it). */
    uint8_t tx_fast_free;
    uint16_t rx_free_thresh;
    uint16_t tx_free_thresh;
}
rusteron_dpdk_config_t;

typedef struct rusteron_dpdk_transport_stct rusteron_dpdk_transport_t;

/*
 * Create a native DPDK runtime and initialize both configured ENA ports.
 * On success *transport is non-NULL and owned by the caller. On failure the
 * native error is recorded for rusteron_dpdk_last_error() and -1 is returned.
 */
int rusteron_dpdk_transport_create(
    const rusteron_dpdk_config_t *config,
    rusteron_dpdk_transport_t **transport);

/*
 * Install the DPDK transport binding into an Aeron driver context. Only the
 * sender/receiver udp_channel_transport_bindings field is replaced; the
 * conductor's resolver bindings are left untouched.
 */
int rusteron_dpdk_transport_install(
    rusteron_dpdk_transport_t *transport,
    aeron_driver_context_t *context);

/* Destroy the native runtime. Idempotent: a NULL transport is a no-op. */
int rusteron_dpdk_transport_close(rusteron_dpdk_transport_t *transport);

/*
 * 1 once a port's PMD has asked for a device reset (RTE_ETH_EVENT_INTR_RESET).
 * The transport does not reset the device. The media driver stops its agents
 * and exits; a NULL transport returns 0. The flag stays set.
 */
int rusteron_dpdk_transport_nic_reset_exit(rusteron_dpdk_transport_t *transport);

/*
 * Ask each port's gateway for its MAC (ARP) from the calling thread, for up
 * to `timeout_ms`, polling the ports' receive queues itself: answered[0] and
 * answered[1] say whether the sender's and the receiver's answered. An
 * answer is learned, so the first send to it does not wait; a silent
 * gateway's entry is forgotten, so its first send resolves afresh. Only
 * before the driver starts: -1 (error set) once a transport is open, as an
 * agent may be polling the ports. 0 when it ran.
 */
int rusteron_dpdk_transport_gateway_check(void *transport, uint32_t timeout_ms, int32_t answered[2]);

/* The populated Aeron transport binding table (static, process-lifetime). */
aeron_udp_channel_transport_bindings_t *rusteron_dpdk_transport_bindings(void);

/* Last recorded native error message (thread-local, valid until next call). */
const char *rusteron_dpdk_last_error(void);

/*
 * The clock the transport started (at create; process-lifetime): its source,
 * as counter type 80 reports it, its rate and where that came from, and why
 * it is CLOCK_MONOTONIC when it is. Before any transport starts it,
 * CLOCK_MONOTONIC, why NOT_STARTED.
 */
#define RUSTERON_DPDK_CLOCK_WHY_COUNTER 0       /* the counter (`rate` says at which rate) */
#define RUSTERON_DPDK_CLOCK_WHY_CHOSEN 1        /* the config chose CLOCK_MONOTONIC */
#define RUSTERON_DPDK_CLOCK_WHY_NO_RATE 2       /* the EAL gave no rate, or one outside 1 MHz..20 GHz */
#define RUSTERON_DPDK_CLOCK_WHY_NOT_INVARIANT 3 /* x86_64: CPUID says the TSC is not invariant */
#define RUSTERON_DPDK_CLOCK_WHY_NOT_CNTFRQ 4    /* aarch64: the EAL's rate is not CNTFRQ_EL0 */
#define RUSTERON_DPDK_CLOCK_WHY_NOT_STARTED 5   /* no transport has started the clock */

/* Which rate the counter is converted at (the report's `rate`). */
#define RUSTERON_DPDK_CLOCK_RATE_EAL 0      /* DPDK's: the start-up measurement agrees to 100 ppm, or is no usable rate */
#define RUSTERON_DPDK_CLOCK_RATE_MEASURED 1 /* the start-up measurement: DPDK's is further off */

typedef struct rusteron_dpdk_clock_report_stct
{
    uint64_t hz;          /* counter ticks a second, the rate it is converted at; 0 on CLOCK_MONOTONIC */
    uint64_t eal_hz;      /* rte_get_tsc_hz() at start; 0 without an EAL */
    uint64_t cntfrq;      /* aarch64: CNTFRQ_EL0 at start; 0 on x86_64 */
    uint64_t measured_hz; /* the counter against CLOCK_MONOTONIC_RAW at start; 0 on CLOCK_MONOTONIC */
    int32_t source;       /* 0 CLOCK_MONOTONIC, 1 the TSC, 2 CNTVCT_EL0 (counter type 80) */
    int32_t why;          /* RUSTERON_DPDK_CLOCK_WHY_* */
    int32_t invariant;    /* x86_64: CPUID's invariant-TSC bit; 0 on aarch64 */
    int32_t rate;         /* RUSTERON_DPDK_CLOCK_RATE_*: which rate hz is (EAL on CLOCK_MONOTONIC) */
}
rusteron_dpdk_clock_report_t;

void rusteron_dpdk_clock_report(rusteron_dpdk_clock_report_t *report);

/* Machine-readable error code for the last native failure (plan §7.2). */
#define RUSTERON_DPDK_ERR_OK 0
#define RUSTERON_DPDK_ERR_NATIVE 1
#define RUSTERON_DPDK_ERR_ALREADY_INITIALIZED 2
#define RUSTERON_DPDK_ERR_INVALID_ENV 3    /* a malformed or contradictory environment value */
#define RUSTERON_DPDK_ERR_INVALID_CONFIG 4 /* a configuration the NIC as set up cannot carry */
#define RUSTERON_DPDK_ERR_NO_ACCELERATED_VF 5 /* Azure: a netvsc port without its accelerated-networking VF */
int rusteron_dpdk_last_error_code(void);

#ifdef __cplusplus
}
#endif

#endif /* RUSTERON_DPDK_TRANSPORT_H */
