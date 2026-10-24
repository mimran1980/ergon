//! The raw floor: `rtt`'s cross-host round trip with no Aeron at all, so an
//! idle round trip's tail can be split between what lies under Aeron (NIC,
//! PMD or kernel, hypervisor, fabric) and Aeron's own driver and client.
//! Two wires, chosen as the media driver chooses its transport
//! (`RUSTERON_MEDIA_DRIVER_TRANSPORT`, the lab's `driver.env`):
//!
//! - unset or `default`: kernel UDP. Two non-blocking sockets, one bound to
//!   this side's endpoint and read by a busy-spinning `recv`, one connected to
//!   the peer's, as Aeron's kernel transport has them (the lab's `kernel-raw`
//!   runs on `kernel-busy`'s host settings: `busy_read`, `busy_poll`).
//! - `dpdk-ena`: ergon-md-dpdk's packet path alone. A [`DpdkTransport`]
//!   configured from the environment as the driver's is (`RUSTERON_DPDK_*`),
//!   its bindings' `send_func` on a sender-port transport and its poller over
//!   that and a receiver-port transport, on this one thread, as an INVOKER
//!   driver runs them, with counters registered as a driver's are: NIC, PMD
//!   and transport, no Aeron driver or client.
//!
//! floor xpong <ping endpoint> <pong endpoint> <pong cpu|->
//! floor xping <ping endpoint> <pong endpoint> <messages> <warmup> <ping cpu|->
//! floor xhairpin <ping endpoint> <pong endpoint> <messages> <warmup> <ping cpu|-> <pong cpu|->
//!
//! The arguments, the pacing (each ping sent once the last one's echo is
//! back, or one every `RTT_GAP_US`), the warm-up, the clock (`lab::Clock`,
//! its ordered read at both ends), the CPU pinning and every output are
//! `rtt`'s (`roundtrip.rs`): the `rtt,...` line, `RTT_HISTOGRAM`,
//! `LOAD_SPIKES`, `LOOP_STALLS_DIR`, under `LABEL`, `RTT_SECONDS`.
//!
//! `xhairpin` runs both ends in one process on one host, the pong on a
//! thread of its own (its CPU the last argument), each end on its own data
//! NIC: the pong listens on the ping endpoint (the receiver NIC's address),
//! the ping on the pong endpoint (the sender NIC's), and each sends from its
//! own address, so a round trip leaves the host by one NIC and comes back
//! by the other, never between hosts. On the kernel wire each end's send
//! socket is bound to its own address (the host's policy routes then take
//! it out of that NIC; the lab's vm.sh `HAIRPIN=1` routes a local
//! destination so, which the kernel would otherwise deliver over loopback);
//! on the DPDK wire the ping has the sender port's transport and the pong
//! the receiver port's, each with a poller of its own, as an Aeron
//! driver's sender and receiver threads have them. The pong ends when the
//! ping's run does. A datagram is 64 bytes, what an Aeron data frame
//! of `rtt`'s 32-byte message is on the wire: 32 bytes where Aeron's header
//! would be, the send stamp and the sequence after it. Nothing repairs a lost
//! one, so a ping whose echo has not come within `RTT_LOST_TIMEOUT_US`
//! (50 ms when unset or 0) is counted lost and the next is sent.
//!
//! Before the run xping probes until the pong echoes, for up to
//! `FLOOR_CONNECT_SECS` (30), and fails saying so if it never does. xpong
//! echoes until SIGTERM, then writes its stalls and says why it ended. With
//! `RUSTERON_HARNESS_START_FILE` set (the vdev runner's) a DPDK side waits
//! for that file, its taps bridged, before it sends.

#[path = "../lab.rs"]
#[allow(dead_code)]
mod lab;
#[path = "../roundtrip.rs"]
mod roundtrip;

use lab::{Clock, STOP, cpu_arg, pin};
use roundtrip::{Pace, Results, Stalls};
use std::hint::spin_loop;
use std::io;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

/// The spikes' stream column, as `rtt`'s.
const PING_STREAM_ID: i32 = 1002;
/// An Aeron data frame of `rtt`'s 32-byte message: its 32-byte header, then
/// the message.
const DATAGRAM: usize = 64;
const STAMP: usize = 32;
const SEQ: usize = 40;
/// Datagrams one poll takes at most: the lab's receiver IO vectors.
const BURST: usize = 16;

/// What a floor client sends on and polls.
trait Wire {
    /// Sends `datagram` to the peer: `false` when it cannot go yet (retry).
    fn send(&mut self, datagram: &[u8]) -> io::Result<bool>;
    /// Polls once, giving each datagram received to `on`: how many.
    fn poll<F: FnMut(&[u8])>(&mut self, on: F) -> io::Result<usize>;
}

fn endpoint(arg: Option<String>) -> SocketAddrV4 {
    let arg = arg.expect("host:port");
    arg.parse()
        .unwrap_or_else(|e| panic!("endpoint {arg}: {e} (an IPv4 host:port)"))
}

fn main() {
    lab::lock_memory();
    let mut args = std::env::args().skip(1);
    let mode = args.next().expect("xping|xpong|xhairpin");
    let (ping, pong) = (endpoint(args.next()), endpoint(args.next()));
    let (messages, warmup) = if mode == "xping" || mode == "xhairpin" {
        let number = |a: Option<String>| -> usize { a.expect("messages and warmup").parse().unwrap() };
        (number(args.next()), number(args.next()))
    } else {
        (0, 0)
    };
    let cpu = cpu_arg(args.next());
    if mode == "xhairpin" {
        hairpin(ping, pong, messages, warmup, cpu, cpu_arg(args.next()));
        return;
    }
    // each side listens on its own endpoint and sends to the other's
    let (listen, peer) = match mode.as_str() {
        "xpong" => (ping, pong),
        "xping" => (pong, ping),
        other => panic!("unknown mode {other}: xping, xpong or xhairpin"),
    };
    match ergon_md_dpdk::env::selector().unwrap_or_else(|e| panic!("floor: {e}")) {
        ergon_md_dpdk::Selector::Default => {
            let mut wire = KernelWire::open(listen, peer).unwrap_or_else(|e| panic!("floor: kernel UDP: {e}"));
            eprintln!("floor: kernel UDP sockets, no Aeron (listen {listen}, peer {peer})");
            run(&mode, &mut wire, peer, messages, warmup, cpu);
        }
        ergon_md_dpdk::Selector::DpdkEna => {
            #[cfg(all(feature = "dpdk", target_os = "linux"))]
            {
                let mut shared = dpdk::DpdkShared::open(cpu).unwrap_or_else(|e| panic!("floor: DPDK: {e}"));
                let mut wire = shared
                    .cross(listen, peer)
                    .unwrap_or_else(|e| panic!("floor: DPDK: {e}"));
                run(&mode, &mut wire, peer, messages, warmup, cpu);
                // the native runtime's threads may still read what it holds
                std::mem::forget(wire);
                std::mem::forget(shared);
            }
            #[cfg(not(all(feature = "dpdk", target_os = "linux")))]
            panic!(
                "floor: RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena, but this build has no DPDK (the harness's dpdk feature, Linux)"
            );
        }
    }
}

fn run(mode: &str, wire: &mut impl Wire, peer: SocketAddrV4, messages: usize, warmup: usize, cpu: Option<usize>) {
    if mode == "xping" {
        run_ping(wire, peer, messages, warmup, cpu);
    } else {
        run_pong(wire, cpu);
    }
}

/// `xhairpin`: both ends here, each on its own endpoint (a data NIC each),
/// the pong on a thread of its own.
fn hairpin(
    ping: SocketAddrV4,
    pong: SocketAddrV4,
    messages: usize,
    warmup: usize,
    ping_cpu: Option<usize>,
    pong_cpu: Option<usize>,
) {
    assert!(
        ping != pong,
        "floor: xhairpin takes two endpoints, one on each data NIC (the pong's, then the ping's): {ping} twice would never leave the host"
    );
    eprintln!(
        "floor: hairpin: ping and pong in one process (the pong listens on {ping}, the ping on {pong}; pong cpu {})",
        pong_cpu.map_or("-".to_string(), |c| c.to_string())
    );
    match ergon_md_dpdk::env::selector().unwrap_or_else(|e| panic!("floor: {e}")) {
        ergon_md_dpdk::Selector::Default => {
            // each end sends from its own address: out of its own NIC
            let ping_wire = KernelWire::open_from(pong, ping, Some(*pong.ip()))
                .unwrap_or_else(|e| panic!("floor: kernel UDP, the ping's: {e}"));
            let pong_wire = KernelWire::open_from(ping, pong, Some(*ping.ip()))
                .unwrap_or_else(|e| panic!("floor: kernel UDP, the pong's: {e}"));
            eprintln!("floor: kernel UDP sockets, no Aeron (each end bound to its own address)");
            run_hairpin(ping_wire, pong_wire, ping, messages, warmup, ping_cpu, pong_cpu);
        }
        ergon_md_dpdk::Selector::DpdkEna => {
            #[cfg(all(feature = "dpdk", target_os = "linux"))]
            {
                use rusteron_media_driver::bindings::aeron_udp_channel_transport_affinity_t as Affinity;
                let mut shared = dpdk::DpdkShared::open(ping_cpu).unwrap_or_else(|e| panic!("floor: DPDK: {e}"));
                let ping_wire = shared
                    .port(pong, ping, Affinity::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER)
                    .unwrap_or_else(|e| panic!("floor: DPDK, the ping's (sender) port: {e}"));
                let pong_wire = shared
                    .port(ping, pong, Affinity::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER)
                    .unwrap_or_else(|e| panic!("floor: DPDK, the pong's (receiver) port: {e}"));
                let wires = run_hairpin(ping_wire, pong_wire, ping, messages, warmup, ping_cpu, pong_cpu);
                // the native runtime's threads may still read what it holds
                std::mem::forget(wires);
                std::mem::forget(shared);
            }
            #[cfg(not(all(feature = "dpdk", target_os = "linux")))]
            panic!(
                "floor: RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena, but this build has no DPDK (the harness's dpdk feature, Linux)"
            );
        }
    }
    eprintln!("floor: pong ended: the ping's run done");
}

/// Sets [`STOP`] when dropped: the hairpin's pong ends with the ping's run,
/// a failed one's too (the scope would otherwise wait for it for ever).
struct StopOnDrop;

impl Drop for StopOnDrop {
    fn drop(&mut self) {
        STOP.store(true, Ordering::Relaxed);
    }
}

/// The ping here, the pong on a scoped thread until the ping is done; the
/// wires back, for the caller to keep or drop.
fn run_hairpin<W: Wire + Send>(
    mut ping_wire: W,
    mut pong_wire: W,
    peer: SocketAddrV4,
    messages: usize,
    warmup: usize,
    ping_cpu: Option<usize>,
    pong_cpu: Option<usize>,
) -> (W, W) {
    std::thread::scope(|scope| {
        let ponger = std::thread::Builder::new()
            .name("floor-pong".into())
            .spawn_scoped(scope, move || {
                pong_loop(&mut pong_wire, pong_cpu);
                pong_wire
            })
            .unwrap_or_else(|e| panic!("floor: the pong's thread: {e}"));
        let stop = StopOnDrop;
        run_ping(&mut ping_wire, peer, messages, warmup, ping_cpu);
        drop(stop);
        let pong_wire = ponger
            .join()
            .unwrap_or_else(|_| panic!("floor: the pong's thread failed"));
        (ping_wire, pong_wire)
    })
}

fn run_ping(wire: &mut impl Wire, peer: SocketAddrV4, messages: usize, warmup: usize, cpu: Option<usize>) {
    connect(wire, peer);
    pin(cpu);
    let clock = Clock::calibrate();

    let budget = Duration::from_secs(std::env::var("RTT_SECONDS").map_or(20, |s| s.parse().unwrap()));
    let mut results = Results::new();
    let mut buffer = [0u8; DATAGRAM];
    // raw UDP repairs nothing: a ping or echo lost is counted lost after
    // this long, and the next sent
    let timeout_ns: u64 = match std::env::var("RTT_LOST_TIMEOUT_US").map(|v| v.parse::<u64>()) {
        Ok(Ok(us)) if us > 0 => us * 1000,
        Ok(Err(e)) => panic!("RTT_LOST_TIMEOUT_US: {e}"),
        _ => 50_000_000,
    };
    let mut seq = 0_u64;
    let mut pace = Pace::from_env();
    let start = Instant::now();
    for i in 0..warmup {
        seq += 1;
        round_trip(wire, &clock, &mut buffer, &mut None, &mut pace, seq, timeout_ns, |_| {});
        if (pace.paced() || i % 64 == 0) && start.elapsed() > budget / 4 {
            break;
        }
    }
    let start = Instant::now();
    results.start(&clock);
    let mut stalls = Stalls::new(&clock);
    let mut lost = 0_u64;
    for i in 0..messages {
        seq += 1;
        let echoed = round_trip(
            wire,
            &clock,
            &mut buffer,
            &mut stalls,
            &mut pace,
            seq,
            timeout_ns,
            |rtt| {
                results.record(&clock, rtt, i);
            },
        );
        if !echoed {
            lost += 1;
        }
        if (pace.paced() || i % 64 == 0) && start.elapsed() > budget {
            break;
        }
    }

    results.report(lost, PING_STREAM_ID);
    if let Some(stalls) = &stalls {
        stalls.write("stalls-ping.csv");
    }
}

/// Probes (sequence 0) every 10 ms until the pong echoes one, for up to
/// `FLOOR_CONNECT_SECS` (30): until then a refused or unresolved send is
/// the pong not up yet, and retried.
fn connect(wire: &mut impl Wire, peer: SocketAddrV4) {
    let secs: u64 = std::env::var("FLOOR_CONNECT_SECS").map_or(30, |v| v.parse().expect("FLOOR_CONNECT_SECS"));
    let deadline = Instant::now() + Duration::from_secs(secs);
    let probe = [0u8; DATAGRAM];
    let mut last = None;
    while Instant::now() < deadline {
        if let Err(e) = wire.send(&probe) {
            last = Some(e);
        }
        let until = Instant::now() + Duration::from_millis(10);
        while Instant::now() < until {
            let mut echoed = false;
            let polled = wire.poll(|reply| echoed |= reply.len() >= DATAGRAM && seq_of(reply) == 0);
            if echoed {
                return;
            }
            if let Err(e) = polled {
                last = Some(e);
            }
        }
    }
    panic!(
        "floor: no echo from the pong at {peer} within {secs} s{}",
        last.map(|e| format!(" (last error: {e})")).unwrap_or_default()
    );
}

fn seq_of(datagram: &[u8]) -> u64 {
    u64::from_le_bytes(datagram[SEQ..SEQ + 8].try_into().unwrap())
}

/// One round trip of datagram `seq`, as `rtt`'s: `true` once its echo is
/// back, `false` if none came within `timeout_ns`. An echo of an earlier,
/// timed-out one is skipped. A send that cannot go yet (a full ring, an ARP
/// answer awaited) is retried, the wire polled meanwhile, stamped again on
/// each attempt. A paced ping waits for its time (`pace`) first, polling the
/// wire as it does for an echo.
#[inline]
#[allow(clippy::too_many_arguments)]
fn round_trip(
    wire: &mut impl Wire,
    clock: &Clock,
    buffer: &mut [u8; DATAGRAM],
    stalls: &mut Option<Stalls>,
    pace: &mut Pace,
    seq: u64,
    timeout_ns: u64,
    mut record: impl FnMut(u64),
) -> bool {
    if let Some(s) = stalls.as_mut() {
        s.pass(clock.now());
    }
    while !pace.ready(clock.now()) {
        if let Some(s) = stalls.as_mut() {
            s.pass(clock.now());
        }
        if wire.poll(|_| {}).unwrap_or_else(|e| wire_failed("poll", &e)) == 0 {
            spin_loop();
        }
    }
    buffer[SEQ..SEQ + 8].copy_from_slice(&seq.to_le_bytes());
    let started = loop {
        let now = clock.stamp();
        buffer[STAMP..STAMP + 8].copy_from_slice(&now.to_le_bytes());
        match wire.send(buffer) {
            Ok(true) => {
                pace.sent(now);
                break now;
            }
            Ok(false) => {
                // nothing of this ping can be in flight yet
                wire.poll(|_| {}).unwrap_or_else(|e| wire_failed("poll", &e));
                spin_loop();
            }
            Err(e) => wire_failed("send", &e),
        }
    };
    let mut echoed = false;
    while !echoed {
        if let Some(s) = stalls.as_mut() {
            s.pass(clock.now());
        }
        let received = wire
            .poll(|reply| {
                if reply.len() >= DATAGRAM && seq_of(reply) == seq {
                    let sent = u64::from_le_bytes(reply[STAMP..STAMP + 8].try_into().unwrap());
                    record(clock.stamp().wrapping_sub(sent));
                    echoed = true;
                }
            })
            .unwrap_or_else(|e| wire_failed("poll", &e));
        if received == 0 {
            if clock.now().wrapping_sub(started) > timeout_ns {
                return false;
            }
            spin_loop();
        }
    }
    true
}

#[cold]
#[inline(never)]
fn wire_failed(what: &str, error: &io::Error) -> ! {
    panic!("floor: {what}: {error}")
}

/// Echoes every datagram until SIGTERM; says so on stderr.
fn run_pong(wire: &mut impl Wire, cpu: Option<usize>) {
    lab::stop_on_sigterm();
    pong_loop(wire, cpu);
    eprintln!("floor: pong ended: SIGTERM");
}

/// Echoes every datagram, pinned to `cpu`, until [`STOP`]; then writes its
/// loop's stalls.
fn pong_loop(wire: &mut impl Wire, cpu: Option<usize>) {
    pin(cpu);
    let clock = Clock::calibrate();
    let mut stalls = Stalls::new(&clock);
    let mut pending = [[0u8; DATAGRAM]; BURST];
    let mut lengths = [0usize; BURST];
    while !STOP.load(Ordering::Relaxed) {
        if let Some(s) = stalls.as_mut() {
            s.pass(clock.now());
        }
        let mut count = 0;
        wire.poll(|ping| {
            if count < BURST {
                let n = ping.len().min(DATAGRAM);
                pending[count][..n].copy_from_slice(&ping[..n]);
                lengths[count] = n;
                count += 1;
            }
        })
        .unwrap_or_else(|e| wire_failed("poll", &e));
        for (echo, &n) in pending.iter().zip(&lengths).take(count) {
            // a ping waits for its echo before the next, so a retry's poll
            // finds nothing of this run to lose
            while !wire.send(&echo[..n]).unwrap_or_else(|e| wire_failed("send", &e)) {
                if STOP.load(Ordering::Relaxed) {
                    break;
                }
                wire.poll(|_| {}).unwrap_or_else(|e| wire_failed("poll", &e));
                spin_loop();
            }
        }
        if count == 0 {
            spin_loop();
        }
    }
    if let Some(stalls) = &stalls {
        stalls.write("stalls-pong.csv");
    }
}

/// Kernel UDP: a socket bound to this side's endpoint, read without
/// blocking, and one connected to the peer's (bound to a source address of
/// its own where given: the hairpin's).
struct KernelWire {
    rx: UdpSocket,
    tx: UdpSocket,
    buffer: [u8; 2048],
}

impl KernelWire {
    fn open(listen: SocketAddrV4, peer: SocketAddrV4) -> io::Result<Self> {
        Self::open_from(listen, peer, None)
    }

    fn open_from(listen: SocketAddrV4, peer: SocketAddrV4, source: Option<Ipv4Addr>) -> io::Result<Self> {
        let rx = UdpSocket::bind(listen)?;
        rx.set_nonblocking(true)?;
        let tx = UdpSocket::bind(SocketAddrV4::new(source.unwrap_or(Ipv4Addr::UNSPECIFIED), 0))?;
        tx.connect(peer)?;
        tx.set_nonblocking(true)?;
        Ok(Self {
            rx,
            tx,
            buffer: [0; 2048],
        })
    }
}

impl Wire for KernelWire {
    #[inline]
    fn send(&mut self, datagram: &[u8]) -> io::Result<bool> {
        match self.tx.send(datagram) {
            Ok(n) if n == datagram.len() => Ok(true),
            Ok(n) => Err(io::Error::other(format!("sent {n} of {} bytes", datagram.len()))),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(false),
            Err(e) => Err(e),
        }
    }

    #[inline]
    fn poll<F: FnMut(&[u8])>(&mut self, mut on: F) -> io::Result<usize> {
        match self.rx.recv(&mut self.buffer) {
            Ok(n) => {
                on(&self.buffer[..n]);
                Ok(1)
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(0),
            Err(e) => Err(e),
        }
    }
}

/// ergon-md-dpdk's bindings, driven directly.
#[cfg(all(feature = "dpdk", target_os = "linux"))]
mod dpdk {
    use super::{BURST, Wire};
    use ergon_md_dpdk::DpdkTransport;
    use rusteron_media_driver::bindings::*;
    use rusteron_media_driver::{AeronDriverContext, AeronIdleStrategyKind};
    use std::ffi::{CStr, c_char, c_void};
    use std::io;
    use std::net::SocketAddrV4;
    use std::ptr;
    use std::time::{Duration, Instant};

    unsafe extern "C" {
        fn rusteron_dpdk_last_error() -> *const c_char;
    }

    /// The transport's last error, for one of its calls that failed.
    fn last_error(what: &str) -> io::Error {
        // SAFETY: the native side returns a NUL-terminated static buffer.
        let why = unsafe { CStr::from_ptr(rusteron_dpdk_last_error()) };
        io::Error::other(format!("{what}: {}", why.to_string_lossy()))
    }

    /// Counter slots: every fixed counter and the PMD's extended statistics,
    /// for both ports, with room to spare.
    const COUNTER_SLOTS: usize = 2048;
    const METADATA_LENGTH: usize = 512;
    const VALUE_LENGTH: usize = 128;

    /// The counters the transport registers into, as a driver's: so every
    /// bump on the packet path costs what it does under a driver.
    struct Counters {
        manager: Box<aeron_counters_manager_t>,
        _clock: Box<aeron_clock_cache_t>,
        _metadata: Box<[u64]>,
        _values: Box<[u64]>,
    }

    impl Counters {
        fn new() -> io::Result<Self> {
            let metadata = vec![0u64; COUNTER_SLOTS * METADATA_LENGTH / 8].into_boxed_slice();
            let values = vec![0u64; COUNTER_SLOTS * VALUE_LENGTH / 8].into_boxed_slice();
            // SAFETY: a plain C struct of integers, all zero a valid state.
            let mut clock: Box<aeron_clock_cache_t> = Box::new(unsafe { std::mem::zeroed() });
            // SAFETY: as above; aeron_counters_manager_init fills it.
            let mut manager: Box<aeron_counters_manager_t> = Box::new(unsafe { std::mem::zeroed() });
            // SAFETY: both buffers outlive the manager (same struct, never dropped
            // while the transport runs), and their lengths are theirs.
            let rc = unsafe {
                aeron_counters_manager_init(
                    &mut *manager,
                    metadata.as_ptr() as *mut u8,
                    metadata.len() * 8,
                    values.as_ptr() as *mut u8,
                    values.len() * 8,
                    &mut *clock,
                    10_000,
                )
            };
            if rc != 0 {
                return Err(io::Error::other("aeron_counters_manager_init failed"));
            }
            Ok(Self {
                manager,
                _clock: clock,
                _metadata: metadata,
                _values: values,
            })
        }
    }

    /// An IPv4 address as the bindings take it.
    fn sockaddr(addr: SocketAddrV4) -> Box<libc::sockaddr_storage> {
        // SAFETY: all zero is a valid sockaddr_storage.
        let mut storage: Box<libc::sockaddr_storage> = Box::new(unsafe { std::mem::zeroed() });
        let sin = libc::sockaddr_in {
            sin_family: libc::AF_INET as libc::sa_family_t,
            sin_port: addr.port().to_be(),
            sin_addr: libc::in_addr {
                s_addr: u32::from_ne_bytes(addr.ip().octets()),
            },
            sin_zero: [0; 8],
        };
        // SAFETY: a sockaddr_in fits in a sockaddr_storage, which is aligned for it.
        unsafe { ptr::write((&raw mut *storage).cast::<libc::sockaddr_in>(), sin) };
        storage
    }

    /// The transport's state for the process: its configuration, the
    /// counters it registers into, and the context its bindings' calls see
    /// (only its counters manager, as the transport's own tests give it).
    /// Every wire is made from it, and it outlives them all.
    pub struct DpdkShared {
        _context: AeronDriverContext,
        _transport: DpdkTransport,
        _counters: Counters,
        init_context: Box<aeron_driver_context_t>,
        bindings: *mut aeron_udp_channel_transport_bindings_t,
        mtu: usize,
    }

    /// What the wires send on and poll: the cross-host one a sender-port
    /// transport to the peer and a receiver-port one on this side's
    /// endpoint, one poller over both (the sender port's ARP answers come in
    /// on it); a hairpin end one port's transport, which sends and receives,
    /// and a poller of its own.
    pub struct DpdkWire {
        bindings: *mut aeron_udp_channel_transport_bindings_t,
        tx: Box<aeron_udp_channel_transport_t>,
        /// The transport datagrams arrive on, where it is not `tx`.
        rx: Option<Box<aeron_udp_channel_transport_t>>,
        poller: Box<aeron_udp_transport_poller_t>,
        data_paths: Box<aeron_udp_channel_data_paths_t>,
        peer: Box<libc::sockaddr_storage>,
        msgvec: Box<[libc::mmsghdr; BURST]>,
    }

    // SAFETY: a wire's transports, poller and buffers are used by the one
    // thread that holds it (a hairpin's pong takes its own to its thread),
    // each port's queues by one thread, as an Aeron driver's sender and
    // receiver threads use theirs; the shared state outlives every wire.
    unsafe impl Send for DpdkWire {}

    impl DpdkShared {
        /// The transport from the environment, as the driver's, polled as an
        /// INVOKER driver's is (its conductor CPU `cpu`, never idling).
        pub fn open(cpu: Option<usize>) -> io::Result<Self> {
            let aeron = |what: &str, e: &dyn std::fmt::Debug| io::Error::other(format!("{what}: {e:?}"));
            let context = AeronDriverContext::new().map_err(|e| aeron("the driver context", &e))?;
            context
                .set_threading_mode(aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER)
                .map_err(|e| aeron("threading mode", &e))?;
            let cpu = cpu.map_or(Ok(-1), i32::try_from).map_err(|e| aeron("cpu", &e))?;
            context.set_conductor_cpu_affinity(cpu).map_err(|e| aeron("cpu", &e))?;
            context
                .set_shared_idle_strategy_kind(AeronIdleStrategyKind::NoOp)
                .map_err(|e| aeron("idle strategy", &e))?;
            // the bundled driver's ranges unless the environment gives them
            if std::env::var_os("AERON_SENDER_WILDCARD_PORT_RANGE").is_none() {
                context
                    .set_sender_wildcard_port_range(20000, 20999)
                    .map_err(|e| aeron("sender wildcard ports", &e))?;
            }
            if std::env::var_os("AERON_RECEIVER_WILDCARD_PORT_RANGE").is_none() {
                context
                    .set_receiver_wildcard_port_range(21000, 21999)
                    .map_err(|e| aeron("receiver wildcard ports", &e))?;
            }
            let mtu = context.get_mtu_length();
            let transport = ergon_md_dpdk::configure_media_transport_from_env(&context)
                .map_err(|e| io::Error::other(e.to_string()))?
                .ok_or_else(|| io::Error::other("RUSTERON_MEDIA_DRIVER_TRANSPORT does not select dpdk-ena"))?;
            eprintln!(
                "floor: ergon-md-dpdk's packet path, no Aeron driver or client (clock {:?})",
                transport.clock()
            );
            // SAFETY: the context is live; install set its bindings.
            let bindings = unsafe { (*context.get_inner()).udp_channel_transport_bindings };
            if bindings.is_null() {
                return Err(io::Error::other("the transport installed no bindings"));
            }
            let mut counters = Counters::new()?;
            // SAFETY: all zero is a valid aeron_driver_context_t for these calls.
            let mut init_context: Box<aeron_driver_context_t> = Box::new(unsafe { std::mem::zeroed() });
            init_context.counters_manager = &raw mut *counters.manager;
            wait_for_start_file()?;
            Ok(Self {
                _context: context,
                _transport: transport,
                _counters: counters,
                init_context,
                bindings,
                mtu,
            })
        }

        /// A transport of `affinity`'s port bound to `bind`, connected to
        /// `connect` where given.
        fn transport(
            &mut self,
            bind: SocketAddrV4,
            connect: Option<SocketAddrV4>,
            affinity: aeron_udp_channel_transport_affinity_t,
            what: &str,
        ) -> io::Result<Box<aeron_udp_channel_transport_t>> {
            // SAFETY: a zeroed C struct the init call fills.
            let mut transport = Box::<aeron_udp_channel_transport_t>::new(unsafe { std::mem::zeroed() });
            let mut bind = sockaddr(bind);
            let mut any = sockaddr(SocketAddrV4::new([0, 0, 0, 0].into(), 0));
            let mut connect = connect.map(sockaddr);
            // SAFETY: bindings are the installed table; every pointer is live
            // for the call, and the init copies what it keeps.
            let rc = unsafe {
                // SAFETY: all zero is a valid parameter set; the MTU is the context's.
                let mut params: aeron_udp_channel_transport_params_t = std::mem::zeroed();
                params.mtu_length = self.mtu;
                (*self.bindings).init_func.expect("init_func")(
                    &mut *transport,
                    (&raw mut *bind).cast(),
                    (&raw mut *any).cast(),
                    connect.as_mut().map_or(ptr::null_mut(), |c| (&raw mut **c).cast()),
                    &mut params,
                    &mut *self.init_context,
                    affinity,
                )
            };
            if rc < 0 {
                return Err(last_error(what));
            }
            Ok(transport)
        }

        /// A receiver-affinity poller over `transports`.
        fn poller(
            &mut self,
            transports: &mut [&mut aeron_udp_channel_transport_t],
        ) -> io::Result<Box<aeron_udp_transport_poller_t>> {
            // SAFETY: a zeroed C struct the init call fills.
            let mut poller = Box::<aeron_udp_transport_poller_t>::new(unsafe { std::mem::zeroed() });
            // SAFETY: the bindings are the installed table; the poller and
            // the transports are live, and live as long as the wire.
            unsafe {
                let b = &*self.bindings;
                if b.poller_init_func.expect("poller_init_func")(
                    &mut *poller,
                    &mut *self.init_context,
                    aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
                ) < 0
                {
                    return Err(last_error("the poller"));
                }
                for t in transports {
                    if b.poller_add_func.expect("poller_add_func")(&mut *poller, &mut **t) < 0 {
                        return Err(last_error("adding a transport to the poller"));
                    }
                }
            }
            Ok(poller)
        }

        fn wire(
            &self,
            tx: Box<aeron_udp_channel_transport_t>,
            rx: Option<Box<aeron_udp_channel_transport_t>>,
            poller: Box<aeron_udp_transport_poller_t>,
            peer: SocketAddrV4,
        ) -> DpdkWire {
            DpdkWire {
                bindings: self.bindings,
                tx,
                rx,
                poller,
                // SAFETY: a zeroed C struct, as the send call takes it.
                data_paths: Box::new(unsafe { std::mem::zeroed() }),
                peer: sockaddr(peer),
                // SAFETY: all zero is a valid mmsghdr array (no names: the
                // poller writes the source into its own storage then).
                msgvec: Box::new(unsafe { std::mem::zeroed() }),
            }
        }

        /// The cross-host wire: sends to `peer` from the sender port,
        /// receives on `listen` at the receiver port.
        pub fn cross(&mut self, listen: SocketAddrV4, peer: SocketAddrV4) -> io::Result<DpdkWire> {
            let any = SocketAddrV4::new([0, 0, 0, 0].into(), 0);
            let mut tx = self.transport(
                any,
                Some(peer),
                aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
                "the sender-port transport",
            )?;
            let mut rx = self.transport(
                listen,
                None,
                aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
                &format!("the receiver-port transport on {listen}"),
            )?;
            let poller = self.poller(&mut [&mut *rx, &mut *tx])?;
            eprintln!("floor: DPDK wire: listen {listen}, peer {peer}");
            Ok(self.wire(tx, Some(rx), poller, peer))
        }

        /// A hairpin end's wire: one port's transport (`affinity`'s: the
        /// sender port's or the receiver port's), bound to `listen` (that
        /// port's address), sending to `peer` (the other port's) and
        /// receiving there, with a poller of its own.
        pub fn port(
            &mut self,
            listen: SocketAddrV4,
            peer: SocketAddrV4,
            affinity: aeron_udp_channel_transport_affinity_t,
        ) -> io::Result<DpdkWire> {
            let sender =
                affinity == aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER;
            let what = if sender { "sender" } else { "receiver" };
            let mut t = self.transport(
                listen,
                sender.then_some(peer),
                affinity,
                &format!("the {what}-port transport on {listen}"),
            )?;
            let poller = self.poller(&mut [&mut *t])?;
            eprintln!("floor: DPDK wire on the {what} port: listen {listen}, peer {peer}");
            Ok(self.wire(t, None, poller, peer))
        }
    }

    /// `RUSTERON_HARNESS_START_FILE` (the vdev runner's): wait for it, up to
    /// a minute, before anything is sent.
    fn wait_for_start_file() -> io::Result<()> {
        let Some(path) = std::env::var_os("RUSTERON_HARNESS_START_FILE") else {
            return Ok(());
        };
        let deadline = Instant::now() + Duration::from_secs(60);
        while !std::path::Path::new(&path).exists() {
            if Instant::now() > deadline {
                return Err(io::Error::other(format!("no start file {}", path.to_string_lossy())));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }

    /// What a poll's receive callback hands each datagram to.
    struct Target<F> {
        rx: *mut aeron_udp_channel_transport_t,
        on: F,
        count: usize,
    }

    /// The bindings' receive callback: datagrams for the receiver-port
    /// transport go to the target's `on`.
    unsafe extern "C" fn received<F: FnMut(&[u8])>(
        _data_paths: *mut aeron_udp_channel_data_paths_t,
        transport: *mut aeron_udp_channel_transport_t,
        clientd: *mut c_void,
        _endpoint_clientd: *mut c_void,
        _destination_clientd: *mut c_void,
        buffer: *mut u8,
        length: usize,
        _addr: *mut sockaddr_storage,
        _media_timestamp: *mut timespec,
    ) {
        // SAFETY: clientd is the Target this poll passed, live for the call.
        let target = unsafe { &mut *clientd.cast::<Target<F>>() };
        if transport == target.rx {
            // SAFETY: the poller hands a payload of `length` bytes, live for the call.
            (target.on)(unsafe { std::slice::from_raw_parts(buffer, length) });
            target.count += 1;
        }
    }

    impl Wire for DpdkWire {
        #[inline]
        fn send(&mut self, datagram: &[u8]) -> io::Result<bool> {
            let mut iov = iovec {
                iov_base: datagram.as_ptr() as *mut c_void,
                iov_len: datagram.len(),
            };
            let mut bytes: i64 = 0;
            // SAFETY: the transport and address are live; one iovec over `datagram`.
            let rc = unsafe {
                (*self.bindings).send_func.expect("send_func")(
                    &mut *self.data_paths,
                    &mut *self.tx,
                    (&raw mut *self.peer).cast(),
                    &mut iov,
                    1,
                    &mut bytes,
                )
            };
            match rc {
                1.. => Ok(true),
                0 => Ok(false),
                _ => Err(last_error("send")),
            }
        }

        #[inline]
        fn poll<F: FnMut(&[u8])>(&mut self, on: F) -> io::Result<usize> {
            let rx: *mut aeron_udp_channel_transport_t = match self.rx.as_mut() {
                Some(rx) => &raw mut **rx,
                None => &raw mut *self.tx,
            };
            let mut target = Target { rx, on, count: 0 };
            let mut bytes: i64 = 0;
            // SAFETY: the poller and its transports are live; the msgvec has
            // BURST entries; `target` outlives the call.
            let rc = unsafe {
                (*self.bindings).poller_poll_func.expect("poller_poll_func")(
                    &mut *self.poller,
                    self.msgvec.as_mut_ptr().cast(),
                    BURST,
                    &mut bytes,
                    Some(received::<F>),
                    None,
                    (&raw mut target).cast(),
                )
            };
            if rc < 0 {
                return Err(last_error("poll"));
            }
            Ok(target.count)
        }
    }
}
