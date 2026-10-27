//! Property tests of the receive classifier, `rusteron_dpdk_packet_classify_rx`
//! (native/dpdk/rusteron_dpdk_packet.c), against frames built here, not by the
//! transport's own builders:
//!
//! - every well-formed IPv4/UDP frame (any unicast MAC and address, IP
//!   options, Ethernet padding, any NIC checksum verdict that is not "IPv4
//!   bad") is accepted, with its addresses, ports and payload exactly;
//! - the same frame cut short anywhere before its UDP payload ends is never
//!   accepted;
//! - with no NIC verdict, any single bit flipped in the IPv4 header (after its
//!   first byte) or in the UDP header and payload is never accepted, except a
//!   flip that leaves the UDP checksum zero (legal IPv4: no checksum);
//! - any bytes at all: an accepted frame always satisfies the header
//!   invariants, and its payload lies inside the frame.
//!
//! Fixed seed, few cases, no failure persistence: the same cases every run.
//! Linux x86_64 or aarch64; links the same archives as dpdk_rx.rs.
#![allow(
    missing_docs,
    unsafe_code,
    unsafe_op_in_unsafe_fn,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]
#![cfg(all(
    feature = "dpdk",
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]

mod common;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, RngSeed};
use std::os::raw::c_void;

/// Mirror of `rusteron_dpdk_mbuf_t` (native/dpdk/rusteron_dpdk_packet.h).
#[repr(C)]
struct Mbuf {
    opaque: *mut c_void,
    data: *mut u8,
    capacity: u32,
    frame_len: u32,
    ol_flags: u32,
    l2_len: u16,
    l3_len: u16,
    l4_len: u16,
    udp_pseudo_csum: u16,
    nb_segs: u32,
    rx_ol_flags: u32,
    hw_rx_ns: u64,
}

/// Mirror of `rusteron_dpdk_parsed_frame_t`.
#[repr(C)]
struct Parsed {
    src_ip: u32,
    dst_ip: u32,
    src_port: u16,
    dst_port: u16,
    payload: *const u8,
    payload_len: usize,
    sw_checksum: i32,
}

unsafe extern "C" {
    fn rusteron_dpdk_packet_classify_rx(m: *const Mbuf, out: *mut Parsed) -> i32;
}

const RX_OK: i32 = 0;
const IPV4_GOOD: u32 = 1 << 0;
const UDP_GOOD: u32 = 1 << 1;
const UDP_BAD: u32 = 1 << 3;

/// The classifier's verdict on `frame` (its first `len` bytes), and what it
/// parsed. The buffer is exactly `len` bytes long.
fn classify(frame: &[u8], rx_ol_flags: u32, nb_segs: u32) -> (i32, Parsed) {
    let mut buf = frame.to_vec();
    let m = Mbuf {
        opaque: std::ptr::null_mut(),
        data: buf.as_mut_ptr(),
        capacity: buf.len() as u32,
        frame_len: buf.len() as u32,
        ol_flags: 0,
        l2_len: 0,
        l3_len: 0,
        l4_len: 0,
        udp_pseudo_csum: 0,
        nb_segs,
        rx_ol_flags,
        hw_rx_ns: 0,
    };
    let mut out = Parsed {
        src_ip: 0,
        dst_ip: 0,
        src_port: 0,
        dst_port: 0,
        payload: std::ptr::null(),
        payload_len: usize::MAX,
        sw_checksum: -1,
    };
    let result = unsafe { rusteron_dpdk_packet_classify_rx(&m, &mut out) };
    // Report the payload as an offset into the frame (the buffer dies here).
    if result == RX_OK && !out.payload.is_null() {
        out.payload = (out.payload as usize).wrapping_sub(buf.as_ptr() as usize) as *const u8;
    }
    (result, out)
}

/// One's-complement sum of 16-bit big-endian words (an odd byte padded).
fn sum16(bytes: &[u8], mut acc: u32) -> u32 {
    for pair in bytes.chunks(2) {
        let word = u32::from(pair[0]) << 8 | u32::from(*pair.get(1).unwrap_or(&0));
        acc += word;
    }
    acc
}

fn fold(mut acc: u32) -> u16 {
    while acc > 0xffff {
        acc = (acc & 0xffff) + (acc >> 16);
    }
    !(acc as u16)
}

/// A well-formed frame and what the classifier must report for it.
#[derive(Debug, Clone)]
struct Frame {
    bytes: Vec<u8>,
    /// Ethernet + IPv4 (with options) + UDP header.
    headers: usize,
    /// Where the UDP datagram ends (padding may follow).
    end: usize,
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    payload_len: usize,
    udp_csum: u16,
}

fn frame_strategy() -> impl Strategy<Value = Frame> {
    (
        any::<[u8; 6]>(),
        any::<[u8; 6]>(),
        any::<[u8; 4]>(),
        (0u8..224, any::<[u8; 3]>()),
        any::<u16>(),
        any::<u16>(),
        0usize..=10,
        prop::collection::vec(any::<u8>(), 0..=1472),
        any::<bool>(),
        0usize..=24,
        any::<bool>(),
        any::<u16>(),
    )
        .prop_map(
            |(
                dst_mac,
                src_mac,
                src_ip,
                (dst0, dst_rest),
                src_port,
                dst_port,
                opt_words,
                payload,
                df,
                pad,
                csum_on,
                ident,
            )| {
                let dst_ip = [dst0, dst_rest[0], dst_rest[1], dst_rest[2]];
                let ihl = 20 + 4 * opt_words;
                let udp_len = 8 + payload.len();
                let total_len = ihl + udp_len;
                let mut b = Vec::with_capacity(14 + total_len + pad);
                let mut dst_mac = dst_mac;
                dst_mac[0] &= 0xfe; // unicast
                b.extend_from_slice(&dst_mac);
                b.extend_from_slice(&src_mac);
                b.extend_from_slice(&[0x08, 0x00]);
                // IPv4 header, options as NOPs (type 1).
                let ip_start = b.len();
                b.push(0x40 | (ihl / 4) as u8);
                b.push(0);
                b.extend_from_slice(&(total_len as u16).to_be_bytes());
                b.extend_from_slice(&ident.to_be_bytes());
                b.extend_from_slice(&(if df { 0x4000u16 } else { 0 }).to_be_bytes());
                b.push(64);
                b.push(17);
                b.extend_from_slice(&[0, 0]);
                b.extend_from_slice(&src_ip);
                b.extend_from_slice(&dst_ip);
                b.resize(ip_start + ihl, 1);
                let ip_csum = fold(sum16(&b[ip_start..ip_start + ihl], 0));
                b[ip_start + 10..ip_start + 12].copy_from_slice(&ip_csum.to_be_bytes());
                // UDP header and payload; checksum over the pseudo-header.
                let udp_start = b.len();
                b.extend_from_slice(&src_port.to_be_bytes());
                b.extend_from_slice(&dst_port.to_be_bytes());
                b.extend_from_slice(&(udp_len as u16).to_be_bytes());
                b.extend_from_slice(&[0, 0]);
                b.extend_from_slice(&payload);
                let mut udp_csum = 0u16;
                if csum_on {
                    let mut pseudo = Vec::with_capacity(12);
                    pseudo.extend_from_slice(&src_ip);
                    pseudo.extend_from_slice(&dst_ip);
                    pseudo.extend_from_slice(&[0, 17]);
                    pseudo.extend_from_slice(&(udp_len as u16).to_be_bytes());
                    udp_csum = fold(sum16(&b[udp_start..], sum16(&pseudo, 0)));
                    if udp_csum == 0 {
                        udp_csum = 0xffff; // 0 means "no checksum" on the wire
                    }
                    b[udp_start + 6..udp_start + 8].copy_from_slice(&udp_csum.to_be_bytes());
                }
                let end = b.len();
                b.resize(end + pad, 0); // Ethernet padding past the datagram
                Frame {
                    bytes: b,
                    headers: udp_start + 8,
                    end,
                    src_ip,
                    dst_ip,
                    src_port,
                    dst_port,
                    payload_len: payload.len(),
                    udp_csum,
                }
            },
        )
}

/// NIC verdicts that never reject a good frame: none, both good, or a UDP
/// "bad" the classifier re-checks in software.
fn verdict_strategy() -> impl Strategy<Value = u32> {
    prop_oneof![
        Just(0u32),
        Just(IPV4_GOOD | UDP_GOOD),
        Just(IPV4_GOOD),
        Just(UDP_BAD),
        Just(IPV4_GOOD | UDP_BAD)
    ]
}

fn config() -> Config {
    Config {
        cases: 96,
        rng_seed: RngSeed::Fixed(0x6572_676f_6e64_7064),
        failure_persistence: None,
        ..Config::default()
    }
}

proptest! {
    #![proptest_config(config())]

    #[test]
    fn well_formed_frames_are_accepted_exactly(f in frame_strategy(), flags in verdict_strategy()) {
        let (result, out) = classify(&f.bytes, flags, 1);
        prop_assert_eq!(result, RX_OK, "{:?}", f);
        prop_assert_eq!(out.src_ip.to_ne_bytes(), f.src_ip);
        prop_assert_eq!(out.dst_ip.to_ne_bytes(), f.dst_ip);
        prop_assert_eq!(out.src_port, f.src_port);
        prop_assert_eq!(out.dst_port, f.dst_port);
        prop_assert_eq!(out.payload as usize, f.headers);
        prop_assert_eq!(out.payload_len, f.payload_len);
        // Verified in software whenever the NIC did not call UDP good.
        let sw = flags & UDP_GOOD == 0 || flags & UDP_BAD != 0;
        prop_assert_eq!(out.sw_checksum, i32::from(sw));
    }

    #[test]
    fn a_frame_cut_short_is_never_accepted(f in frame_strategy(), flags in verdict_strategy(), cut in any::<prop::sample::Index>()) {
        let len = cut.index(f.end);
        let (result, _) = classify(&f.bytes[..len], flags, 1);
        prop_assert_ne!(result, RX_OK, "accepted {} of {} bytes", len, f.end);
    }

    #[test]
    fn a_flipped_bit_without_a_nic_verdict_is_never_accepted(f in frame_strategy(), at in any::<prop::sample::Index>(), bit in 0u8..8) {
        prop_assume!(f.udp_csum != 0);
        // Anywhere in the IPv4 header after version/IHL, the UDP header or payload.
        let start = 15;
        let pos = start + at.index(f.end - start);
        let mut bytes = f.bytes.clone();
        bytes[pos] ^= 1 << bit;
        let udp_csum_at = f.headers - 2;
        let csum_now = u16::from_be_bytes([bytes[udp_csum_at], bytes[udp_csum_at + 1]]);
        // A flip that zeroes the UDP checksum turns it off: legal, accepted.
        prop_assume!(!(pos >= udp_csum_at && pos < udp_csum_at + 2 && csum_now == 0));
        let (result, _) = classify(&bytes, 0, 1);
        prop_assert_ne!(result, RX_OK, "bit {} of byte {} flipped, still accepted", bit, pos);
    }

    #[test]
    fn whatever_is_accepted_is_a_well_formed_udp_frame(
        bytes in prop::collection::vec(any::<u8>(), 0..=96),
        flags in 0u32..16,
        segs in 0u32..3,
        udp_shape in any::<bool>(),
    ) {
        let mut bytes = bytes;
        if udp_shape && bytes.len() >= 42 {
            // Nudge some inputs towards IPv4/UDP so the deep checks run.
            bytes[0] &= 0xfe;
            bytes[12] = 0x08;
            bytes[13] = 0x00;
            bytes[14] = 0x45;
            bytes[20] &= 0x80;
            bytes[21] = 0;
            bytes[23] = 17;
            let total = (bytes.len() - 14) as u16;
            bytes[16..18].copy_from_slice(&total.to_be_bytes());
            let udp = (bytes.len() - 34) as u16;
            bytes[38..40].copy_from_slice(&udp.to_be_bytes());
        }
        let (result, out) = classify(&bytes, flags, segs);
        if result == RX_OK {
            prop_assert_eq!(segs, 1);
            prop_assert!(bytes.len() >= 42);
            prop_assert_eq!(&bytes[12..14], &[0x08, 0x00]);
            prop_assert_eq!(bytes[14] >> 4, 4);
            let ihl = usize::from(bytes[14] & 0x0f) * 4;
            prop_assert!(ihl >= 20);
            prop_assert_eq!(bytes[14 + 9], 17);
            prop_assert_eq!(out.payload as usize, 14 + ihl + 8);
            prop_assert!(out.payload as usize + out.payload_len <= bytes.len());
            prop_assert_eq!(flags & 0b100, 0, "accepted with the NIC's IPv4-bad verdict");
        }
    }
}

/// The generator itself: a frame with nothing special still parses, so a
/// broken generator cannot make the properties pass vacuously.
#[test]
fn the_generator_builds_a_known_frame() {
    let mut runner = proptest::test_runner::TestRunner::new(config());
    let tree = frame_strategy()
        .new_tree(&mut runner)
        .map_err(|e| e.to_string())
        .unwrap();
    let f = tree.current();
    assert!(f.end >= 42 && f.bytes.len() >= f.end);
    assert_eq!(classify(&f.bytes, 0, 1).0, RX_OK, "{f:?}");
}
