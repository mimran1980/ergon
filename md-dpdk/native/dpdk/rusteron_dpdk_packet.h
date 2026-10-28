/*
 * ergon-md-dpdk DPDK transport — frame encoding (plan §7.4).
 *
 * Builds Ethernet/IPv4/UDP frames and ARP frames into an opaque mbuf view so
 * the same encoders drive the real DPDK port ops and the test fake. The view
 * carries the checksum-offload metadata DPDK needs: L2/L3/L4 lengths, the
 * offload flags, and the UDP pseudo-header checksum seed (a raw one's-complement
 * sum, no final complement — that is what DPDK's TX checksum offload expects).
 *
 * This header is DPDK-free and included by the fake and by port ops.
 */
#ifndef RUSTERON_DPDK_PACKET_H
#define RUSTERON_DPDK_PACKET_H

#include <stddef.h>
#include <stdint.h>

#include <sys/uio.h> /* struct iovec */

#ifdef __cplusplus
extern "C" {
#endif

/* Frame layout constants (network byte order where multi-byte). */
#define RUSTERON_DPDK_ETH_ADDR_LEN 6
#define RUSTERON_DPDK_ETH_HDR_LEN 14
#define RUSTERON_DPDK_IPV4_HDR_LEN 20
#define RUSTERON_DPDK_UDP_HDR_LEN 8
#define RUSTERON_DPDK_FRAME_HDR_LEN \
    (RUSTERON_DPDK_ETH_HDR_LEN + RUSTERON_DPDK_IPV4_HDR_LEN + RUSTERON_DPDK_UDP_HDR_LEN)

#define RUSTERON_DPDK_ETH_TYPE_IPV4 0x0800
#define RUSTERON_DPDK_ETH_TYPE_ARP 0x0806
#define RUSTERON_DPDK_ETH_TYPE_IPV6 0x86DD
#define RUSTERON_DPDK_ETH_TYPE_VLAN 0x8100
#define RUSTERON_DPDK_ETH_TYPE_QINQ 0x88A8
#define RUSTERON_DPDK_IP_PROTO_UDP 17
#define RUSTERON_DPDK_IP_PROTO_ICMP 1
#define RUSTERON_DPDK_ARP_HTYPE_ETHERNET 1
#define RUSTERON_DPDK_ARP_PTYPE_IPV4 0x0800
#define RUSTERON_DPDK_ARP_HLEN 6
#define RUSTERON_DPDK_ARP_PLEN 4
#define RUSTERON_DPDK_ARP_OPER_REQUEST 1
#define RUSTERON_DPDK_ARP_OPER_REPLY 2

/* Offload flags carried on the mbuf view; the real port ops map them onto the
 * RTE_MBUF_F_TX_* bits. */
#define RUSTERON_DPDK_MBUF_F_TX_IPV4 (1u << 0)
#define RUSTERON_DPDK_MBUF_F_TX_IP_CKSUM (1u << 1)
#define RUSTERON_DPDK_MBUF_F_TX_UDP_CKSUM (1u << 2)

/* RX offload verdicts copied from the NIC onto the view by the port ops. When
 * neither GOOD nor BAD is set for a protocol, the NIC left the verdict unknown
 * and the transport software-verifies that checksum (plan §7.6). */
#define RUSTERON_DPDK_MBUF_F_RX_IPV4_CKSUM_GOOD (1u << 0)
#define RUSTERON_DPDK_MBUF_F_RX_UDP_CKSUM_GOOD (1u << 1)
#define RUSTERON_DPDK_MBUF_F_RX_IPV4_CKSUM_BAD (1u << 2)
#define RUSTERON_DPDK_MBUF_F_RX_UDP_CKSUM_BAD (1u << 3)

/* DPDK encodes each RX checksum verdict as a two-bit enum (rte_mbuf_core.h,
 * RTE_MBUF_F_RX_{IP,L4}_CKSUM_*), not as two independent bits:
 *
 *   00 UNKNOWN  the device did not verify it
 *   01 BAD      verified, and it is wrong
 *   10 GOOD     verified, and it is correct
 *   11 NONE     the integrity of the packet is verified although the bytes
 *               are not a normal checksum for this protocol
 *
 * NONE has both the GOOD and the BAD bit set, so testing the bits
 * independently classifies a verified packet as both, and the BAD half drops
 * it. Decode the enum, never the individual bits. */
#define RUSTERON_DPDK_CKSUM_UNKNOWN 0u
#define RUSTERON_DPDK_CKSUM_BAD 1u
#define RUSTERON_DPDK_CKSUM_GOOD 2u
#define RUSTERON_DPDK_CKSUM_NONE 3u

/* Map one DPDK checksum verdict onto the (good_bit | bad_bit) view flags.
 * UNKNOWN maps to 0, which makes the transport software-verify; NONE maps to
 * good_bit alone, which trusts the verified integrity and skips the software
 * check; the packet must never come out with bad_bit set. */
static inline uint32_t rusteron_dpdk_cksum_view(unsigned status, uint32_t good_bit, uint32_t bad_bit)
{
    switch (status)
    {
    case RUSTERON_DPDK_CKSUM_GOOD:
    case RUSTERON_DPDK_CKSUM_NONE:
        return good_bit;
    case RUSTERON_DPDK_CKSUM_BAD:
        return bad_bit;
    default:
        return 0;
    }
}

/* Decode a DPDK ol_flags word onto the RX verdict view flags.
 *
 * The two verdicts' bit masks come from rte_mbuf_core.h as arguments, not as
 * a hardcoded shift: their positions moved between DPDK releases (23.11 has
 * the IP pair at bits 4 and 7 and the L4 pair at 3 and 8, where older
 * releases had two adjacent pairs at 5/6 and 7/8). Reading them as adjacent
 * bits on 23.11 puts the IP GOOD bit in the L4 half, where it counts as BAD
 * and the frame is dropped.
 *
 * For each verdict pass RTE_MBUF_F_RX_{IP,L4}_CKSUM_{BAD,GOOD,NONE}. NONE is
 * BAD|GOOD and is also the mask of the pair. */
uint32_t rusteron_dpdk_rx_ol_flags_from_dpdk(
    uint64_t ol, uint64_t ip_bad, uint64_t ip_good, uint64_t ip_none, uint64_t l4_bad, uint64_t l4_good,
    uint64_t l4_none);

/*
 * Opaque-to-the-encoder mbuf view. The port ops fill `opaque`/`data`/`capacity`
 * at allocation and consume everything at transmit; the encoders only write
 * `data` and the metadata fields.
 */
typedef struct rusteron_dpdk_mbuf_stct
{
    void *opaque;            /* rte_mbuf* (or fake tag); owned by the port ops */
    uint8_t *data;           /* writable frame start */
    uint32_t capacity;       /* bytes available at data */
    uint32_t frame_len;      /* bytes written (eth+ip+udp+payload) */
    uint32_t ol_flags;       /* RUSTERON_DPDK_MBUF_F_TX_* */
    uint16_t l2_len;
    uint16_t l3_len;
    uint16_t l4_len;
    uint16_t udp_pseudo_csum; /* UDP pseudo-header checksum for TX offload */
    uint32_t nb_segs;         /* received frame segment count (1 expected) */
    uint32_t rx_ol_flags;     /* RX offload verdicts (RUSTERON_DPDK_MBUF_F_RX_*) */
    uint64_t hw_rx_ns;        /* NIC RX timestamp in ns (the PHC's clock), 0 = none */
} rusteron_dpdk_mbuf_t;

/* IPv4 helpers (addresses are network order). */
uint32_t rusteron_dpdk_ipv4_mask(uint8_t prefix_len);
int rusteron_dpdk_ipv4_is_multicast(uint32_t ip);

/*
 * Build a UDP datagram frame into `m` (already allocated with room). The iovec
 * bytes follow the Ethernet/IPv4/UDP headers; a payload longer than
 * `max_payload` (or the mbuf capacity) is rejected with -1 and the frame is not
 * touched. On success sets frame_len, ol_flags, the L2/L3/L4 lengths and the
 * UDP pseudo-header checksum seed. IPv4 DF is always set; never fragments.
 */
int rusteron_dpdk_packet_build_udp(
    rusteron_dpdk_mbuf_t *m,
    const uint8_t dst_mac[RUSTERON_DPDK_ETH_ADDR_LEN],
    const uint8_t src_mac[RUSTERON_DPDK_ETH_ADDR_LEN],
    uint32_t dst_ip, uint32_t src_ip,
    uint16_t src_port, uint16_t dst_port,
    uint8_t tos, /* the IPv4 ToS byte: the channel's socket-tos, else 0 */
    const struct iovec *iov, size_t iov_count,
    size_t max_payload,
    int hw_csum); /* 1: the NIC completes both checksums; 0: computed here */

/* ARP request: broadcast dst, 00:00:00:00:00:00 target hw addr. */
int rusteron_dpdk_packet_build_arp_request(
    rusteron_dpdk_mbuf_t *m,
    const uint8_t src_mac[RUSTERON_DPDK_ETH_ADDR_LEN],
    uint32_t src_ip, uint32_t target_ip);

/* ARP reply: unicast to the requester's MAC. */
int rusteron_dpdk_packet_build_arp_reply(
    rusteron_dpdk_mbuf_t *m,
    const uint8_t dst_mac[RUSTERON_DPDK_ETH_ADDR_LEN],
    const uint8_t src_mac[RUSTERON_DPDK_ETH_ADDR_LEN],
    uint32_t sender_ip, uint32_t target_ip);

/*
 * An ICMP message from this host: Ethernet + IPv4 (no DF, checksum computed in
 * software) + ICMP type/code, the 4-byte rest-of-header and `body` (the echo
 * data, or the offending datagram's IP header + 8 bytes for an error), ICMP
 * checksum computed in software. -1 when it does not fit.
 */
int rusteron_dpdk_packet_build_icmp(
    rusteron_dpdk_mbuf_t *m,
    const uint8_t dst_mac[RUSTERON_DPDK_ETH_ADDR_LEN],
    const uint8_t src_mac[RUSTERON_DPDK_ETH_ADDR_LEN],
    uint32_t src_ip, uint32_t dst_ip,
    uint8_t type, uint8_t code, const uint8_t rest[4],
    const uint8_t *body, size_t body_len);

/* The internet checksum (RFC 1071) of `len` bytes, ready to store big-endian. */
uint16_t rusteron_dpdk_inet_csum(const uint8_t *p, size_t len);

/* --- Receive classification (plan §7.6) --- */

typedef enum rusteron_dpdk_rx_result_en
{
    RUSTERON_DPDK_RX_RESULT_OK = 0,   /* IPv4/UDP, *out filled */
    RUSTERON_DPDK_RX_RESULT_ARP,      /* feed the frame to the ARP table */
    RUSTERON_DPDK_RX_RESULT_IPV6,
    RUSTERON_DPDK_RX_RESULT_MULTICAST, /* Ethernet or IP multicast/broadcast */
    RUSTERON_DPDK_RX_RESULT_ETHERTYPE, /* unsupported EtherType */
    RUSTERON_DPDK_RX_RESULT_VLAN,      /* VLAN/QinQ encapsulation */
    RUSTERON_DPDK_RX_RESULT_IP_OPTIONS,
    RUSTERON_DPDK_RX_RESULT_FRAGMENT,  /* MF set or non-zero fragment offset */
    RUSTERON_DPDK_RX_RESULT_TRUNCATED, /* short or length-inconsistent frame */
    RUSTERON_DPDK_RX_RESULT_PROTOCOL,  /* non-UDP IP protocol */
    RUSTERON_DPDK_RX_RESULT_CHECKSUM,  /* NIC-reported or software-invalid */
    RUSTERON_DPDK_RX_RESULT_MULTI_SEGMENT,
    RUSTERON_DPDK_RX_RESULT_ICMP       /* ICMP for this host: *out has the addresses, payload = ICMP */
} rusteron_dpdk_rx_result_t;

/* A valid single-segment IPv4/UDP datagram, parsed for dispatch. All addresses
 * are network order; ports are host order. `payload` points into the mbuf. */
typedef struct rusteron_dpdk_parsed_frame_stct
{
    uint32_t src_ip;
    uint32_t dst_ip;
    uint16_t src_port;
    uint16_t dst_port;
    const uint8_t *payload;
    size_t payload_len;
    int sw_checksum; /* its UDP checksum was verified in software (no NIC verdict) */
} rusteron_dpdk_parsed_frame_t;

/*
 * Classify a received mbuf. RUSTERON_DPDK_RX_RESULT_OK fills *out; every other
 * result is a reject class (plan §7.6). No allocation, no locking, no logging.
 */
rusteron_dpdk_rx_result_t rusteron_dpdk_packet_classify_rx(
    const rusteron_dpdk_mbuf_t *m, rusteron_dpdk_parsed_frame_t *out);

#ifdef __cplusplus
}
#endif

#endif /* RUSTERON_DPDK_PACKET_H */
