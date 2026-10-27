//! The transport harness shared by the counters tests and the clock
//! benchmark: one native runtime (both fake ENA ports), a real Aeron counters
//! manager, the genuine bindings' send and poller callbacks, and frame helpers.

use super::*;

use std::os::raw::{c_char, c_int, c_void};
use std::ptr;

// The Aeron types and functions of the driver this crate installs into.
use rusteron_media_driver::bindings::*;

// ---------------------------------------------------------------------------
// Counter layout (AERON_COUNTERS_MANAGER_* descriptor sizes).
// ---------------------------------------------------------------------------

pub const VALUE_LENGTH: usize = 128;
pub const METADATA_LENGTH: usize = 512;
pub const COUNTER_SLOTS: usize = 128;

pub const STATE_OFFSET: usize = 0;
pub const TYPE_ID_OFFSET: usize = 4;
pub const LABEL_LENGTH_OFFSET: usize = 128;
pub const LABEL_OFFSET: usize = 132;

pub const RECORD_ALLOCATED: i32 = 1;

// Type IDs (must match native/dpdk/rusteron_dpdk_counters.h).
pub const TYPE_PORT_INFO: i32 = 75;
pub const TYPE_TRANSPORT: i32 = 76;
pub const TYPE_NOBUFS: i32 = 77;
pub const TYPE_TX_EAGAIN: i32 = 78;
pub const TYPE_ERROR: i32 = 79;
pub const TYPE_CLOCK: i32 = 80;
pub const TYPE_RX_HW_DELAY: i32 = 81;
pub const TYPE_PKTS: i32 = 82;
pub const TYPE_BYTES: i32 = 83;
pub const TYPE_MISSED_PACKETS: i32 = 84;
pub const TYPE_ARP_MISS: i32 = 85;
pub const TYPE_POLLER: i32 = 87;
pub const TYPE_QUEUE_DROP: i32 = 88;
pub const TYPE_CHECKSUM_FAILURE: i32 = 89;
pub const TYPE_FRAGMENTED: i32 = 90;
pub const TYPE_MEMPOOL_AVAILABLE: i32 = 91;
pub const TYPE_EXTENDED_STATS: i32 = 92;
pub const TYPE_UNSUPPORTED_ETHERTYPE: i32 = 93;
pub const TYPE_UNSUPPORTED_PROTOCOL: i32 = 94;
pub const TYPE_RX_RECEIVER_DISCARD: i32 = 95;

// ---------------------------------------------------------------------------
// Native test hooks (fakes + runtime).
// ---------------------------------------------------------------------------

unsafe extern "C" {
    pub fn rusteron_dpdk_transport_bindings() -> *mut aeron_udp_channel_transport_bindings_stct;
    pub fn rusteron_dpdk_fake_set_xstats(names: *const *const c_char, values: *const u64, count: u32);
}

// ---------------------------------------------------------------------------
// IPv4 sockaddr mirror (bindings.rs has no sockaddr_in on Linux either).
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SockAddrIn {
    pub sin_family: u16,
    pub sin_port: u16, // network order
    pub sin_addr: u32, // network-order bytes in memory
    pub sin_zero: [u8; 8],
}

impl SockAddrIn {
    pub const fn new(ip: [u8; 4], port: u16) -> Self {
        Self {
            sin_family: 2, // AF_INET
            sin_port: port.to_be(),
            // s_addr is read natively (no ntohl), so the octets must sit in
            // memory in order (see dpdk_tx.rs).
            sin_addr: u32::from_le_bytes(ip),
            sin_zero: [0; 8],
        }
    }
    pub const fn storage_mut(&mut self) -> *mut sockaddr_storage {
        self as *mut Self as *mut sockaddr_storage
    }
}

// ---------------------------------------------------------------------------
// Receive callback (clientd carries the Vec; the poller forwards data_paths).
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct RxDatagram {
    // Kept for debugging a failed assertion; the tests here count datagrams.
    #[allow(dead_code)]
    pub data: Vec<u8>,
}

pub unsafe extern "C" fn recv_cb(
    _data_paths: *mut aeron_udp_channel_data_paths_t,
    _transport: *mut aeron_udp_channel_transport_t,
    receiver_clientd: *mut c_void,
    _endpoint_clientd: *mut c_void,
    _destination_clientd: *mut c_void,
    buffer: *mut u8,
    length: usize,
    _addr: *mut sockaddr_storage,
    _media_timestamp: *mut timespec,
) {
    let rx = &mut *(receiver_clientd as *mut Vec<RxDatagram>);
    let data = std::slice::from_raw_parts(buffer, length).to_vec();
    rx.push(RxDatagram { data });
}

/// A receive callback that only counts datagrams into the `u64` at clientd.
pub unsafe extern "C" fn count_cb(
    _data_paths: *mut aeron_udp_channel_data_paths_t,
    _transport: *mut aeron_udp_channel_transport_t,
    receiver_clientd: *mut c_void,
    _endpoint_clientd: *mut c_void,
    _destination_clientd: *mut c_void,
    _buffer: *mut u8,
    _length: usize,
    _addr: *mut sockaddr_storage,
    _media_timestamp: *mut timespec,
) {
    *(receiver_clientd as *mut u64) += 1;
}

// ---------------------------------------------------------------------------
// Counters manager: real buffers so the native allocate/free paths run against
// real memory (and the free path's cached-clock deref has a real target).
// ---------------------------------------------------------------------------

pub struct CountersManager {
    pub manager: Box<aeron_counters_manager_stct>,
    // Never read from Rust: owns the memory the manager's cached clock points at.
    #[allow(dead_code)]
    pub clock: Box<aeron_clock_cache_stct>,
    pub metadata: Box<[u64]>,
    pub values: Box<[u64]>,
}

impl CountersManager {
    pub fn new() -> Self {
        let metadata: Box<[u64]> = vec![0u64; COUNTER_SLOTS * METADATA_LENGTH / 8].into_boxed_slice();
        let values: Box<[u64]> = vec![0u64; COUNTER_SLOTS * VALUE_LENGTH / 8].into_boxed_slice();

        let mut clock = Box::new(aeron_clock_cache_stct {
            pre_pad: [0; 56],
            cached_epoch_time: 1_000,
            cached_nano_time: 0,
            post_pad: [0; 56],
        });

        let mut manager = Box::new(unsafe { std::mem::zeroed::<aeron_counters_manager_stct>() });
        let rc = unsafe {
            aeron_counters_manager_init(
                &mut *manager,
                metadata.as_ptr() as *mut u8,
                metadata.len() * 8,
                values.as_ptr() as *mut u8,
                values.len() * 8,
                &mut *clock,
                10_000, // free_to_reuse_timeout_ms
            )
        };
        assert_eq!(rc, 0, "counters manager init failed");

        CountersManager {
            manager,
            clock,
            metadata,
            values,
        }
    }

    /// Read a live counter slot by id (value, type_id, label).
    pub fn read(&self, id: i32) -> (i64, i32, String) {
        let meta = unsafe { std::slice::from_raw_parts(self.metadata.as_ptr() as *const u8, self.metadata.len() * 8) };
        let vals = unsafe { std::slice::from_raw_parts(self.values.as_ptr() as *const u8, self.values.len() * 8) };
        let mb = id as usize * METADATA_LENGTH;
        let vb = id as usize * VALUE_LENGTH;
        let value = i64::from_le_bytes(vals[vb..vb + 8].try_into().unwrap());
        let type_id = i32::from_le_bytes(meta[mb + TYPE_ID_OFFSET..mb + TYPE_ID_OFFSET + 4].try_into().unwrap());
        let len = i32::from_le_bytes(
            meta[mb + LABEL_LENGTH_OFFSET..mb + LABEL_LENGTH_OFFSET + 4]
                .try_into()
                .unwrap(),
        );
        let label = if len > 0 {
            String::from_utf8_lossy(&meta[mb + LABEL_OFFSET..mb + LABEL_OFFSET + len as usize]).into_owned()
        } else {
            String::new()
        };
        (value, type_id, label)
    }

    /// Every live allocated counter's value with this type ID and label substring.
    pub fn find_all(&self, type_id: i32, label_needle: &str) -> Vec<i64> {
        let meta = unsafe { std::slice::from_raw_parts(self.metadata.as_ptr() as *const u8, self.metadata.len() * 8) };
        (0..COUNTER_SLOTS)
            .filter(|id| {
                let mb = id * METADATA_LENGTH;
                i32::from_le_bytes(meta[mb + STATE_OFFSET..mb + STATE_OFFSET + 4].try_into().unwrap())
                    == RECORD_ALLOCATED
            })
            .map(|id| self.read(id as i32))
            .filter(|(_, t, label)| *t == type_id && label.contains(label_needle))
            .map(|(value, _, _)| value)
            .collect()
    }

    /// The ids of every live allocated counter whose label contains `label_needle`.
    pub fn ids_labelled(&self, label_needle: &str) -> Vec<i32> {
        let meta = unsafe { std::slice::from_raw_parts(self.metadata.as_ptr() as *const u8, self.metadata.len() * 8) };
        (0..COUNTER_SLOTS as i32)
            .filter(|&id| {
                let mb = id as usize * METADATA_LENGTH;
                i32::from_le_bytes(meta[mb + STATE_OFFSET..mb + STATE_OFFSET + 4].try_into().unwrap())
                    == RECORD_ALLOCATED
                    && self.read(id).2.contains(label_needle)
            })
            .collect()
    }

    /// Find a live allocated counter by type ID and label substring.
    pub fn find(&self, type_id: i32, label_needle: &str) -> Option<(i64, i32, String)> {
        let meta = unsafe { std::slice::from_raw_parts(self.metadata.as_ptr() as *const u8, self.metadata.len() * 8) };
        for id in 0..COUNTER_SLOTS {
            let mb = id * METADATA_LENGTH;
            let state = i32::from_le_bytes(meta[mb + STATE_OFFSET..mb + STATE_OFFSET + 4].try_into().unwrap());
            if state != RECORD_ALLOCATED {
                continue;
            }
            let (value, t, label) = self.read(id as i32);
            if t == type_id && label.contains(label_needle) {
                return Some((value, t, label));
            }
        }
        None
    }
}

impl Drop for CountersManager {
    fn drop(&mut self) {
        unsafe { aeron_counters_manager_close(&mut *self.manager) };
    }
}

// ---------------------------------------------------------------------------
// Harness: one native runtime (both ENA ports) + a fake driver context whose
// counters_manager owns the counters the transport registers into.
// ---------------------------------------------------------------------------

pub struct Harness {
    pub _env: TestEnv,
    pub native: *mut c_void,
    pub bindings: *mut aeron_udp_channel_transport_bindings_stct,
    pub counters: CountersManager,
    pub context: Box<aeron_driver_context_stct>,
    pub tx: Box<aeron_udp_channel_transport_stct>,
    pub rx: Box<aeron_udp_channel_transport_stct>,
    pub data_paths: Box<aeron_udp_channel_data_paths_stct>,
    pub rx_dgrams: Vec<RxDatagram>,
    pub tx_inited: bool,
    pub rx_inited: bool,
}

impl Harness {
    pub fn new() -> Self {
        Self::build(false)
    }

    /// Both ports offer NIC RX timestamps and the transport requests them.
    pub fn with_rx_hw_timestamp() -> Self {
        Self::build(true)
    }

    pub fn build(rx_hw_timestamp: bool) -> Self {
        Self::build_with(TestEnv::new(), rx_hw_timestamp)
    }

    /// For a test that sets test hooks after resetting them: the harness
    /// takes over without a second reset.
    pub fn new_after_env() -> Self {
        Self::build_with(TestEnv::adopt(), false)
    }

    /// Over `config` instead of the default one.
    pub fn with_config(config: rusteron_dpdk_config_t) -> Self {
        Self::build_from(TestEnv::new(), config)
    }

    /// Over `config`, after the test set the fakes' hooks on `env`.
    pub fn build_after(env: TestEnv, config: rusteron_dpdk_config_t) -> Self {
        Self::build_from(env, config)
    }

    fn build_with(env: TestEnv, rx_hw_timestamp: bool) -> Self {
        let mut config = rusteron_dpdk_config_t::valid();
        if rx_hw_timestamp {
            env.set_rx_timestamp_ok(true);
            config.rx_hw_timestamp = 1;
        }
        Self::build_from(env, config)
    }

    fn build_from(env: TestEnv, config: rusteron_dpdk_config_t) -> Self {
        env.eal_skip();
        let native = create(&config).unwrap_or_else(|e| panic!("create failed: {e}"));
        let bindings = unsafe { rusteron_dpdk_transport_bindings() };
        assert!(!bindings.is_null());

        let mut counters = CountersManager::new();
        let mut context = Box::new(unsafe { std::mem::zeroed::<aeron_driver_context_stct>() });
        context.counters_manager = &mut *counters.manager;

        Harness {
            _env: env,
            native,
            bindings,
            counters,
            context,
            tx: Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() }),
            rx: Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() }),
            data_paths: Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_data_paths_stct>() }),
            rx_dgrams: Vec::new(),
            tx_inited: false,
            rx_inited: false,
        }
    }

    pub fn init_tx(&mut self, bind_ip: [u8; 4], bind_port: u16, connect: Option<([u8; 4], u16)>, mtu: usize) {
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
                &mut *self.context,
                aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
            )
        };
        assert_eq!(rc, 0, "tx init failed: {}", last_error());
    }

    pub fn init_rx(&mut self, bind_ip: [u8; 4], bind_port: u16, mtu: usize) {
        self.init_rx_with(bind_ip, bind_port, mtu, false);
    }

    /// As `init_rx`, for a channel that asks for media receive timestamps
    /// (`media-rcv-ts-offset`) when `media_timestamping`.
    pub fn init_rx_with(&mut self, bind_ip: [u8; 4], bind_port: u16, mtu: usize, media_timestamping: bool) {
        assert!(!self.rx_inited, "rx already initialized");
        self.rx_inited = true;
        let t = &mut *self.rx;
        unsafe { ptr::write_bytes(t as *mut _, 0, 1) };
        let mut bind = SockAddrIn::new(bind_ip, bind_port);
        let mut dummy = SockAddrIn::new([0, 0, 0, 0], 0);
        let mut params: aeron_udp_channel_transport_params_stct = unsafe { std::mem::zeroed() };
        params.mtu_length = mtu;
        params.is_media_timestamping = media_timestamping;
        let rc = unsafe {
            let b = &*self.bindings;
            b.init_func.unwrap()(
                t,
                bind.storage_mut(),
                dummy.storage_mut(),
                ptr::null_mut(),
                &mut params,
                &mut *self.context,
                aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
            )
        };
        assert_eq!(rc, 0, "rx init failed: {}", last_error());
    }

    /// Another receiver-affinity transport on the receiver NIC, registering
    /// into this harness's counters; close it with `close_transport`.
    pub fn init_extra_rx(&mut self, bind_port: u16) -> Box<aeron_udp_channel_transport_stct> {
        self.init_extra_rx_with(RECEIVER_IP, bind_port, false)
    }

    /// As `init_extra_rx`, bound on `bind_ip` (a host-local one runs on a
    /// kernel socket), asking for media receive timestamps when
    /// `media_timestamping`.
    pub fn init_extra_rx_with(
        &mut self,
        bind_ip: [u8; 4],
        bind_port: u16,
        media_timestamping: bool,
    ) -> Box<aeron_udp_channel_transport_stct> {
        let mut t = Box::new(unsafe { std::mem::zeroed::<aeron_udp_channel_transport_stct>() });
        let mut bind = SockAddrIn::new(bind_ip, bind_port);
        let mut dummy = SockAddrIn::new([0, 0, 0, 0], 0);
        let mut params: aeron_udp_channel_transport_params_stct = unsafe { std::mem::zeroed() };
        params.mtu_length = 1472;
        params.is_media_timestamping = media_timestamping;
        let rc = unsafe {
            (*self.bindings).init_func.unwrap()(
                &mut *t,
                bind.storage_mut(),
                dummy.storage_mut(),
                ptr::null_mut(),
                &mut params,
                &mut *self.context,
                aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
            )
        };
        assert_eq!(rc, 0, "extra rx init failed: {}", last_error());
        t
    }

    pub fn send_to(&mut self, dst: ([u8; 4], u16), payloads: &[&[u8]]) -> (c_int, i64) {
        let mut d = SockAddrIn::new(dst.0, dst.1);
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
                d.storage_mut(),
                iovs.as_ptr() as *mut iovec,
                iovs.len(),
                &mut bytes_sent,
            )
        };
        (rc, bytes_sent)
    }

    /// Send with a NULL address on the unconnected transport: the "no
    /// destination" error path.
    /// Send prebuilt iovecs to a prebuilt address (the benchmark's
    /// allocation-free path).
    pub fn send_iov(&mut self, dst: &mut SockAddrIn, iovs: &mut [iovec], bytes_sent: &mut i64) -> c_int {
        unsafe {
            let b = &*self.bindings;
            b.send_func.unwrap()(
                &mut *self.data_paths,
                &mut *self.tx,
                dst.storage_mut(),
                iovs.as_mut_ptr(),
                iovs.len(),
                bytes_sent,
            )
        }
    }

    pub fn send_null_addr(&mut self, payloads: &[&[u8]]) -> (c_int, i64) {
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
                ptr::null_mut(),
                iovs.as_ptr() as *mut iovec,
                iovs.len(),
                &mut bytes_sent,
            )
        };
        (rc, bytes_sent)
    }

    pub fn arp_seed(&self, ip: [u8; 4], mac: [u8; 6]) {
        let ip_str = std::ffi::CString::new(format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])).unwrap();
        let rc = unsafe { rusteron_dpdk_transport_test_arp_seed(self.native, ip_str.as_ptr(), mac.as_ptr()) };
        assert_eq!(rc, 0, "arp_seed failed: {}", last_error());
    }

    pub fn poller(&self) -> Poller {
        Poller::new(self.bindings)
    }

    /// The `address:port` a transport reports it is bound to.
    pub fn bind_addr_and_port(
        bindings: *mut aeron_udp_channel_transport_bindings_stct,
        transport: &mut Box<aeron_udp_channel_transport_stct>,
    ) -> String {
        let mut buf = [0 as c_char; 128];
        let rc = unsafe { (*bindings).bind_addr_and_port_func.unwrap()(&mut **transport, buf.as_mut_ptr(), buf.len()) };
        assert_eq!(rc, 0, "bind_addr_and_port failed: {}", last_error());
        unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    }

    pub fn counter(&self, type_id: i32, needle: &str) -> i64 {
        self.counters
            .find(type_id, needle)
            .unwrap_or_else(|| panic!("no counter type={type_id} label~{needle}"))
            .0
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

pub struct Poller {
    pub inner: Box<aeron_udp_transport_poller_stct>,
    pub bindings: *mut aeron_udp_channel_transport_bindings_stct,
    pub inited: bool,
    pub added: Vec<*mut aeron_udp_channel_transport_t>,
}

impl Poller {
    pub fn new(bindings: *mut aeron_udp_channel_transport_bindings_stct) -> Self {
        Self {
            inner: Box::new(unsafe { std::mem::zeroed::<aeron_udp_transport_poller_stct>() }),
            bindings,
            inited: false,
            added: Vec::new(),
        }
    }

    pub fn add(&mut self, transport: *mut aeron_udp_channel_transport_t) {
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

    pub fn remove(&mut self, transport: *mut aeron_udp_channel_transport_t) {
        let rc = unsafe {
            let b = &*self.bindings;
            b.poller_remove_func.unwrap()(&mut *self.inner, transport)
        };
        assert_eq!(rc, 0, "poller_remove failed: {}", last_error());
        self.added.retain(|t| *t != transport);
    }

    /// One poll into a caller-owned `msgvec`, dispatching to `cb` (the
    /// benchmark's allocation-free path).
    pub fn poll_into(
        &mut self,
        msgvec: &mut [RealMmsghdr],
        cb: aeron_udp_transport_recv_func_t,
        clientd: *mut c_void,
        bytes_rcved: &mut i64,
    ) -> c_int {
        unsafe {
            let b = &*self.bindings;
            b.poller_poll_func.unwrap()(
                &mut *self.inner,
                msgvec.as_mut_ptr() as *mut mmsghdr,
                msgvec.len(),
                bytes_rcved,
                cb,
                None,
                clientd,
            )
        }
    }

    pub fn poll(&mut self, vlen: usize, clientd: *mut c_void, bytes_rcved: &mut i64) -> c_int {
        let mut msgvec = vec![unsafe { std::mem::zeroed::<RealMmsghdr>() }; vlen];
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
pub struct RealMmsghdr {
    pub msg_hdr: msghdr,
    pub msg_len: u32,
    pub _pad: u32,
}

// ---------------------------------------------------------------------------
// Frame helpers (real checksums so rx_ol_flags = 0 exercises software verify).
// ---------------------------------------------------------------------------

pub const RECEIVER_PORT: u16 = 1;
pub const RECEIVER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x02];
pub const RECEIVER_IP: [u8; 4] = [10, 0, 1, 1];
pub const SENDER_IP: [u8; 4] = [10, 0, 0, 1];
pub const PEER_IP: [u8; 4] = [10, 0, 0, 5];
pub const PEER_MAC: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x50];

pub const RX_GOOD: u32 = 0b11; // IPV4_CKSUM_GOOD | UDP_CKSUM_GOOD

pub fn ones_sum(words: &[u16]) -> u32 {
    let mut sum: u32 = 0;
    for w in words {
        sum += *w as u32;
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    sum
}

/// A valid IPv4/UDP frame: real IPv4 + UDP checksums, DF set, unicast MACs.
pub fn build_udp_frame(
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
    let hdr = f[14..34].to_vec();
    let mut words = Vec::new();
    for i in 0..10 {
        words.push(((hdr[i * 2] as u16) << 8) | hdr[i * 2 + 1] as u16);
    }
    let ipc = !(ones_sum(&words) as u16);
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

pub fn udp_csum(src_ip: [u8; 4], dst_ip: [u8; 4], sport: u16, dport: u16, payload: &[u8]) -> u16 {
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

/// A frame addressed to the receiver transport on the given dport.
pub fn frame_to_receiver(dport: u16, payload: &[u8]) -> Vec<u8> {
    build_udp_frame(RECEIVER_MAC, PEER_MAC, PEER_IP, RECEIVER_IP, 7000, dport, payload)
}

pub fn close_transport(
    bindings: *mut aeron_udp_channel_transport_bindings_stct,
    transport: &mut Box<aeron_udp_channel_transport_stct>,
) {
    unsafe { (*bindings).close_func.unwrap()(&mut **transport) };
}
