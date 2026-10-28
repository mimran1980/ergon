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
//!
//! The transport suite's raw floor (its `dpdk-raw` arm, the same wire
//! choice): `floor --role ping|pong` with the suite's flags
//! (transport-cli.md, `io/benches`'s `point.rs`: the schedule, the
//! recording, the row and the files every arm writes):
//!
//! ```text
//! floor --role <ping|pong> --rate MSG_PER_S --label SET:ARM --out DIR --endpoint HOST:PORT --reply HOST:PORT
//!       [--warmup-s 30] [--duration-s 30] [--size 64] [--cpus CPU] [--spike-ns 100000] [--stall-ns 10000]
//!       [--connect-timeout-s 30]
//! ```
//!
//! One stream, one busy thread an end: the ping sends to `--endpoint` (the
//! pong's address) and reads its echoes on `--reply` (its own). A datagram
//! is 96 bytes, an Aeron data frame of the suite's 64-byte message: a
//! 32-byte header (byte 0: 1 a ping or echo, 2 a probe) where Aeron's would
//! be, then the message. The ping probes until the pong echoes one before
//! the schedule starts; nothing repairs a lost datagram (the row's `lost`).

#[path = "../lab.rs"]
#[allow(dead_code)]
mod lab;
#[path = "../roundtrip.rs"]
mod roundtrip;

// io/benches formats it (rustfmt's defaults), not this crate (max_width 120).
#[path = "../../../../io/benches/src/transport/point.rs"]
#[rustfmt::skip]
#[allow(dead_code)]
mod point;

#[global_allocator]
static ALLOC: point::CountingAlloc = point::CountingAlloc;

use lab::{Clock, STOP, cpu_arg, pin};
use roundtrip::{Pace, Results, Stalls, env_u64};
use std::error::Error;
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
/// The sequence a keep-alive datagram carries: never a ping's (they count
/// from 1), so the ping skips its echo as it skips a late one.
const KEEPALIVE_SEQ: u64 = u64::MAX;
/// No keep-alive goes this close (ns) before a ping is due, so the ping
/// never queues behind one: the keep-alive keeps the path warm, it does not
/// share it with the measured round trip.
const KEEPALIVE_GUARD_NS: u64 = 50_000;

/// What a paced run does between its pings, beyond spinning:
///
/// - `KEEPALIVE_US=<us>` (ping only): a keep-alive datagram every that many
///   microseconds, which the pong echoes as any other and the ping drops,
///   none within [`KEEPALIVE_GUARD_NS`] of the next ping. A keep-alive A/B
///   at one gap tells a path slowed by quiet from one slowed by distance.
/// - `FLUSH_KB=<KiB>` (ping and pong): after each round trip (the ping) or
///   each echo of a ping (the pong) one byte of every cache line of a
///   buffer that large is written, so the next round trip runs from cold
///   CPU caches while the network stays as warm as the gap leaves it.
struct Between {
    gap_ns: u64,
    keepalive_ns: u64,
    last_ping: u64,
    last_keepalive: u64,
    flush: Vec<u8>,
}

impl Between {
    fn from_env() -> Result<Self, Box<dyn Error>> {
        let gap_ns = env_u64("RTT_GAP_US", 0)? * 1000;
        let keepalive_ns = env_u64("KEEPALIVE_US", 0)? * 1000;
        if keepalive_ns != 0 && (gap_ns == 0 || keepalive_ns >= gap_ns) {
            return Err("floor: KEEPALIVE_US needs RTT_GAP_US, and a keep-alive more often than the pings".into());
        }
        let flush_kb = env_u64("FLUSH_KB", 0)?;
        let flush = vec![0u8; usize::try_from(flush_kb * 1024).map_err(|e| format!("FLUSH_KB={flush_kb}: {e}"))?];
        if keepalive_ns != 0 || !flush.is_empty() {
            eprintln!(
                "floor: between pings: keep-alive every {} us, flush {flush_kb} KiB",
                keepalive_ns / 1000
            );
        }
        Ok(Self {
            gap_ns,
            keepalive_ns,
            last_ping: 0,
            last_keepalive: 0,
            flush,
        })
    }

    /// A keep-alive is due at `now` (and not too close to the next ping).
    #[inline]
    fn keepalive_due(&self, now: u64) -> bool {
        self.keepalive_ns != 0
            && self.last_ping != 0
            && now.wrapping_sub(self.last_keepalive.max(self.last_ping)) >= self.keepalive_ns
            && now.wrapping_sub(self.last_ping) + KEEPALIVE_GUARD_NS < self.gap_ns
    }

    /// Writes a byte in every cache line of the flush buffer, if any.
    #[inline(never)]
    fn flush(&mut self) {
        if self.flush.is_empty() {
            return;
        }
        for line in self.flush.chunks_mut(64) {
            if let Some(byte) = line.first_mut() {
                *byte = byte.wrapping_add(1);
            }
        }
        std::hint::black_box(&self.flush);
    }
}

/// What a floor client sends on and polls.
trait Wire {
    /// Sends `datagram` to the peer: `false` when it cannot go yet (retry).
    fn send(&mut self, datagram: &[u8]) -> io::Result<bool>;
    /// Polls once, giving each datagram received to `on`: how many.
    fn poll<F: FnMut(&[u8])>(&mut self, on: F) -> io::Result<usize>;
}

fn endpoint(arg: Option<String>) -> Result<SocketAddrV4, Box<dyn Error>> {
    let arg = arg.ok_or("an endpoint: host:port")?;
    Ok(arg
        .parse()
        .map_err(|e| format!("endpoint {arg}: {e} (an IPv4 host:port)"))?)
}

/// What a build without DPDK says when the environment selects it.
#[cfg(not(all(feature = "dpdk", target_os = "linux")))]
const NO_DPDK: &str =
    "floor: RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena, but this build has no DPDK (the harness's dpdk feature, Linux)";

#[allow(clippy::similar_names, reason = "ping and pong name the two ends of a round trip")]
fn main() -> Result<(), Box<dyn Error>> {
    lab::lock_memory()?;
    if std::env::args().nth(1).is_some_and(|a| a.starts_with("--")) {
        std::process::exit(transport::main());
    }
    let mut args = std::env::args().skip(1);
    let mode = args.next().ok_or("xping|xpong|xhairpin")?;
    let (ping, pong) = (endpoint(args.next())?, endpoint(args.next())?);
    let (messages, warmup) = if mode == "xping" || mode == "xhairpin" {
        let number = |a: Option<String>| -> Result<usize, Box<dyn Error>> {
            let a = a.ok_or("messages and warmup")?;
            Ok(a.parse().map_err(|e| format!("{a:?}: a count: {e}"))?)
        };
        (number(args.next())?, number(args.next())?)
    } else {
        (0, 0)
    };
    let cpu = cpu_arg(args.next())?;
    if mode == "xhairpin" {
        return hairpin(ping, pong, messages, warmup, cpu, cpu_arg(args.next())?);
    }
    // each side listens on its own endpoint and sends to the other's
    let (listen, peer) = match mode.as_str() {
        "xpong" => (ping, pong),
        "xping" => (pong, ping),
        other => return Err(format!("unknown mode {other}: xping, xpong or xhairpin").into()),
    };
    match ergon_md_dpdk::env::selector().map_err(|e| format!("floor: {e}"))? {
        ergon_md_dpdk::Selector::Default => {
            let mut wire = KernelWire::open(listen, peer).map_err(|e| format!("floor: kernel UDP: {e}"))?;
            eprintln!("floor: kernel UDP sockets, no Aeron (listen {listen}, peer {peer})");
            run(&mode, &mut wire, peer, messages, warmup, cpu)
        }
        ergon_md_dpdk::Selector::DpdkEna => {
            #[cfg(all(feature = "dpdk", target_os = "linux"))]
            {
                let mut shared = dpdk::DpdkShared::open(cpu).map_err(|e| format!("floor: DPDK: {e}"))?;
                let mut wire = shared.cross(listen, peer).map_err(|e| format!("floor: DPDK: {e}"))?;
                let ran = run(&mode, &mut wire, peer, messages, warmup, cpu);
                // the native runtime's threads may still read what it holds
                std::mem::forget(wire);
                std::mem::forget(shared);
                ran
            }
            #[cfg(not(all(feature = "dpdk", target_os = "linux")))]
            Err(NO_DPDK.into())
        }
    }
}

fn run(
    mode: &str,
    wire: &mut impl Wire,
    peer: SocketAddrV4,
    messages: usize,
    warmup: usize,
    cpu: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    if mode == "xping" {
        run_ping(wire, peer, messages, warmup, cpu)
    } else {
        run_pong(wire, cpu)
    }
}

/// `xhairpin`: both ends here, each on its own endpoint (a data NIC each),
/// the pong on a thread of its own.
#[allow(clippy::similar_names, reason = "ping and pong name the two ends of a round trip")]
fn hairpin(
    ping: SocketAddrV4,
    pong: SocketAddrV4,
    messages: usize,
    warmup: usize,
    ping_cpu: Option<usize>,
    pong_cpu: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    if ping == pong {
        return Err(format!(
            "floor: xhairpin takes two endpoints, one on each data NIC (the pong's, then the ping's): \
             {ping} twice would never leave the host"
        )
        .into());
    }
    eprintln!(
        "floor: hairpin: ping and pong in one process (the pong listens on {ping}, the ping on {pong}; pong cpu {})",
        pong_cpu.map_or_else(|| "-".to_string(), |c| c.to_string())
    );
    match ergon_md_dpdk::env::selector().map_err(|e| format!("floor: {e}"))? {
        ergon_md_dpdk::Selector::Default => {
            // each end sends from its own address: out of its own NIC
            let ping_wire = KernelWire::open_from(pong, ping, Some(*pong.ip()))
                .map_err(|e| format!("floor: kernel UDP, the ping's: {e}"))?;
            let pong_wire = KernelWire::open_from(ping, pong, Some(*ping.ip()))
                .map_err(|e| format!("floor: kernel UDP, the pong's: {e}"))?;
            eprintln!("floor: kernel UDP sockets, no Aeron (each end bound to its own address)");
            run_hairpin(ping_wire, pong_wire, ping, messages, warmup, ping_cpu, pong_cpu)?;
        }
        ergon_md_dpdk::Selector::DpdkEna => {
            #[cfg(all(feature = "dpdk", target_os = "linux"))]
            {
                use rusteron_media_driver::bindings::aeron_udp_channel_transport_affinity_t as Affinity;
                let mut shared = dpdk::DpdkShared::open(ping_cpu).map_err(|e| format!("floor: DPDK: {e}"))?;
                let ping_wire = shared
                    .port(pong, ping, Affinity::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_SENDER)
                    .map_err(|e| format!("floor: DPDK, the ping's (sender) port: {e}"))?;
                let pong_wire = shared
                    .port(ping, pong, Affinity::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER)
                    .map_err(|e| format!("floor: DPDK, the pong's (receiver) port: {e}"))?;
                let wires = run_hairpin(ping_wire, pong_wire, ping, messages, warmup, ping_cpu, pong_cpu);
                // the native runtime's threads may still read what it holds
                std::mem::forget(shared);
                std::mem::forget(wires?);
            }
            #[cfg(not(all(feature = "dpdk", target_os = "linux")))]
            return Err(NO_DPDK.into());
        }
    }
    eprintln!("floor: pong ended: the ping's run done");
    Ok(())
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
#[allow(clippy::similar_names, reason = "ping and pong name the two ends of a round trip")]
fn run_hairpin<W: Wire + Send>(
    mut ping_wire: W,
    mut pong_wire: W,
    peer: SocketAddrV4,
    messages: usize,
    warmup: usize,
    ping_cpu: Option<usize>,
    pong_cpu: Option<usize>,
) -> Result<(W, W), Box<dyn Error>> {
    std::thread::scope(|scope| {
        let ponger = std::thread::Builder::new()
            .name("floor-pong".into())
            .spawn_scoped(scope, move || {
                // its error as text: a boxed error does not cross threads
                let ended = pong_loop(&mut pong_wire, pong_cpu).map_err(|e| e.to_string());
                (pong_wire, ended)
            })
            .map_err(|e| format!("floor: the pong's thread: {e}"))?;
        let stop = StopOnDrop;
        let pinged = run_ping(&mut ping_wire, peer, messages, warmup, ping_cpu);
        drop(stop);
        let (pong_wire, ponged) = ponger.join().map_err(|_| "floor: the pong's thread failed")?;
        pinged?;
        ponged.map_err(|e| format!("floor: the pong: {e}"))?;
        Ok((ping_wire, pong_wire))
    })
}

fn run_ping(
    wire: &mut impl Wire,
    peer: SocketAddrV4,
    messages: usize,
    warmup: usize,
    cpu: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    connect(wire, peer)?;
    pin(cpu)?;
    let clock = Clock::calibrate();

    let budget = Duration::from_secs(env_u64("RTT_SECONDS", 20)?);
    let mut results = Results::new()?;
    let mut buffer = [0u8; DATAGRAM];
    // raw UDP repairs nothing: a ping or echo lost is counted lost after
    // this long (0 or unset: 50 ms), and the next sent
    let timeout_ns = match env_u64("RTT_LOST_TIMEOUT_US", 0)? {
        0 => 50_000_000,
        us => us.saturating_mul(1000),
    };
    let mut seq = 0_u64;
    let mut pace = Pace::from_env()?;
    let mut between = Between::from_env()?;
    let start = Instant::now();
    for i in 0..warmup {
        seq += 1;
        round_trip(
            wire,
            &clock,
            &mut buffer,
            &mut None,
            &mut pace,
            &mut between,
            seq,
            timeout_ns,
            |_| {},
        )?;
        between.flush();
        if (pace.paced() || i % 64 == 0) && start.elapsed() > budget / 4 {
            break;
        }
    }
    let start = Instant::now();
    results.start(&clock);
    let mut stalls = Stalls::new(&clock)?;
    let mut lost = 0_u64;
    for i in 0..messages {
        seq += 1;
        let echoed = round_trip(
            wire,
            &clock,
            &mut buffer,
            &mut stalls,
            &mut pace,
            &mut between,
            seq,
            timeout_ns,
            |rtt| {
                results.record(&clock, rtt, i);
            },
        )?;
        if !echoed {
            lost += 1;
        }
        between.flush();
        if (pace.paced() || i % 64 == 0) && start.elapsed() > budget {
            break;
        }
    }

    results.report(lost, PING_STREAM_ID)?;
    if let Some(stalls) = &stalls {
        stalls.write("stalls-ping.csv")?;
    }
    Ok(())
}

/// Probes (sequence 0) every 10 ms until the pong echoes one, for up to
/// `FLOOR_CONNECT_SECS` (30): until then a refused or unresolved send is
/// the pong not up yet, and retried.
fn connect(wire: &mut impl Wire, peer: SocketAddrV4) -> Result<(), Box<dyn Error>> {
    let secs = env_u64("FLOOR_CONNECT_SECS", 30)?;
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
                return Ok(());
            }
            if let Err(e) = polled {
                last = Some(e);
            }
        }
    }
    Err(format!(
        "floor: no echo from the pong at {peer} within {secs} s{}",
        last.map(|e| format!(" (last error: {e})")).unwrap_or_default()
    )
    .into())
}

fn seq_of(datagram: &[u8]) -> u64 {
    lab::le_u64(datagram, SEQ)
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
    between: &mut Between,
    seq: u64,
    timeout_ns: u64,
    mut record: impl FnMut(u64),
) -> Result<bool, Box<dyn Error>> {
    if let Some(s) = stalls.as_mut() {
        s.pass(clock.now());
    }
    loop {
        let now = clock.now();
        if pace.ready(now) {
            break;
        }
        if let Some(s) = stalls.as_mut() {
            s.pass(now);
        }
        if between.keepalive_due(now) {
            buffer[SEQ..SEQ + 8].copy_from_slice(&KEEPALIVE_SEQ.to_le_bytes());
            // a full ring or an ARP wait skips this one: the next is due anyway
            wire.send(buffer).map_err(|e| wire_failed("send", &e))?;
            between.last_keepalive = now;
        }
        if wire.poll(|_| {}).map_err(|e| wire_failed("poll", &e))? == 0 {
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
                between.last_ping = clock.now();
                break now;
            }
            Ok(false) => {
                // nothing of this ping can be in flight yet
                wire.poll(|_| {}).map_err(|e| wire_failed("poll", &e))?;
                spin_loop();
            }
            Err(e) => return Err(wire_failed("send", &e)),
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
                    let sent = lab::le_u64(reply, STAMP);
                    record(clock.stamp().wrapping_sub(sent));
                    echoed = true;
                }
            })
            .map_err(|e| wire_failed("poll", &e))?;
        if received == 0 {
            if clock.now().wrapping_sub(started) > timeout_ns {
                return Ok(false);
            }
            spin_loop();
        }
    }
    Ok(true)
}

#[cold]
#[inline(never)]
fn wire_failed(what: &str, error: &io::Error) -> Box<dyn Error> {
    format!("floor: {what}: {error}").into()
}

/// Echoes every datagram until SIGTERM; says so on stderr.
fn run_pong(wire: &mut impl Wire, cpu: Option<usize>) -> Result<(), Box<dyn Error>> {
    lab::stop_on_sigterm();
    pong_loop(wire, cpu)?;
    eprintln!("floor: pong ended: SIGTERM");
    Ok(())
}

/// Echoes every datagram, pinned to `cpu`, until [`STOP`]; then writes its
/// loop's stalls.
fn pong_loop(wire: &mut impl Wire, cpu: Option<usize>) -> Result<(), Box<dyn Error>> {
    pin(cpu)?;
    let clock = Clock::calibrate();
    let mut stalls = Stalls::new(&clock)?;
    let mut pending = [[0u8; DATAGRAM]; BURST];
    let mut lengths = [0usize; BURST];
    let mut between = Between::from_env()?;
    while !STOP.load(Ordering::Relaxed) {
        if let Some(s) = stalls.as_mut() {
            s.pass(clock.now());
        }
        let mut count = 0;
        wire.poll(|ping| {
            if let (Some(slot), Some(length)) = (pending.get_mut(count), lengths.get_mut(count)) {
                let n = ping.len().min(DATAGRAM);
                slot[..n].copy_from_slice(&ping[..n]);
                *length = n;
                count += 1;
            }
        })
        .map_err(|e| wire_failed("poll", &e))?;
        for (echo, &n) in pending.iter().zip(&lengths).take(count) {
            // a ping waits for its echo before the next, so a retry's poll
            // finds nothing of this run to lose
            while !wire.send(&echo[..n]).map_err(|e| wire_failed("send", &e))? {
                if STOP.load(Ordering::Relaxed) {
                    break;
                }
                wire.poll(|_| {}).map_err(|e| wire_failed("poll", &e))?;
                spin_loop();
            }
        }
        if count == 0 {
            spin_loop();
        } else if pending
            .iter()
            .zip(&lengths)
            .take(count)
            .any(|(d, &n)| n >= DATAGRAM && !matches!(seq_of(d), 0 | KEEPALIVE_SEQ))
        {
            // FLUSH_KB: the next ping finds this side's caches cold
            between.flush();
        }
    }
    if let Some(stalls) = &stalls {
        stalls.write("stalls-pong.csv")?;
    }
    Ok(())
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
#[allow(
    unsafe_code,
    reason = "the raw floor drives ergon-md-dpdk's C transport bindings directly, with no Aeron driver between"
)]
mod dpdk {
    use super::{BURST, Wire};
    use ergon_md_dpdk::DpdkTransport;
    use rusteron_media_driver::bindings::{
        aeron_clock_cache_t, aeron_counters_manager_init, aeron_counters_manager_t, aeron_driver_context_t,
        aeron_threading_mode_t, aeron_udp_channel_data_paths_t, aeron_udp_channel_transport_affinity_t,
        aeron_udp_channel_transport_bindings_t, aeron_udp_channel_transport_params_t, aeron_udp_channel_transport_t,
        aeron_udp_transport_poller_t, iovec, sockaddr_storage, timespec,
    };
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
            let mut metadata = vec![0u64; COUNTER_SLOTS * METADATA_LENGTH / 8].into_boxed_slice();
            let mut values = vec![0u64; COUNTER_SLOTS * VALUE_LENGTH / 8].into_boxed_slice();
            // SAFETY: a plain C struct of integers, all zero a valid state.
            let mut clock: Box<aeron_clock_cache_t> = Box::new(unsafe { std::mem::zeroed() });
            // SAFETY: as above; aeron_counters_manager_init fills it.
            let mut manager: Box<aeron_counters_manager_t> = Box::new(unsafe { std::mem::zeroed() });
            // SAFETY: both buffers outlive the manager (same struct, never dropped
            // while the transport runs), and their lengths are theirs.
            let rc = unsafe {
                aeron_counters_manager_init(
                    &raw mut *manager,
                    metadata.as_mut_ptr().cast::<u8>(),
                    metadata.len() * 8,
                    values.as_mut_ptr().cast::<u8>(),
                    values.len() * 8,
                    &raw mut *clock,
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
    #[allow(
        clippy::unnecessary_box_returns,
        reason = "the bindings keep pointers to it, so it lives at one address while the wire holding it moves"
    )]
    #[allow(
        clippy::cast_possible_truncation,
        reason = "AF_INET is 2, which every sa_family_t holds"
    )]
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
    #[allow(
        clippy::non_send_fields_in_send_ty,
        reason = "its raw pointers are the installed bindings' table and the wire's own boxed C state, used by one thread at a time"
    )]
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
                let init = (*self.bindings)
                    .init_func
                    .ok_or_else(|| io::Error::other("no init_func"))?;
                init(
                    &raw mut *transport,
                    (&raw mut *bind).cast(),
                    (&raw mut *any).cast(),
                    connect.as_mut().map_or(ptr::null_mut(), |c| (&raw mut **c).cast()),
                    &raw mut params,
                    &raw mut *self.init_context,
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
                let init = b
                    .poller_init_func
                    .ok_or_else(|| io::Error::other("no poller_init_func"))?;
                let add = b
                    .poller_add_func
                    .ok_or_else(|| io::Error::other("no poller_add_func"))?;
                if init(
                    &raw mut *poller,
                    &raw mut *self.init_context,
                    aeron_udp_channel_transport_affinity_t::AERON_UDP_CHANNEL_TRANSPORT_AFFINITY_RECEIVER,
                ) < 0
                {
                    return Err(last_error("the poller"));
                }
                for t in transports {
                    if add(&raw mut *poller, &raw mut **t) < 0 {
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
                // the send reads the datagram, never writes it
                iov_base: datagram.as_ptr().cast_mut().cast::<c_void>(),
                iov_len: datagram.len(),
            };
            let mut bytes: i64 = 0;
            // SAFETY: the bindings are the installed table, live as long as the wire.
            let Some(send) = (unsafe { (*self.bindings).send_func }) else {
                return Err(io::Error::other("no send_func"));
            };
            // SAFETY: the transport and address are live; one iovec over `datagram`.
            let rc = unsafe {
                send(
                    &raw mut *self.data_paths,
                    &raw mut *self.tx,
                    (&raw mut *self.peer).cast(),
                    &raw mut iov,
                    1,
                    &raw mut bytes,
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
            // SAFETY: the bindings are the installed table, live as long as the wire.
            let Some(poll) = (unsafe { (*self.bindings).poller_poll_func }) else {
                return Err(io::Error::other("no poller_poll_func"));
            };
            // SAFETY: the poller and its transports are live; the msgvec has
            // BURST entries; `target` outlives the call.
            let rc = unsafe {
                poll(
                    &raw mut *self.poller,
                    self.msgvec.as_mut_ptr().cast(),
                    BURST,
                    &raw mut bytes,
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

/// The transport suite's raw floor: `floor --role ping|pong` (the module
/// docs' second half).
mod transport {
    use super::point::{self, Common, Error, Flags, Recorder, Report, Role, Rx, Sent, Tx, thread_allocs};
    use super::{Wire, lab};
    use ergon_latency_core::clock::TscClock;
    use ergon_latency_core::manifest::RunManifest;
    use ergon_latency_core::message::{MESSAGE_LEN, echo_in_place, template};
    use std::cell::RefCell;
    use std::net::SocketAddrV4;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    /// The datagram: a 32-byte header where Aeron's would be, the message.
    const HEADER: usize = 32;
    const DATAGRAM: usize = HEADER + MESSAGE_LEN;
    /// UDP, IPv4 and Ethernet headers under a datagram.
    const UDP_IP_ETH: u64 = 8 + 20 + 14;
    const DATA: u8 = 1;
    const PROBE: u8 = 2;
    /// Echoes held from one poll before they go back.
    const BURST: usize = 16;

    const USAGE: &str = "\
floor --role ping|pong --rate MSG_PER_S --label SET:ARM --out DIR --endpoint HOST:PORT --reply HOST:PORT
      [--warmup-s 30] [--duration-s 30] [--size 64] [--cpus CPU] [--spike-ns 100000] [--stall-ns 10000]
      [--connect-timeout-s 30]
The raw floor (no Aeron): kernel UDP sockets, or ergon-md-dpdk's packet path alone under
RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena (the driver's environment), one stream, one busy thread an end.";

    /// The ends' shared wire, sent on and polled in turn by one thread.
    struct End<'a, W>(&'a RefCell<W>, [u8; DATAGRAM]);

    impl<W: Wire> Tx for End<'_, W> {
        #[inline]
        fn try_send(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> Result<Sent, Error> {
            if stream != 0 {
                return Err(Error::Transport(format!("the raw floor has one stream, not {stream}")));
            }
            self.1[0] = DATA;
            self.1[HEADER..].copy_from_slice(msg);
            match self.0.borrow_mut().send(&self.1) {
                Ok(true) => Ok(Sent::Sent),
                Ok(false) => Ok(Sent::Busy),
                Err(e) => Err(Error::Transport(format!("send: {e}"))),
            }
        }
    }

    impl<W: Wire> Rx for End<'_, W> {
        #[inline]
        fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
            self.0
                .borrow_mut()
                .poll(|d| {
                    // a probe's echo, late, is not a ping's
                    if d.len() == DATAGRAM && d[0] == DATA {
                        on(&d[HEADER..]);
                    }
                })
                .map_err(|e| Error::Transport(format!("poll: {e}")))
        }
    }

    fn address(f: &mut Flags, flag: &str) -> Result<SocketAddrV4, Error> {
        let v = f.req(flag)?;
        v.parse()
            .map_err(|_| Error::Usage(format!("--{flag} {v}: an IPv4 HOST:PORT")))
    }

    /// The transport's name in the manifest and its wire's.
    fn transport() -> Result<(&'static str, &'static str), Error> {
        match ergon_md_dpdk::env::selector().map_err(|e| Error::Usage(e.to_string()))? {
            ergon_md_dpdk::Selector::Default => Ok(("raw-udp", "kernel")),
            ergon_md_dpdk::Selector::DpdkEna => Ok(("raw-dpdk", "dpdk")),
        }
    }

    pub fn main() -> i32 {
        if std::env::args().skip(1).any(|a| a == "--help") {
            println!("{USAGE}");
            return 0;
        }
        let go = || -> Result<Option<String>, Error> {
            let mut f = Flags::parse(std::env::args().skip(1))?;
            let c = Common::parse(&mut f)?;
            let endpoint = address(&mut f, "endpoint")?;
            let reply = address(&mut f, "reply")?;
            let connect = Duration::from_secs(f.num("connect-timeout-s", 30)?);
            f.finish()?;
            if !matches!(c.role, Role::Ping | Role::Pong) || c.streams != 1 {
                return Err(Error::Usage(
                    "the raw floor is one stream between two hosts: --role ping or pong, --streams 1".into(),
                ));
            }
            c.check_cpus(1, if c.role == Role::Ping { "ping" } else { "echo" })?;
            let (name, wire_name) = transport()?;
            let mut m = RunManifest::new(name, &template(), DATAGRAM as u64 + UDP_IP_ETH);
            m.setting("transport", name)
                .setting("mode", "raw")
                .setting("wire", wire_name)
                .setting("driver", "none")
                .setting("datagram", &DATAGRAM.to_string())
                .setting("endpoint", &endpoint.to_string())
                .setting("reply", &reply.to_string())
                .setting("thread_order", if c.role == Role::Ping { "ping" } else { "echo" });
            // each end listens on its own address and sends to the other's
            let (listen, peer) = if c.role == Role::Ping {
                (reply, endpoint)
            } else {
                (endpoint, reply)
            };
            match ergon_md_dpdk::env::selector().map_err(|e| Error::Usage(e.to_string()))? {
                ergon_md_dpdk::Selector::Default => {
                    let wire = super::KernelWire::open(listen, peer)
                        .map_err(|e| Error::Transport(format!("kernel UDP: {e}")))?;
                    run(&c, wire, peer, connect, m)
                }
                ergon_md_dpdk::Selector::DpdkEna => dpdk(&c, listen, peer, connect, m),
            }
        };
        match go() {
            Ok(row) => {
                if let Some(row) = row {
                    println!("{row}");
                }
                0
            }
            Err(e) => {
                eprintln!("floor: {e}");
                1
            }
        }
    }

    #[cfg(all(feature = "dpdk", target_os = "linux"))]
    fn dpdk(
        c: &Common,
        listen: SocketAddrV4,
        peer: SocketAddrV4,
        connect: Duration,
        m: RunManifest,
    ) -> Result<Option<String>, Error> {
        let mut shared = super::dpdk::DpdkShared::open(c.cpu(0)).map_err(|e| Error::Transport(format!("DPDK: {e}")))?;
        let wire = shared
            .cross(listen, peer)
            .map_err(|e| Error::Transport(format!("DPDK: {e}")))?;
        let r = run(c, wire, peer, connect, m);
        // the native runtime's threads may still read what it holds
        std::mem::forget(shared);
        r
    }

    #[cfg(not(all(feature = "dpdk", target_os = "linux")))]
    fn dpdk(
        _c: &Common,
        _listen: SocketAddrV4,
        _peer: SocketAddrV4,
        _connect: Duration,
        _m: RunManifest,
    ) -> Result<Option<String>, Error> {
        Err(Error::Usage(
            "RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena, but this build has no DPDK (the harness's dpdk feature, Linux)"
                .into(),
        ))
    }

    /// Probes every 10 ms until the pong echoes one, within `limit`.
    fn probe<W: Wire>(wire: &RefCell<W>, peer: SocketAddrV4, limit: Duration) -> Result<(), Error> {
        let deadline = Instant::now() + limit;
        let mut probe = [0u8; DATAGRAM];
        probe[0] = PROBE;
        let mut last = None;
        while Instant::now() < deadline {
            if let Err(e) = wire.borrow_mut().send(&probe) {
                last = Some(e);
            }
            let until = Instant::now() + Duration::from_millis(10);
            while Instant::now() < until {
                let mut echoed = false;
                match wire
                    .borrow_mut()
                    .poll(|d| echoed |= d.len() == DATAGRAM && d[0] == PROBE)
                {
                    Ok(_) if echoed => return Ok(()),
                    Ok(_) => {}
                    Err(e) => last = Some(e),
                }
            }
        }
        Err(Error::Timeout(format!(
            "no echo from the pong at {peer} within {} s{}",
            limit.as_secs(),
            last.map(|e| format!(" (last error: {e})")).unwrap_or_default()
        )))
    }

    fn run<W: Wire>(
        c: &Common,
        wire: W,
        peer: SocketAddrV4,
        connect: Duration,
        mut m: RunManifest,
    ) -> Result<Option<String>, Error> {
        let wire = RefCell::new(wire);
        if c.role == Role::Pong {
            lab::stop_on_sigterm();
            println!("ready");
            point::pin(c.cpu(0))?;
            let a0 = thread_allocs();
            let echoes = pong(&wire)?;
            let allocs = thread_allocs() - a0;
            m.setting("echoes_sent", &echoes.to_string())
                .setting("hot_path_allocs", &allocs.to_string())
                .setting("busy_threads", "1");
            point::write_manifest(c, m)?;
            return Ok(None);
        }
        probe(&wire, peer, connect)?;
        point::pin(c.cpu(0))?;
        let clock = TscClock::new();
        let (t, rt) = point::start_at(&clock, point::START_DELAY_NS);
        let mut rec = Recorder::new(&clock, t, c)?;
        let (mut tx, mut rx) = (End(&wire, [0u8; DATAGRAM]), End(&wire, [0u8; DATAGRAM]));
        let a0 = thread_allocs();
        let send = point::ping(&clock, c, t, &mut tx, &mut rx, &mut rec)?;
        let allocs = thread_allocs() - a0;
        let end = clock.raw();
        point::finish(
            c,
            &clock,
            rec,
            send,
            rt,
            end,
            Report {
                channel: "udp",
                retrans: None,
                wire_bytes: DATAGRAM as u64 + UDP_IP_ETH,
                busy_threads: 1,
                hot_path_allocs: allocs,
                manifest: m,
            },
        )
        .map(Some)
    }

    /// Echoes every datagram (a ping turned into its echo, a probe as it
    /// came) until SIGTERM; the pings echoed. A send that cannot go yet
    /// (`false`: the DPDK transport's first send to a next hop while its ARP
    /// resolution is in flight, or a full ring) is retried after a poll,
    /// never in a tight loop of sends: the ARP answer and the ring's
    /// completions come in on the poll, and without one the resolution
    /// times out into a hard error (aws-20261028-090309: the pong's first
    /// echo, "next hop unreachable (ARP failed)"). What arrives meanwhile
    /// queues behind the echoes held, up to `BURST`.
    fn pong<W: Wire>(wire: &RefCell<W>) -> Result<u64, Error> {
        let mut pending = [[0u8; DATAGRAM]; BURST];
        let (mut sent, mut count) = (0usize, 0usize);
        let mut echoes = 0u64;
        while !lab::STOP.load(Ordering::Relaxed) {
            if sent < count {
                if wire
                    .borrow_mut()
                    .send(&pending[sent])
                    .map_err(|e| Error::Transport(format!("send: {e}")))?
                {
                    sent += 1;
                    continue;
                }
                std::hint::spin_loop();
            } else {
                (sent, count) = (0, 0);
            }
            let first = count;
            let mut bad = None;
            wire.borrow_mut()
                .poll(|d| {
                    if d.len() != DATAGRAM {
                        bad = Some(d.len());
                    } else if let Some(slot) = pending.get_mut(count) {
                        slot.copy_from_slice(d);
                        count += 1;
                    }
                })
                .map_err(|e| Error::Transport(format!("poll: {e}")))?;
            if let Some(len) = bad {
                return Err(Error::Payload {
                    count: 1,
                    first: format!("a {len}-byte datagram, not {DATAGRAM}"),
                });
            }
            for d in pending.iter_mut().take(count).skip(first) {
                if d[0] == DATA {
                    let msg: &mut [u8; MESSAGE_LEN] = (&mut d[HEADER..])
                        .try_into()
                        .map_err(|_| Error::Transport("a datagram's message".into()))?;
                    echo_in_place(msg)?;
                    echoes += 1;
                }
            }
        }
        Ok(echoes)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use ergon_latency_core::message::Message;
        use std::collections::VecDeque;
        use std::io;

        /// A wire whose sends wait, as the DPDK transport's do while the
        /// echo's next hop is being ARP'd (`send` 0), until a poll has taken
        /// the ARP answer in; sent to 1000 times with no poll between, it
        /// fails as that transport does once the resolution times out
        /// ("next hop unreachable (ARP failed)", aws-20261028-090309).
        struct Arp {
            inbox: VecDeque<[u8; DATAGRAM]>,
            asked: bool,
            resolved: bool,
            unpolled_sends: u32,
            idle_polls: u32,
            sent: Vec<Vec<u8>>,
            expect: usize,
        }

        impl Wire for Arp {
            fn send(&mut self, datagram: &[u8]) -> io::Result<bool> {
                if !self.resolved {
                    self.asked = true;
                    self.unpolled_sends += 1;
                    if self.unpolled_sends > 1000 {
                        return Err(io::Error::other("next hop unreachable (ARP failed)"));
                    }
                    return Ok(false);
                }
                self.sent.push(datagram.to_vec());
                if self.sent.len() == self.expect {
                    lab::STOP.store(true, Ordering::Relaxed);
                }
                Ok(true)
            }

            fn poll<F: FnMut(&[u8])>(&mut self, mut on: F) -> io::Result<usize> {
                // the ARP answer comes in on the poll after the request
                self.resolved |= self.asked;
                self.unpolled_sends = 0;
                if let Some(d) = self.inbox.pop_front() {
                    self.idle_polls = 0;
                    on(&d);
                    return Ok(1);
                }
                // a datagram the pong dropped would never be echoed: end
                self.idle_polls += 1;
                if self.idle_polls > 1_000_000 {
                    lab::STOP.store(true, Ordering::Relaxed);
                }
                Ok(0)
            }
        }

        #[test]
        fn the_pong_polls_while_its_echo_waits_for_arp_and_loses_nothing_meanwhile() -> Result<(), Error> {
            let mut probe = [0u8; DATAGRAM];
            probe[0] = PROBE;
            let mut ping = [0u8; DATAGRAM];
            ping[0] = DATA;
            ping[HEADER..].copy_from_slice(&template());
            // the probe comes first; the ping arrives while its echo waits
            let wire = RefCell::new(Arp {
                inbox: VecDeque::from([probe, ping]),
                asked: false,
                resolved: false,
                unpolled_sends: 0,
                idle_polls: 0,
                sent: Vec::new(),
                expect: 2,
            });
            lab::STOP.store(false, Ordering::Relaxed);
            let echoes = pong(&wire);
            lab::STOP.store(false, Ordering::Relaxed);
            assert_eq!(echoes?, 1, "the ping echoed, the probe sent back as it came");
            let sent = &wire.borrow().sent;
            assert_eq!(sent.len(), 2, "both went back: {sent:?}");
            assert_eq!(sent[0].as_slice(), probe.as_slice(), "the probe first, as it came");
            assert_eq!(sent[1][0], DATA);
            let echo = Message::decode(&sent[1][HEADER..]).map_err(|e| Error::Transport(format!("{e:?}")))?;
            assert_eq!(echo.kind, ergon_latency_core::message::Kind::Echo);
            Ok(())
        }
    }
}
