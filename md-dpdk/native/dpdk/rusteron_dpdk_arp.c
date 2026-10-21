/*
 * ergon-md-dpdk DPDK transport — ARP table (plan §7.5).
 *
 * See rusteron_dpdk_arp.h for the behaviour contract. This file references the
 * full transport/port layout (via internal.h) only to reach the port ops for
 * allocating, building and transmitting ARP frames — it has no libdpdk symbols.
 */
#include "rusteron_dpdk_internal.h"

#include <string.h>

_Static_assert(sizeof(rusteron_dpdk_arp_entry_t) == 32, "an ARP entry is half a cache line");

/* Mixer so consecutive next-hop IPs spread across the open-addressed table. */
static uint32_t rusteron_dpdk_arp_hash(uint32_t ip)
{
    uint32_t h = ip;
    h = (h ^ (h >> 16)) * 0x85ebca6bu;
    h = (h ^ (h >> 13)) * 0xc2b2ae35u;
    return h ^ (h >> 16);
}

/* Open-addressed probe over at most RUSTERON_DPDK_ARP_MAX_PROBE slots.
 * Returns the entry holding `ip`, or the first EMPTY slot when it is absent
 * (where to insert it), or NULL when it is absent and the window is full.
 * Entries are replaced in place, never removed, so a window never develops a
 * hole that would hide an entry beyond it. */
static rusteron_dpdk_arp_entry_t *rusteron_dpdk_arp_slot_for(
    rusteron_dpdk_arp_table_t *table, uint32_t ip)
{
    const uint32_t mask = RUSTERON_DPDK_ARP_TABLE_SIZE - 1;
    const size_t slot = (size_t)(rusteron_dpdk_arp_hash(ip) & mask);

    for (size_t i = 0; i < RUSTERON_DPDK_ARP_MAX_PROBE; i++)
    {
        rusteron_dpdk_arp_entry_t *e = &table->entries[(slot + i) & mask];
        if (RUSTERON_DPDK_ARP_EMPTY == e->state)
        {
            return e; /* empty slot: absent, insert here */
        }
        if (e->ip == ip)
        {
            return e;
        }
    }
    return NULL;
}

/* A full window: the least recently used entry idle long enough to replace
 * (a resolution in flight is used every send, so never idle that long),
 * else NULL. */
static rusteron_dpdk_arp_entry_t *rusteron_dpdk_arp_victim(
    rusteron_dpdk_arp_table_t *table, uint32_t ip, uint64_t now_ns)
{
    const uint32_t mask = RUSTERON_DPDK_ARP_TABLE_SIZE - 1;
    const size_t slot = (size_t)(rusteron_dpdk_arp_hash(ip) & mask);
    const uint32_t now_used = (uint32_t)(now_ns >> RUSTERON_DPDK_ARP_USE_SHIFT);
    const uint32_t idle_min = (uint32_t)(RUSTERON_DPDK_ARP_EVICT_NS >> RUSTERON_DPDK_ARP_USE_SHIFT);
    rusteron_dpdk_arp_entry_t *victim = NULL;
    uint32_t victim_idle = 0;
    for (size_t i = 0; i < RUSTERON_DPDK_ARP_MAX_PROBE; i++)
    {
        rusteron_dpdk_arp_entry_t *e = &table->entries[(slot + i) & mask];
        const uint32_t idle = now_used - e->last_used;
        if (idle >= idle_min && idle >= victim_idle)
        {
            victim = e;
            victim_idle = idle;
        }
    }
    return victim;
}

static void rusteron_dpdk_arp_send_frame(
    rusteron_dpdk_transport_t *runtime, rusteron_dpdk_port_t *port, rusteron_dpdk_mbuf_t *m)
{
    rusteron_dpdk_mbuf_t *burst[1] = { m };
    rusteron_dpdk_port_tx(runtime, port, burst, 1);
}

static void rusteron_dpdk_arp_send_request(
    rusteron_dpdk_transport_t *runtime, rusteron_dpdk_port_t *port, uint32_t next_hop_ip)
{
    const rusteron_dpdk_port_ops_t *ops = runtime->ops;
    rusteron_dpdk_mbuf_t m;
    if (ops->mbuf_alloc(port->mempool, &m) < 0)
    {
        return; /* pool exhausted: the entry stays INCOMPLETE and is retried */
    }
    if (rusteron_dpdk_packet_build_arp_request(&m, port->mac, port->local_ip, next_hop_ip) < 0)
    {
        ops->mbuf_release(&m);
        return;
    }
    rusteron_dpdk_arp_send_frame(runtime, port, &m);
}

static void rusteron_dpdk_arp_send_reply(
    rusteron_dpdk_transport_t *runtime, rusteron_dpdk_port_t *port,
    const uint8_t requester_mac[RUSTERON_DPDK_ETH_ADDR_LEN], uint32_t requester_ip, uint32_t own_ip)
{
    const rusteron_dpdk_port_ops_t *ops = runtime->ops;
    rusteron_dpdk_mbuf_t m;
    if (ops->mbuf_alloc(port->mempool, &m) < 0)
    {
        return;
    }
    if (rusteron_dpdk_packet_build_arp_reply(&m, requester_mac, port->mac, own_ip, requester_ip) < 0)
    {
        ops->mbuf_release(&m);
        return;
    }
    rusteron_dpdk_arp_send_frame(runtime, port, &m);
}

int rusteron_dpdk_arp_resolve(
    rusteron_dpdk_arp_table_t *table,
    rusteron_dpdk_transport_t *runtime,
    rusteron_dpdk_port_t *port,
    uint32_t next_hop_ip,
    uint64_t now_ns,
    uint8_t out_mac[RUSTERON_DPDK_ETH_ADDR_LEN])
{
    rusteron_dpdk_arp_entry_t *e = rusteron_dpdk_arp_slot_for(table, next_hop_ip);

    if (NULL == e)
    {
        e = rusteron_dpdk_arp_victim(table, next_hop_ip, now_ns);
        if (NULL == e)
        {
            /* Every slot it may use holds a next hop in use: dropped until
             * one falls idle, reported at most once a minute. */
            if (!table->full_reported || now_ns - table->full_report_ns >= RUSTERON_DPDK_ARP_EVICT_NS)
            {
                table->full_reported = 1;
                table->full_report_ns = now_ns;
                return RUSTERON_DPDK_ARP_FULL;
            }
            return RUSTERON_DPDK_ARP_DROP;
        }
        memset(e, 0, sizeof(*e)); /* replaced: a fresh insert below */
    }
    e->last_used = (uint32_t)(now_ns >> RUSTERON_DPDK_ARP_USE_SHIFT);

    if (RUSTERON_DPDK_ARP_REACHABLE == e->state)
    {
        /* Never withdraw a learned next hop from the send path: refresh in
         * the background, keep sending to the cached MAC meanwhile. */
        if ((e->stale || now_ns - e->since_ns >= RUSTERON_DPDK_ARP_REFRESH_NS) &&
            (!e->refreshing || now_ns - e->last_request_ns >= RUSTERON_DPDK_ARP_REFRESH_RETRY_NS))
        {
            e->refreshing = 1;
            e->last_request_ns = now_ns;
            rusteron_dpdk_arp_send_request(runtime, port, next_hop_ip);
        }
        memcpy(out_mac, e->mac, RUSTERON_DPDK_ETH_ADDR_LEN);
        return 1;
    }

    if (RUSTERON_DPDK_ARP_FAILED == e->state)
    {
        if (now_ns - e->last_request_ns >= RUSTERON_DPDK_ARP_FAILED_RETRY_NS)
        {
            e->last_request_ns = now_ns;
            rusteron_dpdk_arp_send_request(runtime, port, next_hop_ip);
        }
        if (!e->reported)
        {
            e->reported = 1;
            return -1;
        }
        return RUSTERON_DPDK_ARP_DROP;
    }

    /* Empty (fresh insert) or still unresolved: the first resolution. */
    if (RUSTERON_DPDK_ARP_EMPTY == e->state)
    {
        e->state = RUSTERON_DPDK_ARP_INCOMPLETE;
        e->ip = next_hop_ip;
        e->since_ns = now_ns;
        e->last_request_ns = 0;
    }
    else if (now_ns - e->since_ns >= RUSTERON_DPDK_ARP_FAIL_NS)
    {
        e->state = RUSTERON_DPDK_ARP_FAILED;
        e->reported = 1;
        return -1;
    }
    if (0 == e->last_request_ns || now_ns - e->last_request_ns >= RUSTERON_DPDK_ARP_RETRY_NS)
    {
        e->last_request_ns = now_ns;
        rusteron_dpdk_arp_send_request(runtime, port, next_hop_ip);
    }
    return 0;
}

int rusteron_dpdk_arp_seed(
    rusteron_dpdk_arp_table_t *table, uint32_t ip, const uint8_t mac[RUSTERON_DPDK_ETH_ADDR_LEN], uint64_t now_ns)
{
    rusteron_dpdk_arp_entry_t *e = rusteron_dpdk_arp_slot_for(table, ip);
    if (NULL == e)
    {
        return -1;
    }
    memset(e, 0, sizeof(*e));
    e->ip = ip;
    memcpy(e->mac, mac, RUSTERON_DPDK_ETH_ADDR_LEN);
    e->state = RUSTERON_DPDK_ARP_REACHABLE;
    e->since_ns = now_ns;
    e->last_used = (uint32_t)(now_ns >> RUSTERON_DPDK_ARP_USE_SHIFT);
    return 0;
}

void rusteron_dpdk_arp_observe(
    rusteron_dpdk_arp_table_t *table, uint32_t src_ip, const uint8_t src_mac[RUSTERON_DPDK_ETH_ADDR_LEN])
{
    rusteron_dpdk_arp_entry_t *e = rusteron_dpdk_arp_slot_for(table, src_ip);
    if (NULL != e && RUSTERON_DPDK_ARP_REACHABLE == e->state && e->ip == src_ip &&
        0 != memcmp(e->mac, src_mac, RUSTERON_DPDK_ETH_ADDR_LEN))
    {
        e->stale = 1; /* the next resolve requests a refresh */
    }
}

/* Read a 4-byte network-order address from the frame. */
static uint32_t rusteron_dpdk_frame_read_ip(const uint8_t *p)
{
    uint32_t v;
    memcpy(&v, p, 4);
    return v;
}

static uint16_t rusteron_dpdk_frame_read_u16(const uint8_t *p)
{
    return (uint16_t)((p[0] << 8) | p[1]);
}

int rusteron_dpdk_arp_handle_frame(
    rusteron_dpdk_arp_table_t *table,
    rusteron_dpdk_transport_t *runtime,
    rusteron_dpdk_port_t *rx_port,
    const uint8_t *frame, size_t frame_len)
{
    /* Ethernet + ARP = 42 bytes minimum. */
    if (NULL == frame || frame_len < RUSTERON_DPDK_ETH_HDR_LEN + 28)
    {
        return 0;
    }

    /* EtherType 0x0806 (ARP). */
    if (frame[12] != 0x08 || frame[13] != 0x06)
    {
        return 0;
    }

    const uint8_t *a = frame + RUSTERON_DPDK_ETH_HDR_LEN;
    if (rusteron_dpdk_frame_read_u16(a + 0) != RUSTERON_DPDK_ARP_HTYPE_ETHERNET ||
        rusteron_dpdk_frame_read_u16(a + 2) != RUSTERON_DPDK_ARP_PTYPE_IPV4 ||
        a[4] != RUSTERON_DPDK_ARP_HLEN ||
        a[5] != RUSTERON_DPDK_ARP_PLEN)
    {
        return 0;
    }

    uint16_t oper = rusteron_dpdk_frame_read_u16(a + 6);
    const uint8_t *sha = a + 8;               /* sender hardware address */
    uint32_t spa = rusteron_dpdk_frame_read_ip(a + 14); /* sender protocol */
    uint32_t tpa = rusteron_dpdk_frame_read_ip(a + 24); /* target protocol */

    if (RUSTERON_DPDK_ARP_OPER_REQUEST == oper)
    {
        /* Respond to requests for any of this NIC's addresses, its extra
         * ones included, as their owner. */
        if (rusteron_dpdk_port_owns_ip(rx_port, tpa))
        {
            /* At most RUSTERON_DPDK_ARP_REPLIES_PER_SEC: a flood of requests
             * must not turn into a transmit per frame on the agent. */
            const uint64_t now_ns = rusteron_dpdk_clock_now();
            if (now_ns - rx_port->arp_reply_window_ns >= 1000000000ULL)
            {
                rx_port->arp_reply_window_ns = now_ns;
                rx_port->arp_replies_in_window = 0;
            }
            if (rx_port->arp_replies_in_window >= RUSTERON_DPDK_ARP_REPLIES_PER_SEC)
            {
                return 0;
            }
            rx_port->arp_replies_in_window++;
            rusteron_dpdk_arp_send_reply(runtime, rx_port, sha, spa, tpa);
            return 1;
        }
        return 0;
    }

    if (RUSTERON_DPDK_ARP_OPER_REPLY == oper)
    {
        /* Learn only when the reply is addressed to this role's local MAC and
         * answers an outstanding request: a first resolution (INCOMPLETE) or a
         * refresh. Anything else, gratuitous or unrelated ARP included, is
         * ignored, so it cannot poison a live next hop. */
        if (0 == memcmp(frame, rx_port->mac, RUSTERON_DPDK_ETH_ADDR_LEN))
        {
            rusteron_dpdk_arp_entry_t *e = rusteron_dpdk_arp_slot_for(table, spa);
            if (NULL != e && e->ip == spa &&
                (RUSTERON_DPDK_ARP_INCOMPLETE == e->state || RUSTERON_DPDK_ARP_FAILED == e->state ||
                 (RUSTERON_DPDK_ARP_REACHABLE == e->state && e->refreshing)))
            {
                memcpy(e->mac, sha, RUSTERON_DPDK_ETH_ADDR_LEN);
                e->state = RUSTERON_DPDK_ARP_REACHABLE;
                e->refreshing = 0;
                e->stale = 0;
                e->reported = 0;
                e->since_ns = rusteron_dpdk_clock_now();
                return 1;
            }
        }
        return 0;
    }

    return 0;
}
