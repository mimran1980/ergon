/*
 * ergon-md-dpdk DPDK transport — receive endpoint map (plan §7.6).
 *
 * Open addressing with linear probing over a fixed 4096-entry slot array.
 * Deletion shifts later entries of the cluster back (Knuth's Algorithm R),
 * so a probe chain never develops a hole that would hide later entries. Add/remove are cold path (channel lifecycle); lookup is the hot
 * path and does a single linear scan starting at the hashed slot.
 */
#include "rusteron_dpdk_endpoint_map.h"

#include <string.h>

#include "aeron_alloc.h"

#define RUSTERON_DPDK_ENDPOINT_MAP_MASK (RUSTERON_DPDK_ENDPOINT_MAP_SIZE - 1)

/* Mixer so consecutive (ip, port) keys spread across the table. */
static uint32_t rusteron_dpdk_endpoint_hash(uint32_t ip, uint16_t port)
{
    uint32_t h = ip ^ (uint32_t)port;
    h = (h ^ (h >> 16)) * 0x85ebca6bu;
    h = (h ^ (h >> 13)) * 0xc2b2ae35u;
    return h ^ (h >> 16);
}

int rusteron_dpdk_endpoint_map_init(rusteron_dpdk_endpoint_map_t *map)
{
    if (NULL == map)
    {
        return -1;
    }
    if (aeron_alloc(
            (void **)&map->slots,
            RUSTERON_DPDK_ENDPOINT_MAP_SIZE * sizeof(rusteron_dpdk_endpoint_entry_t)) < 0)
    {
        map->slots = NULL;
        return -1;
    }
    map->count = 0;
    return 0;
}

void rusteron_dpdk_endpoint_map_close(rusteron_dpdk_endpoint_map_t *map)
{
    if (NULL != map && NULL != map->slots)
    {
        aeron_free(map->slots);
        map->slots = NULL;
        map->count = 0;
    }
}

int rusteron_dpdk_endpoint_map_put(
    rusteron_dpdk_endpoint_map_t *map, uint32_t dst_ip, uint16_t dst_port, void *value)
{
    if (NULL == map || NULL == map->slots)
    {
        return -1;
    }

    const uint32_t mask = RUSTERON_DPDK_ENDPOINT_MAP_MASK;
    size_t slot = (size_t)(rusteron_dpdk_endpoint_hash(dst_ip, dst_port) & mask);
    for (uint32_t n = 0; n < RUSTERON_DPDK_ENDPOINT_MAP_SIZE; n++)
    {
        rusteron_dpdk_endpoint_entry_t *entry = &map->slots[slot];
        if (!entry->occupied)
        {
            /* At most 3/4 full: probe chains stay short on the hot path, and
             * an empty slot always ends every probe and removal. */
            if (map->count >= RUSTERON_DPDK_ENDPOINT_MAP_MAX)
            {
                return RUSTERON_DPDK_ENDPOINT_MAP_FULL;
            }
            entry->dst_ip = dst_ip;
            entry->dst_port = dst_port;
            entry->occupied = 1;
            entry->value = value;
            map->count++;
            return 0;
        }
        if (entry->dst_ip == dst_ip && entry->dst_port == dst_port)
        {
            return entry->value == value ? 0 : -1; /* idempotent re-add or conflict */
        }
        slot = (slot + 1) & mask;
    }
    return -1;
}

void rusteron_dpdk_endpoint_map_remove(
    rusteron_dpdk_endpoint_map_t *map, uint32_t dst_ip, uint16_t dst_port)
{
    if (NULL == map || NULL == map->slots)
    {
        return;
    }

    const uint32_t mask = RUSTERON_DPDK_ENDPOINT_MAP_MASK;
    size_t slot = (size_t)(rusteron_dpdk_endpoint_hash(dst_ip, dst_port) & mask);
    size_t found = RUSTERON_DPDK_ENDPOINT_MAP_SIZE;
    for (uint32_t n = 0; n < RUSTERON_DPDK_ENDPOINT_MAP_SIZE; n++)
    {
        rusteron_dpdk_endpoint_entry_t *entry = &map->slots[slot];
        if (!entry->occupied)
        {
            return; /* absent */
        }
        if (entry->dst_ip == dst_ip && entry->dst_port == dst_port)
        {
            found = slot;
            break;
        }
        slot = (slot + 1) & mask;
    }
    if (found == RUSTERON_DPDK_ENDPOINT_MAP_SIZE)
    {
        return;
    }

    /* Backward-shift deletion for linear probing (Knuth, Algorithm R): walk
     * the cluster after the hole to its first empty slot; an entry at `j`
     * whose home slot is not cyclically within (gap, j] would be hidden by
     * the hole, so it moves into the gap and its old slot becomes the gap. An
     * entry already at its home does not end the walk: later ones may still
     * need to move past it. */
    size_t gap = found;
    size_t j = (gap + 1) & mask;
    for (uint32_t n = 1; n < RUSTERON_DPDK_ENDPOINT_MAP_SIZE && map->slots[j].occupied; n++, j = (j + 1) & mask)
    {
        const size_t home =
            (size_t)(rusteron_dpdk_endpoint_hash(map->slots[j].dst_ip, map->slots[j].dst_port) & mask);
        const int home_in_range = gap <= j ? (gap < home && home <= j) : (gap < home || home <= j);
        if (!home_in_range)
        {
            map->slots[gap] = map->slots[j];
            gap = j;
        }
    }
    memset(&map->slots[gap], 0, sizeof(map->slots[gap]));
    map->count--;
}

void *rusteron_dpdk_endpoint_map_get(
    const rusteron_dpdk_endpoint_map_t *map, uint32_t dst_ip, uint16_t dst_port)
{
    if (NULL == map || NULL == map->slots)
    {
        return NULL;
    }

    const uint32_t mask = RUSTERON_DPDK_ENDPOINT_MAP_MASK;
    size_t slot = (size_t)(rusteron_dpdk_endpoint_hash(dst_ip, dst_port) & mask);
    for (uint32_t n = 0; n < RUSTERON_DPDK_ENDPOINT_MAP_SIZE; n++)
    {
        rusteron_dpdk_endpoint_entry_t *entry = &map->slots[slot];
        if (!entry->occupied)
        {
            return NULL;
        }
        if (entry->dst_ip == dst_ip && entry->dst_port == dst_port)
        {
            return entry->value;
        }
        slot = (slot + 1) & mask;
    }
    return NULL;
}

size_t rusteron_dpdk_endpoint_map_count(const rusteron_dpdk_endpoint_map_t *map)
{
    return NULL == map ? 0 : (size_t)map->count;
}
