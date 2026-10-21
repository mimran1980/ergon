/*
 * ergon-md-dpdk DPDK transport — ARP table (plan §7.5).
 *
 * A fixed 1024-entry open-addressed next-hop cache shared by both role ENAs.
 * Entries move EMPTY -> INCOMPLETE (request sent) -> REACHABLE (reply learned).
 * Only the first
 * resolution of a next hop waits (a zero send result Aeron retries; requests
 * at most one per 100 ms). Once learned, a next hop never leaves the send
 * path: after 30 s a refresh request goes out alongside the traffic, retried
 * at most once a second, while sends keep using the cached MAC — no pause,
 * ever, and an unanswered refresh keeps the MAC. A refresh's answer may move
 * the next hop to a new MAC (addresses change under Kubernetes).
 *
 * Learning is strict: only replies addressed to a role's local MAC/IP and
 * answering an outstanding request (a first resolution or a refresh) update
 * the table, so gratuitous/unrelated ARP cannot poison a live next hop.
 */
#ifndef RUSTERON_DPDK_ARP_H
#define RUSTERON_DPDK_ARP_H

#include <stddef.h>
#include <stdint.h>

#include "rusteron_dpdk_packet.h"

#ifdef __cplusplus
extern "C" {
#endif

#define RUSTERON_DPDK_ARP_TABLE_SIZE 1024
#define RUSTERON_DPDK_ARP_RETRY_NS 100000000ULL    /* 100 ms */
#define RUSTERON_DPDK_ARP_REFRESH_NS 30000000000ULL      /* 30 s: refresh in the background */
#define RUSTERON_DPDK_ARP_REFRESH_RETRY_NS 1000000000ULL /* 1 s between unanswered refreshes */
#define RUSTERON_DPDK_ARP_FAIL_NS 3000000000ULL          /* first resolution: 3 s, then FAILED */
#define RUSTERON_DPDK_ARP_FAILED_RETRY_NS 1000000000ULL  /* FAILED: re-probe once a second */
#define RUSTERON_DPDK_ARP_DROP 2                          /* resolve: failed or full, already reported */
#define RUSTERON_DPDK_ARP_FULL (-2)                       /* resolve: no room for a new next hop (reported once) */
#define RUSTERON_DPDK_ARP_MAX_PROBE 16                    /* slots a lookup or insert examines */
#define RUSTERON_DPDK_ARP_EVICT_NS 60000000000ULL         /* a next hop unused this long may be replaced */
#define RUSTERON_DPDK_ARP_REPLIES_PER_SEC 1000                 /* answers to requests, per NIC */
#define RUSTERON_DPDK_ARP_USE_SHIFT 24                    /* last_used units: 2^24 ns (~16.8 ms) */

/* Opaque structs defined in rusteron_dpdk_internal.h; only the pointers are
 * needed by the arp.h declarations (arp.c includes internal.h). */
typedef struct rusteron_dpdk_transport_stct rusteron_dpdk_transport_t;
typedef struct rusteron_dpdk_port_stct rusteron_dpdk_port_t;

typedef enum rusteron_dpdk_arp_state_en
{
    RUSTERON_DPDK_ARP_EMPTY = 0,
    RUSTERON_DPDK_ARP_INCOMPLETE,
    RUSTERON_DPDK_ARP_REACHABLE,
    RUSTERON_DPDK_ARP_FAILED
} rusteron_dpdk_arp_state_t;

/* 32 bytes: two per cache line, so a probe touches few lines. */
typedef struct rusteron_dpdk_arp_entry_stct
{
    uint32_t ip;                 /* next-hop IPv4, network order */
    uint8_t mac[RUSTERON_DPDK_ETH_ADDR_LEN];
    uint8_t state;               /* rusteron_dpdk_arp_state_t */
    uint8_t refreshing : 1;      /* a refresh request is outstanding */
    uint8_t stale : 1;           /* the peer was seen at another MAC: refresh now */
    uint8_t reported : 1;        /* this FAILED episode was reported to a sender */
    uint32_t last_used;          /* clock >> RUSTERON_DPDK_ARP_USE_SHIFT at the last resolve */
    uint64_t last_request_ns;    /* clock ns of the most recent request */
    uint64_t since_ns;           /* REACHABLE: learned/refreshed; INCOMPLETE: first request */
} rusteron_dpdk_arp_entry_t;

typedef struct rusteron_dpdk_arp_table_stct
{
    rusteron_dpdk_arp_entry_t entries[RUSTERON_DPDK_ARP_TABLE_SIZE];
    int full_reported;           /* a next hop found no room and was reported... */
    uint64_t full_report_ns;     /* ...at this time: again at most once a minute */
} rusteron_dpdk_arp_table_t;

/*
 * Resolve a next hop. Returns 1 and copies the MAC for any learned (REACHABLE)
 * entry, starting a background refresh when a learned one is 30 s old. An
 * unresolved next hop returns 0 while its first resolution runs (the entry
 * INCOMPLETE, requests 100 ms apart; the caller reports a retryable zero-send
 * result). Once it has gone unanswered for 3 s it is FAILED, as Linux's
 * NUD_FAILED: the first send after that returns -1 (the caller reports
 * EHOSTUNREACH, once per failure episode, as a socket's pending error), later
 * ones RUSTERON_DPDK_ARP_DROP (the caller drops the datagrams silently, as an
 * unconnected sendto succeeds). A FAILED entry is re-probed once a second and
 * recovers on any answer. Lookups examine at most RUSTERON_DPDK_ARP_MAX_PROBE
 * slots; a new next hop finding none free replaces the least recently used
 * entry there idle for RUSTERON_DPDK_ARP_EVICT_NS (a next hop in use is never
 * replaced), else gets RUSTERON_DPDK_ARP_FULL (at most once a minute per
 * table) or RUSTERON_DPDK_ARP_DROP.
 * `now_ns` is the caller's rusteron_dpdk_clock_now() read.
 */
int rusteron_dpdk_arp_resolve(
    rusteron_dpdk_arp_table_t *table,
    rusteron_dpdk_transport_t *runtime,
    rusteron_dpdk_port_t *port,
    uint32_t next_hop_ip,
    uint64_t now_ns,
    uint8_t out_mac[RUSTERON_DPDK_ETH_ADDR_LEN]);

/*
 * Handle an incoming ARP frame (request or reply). Replies to requests for a
 * role's local IPv4 are sent; replies addressed to a role's local MAC/IP that
 * match an outstanding request are learned. Returns 1 when the frame was
 * consumed (a reply was sent or a request learned), 0 when ignored.
 */
/*
 * A received frame came from `src_ip` at `src_mac`. When `src_ip` is a learned
 * next hop cached at a different MAC (the peer moved: a pod rescheduled, a
 * failover), its refresh falls due now: the next send to it requests one,
 * still sending to the cached MAC meanwhile. Only a solicited reply changes
 * the MAC, so a forged frame can at most cause a refresh.
 */
void rusteron_dpdk_arp_observe(
    rusteron_dpdk_arp_table_t *table, uint32_t src_ip, const uint8_t src_mac[RUSTERON_DPDK_ETH_ADDR_LEN]);

/* Install a learned (REACHABLE) entry directly: tests. -1 when full. */
int rusteron_dpdk_arp_seed(
    rusteron_dpdk_arp_table_t *table, uint32_t ip, const uint8_t mac[RUSTERON_DPDK_ETH_ADDR_LEN], uint64_t now_ns);

int rusteron_dpdk_arp_handle_frame(
    rusteron_dpdk_arp_table_t *table,
    rusteron_dpdk_transport_t *runtime,
    rusteron_dpdk_port_t *rx_port,
    const uint8_t *frame, size_t frame_len);

#ifdef __cplusplus
}
#endif

#endif /* RUSTERON_DPDK_ARP_H */
