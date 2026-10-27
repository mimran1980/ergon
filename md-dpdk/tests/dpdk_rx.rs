//! Aeron receive polling and endpoint dispatch over DPDK (plan §7.6/§7.7).
//!
//! Drives the real Aeron poller/recvmmsg bindings against the DPDK-free fakes:
//! every accepted and rejected frame class, endpoint-map dispatch (including a
//! shared ENA polled once), ARP routing from the receive loop, callback-before-
//! recycle ordering, the vlen/burst caps, foreign-destination and unknown-port
//! counting, and zero allocations across repeated empty polls.
//!
//! Linux x86_64 or aarch64; links the same archives as dpdk_tx.rs.
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

use std::os::raw::{c_int, c_void};
use std::ptr;

// The Aeron types and functions of the driver this crate installs into.
use rusteron_media_driver::bindings::*;

// ---------------------------------------------------------------------------
// Native test hooks (fakes + runtime).
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn rusteron_dpdk_transport_bindings() -> *mut aeron_udp_channel_transport_bindings_stct;
    fn rusteron_dpdk_fake_rx_queued(port_id: u16) -> c_int;
    fn rusteron_dpdk_test_reports_formatted() -> c_int;
    fn rusteron_dpdk_rx_ol_flags_from_dpdk(
        ol: u64,
        ip_bad: u64,
        ip_good: u64,
        ip_none: u64,
        l4_bad: u64,
        l4_good: u64,
        l4_none: u64,
    ) -> u32;
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
            sin_addr: u32::from_le_bytes(ip),
            sin_zero: [0; 8],
        }
    }
    fn storage_mut(&mut self) -> *mut sockaddr_storage {
        self as *mut Self as *mut sockaddr_storage
    }
}

/// The C poller writes a sockaddr_in into the sockaddr_storage the callback
/// receives; the octets sit at fixed offsets (family, port, addr, zero).
fn sin_addr_of(addr: *mut sockaddr_storage) -> ([u8; 4], u16) {
    let mut b = [0u8; 128];
    if !addr.is_null() {
        unsafe { ptr::copy_nonoverlapping(addr as *const u8, b.as_mut_ptr(), 128) };
    }
    let port = u16::from_be_bytes([b[2], b[3]]);
    let ip = [b[4], b[5], b[6], b[7]];
    (ip, port)
}

// ---------------------------------------------------------------------------
// Receive callback (clientd carries the Vec; no statics).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct RxDatagram {
    transport: usize,
    data: Vec<u8>,
    src_ip: [u8; 4],
    src_port: u16,
    alloc_at_cb: c_int,
    released_at_cb: c_int,
    /// Frames the fake NIC had transmitted when this datagram was dispatched.
    captures_at_cb: c_int,
}

unsafe extern "C" fn recv_cb(
    _data_paths: *mut aeron_udp_channel_data_paths_t,
    transport: *mut aeron_udp_channel_transport_t,
    receiver_clientd: *mut c_void,
    _endpoint_clientd: *mut c_void,
    _destination_clientd: *mut c_void,
    buffer: *mut u8,
    length: usize,
    addr: *mut sockaddr_storage,
    _media_timestamp: *mut timespec,
) {
    let rx = &mut *(receiver_clientd as *mut Vec<RxDatagram>);
    let data = std::slice::from_raw_parts(buffer, length).to_vec();
    let (src_ip, src_port) = sin_addr_of(addr);
    // Snapshot the fake's mbuf accounting: this proves whether the frame's mbuf
    // is still live (callback before recycle) or already released.
    rx.push(RxDatagram {
        transport: transport as usize,
        data,
        src_ip,
        src_port,
        alloc_at_cb: rusteron_dpdk_fake_allocated(),
        released_at_cb: rusteron_dpdk_fake_released(),
        captures_at_cb: rusteron_dpdk_fake_capture_count(),
    });
}

// ---------------------------------------------------------------------------
// Harness: native runtime + one genuine Aeron transport + the receive Vec.
// ---------------------------------------------------------------------------

struct Harness {
    _env: TestEnv,
    native: *mut c_void,
    bindings: *mut aeron_udp_channel_transport_bindings_stct,
    transport: Box<aeron_udp_channel_transport_stct>,
    rx: Vec<RxDatagram>,
    initialized: bool,
}

impl Harness {
    fn new() -> Self {
        Harness::new_with_burst(rusteron_dpdk_config_t::valid().burst_size)
    }

    fn new_with_burst(burst_size: u16) -> Self {
        let env = TestEnv::new();
        env.eal_skip();
        let mut config = rusteron_dpdk_config_t::valid();
        config.burst_size = burst_size;
        let native = create(&config).unwrap_or_else(|e| panic!("create failed: {e}"));
        let bindings = unsafe { rusteron_dpdk_transport_bindings() };
        assert!(!bindings.is_null());
        Harness {
            _env: env,
            native,
            bindings,
            transport: Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() }),
            rx: Vec::new(),
            initialized: false,
        }
    }

    fn init(&mut self, affinity: aeron_udp_channel_transport_affinity_t, bind_ip: [u8; 4], bind_port: u16, mtu: usize) {
        assert!(!self.initialized, "harness already initialized");
        init_transport(self.bindings, &mut self.transport, affinity, bind_ip, bind_port, mtu);
        self.initialized = true;
    }

    fn stats(&self) -> RxStats {
        let mut s = RxStats::default();
        unsafe { rusteron_dpdk_transport_test_rx_stats(self.native, &mut s) };
        s
    }

    fn poller(&self) -> Poller {
        Poller::new(self.bindings)
    }

    fn recvmmsg(&mut self, vlen: usize, bytes_rcved: &mut i64) -> c_int {
        let mut msgvec = vec![unsafe { std::mem::zeroed::<RealMmsghdr>() }; vlen];
        let clientd = &mut self.rx as *mut Vec<RxDatagram> as *mut c_void;
        unsafe {
            let b = &*self.bindings;
            b.recvmmsg_func.unwrap()(
                &mut *self.transport,
                msgvec.as_mut_ptr() as *mut mmsghdr,
                vlen,
                bytes_rcved,
                Some(recv_cb),
                clientd,
            )
        }
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

fn init_transport(
    bindings: *mut aeron_udp_channel_transport_bindings_stct,
    transport: &mut aeron_udp_channel_transport_stct,
    affinity: aeron_udp_channel_transport_affinity_t,
    bind_ip: [u8; 4],
    bind_port: u16,
    mtu: usize,
) {
    unsafe { ptr::write_bytes(transport as *mut _, 0, 1) };
    let mut bind = SockAddrIn::new(bind_ip, bind_port);
    let mut dummy = SockAddrIn::new([0, 0, 0, 0], 0);
    let mut params: aeron_udp_channel_transport_params_stct = unsafe { std::mem::zeroed() };
    params.mtu_length = mtu;
    let rc = unsafe {
        let b = &*bindings;
        b.init_func.unwrap()(
            transport,
            bind.storage_mut(),
            dummy.storage_mut(),
            ptr::null_mut(),
            &mut params,
            ptr::null_mut(),
            affinity,
        )
    };
    assert_eq!(rc, 0, "init failed: {}", last_error());
}

/// A second Aeron transport for multi-endpoint tests (closed by the caller).
fn init_extra_transport(
    bindings: *mut aeron_udp_channel_transport_bindings_stct,
    bind_ip: [u8; 4],
    bind_port: u16,
) -> Box<aeron_udp_channel_transport_stct> {
    let mut t = Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() });
    init_transport(
        bindings,
        &mut t,
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        bind_ip,
        bind_port,
        1472,
    );
    t
}

// ---------------------------------------------------------------------------
// Poller wrapper over the genuine bindings' poller callbacks.
// ---------------------------------------------------------------------------

struct Poller {
    inner: Box<aeron_udp_transport_poller_stct>,
    bindings: *mut aeron_udp_channel_transport_bindings_stct,
    inited: bool,
    added: Vec<*mut aeron_udp_channel_transport_t>,
}

impl Poller {
    fn new(bindings: *mut aeron_udp_channel_transport_bindings_stct) -> Self {
        Self {
            inner: Box::new(unsafe { std::mem::zeroed::<aeron_udp_transport_poller_stct>() }),
            bindings,
            inited: false,
            added: Vec::new(),
        }
    }

    fn init(&mut self) {
        if self.inited {
            return;
        }
        let rc = unsafe {
            let b = &*self.bindings;
            b.poller_init_func.unwrap()(
                &mut *self.inner,
                ptr::null_mut(),
                aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
            )
        };
        assert_eq!(rc, 0, "poller_init failed: {}", last_error());
        self.inited = true;
    }

    fn add(&mut self, transport: *mut aeron_udp_channel_transport_t) {
        self.try_add(transport)
            .unwrap_or_else(|e| panic!("poller_add failed: {e}"));
    }

    fn try_add(&mut self, transport: *mut aeron_udp_channel_transport_t) -> Result<(), String> {
        self.init();
        let rc = unsafe {
            let b = &*self.bindings;
            b.poller_add_func.unwrap()(&mut *self.inner, transport)
        };
        if rc == 0 {
            self.added.push(transport);
            Ok(())
        } else {
            Err(last_error())
        }
    }

    fn remove(&mut self, transport: *mut aeron_udp_channel_transport_t) {
        let rc = unsafe {
            let b = &*self.bindings;
            b.poller_remove_func.unwrap()(&mut *self.inner, transport)
        };
        assert_eq!(rc, 0, "poller_remove failed: {}", last_error());
    }

    fn poll(&mut self, vlen: usize, clientd: *mut c_void, bytes_rcved: &mut i64) -> c_int {
        let mut msgvec = vec![unsafe { std::mem::zeroed::<RealMmsghdr>() }; vlen];
        self.poll_with(&mut msgvec, clientd, bytes_rcved)
    }

    /// Poll with a caller-owned msgvec, so a steady-state loop never reallocates
    /// (the msgvec is Aeron's own reusable buffer in production).
    fn poll_with(&mut self, msgvec: &mut [RealMmsghdr], clientd: *mut c_void, bytes_rcved: &mut i64) -> c_int {
        unsafe {
            let b = &*self.bindings;
            b.poller_poll_func.unwrap()(
                &mut *self.inner,
                msgvec.as_mut_ptr() as *mut mmsghdr,
                msgvec.len(),
                bytes_rcved,
                Some(recv_cb),
                None,
                clientd,
            )
        }
    }
}

/// Close an extra transport via the genuine close callback (frees the native
/// client state it allocated in init).
fn close_transport(
    bindings: *mut aeron_udp_channel_transport_bindings_stct,
    transport: &mut Box<aeron_udp_channel_transport_stct>,
) {
    unsafe {
        let b = &*bindings;
        b.close_func.unwrap()(&mut **transport);
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        if !self.inited {
            return;
        }
        unsafe {
            let b = &*self.bindings;
            for t in self.added.drain(..) {
                b.poller_remove_func.unwrap()(&mut *self.inner, t);
            }
            b.poller_close_func.unwrap()(&mut *self.inner);
        }
    }
}

/// Linux x86_64 and aarch64 `struct mmsghdr`, one glibc layout (bindings.rs only declares an opaque stub).
/// The C poller touches msg_name (offset 0) and msg_len (offset 48), both within
/// this 56-byte layout; it is zeroed so msg_name is NULL (the C side falls back
/// to its own source-address buffer).
#[repr(C)]
#[derive(Clone, Copy)]
struct RealMmsghdr {
    msg_hdr: msghdr,
    msg_len: u32,
    _pad: u32,
}

// ---------------------------------------------------------------------------
// Frame helpers (real checksums so rx_ol_flags = 0 exercises software verify).
// ---------------------------------------------------------------------------

const RECEIVER_PORT: u16 = 1;
const RECEIVER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x02];
const RECEIVER_IP: [u8; 4] = [10, 0, 1, 1];
const PEER_IP: [u8; 4] = [10, 0, 1, 5];
const PEER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x50];

// The RX verdict view flags (rusteron_dpdk_packet.h).
const RX_IPV4_GOOD: u32 = 0b0001;
const RX_UDP_GOOD: u32 = 0b0010;
const RX_GOOD: u32 = RX_IPV4_GOOD | RX_UDP_GOOD;
const RX_IPV4_BAD: u32 = 0b0100;
const RX_UDP_BAD: u32 = 0b1000;

fn ones_sum(words: &[u16]) -> u32 {
    let mut sum: u32 = 0;
    for w in words {
        sum += *w as u32;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    sum
}

/// IPv4 header checksum over the 20-byte header (checksum field read as zero).
fn ip_csum(hdr: &[u8]) -> u16 {
    let mut words = Vec::with_capacity(10);
    for i in 0..10 {
        words.push(((hdr[i * 2] as u16) << 8) | hdr[i * 2 + 1] as u16);
    }
    !(ones_sum(&words) as u16)
}

/// The internet checksum (RFC 1071) of any byte string; 0 over a valid
/// header or message that includes its own checksum field.
fn inet_csum(bytes: &[u8]) -> u16 {
    let words: Vec<u16> = bytes
        .chunks(2)
        .map(|c| ((c[0] as u16) << 8) | *c.get(1).unwrap_or(&0) as u16)
        .collect();
    !(ones_sum(&words) as u16)
}

/// UDP checksum including the pseudo-header (UDP checksum field zeroed).
fn udp_csum(src_ip: [u8; 4], dst_ip: [u8; 4], sport: u16, dport: u16, payload: &[u8]) -> u16 {
    let udp_len = 8 + payload.len();
    let mut words = Vec::new();
    for i in 0..2 {
        words.push(((src_ip[i * 2] as u16) << 8) | src_ip[i * 2 + 1] as u16);
        words.push(((dst_ip[i * 2] as u16) << 8) | dst_ip[i * 2 + 1] as u16);
    }
    words.push(17); // IPPROTO_UDP
    words.push(udp_len as u16); // pseudo-header length
    words.push(sport);
    words.push(dport);
    words.push(udp_len as u16); // UDP length field
    words.push(0); // checksum field (zeroed during computation)
    for c in payload.chunks(2) {
        if c.len() == 2 {
            words.push(((c[0] as u16) << 8) | c[1] as u16);
        } else {
            words.push((c[0] as u16) << 8);
        }
    }
    let c = !(ones_sum(&words) as u16);
    if c == 0 { 0xFFFF } else { c }
}

/// A valid IPv4/UDP frame: real IPv4 + UDP checksums, DF set, unicast MACs.
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
    f.extend_from_slice(&((20 + 8 + payload.len()) as u16).to_be_bytes());
    f.extend_from_slice(&[0x00, 0x00]); // id
    f.extend_from_slice(&[0x40, 0x00]); // DF
    f.push(64);
    f.push(17);
    f.extend_from_slice(&[0x00, 0x00]); // csum placeholder
    f.extend_from_slice(&src_ip);
    f.extend_from_slice(&dst_ip);
    let csum = ip_csum(&f[14..34]);
    f[24] = (csum >> 8) as u8;
    f[25] = (csum & 0xFF) as u8;
    f.extend_from_slice(&sport.to_be_bytes());
    f.extend_from_slice(&dport.to_be_bytes());
    f.extend_from_slice(&((8 + payload.len()) as u16).to_be_bytes());
    f.extend_from_slice(&[0x00, 0x00]); // csum placeholder
    let uc = udp_csum(src_ip, dst_ip, sport, dport, payload);
    let n = f.len();
    f[n - 2] = (uc >> 8) as u8;
    f[n - 1] = (uc & 0xFF) as u8;
    f.extend_from_slice(payload);
    f
}

/// A valid frame addressed to the receiver transport's own endpoint.
fn frame_to_receiver(dport: u16, payload: &[u8]) -> Vec<u8> {
    build_udp_frame(RECEIVER_MAC, PEER_MAC, PEER_IP, RECEIVER_IP, 7000, dport, payload)
}

/// ARP request for the receiver's IPv4 (broadcast dst, like a real probe).
fn arp_request_for_receiver(sha: [u8; 6], spa: [u8; 4]) -> Vec<u8> {
    let mut f = Vec::new();
    f.extend_from_slice(&[0xff; 6]);
    f.extend_from_slice(&sha);
    f.extend_from_slice(&[0x08, 0x06]);
    f.extend_from_slice(&1u16.to_be_bytes()); // htype
    f.extend_from_slice(&0x0800u16.to_be_bytes()); // ptype
    f.push(6);
    f.push(4);
    f.extend_from_slice(&1u16.to_be_bytes()); // oper request
    f.extend_from_slice(&sha);
    f.extend_from_slice(&spa);
    f.extend_from_slice(&[0; 6]);
    f.extend_from_slice(&RECEIVER_IP);
    f
}

// ---------------------------------------------------------------------------
// Accepted frames
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn valid_frame_software_checksums_dispatched() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    let mut p = h.poller();
    p.add(&mut *h.transport);

    // rx_ol_flags = 0: the NIC left the verdict unknown, so the transport must
    // software-verify the real IPv4 + UDP checksums (plan §7.6).
    let frame = frame_to_receiver(60000, b"hello rx");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), 0, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    let work = p.poll(16, clientd, &mut bytes);
    assert_eq!(work, 1, "one datagram dispatched");
    assert_eq!(bytes, 8, "bytes_rcved accrues the payload length only");
    assert_eq!(h.rx.len(), 1);
    let d = &h.rx[0];
    assert_eq!(d.transport, &mut *h.transport as *mut _ as usize, "dispatch target");
    assert_eq!(d.data, b"hello rx");
    assert_eq!(d.src_ip, PEER_IP);
    assert_eq!(d.src_port, 7000);
    let s = h.stats();
    assert_eq!(s.accepted, 1);
    assert_eq!(s.foreign_dst, 0);
    assert_eq!(s.unknown_port, 0);
    assert_no_leak();
}

#[test]
#[serial]
fn nic_good_verdict_trusted_skips_software_verify() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    let mut p = h.poller();
    p.add(&mut *h.transport);

    // A frame whose UDP checksum field is NOT a valid final checksum (it carries
    // the TX-offload pseudo-header seed): with NIC GOOD verdicts the transport
    // must trust the NIC and accept it anyway.
    let mut frame = frame_to_receiver(60000, b"trust nic");
    frame[41] ^= 0xFF; // corrupt the UDP checksum field so software verify would reject
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    let work = p.poll(16, clientd, &mut bytes);
    assert_eq!(work, 1, "NIC GOOD verdict is trusted");
    assert_eq!(h.stats().accepted, 1);
    assert_no_leak();
}

#[test]
#[serial]
fn recvmmsg_dispatches_single_transport() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );

    let frame = frame_to_receiver(60000, b"direct recvmmsg");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), 0, 1) },
        0
    );
    let mut bytes = 0i64;
    let work = h.recvmmsg(16, &mut bytes);
    assert_eq!(work, 1);
    assert_eq!(bytes, 15, "payload length only");
    assert_eq!(h.rx.len(), 1);
    assert_eq!(h.rx[0].data, b"direct recvmmsg");
    assert_eq!(h.stats().accepted, 1);
    assert_no_leak();
}

#[test]
#[serial]
fn two_endpoints_same_ena_dispatched_by_map() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    let mut t2 = init_extra_transport(h.bindings, RECEIVER_IP, 60001);
    let mut p = h.poller();
    p.add(&mut *h.transport);
    p.add(&mut *t2);

    // Two frames on the same ENA (receiver port), one per endpoint; the poller
    // must drain the shared port once and dispatch each to its own transport.
    let f1 = frame_to_receiver(60000, b"to-endpoint-a");
    let f2 = frame_to_receiver(60001, b"to-endpoint-b");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, f1.as_ptr(), f1.len(), 0, 1) },
        0
    );
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, f2.as_ptr(), f2.len(), 0, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    let work = p.poll(16, clientd, &mut bytes);
    assert_eq!(work, 2, "both endpoints dispatched in one poll");
    assert_eq!(bytes, 26, "13-byte + 13-byte payloads");
    assert_eq!(h.rx.len(), 2, "exactly one callback per datagram");
    let a = h.rx.iter().find(|d| d.data == b"to-endpoint-a").unwrap();
    let b = h.rx.iter().find(|d| d.data == b"to-endpoint-b").unwrap();
    assert_eq!(a.transport, &mut *h.transport as *mut _ as usize);
    assert_eq!(b.transport, &mut *t2 as *mut _ as usize);
    assert_eq!(h.stats().accepted, 2);
    assert_no_leak();
    close_transport(h.bindings, &mut t2);
}

// ---------------------------------------------------------------------------
// ARP routing from the receive loop
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn arp_request_handled_and_reply_sent() {
    let mut h = Harness::new();
    pin_clock_ms(1000);
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    let mut p = h.poller();
    p.add(&mut *h.transport);

    let req = arp_request_for_receiver(PEER_MAC, PEER_IP);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, req.as_ptr(), req.len(), 0, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    let work = p.poll(16, clientd, &mut bytes);
    assert_eq!(work, 0, "ARP is not a UDP datagram");
    assert_eq!(h.rx.len(), 0, "no Aeron callback for ARP");
    assert_eq!(h.stats().arp, 1, "ARP frames are counted");

    // The handler answered with a unicast ARP reply on the same ENA.
    let n = unsafe { rusteron_dpdk_fake_capture_count() };
    assert_eq!(n, 1, "one ARP reply transmitted");
    let mut c = unsafe { std::mem::zeroed::<FakeCapture>() };
    assert_eq!(unsafe { rusteron_dpdk_fake_capture_at(0, &mut c) }, 0);
    assert_eq!(&c.data[0..6], &PEER_MAC, "reply dst = requester's MAC, not broadcast");
    assert_eq!(&c.data[6..12], &RECEIVER_MAC, "reply src = this role's MAC");
    assert_eq!(c.data[12], 0x08);
    assert_eq!(c.data[13], 0x06, "ARP ethertype");
    // ARP body: oper at +20, sender hw at +22, sender proto at +28, target proto at +38.
    assert_eq!(u16::from_be_bytes([c.data[14 + 6], c.data[14 + 7]]), 2, "oper = reply");
    assert_eq!(&c.data[14 + 8..14 + 14], &RECEIVER_MAC, "reply sender hw = us");
    assert_eq!(&c.data[14 + 14..14 + 18], &RECEIVER_IP, "reply sender proto = us");
    assert_eq!(&c.data[14 + 24..14 + 28], &PEER_IP, "reply target proto = requester");
    assert_no_leak();
}

/// An ICMP echo request from the peer to the receiver's address.
fn icmp_echo_to_receiver() -> Vec<u8> {
    let mut icmp = vec![8u8, 0, 0, 0, 0x12, 0x34, 0x00, 0x01, b'p', b'i', b'n', b'g'];
    let c = inet_csum(&icmp);
    icmp[2..4].copy_from_slice(&c.to_be_bytes());
    let mut f = Vec::new();
    f.extend_from_slice(&RECEIVER_MAC);
    f.extend_from_slice(&PEER_MAC);
    f.extend_from_slice(&[0x08, 0x00, 0x45, 0x00]);
    f.extend_from_slice(&((20 + icmp.len()) as u16).to_be_bytes());
    f.extend_from_slice(&[0, 0, 0, 0, 64, 1, 0, 0]);
    f.extend_from_slice(&PEER_IP);
    f.extend_from_slice(&RECEIVER_IP);
    let c = ip_csum(&f[14..34]);
    f[24..26].copy_from_slice(&c.to_be_bytes());
    f.extend_from_slice(&icmp);
    f
}

/// ARP and ICMP frames that arrive in a burst with data are answered after
/// the burst's data frames are dispatched: a data frame never waits behind
/// the build and transmit of a reply to a frame that came before it.
#[test]
#[serial]
fn control_frames_are_answered_after_the_bursts_data() {
    let mut h = Harness::new();
    pin_clock_ms(1000);
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    let mut p = h.poller();
    p.add(&mut *h.transport);

    let arp = arp_request_for_receiver(PEER_MAC, PEER_IP);
    let echo = icmp_echo_to_receiver();
    let data = frame_to_receiver(60000, b"data");
    for f in [&arp, &echo, &data] {
        assert_eq!(
            unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, f.as_ptr(), f.len(), 0, 1) },
            0
        );
    }
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 1, "one datagram in the burst");
    assert_eq!(h.rx.len(), 1);
    assert_eq!(h.rx[0].data, b"data");
    assert_eq!(h.rx[0].captures_at_cb, 0, "dispatched before either reply was sent");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_capture_count() },
        2,
        "both replied after it"
    );
    let mut c = unsafe { std::mem::zeroed::<FakeCapture>() };
    assert_eq!(unsafe { rusteron_dpdk_fake_capture_at(0, &mut c) }, 0);
    assert_eq!(
        (c.data[12], c.data[13]),
        (0x08, 0x06),
        "the ARP reply first, in arrival order"
    );
    assert_eq!(unsafe { rusteron_dpdk_fake_capture_at(1, &mut c) }, 0);
    assert_eq!((c.data[23], c.data[34]), (1, 0), "then the echo reply");
    assert_eq!((h.stats().arp, h.stats().icmp), (1, 1), "both counted");
    assert_no_leak();
}

// ---------------------------------------------------------------------------
// Rejected frame classes (plan §7.6)
// ---------------------------------------------------------------------------

fn assert_reject(
    h: &mut Harness,
    p: &mut Poller,
    frame: &[u8],
    ol_flags: u32,
    nb_segs: u32,
    stat: impl Fn(&RxStats) -> u64,
) {
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), ol_flags, nb_segs) },
        0,
        "inject failed (frame too long?)"
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    let work = p.poll(16, clientd, &mut bytes);
    assert_eq!(work, 0, "rejected frame must not dispatch");
    assert_eq!(bytes, 0);
    assert_eq!(h.rx.len(), 0, "no callback for a rejected frame");
    let s = h.stats();
    assert_eq!(stat(&s), 1, "the reject bucket must count");
    assert_no_leak();
}

/// Return the poller first so it drops before the harness (its Drop removes the
/// transport from the map while the transport is still alive).
fn reject_setup() -> (Poller, Harness) {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    let mut p = h.poller();
    p.add(&mut *h.transport);
    (p, h)
}

#[test]
#[serial]
fn reject_ipv6() {
    let (mut p, mut h) = reject_setup();
    let mut frame = frame_to_receiver(60000, b"x");
    frame[12] = 0x86;
    frame[13] = 0xDD;
    assert_reject(&mut h, &mut p, &frame, 0, 1, |s| s.ipv6);
}

#[test]
#[serial]
fn reject_vlan() {
    let (mut p, mut h) = reject_setup();
    let mut frame = frame_to_receiver(60000, b"x");
    frame[12] = 0x81;
    frame[13] = 0x00;
    assert_reject(&mut h, &mut p, &frame, 0, 1, |s| s.vlan);
}

#[test]
#[serial]
fn reject_unknown_ethertype() {
    let (mut p, mut h) = reject_setup();
    let mut frame = frame_to_receiver(60000, b"x");
    frame[12] = 0x12;
    frame[13] = 0x34;
    assert_reject(&mut h, &mut p, &frame, 0, 1, |s| s.ethertype);
}

#[test]
#[serial]
fn reject_multicast_mac() {
    let (mut p, mut h) = reject_setup();
    let mut frame = frame_to_receiver(60000, b"x");
    frame[0] = 0x01; // multicast dst MAC (LSB of first octet set)
    assert_reject(&mut h, &mut p, &frame, 0, 1, |s| s.multicast);
}

#[test]
#[serial]
fn reject_multicast_ip() {
    let (mut p, mut h) = reject_setup();
    let frame = build_udp_frame(RECEIVER_MAC, PEER_MAC, PEER_IP, [224, 0, 0, 1], 7000, 60000, b"x");
    assert_reject(&mut h, &mut p, &frame, 0, 1, |s| s.multicast);
}

/// An IPv4 header with options is skipped by its length, as the kernel does,
/// and the datagram delivered; a header length below 20 bytes is malformed.
#[test]
#[serial]
fn ip_options_are_skipped_not_rejected() {
    let (mut p, mut h) = reject_setup();
    let plain = frame_to_receiver(60000, b"opts");
    // Insert one 4-byte option word (NOP NOP NOP EOL) after the 20-byte header.
    let mut frame = plain[..34].to_vec();
    frame.extend_from_slice(&[0x01, 0x01, 0x01, 0x00]);
    frame.extend_from_slice(&plain[34..]);
    frame[14] = 0x46; // IHL = 6
    let total = u16::from_be_bytes([frame[16], frame[17]]) + 4;
    frame[16..18].copy_from_slice(&total.to_be_bytes());
    frame[24] = 0;
    frame[25] = 0;
    let c = inet_csum(&frame[14..38]);
    frame[24..26].copy_from_slice(&c.to_be_bytes());
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), 0, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 1, "delivered past the options");
    assert_eq!(h.rx[0].data, b"opts");

    let mut bad = frame_to_receiver(60000, b"x");
    bad[14] = 0x44; // IHL = 4: malformed
    h.rx.clear();
    assert_reject(&mut h, &mut p, &bad, 0, 1, |s| s.ip_options);
}

/// RFC 768: a UDP checksum that computes to 0 is sent as 0xFFFF. That valid
/// datagram (about 1 in 65535) must be delivered, not rejected.
#[test]
#[serial]
fn udp_checksum_ffff_is_valid() {
    let (mut p, mut h) = reject_setup();
    // Find a payload whose checksum computes to zero (sent as 0xFFFF).
    let mut payload = vec![0u8; 8];
    let mut found = false;
    for x in 0..=u16::MAX {
        payload[6..8].copy_from_slice(&x.to_be_bytes());
        if udp_csum(PEER_IP, RECEIVER_IP, 7000, 60000, &payload) == 0xFFFF {
            found = true;
            break;
        }
    }
    assert!(found);
    let frame = build_udp_frame(RECEIVER_MAC, PEER_MAC, PEER_IP, RECEIVER_IP, 7000, 60000, &payload);
    assert_eq!(&frame[40..42], &[0xFF, 0xFF], "the checksum field is all ones");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), 0, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 1, "a valid datagram");
    assert_eq!(h.stats().checksum, 0);
}

/// A zero UDP checksum (omitted, legal in IPv4) is accepted even when the NIC
/// flags the UDP checksum bad; a real bad checksum the NIC flags is rejected.
#[test]
#[serial]
fn zero_udp_checksum_accepted_despite_a_nic_bad_verdict() {
    let (mut p, mut h) = reject_setup();
    let mut frame = frame_to_receiver(60000, b"nocsum");
    frame[40] = 0;
    frame[41] = 0;
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_UDP_BAD, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);

    let mut bad = frame_to_receiver(60000, b"payload");
    let n = bad.len();
    bad[n - 1] ^= 0xFF;
    h.rx.clear();
    assert_reject(&mut h, &mut p, &bad, RX_UDP_BAD, 1, |s| s.checksum);
}

/// A SETUP from a peer whose MTU exceeds max_aeron_mtu is dropped (its data
/// frames could never arrive whole), so no image forms; one within it passes.
#[test]
#[serial]
fn setup_from_a_peer_with_a_larger_mtu_is_dropped() {
    let (mut p, mut h) = reject_setup();
    let setup = |mtu: u32| {
        let mut f = vec![0u8; 40];
        f[0..4].copy_from_slice(&40u32.to_le_bytes()); // frame length
        f[6..8].copy_from_slice(&0x05u16.to_le_bytes()); // HDR_TYPE_SETUP
        f[12..16].copy_from_slice(&7i32.to_le_bytes()); // session
        f[16..20].copy_from_slice(&1001i32.to_le_bytes()); // stream
        f[32..36].copy_from_slice(&mtu.to_le_bytes());
        f
    };
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    let big = frame_to_receiver(60000, &setup(8192));
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, big.as_ptr(), big.len(), RX_GOOD, 1) },
        0
    );
    assert_eq!(p.poll(16, clientd, &mut bytes), 0, "dropped");
    // Reported by housekeeping (dpdk_health.rs has the text), not the agent.
    assert_eq!(unsafe { rusteron_dpdk_test_reports_formatted() }, 0);
    unsafe { rusteron_dpdk_test_housekeeping_tick(h.native) };
    assert_eq!(unsafe { rusteron_dpdk_test_reports_formatted() }, 1, "reported");
    let ok = frame_to_receiver(60000, &setup(1408));
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, ok.as_ptr(), ok.len(), RX_GOOD, 1) },
        0
    );
    assert_eq!(p.poll(16, clientd, &mut bytes), 1, "within our MTU");
}

/// ICMP for this NIC: echo is answered (ping and health checks work); a
/// datagram to a port nobody listens on gets a port unreachable, rate-limited
/// to 100 per second.
#[test]
#[serial]
fn icmp_echo_answered_and_port_unreachable_sent() {
    let (mut p, mut h) = reject_setup();
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;

    // Echo request 10.0.1.5 -> 10.0.1.1, id 0x1234 seq 1, data "ping".
    let mut icmp = vec![8u8, 0, 0, 0, 0x12, 0x34, 0x00, 0x01, b'p', b'i', b'n', b'g'];
    let c = inet_csum(&icmp);
    icmp[2..4].copy_from_slice(&c.to_be_bytes());
    let mut f = Vec::new();
    f.extend_from_slice(&RECEIVER_MAC);
    f.extend_from_slice(&PEER_MAC);
    f.extend_from_slice(&[0x08, 0x00, 0x45, 0x00]);
    f.extend_from_slice(&((20 + icmp.len()) as u16).to_be_bytes());
    f.extend_from_slice(&[0, 0, 0, 0, 64, 1, 0, 0]);
    f.extend_from_slice(&PEER_IP);
    f.extend_from_slice(&RECEIVER_IP);
    let c = ip_csum(&f[14..34]);
    f[24..26].copy_from_slice(&c.to_be_bytes());
    f.extend_from_slice(&icmp);
    let before = unsafe { rusteron_dpdk_fake_capture_count() };
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, f.as_ptr(), f.len(), 0, 1) },
        0
    );
    assert_eq!(p.poll(16, clientd, &mut bytes), 0);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_capture_count() },
        before + 1,
        "an echo reply"
    );
    let mut reply = unsafe { std::mem::zeroed::<FakeCapture>() };
    unsafe { rusteron_dpdk_fake_capture_at(before, &mut reply) };
    let r = &reply.data;
    assert_eq!(&r[0..6], &PEER_MAC);
    assert_eq!(r[23], 1, "ICMP");
    assert_eq!(&r[26..30], &RECEIVER_IP);
    assert_eq!(&r[30..34], &PEER_IP);
    assert_eq!((r[34], r[35]), (0, 0), "echo reply");
    assert_eq!(&r[38..42], &[0x12, 0x34, 0x00, 0x01], "id and sequence echoed");
    assert_eq!(&r[42..46], b"ping");
    assert_eq!(inet_csum(&r[14..34]), 0, "valid IP checksum");
    assert_eq!(inet_csum(&r[34..46]), 0, "valid ICMP checksum");

    // 150 datagrams to an unbound port: 100 port unreachables, then silence.
    let stray = frame_to_receiver(61000, b"x");
    let start = unsafe { rusteron_dpdk_fake_capture_count() };
    for _ in 0..150 {
        assert_eq!(
            unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, stray.as_ptr(), stray.len(), RX_GOOD, 1) },
            0
        );
        p.poll(16, clientd, &mut bytes);
    }
    assert_eq!(
        unsafe { rusteron_dpdk_fake_capture_count() } - start,
        100,
        "rate-limited to 100/s"
    );
    let mut u = unsafe { std::mem::zeroed::<FakeCapture>() };
    unsafe { rusteron_dpdk_fake_capture_at(start, &mut u) };
    assert_eq!((u.data[34], u.data[35]), (3, 3), "port unreachable");
    assert_eq!(&u.data[42..62], &stray[14..34], "quotes the offending IP header");
    assert_no_leak();
}

#[test]
#[serial]
fn reject_ip_fragment() {
    let (mut p, mut h) = reject_setup();
    let mut frame = frame_to_receiver(60000, b"x");
    frame[20] = 0x20; // MF set
    assert_reject(&mut h, &mut p, &frame, 0, 1, |s| s.fragment);
}

#[test]
#[serial]
fn reject_ip_fragment_offset() {
    let (mut p, mut h) = reject_setup();
    let mut frame = frame_to_receiver(60000, b"x");
    frame[20] = 0x00;
    frame[21] = 0x01; // non-zero fragment offset
    assert_reject(&mut h, &mut p, &frame, 0, 1, |s| s.fragment);
}

#[test]
#[serial]
fn reject_non_udp_protocol() {
    let (mut p, mut h) = reject_setup();
    let mut frame = frame_to_receiver(60000, b"x");
    frame[23] = 6; // TCP
    assert_reject(&mut h, &mut p, &frame, 0, 1, |s| s.protocol);
}

#[test]
#[serial]
fn reject_truncated_short_frame() {
    let (mut p, mut h) = reject_setup();
    let frame = frame_to_receiver(60000, b"x");
    assert_reject(&mut h, &mut p, &frame[..20], 0, 1, |s| s.truncated);
}

#[test]
#[serial]
fn reject_truncated_length_inconsistent() {
    let (mut p, mut h) = reject_setup();
    let mut frame = frame_to_receiver(60000, b"x");
    frame[16] = 0x00;
    frame[17] = 0xC8; // total_len (200) exceeds the real frame length
    assert_reject(&mut h, &mut p, &frame, 0, 1, |s| s.truncated);
}

#[test]
#[serial]
fn reject_bad_ip_checksum() {
    let (mut p, mut h) = reject_setup();
    let mut frame = frame_to_receiver(60000, b"x");
    frame[24] ^= 0xFF; // corrupt the IPv4 header checksum
    assert_reject(&mut h, &mut p, &frame, 0, 1, |s| s.checksum);
}

#[test]
#[serial]
fn reject_bad_udp_checksum() {
    let (mut p, mut h) = reject_setup();
    let mut frame = frame_to_receiver(60000, b"payload");
    let n = frame.len();
    frame[n - 1] ^= 0xFF; // corrupt a payload byte -> UDP checksum invalid
    assert_reject(&mut h, &mut p, &frame, 0, 1, |s| s.checksum);
}

#[test]
#[serial]
fn reject_nic_bad_checksum() {
    let (mut p, mut h) = reject_setup();
    let frame = frame_to_receiver(60000, b"x");
    assert_reject(&mut h, &mut p, &frame, RX_IPV4_BAD, 1, |s| s.checksum);
}

/// One DPDK RX checksum verdict's bit masks (rte_mbuf_core.h). BAD and GOOD
/// are one bit each and NONE is both; the two bits are not adjacent in every
/// DPDK release, so the decode takes the masks rather than a shift.
#[derive(Clone, Copy)]
struct Masks {
    bad: u64,
    good: u64,
    none: u64,
}

impl Masks {
    const fn new(bad: u32, good: u32) -> Self {
        Self {
            bad: 1 << bad,
            good: 1 << good,
            none: (1 << bad) | (1 << good),
        }
    }
    fn encode(self, verdict: u64) -> u64 {
        match verdict {
            0 => 0,
            1 => self.bad,
            2 => self.good,
            3 => self.none,
            _ => panic!("verdict {verdict} is out of range"),
        }
    }
}

fn decode(ol: u64, ip: Masks, l4: Masks) -> u32 {
    unsafe { rusteron_dpdk_rx_ol_flags_from_dpdk(ol, ip.bad, ip.good, ip.none, l4.bad, l4.good, l4.none) }
}

/// DPDK encodes each RX checksum verdict as a two-bit enum, and NONE carries
/// both the GOOD and the BAD bit (rte_mbuf_core.h). Decoding the bits
/// independently classifies a NONE packet as both good and bad, and the BAD
/// half drops it. Decode the enum instead.
///
/// The masks are arguments because their positions moved between DPDK
/// releases: DPDK 23.11 has IP at bits 4 and 7 and L4 at bits 3 and 8, where
/// older releases had two adjacent pairs at 5/6 and 7/8. A shift-based decode
/// reads the wrong bits on 23.11 and classifies every packet whose IP
/// checksum is GOOD as an L4 checksum error.
#[test]
#[serial]
fn a_none_checksum_verdict_is_verified_good_never_bad() {
    const UNKNOWN: u64 = 0;
    const BAD: u64 = 1;
    const GOOD: u64 = 2;
    const NONE: u64 = 3;

    // View bits: IPV4_GOOD 0b0001, UDP_GOOD 0b0010, IPV4_BAD 0b0100, UDP_BAD 0b1000.
    let cases = [
        (UNKNOWN, UNKNOWN, 0b0000),
        (GOOD, UNKNOWN, 0b0001),
        (BAD, UNKNOWN, 0b0100),
        (NONE, UNKNOWN, 0b0001), // NONE is verified good: bit-wise reads made it 0b0101
        (UNKNOWN, GOOD, 0b0010),
        (UNKNOWN, BAD, 0b1000),
        (UNKNOWN, NONE, 0b0010), // NONE is verified good: bit-wise reads made it 0b1010
        (NONE, NONE, 0b0011),
        (BAD, BAD, 0b1100),
        (GOOD, GOOD, 0b0011),
    ];
    // Both a current DPDK layout and the older adjacent one: the decode must
    // not care where the bits are.
    let layouts = [
        ("dpdk 23.11", Masks::new(4, 7), Masks::new(3, 8)),
        ("older dpdk", Masks::new(5, 6), Masks::new(7, 8)),
    ];
    for (name, ip, l4) in layouts {
        for (ipv, l4v, want) in cases {
            let ol = ip.encode(ipv) | l4.encode(l4v);
            assert_eq!(decode(ol, ip, l4), want, "{name}: ol_flags {ol:#x} (ip {ipv} l4 {l4v})");
        }
        // A verified NONE packet must never come out with a BAD half: that
        // half is what the classifier drops on.
        for (label, one) in [("ip", ip), ("l4", l4)] {
            let got = decode(one.encode(NONE), ip, l4);
            assert_eq!(got & (RX_IPV4_BAD | RX_UDP_BAD), 0, "{name} {label}: {got:#x}");
        }
        // UNKNOWN is the only verdict that sends a checksum to software
        // verify: it alone leaves its GOOD bit clear.
        assert_eq!(decode(0, ip, l4), 0, "{name}");
    }

    // DPDK 23.11 specifically: a packet whose IP checksum is verified good is
    // not an L4 checksum error. Reading the verdict as adjacent bits puts the
    // IP GOOD bit in the L4 half, where it counts as BAD and the frame is
    // dropped.
    let ip = Masks::new(4, 7);
    let l4 = Masks::new(3, 8);
    assert_eq!(decode(ip.good, ip, l4), RX_IPV4_GOOD, "IP GOOD alone");
    assert_eq!(decode(l4.good, ip, l4), RX_UDP_GOOD, "L4 GOOD alone");
}

#[test]
#[serial]
fn reject_multi_segment() {
    let (mut p, mut h) = reject_setup();
    let frame = frame_to_receiver(60000, b"x");
    assert_reject(&mut h, &mut p, &frame, 0, 2, |s| s.multi_segment);
}

// ---------------------------------------------------------------------------
// Dispatch statistics and lifecycle
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn foreign_dst_and_unknown_port_counted_not_dispatched() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    let mut p = h.poller();
    p.add(&mut *h.transport);

    // Valid frame for a foreign destination IP on this NIC.
    let foreign = build_udp_frame(RECEIVER_MAC, PEER_MAC, PEER_IP, [10, 0, 1, 99], 7000, 60000, b"f");
    // Valid frame for our IP but an unregistered port.
    let unknown = frame_to_receiver(60099, b"u");

    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, foreign.as_ptr(), foreign.len(), 0, 1) },
        0
    );
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, unknown.as_ptr(), unknown.len(), 0, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    let work = p.poll(16, clientd, &mut bytes);
    assert_eq!(work, 0);
    assert_eq!(bytes, 0);
    assert_eq!(h.rx.len(), 0);
    let s = h.stats();
    assert_eq!(s.foreign_dst, 1, "wrong-destination IP counted separately");
    assert_eq!(s.unknown_port, 1, "unregistered port on our IP counted separately");
    assert_eq!(s.accepted, 0);
    assert_no_leak();
}

#[test]
#[serial]
fn callback_before_recycle_ordering() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    let mut p = h.poller();
    p.add(&mut *h.transport);

    // Two frames in one burst: rx_burst allocates both views up front, so the
    // first callback must observe the second mbuf still live, and neither frame
    // may be released before its own callback runs (plan §7.6).
    let f1 = frame_to_receiver(60000, b"first");
    let f2 = frame_to_receiver(60000, b"second");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, f1.as_ptr(), f1.len(), 0, 1) },
        0
    );
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, f2.as_ptr(), f2.len(), 0, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    // counted from just before the poll
    let (allocated, released) = unsafe { (rusteron_dpdk_fake_allocated(), rusteron_dpdk_fake_released()) };
    let work = p.poll(16, clientd, &mut bytes);
    assert_eq!(work, 2);
    assert_eq!(h.rx.len(), 2);

    let (a, b) = (&h.rx[0], &h.rx[1]);
    assert_eq!(a.data, b"first");
    assert_eq!(b.data, b"second");
    // Both views were allocated before any callback ran.
    assert_eq!(
        a.alloc_at_cb - allocated,
        2,
        "frame 1's mbuf is live during frame 0's callback"
    );
    assert_eq!(
        a.released_at_cb - released,
        0,
        "frame 0 recycled strictly after its callback"
    );
    assert_eq!(
        b.released_at_cb - released,
        1,
        "frame 0 recycled before frame 1's callback"
    );
    assert_eq!(b.alloc_at_cb - allocated, 2);
    assert_no_leak();
}

/// Aeron's vlen does not cap a poll of the NIC: every frame is dispatched
/// through the first msgvec entry the moment it is parsed (zero copy), so
/// vlen sizes nothing here, and the sender's control poll (vlen 1) would
/// otherwise drain one frame a duty cycle.
#[test]
#[serial]
fn a_poll_is_not_capped_by_aerons_vlen() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    let mut p = h.poller();
    p.add(&mut *h.transport);

    for payload in [b"one", b"two"] {
        let f = frame_to_receiver(60000, payload);
        assert_eq!(
            unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, f.as_ptr(), f.len(), 0, 1) },
            0
        );
    }
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(1, clientd, &mut bytes), 2, "vlen 1 still drains both");
    assert_eq!(bytes, 6);
    assert_eq!(h.rx.len(), 2);
    assert_eq!(unsafe { rusteron_dpdk_fake_rx_queued(RECEIVER_PORT) }, 0);

    let f = frame_to_receiver(60000, b"six");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, f.as_ptr(), f.len(), 0, 1) },
        0
    );
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!(bytes, 9, "bytes_rcved accumulates across polls");
    assert_eq!(h.stats().accepted, 3);
    assert_no_leak();
}

fn inject_n(n: usize) {
    for i in 0..n {
        let f = frame_to_receiver(60000, &[b'a' + (i % 26) as u8; 4]);
        assert_eq!(
            unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, f.as_ptr(), f.len(), 0, 1) },
            0
        );
    }
}

/// A full burst means the ring may hold more: the poll keeps draining, a
/// burst at a time, so a backlog a stall left behind clears in one duty
/// cycle, not one burst per cycle while the ring keeps filling.
#[test]
#[serial]
fn a_full_burst_keeps_the_poll_draining() {
    let mut h = Harness::new_with_burst(2);
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    let mut p = h.poller();
    p.add(&mut *h.transport);
    inject_n(5);
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 5, "bursts of 2, 2 and 1 in one poll");
    assert_eq!(h.rx.len(), 5);
    assert_eq!(unsafe { rusteron_dpdk_fake_rx_queued(RECEIVER_PORT) }, 0);
    assert_no_leak();
}

/// By default a poll drains as many bursts as fill the receive ring the
/// ports took (at most 8): unset, min(4096, the NIC's maximum).
#[test]
#[serial]
fn the_default_drain_fills_the_ring_the_ports_took() {
    let env = TestEnv::new();
    env.eal_skip();
    unsafe { rusteron_dpdk_fake_set_desc_limits(64, 64, 0, 0) };
    let mut config = rusteron_dpdk_config_t::valid();
    config.rx_descriptors = 0;
    config.burst_size = 16; // a 64-descriptor ring: 4 bursts
    let native = create(&config).unwrap_or_else(|e| panic!("create failed: {e}"));
    let mut h = Harness {
        _env: env,
        native,
        bindings: unsafe { rusteron_dpdk_transport_bindings() },
        transport: Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() }),
        rx: Vec::new(),
        initialized: false,
    };
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    let mut p = h.poller();
    p.add(&mut *h.transport);
    inject_n(80);
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 64, "four bursts of 16");
    assert_eq!(p.poll(16, clientd, &mut bytes), 16);
    drop(p);
    drop(h);
    assert_no_leak();
}

/// RUSTERON_DPDK_RX_MAX_BURSTS caps the bursts one poll drains; 1 is one
/// burst a poll.
#[test]
#[serial]
fn rx_max_bursts_caps_the_drain() {
    for (max_bursts, first, second) in [("1", 2, 2), ("2", 4, 3)] {
        let _bursts = EnvGuard::set("RUSTERON_DPDK_RX_MAX_BURSTS", max_bursts);
        let mut h = Harness::new_with_burst(2);
        h.init(
            aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
            RECEIVER_IP,
            60000,
            1472,
        );
        let mut p = h.poller();
        p.add(&mut *h.transport);
        inject_n(7);
        let mut bytes = 0i64;
        let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
        assert_eq!(p.poll(16, clientd, &mut bytes), first, "max {max_bursts}: first poll");
        assert_eq!(p.poll(16, clientd, &mut bytes), second, "max {max_bursts}: second poll");
        drop(p);
        drop(h);
        assert_no_leak();
    }
}

#[test]
#[serial]
fn zero_allocations_during_empty_steady_state_poll() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    let mut p = h.poller();
    p.add(&mut *h.transport);

    let before_alloc = unsafe { rusteron_dpdk_fake_allocated() };
    // A caller-owned msgvec (Aeron's reusable buffer in production) so the loop
    // itself performs no allocation: any growth in the mbuf accounting would
    // prove the C hot path allocates.
    let mut msgvec = vec![unsafe { std::mem::zeroed::<RealMmsghdr>() }; 16];
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    for _ in 0..1000 {
        assert_eq!(p.poll_with(&mut msgvec, clientd, &mut bytes), 0);
    }
    assert_eq!(bytes, 0);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_allocated() },
        before_alloc,
        "no mbufs in steady state"
    );
    assert_no_leak();
}

#[test]
#[serial]
fn duplicate_endpoint_bind_rejected_at_init() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    // The same endpoint again fails at once, as a kernel bind() would
    // (EADDRINUSE), not later in poller_add on the agent thread.
    let mut t2 = Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() });
    let mut bind = SockAddrIn::new(RECEIVER_IP, 60000);
    let mut dummy = SockAddrIn::new([0, 0, 0, 0], 0);
    let mut params: aeron_udp_channel_transport_params_stct = unsafe { std::mem::zeroed() };
    params.mtu_length = 1472;
    let rc = unsafe {
        (*h.bindings).init_func.unwrap()(
            &mut *t2,
            bind.storage_mut(),
            dummy.storage_mut(),
            ptr::null_mut(),
            &mut params,
            ptr::null_mut(),
            aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        )
    };
    assert_eq!(rc, -1);
    assert!(last_error().contains("already bound"), "got: {}", last_error());
    let mut p = h.poller();
    p.add(&mut *h.transport);

    // The first registration still dispatches.
    let frame = frame_to_receiver(60000, b"still works");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), 0, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!(h.rx[0].transport, &mut *h.transport as *mut _ as usize);
    assert_no_leak();
}

#[test]
#[serial]
fn remove_unregisters_endpoint() {
    let mut h = Harness::new();
    h.init(
        aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        RECEIVER_IP,
        60000,
        1472,
    );
    // A second endpoint on the same ENA keeps the poller draining the port after
    // the first is removed (an empty poller would return before any rx_burst).
    let mut t2 = init_extra_transport(h.bindings, RECEIVER_IP, 60001);
    let mut p = h.poller();
    p.add(&mut *h.transport);
    p.add(&mut *t2);

    // Before removal: dispatched to the first transport.
    let frame = frame_to_receiver(60000, b"before");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), 0, 1) },
        0
    );
    let mut bytes = 0i64;
    let clientd = &mut h.rx as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!(h.rx.len(), 1);

    // After removal: the same frame is no longer dispatched and is counted as an
    // unknown port (the endpoint map entry is gone).
    p.remove(&mut *h.transport);
    let frame = frame_to_receiver(60000, b"after");
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), 0, 1) },
        0
    );
    assert_eq!(p.poll(16, clientd, &mut bytes), 0);
    assert_eq!(h.rx.len(), 1, "no dispatch after removal");
    let s = h.stats();
    assert_eq!(s.accepted, 1);
    assert_eq!(s.unknown_port, 1, "the unregistered endpoint is counted");
    assert_no_leak();
    close_transport(h.bindings, &mut t2);
}
