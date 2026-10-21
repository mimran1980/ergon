//! Bidirectional Aeron semantics and interoperability over DPDK (plan §8).
//!
//! Drives the real Aeron binding callbacks on both roles of a single native
//! runtime — a sender-affinity transport and a receiver-affinity transport on
//! the same two ENA ports — and asserts the §8 interoperability contract:
//!   * DPDK frames preserve Aeron's UDP payload bytes exactly (a data frame
//!     emitted by the TX path arrives byte-identical at the RX path).
//!   * Connected and unconnected transports coexist and follow their own
//!     addressing.
//!   * bind_addr_and_port reports the effective local IPv4/port: the port the
//!     caller (Aeron's wildcard port manager) bound, or, for a wildcard bind
//!     (port 0), the ephemeral port the transport assigned. Port 0 never
//!     reaches the wire.
//!   * reconnect replaces the copied remote address without retaining a
//!     caller-owned pointer.
//!   * interceptors are reachable: the poller forwards `target->data_paths` to
//!     the recv callback exactly as the kernel socket path does.
//!   * NAK/status/setup/RTT/data/retransmit role frames are opaque UDP payloads
//!     dispatched to the correct endpoint (manual and dynamic MDC); removing an
//!     endpoint stops its dispatch.
//!   * every emitted datagram stays within `max_aeron_mtu` and never fragments
//!     (DF set), with oversized iovecs rejected.
//!
//! Linux x86_64 or aarch64; links the same archives as dpdk_rx.rs/dpdk_tx.rs.
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

use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

// The Aeron types and functions of the driver this crate installs into.
use rusteron_media_driver::bindings::*;

// ---------------------------------------------------------------------------
// Native test hooks (fakes + runtime).
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn rusteron_dpdk_transport_bindings() -> *mut aeron_udp_channel_transport_bindings_stct;
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
            // s_addr is read natively (no ntohl), so the octets must sit in
            // memory in order (see dpdk_tx.rs).
            sin_addr: u32::from_le_bytes(ip),
            sin_zero: [0; 8],
        }
    }
    fn storage_mut(&mut self) -> *mut sockaddr_storage {
        self as *mut Self as *mut sockaddr_storage
    }
}

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
// Receive callback (clientd carries the Vec; records data_paths + target so the
// interop contract is observable from the callback).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct RxDatagram {
    data_paths: usize,
    transport: usize,
    data: Vec<u8>,
    src_ip: [u8; 4],
    src_port: u16,
}

unsafe extern "C" fn recv_cb(
    data_paths: *mut aeron_udp_channel_data_paths_t,
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
    rx.push(RxDatagram {
        data_paths: data_paths as usize,
        transport: transport as usize,
        data,
        src_ip,
        src_port,
    });
}

// ---------------------------------------------------------------------------
// Harness: one native runtime (both ENA ports) + a sender- and receiver-
// affinity Aeron transport, exactly as one media-driver process would own it.
// ---------------------------------------------------------------------------

struct Harness {
    _env: TestEnv,
    native: *mut c_void,
    bindings: *mut aeron_udp_channel_transport_bindings_stct,
    tx: Box<aeron_udp_channel_transport_stct>,
    rx: Box<aeron_udp_channel_transport_stct>,
    data_paths: Box<aeron_udp_channel_data_paths_stct>,
    rx_dgrams: Vec<RxDatagram>,
    tx_inited: bool,
    rx_inited: bool,
}

impl Harness {
    fn new() -> Self {
        Self::with_config(rusteron_dpdk_config_t::valid())
    }

    /// Over `config` instead of the default one.
    fn with_config(config: rusteron_dpdk_config_t) -> Self {
        let env = TestEnv::new();
        env.eal_skip();
        let native = create(&config).unwrap_or_else(|e| panic!("create failed: {e}"));
        let bindings = unsafe { rusteron_dpdk_transport_bindings() };
        assert!(!bindings.is_null());
        Harness {
            _env: env,
            native,
            bindings,
            tx: Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() }),
            rx: Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() }),
            data_paths: Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_data_paths_stct>() }),
            rx_dgrams: Vec::new(),
            tx_inited: false,
            rx_inited: false,
        }
    }

    /// The sender-affinity transport: emits frames on the sender ENA.
    fn init_tx(&mut self, bind_ip: [u8; 4], bind_port: u16, connect: Option<([u8; 4], u16)>, mtu: usize) {
        assert!(!self.tx_inited, "tx already initialized");
        self.tx_inited = true;
        let t = &mut *self.tx;
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
                aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
            )
        };
        assert_eq!(rc, 0, "tx init failed: {}", last_error());
    }

    /// The receiver-affinity transport: receives frames on the receiver ENA.
    fn init_rx(&mut self, bind_ip: [u8; 4], bind_port: u16, mtu: usize) {
        assert!(!self.rx_inited, "rx already initialized");
        self.rx_inited = true;
        let t = &mut *self.rx;
        unsafe { ptr::write_bytes(t as *mut _, 0, 1) };
        let mut bind = SockAddrIn::new(bind_ip, bind_port);
        let mut dummy = SockAddrIn::new([0, 0, 0, 0], 0);
        let mut params: aeron_udp_channel_transport_params_stct = unsafe { std::mem::zeroed() };
        params.mtu_length = mtu;
        let rc = unsafe {
            let b = &*self.bindings;
            b.init_func.unwrap()(
                t,
                bind.storage_mut(),
                dummy.storage_mut(),
                ptr::null_mut(),
                &mut params,
                ptr::null_mut(),
                aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
            )
        };
        assert_eq!(rc, 0, "rx init failed: {}", last_error());
    }

    fn send_to(&mut self, dst: ([u8; 4], u16), payloads: &[&[u8]]) -> (c_int, i64) {
        let mut d = SockAddrIn::new(dst.0, dst.1);
        self.send_common(d.storage_mut(), payloads)
    }

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
                &mut *self.tx,
                addr,
                iovs.as_ptr() as *mut iovec,
                iovs.len(),
                &mut bytes_sent,
            )
        };
        (rc, bytes_sent)
    }

    /// Call reconnect_func with a caller-owned address that is dropped right
    /// after the call — any subsequent connected send that reaches the new
    /// address proves the native side copied it (no pointer retention).
    fn reconnect(&mut self, new_addr: ([u8; 4], u16)) {
        let mut a = SockAddrIn::new(new_addr.0, new_addr.1);
        let rc = unsafe {
            let b = &*self.bindings;
            b.reconnect_func.unwrap()(&mut *self.tx, a.storage_mut())
        };
        assert_eq!(rc, 0, "reconnect failed: {}", last_error());
    }

    fn bind_addr_and_port(
        bindings: *mut aeron_udp_channel_transport_bindings_stct,
        transport: &mut Box<aeron_udp_channel_transport_stct>,
    ) -> String {
        let mut buf = [0 as c_char; 128];
        let rc = unsafe {
            let b = &*bindings;
            b.bind_addr_and_port_func.unwrap()(&mut **transport, buf.as_mut_ptr(), buf.len())
        };
        assert_eq!(rc, 0, "bind_addr_and_port failed: {}", last_error());
        unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }

    fn arp_seed(&self, ip: [u8; 4], mac: [u8; 6]) {
        let ip_str = std::ffi::CString::new(format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])).unwrap();
        let rc = unsafe { rusteron_dpdk_transport_test_arp_seed(self.native, ip_str.as_ptr(), mac.as_ptr()) };
        assert_eq!(rc, 0, "arp_seed failed: {}", last_error());
    }

    fn stats(&self) -> RxStats {
        let mut s = RxStats::default();
        unsafe { rusteron_dpdk_transport_test_rx_stats(self.native, &mut s) };
        s
    }

    fn poller(&self) -> Poller {
        Poller::new(self.bindings)
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        unsafe {
            let b = &*self.bindings;
            if self.rx_inited {
                b.close_func.unwrap()(&mut *self.rx);
            }
            if self.tx_inited {
                b.close_func.unwrap()(&mut *self.tx);
            }
        }
        close(self.native);
    }
}

// ---------------------------------------------------------------------------
// Poller wrapper over the genuine bindings' poller callbacks (as dpdk_rx.rs).
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

    fn add(&mut self, transport: *mut aeron_udp_channel_transport_t) {
        if !self.inited {
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
        let rc = unsafe {
            let b = &*self.bindings;
            b.poller_add_func.unwrap()(&mut *self.inner, transport)
        };
        assert_eq!(rc, 0, "poller_add failed: {}", last_error());
        self.added.push(transport);
    }

    fn remove(&mut self, transport: *mut aeron_udp_channel_transport_t) {
        let rc = unsafe {
            let b = &*self.bindings;
            b.poller_remove_func.unwrap()(&mut *self.inner, transport)
        };
        assert_eq!(rc, 0, "poller_remove failed: {}", last_error());
        self.added.retain(|&t| t != transport);
    }

    fn poll(&mut self, vlen: usize, clientd: *mut c_void, bytes_rcved: &mut i64) -> c_int {
        // Real receive buffers and address slots, as Aeron's agents provide:
        // the kernel path (host-local channels) receives into them.
        let mut bufs = vec![[0u8; 2048]; vlen];
        let mut names = vec![unsafe { std::mem::zeroed::<sockaddr_storage>() }; vlen];
        let mut iovs: Vec<iovec> = bufs
            .iter_mut()
            .map(|b| iovec {
                iov_base: b.as_mut_ptr() as *mut c_void,
                iov_len: b.len(),
            })
            .collect();
        let mut msgvec = vec![unsafe { std::mem::zeroed::<RealMmsghdr>() }; vlen];
        for i in 0..vlen {
            msgvec[i].msg_hdr.msg_iov = &mut iovs[i];
            msgvec[i].msg_hdr.msg_iovlen = 1;
            msgvec[i].msg_hdr.msg_name = &mut names[i] as *mut _ as *mut c_void;
            msgvec[i].msg_hdr.msg_namelen = std::mem::size_of::<sockaddr_storage>() as u32;
        }
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

#[repr(C)]
#[derive(Clone, Copy)]
struct RealMmsghdr {
    msg_hdr: msghdr,
    msg_len: u32,
    _pad: u32,
}

/// A second receiver-affinity transport for multi-endpoint tests (closed by
/// the caller). The local IPv4 must be the receiver role's own IP so the
/// endpoint map keys it alongside the primary rx transport.
fn init_extra_transport(h: &mut Harness, bind_port: u16) -> Box<aeron_udp_channel_transport_stct> {
    let mut t = Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() });
    let t_ptr = &mut *t;
    unsafe { ptr::write_bytes(t_ptr as *mut _, 0, 1) };
    let mut bind = SockAddrIn::new(RECEIVER_IP, bind_port);
    let mut dummy = SockAddrIn::new([0, 0, 0, 0], 0);
    let mut params: aeron_udp_channel_transport_params_stct = unsafe { std::mem::zeroed() };
    params.mtu_length = 1472;
    let rc = unsafe {
        let b = &*h.bindings;
        b.init_func.unwrap()(
            t_ptr,
            bind.storage_mut(),
            dummy.storage_mut(),
            ptr::null_mut(),
            &mut params,
            ptr::null_mut(),
            aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
        )
    };
    assert_eq!(rc, 0, "extra transport init failed: {}", last_error());
    t
}

/// Initialise a transport, returning init's result and the transport.
fn try_init(
    h: &mut Harness,
    affinity: aeron_udp_channel_transport_affinity_t,
    bind: ([u8; 4], u16),
    connect: Option<([u8; 4], u16)>,
    context: *mut aeron_driver_context_t,
) -> (c_int, Box<aeron_udp_channel_transport_stct>) {
    let mut t = Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() });
    let mut b = SockAddrIn::new(bind.0, bind.1);
    let mut dummy = SockAddrIn::new([0, 0, 0, 0], 0);
    let mut conn = connect.map(|(ip, p)| SockAddrIn::new(ip, p));
    let mut params: aeron_udp_channel_transport_params_stct = unsafe { std::mem::zeroed() };
    params.mtu_length = 1408;
    let rc = unsafe {
        let bindings = &*h.bindings;
        bindings.init_func.unwrap()(
            &mut *t,
            b.storage_mut(),
            dummy.storage_mut(),
            conn.as_mut().map_or(ptr::null_mut(), SockAddrIn::storage_mut),
            &mut params,
            context,
            affinity,
        )
    };
    (rc, t)
}

const SND: aeron_udp_channel_transport_affinity_t =
    aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER;
const RCV: aeron_udp_channel_transport_affinity_t =
    aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER;

fn port_of(bound: &str) -> u16 {
    bound.rsplit(':').next().unwrap().parse().unwrap()
}

/// Another sender-affinity transport on the sender NIC, bound to `bind_port`.
fn init_extra_tx(h: &mut Harness, bind_port: u16) -> Box<aeron_udp_channel_transport_stct> {
    let mut t = Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() });
    let mut bind = SockAddrIn::new(SENDER_IP, bind_port);
    let mut dummy = SockAddrIn::new([0, 0, 0, 0], 0);
    let mut params: aeron_udp_channel_transport_params_stct = unsafe { std::mem::zeroed() };
    params.mtu_length = 1472;
    let rc = unsafe {
        let b = &*h.bindings;
        b.init_func.unwrap()(
            &mut *t,
            bind.storage_mut(),
            dummy.storage_mut(),
            ptr::null_mut(),
            &mut params,
            ptr::null_mut(),
            aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
        )
    };
    assert_eq!(rc, 0, "extra sender transport init failed: {}", last_error());
    t
}

fn close_transport(
    bindings: *mut aeron_udp_channel_transport_bindings_stct,
    transport: &mut Box<aeron_udp_channel_transport_stct>,
) {
    unsafe {
        let b = &*bindings;
        b.close_func.unwrap()(&mut **transport);
    }
}

// ---------------------------------------------------------------------------
// Frame helpers.
// ---------------------------------------------------------------------------

const RECEIVER_PORT: u16 = 1; // fake port id of the receiver ENA
const SENDER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];
const RECEIVER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x02];
const SENDER_IP: [u8; 4] = [10, 0, 0, 1];
const RECEIVER_IP: [u8; 4] = [10, 0, 1, 1];

const IP: usize = 14;
const UDP: usize = IP + 20;
const PAYLOAD: usize = UDP + 8;

const RX_GOOD: u32 = 0b11; // IPV4_CKSUM_GOOD | UDP_CKSUM_GOOD

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

fn udp_payload(c: &FakeCapture) -> &[u8] {
    &c.data[PAYLOAD..c.len as usize]
}

fn u32_ip_at(c: &FakeCapture, off: usize) -> [u8; 4] {
    c.data[off..off + 4].try_into().unwrap()
}

fn u16_at(c: &FakeCapture, off: usize) -> u16 {
    u16::from_be_bytes([c.data[off], c.data[off + 1]])
}

fn ip_flags_frag(c: &FakeCapture) -> u16 {
    u16_at(c, IP + 6)
}

/// A plausible Aeron data frame (little-endian header, type DATA). The
/// transport treats it as opaque bytes; the layout matches aeron's wire format
/// so an unmodified peer could decode it.
fn aeron_data_frame(payload: &[u8]) -> Vec<u8> {
    let mut f = Vec::new();
    let frame_len = (24 + payload.len()) as u16;
    f.extend_from_slice(&frame_len.to_le_bytes()); // frame_length
    f.push(0); // version
    f.push(0x18); // flags: BEGIN | END
    f.extend_from_slice(&0x0001u16.to_le_bytes()); // type = DATA
    f.extend_from_slice(&0x0000_0123i32.to_le_bytes()); // session_id
    f.extend_from_slice(&0x0000_0065i32.to_le_bytes()); // stream_id
    f.extend_from_slice(&0x0000_1000i32.to_le_bytes()); // term_id
    f.extend_from_slice(&0i32.to_le_bytes()); // reserved_value
    f.extend_from_slice(&24u32.to_le_bytes()); // data_offset
    f.extend_from_slice(payload);
    f
}

/// A plausible Aeron control frame (NAK/SM/SETUP/RTT/ERRT) with the given type.
fn aeron_control_frame(frame_type: u16, payload: &[u8]) -> Vec<u8> {
    let mut f = Vec::new();
    let frame_len = (24 + payload.len()) as u16;
    f.extend_from_slice(&frame_len.to_le_bytes());
    f.push(0);
    f.push(0);
    f.extend_from_slice(&frame_type.to_le_bytes());
    f.extend_from_slice(&0i32.to_le_bytes()); // session_id
    f.extend_from_slice(&0i32.to_le_bytes()); // stream_id
    f.extend_from_slice(&0i32.to_le_bytes()); // term_id
    f.extend_from_slice(&0i32.to_le_bytes()); // reserved_value
    f.extend_from_slice(&24u32.to_le_bytes()); // data_offset
    f.extend_from_slice(payload);
    f
}

/// A valid IPv4/UDP frame with the given payload (real checksums so the
/// software-verify path in packet.c accepts it).
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
    f.extend_from_slice(&[0x00, 0x00]);
    f.extend_from_slice(&[0x40, 0x00]); // DF
    f.push(64);
    f.push(17);
    f.extend_from_slice(&[0x00, 0x00]);
    f.extend_from_slice(&src_ip);
    f.extend_from_slice(&dst_ip);
    // IPv4 header checksum over bytes 14..34.
    let mut sum: u32 = 0;
    for i in 0..10 {
        let w = ((f[14 + i * 2] as u16) << 8) | f[14 + i * 2 + 1] as u16;
        sum += w as u32;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    let ipc = !(sum as u16);
    f[24] = (ipc >> 8) as u8;
    f[25] = (ipc & 0xFF) as u8;
    f.extend_from_slice(&sport.to_be_bytes());
    f.extend_from_slice(&dport.to_be_bytes());
    let ulen = (8 + payload.len()) as u16;
    f.extend_from_slice(&ulen.to_be_bytes());
    f.extend_from_slice(&[0x00, 0x00]);
    let uc = udp_csum(src_ip, dst_ip, sport, dport, payload);
    let n = f.len();
    f[n - 2] = (uc >> 8) as u8;
    f[n - 1] = (uc & 0xFF) as u8;
    f.extend_from_slice(payload);
    f
}

fn udp_csum(src_ip: [u8; 4], dst_ip: [u8; 4], sport: u16, dport: u16, payload: &[u8]) -> u16 {
    let udp_len = 8 + payload.len();
    let mut words = Vec::new();
    for i in 0..2 {
        words.push(((src_ip[i * 2] as u16) << 8) | src_ip[i * 2 + 1] as u16);
        words.push(((dst_ip[i * 2] as u16) << 8) | dst_ip[i * 2 + 1] as u16);
    }
    words.push(17);
    words.push(udp_len as u16);
    words.push(sport);
    words.push(dport);
    words.push(udp_len as u16);
    words.push(0);
    for c in payload.chunks(2) {
        if c.len() == 2 {
            words.push(((c[0] as u16) << 8) | c[1] as u16);
        } else {
            words.push((c[0] as u16) << 8);
        }
    }
    let mut sum: u32 = 0;
    for w in words {
        sum += w as u32;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    let c = !(sum as u16);
    if c == 0 { 0xFFFF } else { c }
}

// ---------------------------------------------------------------------------
// §8 interop semantics
// ---------------------------------------------------------------------------

/// A data frame emitted by the DPDK TX path arrives byte-identical at the DPDK
/// RX path: the TX capture holds an ordinary IPv4/UDP datagram, and injecting
/// that datagram's UDP payload into the receiver yields the exact same bytes.
#[test]
#[serial]
fn aeron_data_frame_roundtrip_preserves_bytes() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    const PEER: [u8; 4] = [10, 0, 0, 5];
    const PEER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x50];
    h.arp_seed(PEER, PEER_MAC);

    let frame = aeron_data_frame(b"interop payload");
    let (rc, bytes_sent) = h.send_to((PEER, 40123), &[&frame]);
    assert_eq!(rc, 1);
    assert_eq!(bytes_sent, frame.len() as i64);

    // TX side: the emitted datagram carries the Aeron frame bytes unchanged.
    let caps = captures();
    assert_eq!(caps.len(), 1);
    let c = &caps[0];
    assert_eq!(udp_payload(c), frame.as_slice(), "TX path preserves payload bytes");
    assert_eq!(u32_ip_at(c, IP + 12), SENDER_IP, "source is the sender ENA IPv4");
    assert_eq!(u16_at(c, IP + 20 + 2), 40123, "destination UDP port");

    // RX side: wrap that same UDP payload in a frame addressed to the receiver
    // transport (with the sender's real identity as source) and dispatch it.
    let eth = build_udp_frame(RECEIVER_MAC, SENDER_MAC, SENDER_IP, RECEIVER_IP, 40000, 60000, &frame);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, eth.as_ptr(), eth.len(), RX_GOOD, 1) },
        0
    );
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!(bytes, frame.len() as i64, "bytes_rcved = the full payload");
    assert_eq!(h.rx_dgrams.len(), 1);
    let d = &h.rx_dgrams[0];
    assert_eq!(d.data, frame, "RX path yields the exact bytes the TX path emitted");
    assert_eq!(d.src_ip, SENDER_IP, "the sender's IPv4 is reconstructed");
    assert_eq!(d.src_port, 40000, "the sender's UDP port is reconstructed");
    assert_eq!(
        d.transport, &mut *h.rx as *mut _ as usize,
        "dispatched to the rx transport"
    );
    assert_eq!(h.stats().accepted, 1);
    assert_no_leak();
}

/// Connected and unconnected transports coexist on one runtime: a connected
/// transport routes via its connected address, an unconnected one routes per
/// call, both through the sender ENA.
#[test]
#[serial]
fn connected_and_unconnected_transports_coexist() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, Some(([10, 0, 0, 77], 5000)), 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    const CONN_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x77];
    const PEER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x50];
    h.arp_seed([10, 0, 0, 77], CONN_MAC);
    h.arp_seed([10, 0, 0, 5], PEER_MAC);

    let (rc, sent) = h.send_connected(&[b"connected"]);
    assert_eq!((rc, sent), (1, 9));
    let (rc, sent) = h.send_to(([10, 0, 0, 5], 40123), &[b"unconnected"]);
    assert_eq!((rc, sent), (1, 11));

    let caps = captures();
    assert_eq!(caps.len(), 2, "one datagram per transport");
    assert_eq!(
        u32_ip_at(&caps[0], IP + 16),
        [10, 0, 0, 77],
        "connected -> connected address"
    );
    assert_eq!(u16_at(&caps[0], IP + 20 + 2), 5000);
    assert_eq!(
        u32_ip_at(&caps[1], IP + 16),
        [10, 0, 0, 5],
        "unconnected -> explicit destination"
    );
    assert_eq!(u16_at(&caps[1], IP + 20 + 2), 40123);
    assert_eq!(caps[0].port_id, 0, "both emit on the sender ENA");
    assert_no_leak();
}

/// bind_addr_and_port reports the effective local identity, i.e. exactly what
/// the wildcard port manager bound (the DPDK transport assigns no port of its
/// own).
#[test]
#[serial]
fn bind_addr_and_port_reports_effective_local_identity() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    h.init_rx(RECEIVER_IP, 60000, 1472);
    assert_eq!(Harness::bind_addr_and_port(h.bindings, &mut h.tx), "10.0.0.1:40000");
    assert_eq!(Harness::bind_addr_and_port(h.bindings, &mut h.rx), "10.0.1.1:60000");
}

/// A wildcard bind (port 0) gets an ephemeral port, as a kernel socket would:
/// unique on the NIC, in 32768..=60999, reported by bind_addr_and_port and used
/// as the UDP source port, so peers (kernel UDP included) can answer it. Port
/// 0 never goes on the wire.
#[test]
#[serial]
fn wildcard_bind_gets_an_ephemeral_port() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 0, None, 1472);
    let bound = Harness::bind_addr_and_port(h.bindings, &mut h.tx);
    let port: u16 = bound.strip_prefix("10.0.0.1:").expect(&bound).parse().unwrap();
    assert!((32768..=60999).contains(&port), "ephemeral range: {bound}");

    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    let (rc, _) = h.send_to(([10, 0, 0, 5], 40123), &[b"x"]);
    assert_eq!(rc, 1, "send: {}", last_error());
    let caps = captures();
    assert_eq!(
        u16_at(caps.last().unwrap(), UDP),
        port,
        "the UDP source port is the assigned one"
    );

    // Another wildcard bind on the same NIC gets a different port; an explicit
    // port is kept as given.
    let mut second = init_extra_tx(&mut h, 0);
    let mut explicit = init_extra_tx(&mut h, 40000);
    let second_port = Harness::bind_addr_and_port(h.bindings, &mut second);
    assert_ne!(second_port, bound, "unique per NIC");
    assert_eq!(Harness::bind_addr_and_port(h.bindings, &mut explicit), "10.0.0.1:40000");
    close_transport(h.bindings, &mut second);
    close_transport(h.bindings, &mut explicit);
    assert_no_leak();
}

/// Reconnect replaces the copied remote address without retaining a caller-
/// owned pointer: the new address is passed from a temporary buffer that dies
/// before the next send, yet the connected send still reaches it.
#[test]
#[serial]
fn reconnect_replaces_copied_remote_address() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, Some(([10, 0, 0, 77], 5000)), 1472);
    const A_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x77];
    const B_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x88];
    h.arp_seed([10, 0, 0, 77], A_MAC);
    h.arp_seed([10, 0, 0, 88], B_MAC);

    // Before reconnect: connected sends reach the original address.
    let (rc, _) = h.send_connected(&[b"to-a"]);
    assert_eq!(rc, 1);
    assert_eq!(u32_ip_at(&captures()[0], IP + 16), [10, 0, 0, 77]);

    // Reconnect to B from a temporary that is dropped here — the native side
    // must have copied the address into its own client state.
    h.reconnect(([10, 0, 0, 88], 6000));

    let (rc, _) = h.send_connected(&[b"to-b"]);
    assert_eq!(rc, 1);
    let caps = captures();
    assert_eq!(caps.len(), 2);
    assert_eq!(
        u32_ip_at(&caps[1], IP + 16),
        [10, 0, 0, 88],
        "connected send follows the new address"
    );
    assert_eq!(u16_at(&caps[1], IP + 20 + 2), 6000);
    assert_no_leak();
}

/// Interceptors are usable through `aeron_udp_channel_data_paths_t`: the
/// poller dispatches with the registered transport's data_paths pointer, so the
/// incoming-interceptor chain reachable from the callback gets the channel's
/// data path exactly as the kernel socket path would.
#[test]
#[serial]
fn data_paths_forwarded_to_recv_callback() {
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let dp = Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_data_paths_stct>() });
    h.rx.data_paths = &*dp as *const _ as *mut _;

    let frame = build_udp_frame(
        RECEIVER_MAC,
        SENDER_MAC,
        SENDER_IP,
        RECEIVER_IP,
        40000,
        60000,
        b"interceptor",
    );
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
        0
    );
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!(h.rx_dgrams.len(), 1);
    assert_eq!(
        h.rx_dgrams[0].data_paths, &*dp as *const _ as usize,
        "recv_cb received the transport's data_paths"
    );
    assert_no_leak();
}

/// NAK/status/setup/RTT/data/retransmit role frames are opaque UDP payloads
/// that dispatch to the correct endpoint: two receiver transports on the shared
/// receiver ENA behave like a manual MDC control endpoint plus a data endpoint,
/// and removing one stops its dispatch (dynamic MDC close) without disturbing
/// the other.
#[test]
#[serial]
fn role_frames_dispatch_by_endpoint_and_removal_stops() {
    let mut h = Harness::new();
    h.init_rx(RECEIVER_IP, 60000, 1472);
    let mut control = init_extra_transport(&mut h, 60001);
    let mut p = h.poller();
    p.add(&mut *h.rx);
    p.add(&mut *control);

    // (dport, frame type, label) — every Aeron control-plane and data-plane
    // role Aeron produces, delivered as UDP payloads.
    let roles: &[(u16, u16, &[u8])] = &[
        (60000, 0x03, b"nak"),        // NAK (retransmission request)
        (60001, 0x04, b"sm"),         // status message
        (60000, 0x06, b"setup"),      // stream setup
        (60001, 0x07, b"rtt"),        // RTT measurement
        (60000, 0x01, b"data"),       // data frame
        (60001, 0x05, b"retransmit"), // ERRT retransmit
    ];
    for (dport, ty, label) in roles {
        let payload = aeron_control_frame(*ty, label);
        let eth = build_udp_frame(
            RECEIVER_MAC,
            SENDER_MAC,
            SENDER_IP,
            RECEIVER_IP,
            40000,
            *dport,
            &payload,
        );
        assert_eq!(
            unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, eth.as_ptr(), eth.len(), RX_GOOD, 1) },
            0
        );
    }

    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 6, "all six role frames dispatched");
    assert_eq!(h.rx_dgrams.len(), 6);
    let primary = &mut *h.rx as *mut _ as usize;
    let control_ptr = &mut *control as *mut _ as usize;
    for (dport, ty, label) in roles {
        let want_frame = aeron_control_frame(*ty, label);
        let d = h.rx_dgrams.iter().find(|d| d.data == want_frame).unwrap();
        let want = if *dport == 60000 { primary } else { control_ptr };
        assert_eq!(
            d.transport, want,
            "role frame on port {dport} dispatched to the right endpoint"
        );
    }
    assert_eq!(h.stats().accepted, 6);

    // Dynamic MDC close: removing the control endpoint stops its dispatch; the
    // data endpoint is unaffected.
    p.remove(&mut *control);
    let payload = aeron_control_frame(0x07, b"rtt");
    let eth = build_udp_frame(RECEIVER_MAC, SENDER_MAC, SENDER_IP, RECEIVER_IP, 40000, 60001, &payload);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, eth.as_ptr(), eth.len(), RX_GOOD, 1) },
        0
    );
    assert_eq!(p.poll(16, clientd, &mut bytes), 0, "no dispatch after removal");
    assert_eq!(h.rx_dgrams.len(), 6);
    let s = h.stats();
    assert_eq!(s.accepted, 6);
    assert_eq!(s.unknown_port, 1, "the removed endpoint is an unknown port");

    let payload = aeron_data_frame(b"still alive");
    let eth = build_udp_frame(RECEIVER_MAC, SENDER_MAC, SENDER_IP, RECEIVER_IP, 40000, 60000, &payload);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, eth.as_ptr(), eth.len(), RX_GOOD, 1) },
        0
    );
    assert_eq!(
        p.poll(16, clientd, &mut bytes),
        1,
        "the remaining endpoint still dispatches"
    );
    assert_no_leak();
    close_transport(h.bindings, &mut control);
}

/// Messages exceeding one Aeron frame use Aeron fragmentation at the transport
/// boundary: every emitted datagram stays within `max_aeron_mtu`, DF is always
/// set (so a router never fragments), and an oversized iovec is rejected.
#[test]
#[serial]
fn mtu_bound_never_fragments() {
    let mut h = Harness::new();
    // A 32-byte channel MTU: 32 fits, 33 is oversized.
    h.init_tx(SENDER_IP, 40000, None, 32);
    const PEER: [u8; 4] = [10, 0, 0, 5];
    const PEER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x50];
    h.arp_seed(PEER, PEER_MAC);

    // Aeron fragments a large message into per-frame iovecs; each iovec is one
    // datagram within the MTU.
    let (rc, sent) = h.send_to((PEER, 40123), &[&[1u8; 32], &[2u8; 16], &[3u8; 24]]);
    assert_eq!(rc, 3);
    assert_eq!(sent, 32 + 16 + 24);
    let caps = captures();
    assert_eq!(caps.len(), 3);
    for c in &caps {
        assert_eq!(
            udp_payload(c).len(),
            c.len as usize - PAYLOAD,
            "each datagram is its own iovec"
        );
        assert!(udp_payload(c).len() <= 32, "no datagram exceeds max_aeron_mtu");
        assert_eq!(ip_flags_frag(c), 0x4000, "DF set: never emit IP fragments");
    }

    // A single iovec beyond the MTU is rejected outright.
    let (rc, sent) = h.send_to((PEER, 40123), &[&[4u8; 33]]);
    assert_eq!(rc, -1);
    assert_eq!(sent, 0);
    assert!(last_error().contains("oversized"), "got: {}", last_error());
    assert_no_leak();
}

/// Jumbo frames end to end, as on an AWS 9001-byte VPC with max_aeron_mtu
/// 8960: a full 8960-byte datagram leaves as one 9002-byte Ethernet frame in
/// one mbuf (IPv4 total length 8988, DF set), and the same frame received is
/// delivered whole, byte for byte. A frame larger than the port's mbufs (from
/// a peer whose MTU exceeds ours) arrives split over two, as ENA's always
/// scattered receive splits it, and is dropped and counted, never delivered
/// in part.
#[test]
#[serial]
fn jumbo_frames_cross_whole_in_one_mbuf() {
    let mut config = rusteron_dpdk_config_t::valid();
    config.max_aeron_mtu = 8960;
    let mut h = Harness::with_config(config);
    h.init_tx(SENDER_IP, 40000, None, 8960);
    h.init_rx(RECEIVER_IP, 60000, 8960);
    const PEER: [u8; 4] = [10, 0, 0, 5];
    const PEER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x50];
    h.arp_seed(PEER, PEER_MAC);

    let payload: Vec<u8> = (0..8960u32).map(|i| (i * 7) as u8).collect();
    assert_eq!(h.send_to((PEER, 40123), &[&payload]), (1, 8960), "{}", last_error());
    let caps = captures();
    assert_eq!(caps.len(), 1, "one frame, never fragmented");
    let c = &caps[0];
    assert_eq!(c.len, 9002, "Ethernet + IPv4 + UDP + 8960");
    assert_eq!(u16_at(c, IP + 2), 8988, "IPv4 total length: the port's L3 MTU");
    assert_eq!(ip_flags_frag(c), 0x4000, "DF set, no fragment");
    assert_eq!(udp_payload(c), payload.as_slice());

    let eth = build_udp_frame(RECEIVER_MAC, SENDER_MAC, SENDER_IP, RECEIVER_IP, 40000, 60000, &payload);
    assert_eq!(eth.len(), 9002);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, eth.as_ptr(), eth.len(), RX_GOOD, 1) },
        0
    );
    let mut p = h.poller();
    p.add(&mut *h.rx);
    let mut bytes = 0i64;
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    assert_eq!(p.poll(16, clientd, &mut bytes), 1);
    assert_eq!(bytes, 8960);
    assert_eq!(h.rx_dgrams[0].data, payload, "delivered whole");

    // A 9001-byte IPv4 packet (a peer at the VPC's full MTU): 13 bytes more
    // than this port's mbufs hold.
    let big = build_udp_frame(
        RECEIVER_MAC,
        SENDER_MAC,
        SENDER_IP,
        RECEIVER_IP,
        40000,
        60000,
        &[1u8; 8973],
    );
    assert_eq!(big.len(), 9015);
    assert_eq!(
        unsafe { rusteron_dpdk_fake_rx_inject(RECEIVER_PORT, big.as_ptr(), big.len(), RX_GOOD, 1) },
        0
    );
    assert_eq!(p.poll(16, clientd, &mut bytes), 0, "never delivered in part");
    assert_eq!(h.rx_dgrams.len(), 1);
    assert_eq!(h.stats().multi_segment, 1, "dropped and counted");
    drop(p);
    assert_no_leak();
}

// ---------------------------------------------------------------------------
// Kernel bind() semantics (ports, addresses)
// ---------------------------------------------------------------------------

/// An explicit port already bound on the NIC fails init with EADDRINUSE, at
/// once, as a kernel bind() does; closing the holder frees it.
#[test]
#[serial]
fn explicit_duplicate_bind_is_eaddrinuse_until_closed() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    let (rc, _t) = try_init(&mut h, SND, (SENDER_IP, 40000), None, ptr::null_mut());
    assert_eq!(rc, -1, "a second bind of 10.0.0.1:40000");
    assert!(last_error().contains("already bound"), "{}", last_error());

    // The same port on the other NIC is a different address: fine.
    let (rc, mut other) = try_init(&mut h, RCV, (RECEIVER_IP, 40000), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    close_transport(h.bindings, &mut other);

    let (rc, mut first) = try_init(&mut h, SND, (SENDER_IP, 40001), None, ptr::null_mut());
    assert_eq!(rc, 0);
    close_transport(h.bindings, &mut first);
    let (rc, mut again) = try_init(&mut h, SND, (SENDER_IP, 40001), None, ptr::null_mut());
    assert_eq!(rc, 0, "freed by close: {}", last_error());
    close_transport(h.bindings, &mut again);
}

/// Wildcard binds draw from RUSTERON_DPDK_EPHEMERAL_PORT_RANGE, skip bound
/// ports, and fail with EADDRINUSE when the range is used up.
#[test]
#[serial]
fn ephemeral_ports_come_from_the_configured_range() {
    let _guard = EnvGuard::set("RUSTERON_DPDK_EPHEMERAL_PORT_RANGE", "40000 40002");
    let mut h = Harness::new();
    let (rc, mut explicit) = try_init(&mut h, SND, (SENDER_IP, 40001), None, ptr::null_mut());
    assert_eq!(rc, 0);
    let (rc, mut a) = try_init(&mut h, SND, ([0, 0, 0, 0], 0), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    let (rc, mut b) = try_init(&mut h, SND, ([0, 0, 0, 0], 0), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    let mut got = vec![
        port_of(&Harness::bind_addr_and_port(h.bindings, &mut a)),
        port_of(&Harness::bind_addr_and_port(h.bindings, &mut b)),
    ];
    got.sort_unstable();
    assert_eq!(got, vec![40000, 40002], "the explicit 40001 is skipped");
    let (rc, _c) = try_init(&mut h, SND, ([0, 0, 0, 0], 0), None, ptr::null_mut());
    assert_eq!(rc, -1, "range exhausted");
    assert!(last_error().contains("no free ephemeral UDP port"), "{}", last_error());
    for t in [&mut explicit, &mut a, &mut b] {
        close_transport(h.bindings, t);
    }
}

/// A malformed RUSTERON_DPDK_EPHEMERAL_PORT_RANGE fails runtime start.
#[test]
#[serial]
fn malformed_ephemeral_range_fails_start() {
    for bad in ["40000", "40002 40000", "0 10", "a b", "40000 70000"] {
        let env = TestEnv::new();
        env.eal_skip();
        let _guard = EnvGuard::set("RUSTERON_DPDK_EPHEMERAL_PORT_RANGE", bad);
        let err = create(&rusteron_dpdk_config_t::valid()).unwrap_err();
        assert!(err.contains("RUSTERON_DPDK_EPHEMERAL_PORT_RANGE"), "{bad}: {err}");
    }
}

/// Ephemeral picks avoid Aeron's own configured wildcard port ranges, which
/// Aeron hands out itself (receivers, control-address senders).
#[test]
#[serial]
fn ephemeral_ports_avoid_aerons_wildcard_ranges() {
    let _guard = EnvGuard::set("RUSTERON_DPDK_EPHEMERAL_PORT_RANGE", "40000 40002");
    let mut h = Harness::new();
    let mut ctx = Box::new(unsafe { std::mem::zeroed::<aeron_driver_context_t>() });
    ctx.sender_wildcard_port_manager.is_os_wildcard = false;
    ctx.sender_wildcard_port_manager.low_port = 40000;
    ctx.sender_wildcard_port_manager.high_port = 40000;
    ctx.receiver_wildcard_port_manager.is_os_wildcard = false;
    ctx.receiver_wildcard_port_manager.low_port = 40001;
    ctx.receiver_wildcard_port_manager.high_port = 40001;
    let (rc, mut t) = try_init(&mut h, SND, ([0, 0, 0, 0], 0), None, &mut *ctx);
    assert_eq!(rc, 0, "{}", last_error());
    assert_eq!(port_of(&Harness::bind_addr_and_port(h.bindings, &mut t)), 40002);
    close_transport(h.bindings, &mut t);
}

/// A bind address that is neither the wildcard nor the NIC's own fails with
/// EADDRNOTAVAIL, as the kernel would; multicast fails with EPROTONOSUPPORT.
#[test]
#[serial]
fn bind_addresses_follow_kernel_rules() {
    let mut h = Harness::new();
    let (rc, _t) = try_init(&mut h, SND, ([10, 0, 0, 99], 40000), None, ptr::null_mut());
    assert_eq!(rc, -1);
    assert!(last_error().contains("not this NIC's"), "{}", last_error());
    let (rc, _t) = try_init(&mut h, RCV, (SENDER_IP, 40000), None, ptr::null_mut());
    assert_eq!(rc, -1, "the sender NIC's address on the receiver NIC");
    let (rc, _t) = try_init(&mut h, RCV, ([239, 1, 2, 3], 40000), None, ptr::null_mut());
    assert_eq!(rc, -1);
    assert!(last_error().contains("multicast"), "{}", last_error());
    let (rc, _t) = try_init(
        &mut h,
        SND,
        ([0, 0, 0, 0], 0),
        Some(([239, 1, 2, 3], 40000)),
        ptr::null_mut(),
    );
    assert_eq!(rc, -1, "multicast connect");
    let (rc, mut ok) = try_init(&mut h, SND, (SENDER_IP, 40000), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    close_transport(h.bindings, &mut ok);
}

/// The kernel-peer case that failed on Azure: a reply to the assigned
/// ephemeral port reaches the transport (wire port == demux port == reported
/// port), and nothing addressed to port 0 is ever dispatched.
#[test]
#[serial]
fn reply_to_the_ephemeral_port_is_received() {
    const SENDER_PORT: u16 = 0;
    let mut h = Harness::new();
    h.init_tx([0, 0, 0, 0], 0, None, 1472);
    let port = port_of(&Harness::bind_addr_and_port(h.bindings, &mut h.tx));
    let mut p = h.poller();
    p.add(&mut *h.tx);
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let mut bytes = 0i64;
    let peer = [10, 0, 0, 5];
    for (dport, expect) in [(port, 1), (0, 0)] {
        let frame = build_udp_frame(
            SENDER_MAC,
            [0x02, 0, 0, 0, 0, 0x50],
            peer,
            SENDER_IP,
            7000,
            dport,
            b"sm",
        );
        assert_eq!(
            unsafe { rusteron_dpdk_fake_rx_inject(SENDER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
            0
        );
        assert_eq!(p.poll(16, clientd, &mut bytes), expect, "to port {dport}");
    }
    assert_eq!(h.rx_dgrams.len(), 1);
    assert_eq!(h.rx_dgrams[0].transport, &*h.tx as *const _ as usize);
    assert_eq!((h.rx_dgrams[0].src_ip, h.rx_dgrams[0].src_port), (peer, 7000));
}

/// A connected transport hears only its peer, as a connect()ed socket does.
#[test]
#[serial]
fn connected_transport_receives_only_from_its_peer() {
    const SENDER_PORT: u16 = 0;
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, Some(([10, 0, 0, 5], 40123)), 1472);
    let mut p = h.poller();
    p.add(&mut *h.tx);
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let mut bytes = 0i64;
    for (src, sport, expect) in [
        ([10, 0, 0, 6], 40123, 0),
        ([10, 0, 0, 5], 9, 0),
        ([10, 0, 0, 5], 40123, 1),
    ] {
        let frame = build_udp_frame(
            SENDER_MAC,
            [0x02, 0, 0, 0, 0, 0x50],
            src,
            SENDER_IP,
            sport,
            40000,
            b"sm",
        );
        assert_eq!(
            unsafe { rusteron_dpdk_fake_rx_inject(SENDER_PORT, frame.as_ptr(), frame.len(), RX_GOOD, 1) },
            0
        );
        assert_eq!(p.poll(16, clientd, &mut bytes), expect, "from {src:?}:{sport}");
    }
    assert_eq!(h.stats().not_connected_peer, 2);
}

/// reconnect, as in the kernel transport, only re-targets a transport that
/// was connected; an unconnected one stays unconnected.
#[test]
#[serial]
fn reconnect_only_applies_to_connected_transports() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    let mut to = SockAddrIn::new([10, 0, 0, 5], 40123);
    let rc = unsafe { (*h.bindings).reconnect_func.unwrap()(&mut *h.tx, to.storage_mut()) };
    assert_eq!(rc, 0);
    assert!(
        h.tx.connected_address.is_null(),
        "an unconnected transport stays unconnected"
    );
}

// ---------------------------------------------------------------------------
// Host-local channels: loopback and the host's kernel interfaces go through a
// kernel socket, never the DPDK NIC
// ---------------------------------------------------------------------------

/// A subscription bound on 127.0.0.1 receives over the kernel: a datagram from
/// an ordinary socket arrives through the poller, with its source.
#[test]
#[serial]
fn loopback_subscription_receives_through_the_kernel() {
    let mut h = Harness::new();
    let (rc, mut t) = try_init(&mut h, RCV, ([127, 0, 0, 1], 0), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    let bound = Harness::bind_addr_and_port(h.bindings, &mut t);
    assert!(bound.starts_with("127.0.0.1:"), "{bound}");
    let mut p = h.poller();
    p.add(&mut *t);

    let peer = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.send_to(b"hello", &bound).unwrap();
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let mut bytes = 0i64;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while h.rx_dgrams.is_empty() && std::time::Instant::now() < deadline {
        p.poll(16, clientd, &mut bytes);
    }
    assert_eq!(h.rx_dgrams.len(), 1, "received over the kernel");
    assert_eq!(h.rx_dgrams[0].data, b"hello");
    assert_eq!(h.rx_dgrams[0].src_port, peer.local_addr().unwrap().port());
    assert!(captures().is_empty(), "nothing on the DPDK NIC");
    p.remove(&mut *t);
    close_transport(h.bindings, &mut t);
}

/// A publication connected to 127.0.0.1 sends over the kernel and hears the
/// peer's replies.
#[test]
#[serial]
fn loopback_publication_sends_and_hears_replies() {
    let mut h = Harness::new();
    let peer = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
    let pport = peer.local_addr().unwrap().port();
    let (rc, mut t) = try_init(
        &mut h,
        SND,
        ([0, 0, 0, 0], 0),
        Some(([127, 0, 0, 1], pport)),
        ptr::null_mut(),
    );
    assert_eq!(rc, 0, "{}", last_error());
    let mut p = h.poller();
    p.add(&mut *t);

    let msg = b"data";
    let mut iov = [iovec {
        iov_base: msg.as_ptr() as *mut c_void,
        iov_len: msg.len(),
    }];
    let mut sent = 0i64;
    let rc = unsafe {
        (*h.bindings).send_func.unwrap()(
            &mut *h.data_paths,
            &mut *t,
            ptr::null_mut(),
            iov.as_mut_ptr(),
            1,
            &mut sent,
        )
    };
    assert_eq!((rc, sent), (1, 4), "{}", last_error());
    let mut buf = [0u8; 16];
    let (n, from) = peer.recv_from(&mut buf).unwrap();
    assert_eq!(&buf[..n], msg);

    peer.send_to(b"sm", from).unwrap();
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let mut bytes = 0i64;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while h.rx_dgrams.is_empty() && std::time::Instant::now() < deadline {
        p.poll(16, clientd, &mut bytes);
    }
    assert_eq!(h.rx_dgrams.len(), 1, "the reply came back");
    assert!(captures().is_empty(), "nothing on the DPDK NIC");
    p.remove(&mut *t);
    close_transport(h.bindings, &mut t);
}

/// A DPDK publication with a host-local destination among its remote ones
/// (MDC): the local one goes through a kernel socket bound on the
/// destination's own address (with the transport's port where free), opened
/// by the local-I/O thread, whose replies reach the transport; the remote one
/// still goes over DPDK.
#[test]
#[serial]
fn mixed_destinations_split_between_dpdk_and_the_kernel_on_one_port() {
    let mut h = Harness::new();
    h.init_tx([0, 0, 0, 0], 0, None, 1472);
    let port = port_of(&Harness::bind_addr_and_port(h.bindings, &mut h.tx));
    let mut p = h.poller();
    p.add(&mut *h.tx);
    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    let peer = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(std::time::Duration::from_secs(2))).unwrap();
    let pport = peer.local_addr().unwrap().port();

    // The local-I/O thread opens its socket: zero sends (Aeron retries)
    // until it is ready, within a few milliseconds; the agent never does.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let rc = loop {
        let (rc, _) = h.send_to(([127, 0, 0, 1], pport), &[b"local"]);
        if rc != 0 || std::time::Instant::now() > deadline {
            break rc;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    };
    assert_eq!(rc, 1, "{}", last_error());
    let mut buf = [0u8; 16];
    let (n, from) = peer.recv_from(&mut buf).unwrap();
    assert_eq!(&buf[..n], b"local");
    assert_eq!(
        from.ip(),
        std::net::Ipv4Addr::LOCALHOST,
        "sent from the destination's own address"
    );
    assert_eq!(from.port(), port, "with the transport's port, free on loopback");
    let (rc, _) = h.send_to(([10, 0, 0, 5], 40123), &[b"remote"]);
    assert_eq!(rc, 1);
    assert_eq!(captures().len(), 1, "only the remote destination used the NIC");
    assert_eq!(
        u16_at(&captures()[0], UDP),
        port,
        "the NIC carries the transport's own port"
    );

    peer.send_to(b"nak", from).unwrap();
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let mut bytes = 0i64;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while h.rx_dgrams.is_empty() && std::time::Instant::now() < deadline {
        p.poll(16, clientd, &mut bytes);
    }
    assert_eq!(h.rx_dgrams.len(), 1, "the local peer's reply reached the transport");
    assert_no_leak();
}

/// Destinations no NIC can carry fail loudly: this NIC itself, 0.0.0.0, the
/// broadcast address, port 0.
#[test]
#[serial]
fn invalid_destinations_are_errors() {
    let mut h = Harness::new();
    h.init_tx(SENDER_IP, 40000, None, 1472);
    for dst in [
        (SENDER_IP, 40123),
        ([0, 0, 0, 0], 40123),
        ([255, 255, 255, 255], 40123),
        ([10, 0, 0, 255], 40123),
        ([10, 0, 0, 5], 0),
    ] {
        let (rc, _) = h.send_to(dst, &[b"x"]);
        assert_eq!(rc, -1, "{dst:?}");
    }
    assert!(captures().is_empty());
}

/// A 0.0.0.0 subscription also hears same-host publishers (loopback), as a
/// kernel socket bound to the wildcard does: on by default, at no cost while
/// idle (the local-I/O thread flags its socket only when data waits).
#[test]
#[serial]
fn wildcard_subscription_hears_localhost_by_default() {
    let mut h = Harness::new();
    let (rc, mut t) = try_init(&mut h, RCV, ([0, 0, 0, 0], 0), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    let port = port_of(&Harness::bind_addr_and_port(h.bindings, &mut t));
    let mut p = h.poller();
    p.add(&mut *t);
    let peer = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.send_to(b"local", ("127.0.0.1", port)).unwrap();
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let mut bytes = 0i64;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while h.rx_dgrams.is_empty() && std::time::Instant::now() < deadline {
        p.poll(16, clientd, &mut bytes);
    }
    assert_eq!(h.rx_dgrams.len(), 1, "the loopback datagram arrived");
    assert_eq!(h.rx_dgrams[0].data, b"local");
    p.remove(&mut *t);
    close_transport(h.bindings, &mut t);
}

/// RUSTERON_DPDK_WILDCARD_LOCAL=0 keeps a wildcard subscription on the NIC
/// alone: no kernel socket, so the port stays free on the host.
#[test]
#[serial]
fn wildcard_local_opt_out_opens_no_kernel_socket() {
    let _guard = EnvGuard::set("RUSTERON_DPDK_WILDCARD_LOCAL", "0");
    let mut h = Harness::new();
    let (rc, mut t) = try_init(&mut h, RCV, ([0, 0, 0, 0], 0), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    let port = port_of(&Harness::bind_addr_and_port(h.bindings, &mut t));
    std::net::UdpSocket::bind(("0.0.0.0", port)).expect("the host's port is free");
    close_transport(h.bindings, &mut t);
}

/// Host-local channels are not limited to max_aeron_mtu: loopback carries up
/// to 64 KiB, as on the kernel driver. The NIC path keeps the limit.
#[test]
#[serial]
fn host_local_channel_may_exceed_max_aeron_mtu() {
    let mut h = Harness::new();
    let init = |h: &mut Harness, ip: [u8; 4]| {
        let mut t = Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() });
        let mut b = SockAddrIn::new(ip, 0);
        let mut dummy = SockAddrIn::new([0, 0, 0, 0], 0);
        let mut params: aeron_udp_channel_transport_params_stct = unsafe { std::mem::zeroed() };
        params.mtu_length = 8192;
        let rc = unsafe {
            (*h.bindings).init_func.unwrap()(
                &mut *t,
                b.storage_mut(),
                dummy.storage_mut(),
                ptr::null_mut(),
                &mut params,
                ptr::null_mut(),
                RCV,
            )
        };
        (rc, t)
    };
    let (rc, mut local) = init(&mut h, [127, 0, 0, 1]);
    assert_eq!(rc, 0, "{}", last_error());
    close_transport(h.bindings, &mut local);
    let (rc, _nic) = init(&mut h, RECEIVER_IP);
    assert_eq!(rc, -1, "the NIC path keeps the max_aeron_mtu limit");
}

/// RUSTERON_DPDK_HOST_LOCAL=all (the default outside tests) treats the host's
/// kernel-interface addresses as local, so a channel on one runs on a kernel
/// socket; "loopback" (the tests' default) keeps only 127/8.
#[test]
#[serial]
fn host_local_all_covers_kernel_interface_addresses() {
    // This machine's non-loopback address (connect sends no packet).
    let probe = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    probe.connect("192.0.2.1:9").expect("a default route");
    let std::net::IpAddr::V4(own) = probe.local_addr().unwrap().ip() else {
        panic!("IPv4 expected")
    };
    let own = own.octets();
    {
        let mut h = Harness::new();
        let (rc, _t) = try_init(&mut h, RCV, (own, 0), None, ptr::null_mut());
        assert_eq!(
            rc, -1,
            "loopback only: a kernel address is neither local nor this NIC's"
        );
    }
    let _guard = EnvGuard::set("RUSTERON_DPDK_HOST_LOCAL", "all");
    let mut h = Harness::new();
    let (rc, mut t) = try_init(&mut h, RCV, (own, 0), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    let bound = Harness::bind_addr_and_port(h.bindings, &mut t);
    assert!(
        bound.starts_with(&format!("{}.{}.{}.{}:", own[0], own[1], own[2], own[3])),
        "{bound}"
    );
    close_transport(h.bindings, &mut t);
}

/// An extra address (RUSTERON_DPDK_SENDER_EXTRA_IPV4) behaves as a real one: a
/// bind on it is reported with it, sends from it, and its ARP is answered; it
/// and the NIC's own address hold the same port independently.
#[test]
#[serial]
fn extra_addresses_behave_as_real_ones() {
    let _guard = EnvGuard::set("RUSTERON_DPDK_SENDER_EXTRA_IPV4", "10.0.0.9");
    let mut h = Harness::new();
    let (rc, mut extra) = try_init(&mut h, SND, ([10, 0, 0, 9], 40000), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    assert_eq!(Harness::bind_addr_and_port(h.bindings, &mut extra), "10.0.0.9:40000");
    let (rc, mut own) = try_init(&mut h, SND, (SENDER_IP, 40000), None, ptr::null_mut());
    assert_eq!(rc, 0, "the same port on another address: {}", last_error());
    let (rc, _wild) = try_init(&mut h, SND, ([0, 0, 0, 0], 40000), None, ptr::null_mut());
    assert_eq!(rc, -1, "a wildcard bind conflicts with both");

    h.arp_seed([10, 0, 0, 5], [0x02, 0, 0, 0, 0, 0x50]);
    let msg = b"from-extra";
    let mut iov = [iovec {
        iov_base: msg.as_ptr() as *mut c_void,
        iov_len: msg.len(),
    }];
    let mut to = SockAddrIn::new([10, 0, 0, 5], 40123);
    let mut sent = 0i64;
    let rc = unsafe {
        (*h.bindings).send_func.unwrap()(
            &mut *h.data_paths,
            &mut *extra,
            to.storage_mut(),
            iov.as_mut_ptr(),
            1,
            &mut sent,
        )
    };
    assert_eq!(rc, 1, "{}", last_error());
    let c = captures();
    assert_eq!(
        u32_ip_at(c.last().unwrap(), IP + 12),
        [10, 0, 0, 9],
        "sent from the extra address"
    );
    close_transport(h.bindings, &mut extra);
    close_transport(h.bindings, &mut own);
}

/// A wildcard subscription's kernel listener serves this host only: a
/// datagram from a remote source reaching it through a kernel interface is
/// not delivered (its replies would leave over DPDK from another address),
/// one from loopback is.
#[test]
#[serial]
fn wildcard_local_listener_hears_only_this_host() {
    let probe = std::net::UdpSocket::bind("0.0.0.0:0").unwrap();
    probe.connect("192.0.2.1:9").expect("a default route");
    let std::net::IpAddr::V4(own) = probe.local_addr().unwrap().ip() else {
        panic!("IPv4 expected")
    };
    let mut h = Harness::new(); // host-local is loopback only: `own` counts as remote
    let (rc, mut t) = try_init(&mut h, RCV, ([0, 0, 0, 0], 0), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    let port = port_of(&Harness::bind_addr_and_port(h.bindings, &mut t));
    let mut p = h.poller();
    p.add(&mut *t);
    let remote = std::net::UdpSocket::bind((own, 0)).unwrap();
    remote.send_to(b"remote", (own, port)).unwrap();
    let local = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    local.send_to(b"local", ("127.0.0.1", port)).unwrap();
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let mut bytes = 0i64;
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
    while std::time::Instant::now() < deadline {
        p.poll(16, clientd, &mut bytes);
    }
    let got: Vec<&[u8]> = h.rx_dgrams.iter().map(|d| d.data.as_slice()).collect();
    assert_eq!(got, vec![&b"local"[..]]);
    p.remove(&mut *t);
    close_transport(h.bindings, &mut t);
}

/// A connected host-local publication whose peer was not yet listening has a
/// pending ICMP error on its socket; the peer's first reply queued behind it
/// is still read (the error does not count as "drained", or the reply would
/// wait for another datagram to wake it).
#[test]
#[serial]
fn reply_queued_behind_a_socket_error_is_read() {
    let free_port = std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let mut h = Harness::new();
    let (rc, mut t) = try_init(
        &mut h,
        SND,
        ([127, 0, 0, 1], 0),
        Some(([127, 0, 0, 1], free_port)),
        ptr::null_mut(),
    );
    assert_eq!(rc, 0, "{}", last_error());
    let own = port_of(&Harness::bind_addr_and_port(h.bindings, &mut t));
    let mut p = h.poller();
    p.add(&mut *t);

    // Nobody listens yet: the kernel queues ECONNREFUSED on the socket.
    let msg = b"data";
    let mut iov = [iovec {
        iov_base: msg.as_ptr() as *mut c_void,
        iov_len: msg.len(),
    }];
    let mut sent = 0i64;
    let rc = unsafe {
        (*h.bindings).send_func.unwrap()(
            &mut *h.data_paths,
            &mut *t,
            ptr::null_mut(),
            iov.as_mut_ptr(),
            1,
            &mut sent,
        )
    };
    assert_eq!(rc, 1, "{}", last_error());
    std::thread::sleep(std::time::Duration::from_millis(50));

    // The peer comes up and replies before the transport is polled again.
    let peer = std::net::UdpSocket::bind(("127.0.0.1", free_port)).unwrap();
    peer.send_to(b"status", ("127.0.0.1", own)).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let mut bytes = 0i64;
    let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
    while h.rx_dgrams.is_empty() && std::time::Instant::now() < deadline {
        p.poll(16, clientd, &mut bytes);
    }
    assert_eq!(h.rx_dgrams.len(), 1, "the reply behind the error was read");
    assert_eq!(h.rx_dgrams[0].data, b"status");
    p.remove(&mut *t);
    close_transport(h.bindings, &mut t);
}

/// With RUSTERON_DPDK_LOCAL_IO=spin the agents read host-local sockets every
/// duty cycle instead of on the local-I/O thread's flag: same delivery.
#[test]
#[serial]
fn loopback_subscription_receives_with_spin_local_io() {
    let _mode = EnvGuard::set("RUSTERON_DPDK_LOCAL_IO", "spin");
    let mut h = Harness::new();
    let (rc, mut t) = try_init(&mut h, RCV, ([127, 0, 0, 1], 0), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    let bound = Harness::bind_addr_and_port(h.bindings, &mut t);
    let mut p = h.poller();
    p.add(&mut *t);
    let peer = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let mut bytes = 0i64;
    for n in 0..3u8 {
        peer.send_to(&[n], &bound).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while h.rx_dgrams.len() <= n as usize && std::time::Instant::now() < deadline {
            p.poll(16, clientd, &mut bytes);
        }
    }
    assert_eq!(h.rx_dgrams.iter().map(|d| d.data[0]).collect::<Vec<_>>(), vec![0, 1, 2]);
    p.remove(&mut *t);
    close_transport(h.bindings, &mut t);
}

/// RUSTERON_DPDK_LOCAL_IO=adaptive: an idle host-local socket waits for the
/// local-I/O thread's flag, as with epoll; once it has had data it is read
/// every duty cycle for the hot window, so the next datagram does not wait
/// for a wake-up; after the window it waits for the flag again.
#[test]
#[serial]
fn adaptive_local_io_reads_hot_sockets_every_cycle() {
    let _mode = EnvGuard::set("RUSTERON_DPDK_LOCAL_IO", "adaptive");
    let _hot = EnvGuard::set("RUSTERON_DPDK_LOCAL_HOT_US", "200000");
    let mut h = Harness::new();
    let (rc, mut t) = try_init(&mut h, RCV, ([127, 0, 0, 1], 0), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    let bound = Harness::bind_addr_and_port(h.bindings, &mut t);
    let mut p = h.poller();
    p.add(&mut *t);
    let peer = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let clientd = &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void;
    let mut bytes = 0i64;
    let poll_until = |p: &mut Poller, h: &mut Harness, n: usize, bytes: &mut i64| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while h.rx_dgrams.len() < n && std::time::Instant::now() < deadline {
            p.poll(16, &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void, bytes);
        }
    };
    let _ = clientd;
    peer.send_to(&[0], &bound).unwrap();
    poll_until(&mut p, &mut h, 1, &mut bytes);
    assert_eq!(h.rx_dgrams.len(), 1, "delivered on the flag");

    // Hot: the next datagram is read on the very next polls, even with the
    // local-I/O thread's flag never observed (the poll loop runs at once).
    peer.send_to(&[1], &bound).unwrap();
    poll_until(&mut p, &mut h, 2, &mut bytes);
    assert_eq!(h.rx_dgrams.iter().map(|d| d.data[0]).collect::<Vec<_>>(), vec![0, 1]);

    // Cooled: still delivered, through the flag.
    std::thread::sleep(std::time::Duration::from_millis(300));
    peer.send_to(&[2], &bound).unwrap();
    poll_until(&mut p, &mut h, 3, &mut bytes);
    assert_eq!(h.rx_dgrams.iter().map(|d| d.data[0]).collect::<Vec<_>>(), vec![0, 1, 2]);
    p.remove(&mut *t);
    close_transport(h.bindings, &mut t);
}

/// (watched, unwatched) host-local sockets of the harness's transport.
fn local_watch(h: &Harness) -> (c_int, c_int) {
    let (mut watched, mut unwatched) = (0, 0);
    unsafe { rusteron_dpdk_transport_test_local_watch(h.native, &mut watched, &mut unwatched) };
    (watched, unwatched)
}

/// Waits up to two seconds for the local-I/O thread to reach `want`.
fn wait_local_watch(h: &Harness, want: (c_int, c_int)) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while local_watch(h) != want && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert_eq!(local_watch(h), want, "(watched, unwatched)");
}

/// Adaptive: a socket its agent reads every cycle is taken off the local-I/O
/// thread's epoll, so a busy sender's datagrams stop waking that thread (a
/// wake-up per datagram, paid in the sender's own syscall over loopback).
/// It goes back on once the agent has cooled, without losing a datagram that
/// arrived in between (re-adding it reports what is already waiting), and
/// comes off again when traffic resumes.
#[test]
#[serial]
fn adaptive_local_io_unwatches_hot_sockets() {
    let _mode = EnvGuard::set("RUSTERON_DPDK_LOCAL_IO", "adaptive");
    let _hot = EnvGuard::set("RUSTERON_DPDK_LOCAL_HOT_US", "200000");
    let mut h = Harness::new();
    let (rc, mut t) = try_init(&mut h, RCV, ([127, 0, 0, 1], 0), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    let bound = Harness::bind_addr_and_port(h.bindings, &mut t);
    let mut p = h.poller();
    p.add(&mut *t);
    let peer = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let mut bytes = 0i64;
    let poll_until = |p: &mut Poller, h: &mut Harness, n: usize, bytes: &mut i64| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while h.rx_dgrams.len() < n && std::time::Instant::now() < deadline {
            p.poll(16, &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void, bytes);
        }
    };
    assert_eq!(local_watch(&h), (1, 0), "an idle socket is watched");

    // The first datagram arrives cold, on the flag; the agent is then hot,
    // so the next one's event takes the socket off the epoll.
    peer.send_to(&[0], &bound).unwrap();
    poll_until(&mut p, &mut h, 1, &mut bytes);
    peer.send_to(&[1], &bound).unwrap();
    poll_until(&mut p, &mut h, 2, &mut bytes);
    wait_local_watch(&h, (0, 1));
    peer.send_to(&[2], &bound).unwrap();
    poll_until(&mut p, &mut h, 3, &mut bytes);
    assert_eq!(h.rx_dgrams.len(), 3, "read every cycle while unwatched");

    // Unpolled past the hot window, a datagram waits unwatched (nothing
    // flags it); the local-I/O thread puts the socket back, and that
    // reports the waiting datagram, so the cold agent still reads it.
    peer.send_to(&[3], &bound).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));
    wait_local_watch(&h, (1, 0));
    poll_until(&mut p, &mut h, 4, &mut bytes);
    assert_eq!(
        h.rx_dgrams.iter().map(|d| d.data[0]).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );
    // That read turned the agent hot again: off the epoll again.
    wait_local_watch(&h, (0, 1));
    p.remove(&mut *t);
    close_transport(h.bindings, &mut t);
    assert_eq!(local_watch(&h), (0, 0), "closed: nothing left registered");
}

/// RUSTERON_DPDK_LOCAL_UNWATCH=0 keeps a hot socket on the epoll (the
/// adaptive behaviour before sockets came off it): still delivered.
#[test]
#[serial]
fn adaptive_local_io_keeps_sockets_watched_when_told() {
    let _mode = EnvGuard::set("RUSTERON_DPDK_LOCAL_IO", "adaptive");
    let _hot = EnvGuard::set("RUSTERON_DPDK_LOCAL_HOT_US", "200000");
    let _unwatch = EnvGuard::set("RUSTERON_DPDK_LOCAL_UNWATCH", "0");
    let mut h = Harness::new();
    let (rc, mut t) = try_init(&mut h, RCV, ([127, 0, 0, 1], 0), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    let bound = Harness::bind_addr_and_port(h.bindings, &mut t);
    let mut p = h.poller();
    p.add(&mut *t);
    let peer = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let mut bytes = 0i64;
    for n in 0..3u8 {
        peer.send_to(&[n], &bound).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while h.rx_dgrams.len() <= usize::from(n) && std::time::Instant::now() < deadline {
            p.poll(16, &mut h.rx_dgrams as *mut Vec<RxDatagram> as *mut c_void, &mut bytes);
        }
    }
    assert_eq!(h.rx_dgrams.len(), 3);
    // past the local-I/O thread's next pass (10 ms): still on the epoll
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert_eq!(local_watch(&h), (1, 0), "kept watched");
    p.remove(&mut *t);
    close_transport(h.bindings, &mut t);
}

/// Spin: the agents read every host-local socket every cycle, so none is
/// registered with the epoll at all (its wake-ups would buy nothing).
#[test]
#[serial]
fn spin_local_io_registers_nothing() {
    let _mode = EnvGuard::set("RUSTERON_DPDK_LOCAL_IO", "spin");
    let mut h = Harness::new();
    let (rc, mut t) = try_init(&mut h, RCV, ([127, 0, 0, 1], 0), None, ptr::null_mut());
    assert_eq!(rc, 0, "{}", last_error());
    assert_eq!(local_watch(&h), (0, 0));
    close_transport(h.bindings, &mut t);
}
