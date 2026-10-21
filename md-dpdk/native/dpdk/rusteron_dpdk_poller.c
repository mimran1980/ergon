/*
 * ergon-md-dpdk DPDK transport — receive polling and endpoint dispatch
 * (plan §7.6/§7.7).
 *
 * The poller reuses the vendored aeron_udp_transport_poller_t layout: the
 * `transports` array tracks the registered Aeron transports and `bindings_clientd`
 * owns a fixed 4096-entry endpoint map keyed by (destination IPv4, UDP port).
 * poller_add/remove mutate the map outside the hot path; poller_poll drains each
 * distinct role port once per call and dispatches every valid frame to the
 * registered transport via Aeron's recv_func, exactly as a socket recvmmsg
 * would: UDP payload + reconstructed source sockaddr, mbuf recycled immediately
 * after the callback returns.
 *
 * The hot path (poller_poll -> rusteron_dpdk_poller_receive -> classify ->
 * dispatch) performs no heap allocation (mbuf acquisition/recycling excepted),
 * no locking, no logging, no string construction and no syscalls.
 */
/* The poller writes the received-datagram metadata back into the caller's
 * mmsghdr (msg_name, msg_len) exactly as the kernel recvmmsg path does. The
 * complete `struct mmsghdr` is a GNU extension (glibc bits/mmsghdr.h) gated on
 * _GNU_SOURCE; the vendored bindings only forward-declare the tag. This must
 * be defined before any system header is included. */
#define _GNU_SOURCE

#include "rusteron_dpdk_internal.h"

#include <arpa/inet.h>
#include <errno.h>
#include <string.h>
#include <sys/socket.h> /* struct mmsghdr (see above) */
#include <time.h>       /* struct timespec: media receive timestamps */

#include "media/aeron_udp_transport_poller.h" /* full poller struct + entry type */
#include "aeron_alloc.h"

#define RUSTERON_DPDK_POLLER_MAX_PORTS 4

/* A NIC this poller drains, with how many of its transports are registered
 * and one of them to drain it through. */
typedef struct rusteron_dpdk_poller_port_stct
{
    rusteron_dpdk_client_t *client;
    size_t refs;
} rusteron_dpdk_poller_port_t;

/* Private poller state anchored at poller->bindings_clientd. */
typedef struct rusteron_dpdk_poller_state_stct
{
    rusteron_dpdk_endpoint_map_t endpoints;
    /* The distinct NICs, so a poll touches only them, not every transport. */
    rusteron_dpdk_poller_port_t ports[RUSTERON_DPDK_POLLER_MAX_PORTS];
    size_t port_count;
    /* Transports with a kernel socket (host-local channels, a DPDK
     * transport's host-local destinations, a wildcard bind's local listener),
     * read only when the local-I/O thread has flagged data waiting. */
    aeron_udp_channel_transport_t **kernel;
    size_t kernel_count;
    size_t kernel_capacity;
} rusteron_dpdk_poller_state_t;

/* Room for `needed` kernel sockets; the list is kept as is on failure. */
static int rusteron_dpdk_poller_kernel_reserve(rusteron_dpdk_poller_state_t *state, size_t needed)
{
    if (needed <= state->kernel_capacity)
    {
        return 0;
    }
    size_t capacity = 0 == state->kernel_capacity ? 8 : state->kernel_capacity;
    while (capacity < needed)
    {
        capacity *= 2;
    }
    void *array = realloc(state->kernel, capacity * sizeof(state->kernel[0]));
    if (NULL == array)
    {
        rusteron_dpdk_set_error("poller: out of memory growing the host-local list");
        return -1;
    }
    state->kernel = array;
    state->kernel_capacity = capacity;
    return 0;
}

int rusteron_dpdk_poller_kernel_add(rusteron_dpdk_poller_state_t *state, aeron_udp_channel_transport_t *transport)
{
    for (size_t i = 0; i < state->kernel_count; i++)
    {
        if (state->kernel[i] == transport)
        {
            return 0;
        }
    }
    if (rusteron_dpdk_poller_kernel_reserve(state, state->kernel_count + 1) < 0)
    {
        return -1;
    }
    state->kernel[state->kernel_count++] = transport;
    return 0;
}

static void rusteron_dpdk_poller_kernel_remove(rusteron_dpdk_poller_state_t *state, aeron_udp_channel_transport_t *transport)
{
    for (size_t i = 0; i < state->kernel_count; i++)
    {
        if (state->kernel[i] == transport)
        {
            state->kernel[i] = state->kernel[--state->kernel_count];
            return;
        }
    }
}

int rusteron_dpdk_poller_receive_registered(
    rusteron_dpdk_client_t *client,
    struct mmsghdr *msgvec,
    size_t vlen,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    void *clientd)
{
    return rusteron_dpdk_poller_receive(
        client, NULL, &client->poller->endpoints, msgvec, vlen, bytes_rcved, recv_func, clientd);
}

/* The endpoint-map keys of a DPDK transport: its bound address, or for a
 * wildcard bind every address of its NIC. Registered all-or-nothing. */
static int rusteron_dpdk_endpoints_put(
    rusteron_dpdk_poller_state_t *state, rusteron_dpdk_client_t *client, aeron_udp_channel_transport_t *transport)
{
    const rusteron_dpdk_port_t *port = client->port;
    if (0 != client->bound_ip)
    {
        return rusteron_dpdk_endpoint_map_put(&state->endpoints, client->bound_ip, client->local_udp_port, transport);
    }
    int rc = rusteron_dpdk_endpoint_map_put(&state->endpoints, port->local_ip, client->local_udp_port, transport);
    if (rc < 0)
    {
        return rc;
    }
    for (uint32_t x = 0; x < port->extra_ip_count; x++)
    {
        rc = rusteron_dpdk_endpoint_map_put(&state->endpoints, port->extra_ips[x], client->local_udp_port, transport);
        if (rc < 0)
        {
            for (uint32_t y = 0; y < x; y++)
            {
                rusteron_dpdk_endpoint_map_remove(&state->endpoints, port->extra_ips[y], client->local_udp_port);
            }
            rusteron_dpdk_endpoint_map_remove(&state->endpoints, port->local_ip, client->local_udp_port);
            return rc;
        }
    }
    return 0;
}

static void rusteron_dpdk_endpoints_remove(rusteron_dpdk_poller_state_t *state, const rusteron_dpdk_client_t *client)
{
    const rusteron_dpdk_port_t *port = client->port;
    if (0 != client->bound_ip)
    {
        rusteron_dpdk_endpoint_map_remove(&state->endpoints, client->bound_ip, client->local_udp_port);
        return;
    }
    rusteron_dpdk_endpoint_map_remove(&state->endpoints, port->local_ip, client->local_udp_port);
    for (uint32_t x = 0; x < port->extra_ip_count; x++)
    {
        rusteron_dpdk_endpoint_map_remove(&state->endpoints, port->extra_ips[x], client->local_udp_port);
    }
}

/* Whether the transport's bind covers `ip` (network order). */
static int rusteron_dpdk_client_owns_ip(const rusteron_dpdk_client_t *client, uint32_t ip)
{
    return 0 != client->bound_ip ? ip == client->bound_ip : rusteron_dpdk_port_owns_ip(client->port, ip);
}

/* A wildcard transport replies to each peer from the address the peer reached
 * it on (the kernel's source choice): remember the pairing. */
/* Remember which local address a peer reached a wildcard transport on, so
 * replies leave from it, as the kernel picks a reply's source. Kept current
 * every datagram (written only on change); a hash collision keeps the
 * latest peer, the other falls back to the NIC's address. */
static void rusteron_dpdk_client_note_reply_ip(rusteron_dpdk_client_t *client, uint32_t peer_ip, uint32_t local_ip)
{
    rusteron_dpdk_reply_ip_t *r = &client->reply[rusteron_dpdk_reply_slot(peer_ip)];
    if (r->peer_ip != peer_ip || r->local_ip != local_ip)
    {
        r->peer_ip = peer_ip;
        r->local_ip = local_ip;
    }
}

/* Aeron's distinct error log, from the driver library when it is linked
 * (production); absent from the DPDK-free test archives. */
extern int aeron_distinct_error_log_record(aeron_distinct_error_log_t *log, int error_code, const char *description)
    __attribute__((weak));

#define RUSTERON_DPDK_AERON_HDR_TYPE_SETUP 0x05
#define RUSTERON_DPDK_ICMP_UNREACH_PER_SEC 100
#define RUSTERON_DPDK_ICMP_ECHO_PER_SEC 100

static uint32_t rusteron_dpdk_le32(const uint8_t *p)
{
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

/* An Aeron SETUP frame (type at offset 6, mtu at 32, little-endian) whose
 * MTU exceeds this transport's max_aeron_mtu. */
static int rusteron_dpdk_setup_mtu_too_large(const rusteron_dpdk_transport_t *native, const rusteron_dpdk_parsed_frame_t *f)
{
    if (f->payload_len < 36 || RUSTERON_DPDK_AERON_HDR_TYPE_SETUP != (f->payload[6] | (f->payload[7] << 8)))
    {
        return 0;
    }
    return rusteron_dpdk_le32(f->payload + 32) > native->config.max_aeron_mtu;
}

/* At most one report a second per NIC: spoofed frames must not cost string
 * formatting per packet or fill the driver's distinct error log. */
static int rusteron_dpdk_report_due(rusteron_dpdk_port_t *port)
{
    const uint64_t now_ns = rusteron_dpdk_clock_now();
    if (0 != port->last_report_ns && now_ns - port->last_report_ns < 1000000000ULL)
    {
        return 0;
    }
    port->last_report_ns = now_ns;
    return 1;
}

static void rusteron_dpdk_record_peer_mtu(
    aeron_udp_channel_transport_t *target, const rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *port,
    const rusteron_dpdk_parsed_frame_t *f)
{
    if (!rusteron_dpdk_report_due(port))
    {
        return;
    }
    char ip_text[INET_ADDRSTRLEN] = "";
    char message[256];
    inet_ntop(AF_INET, &f->src_ip, ip_text, sizeof(ip_text));
    snprintf(message, sizeof(message),
             "DPDK transport: SETUP from %s:%u has mtu %u, above max_aeron_mtu %zu: dropped; every peer's Aeron "
             "mtu must be <= max_aeron_mtu",
             ip_text, (unsigned)f->src_port, rusteron_dpdk_le32(f->payload + 32), native->config.max_aeron_mtu);
    rusteron_dpdk_set_error(message);
    if (NULL != aeron_distinct_error_log_record && NULL != target->error_log)
    {
        /* A fixed description: the distinct log keeps one entry, not one per
         * spoofed source. The details are in the counter and last_error. */
        aeron_distinct_error_log_record(
            target->error_log, EMSGSIZE,
            "DPDK transport: SETUP from a peer whose mtu exceeds max_aeron_mtu dropped (counter peer-mtu-too-large)");
    }
}

/* Transmit one locally built frame (ICMP) on the port. */
static void rusteron_dpdk_tx_one(rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *port, rusteron_dpdk_mbuf_t *m)
{
    rusteron_dpdk_mbuf_t *burst[1] = { m };
    rusteron_dpdk_port_tx(native, port, burst, 1);
}

/* ICMP port unreachable for a datagram to a port nobody listens on, at most
 * RUSTERON_DPDK_ICMP_UNREACH_PER_SEC per NIC, quoting its IP header + 8. */
static void rusteron_dpdk_icmp_port_unreachable(
    rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *port, const rusteron_dpdk_mbuf_t *orig)
{
    const uint64_t now_ns = rusteron_dpdk_clock_now();
    if (now_ns - port->icmp_window_ns >= 1000000000ULL)
    {
        port->icmp_window_ns = now_ns;
        port->icmp_sent_in_window = 0;
    }
    if (port->icmp_sent_in_window >= RUSTERON_DPDK_ICMP_UNREACH_PER_SEC || NULL == orig)
    {
        return;
    }
    const uint8_t *ip = orig->data + RUSTERON_DPDK_ETH_HDR_LEN;
    const size_t ihl = (size_t)(ip[0] & 0x0F) * 4;
    size_t quote = ihl + 8;
    if (RUSTERON_DPDK_ETH_HDR_LEN + quote > orig->frame_len)
    {
        return;
    }
    uint32_t src_ip, dst_ip;
    memcpy(&src_ip, ip + 12, 4);
    memcpy(&dst_ip, ip + 16, 4);
    rusteron_dpdk_mbuf_t m;
    if (native->ops->mbuf_alloc(port->mempool, &m) < 0)
    {
        return;
    }
    static const uint8_t unused[4] = { 0, 0, 0, 0 };
    if (rusteron_dpdk_packet_build_icmp(&m, orig->data + RUSTERON_DPDK_ETH_ADDR_LEN, port->mac, dst_ip, src_ip, 3, 3,
                                        unused, ip, quote) < 0)
    {
        native->ops->mbuf_release(&m);
        return;
    }
    port->icmp_sent_in_window++;
    rusteron_dpdk_tx_one(native, port, &m);
    rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_ICMP_UNREACH_SENT, 1);
}

/* ICMP addressed to this NIC: answer echo, as the kernel would (so ping and
 * health checks work); record fragmentation-needed (a path MTU below ours
 * black-holes our DF frames); count the rest. */
static void rusteron_dpdk_icmp_handle(
    rusteron_dpdk_transport_t *native, rusteron_dpdk_port_t *port, const rusteron_dpdk_mbuf_t *orig,
    const rusteron_dpdk_parsed_frame_t *f)
{
    const uint8_t *icmp = f->payload;
    const uint8_t type = icmp[0], code = icmp[1];
    if (8 == type && 0 == code)
    {
        /* Echo: at most RUSTERON_DPDK_ICMP_ECHO_PER_SEC replies a second, small
         * payloads only, so a flood cannot take the agent or the TX queue. */
        const uint64_t now_ns = rusteron_dpdk_clock_now();
        if (now_ns - port->icmp_echo_window_ns >= 1000000000ULL)
        {
            port->icmp_echo_window_ns = now_ns;
            port->icmp_echo_sent = 0;
        }
        if (port->icmp_echo_sent >= RUSTERON_DPDK_ICMP_ECHO_PER_SEC || f->payload_len > 8 + 512 ||
            0 != rusteron_dpdk_inet_csum(icmp, f->payload_len))
        {
            rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_ICMP_OTHER, 1);
            return;
        }
        rusteron_dpdk_mbuf_t m;
        if (native->ops->mbuf_alloc(port->mempool, &m) < 0)
        {
            return;
        }
        if (rusteron_dpdk_packet_build_icmp(&m, orig->data + RUSTERON_DPDK_ETH_ADDR_LEN, port->mac, f->dst_ip, f->src_ip,
                                            0, 0, icmp + 4, icmp + 8, f->payload_len - 8) < 0)
        {
            native->ops->mbuf_release(&m);
            return;
        }
        port->icmp_echo_sent++;
        rusteron_dpdk_tx_one(native, port, &m);
        rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_ICMP_ECHO, 1);
    }
    else if (3 == type && 4 == code)
    {
        rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_ICMP_FRAG_NEEDED, 1);
        if (rusteron_dpdk_report_due(port))
        {
            char ip_text[INET_ADDRSTRLEN] = "";
            char message[256];
            inet_ntop(AF_INET, &f->src_ip, ip_text, sizeof(ip_text));
            snprintf(message, sizeof(message),
                     "DPDK transport: %s reports fragmentation needed, next-hop MTU %u: a path MTU below "
                     "max_aeron_mtu + 28 drops this transport's (DF) frames",
                     ip_text, (unsigned)((icmp[6] << 8) | icmp[7]));
            rusteron_dpdk_set_error(message);
        }
    }
    else
    {
        rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_ICMP_OTHER, 1);
    }
}

/* Reject-class bookkeeping: the rx_stats bucket (existing §7.6 accounting) plus
 * the matching Aeron counter (plan §9). */
static void rusteron_dpdk_rx_count(
    rusteron_dpdk_rx_stats_t *stats, rusteron_dpdk_counters_t *counters, rusteron_dpdk_rx_result_t result)
{
    switch (result)
    {
        case RUSTERON_DPDK_RX_RESULT_IPV6:
            stats->ipv6++;
            rusteron_dpdk_counters_add(counters, RD_COUNTER_DISCARD, 1);
            break;
        case RUSTERON_DPDK_RX_RESULT_MULTICAST:
            stats->multicast++;
            rusteron_dpdk_counters_add(counters, RD_COUNTER_DISCARD, 1);
            break;
        case RUSTERON_DPDK_RX_RESULT_ETHERTYPE:
            stats->ethertype++;
            rusteron_dpdk_counters_add(counters, RD_COUNTER_UNSUPPORTED_ETHERTYPE, 1);
            break;
        case RUSTERON_DPDK_RX_RESULT_VLAN:
            stats->vlan++;
            rusteron_dpdk_counters_add(counters, RD_COUNTER_DISCARD, 1);
            break;
        case RUSTERON_DPDK_RX_RESULT_IP_OPTIONS:
            stats->ip_options++;
            rusteron_dpdk_counters_add(counters, RD_COUNTER_DISCARD, 1);
            break;
        case RUSTERON_DPDK_RX_RESULT_FRAGMENT:
            stats->fragment++;
            rusteron_dpdk_counters_add(counters, RD_COUNTER_FRAGMENTED, 1);
            break;
        case RUSTERON_DPDK_RX_RESULT_TRUNCATED:
            stats->truncated++;
            rusteron_dpdk_counters_add(counters, RD_COUNTER_DISCARD, 1);
            break;
        case RUSTERON_DPDK_RX_RESULT_PROTOCOL:
            stats->protocol++;
            rusteron_dpdk_counters_add(counters, RD_COUNTER_UNSUPPORTED_PROTOCOL, 1);
            break;
        case RUSTERON_DPDK_RX_RESULT_CHECKSUM:
            stats->checksum++;
            rusteron_dpdk_counters_add(counters, RD_COUNTER_CHECKSUM, 1);
            break;
        case RUSTERON_DPDK_RX_RESULT_MULTI_SEGMENT:
            stats->multi_segment++;
            rusteron_dpdk_counters_add(counters, RD_COUNTER_MULTI_SEGMENT, 1);
            break;
        default:
            break;
    }
}

/* Dispatch one parsed frame. The target transport is chosen by the endpoint map
 * (endpoints != NULL) or matched against only_transport's own local endpoint.
 * Rejected frames are counted and dropped; accepted frames invoke recv_func
 * with the UDP payload, a reconstructed source sockaddr_in and the media
 * receive timestamp (NULL when no channel on the NIC asks for one; Aeron's
 * image writes it only into a channel that does) and return 1. */
static int rusteron_dpdk_dispatch_frame(
    rusteron_dpdk_client_t *client,
    aeron_udp_channel_transport_t *only_transport,
    const rusteron_dpdk_endpoint_map_t *endpoints,
    const rusteron_dpdk_mbuf_t *m,
    const rusteron_dpdk_parsed_frame_t *frame,
    struct mmsghdr *msgvec,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    void *clientd,
    struct timespec *media_timestamp)
{
    aeron_udp_channel_transport_t *target = NULL;

    if (NULL != endpoints)
    {
        target = (aeron_udp_channel_transport_t *)rusteron_dpdk_endpoint_map_get(
            endpoints, frame->dst_ip, frame->dst_port);
    }
    else if (rusteron_dpdk_client_owns_ip(client, frame->dst_ip) && frame->dst_port == client->local_udp_port)
    {
        target = only_transport;
    }

    if (NULL == target)
    {
        rusteron_dpdk_counters_t *counters = &client->port->counters;
        if (!rusteron_dpdk_port_owns_ip(client->port, frame->dst_ip))
        {
            client->port->rx_stats.foreign_dst++;
            rusteron_dpdk_counters_add(counters, RD_COUNTER_DISCARD, 1);
        }
        else
        {
            client->port->rx_stats.unknown_port++;
            rusteron_dpdk_counters_add(counters, RD_COUNTER_DISCARD, 1);
            rusteron_dpdk_counters_add(counters, RD_COUNTER_QUEUE_DROP, 1);
            /* As the kernel: tell the sender nobody listens (rate-limited). */
            rusteron_dpdk_icmp_port_unreachable(client->runtime, client->port, m);
        }
        return 0;
    }

    /* A connected transport, like a connect()ed socket, hears only its peer. */
    rusteron_dpdk_client_t *tclient = target->bindings_clientd;
    if (NULL != tclient && tclient->connected &&
        (frame->src_ip != tclient->connected_ip || frame->src_port != tclient->connected_port))
    {
        client->port->rx_stats.not_connected_peer++;
        rusteron_dpdk_counters_add(&client->port->counters, RD_COUNTER_NOT_CONNECTED_PEER, 1);
        return 0;
    }

    /* A peer whose MTU exceeds ours would have its data frames dropped on our
     * NIC while its small SETUP and heartbeats got through, stalling the image
     * in NAK loops: drop the SETUP instead, so the publication shows as not
     * connected, and say why (at most once a second). */
    if (rusteron_dpdk_setup_mtu_too_large(client->runtime, frame))
    {
        rusteron_dpdk_counters_add(&client->port->counters, RD_COUNTER_PEER_MTU_TOO_LARGE, 1);
        rusteron_dpdk_record_peer_mtu(target, client->runtime, client->port, frame);
        return 0;
    }

    if (NULL != tclient && 0 == tclient->bound_ip && 0 != client->port->extra_ip_count)
    {
        rusteron_dpdk_client_note_reply_ip(tclient, frame->src_ip, frame->dst_ip);
    }

    /* Reconstruct the source sockaddr the kernel would have returned from a
     * recvmsg on the matching UDP socket (plan §7.6). */
    struct sockaddr_storage *addr = (struct sockaddr_storage *)msgvec[0].msg_hdr.msg_name;
    struct sockaddr_storage local_addr;
    if (NULL == addr)
    {
        addr = &local_addr;
    }
    memset(addr, 0, sizeof(struct sockaddr_in));
    struct sockaddr_in *sin = (struct sockaddr_in *)addr;
    sin->sin_family = AF_INET;
    sin->sin_port = htons(frame->src_port);
    sin->sin_addr.s_addr = frame->src_ip;
    msgvec[0].msg_len = (unsigned int)frame->payload_len;

    recv_func(
        target->data_paths,
        target,
        clientd,
        target->dispatch_clientd,
        target->destination_clientd,
        (uint8_t *)frame->payload,
        frame->payload_len,
        addr,
        media_timestamp);

    *bytes_rcved += (int64_t)frame->payload_len;
    client->port->rx_stats.accepted++;
    return 1;
}

/* Counts a gap of 20 µs or more since the agent's last cycle on `port`
 * (poll-cycle-gap-*); the caller sets last_poll_ns. With attribution on it
 * also splits the gap: the thread's own CPU time across it, half or more of
 * the wall length being the agent working between polls and less being the
 * host descheduling its vCPU (RUSTERON_DPDK_GAP_ATTRIB=1; `now_cpu_ns` is 0
 * and last_poll_cpu_ns stays 0 otherwise). */
static inline void rusteron_dpdk_poll_cycle(rusteron_dpdk_port_t *port, uint64_t now_ns, uint64_t now_cpu_ns)
{
    if (0 != port->last_poll_ns && now_ns - port->last_poll_ns >= 20000)
    {
        const uint64_t cycle = now_ns - port->last_poll_ns;
        rusteron_dpdk_counters_max(&port->counters, RD_COUNTER_POLL_CYCLE_GAP_MAX, (int64_t)cycle);
        rusteron_dpdk_counters_add(
            &port->counters,
            cycle >= 100000  ? RD_COUNTER_POLL_CYCLE_GAPS_100US
            : cycle >= 50000 ? RD_COUNTER_POLL_CYCLE_GAPS_50US
                             : RD_COUNTER_POLL_CYCLE_GAPS_20US,
            1);
        if (0 != port->last_poll_cpu_ns)
        {
            const uint64_t cpu = now_cpu_ns - port->last_poll_cpu_ns;
            const int work = cpu >= cycle || cpu * 2 >= cycle;
            rusteron_dpdk_counters_add(
                &port->counters, work ? RD_COUNTER_POLL_CYCLE_GAPS_WORK : RD_COUNTER_POLL_CYCLE_GAPS_DESCHED, 1);
            rusteron_dpdk_counters_max(
                &port->counters,
                work ? RD_COUNTER_POLL_CYCLE_GAP_WORK_MAX : RD_COUNTER_POLL_CYCLE_GAP_DESCHED_MAX,
                (int64_t)cycle);
        }
    }
}

int rusteron_dpdk_poller_receive(
    rusteron_dpdk_client_t *client,
    aeron_udp_channel_transport_t *only_transport,
    const rusteron_dpdk_endpoint_map_t *endpoints,
    struct mmsghdr *msgvec,
    size_t vlen,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    void *clientd)
{
    if (NULL == client || NULL == client->runtime || NULL == client->port ||
        NULL == msgvec || NULL == bytes_rcved || NULL == recv_func)
    {
        return -1;
    }

    /* The Aeron receiver is a non-EAL thread; register it with DPDK before its
     * first rx (plan §7.2). The seam is a cheap lcore-id check after the first
     * call and is a no-op on already-registered/EAL threads. */
    if (rusteron_dpdk_eal_thread_register() < 0)
    {
        rusteron_dpdk_set_error("poller_receive: failed to register thread with DPDK");
        return -1;
    }

    rusteron_dpdk_transport_t *native = client->runtime;
    rusteron_dpdk_port_t *port = client->port;
    const rusteron_dpdk_port_ops_t *ops = native->ops;

    /* Up to the configured burst size a burst, whatever Aeron's vlen: every
     * frame is dispatched through msgvec[0] the moment it is parsed (zero
     * copy), so vlen sizes nothing here, and the sender's control poll
     * (vlen 1) would otherwise take one frame a duty cycle. */
    size_t limit = native->config.burst_size;
    if (0 == vlen || 0 == limit)
    {
        return 0;
    }
    if (limit > 256)
    {
        limit = 256;
    }

    rusteron_dpdk_mbuf_t views[256];
    rusteron_dpdk_mbuf_t *batch[256];
    for (size_t i = 0; i < limit; i++)
    {
        batch[i] = &views[i];
    }

    /* The per-frame tallies stay in locals and reach the shared counters once
     * per poll: one atomic add each, not several per frame. */
    int work_count = 0;
    int64_t rx_bytes = 0;
    int64_t sw_csum = 0;
    /* NIC arrival to the receive burst that carried the frame, over the frames
     * the NIC stamped no later than the host clock read at that burst; one
     * stamped later is clock skew between the NIC's PTP hardware clock and
     * CLOCK_REALTIME, counted apart. The read is once a burst and before any
     * frame of it is classified or dispatched, so a frame after the burst's
     * first is measured before its own dispatch: a lower bound on arrival to
     * dispatch, exact only for the first frame of the burst. Per-frame dispatch
     * stamps would cost a clock read per frame on the receive path; the burst
     * endpoint is the one this path can report without adding that. */
    int64_t hw_sum = 0, hw_count = 0, hw_max = INT64_MIN, hw_min = INT64_MAX, hw_negative = 0;
    int observed = 0; /* the first datagram's source checked against ARP */
    /* A channel on this NIC asking for media receive timestamps
     * (media-rcv-ts-offset): Aeron's kernel transport has the kernel stamp each
     * datagram (SO_TIMESTAMPNS, CLOCK_REALTIME at its receive); here each burst
     * reads CLOCK_REALTIME once, and a frame the NIC stamped
     * (RUSTERON_DPDK_RX_HW_TIMESTAMP=1) carries its arrival instead. Without
     * one, nothing is read and every frame passes NULL: one branch a burst. */
    const int media_ts = 0 != __atomic_load_n(&port->media_ts_transports, __ATOMIC_RELAXED);
    struct timespec burst_ts;
    const uint32_t on_link_mask = rusteron_dpdk_ipv4_mask(port->prefix_len);
    /* A full burst means the ring may hold more: keep draining, a burst at a
     * time up to rx_max_bursts, so a backlog a stall left clears in this duty
     * cycle instead of one burst per cycle while the ring keeps filling (on
     * MANA, overflowing without a count). */
    size_t total = 0;
    int full = 0;
    for (uint32_t burst = 0; burst < native->rx_max_bursts; burst++)
    {
        const uint16_t received = ops->rx_burst(port->port_id, port->rx_queue_id, batch, (uint16_t)limit);
        /* CLOCK_REALTIME, read at most once a burst, after the NIC handed the
         * burst over (every frame of it arrived before): at once for media
         * timestamps, else at the burst's first NIC-timestamped frame. Never
         * once a poll: a later burst's frames arrived after that read. */
        uint64_t realtime_ns = 0;
        struct timespec *burst_media_ts = NULL;
        if (media_ts && received > 0)
        {
            realtime_ns = rusteron_dpdk_realtime_ns();
            burst_ts.tv_sec = (time_t)(realtime_ns / 1000000000ULL);
            burst_ts.tv_nsec = (long)(realtime_ns % 1000000000ULL);
            burst_media_ts = &burst_ts;
        }
        for (uint16_t i = 0; i < received; i++)
        {
            rusteron_dpdk_mbuf_t *m = &views[i];
            rusteron_dpdk_parsed_frame_t frame;
            rusteron_dpdk_rx_result_t result = rusteron_dpdk_packet_classify_rx(m, &frame);

            if (RUSTERON_DPDK_RX_RESULT_OK == result)
            {
                sw_csum += frame.sw_checksum;
                if (!observed)
                {
                    /* Once per poll: has the peer's MAC changed under us? Only an
                     * on-link source is a next hop of its own; anything else
                     * arrives from the gateway. */
                    if (((frame.src_ip ^ port->local_ip) & on_link_mask) == 0)
                    {
                        rusteron_dpdk_arp_observe(&port->arp, frame.src_ip, m->data + RUSTERON_DPDK_ETH_ADDR_LEN);
                    }
                    observed = 1;
                }
                struct timespec *media_timestamp = burst_media_ts;
                struct timespec nic_ts;
                if (0 != m->hw_rx_ns)
                {
                    /* NIC arrival to this burst: the NIC stamps with its PTP
                     * hardware clock, which chrony keeps CLOCK_REALTIME on. */
                    if (0 == realtime_ns)
                    {
                        realtime_ns = rusteron_dpdk_realtime_ns();
                    }
                    const int64_t delay = (int64_t)(realtime_ns - m->hw_rx_ns);
                    if (delay < 0)
                    {
                        hw_negative++;
                    }
                    else
                    {
                        hw_sum += delay;
                        hw_count++;
                        hw_max = delay > hw_max ? delay : hw_max;
                        hw_min = delay < hw_min ? delay : hw_min;
                    }
                    /* The media timestamp is the NIC's arrival: the PHC counts
                     * UTC nanoseconds since the epoch, CLOCK_REALTIME's scale,
                     * what the kernel's SO_TIMESTAMPNS reports. */
                    if (NULL != media_timestamp)
                    {
                        nic_ts.tv_sec = (time_t)(m->hw_rx_ns / 1000000000ULL);
                        nic_ts.tv_nsec = (long)(m->hw_rx_ns % 1000000000ULL);
                        media_timestamp = &nic_ts;
                    }
                }
                const int dispatched = rusteron_dpdk_dispatch_frame(
                    client, only_transport, endpoints, m, &frame, msgvec, bytes_rcved, recv_func, clientd,
                    media_timestamp);
                work_count += dispatched;
                rx_bytes += dispatched ? (int64_t)frame.payload_len : 0;
            }
            else if (RUSTERON_DPDK_RX_RESULT_ICMP == result)
            {
                port->rx_stats.icmp++;
                if (rusteron_dpdk_port_owns_ip(port, frame.dst_ip))
                {
                    rusteron_dpdk_icmp_handle(native, port, m, &frame);
                }
            }
            else if (RUSTERON_DPDK_RX_RESULT_ARP == result)
            {
                port->rx_stats.arp++;
                rusteron_dpdk_arp_handle_frame(&port->arp, native, port, m->data, m->frame_len);
            }
            else
            {
                rusteron_dpdk_rx_count(&port->rx_stats, &port->counters, result);
            }

            /* The callback has returned (and the ARP handler has run); the mbuf is
             * recycled now and no later (plan §7.6). */
            ops->mbuf_release(m);
        }
        total += received;
        if ((size_t)received < limit)
        {
            break;
        }
        full = 1;
    }

    if (total > 0)
    {
        rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_POLLER, (int64_t)total);
        rusteron_dpdk_counters_max(&port->counters, RD_COUNTER_RX_BACKLOG_MAX, (int64_t)total);
        /* drained within a burst of the ring's size: it may have overflowed
         * (never on an ENA, which counts its drops: rx_ring_full_at) */
        if (total >= port->rx_ring_full_at)
        {
            rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_RX_RING_FULL_SUSPECTED, 1);
        }
        if (work_count > 0)
        {
            rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_RX_PKTS, work_count);
            rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_RX_BYTES, rx_bytes);
        }
        if (sw_csum > 0)
        {
            rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_RX_SW_CHECKSUM, sw_csum);
        }
        if (hw_count > 0)
        {
            rusteron_dpdk_counters_min_counted(
                &port->counters, RD_COUNTER_RX_HW_DELAY_MIN, hw_min, RD_COUNTER_RX_HW_TIMESTAMPED);
            rusteron_dpdk_counters_max(&port->counters, RD_COUNTER_RX_HW_DELAY_MAX, hw_max);
            rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_RX_HW_DELAY_SUM, hw_sum);
            rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_RX_HW_TIMESTAMPED, hw_count);
        }
        if (hw_negative > 0)
        {
            rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_RX_HW_DELAY_NEGATIVE, hw_negative);
        }
    }

    /* Receive-ring health: a frame arriving while the agent is away waits in
     * the ring, which overflows (silently, on MANA) once the gap outlasts it.
     * So record the gaps that ended with frames waiting, and full bursts. */
    const uint64_t now_ns = rusteron_dpdk_clock_now();
    /* The agent's own loop: any gap since the port's last poll of 20 µs or
     * more is the agent not running (preempted, or its vCPU descheduled), so
     * a latency spike can be matched to it. The clock is read here anyway. */
    const uint64_t now_cpu_ns = rusteron_dpdk_gap_attributing() ? rusteron_dpdk_thread_cpu_ns() : 0;
    rusteron_dpdk_poll_cycle(port, now_ns, now_cpu_ns);
    if (total > 0 && 0 != port->last_poll_ns)
    {
        const int64_t gap = (int64_t)(now_ns - port->last_poll_ns);
        rusteron_dpdk_counters_max(&port->counters, RD_COUNTER_RX_POLL_GAP_MAX, gap);
        if (gap >= 100000)
        {
            rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_RX_POLL_GAPS_100US, 1);
        }
    }
    if (full)
    {
        rusteron_dpdk_counters_add(&port->counters, RD_COUNTER_RX_BURST_FULL, 1);
    }
    port->last_poll_ns = now_ns;
    port->last_poll_cpu_ns = now_cpu_ns;
    /* The 1 Hz statistics sample runs on the housekeeping thread, never here. */
    return work_count;
}

int rusteron_dpdk_poller_init(
    aeron_udp_transport_poller_t *poller,
    aeron_driver_context_t *context,
    aeron_udp_channel_transport_affinity_t affinity)
{
    (void)context;
    (void)affinity;

    if (NULL == poller)
    {
        rusteron_dpdk_set_error("poller_init: poller must not be NULL");
        return -1;
    }

    poller->transports.array = NULL;
    poller->transports.length = 0;
    poller->transports.capacity = 0;
    poller->fd = -1;

    rusteron_dpdk_poller_state_t *state = NULL;
    if (aeron_alloc((void **)&state, sizeof(rusteron_dpdk_poller_state_t)) < 0)
    {
        rusteron_dpdk_set_error("poller_init: out of memory allocating poller state");
        return -1;
    }
    if (rusteron_dpdk_endpoint_map_init(&state->endpoints) < 0)
    {
        aeron_free(state);
        rusteron_dpdk_set_error("poller_init: out of memory allocating endpoint map");
        return -1;
    }
    poller->bindings_clientd = state;
    return 0;
}

int rusteron_dpdk_poller_close(aeron_udp_transport_poller_t *poller)
{
    if (NULL == poller)
    {
        return 0;
    }

    /* Best-effort unregister (plan §7.2). This runs on the conductor's close
     * path (main lcore), where the seam is a safe no-op; the registered sender
     * and receiver network threads are process-lifetime because EAL cannot be
     * reinitialized (§7.2 line 272). */
    rusteron_dpdk_eal_thread_unregister();

    rusteron_dpdk_poller_state_t *state = (rusteron_dpdk_poller_state_t *)poller->bindings_clientd;
    if (NULL != state)
    {
        rusteron_dpdk_endpoint_map_close(&state->endpoints);
        aeron_free(state->kernel);
        aeron_free(state);
        poller->bindings_clientd = NULL;
    }
    if (NULL != poller->transports.array)
    {
        aeron_free(poller->transports.array);
    }
    poller->transports.array = NULL;
    poller->transports.length = 0;
    poller->transports.capacity = 0;
    poller->fd = -1;
    return 0;
}

/* Count `client`'s NIC among the poller's drained NICs. */
static int rusteron_dpdk_poller_port_ref(rusteron_dpdk_poller_state_t *state, rusteron_dpdk_client_t *client)
{
    for (size_t i = 0; i < state->port_count; i++)
    {
        if (state->ports[i].client->port == client->port)
        {
            state->ports[i].refs++;
            return 0;
        }
    }
    if (state->port_count == RUSTERON_DPDK_POLLER_MAX_PORTS)
    {
        rusteron_dpdk_set_error("poller_add: too many NICs on one poller");
        return -1;
    }
    state->ports[state->port_count].client = client;
    state->ports[state->port_count].refs = 1;
    state->port_count++;
    return 0;
}

/* Uncount `client`'s NIC; when it was the one draining it, hand that role to
 * another registered transport of the NIC. */
static void rusteron_dpdk_poller_port_unref(
    aeron_udp_transport_poller_t *poller, rusteron_dpdk_poller_state_t *state, rusteron_dpdk_client_t *client)
{
    for (size_t i = 0; i < state->port_count; i++)
    {
        if (state->ports[i].client->port != client->port)
        {
            continue;
        }
        if (0 == --state->ports[i].refs)
        {
            /* no longer polled here: the time until a transport of it comes
             * back is not the agent's loop, so its timing starts afresh */
            client->port->last_poll_ns = 0;
            client->port->last_poll_cpu_ns = 0;
            state->ports[i] = state->ports[--state->port_count];
            return;
        }
        if (state->ports[i].client == client)
        {
            for (size_t t = 0; t < poller->transports.length; t++)
            {
                rusteron_dpdk_client_t *other = poller->transports.array[t].transport->bindings_clientd;
                if (NULL != other && other != client && !other->delegated && other->port == client->port)
                {
                    state->ports[i].client = other;
                    break;
                }
            }
        }
        return;
    }
}

int rusteron_dpdk_poller_add(
    aeron_udp_transport_poller_t *poller, aeron_udp_channel_transport_t *transport)
{
    if (NULL == poller || NULL == transport)
    {
        rusteron_dpdk_set_error("poller_add: poller/transport must not be NULL");
        return -1;
    }
    rusteron_dpdk_poller_state_t *state = (rusteron_dpdk_poller_state_t *)poller->bindings_clientd;
    if (NULL == state)
    {
        rusteron_dpdk_set_error("poller_add: poller is not initialized");
        return -1;
    }
    rusteron_dpdk_client_t *client = transport->bindings_clientd;
    if (NULL == client || NULL == client->port)
    {
        rusteron_dpdk_set_error("poller_add: transport has no DPDK client state");
        return -1;
    }

    /* Grow the registered-transport list first, so nothing below needs
     * undoing on its account. */
    if (poller->transports.length == poller->transports.capacity)
    {
        size_t new_capacity = poller->transports.capacity == 0 ? 4 : poller->transports.capacity * 2;
        void *array = realloc(poller->transports.array, new_capacity * sizeof(aeron_udp_channel_transport_entry_t));
        if (NULL == array)
        {
            rusteron_dpdk_set_error("poller_add: out of memory growing transport list");
            return -1;
        }
        poller->transports.array = (aeron_udp_channel_transport_entry_t *)array;
        poller->transports.capacity = new_capacity;
    }
    /* Each transport has at most one kernel socket: with room for all of
     * them, a DPDK transport's agent adopting its host-local socket later
     * (send_local) never allocates. */
    if (rusteron_dpdk_poller_kernel_reserve(state, poller->transports.length + 1) < 0)
    {
        return -1;
    }

    /* All or nothing: the kernel-socket entry, the endpoint keys, the NIC. */
    const int kernel = client->delegated || client->kernel_fd >= 0;
    if (kernel && rusteron_dpdk_poller_kernel_add(state, transport) < 0)
    {
        return -1;
    }
    if (!client->delegated)
    {
        const int put = rusteron_dpdk_endpoints_put(state, client, transport);
        if (put < 0)
        {
            if (kernel)
            {
                rusteron_dpdk_poller_kernel_remove(state, transport);
            }
            if (RUSTERON_DPDK_ENDPOINT_MAP_FULL == put)
            {
                AERON_SET_ERR(ENOSPC, "poller_add: endpoint map full (%d endpoints)", RUSTERON_DPDK_ENDPOINT_MAP_MAX);
                rusteron_dpdk_set_error("poller_add: endpoint map full");
                return -1;
            }
            AERON_SET_ERR(EADDRINUSE, "poller_add: endpoint port %u is already registered",
                          (unsigned)client->local_udp_port);
            rusteron_dpdk_set_error("poller_add: duplicate endpoint");
            return -1;
        }
        if (rusteron_dpdk_poller_port_ref(state, client) < 0)
        {
            rusteron_dpdk_endpoints_remove(state, client);
            if (kernel)
            {
                rusteron_dpdk_poller_kernel_remove(state, transport);
            }
            return -1;
        }
    }

    client->poller = state;
    client->kernel_polled = kernel;
    poller->transports.array[poller->transports.length++].transport = transport;
    return 0;
}

int rusteron_dpdk_poller_remove(
    aeron_udp_transport_poller_t *poller, aeron_udp_channel_transport_t *transport)
{
    if (NULL == poller || NULL == transport)
    {
        return -1;
    }
    rusteron_dpdk_poller_state_t *state = (rusteron_dpdk_poller_state_t *)poller->bindings_clientd;

    int index = -1;
    for (size_t i = 0; i < poller->transports.length; i++)
    {
        if (poller->transports.array[i].transport == transport)
        {
            index = (int)i;
            break;
        }
    }
    if (index < 0)
    {
        return 0; /* not registered */
    }

    size_t last = poller->transports.length - 1;
    if ((size_t)index < last)
    {
        poller->transports.array[index] = poller->transports.array[last];
    }
    poller->transports.length--;

    rusteron_dpdk_client_t *client = transport->bindings_clientd;
    if (NULL != state)
    {
        /* the host-local channel that times its port's loop is leaving: the
         * timing starts afresh with whichever channel times it next */
        if (state->kernel_count > 0 && state->kernel[0] == transport && NULL != client && NULL != client->port)
        {
            client->port->last_poll_ns = 0;
            client->port->last_poll_cpu_ns = 0;
        }
        rusteron_dpdk_poller_kernel_remove(state, transport);
        if (NULL != client && NULL != client->port && !client->delegated)
        {
            rusteron_dpdk_endpoints_remove(state, client);
            rusteron_dpdk_poller_port_unref(poller, state, client);
        }
    }
    if (NULL != client)
    {
        client->poller = NULL;
        client->kernel_polled = 0;
    }
    return 0;
}

int rusteron_dpdk_poller_poll(
    aeron_udp_transport_poller_t *poller,
    struct mmsghdr *msgvec,
    size_t vlen,
    int64_t *bytes_rcved,
    aeron_udp_transport_recv_func_t recv_func,
    aeron_udp_channel_transport_recvmmsg_func_t recvmmsg_func,
    void *clientd)
{
    (void)recvmmsg_func;

    if (NULL == poller || NULL == msgvec || NULL == bytes_rcved || NULL == recv_func)
    {
        return -1;
    }
    rusteron_dpdk_poller_state_t *state = (rusteron_dpdk_poller_state_t *)poller->bindings_clientd;
    if (NULL == state)
    {
        return 0;
    }

    int work_count = 0;

    /* Host-local channels only (every localhost run): no NIC to poll, so the
     * agent's loop gaps are measured here instead, on the first channel's
     * port, at one clock read a call. */
    if (0 == state->port_count && state->kernel_count > 0)
    {
        rusteron_dpdk_client_t *first = state->kernel[0]->bindings_clientd;
        if (NULL != first && NULL != first->port)
        {
            const uint64_t now = rusteron_dpdk_clock_now();
            const uint64_t now_cpu = rusteron_dpdk_gap_attributing() ? rusteron_dpdk_thread_cpu_ns() : 0;
            rusteron_dpdk_poll_cycle(first->port, now, now_cpu);
            first->port->last_poll_ns = now;
            first->port->last_poll_cpu_ns = now_cpu;
        }
    }

    /* Each distinct NIC once per call (plan §7.6), through any of its
     * transports: the endpoint map dispatches to the right one. */
    for (size_t i = 0; i < state->port_count; i++)
    {
        /* Frames queued behind a full TX ring go out as the ring drains, on
         * this agent (it owns the NIC), not only at its next send. */
        rusteron_dpdk_client_t *c = state->ports[i].client;
        if (0 != c->port->tx_queued_count)
        {
            rusteron_dpdk_port_tx_flush(c->runtime, c->port);
        }
        int received = rusteron_dpdk_poller_receive(
            c, NULL, &state->endpoints, msgvec, vlen, bytes_rcved, recv_func, clientd);
        if (received < 0)
        {
            return -1;
        }
        work_count += received;
    }

    /* Kernel sockets: only those the local-I/O thread flagged (epoll), plus
     * those that had data within the hot window or that it has taken off its
     * epoll (adaptive), or all (spin).
     * A flag is cleared before reading (a full barrier, so a datagram
     * arriving after it raises a new edge), and a full read raises it again
     * to keep draining. Idle ones cost a load each, and the clock is read
     * at most once per call, only when a socket is hot. */
    uint64_t now_ns = 0;
    for (size_t k = 0; k < state->kernel_count; k++)
    {
        aeron_udp_channel_transport_t *transport = state->kernel[k];
        rusteron_dpdk_client_t *client = transport->bindings_clientd;
        if (NULL == client || client->kernel_fd < 0 || client->ksock < 0)
        {
            continue;
        }
        rusteron_dpdk_transport_t *native = client->runtime;
        rusteron_dpdk_ksock_t *ksock = &native->ksocks[client->ksock];
        volatile uint8_t *ready = &ksock->ready;
        const int flagged = __atomic_load_n(ready, __ATOMIC_ACQUIRE);
        /* spin, or off the epoll (nothing would flag it): every cycle */
        int read_anyway = RUSTERON_DPDK_LOCAL_IO_SPIN == native->local_io_mode ||
                          __atomic_load_n(&ksock->unwatched, __ATOMIC_ACQUIRE);
        if (!flagged && !read_anyway && 0 != client->local_hot_until_ns)
        {
            if (0 == now_ns)
            {
                now_ns = rusteron_dpdk_clock_now();
            }
            read_anyway = now_ns < client->local_hot_until_ns;
            if (!read_anyway)
            {
                client->local_hot_until_ns = 0; /* cooled: back to waiting for the flag */
            }
        }
        if (!flagged && !read_anyway)
        {
            continue;
        }
        if (flagged)
        {
            __atomic_exchange_n(ready, 0, __ATOMIC_SEQ_CST);
        }
        int received = rusteron_dpdk_kernel_recv(
            client->kernel_fd, client->delegated ? NULL : client, transport, msgvec, vlen, bytes_rcved, recv_func,
            clientd);
        if (received > 0 && RUSTERON_DPDK_LOCAL_IO_ADAPTIVE == native->local_io_mode)
        {
            if (0 == now_ns)
            {
                now_ns = rusteron_dpdk_clock_now();
            }
            /* expired counts as cold: off the epoll, the cooling branch never runs */
            const int was_cold = client->local_hot_until_ns <= now_ns;
            client->local_hot_until_ns = now_ns + native->local_hot_ns;
            /* for the local-I/O thread: while hot, the socket comes off the epoll */
            __atomic_store_n(&ksock->hot_until_ns, client->local_hot_until_ns, __ATOMIC_RELEASE);
            if (was_cold)
            {
                __atomic_fetch_add(&native->local_hot_epoch.value, 1, __ATOMIC_RELEASE);
            }
        }
        if (received < 0 || (size_t)received == vlen)
        {
            /* A full read, or an error (a pending ICMP error is reported
             * ahead of queued datagrams): more may wait, so read again next
             * cycle rather than wait for an edge that may never come. */
            __atomic_store_n(ready, 1, __ATOMIC_RELEASE);
        }
        if (RUSTERON_DPDK_KERNEL_RECV_AGAIN == received)
        {
            received = 0;
        }
        else if (received < 0)
        {
            return -1;
        }
        work_count += received;
    }

    return work_count;
}
