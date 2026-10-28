//! The DPDK port under the smoltcp stack: EAL and port start-up, and
//! smoltcp's [`Device`] over one RX and one TX queue.
//!
//! Its own thin binding (`native/dpdk/ergon_io_dpdk.c`), not ergon-md-dpdk's
//! port code: that C compiles against the Aeron headers rusteron's media
//! driver build exports and fills in Aeron's transport bindings, so reusing it
//! would make ergon-io build Aeron. The shim borrows its choices (one queue
//! pair, `rte_timer_alt_manage` for ENA's watchdog, the caller's affinity
//! restored after `rte_eal_init`).
//!
//! - Receive: one `rte_eth_rx_burst` of up to [`RX_BURST`] frames per stack
//!   pass; smoltcp parses each frame where the NIC wrote it (the mbuf), and
//!   the mbuf is freed when smoltcp is done with it. Chained mbufs are
//!   dropped (an mbuf holds a whole frame).
//! - Transmit: smoltcp writes each frame straight into a fresh mbuf; frames
//!   collect in a queue of [`TX_QUEUE`] and leave in one `rte_eth_tx_burst`
//!   at the end of the pass. Frames the NIC refuses stay queued for the next
//!   burst. With checksum offload the shim sets the mbuf's flags and the
//!   pseudo-header sum.
//! - Nothing here allocates after start-up: mbufs come from the pool, the
//!   queues are fixed arrays.
#![allow(unsafe_code)]

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::io;

use smoltcp::phy::{Checksum, ChecksumCapabilities, Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;

/// Frames one RX burst may return.
pub const RX_BURST: usize = 32;
/// Frames queued for one TX burst.
pub const TX_QUEUE: usize = 128;

/// `eio_port_t`.
#[repr(C)]
#[derive(Clone, Copy)]
struct PortInfo {
    port_id: u16,
    mac: [u8; 6],
    mtu: u16,
    tx_csum: c_int,
    data_room: u32,
    rx_desc: u16,
    tx_desc: u16,
    driver: [c_char; 32],
    pool: *mut c_void,
}

/// `eio_rx_t`.
#[repr(C)]
#[derive(Clone, Copy)]
struct RxDesc {
    mbuf: *mut c_void,
    data: *const u8,
    len: u32,
    segs: u32,
}

const EMPTY_RX: RxDesc = RxDesc {
    mbuf: std::ptr::null_mut(),
    data: std::ptr::null(),
    len: 0,
    segs: 0,
};

unsafe extern "C" {
    fn eio_last_error() -> *const c_char;
    fn eio_eal_init(argc: c_int, argv: *mut *mut c_char) -> c_int;
    fn eio_lcores_arg(out: *mut c_char, len: usize) -> c_int;
    fn eio_thread_attach() -> c_int;
    fn eio_thread_detach();
    fn eio_port_start(
        name: *const c_char,
        ip_mtu: u16,
        rx_desc: u16,
        tx_desc: u16,
        nb_mbufs: u32,
        want_tx_csum: c_int,
        link_wait_ms: u32,
        out: *mut PortInfo,
    ) -> c_int;
    fn eio_rx_burst(port: u16, out: *mut RxDesc, n: u16) -> u16;
    fn eio_mbuf_free(m: *mut c_void);
    fn eio_tx_alloc(pool: *mut c_void, len: u16, mbuf: *mut *mut c_void) -> *mut u8;
    fn eio_tx_offload(m: *mut c_void, l3: u16, proto: u8);
    fn eio_tx_burst(port: u16, mbufs: *mut *mut c_void, n: u16) -> u16;
    fn eio_timer_service() -> c_int;
    fn eio_port_stats(port: u16, out: *mut u64) -> c_int;
}

fn last_error() -> String {
    // SAFETY: the shim returns its static, NUL-terminated error buffer.
    unsafe { CStr::from_ptr(eio_last_error()) }
        .to_string_lossy()
        .into_owned()
}

/// Whether `args` already has an option starting with one of `names`.
fn has(args: &[String], names: &[&str]) -> bool {
    args.iter().any(|a| {
        names
            .iter()
            .any(|n| a == n || (n.starts_with("--") && a.starts_with(&format!("{n}="))))
    })
}

/// The EAL arguments the stack passes: the caller's, plus `--lcores` (lcore
/// 0 on the calling thread's CPUs), `--no-telemetry` and `--in-memory`
/// (`--no-shconf` with `--no-huge`) unless the caller set them or something
/// they exclude.
pub fn eal_argv(args: &[String], lcores: &str) -> Vec<String> {
    let mut v = vec!["ergon-io".to_owned()];
    v.extend(args.iter().cloned());
    if !has(args, &["-l", "-c", "--lcores"]) && !args.iter().any(|a| is_short_lcore(a)) {
        v.push(format!("--lcores={lcores}"));
    }
    if !has(args, &["--no-telemetry", "--telemetry"]) {
        v.push("--no-telemetry".into());
    }
    if !has(args, &["--in-memory", "--file-prefix", "--no-shconf"]) {
        // --no-huge implies legacy memory, which --in-memory refuses.
        v.push(
            if has(args, &["--no-huge"]) {
                "--no-shconf"
            } else {
                "--in-memory"
            }
            .into(),
        );
    }
    v
}

/// `-l0-3`, `-c0x3`: the short lcore options with their value attached.
fn is_short_lcore(a: &str) -> bool {
    (a.starts_with("-l") || a.starts_with("-c")) && a.len() > 2 && !a.starts_with("--")
}

/// Starts DPDK's EAL once per process with `args` (see [`eal_argv`]).
/// `Ok(false)`: it was already up (the arguments are not used).
pub fn eal_init(user: &[String]) -> io::Result<bool> {
    let mut lc = [0 as c_char; 512];
    // SAFETY: `lc` is writable for its length; the shim NUL-terminates.
    let lcores = if unsafe { eio_lcores_arg(lc.as_mut_ptr(), lc.len()) } == 0 {
        // SAFETY: NUL-terminated by the shim on success.
        unsafe { CStr::from_ptr(lc.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    } else {
        "0".to_owned()
    };
    let full = eal_argv(user, &lcores);
    let owned: Vec<CString> = full
        .iter()
        .map(|a| CString::new(a.as_str()))
        .collect::<Result<_, _>>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "an EAL argument holds a NUL"))?;
    // EAL may keep pointers into argv (device arguments), and runs once per
    // process: the strings live for the process.
    let ptrs: &'static mut [*mut c_char] = Box::leak(
        owned
            .into_iter()
            .map(CString::into_raw)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    );
    let count =
        c_int::try_from(ptrs.len()).map_err(|_| io::Error::other("too many EAL arguments"))?;
    // SAFETY: `ptrs` holds `count` valid C strings for the process's life.
    match unsafe { eio_eal_init(count, ptrs.as_mut_ptr()) } {
        0 => Ok(true),
        1 => Ok(false),
        _ => Err(io::Error::other(format!(
            "DPDK EAL ({}): {}",
            full.join(" "),
            last_error()
        ))),
    }
}

/// Gives the calling thread an lcore id (unless it has one).
pub fn thread_attach() -> io::Result<()> {
    // SAFETY: no arguments; EAL is up.
    if unsafe { eio_thread_attach() } == 0 {
        Ok(())
    } else {
        Err(io::Error::other(
            "cannot register the polling thread with DPDK (out of lcores?)",
        ))
    }
}

/// Gives back the lcore id [`thread_attach`] took.
pub fn thread_detach() {
    // SAFETY: no arguments; a no-op on threads without a registered lcore.
    unsafe { eio_thread_detach() }
}

/// The PMDs' due timers (ENA's watchdog).
pub fn timer_service() {
    // SAFETY: no arguments; never blocks.
    let _ = unsafe { eio_timer_service() };
}

/// The started port.
pub struct Port {
    pub id: u16,
    pub mac: [u8; 6],
    pub mtu: u16,
    pub tx_csum: bool,
    pub driver: String,
    pool: *mut c_void,
}

/// Starts `name` once per process (later calls return the started port).
pub fn port_start(cfg: &crate::smoltcp_dpdk::StackConfig) -> io::Result<Port> {
    let name = CString::new(cfg.port.as_str())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "the port name holds a NUL"))?;
    let mut out = PortInfo {
        port_id: 0,
        mac: [0; 6],
        mtu: 0,
        tx_csum: 0,
        data_room: 0,
        rx_desc: 0,
        tx_desc: 0,
        driver: [0; 32],
        pool: std::ptr::null_mut(),
    };
    // SAFETY: `name` is a C string and `out` a writable `eio_port_t`.
    let rc = unsafe {
        eio_port_start(
            name.as_ptr(),
            cfg.mtu,
            cfg.rx_desc,
            cfg.tx_desc,
            cfg.mbufs,
            c_int::from(cfg.tx_checksum_offload),
            cfg.link_wait_ms,
            &raw mut out,
        )
    };
    if rc != 0 || out.pool.is_null() {
        return Err(io::Error::other(last_error()));
    }
    // SAFETY: the shim NUL-terminates `driver`.
    let driver = unsafe { CStr::from_ptr(out.driver.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    Ok(Port {
        id: out.port_id,
        mac: out.mac,
        mtu: out.mtu,
        tx_csum: out.tx_csum != 0,
        driver,
        pool: out.pool,
    })
}

/// The port's packet counters: received, sent, receive errors, send
/// errors, missed (RX ring full), RX mbuf allocation failures.
pub fn port_stats(port: u16) -> Option<[u64; 6]> {
    let mut out = [0u64; 6];
    // SAFETY: `out` holds the six counters the shim writes.
    (unsafe { eio_port_stats(port, out.as_mut_ptr()) } == 0).then_some(out)
}

/// Receive-side counters.
#[derive(Debug, Default, Clone, Copy)]
pub struct RxStats {
    pub bursts: u64,
    pub frames: u64,
    pub chained: u64,
}

/// Transmit-side counters.
#[derive(Debug, Default, Clone, Copy)]
pub struct TxStats {
    pub frames: u64,
    pub no_mbuf: u64,
    pub ring_full: u64,
    pub retransmits: u64,
}

/// smoltcp's device over the port.
pub struct Dev {
    port: u16,
    rx: [RxDesc; RX_BURST],
    rx_n: usize,
    rx_i: usize,
    /// This pass made its RX burst: `receive` returns `None` once the
    /// frames it brought are consumed, so a pass's work is bounded.
    pub burst_done: bool,
    /// A frame to hand smoltcp before the NIC's (a pinned ARP answer).
    inject: [u8; 64],
    inject_len: usize,
    pub tx: TxSide,
    caps: DeviceCapabilities,
    pub rx_stats: RxStats,
    /// Stamp each non-empty burst with `CLOCK_REALTIME`.
    pub stamp: bool,
    /// When the last non-empty burst returned (`CLOCK_REALTIME` ns).
    pub burst_ns: i64,
}

/// The transmit side, split off so a receive token pair can hold it.
pub struct TxSide {
    port: u16,
    pool: *mut c_void,
    queue: [*mut c_void; TX_QUEUE],
    queued: usize,
    offload: bool,
    /// A frame's room when the pool is empty: smoltcp still writes it, and
    /// it is dropped.
    scratch: Box<[u8]>,
    /// Per local port: the end of the highest TCP sequence sent.
    seq_end: Box<[u32]>,
    /// Per local port: retransmissions counted.
    retx: Box<[u32]>,
    pub stats: TxStats,
}

impl Dev {
    /// The device over `port`, with frames of `ip_mtu`.
    pub fn new(port: &Port) -> Self {
        let mut checksum = ChecksumCapabilities::default();
        if port.tx_csum {
            // Received checksums are still checked in software; the NIC
            // fills the transmitted ones.
            checksum.ipv4 = Checksum::Rx;
            checksum.tcp = Checksum::Rx;
            checksum.udp = Checksum::Rx;
        }
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ethernet;
        caps.max_transmission_unit = usize::from(port.mtu) + 14;
        caps.checksum = checksum;
        Self {
            port: port.id,
            rx: [EMPTY_RX; RX_BURST],
            rx_n: 0,
            rx_i: 0,
            burst_done: false,
            inject: [0; 64],
            inject_len: 0,
            tx: TxSide {
                port: port.id,
                pool: port.pool,
                queue: [std::ptr::null_mut(); TX_QUEUE],
                queued: 0,
                offload: port.tx_csum,
                scratch: vec![0u8; usize::from(port.mtu) + 64].into_boxed_slice(),
                seq_end: vec![0u32; 1 << 16].into_boxed_slice(),
                retx: vec![0u32; 1 << 16].into_boxed_slice(),
                stats: TxStats::default(),
            },
            caps,
            rx_stats: RxStats::default(),
            stamp: true,
            burst_ns: 0,
        }
    }

    /// Whether the NIC fills the transmitted checksums.
    pub const fn tx_offload(&self) -> bool {
        self.tx.offload
    }

    /// Hands smoltcp an ARP reply from `peer` (`peer_mac`) to us before the
    /// next received frame: what a pinned neighbour answers.
    pub fn inject_arp_reply(
        &mut self,
        mac: [u8; 6],
        ip: [u8; 4],
        peer_mac: [u8; 6],
        peer: [u8; 4],
    ) {
        self.inject = [0; 64];
        arp_frame(&mut self.inject, 2, peer_mac, peer, mac, mac, ip);
        self.inject_len = 60;
    }
}

/// Writes an ARP frame (60 bytes, padded) into `f`: operation `op` (1
/// request, 2 reply) from `src_mac`/`src_ip` to `dst_mac` (the Ethernet
/// destination) about `target_ip`, whose MAC field is `dst_mac` for a reply
/// and zero for a request.
fn arp_frame(
    f: &mut [u8],
    op: u8,
    src_mac: [u8; 6],
    src_ip: [u8; 4],
    dst_mac: [u8; 6],
    target_mac: [u8; 6],
    target_ip: [u8; 4],
) {
    f[0..6].copy_from_slice(&dst_mac);
    f[6..12].copy_from_slice(&src_mac);
    f[12..14].copy_from_slice(&[0x08, 0x06]);
    f[14..22].copy_from_slice(&[0, 1, 0x08, 0x00, 6, 4, 0, op]);
    f[22..28].copy_from_slice(&src_mac);
    f[28..32].copy_from_slice(&src_ip);
    if op == 2 {
        f[32..38].copy_from_slice(&target_mac);
    } else {
        f[32..38].fill(0);
    }
    f[38..42].copy_from_slice(&target_ip);
    f[42..60].fill(0);
}

impl TxSide {
    /// Sends the queued frames; what the NIC refuses stays queued.
    pub fn flush(&mut self) {
        if self.queued == 0 {
            return;
        }
        let n = u16::try_from(self.queued).unwrap_or(u16::MAX);
        // SAFETY: the first `queued` entries are mbufs this side owns; the
        // NIC takes a prefix of them.
        let sent = usize::from(unsafe { eio_tx_burst(self.port, self.queue.as_mut_ptr(), n) });
        if sent < self.queued {
            self.queue.copy_within(sent..self.queued, 0);
        }
        self.queued -= sent;
    }

    /// Queues an ARP request for `target` (broadcast).
    pub fn send_arp_request(&mut self, mac: [u8; 6], ip: [u8; 4], target: [u8; 4]) {
        self.emit(60, |f| arp_frame(f, 1, mac, ip, [0xff; 6], [0; 6], target));
    }

    /// The retransmissions counted on local port `port`.
    pub fn retransmits(&self, port: u16) -> u32 {
        self.retx[usize::from(port)]
    }

    /// Forgets local port `port`'s sequence and count (a new connection).
    pub fn reset_port(&mut self, port: u16) {
        self.seq_end[usize::from(port)] = 0;
        self.retx[usize::from(port)] = 0;
    }

    /// Builds a frame of `len` bytes with `f` in a fresh mbuf and queues
    /// it; with no mbuf or no room, `f` writes the scratch frame, which is
    /// dropped and counted.
    fn emit<R>(&mut self, len: usize, f: impl FnOnce(&mut [u8]) -> R) -> R {
        if self.queued == TX_QUEUE {
            self.flush();
        }
        let len16 = u16::try_from(len).unwrap_or(u16::MAX);
        let mut m: *mut c_void = std::ptr::null_mut();
        let p = if self.queued < TX_QUEUE && len <= self.scratch.len() {
            // SAFETY: `pool` is the started port's pool; `m` receives the
            // mbuf on success.
            unsafe { eio_tx_alloc(self.pool, len16, &raw mut m) }
        } else {
            std::ptr::null_mut()
        };
        if p.is_null() {
            if self.queued == TX_QUEUE {
                self.stats.ring_full += 1;
            } else {
                self.stats.no_mbuf += 1;
            }
            let n = len.min(self.scratch.len());
            return f(&mut self.scratch[..n]);
        }
        // SAFETY: the shim appended `len` bytes to the fresh mbuf at `p`;
        // this side owns the mbuf until it is queued and burst.
        let buf = unsafe { std::slice::from_raw_parts_mut(p, len) };
        let r = f(buf);
        if let Some((l3, proto)) = self.inspect(buf)
            && self.offload
        {
            // SAFETY: `m` holds the IPv4 frame just written, `l3` its
            // header length.
            unsafe { eio_tx_offload(m, l3, proto) };
        }
        self.queue[self.queued] = m;
        self.queued += 1;
        self.stats.frames += 1;
        r
    }

    /// Reads an outgoing IPv4 frame's header: counts a TCP segment that
    /// resends sequence space already sent on its local port (not a
    /// keepalive probe), and returns
    /// the IP header's length and protocol for a TCP or UDP frame (the
    /// checksum offload's input).
    fn inspect(&mut self, f: &[u8]) -> Option<(u16, u8)> {
        if f.len() < 34 || f[12..14] != [0x08, 0x00] {
            return None;
        }
        let ip = &f[14..];
        let ihl = usize::from(ip[0] & 0x0f) * 4;
        let proto = ip[9];
        if ihl < 20 || ip.len() < ihl + 8 {
            return None;
        }
        match proto {
            6 if ip.len() >= ihl + 20 => {
                let total = usize::from(u16::from_be_bytes([ip[2], ip[3]]));
                let tcp = &ip[ihl..];
                let sport = u16::from_be_bytes([tcp[0], tcp[1]]);
                let seq = u32::from_be_bytes([tcp[4], tcp[5], tcp[6], tcp[7]]);
                let doff = usize::from(tcp[12] >> 4) * 4;
                let flags = tcp[13];
                let (fin, syn, rst, ack) = (flags & 1, flags & 2, flags & 4, flags & 16);
                let seg =
                    total.saturating_sub(ihl + doff) + usize::from(fin) + usize::from(syn >> 1);
                if rst == 0 && seg > 0 {
                    let end = seq.wrapping_add(u32::try_from(seg).unwrap_or(0));
                    let last = &mut self.seq_end[usize::from(sport)];
                    // A SYN without ACK opens a connection: a new one unless
                    // it repeats the last SYN exactly.
                    let again = if syn != 0 && ack == 0 {
                        *last == end
                    } else {
                        end.wrapping_sub(*last).cast_signed() <= 0
                    };
                    // smoltcp's keepalive probe: one byte below the last sent,
                    // a bare ACK (data it sends carries PSH).
                    let probe = seg == 1 && flags == 0x10 && seq == last.wrapping_sub(1);
                    if again && !probe {
                        self.stats.retransmits += 1;
                        let r = &mut self.retx[usize::from(sport)];
                        *r = r.saturating_add(1);
                    } else {
                        *last = end;
                    }
                }
                Some((u16::try_from(ihl).unwrap_or(20), 6))
            }
            17 => Some((u16::try_from(ihl).unwrap_or(20), 17)),
            _ => None,
        }
    }
}

impl Drop for TxSide {
    fn drop(&mut self) {
        for &m in &self.queue[..self.queued] {
            // SAFETY: queued mbufs are this side's.
            unsafe { eio_mbuf_free(m) };
        }
        self.queued = 0;
    }
}

impl Drop for Dev {
    fn drop(&mut self) {
        for d in &self.rx[self.rx_i..self.rx_n] {
            // SAFETY: received mbufs not yet handed to smoltcp are ours.
            unsafe { eio_mbuf_free(d.mbuf) };
        }
        self.rx_i = self.rx_n;
    }
}

/// One received frame: an mbuf the NIC wrote, or an injected frame.
pub enum RxTok {
    Mbuf {
        m: *mut c_void,
        data: *const u8,
        len: usize,
    },
    Local {
        buf: [u8; 64],
        len: usize,
    },
}

impl smoltcp::phy::RxToken for RxTok {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&[u8]) -> R,
    {
        match &self {
            // SAFETY: `data..data+len` is the frame in an mbuf this token
            // owns until it drops (after `f`).
            Self::Mbuf { data, len, .. } => f(unsafe { std::slice::from_raw_parts(*data, *len) }),
            Self::Local { buf, len } => f(&buf[..*len]),
        }
    }
}

impl Drop for RxTok {
    fn drop(&mut self) {
        if let Self::Mbuf { m, .. } = self {
            // SAFETY: the token owns the mbuf; smoltcp is done with it.
            unsafe { eio_mbuf_free(*m) };
        }
    }
}

/// Room for one frame on the transmit side.
pub struct TxTok<'a> {
    tx: &'a mut TxSide,
}

impl smoltcp::phy::TxToken for TxTok<'_> {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        self.tx.emit(len, f)
    }
}

impl Device for Dev {
    type RxToken<'a> = RxTok;
    type TxToken<'a> = TxTok<'a>;

    fn receive(&mut self, _now: Instant) -> Option<(RxTok, TxTok<'_>)> {
        if self.inject_len > 0 {
            let len = std::mem::take(&mut self.inject_len);
            return Some((
                RxTok::Local {
                    buf: self.inject,
                    len,
                },
                TxTok { tx: &mut self.tx },
            ));
        }
        loop {
            if self.rx_i == self.rx_n {
                if self.burst_done {
                    return None;
                }
                self.burst_done = true;
                let n = u16::try_from(RX_BURST).unwrap_or(u16::MAX);
                // SAFETY: `rx` holds RX_BURST descriptors the shim fills.
                self.rx_n =
                    usize::from(unsafe { eio_rx_burst(self.port, self.rx.as_mut_ptr(), n) });
                self.rx_i = 0;
                if self.rx_n == 0 {
                    return None;
                }
                self.rx_stats.bursts += 1;
                self.rx_stats.frames += self.rx_n as u64;
                if self.stamp {
                    self.burst_ns = crate::clock::realtime_ns();
                }
            }
            let d = self.rx[self.rx_i];
            self.rx_i += 1;
            if d.segs != 1 {
                self.rx_stats.chained += 1;
                // SAFETY: the descriptor's mbuf is ours.
                unsafe { eio_mbuf_free(d.mbuf) };
                continue;
            }
            return Some((
                RxTok::Mbuf {
                    m: d.mbuf,
                    data: d.data,
                    len: d.len as usize,
                },
                TxTok { tx: &mut self.tx },
            ));
        }
    }

    fn transmit(&mut self, _now: Instant) -> Option<TxTok<'_>> {
        if self.tx.queued == TX_QUEUE {
            self.tx.flush();
            if self.tx.queued == TX_QUEUE {
                return None;
            }
        }
        Some(TxTok { tx: &mut self.tx })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        self.caps.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eal_defaults_are_added_unless_given() {
        let a = |s: &[&str]| s.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        let v = eal_argv(&a(&["-a", "0000:00:06.0"]), "0@(2)");
        assert_eq!(
            v,
            a(&[
                "ergon-io",
                "-a",
                "0000:00:06.0",
                "--lcores=0@(2)",
                "--no-telemetry",
                "--in-memory"
            ])
        );
        let v = eal_argv(&a(&["--no-huge", "-m", "512"]), "0@(2)");
        assert_eq!(
            v,
            a(&[
                "ergon-io",
                "--no-huge",
                "-m",
                "512",
                "--lcores=0@(2)",
                "--no-telemetry",
                "--no-shconf"
            ])
        );
        let v = eal_argv(&a(&["-l", "3", "--file-prefix=x", "--telemetry"]), "0@(2)");
        assert_eq!(
            v,
            a(&["ergon-io", "-l", "3", "--file-prefix=x", "--telemetry"])
        );
        let v = eal_argv(&a(&["-l0-1", "--lcores=1", "--no-shconf"]), "0@(2)");
        assert_eq!(
            v,
            a(&[
                "ergon-io",
                "-l0-1",
                "--lcores=1",
                "--no-shconf",
                "--no-telemetry"
            ])
        );
    }

    #[test]
    fn an_arp_request_and_reply_have_their_fields() {
        let mut f = [0xaau8; 64];
        arp_frame(
            &mut f,
            1,
            [2, 0, 0, 0, 0, 1],
            [10, 0, 0, 2],
            [0xff; 6],
            [9; 6],
            [10, 0, 0, 1],
        );
        assert_eq!(&f[0..6], &[0xff; 6]);
        assert_eq!(&f[12..14], &[8, 6]);
        assert_eq!(f[21], 1);
        assert_eq!(&f[32..38], &[0; 6], "a request's target MAC is zero");
        assert_eq!(&f[38..42], &[10, 0, 0, 1]);
        assert_eq!(&f[42..60], &[0; 18]);
        arp_frame(
            &mut f,
            2,
            [2, 0, 0, 0, 0, 9],
            [10, 0, 0, 1],
            [2, 0, 0, 0, 0, 1],
            [2, 0, 0, 0, 0, 1],
            [10, 0, 0, 2],
        );
        assert_eq!(f[21], 2);
        assert_eq!(&f[32..38], &[2, 0, 0, 0, 0, 1]);
    }
}
