/*
 * ergon-md-dpdk DPDK transport — receive endpoint map (plan §7.6).
 *
 * A fixed-capacity open-addressed map keyed by (destination IPv4, UDP port)
 * that the poller uses to dispatch a received frame to the Aeron transport
 * whose local endpoint it is addressed to. `poller_add`/`poller_remove` mutate
 * it outside the hot polling path; the hot path only probes. No allocation
 * after init, no locking, no logging.
 *
 * This header is DPDK-free and holds no Aeron types (the value is an opaque
 * pointer), so the map stays linkable wherever the core archive is.
 */
#ifndef RUSTERON_DPDK_ENDPOINT_MAP_H
#define RUSTERON_DPDK_ENDPOINT_MAP_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define RUSTERON_DPDK_ENDPOINT_MAP_SIZE 4096
#define RUSTERON_DPDK_ENDPOINT_MAP_MAX 3072 /* entries: 3/4 of the slots */
#define RUSTERON_DPDK_ENDPOINT_MAP_FULL (-2) /* put: no room (-1: the key is another's) */

typedef struct rusteron_dpdk_endpoint_entry_stct
{
    uint32_t dst_ip;   /* network order */
    uint16_t dst_port; /* host order */
    uint8_t occupied;
    uint8_t padding;
    void *value;
} rusteron_dpdk_endpoint_entry_t;

typedef struct rusteron_dpdk_endpoint_map_stct
{
    rusteron_dpdk_endpoint_entry_t *slots; /* RUSTERON_DPDK_ENDPOINT_MAP_SIZE entries */
    uint32_t count;
} rusteron_dpdk_endpoint_map_t;

/* Allocate and zero the slot array. 0 on success. */
int rusteron_dpdk_endpoint_map_init(rusteron_dpdk_endpoint_map_t *map);

/* Release the slot array; the map is unusable until re-inited. */
void rusteron_dpdk_endpoint_map_close(rusteron_dpdk_endpoint_map_t *map);

/* Insert (dst_ip, dst_port) -> value. 0 on success; -1 when the key is already
 * mapped to a different value (duplicate endpoint) or the table is full. */
int rusteron_dpdk_endpoint_map_put(
    rusteron_dpdk_endpoint_map_t *map, uint32_t dst_ip, uint16_t dst_port, void *value);

/* Remove the key (no-op when absent). */
void rusteron_dpdk_endpoint_map_remove(
    rusteron_dpdk_endpoint_map_t *map, uint32_t dst_ip, uint16_t dst_port);

/* Look up a key; NULL when absent. Hot-path safe. */
void *rusteron_dpdk_endpoint_map_get(
    const rusteron_dpdk_endpoint_map_t *map, uint32_t dst_ip, uint16_t dst_port);

size_t rusteron_dpdk_endpoint_map_count(const rusteron_dpdk_endpoint_map_t *map);

#ifdef __cplusplus
}
#endif

#endif /* RUSTERON_DPDK_ENDPOINT_MAP_H */
