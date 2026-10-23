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
//!
//! The arguments, the pacing (each ping sent once the last one's echo is
//! back), the warm-up, the clock (`lab::Clock`, its ordered read at both
//! ends), the CPU pinning and every output are `rtt`'s (`roundtrip.rs`): the
//! `rtt,...` line, `RTT_HISTOGRAM`, `LOAD_SPIKES`, `LOOP_STALLS_DIR`, under
//! `LABEL`, `RTT_SECONDS`. A datagram is 64 bytes, what an Aeron data frame
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
use roundtrip::{Results, Stalls};
use std::hint::spin_loop;
use std::io;
use std::net::{SocketAddrV4, UdpSocket};
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
    let mode = args.next().expect("xping|xpong");
    let (ping, pong) = (endpoint(args.next()), endpoint(args.next()));
    // each side listens on its own endpoint and sends to the other's
    let (listen, peer) = match mode.as_str() {
        "xpong" => (ping, pong),
        "xping" => (pong, ping),
        other => panic!("unknown mode {other}: xping or xpong"),
    };
    let (messages, warmup) = if mode == "xping" {
        let number = |a: Option<String>| -> usize { a.expect("messages and warmup").parse().unwrap() };
        (number(args.next()), number(args.next()))
    } else {
        (0, 0)
    };
    let cpu = cpu_arg(args.next());
    match ergon_md_dpdk::env::selector().unwrap_or_else(|e| panic!("floor: {e}")) {
        ergon_md_dpdk::Selector::Default => {
            let mut wire = KernelWire::open(listen, peer).unwrap_or_else(|e| panic!("floor: kernel UDP: {e}"));
            eprintln!("floor: kernel UDP sockets, no Aeron (listen {listen}, peer {peer})");
            run(&mode, &mut wire, peer, messages, warmup, cpu);
        }
        ergon_md_dpdk::Selector::DpdkEna => {
            #[cfg(all(feature = "dpdk", target_os = "linux"))]
            {
                let mut wire = dpdk::DpdkWire::open(listen, peer, cpu).unwrap_or_else(|e| panic!("floor: DPDK: {e}"));
                run(&mode, &mut wire, peer, messages, warmup, cpu);
                // the native runtime's threads may still read what it holds
                std::mem::forget(wire);
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
    let start = Instant::now();
    for i in 0..warmup {
        seq += 1;
        round_trip(wire, &clock, &mut buffer, &mut None, seq, timeout_ns, |_| {});
        if i % 64 == 0 && start.elapsed() > budget / 4 {
            break;
        }
    }
    let start = Instant::now();
    results.start(&clock);
    let mut stalls = Stalls::new(&clock);
    let mut lost = 0_u64;
    for i in 0..messages {
        seq += 1;
        let echoed = round_trip(wire, &clock, &mut buffer, &mut stalls, seq, timeout_ns, |rtt| {
            results.record(&clock, rtt, i);
        });
        if !echoed {
            lost += 1;
        }
        if i % 64 == 0 && start.elapsed() > budget {
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
/// each attempt.
#[inline]
fn round_trip(
    wire: &mut impl Wire,
    clock: &Clock,
    buffer: &mut [u8; DATAGRAM],
    stalls: &mut Option<Stalls>,
    seq: u64,
    timeout_ns: u64,
    mut record: impl FnMut(u64),
) -> bool {
    if let Some(s) = stalls.as_mut() {
        s.pass(clock.now());
    }
    buffer[SEQ..SEQ + 8].copy_from_slice(&seq.to_le_bytes());
    let started = loop {
        let now = clock.stamp();
        buffer[STAMP..STAMP + 8].copy_from_slice(&now.to_le_bytes());
        match wire.send(buffer) {
            Ok(true) => break now,
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
    pin(cpu);
    lab::stop_on_sigterm();
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
    eprintln!("floor: pong ended: SIGTERM");
}

/// Kernel UDP: a socket bound to this side's endpoint, read without
/// blocking, and one connected to the peer's.
struct KernelWire {
    rx: UdpSocket,
    tx: UdpSocket,
    buffer: [u8; 2048],
}

impl KernelWire {
    fn open(listen: SocketAddrV4, peer: SocketAddrV4) -> io::Result<Self> {
        let rx = UdpSocket::bind(listen)?;
        rx.set_nonblocking(true)?;
        let tx = UdpSocket::bind("0.0.0.0:0")?;
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

    /// A sender-port transport to the peer and a receiver-port one on this
    /// side's endpoint, one poller over both (the sender port's ARP answers
    /// come in on it), the transport's whole state held for the process.
    pub struct DpdkWire {
        _context: AeronDriverContext,
        _transport: DpdkTransport,
        _counters: Counters,
        _init_context: Box<aeron_driver_context_t>,
        bindings: *mut aeron_udp_channel_transport_bindings_t,
        tx: Box<aeron_udp_channel_transport_t>,
        rx: Box<aeron_udp_channel_transport_t>,
        poller: Box<aeron_udp_transport_poller_t>,
        data_paths: Box<aeron_udp_channel_data_paths_t>,
        peer: Box<libc::sockaddr_storage>,
        msgvec: Box<[libc::mmsghdr; BURST]>,
    }

    impl DpdkWire {
        /// The transport from the environment, as the driver's, on this thread
        /// as an INVOKER driver's (its CPU `cpu`, never idling).
        pub fn open(listen: SocketAddrV4, peer: SocketAddrV4, cpu: Option<usize>) -> io::Result<Self> {
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
                "floor: ergon-md-dpdk's packet path, no Aeron driver or client (listen {listen}, peer {peer}, clock {:?})",
                transport.clock()
            );
            // SAFETY: the context is live; install set its bindings.
            let bindings = unsafe { (*context.get_inner()).udp_channel_transport_bindings };
            if bindings.is_null() {
                return Err(io::Error::other("the transport installed no bindings"));
            }
            let mut counters = Counters::new()?;
            // The context the bindings' calls see: only its counters manager,
            // as the transport's own tests give it.
            // SAFETY: all zero is a valid aeron_driver_context_t for these calls.
            let mut init_context: Box<aeron_driver_context_t> = Box::new(unsafe { std::mem::zeroed() });
            init_context.counters_manager = &raw mut *counters.manager;
            wait_for_start_file()?;

            // SAFETY: zeroed C structs the init and poller calls fill.
            let (mut tx, mut rx, mut poller, data_paths) = unsafe {
                (
                    Box::<aeron_udp_channel_transport_t>::new(std::mem::zeroed()),
                    Box::<aeron_udp_channel_transport_t>::new(std::mem::zeroed()),
                    Box::<aeron_udp_transport_poller_t>::new(std::mem::zeroed()),
                    Box::<aeron_udp_channel_data_paths_t>::new(std::mem::zeroed()),
                )
            };
            let mut peer_addr = sockaddr(peer);
            let mut any = sockaddr(SocketAddrV4::new([0, 0, 0, 0].into(), 0));
            let mut bind = sockaddr(listen);
            // SAFETY: bindings are the installed table; every pointer is live
            // for the call and the transports live as long as the wire.
            unsafe {
                let b = &*bindings;
                // SAFETY: all zero is a valid parameter set; the MTU is the context's.
                let mut params: aeron_udp_channel_transport_params_t = std::mem::zeroed();
                params.mtu_length = mtu;
                let rc = b.init_func.expect("init_func")(
                    &mut *tx,
                    (&raw mut *any).cast(),
                    (&raw mut *any).cast(),
                    (&raw mut *peer_addr).cast(),
                    &mut params,
                    &mut *init_context,
                    aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER,
                );
                if rc < 0 {
                    return Err(last_error("the sender-port transport"));
                }
                let rc = b.init_func.expect("init_func")(
                    &mut *rx,
                    (&raw mut *bind).cast(),
                    (&raw mut *any).cast(),
                    ptr::null_mut(),
                    &mut params,
                    &mut *init_context,
                    aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
                );
                if rc < 0 {
                    return Err(last_error(&format!("the receiver-port transport on {listen}")));
                }
                if b.poller_init_func.expect("poller_init_func")(
                    &mut *poller,
                    &mut *init_context,
                    aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
                ) < 0
                {
                    return Err(last_error("the poller"));
                }
                for t in [&mut *rx, &mut *tx] {
                    if b.poller_add_func.expect("poller_add_func")(&mut *poller, t) < 0 {
                        return Err(last_error("adding a transport to the poller"));
                    }
                }
            }
            Ok(Self {
                _context: context,
                _transport: transport,
                _counters: counters,
                _init_context: init_context,
                bindings,
                tx,
                rx,
                poller,
                data_paths,
                peer: peer_addr,
                // SAFETY: all zero is a valid mmsghdr array (no names: the
                // poller writes the source into its own storage then).
                msgvec: Box::new(unsafe { std::mem::zeroed() }),
            })
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
            let mut target = Target {
                rx: &raw mut *self.rx,
                on,
                count: 0,
            };
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
