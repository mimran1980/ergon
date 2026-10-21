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
 * The transport's clock: ergon-clock's calibration (its `Params`, the same
 * layout), made once in Rust and adopted at create. Both kinds convert with
 * ergon-clock's formula:
 *
 *   ns = base_ns + (int64_t)(((__int128)(int64_t)(ticks - base_ticks) * (int64_t)mult) >> 32)
 *
 * with `ticks` the hardware counter (kind 1: rdtsc on x86_64, CNTVCT_EL0 on
 * aarch64) or CLOCK_MONOTONIC in nanoseconds (kind 0, the fallback, whose
 * factor is 2^32 until its owner retunes it).
 */
#define RUSTERON_DPDK_CLOCK_PARAMS_VERSION 1 /* ergon-clock's PARAMS_VERSION */
#define RUSTERON_DPDK_CLOCK_KIND_MONOTONIC 0
#define RUSTERON_DPDK_CLOCK_KIND_COUNTER 1

typedef struct rusteron_dpdk_clock_params_stct
{
    uint32_t version;    /* RUSTERON_DPDK_CLOCK_PARAMS_VERSION */
    uint32_t kind;       /* RUSTERON_DPDK_CLOCK_KIND_{COUNTER,MONOTONIC} */
    uint32_t domain;     /* 0: the monotonic domain, the only one the transport takes */
    uint32_t reserved;   /* 0 */
    uint64_t base_ticks; /* the ticks at the base */
    uint64_t base_ns;    /* the time, ns, at the base */
    uint64_t mult;       /* ns per tick, 32.32 fixed point, in (0, 2^63) */
    uint64_t hz;         /* ticks a second (the rate mult stands for; not read) */
}
rusteron_dpdk_clock_params_t;

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
     * ergon-clock's calibration, checked at create and adopted for the
     * process. */
    rusteron_dpdk_clock_params_t clock;
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

/* The populated Aeron transport binding table (static, process-lifetime). */
aeron_udp_channel_transport_bindings_t *rusteron_dpdk_transport_bindings(void);

/* Last recorded native error message (thread-local, valid until next call). */
const char *rusteron_dpdk_last_error(void);

/* Machine-readable error code for the last native failure (plan §7.2). */
#define RUSTERON_DPDK_ERR_OK 0
#define RUSTERON_DPDK_ERR_NATIVE 1
#define RUSTERON_DPDK_ERR_ALREADY_INITIALIZED 2
int rusteron_dpdk_last_error_code(void);

#ifdef __cplusplus
}
#endif

#endif /* RUSTERON_DPDK_TRANSPORT_H */
