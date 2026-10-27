//! Aeron transmit path over DPDK (plan §7.4/§7.5).
//!
//! Drives the real Aeron binding callbacks (init/send/close/bind_addr_and_port)
//! against the DPDK-free fakes and asserts on the captured frames: golden
//! IPv4/UDP vectors, checksum-offload metadata, DF, routing (direct vs gateway),
//! the ARP state machine (rate limiting, learning, expiry, anti-poisoning), MTU
//! rejection, mbuf exhaustion, partial bursts, and mbuf ownership.
//!
//! The native transport links the same `rusteron_dpdk` core archive plus the
//! fakes declared in `common/mod.rs`. Linux x86_64 or aarch64.
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
use common::*;

use serial_test::serial;

use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

// Real Aeron types/functions from the generated bindings so we exercise the
// genuine callback contract.
// The Aeron types and functions of the driver this crate installs into.
use common::transport::{Poller, RxDatagram};
use rusteron_media_driver::bindings::*;

// ---------------------------------------------------------------------------
// Native test hooks (fakes + runtime).
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn rusteron_dpdk_transport_bindings() -> *mut aeron_udp_channel_transport_bindings_stct;
    fn rusteron_dpdk_transport_test_arp_rx(transport: *mut c_void, role: c_int, frame: *const u8, len: usize) -> c_int;
}

// ---------------------------------------------------------------------------
// IPv4 sockaddr mirror (bindings.rs has no sockaddr_in on Linux either).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct SockAddrIn {
    sin_family: u16,
    sin_port: u16, // network order
    sin_addr: u32, // network-order bytes in memory
    sin_zero: [u8; 8],
}

impl SockAddrIn {
    fn new(ip: [u8; 4], port: u16) -> Self {
        Self {
            sin_family: 2, // AF_INET
            sin_port: port.to_be(),
            // s_addr is a plain u32 read natively by the C side (no ntohl), so
            // the octets must sit in memory in order: ip[0] at the low byte on
            // little-endian x86_64 and aarch64. from_be_bytes would store them reversed and
            // every ARP lookup / multicast check in transport.c would miss.
            sin_addr: u32::from_le_bytes(ip),
            sin_zero: [0; 8],
        }
    }
    fn storage_mut(&mut self) -> *mut sockaddr_storage {
        self as *mut Self as *mut sockaddr_storage
    }
}

// ---------------------------------------------------------------------------
// Harness: native runtime + a genuine Aeron transport struct + binding table.
// ---------------------------------------------------------------------------

struct Harness {
    _env: TestEnv,
    native: *mut c_void,
    bindings: *mut aeron_udp_channel_transport_bindings_stct,
    transport: Box<aeron_udp_channel_transport_stct>,
    data_paths: Box<aeron_udp_channel_data_paths_stct>,
    initialized: bool,
}

impl Harness {
    fn new() -> Self {
        Harness::new_with_burst(rusteron_dpdk_config_t::valid().burst_size)
    }

    fn new_with_burst(burst_size: u16) -> Self {
        let mut config = rusteron_dpdk_config_t::valid();
        config.burst_size = burst_size;
        Self::new_with_config(config, |_| {})
    }

    /// A harness over `config`, with `setup` run on the fresh test env first.
    fn new_with_config(config: rusteron_dpdk_config_t, setup: impl FnOnce(&TestEnv)) -> Self {
        let env = TestEnv::new();
        env.eal_skip();
        setup(&env);
        let native = create(&config).unwrap_or_else(|e| panic!("create failed: {e}"));
        let bindings = unsafe { rusteron_dpdk_transport_bindings() };
        assert!(!bindings.is_null());
        let transport = Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() });
        let data_paths = Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_data_paths_stct>() });
        Harness {
            _env: env,
            native,
            bindings,
            transport,
            data_paths,
            initialized: false,
        }
    }

    /// Run the binding init_func with an IPv4 bind address, optional connect
    /// address, and the given channel MTU.
    fn init(
        &mut self,
        affinity: aeron_udp_channel_transport_affinity_t,
        bind_ip: [u8; 4],
        bind_port: u16,
        connect: Option<([u8; 4], u16)>,
        mtu: usize,
    ) {
        assert!(!self.initialized, "harness already initialized");
        let t = &mut *self.transport;
        unsafe { ptr::write_bytes(t as *mut _, 0, 1) };

        let mut bind = SockAddrIn::new(bind_ip, bind_port);
        let mut dummy = SockAddrIn::new([0, 0, 0, 0], 0);
        let mut conn = connect.map(|(ip, p)| SockAddrIn::new(ip, p));
        let mut params: aeron_udp_channel_transport_params_stct = unsafe { std::mem::zeroed() };
        params.mtu_length = mtu;

        let rc = unsafe {
            let b = &*self.bindings;
            b.init_func.unwrap()(
                t,
                bind.storage_mut(),
                dummy.storage_mut(),
                conn.as_mut().map_or(ptr::null_mut(), SockAddrIn::storage_mut),
                &mut params,
                ptr::null_mut(),
                affinity,
            )
        };
        assert_eq!(rc, 0, "init failed: {}", last_error());
        self.initialized = true;
    }

    /// Send to an explicit destination address (unconnected path).
    fn send_to(&mut self, dst: ([u8; 4], u16), payloads: &[&[u8]]) -> (c_int, i64) {
        let mut d = SockAddrIn::new(dst.0, dst.1);
        self.send_common(d.storage_mut(), payloads)
    }

    /// Send with a NULL address (connected path: uses connected_address).
    fn send_connected(&mut self, payloads: &[&[u8]]) -> (c_int, i64) {
        self.send_common(ptr::null_mut(), payloads)
    }

    fn send_common(&mut self, addr: *mut sockaddr_storage, payloads: &[&[u8]]) -> (c_int, i64) {
        let iovs: Vec<iovec> = payloads
            .iter()
            .map(|p| iovec {
                iov_base: p.as_ptr() as *mut c_void,
                iov_len: p.len(),
            })
            .collect();
        let mut bytes_sent: i64 = 0;
        let rc = unsafe {
            let b = &*self.bindings;
            b.send_func.unwrap()(
                &mut *self.data_paths,
                &mut *self.transport,
                addr,
                iovs.as_ptr() as *mut iovec,
                iovs.len(),
                &mut bytes_sent,
            )
        };
        (rc, bytes_sent)
    }

    fn bind_addr_and_port(&mut self) -> String {
        let mut buf = [0 as c_char; 128];
        let rc = unsafe {
            let b = &*self.bindings;
            b.bind_addr_and_port_func.unwrap()(&mut *self.transport, buf.as_mut_ptr(), buf.len())
        };
        assert_eq!(rc, 0, "bind_addr_and_port failed: {}", last_error());
        let s = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) };
        s.to_string_lossy().into_owned()
    }

    fn arp_seed(&self, ip: [u8; 4], mac: [u8; 6]) {
        let ip_str = CString::new(format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])).unwrap();
        let rc = unsafe { rusteron_dpdk_transport_test_arp_seed(self.native, ip_str.as_ptr(), mac.as_ptr()) };
        assert_eq!(rc, 0, "arp_seed failed: {}", last_error());
    }

    /// A poller over this harness's transport, as the agent that owns it.
    fn poller(&mut self) -> Poller {
        let mut p = Poller::new(self.bindings);
        p.add(&mut *self.transport);
        p
    }

    fn arp_rx(&self, role: c_int, frame: &[u8]) -> c_int {
        unsafe { rusteron_dpdk_transport_test_arp_rx(self.native, role, frame.as_ptr(), frame.len()) }
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                let b = &*self.bindings;
                b.close_func.unwrap()(&mut *self.transport);
            }
        }
        close(self.native);
    }
}

// ---------------------------------------------------------------------------
// Frame helpers.
// ---------------------------------------------------------------------------

const SENDER: i32 = 0;
const SENDER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];
const RECEIVER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x02];
const SENDER_IP: [u8; 4] = [10, 0, 0, 1];
const SENDER_GATEWAY: [u8; 4] = [10, 0, 0, 254];
const RECEIVER_IP: [u8; 4] = [10, 0, 1, 1];

fn captures() -> Vec<FakeCapture> {
    let n = unsafe { rusteron_dpdk_fake_capture_count() };
    (0..n)
        .map(|i| {
            let mut c = unsafe { std::mem::zeroed::<FakeCapture>() };
            let rc = unsafe { rusteron_dpdk_fake_capture_at(i, &mut c) };
            assert_eq!(rc, 0);
            c
        })
        .collect()
}

fn cap_bytes(c: &FakeCapture) -> &[u8] {
    &c.data[..c.len as usize]
}

fn eth_dst(c: &FakeCapture) -> [u8; 6] {
    c.data[0..6].try_into().unwrap()
}
fn eth_src(c: &FakeCapture) -> [u8; 6] {
    c.data[6..12].try_into().unwrap()
}
fn eth_type(c: &FakeCapture) -> u16 {
    u16::from_be_bytes([c.data[12], c.data[13]])
}
fn u16_at(c: &FakeCapture, off: usize) -> u16 {
    u16::from_be_bytes([c.data[off], c.data[off + 1]])
}
fn u32_ip_at(c: &FakeCapture, off: usize) -> [u8; 4] {
    c.data[off..off + 4].try_into().unwrap()
}

const IP: usize = 14;
const IP_SRC: usize = IP + 12;
const IP_DST: usize = IP + 16;
const UDP: usize = IP + 20;
const UDP_DPORT: usize = UDP + 2;
const PAYLOAD: usize = UDP + 8;

fn ip_ver_ihl(c: &FakeCapture) -> u8 {
    c.data[IP]
}
fn ip_flags_frag(c: &FakeCapture) -> u16 {
    u16_at(c, IP + 6)
}
fn ip_ttl(c: &FakeCapture) -> u8 {
    c.data[IP + 8]
}
fn ip_proto(c: &FakeCapture) -> u8 {
    c.data[IP + 9]
}
fn ip_csum(c: &FakeCapture) -> u16 {
    u16_at(c, IP + 10)
}
fn udp_payload(c: &FakeCapture) -> &[u8] {
    &c.data[PAYLOAD..c.len as usize]
}

/// The same pseudo-header seed DPDK's TX offload expects (packet.c).
fn pseudo_csum(src: [u8; 4], dst: [u8; 4], udp_len: u16) -> u16 {
    let mut sum: u32 = 0;
    for (s, d) in src.chunks(2).zip(dst.chunks(2)) {
        sum += u16::from_be_bytes([s[0], s[1]]) as u32;
        sum += u16::from_be_bytes([d[0], d[1]]) as u32;
    }
    sum += 17; // IPPROTO_UDP
    sum += udp_len as u32;
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    sum as u16
}

/// Build a full UDP golden frame (mirrors packet.c byte-for-byte).
fn build_udp_frame(
    dst_mac: [u8; 6],
    src_mac: [u8; 6],
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    sport: u16,
    dport: u16,
    payload: &[u8],
) -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(&dst_mac);
    f.extend_from_slice(&src_mac);
    f.extend_from_slice(&[0x08, 0x00]);
    f.push(0x45);
    f.push(0x00);
    let total = (20 + 8 + payload.len()) as u16;
    f.extend_from_slice(&total.to_be_bytes());
    f.extend_from_slice(&[0x00, 0x00]); // id
    f.extend_from_slice(&[0x40, 0x00]); // DF
    f.push(64); // ttl
    f.push(17); // proto
    f.extend_from_slice(&[0x00, 0x00]); // csum (hw offload)
    f.extend_from_slice(&src_ip);
    f.extend_from_slice(&dst_ip);
    f.extend_from_slice(&sport.to_be_bytes());
    f.extend_from_slice(&dport.to_be_bytes());
    let ulen = (8 + payload.len()) as u16;
    f.extend_from_slice(&ulen.to_be_bytes());
    f.extend_from_slice(&pseudo_csum(src_ip, dst_ip, ulen).to_be_bytes());
    f.extend_from_slice(payload);
    f
}

/// Build an ARP request frame (42 bytes).
fn arp_request(sha: [u8; 6], spa: [u8; 4], tpa: [u8; 4]) -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(&[0xff; 6]);
    f.extend_from_slice(&sha);
    f.extend_from_slice(&[0x08, 0x06]);
    f.extend_from_slice(&0x0001u16.to_be_bytes()); // htype
    f.extend_from_slice(&0x0800u16.to_be_bytes()); // ptype
    f.push(6);
    f.push(4);
    f.extend_from_slice(&1u16.to_be_bytes()); // oper request
    f.extend_from_slice(&sha);
    f.extend_from_slice(&spa);
    f.extend_from_slice(&[0; 6]);
    f.extend_from_slice(&tpa);
    f
}

/// Build an ARP reply frame (42 bytes).
fn arp_reply(dst_mac: [u8; 6], sha: [u8; 6], spa: [u8; 4], tpa: [u8; 4]) -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(&dst_mac);
    f.extend_from_slice(&sha);
    f.extend_from_slice(&[0x08, 0x06]);
    f.extend_from_slice(&0x0001u16.to_be_bytes());
    f.extend_from_slice(&0x0800u16.to_be_bytes());
    f.push(6);
    f.push(4);
    f.extend_from_slice(&2u16.to_be_bytes()); // oper reply
    f.extend_from_slice(&sha);
    f.extend_from_slice(&spa);
    f.extend_from_slice(&dst_mac);
    f.extend_from_slice(&tpa);
    f
}

fn arp_fields(c: &FakeCapture) -> (u16, u16, [u8; 6], [u8; 4], [u8; 6], [u8; 4]) {
    let a = 14;
    (
        u16_at(c, a),
        u16_at(c, a + 6),
        c.data[a + 8..a + 14].try_into().unwrap(),
        u32_ip_at(c, a + 14),
        c.data[a + 18..a + 24].try_into().unwrap(),
        u32_ip_at(c, a + 24),
    )
}

// ---------------------------------------------------------------------------
// Golden vectors / frame correctness
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn udp_frame_golden_vector() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    const TARGET_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x50];
    h.arp_seed([10, 0, 0, 5], TARGET_MAC);

    let payload = b"hello";
    let (rc, bytes_sent) = h.send_to(([10, 0, 0, 5], 40123), &[payload]);
    assert_eq!(rc, 1);
    assert_eq!(bytes_sent, 5);

    let caps = captures();
    assert_eq!(caps.len(), 1);
    let c = &caps[0];
    let expected = build_udp_frame(TARGET_MAC, SENDER_MAC, SENDER_IP, [10, 0, 0, 5], 40000, 40123, payload);
    assert_eq!(cap_bytes(c), expected.as_slice());

    // Checksum-offload metadata (plan §7.4).
    assert_eq!(c.ol_flags, 0b111, "IPV4|IP_CKSUM|UDP_CKSUM");
    assert_eq!((c.l2_len, c.l3_len, c.l4_len), (14, 20, 8));
    assert_eq!(c.udp_pseudo_csum, pseudo_csum(SENDER_IP, [10, 0, 0, 5], 13));
    assert_eq!(c.port_id, 0, "sends on the sender ENA");
    assert_no_leak();
}

#[test]
#[serial]
fn frame_fields_are_ordinary_valid_ipv4_udp() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 9], [0x02, 0, 0, 0, 0, 0x51]);

    let (rc, _) = h.send_to(([10, 0, 0, 9], 40123), &[b"abcd", b"efgh"]);
    assert_eq!(rc, 2);
    let caps = captures();
    assert_eq!(caps.len(), 2, "each iovec is one datagram");
    for c in &caps {
        assert_eq!(eth_type(c), 0x0800);
        assert_eq!(ip_ver_ihl(c), 0x45);
        assert_eq!(ip_proto(c), 17);
        assert_eq!(ip_ttl(c), 64);
        assert_eq!(ip_flags_frag(c), 0x4000, "DF must be set, never fragment");
        assert_eq!(ip_csum(c), 0, "IPv4 checksum delegated to hw offload");
        assert_eq!(c.l2_len, 14);
        assert_eq!(c.l3_len, 20);
        assert_eq!(c.l4_len, 8);
    }
    assert_eq!(udp_payload(&caps[0]), b"abcd");
    assert_eq!(udp_payload(&caps[1]), b"efgh");
    assert_eq!(caps[0].len, 42 + 4);
    assert_eq!(caps[1].len, 42 + 4);
    assert_no_leak();
}

// ---------------------------------------------------------------------------
// Routing
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn routing_direct_for_in_subnet_gateway_otherwise() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    const PEER: [u8; 6] = [0x02, 0, 0, 0, 0, 0x60];
    const GW: [u8; 6] = [0x02, 0, 0, 0, 0, 0xfe];
    h.arp_seed([10, 0, 0, 5], PEER);
    h.arp_seed(SENDER_GATEWAY, GW);

    // In-subnet: ARP the destination itself.
    let (rc, _) = h.send_to(([10, 0, 0, 5], 1000), &[b"direct"]);
    assert_eq!(rc, 1);
    let c0 = &captures()[0];
    assert_eq!(eth_dst(c0), PEER);
    assert_eq!(u32_ip_at(c0, IP_DST), [10, 0, 0, 5]);

    // Outside the /24: ARP the gateway, keep the destination IP in the header.
    let (rc, _) = h.send_to(([10, 0, 2, 5], 1000), &[b"viagw"]);
    assert_eq!(rc, 1);
    let c1 = &captures()[1];
    assert_eq!(eth_dst(c1), GW);
    assert_eq!(u32_ip_at(c1, IP_DST), [10, 0, 2, 5]);
    assert_eq!(u32_ip_at(c1, IP_SRC), SENDER_IP);
    assert_no_leak();
}

#[test]
#[serial]
fn connected_transport_uses_connected_address() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        Some(([10, 0, 0, 77], 5000)),
        1472,
    );
    h.arp_seed([10, 0, 0, 77], [0x02, 0, 0, 0, 0, 0x77]);

    let (rc, bytes_sent) = h.send_connected(&[b"connected"]);
    assert_eq!(rc, 1);
    assert_eq!(bytes_sent, 9);
    let c = &captures()[0];
    assert_eq!(u32_ip_at(c, IP_DST), [10, 0, 0, 77]);
    assert_eq!(u16_at(c, UDP_DPORT), 5000);
    assert_no_leak();
}

#[test]
#[serial]
fn multicast_destination_rejected() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    let (rc, bytes_sent) = h.send_to(([224, 0, 0, 5], 1234), &[b"mc"]);
    assert_eq!(rc, -1);
    assert_eq!(bytes_sent, 0);
    assert!(last_error().contains("multicast"), "got: {}", last_error());
    assert_eq!(captures().len(), 0);
}

#[test]
#[serial]
fn receiver_affinity_uses_receiver_port() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 1, 9], [0x02, 0, 0, 0, 0, 0x59]);

    let (rc, _) = h.send_to(([10, 0, 1, 9], 7000), &[b"rx"]);
    assert_eq!(rc, 1);
    let c = &captures()[0];
    assert_eq!(c.port_id, 1);
    assert_eq!(eth_src(c), RECEIVER_MAC);
    assert_eq!(u32_ip_at(c, IP_SRC), RECEIVER_IP);
    assert_no_leak();
}

// ---------------------------------------------------------------------------
// MTU / oversized
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn oversized_datagram_rejected() {
    let mut h = Harness::new();
    // Channel MTU of 32 bytes: 32 fits, 33 is rejected.
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        32,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);

    let ok_payload = [7u8; 32];
    let (rc, bytes_sent) = h.send_to(([10, 0, 0, 5], 40123), &[&ok_payload]);
    assert_eq!(rc, 1);
    assert_eq!(bytes_sent, 32);

    let (rc, bytes_sent) = h.send_to(([10, 0, 0, 5], 40123), &[&[1u8; 33]]);
    assert_eq!(rc, -1);
    assert_eq!(bytes_sent, 0);
    assert!(last_error().contains("oversized"), "got: {}", last_error());
    assert_no_leak();
}

#[test]
#[serial]
fn oversized_after_valid_prefix_flushes_prefix_then_errors() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        32,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);

    let small = [7u8; 8];
    let (rc, _) = h.send_to(([10, 0, 0, 5], 40123), &[&small, &[1u8; 33]]);
    assert_eq!(rc, -1, "permanent validation error after sending the prefix");
    let caps = captures();
    assert_eq!(caps.len(), 1, "the valid prefix was transmitted");
    assert_eq!(udp_payload(&caps[0]), small.as_slice());
    assert_no_leak();
}

#[test]
#[serial]
fn oversized_channel_mtu_rejected_at_init() {
    // A channel MTU over max_aeron_mtu is rejected at init (plan §6.5), not
    // clamped: it fails loudly instead of silently degrading to a different
    // MTU. The reject fires before any port/affinity work (§11.1), so the
    // failed init leaves nothing to roll back.
    let env = TestEnv::new();
    env.eal_skip();
    let mut config = rusteron_dpdk_config_t::valid();
    config.max_aeron_mtu = 32; // small 32-aligned ceiling
    let native = create(&config).unwrap_or_else(|e| panic!("create failed: {e}"));
    let bindings = unsafe { rusteron_dpdk_transport_bindings() };
    assert!(!bindings.is_null());
    let mut transport = Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() });
    let mut bind = SockAddrIn::new(SENDER_IP, 40000);
    let mut params: aeron_udp_channel_transport_params_stct = unsafe { std::mem::zeroed() };
    params.mtu_length = 64; // exceeds the 32-byte ceiling
    let rc = unsafe {
        let b = &*bindings;
        b.init_func.unwrap()(
            &mut *transport,
            bind.storage_mut(),
            ptr::null_mut(),
            ptr::null_mut(),
            &mut params,
            ptr::null_mut(),
            aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        )
    };
    assert_eq!(rc, -1, "init must reject an over-limit channel MTU");
    assert!(last_error().contains("exceeds max_aeron_mtu"), "got: {}", last_error());
    close(native);
    assert_no_leak();
}

// ---------------------------------------------------------------------------
// Batching / mbuf ownership
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn each_iovec_is_one_datagram() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);

    let (rc, bytes_sent) = h.send_to(([10, 0, 0, 5], 40123), &[b"a", b"bb", b"ccc"]);
    assert_eq!(rc, 3, "returns the accepted datagram count");
    assert_eq!(bytes_sent, 6, "bytes_sent accrues the accepted payload bytes");
    let caps = captures();
    assert_eq!(caps.len(), 3);
    assert_eq!(udp_payload(&caps[0]), b"a");
    assert_eq!(udp_payload(&caps[1]), b"bb");
    assert_eq!(udp_payload(&caps[2]), b"ccc");
    assert_eq!(caps[1].len, 42 + 2);
    assert_no_leak();
}

#[test]
#[serial]
fn burst_flushes_at_burst_size_and_accumulates() {
    let mut h = Harness::new_with_burst(4);
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);

    let payloads: Vec<Vec<u8>> = (0..8).map(|i| vec![i; 4]).collect();
    let refs: Vec<&[u8]> = payloads.iter().map(|p| p.as_slice()).collect();
    let (rc, bytes_sent) = h.send_to(([10, 0, 0, 5], 40123), &refs);
    assert_eq!(rc, 8);
    assert_eq!(bytes_sent, 8 * 4);
    assert_eq!(captures().len(), 8, "two bursts of 4");
    assert_no_leak();
}

/// The payloads of every frame the NIC has taken, in order.
fn sent_payloads() -> Vec<Vec<u8>> {
    captures().iter().map(|c| udp_payload(c).to_vec()).collect()
}

/// A full ring takes part of a vector: the rest waits, already built, in the
/// port's queue, and the whole vector is reported sent, as a kernel socket
/// queues what its NIC cannot take yet. Reporting the accepted prefix instead
/// made Aeron send the whole vector again, the accepted frames a second time.
#[test]
#[serial]
fn a_full_ring_queues_the_rest_and_reports_the_whole_vector() {
    let mut h = Harness::new_with_burst(4);
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);

    // The NIC accepts only 2 frames a burst.
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(2) };
    let payloads: Vec<Vec<u8>> = (0..8).map(|i| vec![i; 4]).collect();
    let refs: Vec<&[u8]> = payloads.iter().map(|p| p.as_slice()).collect();
    let (rc, bytes_sent) = h.send_to(([10, 0, 0, 5], 40123), &refs);
    assert_eq!(rc, 8, "the whole vector: the refused frames are queued");
    assert_eq!(bytes_sent, 8 * 4);
    assert_eq!(sent_payloads(), vec![vec![0; 4], vec![1; 4]]);

    // The ring drains: the queued frames go first, then the new one.
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(64) };
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[&[9u8; 4]]).0, 1);
    let mut expected: Vec<Vec<u8>> = (0..8).map(|i| vec![i; 4]).collect();
    expected.push(vec![9; 4]);
    assert_eq!(sent_payloads(), expected, "each frame once, in order");
    assert_no_leak();
}

/// Aeron advances only when a send takes the whole vector, and otherwise
/// sends the same vector again. Whatever the NIC takes per call, every frame
/// must reach the wire exactly once and in order.
#[test]
#[serial]
fn aerons_resend_rule_never_duplicates_a_frame() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    // The NIC takes 1, 2 or 3 frames a call: three 4-frame vectors each.
    for (round, cap) in [1u16, 2, 3].into_iter().enumerate() {
        unsafe { rusteron_dpdk_fake_set_tx_burst_cap(cap) };
        for v in 0..3u32 {
            let first = u32::try_from(round).unwrap() * 12 + v * 4;
            let payloads: Vec<Vec<u8>> = (first..first + 4).map(|i| i.to_le_bytes().to_vec()).collect();
            let refs: Vec<&[u8]> = payloads.iter().map(|p| p.as_slice()).collect();
            let mut attempts = 0;
            loop {
                let (rc, _) = h.send_to(([10, 0, 0, 5], 40123), &refs);
                assert!(rc == 0 || rc == 4, "all or nothing, never a part: {rc}");
                if rc == 4 {
                    break;
                }
                attempts += 1;
                assert!(attempts < 100, "the vector never went out");
            }
        }
    }
    // Drain what is still queued.
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(64) };
    let mut p = h.poller();
    let mut bytes = 0i64;
    let mut dgrams: Vec<RxDatagram> = Vec::new();
    let clientd = &mut dgrams as *mut Vec<RxDatagram> as *mut c_void;
    p.poll(16, clientd, &mut bytes);
    let expected: Vec<Vec<u8>> = (0..36u32).map(|i| i.to_le_bytes().to_vec()).collect();
    assert_eq!(sent_payloads(), expected, "every frame exactly once, in order");
    drop(p);
    assert_no_leak();
}

/// While queued frames cannot all go, a new vector is refused whole before
/// anything of it is built: no mbuf taken, no copy made, nothing charged.
#[test]
#[serial]
fn a_new_vector_is_refused_whole_while_frames_wait() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(0) }; // a full ring
    let first: Vec<Vec<u8>> = (0..4).map(|i| vec![i; 4]).collect();
    let refs: Vec<&[u8]> = first.iter().map(|p| p.as_slice()).collect();
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &refs).0, 4, "queued whole");
    let allocated = unsafe { rusteron_dpdk_fake_allocated() };
    let second: Vec<Vec<u8>> = (4..8).map(|i| vec![i; 4]).collect();
    let refs2: Vec<&[u8]> = second.iter().map(|p| p.as_slice()).collect();
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &refs2).0, 0, "refused whole");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_allocated() },
        allocated,
        "nothing built for it"
    );
    assert!(sent_payloads().is_empty());

    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(64) };
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &refs2).0, 4);
    let expected: Vec<Vec<u8>> = (0..8).map(|i| vec![i; 4]).collect();
    assert_eq!(sent_payloads(), expected);
    assert_no_leak();
}

/// Queued frames do not wait for the next send: the agent's poll of the NIC
/// sends them.
#[test]
#[serial]
fn the_agents_poll_sends_queued_frames() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    let mut p = h.poller();
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(0) };
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[&[1u8; 4], &[2u8; 4]]).0, 2);
    assert!(sent_payloads().is_empty());
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(64) };
    let mut bytes = 0i64;
    let mut dgrams: Vec<RxDatagram> = Vec::new();
    let clientd = &mut dgrams as *mut Vec<RxDatagram> as *mut c_void;
    p.poll(16, clientd, &mut bytes);
    assert_eq!(sent_payloads(), vec![vec![1u8; 4], vec![2u8; 4]], "sent by the poll");
    drop(p);
    assert_no_leak();
}

/// Frames still queued when the NIC's last transport closes are released.
#[test]
#[serial]
fn queued_frames_are_released_when_the_transport_closes() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(0) };
    assert_eq!(
        h.send_to(([10, 0, 0, 5], 40123), &[&[1u8; 4], &[2u8; 4], &[3u8; 4]]).0,
        3
    );
    drop(h);
    assert_no_leak();
}

/// With RUSTERON_DPDK_TX_RETRY_US the refused tail of a burst is offered
/// again while the NIC drains its ring, so a momentarily full ring does not
/// turn into a short send (which makes Aeron resend the whole vector,
/// duplicates included). Frames still refused at the deadline are released.
#[test]
#[serial]
fn refused_tail_is_offered_again_within_the_retry_window() {
    let _retry = EnvGuard::set("RUSTERON_DPDK_TX_RETRY_US", "1000");
    let mut h = Harness::new_with_burst(4);
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    // Each offer takes 2 frames: the ring drains between offers.
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(2) };
    let payloads: Vec<Vec<u8>> = (0..8).map(|i| vec![i; 4]).collect();
    let refs: Vec<&[u8]> = payloads.iter().map(|p| p.as_slice()).collect();
    let (rc, bytes_sent) = h.send_to(([10, 0, 0, 5], 40123), &refs);
    assert_eq!(rc, 8, "every datagram sent, in order");
    assert_eq!(bytes_sent, 8 * 4);
    let caps = captures();
    assert_eq!(caps.len(), 8);
    for (i, c) in caps.iter().enumerate() {
        assert_eq!(udp_payload(c), vec![i as u8; 4].as_slice());
    }
    assert_no_leak();

    // A ring that never drains: still refused at the deadline, so queued.
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(0) };
    let (rc, _) = h.send_to(([10, 0, 0, 5], 40123), &refs);
    assert_eq!(rc, 8, "queued behind the full ring");
    drop(h);
    assert_no_leak();
}

/// Out of mbufs part-way through a vector, nothing of it is sent: a part
/// would make Aeron send the whole vector again, the part a second time.
#[test]
#[serial]
fn mbuf_exhaustion_sends_nothing_of_the_vector() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);

    // Only 2 mbufs may be live; the third allocation fails.
    unsafe { rusteron_dpdk_fake_set_pool_avail(2) };

    let payloads: Vec<Vec<u8>> = (0..16).map(|i| vec![i; 4]).collect();
    let refs: Vec<&[u8]> = payloads.iter().map(|p| p.as_slice()).collect();
    let (rc, bytes_sent) = h.send_to(([10, 0, 0, 5], 40123), &refs);
    assert_eq!(rc, 0, "all or nothing: Aeron offers the vector again");
    assert_eq!(bytes_sent, 0);
    assert!(captures().is_empty());
    assert_no_leak();
}

#[test]
#[serial]
fn zero_length_send_is_retryable_zero() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    let iovs: Vec<iovec> = vec![];
    let mut bytes_sent: i64 = 0;
    let mut d = SockAddrIn::new([10, 0, 0, 5], 40123);
    let rc = unsafe {
        let b = &*h.bindings;
        b.send_func.unwrap()(
            &mut *h.data_paths,
            &mut *h.transport,
            d.storage_mut(),
            iovs.as_ptr() as *mut iovec,
            0,
            &mut bytes_sent,
        )
    };
    assert_eq!(rc, 0);
    assert_eq!(bytes_sent, 0);
}

// ---------------------------------------------------------------------------
// ARP state machine (plan §7.5)
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn arp_request_on_unresolved_rate_limited_100ms() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    // A non-zero base so the first send (last_request_ms starts at 0) clears
    // the 100 ms retry gate; the pinned clock must sit past the window.
    pin_clock_ms(1000);

    // First send: unresolved -> ARP request, retryable zero result.
    let (rc, bytes_sent) = h.send_to(([10, 0, 0, 5], 40123), &[b"x"]);
    assert_eq!(rc, 0, "unresolved ARP returns zero so Aeron retries");
    assert_eq!(bytes_sent, 0);
    assert_eq!(captures().len(), 1);

    // Within the 100 ms retry window: still unresolved, no new request.
    pin_clock_ms(1050);
    let (rc, _) = h.send_to(([10, 0, 0, 5], 40123), &[b"x"]);
    assert_eq!(rc, 0);
    assert_eq!(captures().len(), 1, "rate-limited: no request within 100 ms");

    // Past the window: request sent again.
    pin_clock_ms(1101);
    let (rc, _) = h.send_to(([10, 0, 0, 5], 40123), &[b"x"]);
    assert_eq!(rc, 0);
    assert_eq!(captures().len(), 2);

    // The request is a broadcast ARP request for the next hop.
    let req = &captures()[0];
    assert_eq!(eth_type(req), 0x0806);
    assert_eq!(eth_dst(req), [0xff; 6]);
    let (htype, oper, sha, spa, tha, tpa) = arp_fields(req);
    assert_eq!(htype, 1);
    assert_eq!(oper, 1);
    assert_eq!(sha, SENDER_MAC);
    assert_eq!(spa, SENDER_IP);
    assert_eq!(tha, [0; 6]);
    assert_eq!(tpa, [10, 0, 0, 5]);
    assert_no_leak();
}

/// The ARP retry gate runs on the nanosecond clock: one nanosecond short of
/// 100 ms is still inside the window, 100 ms exactly is not.
#[test]
#[serial]
fn arp_retry_gate_has_nanosecond_resolution() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    pin_clock_ms(1000);
    let (rc, _) = h.send_to(([10, 0, 0, 5], 40123), &[b"x"]);
    assert_eq!(rc, 0);
    assert_eq!(captures().len(), 1, "first ARP request");

    pin_clock_ns(1_100_000_000 - 1);
    let (rc, _) = h.send_to(([10, 0, 0, 5], 40123), &[b"x"]);
    assert_eq!(rc, 0);
    assert_eq!(captures().len(), 1, "1 ns inside the 100 ms window: no request");

    pin_clock_ns(1_100_000_000);
    let (rc, _) = h.send_to(([10, 0, 0, 5], 40123), &[b"x"]);
    assert_eq!(rc, 0);
    assert_eq!(captures().len(), 2, "window elapsed: request sent again");
    assert_no_leak();
}

#[test]
#[serial]
fn arp_learns_reply_and_sends() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    // Non-zero base so the unresolved send clears the 100 ms ARP retry gate.
    pin_clock_ms(1000);
    const PEER_MAC: [u8; 6] = [0x02, 0, 0, 0, 0, 0x50];

    let (rc, _) = h.send_to(([10, 0, 0, 5], 40123), &[b"x"]);
    assert_eq!(rc, 0);
    assert_eq!(captures().len(), 1, "ARP request");

    // A reply addressed to our MAC for the outstanding request is learned.
    let reply = arp_reply(SENDER_MAC, PEER_MAC, [10, 0, 0, 5], SENDER_IP);
    let consumed = h.arp_rx(SENDER, &reply);
    assert_eq!(consumed, 1, "reply for an outstanding request is consumed");

    let (rc, bytes_sent) = h.send_to(([10, 0, 0, 5], 40123), &[b"nowresolved"]);
    assert_eq!(rc, 1);
    assert_eq!(bytes_sent, 11);
    let caps = captures();
    assert_eq!(caps.len(), 2, "request + the data frame");
    assert_eq!(eth_dst(&caps[1]), PEER_MAC, "data frame goes to the learned MAC");
    assert_no_leak();
}

/// A learned next hop is never withdrawn from the send path (latency: no pause,
/// ever). After 30 s each send still goes to the cached MAC at once while a
/// refresh request goes out in the background, retried at most once a second
/// until answered; the answer restarts the 30 s.
#[test]
#[serial]
fn arp_refresh_never_pauses_sends() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    pin_clock_ms(1000);
    const PEER_MAC: [u8; 6] = [0x02, 0, 0, 0, 0, 0x50];
    h.arp_seed([10, 0, 0, 5], PEER_MAC);
    let data_frames = || captures().iter().filter(|c| eth_type(c) == 0x0800).count();
    let requests = || captures().iter().filter(|c| eth_type(c) == 0x0806).count();

    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"fresh"]).0, 1);
    assert_eq!((data_frames(), requests()), (1, 0));

    // 30 s on: the send still goes out, now, to the cached MAC, and a refresh
    // request goes out alongside it.
    pin_clock_ms(1000 + 30_000);
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"stale"]).0, 1, "never a zero send");
    assert_eq!(
        (data_frames(), requests()),
        (2, 1),
        "sent to the cached MAC, refresh requested"
    );
    let data: Vec<_> = captures().into_iter().filter(|c| eth_type(c) == 0x0800).collect();
    assert_eq!(eth_dst(&data[1]), PEER_MAC);

    // Unanswered: sends carry on; the refresh is retried at most once a second.
    pin_clock_ms(1000 + 30_500);
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"x"]).0, 1);
    assert_eq!(requests(), 1, "no retry within the second");
    pin_clock_ms(1000 + 31_000);
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"x"]).0, 1);
    assert_eq!(requests(), 2, "retried after a second");

    // The answer restarts the 30 s: no more requests until then.
    let reply = arp_reply(SENDER_MAC, PEER_MAC, [10, 0, 0, 5], SENDER_IP);
    assert_eq!(h.arp_rx(SENDER, &reply), 1, "a solicited refresh reply is learned");
    pin_clock_ms(1000 + 31_000 + 29_999);
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"x"]).0, 1);
    assert_eq!(requests(), 2, "fresh again");
    assert_no_leak();
}

/// The refresh request a send starts goes out after that send's data frame,
/// never before it: the data frame pays for no ARP build or transmit.
#[test]
#[serial]
fn an_arp_refresh_goes_out_after_the_data_frame() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    pin_clock_ms(1000);
    const PEER_MAC: [u8; 6] = [0x02, 0, 0, 0, 0, 0x50];
    h.arp_seed([10, 0, 0, 5], PEER_MAC);
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"fresh"]).0, 1);
    pin_clock_ms(1000 + 30_000);
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"due"]).0, 1);
    let order: Vec<u16> = captures().iter().map(eth_type).collect();
    assert_eq!(order, vec![0x0800, 0x0800, 0x0806], "[data, data, ARP]");
    assert_eq!(udp_payload(&captures()[1]), b"due");
    // A retried refresh (unanswered for a second) also follows the data.
    pin_clock_ms(1000 + 31_000);
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"again"]).0, 1);
    let order: Vec<u16> = captures().iter().map(eth_type).collect();
    assert_eq!(order[3..], [0x0800, 0x0806], "[data, ARP]");
    assert_no_leak();
}

/// A refresh reply may move the next hop to a new MAC (a pod rescheduled, a
/// failover), but only a
/// reply to an outstanding refresh: unsolicited replies still cannot.
#[test]
#[serial]
fn arp_refresh_reply_updates_the_mac() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    pin_clock_ms(1000);
    const OLD: [u8; 6] = [0x02, 0, 0, 0, 0, 0x50];
    const NEW: [u8; 6] = [0x02, 0, 0, 0, 0, 0x51];
    h.arp_seed([10, 0, 0, 5], OLD);

    let unsolicited = arp_reply(SENDER_MAC, NEW, [10, 0, 0, 5], SENDER_IP);
    assert_eq!(h.arp_rx(SENDER, &unsolicited), 0, "no refresh outstanding: ignored");

    pin_clock_ms(1000 + 30_000);
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"x"]).0, 1); // starts the refresh
    assert_eq!(h.arp_rx(SENDER, &unsolicited), 1, "the refresh's answer is learned");
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"y"]).0, 1);
    let data: Vec<_> = captures().into_iter().filter(|c| eth_type(c) == 0x0800).collect();
    assert_eq!(eth_dst(data.last().unwrap()), NEW);
    assert_no_leak();
}

#[test]
#[serial]
fn arp_reachable_entry_not_poisoned_by_unrelated_reply() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    pin_clock_ms(0);
    const GOOD: [u8; 6] = [0x02, 0, 0, 0, 0, 0x50];
    const EVIL: [u8; 6] = [0x02, 0, 0, 0, 0, 0x99];
    h.arp_seed([10, 0, 0, 5], GOOD);

    // An attacker claims the peer's IP with a different MAC while the entry is
    // reachable: it must be ignored (plan §7.5 anti-poisoning).
    let reply = arp_reply(SENDER_MAC, EVIL, [10, 0, 0, 5], SENDER_IP);
    let consumed = h.arp_rx(SENDER, &reply);
    assert_eq!(consumed, 0, "reply not addressed to an outstanding request");

    let (rc, _) = h.send_to(([10, 0, 0, 5], 40123), &[b"safe"]);
    assert_eq!(rc, 1);
    assert_eq!(eth_dst(&captures()[0]), GOOD, "reachable entry is unchanged");
    assert_no_leak();
}

#[test]
#[serial]
fn arp_responds_to_requests_for_local_ip() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );

    // A request for our local IPv4 gets a unicast reply.
    let req = arp_request([0x02, 0, 0, 0, 0, 0x42], [10, 0, 0, 42], SENDER_IP);
    let consumed = h.arp_rx(SENDER, &req);
    assert_eq!(consumed, 1);
    let caps = captures();
    assert_eq!(caps.len(), 1);
    let rep = &caps[0];
    assert_eq!(eth_type(rep), 0x0806);
    assert_eq!(eth_dst(rep), [0x02, 0, 0, 0, 0, 0x42]);
    let (_, oper, sha, spa, tha, tpa) = arp_fields(rep);
    assert_eq!(oper, 2, "reply");
    assert_eq!(sha, SENDER_MAC);
    assert_eq!(spa, SENDER_IP);
    assert_eq!(tha, [0x02, 0, 0, 0, 0, 0x42]);
    assert_eq!(tpa, [10, 0, 0, 42]);
    assert_no_leak();

    // A request for a foreign IP is ignored.
    let foreign = arp_request([0x02, 0, 0, 0, 0, 0x43], [10, 0, 0, 43], [10, 0, 0, 99]);
    let consumed = h.arp_rx(SENDER, &foreign);
    assert_eq!(consumed, 0, "requests for other IPs are not answered");
    assert_eq!(captures().len(), 1, "no extra frame");
}

/// An extra address (RUSTERON_DPDK_SENDER_EXTRA_IPV4) is answered as the NIC's
/// own: peers must resolve it to send replies to a channel bound on it.
#[test]
#[serial]
fn arp_responds_to_requests_for_extra_addresses() {
    let _guard = EnvGuard::set("RUSTERON_DPDK_SENDER_EXTRA_IPV4", "10.0.0.9");
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    let req = arp_request([0x02, 0, 0, 0, 0, 0x42], [10, 0, 0, 42], [10, 0, 0, 9]);
    assert_eq!(h.arp_rx(SENDER, &req), 1);
    let caps = captures();
    assert_eq!(caps.len(), 1);
    let (_, oper, sha, spa, _, tpa) = arp_fields(&caps[0]);
    assert_eq!(oper, 2, "reply");
    assert_eq!(sha, SENDER_MAC);
    assert_eq!(spa, [10, 0, 0, 9], "answered as the extra address");
    assert_eq!(tpa, [10, 0, 0, 42]);
    assert_no_leak();
}

// ---------------------------------------------------------------------------
// bind_addr_and_port
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn bind_addr_and_port_formats_local_identity() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    assert_eq!(h.bind_addr_and_port(), "10.0.0.1:40000");
}

/// A next hop that never answers ARP is reported, as Linux reports a
/// NUD_FAILED neighbour: sends return 0 while the first resolution runs (3 s),
/// then -1 with EHOSTUNREACH once per failure episode, after which its
/// datagrams are dropped silently (as an unconnected sendto succeeds, so Aeron
/// does not log an error per send); it keeps being probed once a second and
/// recovers on the first answer.
#[test]
#[serial]
fn unanswered_arp_fails_with_ehostunreach_then_recovers() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    const PEER_MAC: [u8; 6] = [0x02, 0, 0, 0, 0, 0x50];
    pin_clock_ms(1_000);
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"x"]).0, 0, "resolving");
    pin_clock_ms(3_999);
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"x"]).0, 0, "still resolving");
    pin_clock_ms(4_000);
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"x"]).0, -1, "3 s unanswered");
    assert!(last_error().contains("unreachable"), "{}", last_error());
    let probes = captures().iter().filter(|c| eth_type(c) == 0x0806).count();
    pin_clock_ms(5_000);
    let before = captures().iter().filter(|c| eth_type(c) == 0x0800).count();
    assert_eq!(
        h.send_to(([10, 0, 0, 5], 40123), &[b"x"]).0,
        1,
        "reported once: now dropped silently"
    );
    assert_eq!(
        captures().iter().filter(|c| eth_type(c) == 0x0800).count(),
        before,
        "nothing sent"
    );
    assert_eq!(
        captures().iter().filter(|c| eth_type(c) == 0x0806).count(),
        probes + 1,
        "re-probed each second"
    );

    let reply = arp_reply(SENDER_MAC, PEER_MAC, [10, 0, 0, 5], SENDER_IP);
    assert_eq!(h.arp_rx(SENDER, &reply), 1, "an answer to a failed entry is learned");
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[b"x"]).0, 1, "recovered");
    assert_no_leak();
}

/// On a NIC without checksum offload (a vdev here) the transport completes
/// the IPv4 and UDP checksums itself: a kernel peer verifies every frame. It
/// never sends the pseudo-header seed.
#[test]
#[serial]
fn software_checksums_when_the_nic_cannot_offload() {
    let mut config = rusteron_dpdk_config_t::valid();
    fill_cstr(&mut config.sender_pci, "net_tap0");
    fill_cstr(&mut config.receiver_pci, "net_tap1");
    let mut h = Harness::new_with_config(config, |env| env.set_csum_ok(false));
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    for payload in [&b"odd-length"[..], b"even"] {
        assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[payload]).0, 1);
    }
    for c in captures() {
        let f = cap_bytes(&c);
        assert_eq!(c.ol_flags, 0, "no offload requested");
        assert_eq!(fold(&f[14..34]), 0xFFFF, "valid IPv4 checksum");
        let udp_len = u16::from_be_bytes([f[38], f[39]]) as usize;
        let mut pseudo = Vec::new();
        pseudo.extend_from_slice(&f[26..34]);
        pseudo.extend_from_slice(&[0, 17]);
        pseudo.extend_from_slice(&(udp_len as u16).to_be_bytes());
        pseudo.extend_from_slice(&f[34..34 + udp_len]);
        assert_eq!(fold(&pseudo), 0xFFFF, "a kernel peer's UDP checksum check passes");
    }
    assert_no_leak();
}

/// The one's-complement sum of a byte string, folded.
fn fold(bytes: &[u8]) -> u16 {
    let mut sum: u32 = bytes
        .chunks(2)
        .map(|c| (((c[0] as u16) << 8) | *c.get(1).unwrap_or(&0) as u16) as u32)
        .sum();
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    sum as u16
}

/// ARP replies are rate-limited per NIC: a flood of requests for our address
/// costs at most 1000 transmits a second on the agent, and the limit resets
/// with the next second.
#[test]
#[serial]
fn arp_replies_are_rate_limited() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    pin_clock_ms(1_000);
    let req = arp_request([0x02, 0, 0, 0, 0, 0x42], [10, 0, 0, 42], SENDER_IP);
    let answered = (0..1001).filter(|_| h.arp_rx(SENDER, &req) == 1).count();
    assert_eq!(answered, 1000, "at most 1000 a second");
    pin_clock_ms(2_000);
    assert_eq!(h.arp_rx(SENDER, &req), 1, "a new second, a new allowance");
    assert_no_leak();
}

/// The ARP table never fills for good: a next hop finding no room is reported
/// once (ENOBUFS) and later ones are dropped, never an error per send; a
/// next hop idle for 60 s gives its slot to a new one. Next hops in use are
/// never replaced.
#[test]
#[serial]
fn arp_table_full_is_reported_once_and_idle_entries_are_replaced() {
    let mut config = rusteron_dpdk_config_t::valid();
    config.sender_prefix_len = 16; // room for more next hops than the table holds
    let mut h = Harness::new_with_config(config, |_| {});
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    pin_clock_ms(1_000);
    let ip = |n: u32| [10, 0, (2 + n / 250) as u8, (1 + n % 250) as u8];
    let mut first_full = None;
    let mut errors = 0;
    let mut dropped = 0;
    for n in 0..4000 {
        match h.send_to((ip(n), 40123), &[b"x"]).0 {
            -1 => {
                errors += 1;
                assert!(last_error().contains("ARP table full"), "{}", last_error());
                first_full.get_or_insert(n);
            }
            1 => dropped += 1,
            0 => {}
            rc => panic!("unexpected send result {rc}"),
        }
    }
    let first_full = first_full.expect("the table filled");
    assert_eq!(errors, 1, "reported once");
    assert!(dropped > 0, "later next hops without room are dropped");

    // A minute on, every entry is idle: the one that found no room gets a
    // slot and starts resolving (0: retry), as any new next hop.
    pin_clock_ms(62_000);
    assert_eq!(
        h.send_to((ip(first_full), 40123), &[b"x"]).0,
        0,
        "resolving in a replaced slot"
    );
    assert_no_leak();
}

/// A /31 point-to-point subnet (RFC 3021) has no broadcast address: the
/// other end, all host bits set, is an ordinary destination.
#[test]
#[serial]
fn slash_31_peer_is_a_destination() {
    let mut config = rusteron_dpdk_config_t::valid();
    fill_cstr(&mut config.sender_ipv4, "10.0.0.0");
    config.sender_prefix_len = 31;
    fill_cstr(&mut config.sender_gateway, "10.0.0.1");
    let mut h = Harness::new_with_config(config, |_| {});
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        [10, 0, 0, 0],
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 1], [0x02, 0, 0, 0, 0, 0x50]);
    let (rc, _) = h.send_to(([10, 0, 0, 1], 40123), &[b"x"]);
    assert_eq!(rc, 1, "{}", last_error());
    assert_no_leak();
}

/// RUSTERON_DPDK_TX_RATE_MBPS caps a NIC's transmit rate below the cloud's
/// per-VM bandwidth allowance with a token bucket: sends within it go out at
/// once; one that would overdraw past the spin limit sends nothing (a zero
/// result Aeron retries whole), counted as tx-paced, and goes out once the
/// allowance has refilled.
#[test]
#[serial]
fn bandwidth_allowance_defers_sends_beyond_it() {
    let _rate = EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", "8"); // 1000 bytes a millisecond
    let _bucket = EnvGuard::set("RUSTERON_DPDK_TX_BUCKET_BYTES", "1500");
    let _spin = EnvGuard::set("RUSTERON_DPDK_TX_PACE_SPIN_US", "0");
    let mut h = Harness::new();
    pin_clock_ms(1_000); // after the harness's reset
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    let payload = vec![7u8; 958]; // 1000 bytes on the wire with the headers
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[&payload]).0, 1, "within the bucket");
    assert_eq!(
        h.send_to(([10, 0, 0, 5], 40123), &[&payload]).0,
        1,
        "the last token: may overdraw"
    );
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[&payload]).0, 0, "in debt: deferred");
    assert_eq!(captures().len(), 2);
    pin_clock_ms(1_002); // two milliseconds: 2000 bytes of allowance back, to the 1500 bucket
    assert_eq!(h.send_to(([10, 0, 0, 5], 40123), &[&payload]).0, 1, "refilled");
    assert_eq!(captures().len(), 3);
    assert_no_leak();
}

/// An allowance is credited for all the time between refills, however often
/// they come: a sender retrying a deferred send more often than its
/// allowance earns a whole byte (Aeron retries at its next duty cycle) still
/// sends as soon as the debt is repaid, instead of losing each retry's
/// fraction of a byte and never refilling.
#[test]
#[serial]
fn an_allowance_refills_however_often_a_send_is_retried() {
    let _rate = EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", "8"); // a byte a microsecond
    let _bucket = EnvGuard::set("RUSTERON_DPDK_TX_BUCKET_BYTES", "1500");
    let _spin = EnvGuard::set("RUSTERON_DPDK_TX_PACE_SPIN_US", "0");
    let mut h = Harness::new();
    const T0: u64 = 1_000_000_000;
    pin_clock_ns(T0); // after the harness's reset
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    let payload = vec![7u8; 958]; // 1000 bytes on the wire with the headers
    let dst = ([10, 0, 0, 5], 40123);
    assert_eq!(h.send_to(dst, &[&payload]).0, 1, "within the bucket");
    assert_eq!(h.send_to(dst, &[&payload]).0, 1, "the last token: 500 owed");
    // retried every 500 ns: half a byte of allowance back each time
    let mut sent_after_ns = None;
    for retry in 1..=2_000u64 {
        pin_clock_ns(T0 + retry * 500);
        if 1 == h.send_to(dst, &[&payload]).0 {
            sent_after_ns = Some(retry * 500);
            break;
        }
    }
    assert_eq!(sent_after_ns, Some(500_000), "the 500 bytes owed repaid in 500 µs");
    assert_eq!(captures().len(), 3);
    assert_no_leak();
}

/// A clock read behind an allowance's last refill (a counter read a little
/// behind on another core, a recalibration) credits nothing, and the refill
/// stays at the furthest time it has credited, so no interval is credited
/// twice: the debt is repaid only as the clock passes it.
#[test]
#[serial]
fn a_clock_read_behind_the_last_refill_credits_nothing() {
    let _rate = EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", "8"); // a byte a microsecond
    let _bucket = EnvGuard::set("RUSTERON_DPDK_TX_BUCKET_BYTES", "1500");
    let _spin = EnvGuard::set("RUSTERON_DPDK_TX_PACE_SPIN_US", "0");
    let mut h = Harness::new();
    const T0: u64 = 1_000_000_000;
    pin_clock_ns(T0); // after the harness's reset
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    let payload = vec![7u8; 958]; // 1000 bytes on the wire with the headers
    let dst = ([10, 0, 0, 5], 40123);
    assert_eq!(h.send_to(dst, &[&payload]).0, 1, "within the bucket");
    assert_eq!(h.send_to(dst, &[&payload]).0, 1, "the last token: 500 owed");
    pin_clock_ns(T0 - 1_000_000); // a millisecond behind the last refill
    assert_eq!(h.send_to(dst, &[&payload]).0, 0, "a step back credits nothing");
    pin_clock_ns(T0 + 400_000); // 400 of the 500 bytes owed back
    assert_eq!(
        h.send_to(dst, &[&payload]).0,
        0,
        "credited from the last refill, not from the step back"
    );
    pin_clock_ns(T0 + 500_000);
    assert_eq!(h.send_to(dst, &[&payload]).0, 1, "repaid");
    // the datagrams (the step back also has ARP refresh the next hop)
    assert_eq!(captures().iter().filter(|c| eth_type(c) == 0x0800).count(), 3);
    assert_no_leak();
}

/// RUSTERON_DPDK_TX_ROUTE_MBPS gives a destination prefix its own allowance
/// (a slower link on the way, such as a direct link to another site), on top
/// of the NIC's: the longest matching prefix applies, so one destination is
/// held to its slow link while another, on a faster route, is not.
#[test]
#[serial]
fn route_allowances_hold_each_destination_to_its_own_link() {
    let _routes = EnvGuard::set("RUSTERON_DPDK_TX_ROUTE_MBPS", "10.0.0.0/24=8000, 10.0.0.5/32=8");
    // the NIC's off: its default allowance, in a 1500-byte bucket, would hold the fast route too
    let _rate = EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", "0");
    let _bucket = EnvGuard::set("RUSTERON_DPDK_TX_BUCKET_BYTES", "1500");
    let _spin = EnvGuard::set("RUSTERON_DPDK_TX_PACE_SPIN_US", "0");
    let mut h = Harness::new();
    pin_clock_ms(1_000);
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    h.arp_seed([10, 0, 0, 6], [0x02, 0, 0, 0, 0, 0x60]);
    let payload = vec![7u8; 958]; // 1000 bytes on the wire
    let slow = ([10, 0, 0, 5], 40123);
    let fast = ([10, 0, 0, 6], 40123);
    assert_eq!(h.send_to(slow, &[&payload]).0, 1);
    assert_eq!(h.send_to(slow, &[&payload]).0, 1, "overdraws its 8 Mbit/s link");
    assert_eq!(h.send_to(slow, &[&payload]).0, 0, "held to its link");
    // its own bucket: unaffected by the slow link's debt
    assert_eq!(h.send_to(fast, &[&payload]).0, 1, "a faster route is not held");
    pin_clock_ms(1_002);
    assert_eq!(h.send_to(slow, &[&payload]).0, 1, "its link's allowance refilled");
    assert_no_leak();
}

/// Frames queued behind a full ring are charged once, as they will be sent;
/// a vector refused while they wait is not charged at all (Aeron offers it
/// again), so a full ring never holds the NIC below its allowance.
#[test]
#[serial]
fn refused_frames_are_not_charged_to_the_allowance() {
    let _rate = EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", "8"); // 1000 bytes a millisecond
    let _bucket = EnvGuard::set("RUSTERON_DPDK_TX_BUCKET_BYTES", "1500");
    let _spin = EnvGuard::set("RUSTERON_DPDK_TX_PACE_SPIN_US", "0");
    let mut h = Harness::new();
    pin_clock_ms(1_000); // after the harness's reset
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    let payload = vec![7u8; 958]; // 1000 bytes on the wire with the headers
    let dst = ([10, 0, 0, 5], 40123);
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(0) }; // a full ring
    assert_eq!(h.send_to(dst, &[&payload]).0, 1, "queued, charged once");
    for _ in 0..3 {
        assert_eq!(
            h.send_to(dst, &[&payload]).0,
            0,
            "refused whole while it waits: not charged"
        );
    }
    unsafe { rusteron_dpdk_fake_set_tx_burst_cap(64) };
    assert_eq!(
        h.send_to(dst, &[&payload]).0,
        1,
        "the queued frame goes, then this one on the last token"
    );
    assert_eq!(h.send_to(dst, &[&payload]).0, 0, "in debt: deferred");
    assert_eq!(captures().len(), 2);
    assert_no_leak();
}

/// An Aeron frame of `frame_type` for `stream`, `len` bytes with its header:
/// a data frame (1) carries the stream id at offset 16.
fn aeron_frame(frame_type: u16, stream: i32, len: usize) -> Vec<u8> {
    let mut f = vec![7u8; len];
    f[0..4].copy_from_slice(&i32::try_from(len).unwrap().to_le_bytes());
    f[4] = 0; // version
    f[5] = 0xc0; // begin and end of a message
    f[6..8].copy_from_slice(&frame_type.to_le_bytes());
    f[16..20].copy_from_slice(&stream.to_le_bytes());
    f
}

/// RUSTERON_DPDK_TX_LOW_STREAMS puts streams that can wait (bulk) below the
/// rest under a bandwidth allowance: they send only from what leaves
/// RUSTERON_DPDK_TX_LOW_RESERVE_BYTES in the bucket for the others, never
/// overdraw it and never wait for it (a zero result Aeron retries), so a
/// latency-critical stream never queues behind them. Control frames are
/// never low, whatever stream they name.
#[test]
#[serial]
fn low_priority_streams_leave_the_reserve_to_the_rest() {
    let _rate = EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", "8"); // 1000 bytes a millisecond
    let _bucket = EnvGuard::set("RUSTERON_DPDK_TX_BUCKET_BYTES", "4000");
    let _low = EnvGuard::set("RUSTERON_DPDK_TX_LOW_STREAMS", "3003, 5000-5999");
    let _reserve = EnvGuard::set("RUSTERON_DPDK_TX_LOW_RESERVE_BYTES", "2000");
    // a millisecond's wait allowed: a low stream that waited (on the pinned
    // clock, out to the spin bound) would then send, so a 0 shows it did not
    let _spin = EnvGuard::set("RUSTERON_DPDK_TX_PACE_SPIN_US", "1000");
    let mut h = Harness::new();
    pin_clock_ms(1_000); // after the harness's reset
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    let dst = ([10, 0, 0, 5], 40123);
    // 1000 bytes on the wire each, with the Ethernet, IPv4 and UDP headers
    let bulk = aeron_frame(1, 3003, 958);
    let bulk_range = aeron_frame(1, 5500, 958);
    let orders = aeron_frame(1, 3001, 958);
    let status = aeron_frame(3, 3003, 958); // a status message, never low

    assert_eq!(h.send_to(dst, &[&bulk_range]).0, 1, "4000 in the bucket: 3000 left");
    assert_eq!(h.send_to(dst, &[&bulk]).0, 1, "2000 left: the reserve, untouched");
    assert_eq!(
        h.send_to(dst, &[&bulk]).0,
        0,
        "would dip into the reserve: deferred, not waited for"
    );
    assert_eq!(h.send_to(dst, &[&status]).0, 1, "control frames draw on the reserve");
    assert_eq!(h.send_to(dst, &[&orders]).0, 1, "so do the other streams");
    assert_eq!(h.send_to(dst, &[&orders]).0, 1, "and may overdraw the last of it");
    assert_eq!(h.send_to(dst, &[&bulk]).0, 0, "in debt: low streams wait their turn");
    assert_eq!(captures().len(), 5);
    pin_clock_ms(1_005); // refilled to the full 4000
    assert_eq!(h.send_to(dst, &[&bulk]).0, 1, "refilled: low streams send again");
    assert_eq!(captures().len(), 6);
    assert_no_leak();
}

/// A full bucket always lets a low-priority send through, so a batch larger
/// than the bucket less the reserve is held back, never starved.
#[test]
#[serial]
fn low_priority_streams_send_from_a_full_bucket() {
    let _rate = EnvGuard::set("RUSTERON_DPDK_TX_RATE_MBPS", "8");
    let _bucket = EnvGuard::set("RUSTERON_DPDK_TX_BUCKET_BYTES", "1500");
    let _low = EnvGuard::set("RUSTERON_DPDK_TX_LOW_STREAMS", "3003");
    let _reserve = EnvGuard::set("RUSTERON_DPDK_TX_LOW_RESERVE_BYTES", "1000");
    let _spin = EnvGuard::set("RUSTERON_DPDK_TX_PACE_SPIN_US", "0");
    let mut h = Harness::new();
    pin_clock_ms(1_000);
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        SENDER_IP,
        40000,
        None,
        1472,
    );
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    let dst = ([10, 0, 0, 5], 40123);
    let bulk = aeron_frame(1, 3003, 958); // 1000 on the wire: more than 1500 - 1000
    assert_eq!(h.send_to(dst, &[&bulk]).0, 1, "from a full bucket");
    assert_eq!(h.send_to(dst, &[&bulk]).0, 0, "500 left: below the reserve");
    pin_clock_ms(1_001); // back to 1500, full
    assert_eq!(h.send_to(dst, &[&bulk]).0, 1, "full again");
    assert_no_leak();
}
