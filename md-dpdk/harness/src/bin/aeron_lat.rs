//! The transport latency suite's Aeron arms: IPC, spy, spy on a live UDP
//! stream, kernel UDP and DPDK.
//!
//! One binary against a media driver (`--driver`):
//!
//! | `--driver` | the media driver | its agents |
//! |---|---|---|
//! | `external` (default) | a process of its own at `--aeron-dir` (the runner's `vm.sh driver`) | its threads, as that process's threading mode has them |
//! | `embedded` | this process starts it on a thread of its own, which runs its first agent | the threads the environment's `AERON_THREADING_MODE` gives it (the conductor that thread; under `DEDICATED` a sender and a receiver thread too) |
//! | `invoker` | this process starts it in Aeron's `INVOKER` mode on the measuring thread | none: the measuring thread runs the driver's agents and its own client's conductor itself, every pass of its loop, and after every offer (the sender's work, so the frame leaves at once) |
//!
//! An embedded or invoked driver is configured from the environment as the
//! bundled `media_driver` is (`AERON_*`, `RUSTERON_DPDK_*`,
//! `RUSTERON_MEDIA_DRIVER_TRANSPORT`), through
//! `ergon_md_dpdk::driver::ConfiguredDriver`; the invoked one's threading
//! mode is `INVOKER`, its conductor CPU (where Aeron pins the thread that
//! starts it) the measuring thread's `--cpus`, and it never idles, whatever
//! the environment says. `invoker` is `--role ping` or `pong` over `--mode
//! udp` alone: one measuring thread a process, so no `local` (two) and no
//! spy-udp sink. Every measurement, the echo and the files are
//! `io_bench::transport::point`'s, taken by path, so the Aeron arms measure
//! exactly as the others do.
//!
//! ```text
//! aeron_lat --role <ping|pong|local> [--round-trip] --mode <ipc|spy|spy-udp|udp> --rate MSG_PER_S
//!           --label TEXT --out DIR --aeron-dir PATH [--driver external|embedded|invoker]
//!           [--reliable true|false] [--endpoint HOST:PORT] [--reply HOST:PORT]
//!           [--interface IP] [--term-length 1048576] [--mtu 1408] [--stream-id 1001]
//!           [--driver-busy-threads 3 (invoker: 0)] [--connect-timeout-s 30]
//!           [--streams N] [--warmup-s 30] [--duration-s 30] [--size 64] [--cpus CSV]
//!           [--spike-ns 100000] [--stall-ns 10000] [--stage-trace N]
//! ```
//!
//! | role | busy threads, in `--cpus` order | row |
//! |---|---|---|
//! | `local` | sender, receiver | `oneway` |
//! | `local --round-trip` | ping, echo | `load` |
//! | `ping` | ping (sends, polls the echoes) | `load` |
//! | `pong` | echo | none: `manifest.json` alone |
//! | `pong --mode spy-udp` | sink (reads every frame, echoes nothing) | none: `manifest.json` alone |
//!
//! A `pong` prints one `ready` line once its subscriptions and publications
//! are added (the runner starts the ping on it); its echo then waits for the
//! ping's (`--connect-timeout-s`). No stream's port may fall in the media
//! driver's wildcard ranges (20000-21999 in the lab).
//!
//! Each busy thread has its own Aeron client, created before it pins
//! itself, so the client's conductor thread keeps the process's starting
//! affinity (the runner starts the binary on a housekeeping CPU). Under
//! `--driver invoker` the one client's conductor is the measuring thread's
//! to run (the agent invoker): no conductor thread.
//!
//! | `--mode` | stream `i`'s ping (stream id `S+i`) | its echo (stream id `S+10000+i`) |
//! |---|---|---|
//! | `ipc` | `aeron:ipc` | `aeron:ipc` |
//! | `spy` | a manual-MDC UDP publication with no destinations (`ssc=true`, `tags=` its stream id), read by `aeron-spy:` on that tag | the same |
//! | `spy-udp` | `endpoint=<--endpoint host>:<port+i>` (`ssc=false`), read by `aeron-spy:` on that channel | `endpoint=<--reply host>:<port+i>`, the same |
//! | `udp` | `endpoint=<--endpoint host>:<port+i>` | `endpoint=<--reply host>:<port+i>` |
//!
//! `spy` sends nothing on the wire: its publication has no destination, so
//! the spy reads it as IPC through a UDP publication's term buffer.
//! `spy-udp` measures the same path (`--role local`: publication, spy,
//! echo, spy) on publications that each send every frame to another host:
//! there `--role pong --mode spy-udp` is a sink subscribed to both channels
//! (`--endpoint` and `--reply` are both its addresses). The publications are
//! reliable (Aeron's default) and `ssc=false`, so only the sink's status
//! messages connect them and drive their flow control, never the spy: a
//! `--role local` with no sink times out waiting for them. The sink checks
//! each stream's sequence numbers and writes its counts to its manifest
//! (`sink.*`); the measuring end records `echoes_sent` and the driver's
//! `driver.bytes-sent`, which fairness.py holds the sink's counts against.
//!
//! Every publication: `term-length`, `mtu` and `sparse=false`; `--interface`
//! is added to the UDP publications. `--reliable false` (UDP only) adds the
//! md-dpdk lab's `reliable=false|group=false|tether=false` to the UDP
//! subscriptions. The row's `retrans` is the growth of the driver's
//! `Retransmits sent` counter over the point (this host's driver), and the
//! manifest records its NAK, loss-gap-fill and short-send counters too.
//!
//! `--stage-trace N` (a power of two; ping, pong and `--role local
//! --round-trip`): the application's end of md-dpdk's stage trace
//! (`RUSTERON_DPDK_STAGE_TRACE`, the README's "Stage trace"). Every message
//! whose seq is a multiple of N is stamped on the CPU counter as its offer
//! starts (`app-offer`, the first attempt) and returns (`app-offered`), and
//! as its poll hands it over (`app-read`, `value` the payload's `send_tsc`),
//! into a preallocated log of each measuring thread's (no allocation while
//! measuring; a full log drops and counts). After the run they go to
//! `<out>/stages-app.csv`: `#` lines (role, N, the counter's ticks per ns,
//! the schedule's start and the first measured slot's counter on a ping),
//! then `stage,tsc,stream_id,seq,term_offset,value,ring` rows, the media
//! driver's columns (stream id the Aeron stream's, term offset 0, ring 0 the
//! sending log, 1 the receiving one). md-dpdk/benches/cloud/transport_stages.py
//! joins them with each host's driver file.

// io/benches formats it (rustfmt's defaults), not this crate (max_width 120).
#[path = "../../../../io/benches/src/transport/point.rs"]
#[rustfmt::skip]
mod point;

#[path = "aeron_lat/mixed.rs"]
mod mixed;

use ergon_latency_core::clock::TscClock;
use ergon_latency_core::manifest::RunManifest;
use ergon_latency_core::message::{MESSAGE_LEN, template};
use ergon_md_dpdk::driver::{ConfiguredDriver, EnvMediaDriver, threading_mode_name};
use point::{
    Common, CountingAlloc, Error, Flags, POLL_LIMIT, Progress, Recorder, Report, Role, Rx, START_DELAY_NS, SendStats,
    Sent, Tx, thread_allocs,
};
use rusteron_client::{
    Aeron, AeronCError, AeronContext, AeronExclusivePublication, AeronImage, AeronSubscription, Handlers,
};
use rusteron_media_driver::bindings::aeron_threading_mode_t;
use rusteron_media_driver::{AeronDriverContext, AeronIdleStrategyKind};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::process::ExitCode;
use std::sync::Barrier;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

#[global_allocator]
static ALLOC: CountingAlloc = CountingAlloc;

/// An echo's stream id is its ping's plus this.
const ECHO_OFFSET: i32 = 10_000;
/// Aeron's data frame header, on every message.
const DATA_HEADER: u64 = 32;
/// UDP, IPv4 and Ethernet headers under a UDP frame.
const UDP_IP_ETH: u64 = 8 + 20 + 14;
/// The driver's counters the manifest records, by label prefix.
const LOSS_COUNTERS: [(&str, &str); 6] = [
    ("NAKs sent", "naks-sent"),
    ("NAKs received", "naks-received"),
    ("Retransmits sent", "retransmits-sent"),
    ("Loss gap fills", "loss-gap-fills"),
    ("Short sends", "short-sends"),
    ("Errors", "errors"),
];
/// The driver's traffic counters the manifest records (`driver.<key>`):
/// what this host's driver put on and took off the wire.
const WIRE_COUNTERS: [(&str, &str); 2] = [("Bytes sent", "bytes-sent"), ("Bytes received", "bytes-received")];

/// `--help`.
const USAGE: &str = "\
aeron_lat --role ping|pong|local [--round-trip] --mode ipc|spy|spy-udp|udp --rate MSG_PER_S --label SET:ARM --out DIR
          --aeron-dir PATH [--driver external|embedded|invoker] [--driver-busy-threads 3] [--reliable true|false]
          [--endpoint HOST:PORT] [--reply HOST:PORT] [--interface IP] [--term-length 1048576] [--mtu 1408]
          [--stream-id 1001] [--connect-timeout-s 30] [--streams 1] [--warmup-s 30] [--duration-s 30]
          [--size 64] [--cpus CSV] [--spike-ns 100000] [--stall-ns 10000] [--stage-trace N]
          [--load uniform|mixed --hot-rate MSG_PER_S]
--stage-trace N: stamp every Nth message (a power of two, by seq) at offer and read into <out>/stages-app.csv
  (ping, pong, local --round-trip; under --load mixed the hot stream's): the application's end of md-dpdk's
  RUSTERON_DPDK_STAGE_TRACE.
--mode udp: stream i's pings go to --endpoint port + i (stream id S+i), its echoes to --reply port + i
  (S+10000+i); ping and pong take the same --endpoint (the pong's address) and --reply (the ping's), and
  --interface is this end's own address. --endpoint, --reply and --interface are UDP only. Ports
  20000-21999 (the media driver's wildcard ranges) are refused.
--mode spy-udp: --role local measures as --mode spy does, its pings published to --endpoint and its echoes
  to --reply, both a sink's addresses on another host (reliable, ssc=false: the sink connects them);
  --role pong is that sink: it reads every frame of both channels and writes its counts (sink.*).
--cpus: ping: ping; pong: echo; local: sender,receiver (--round-trip: ping,echo).
--driver: external (a media driver process at --aeron-dir), embedded (this process starts one on a thread
  of its own, configured from the environment), invoker (Aeron's INVOKER mode: the measuring thread runs
  the driver's agents and its client's conductor itself; --role ping or pong, --mode udp).
--driver-busy-threads: the driver's busy threads (DEDICATED: 3; invoker: 0, its only value), counted in
  the row's busy_threads.
--load mixed (--role ping or pong, --mode udp): stream 0 the hot stream at --hot-rate (the row's latency), streams
  1 to --streams - 1 a background of steady, bursty and idle-then-burst streams of 64 to 4096 B, two publishers on
  some, fan-out 1 to 4 on the pong, together --rate (the row's); --cpus hot,background (mixed.rs).
A pong prints `ready` once its channels are added, and no row.";
/// The media driver's wildcard port ranges in the lab (vm.sh `common_env`).
const WILDCARD_PORTS: std::ops::RangeInclusive<u32> = 20_000..=21_999;

/// SIGTERM: a pong stops.
static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigterm(_signal: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

#[allow(unsafe_code)]
fn stop_on_sigterm() {
    let handler: extern "C" fn(libc::c_int) = on_sigterm;
    // SAFETY: the handler only stores to an atomic, which is async-signal-safe.
    unsafe { libc::signal(libc::SIGTERM, handler as libc::sighandler_t) };
}

fn aeron_err<E: std::fmt::Debug>(what: &str) -> impl Fn(E) -> Error + '_ {
    move |e| Error::Transport(format!("{what}: {e:?}"))
}

/// `--mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Ipc,
    Spy,
    SpyUdp,
    Udp,
}

/// `--driver`: where the media driver runs, and who runs its agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Driver {
    /// A process of its own at `--aeron-dir`.
    External,
    /// This process's, on a thread of its own.
    Embedded,
    /// This process's, run by the measuring thread (Aeron's `INVOKER`).
    Invoker,
}

impl Driver {
    const fn name(self) -> &'static str {
        match self {
            Self::External => "external",
            Self::Embedded => "embedded",
            Self::Invoker => "invoker",
        }
    }
}

/// `HOST:PORT`, the port a base.
#[derive(Debug, Clone)]
struct Endpoint {
    host: String,
    port: u16,
}

impl Endpoint {
    fn parse(flag: &str, s: &str) -> Result<Self, Error> {
        let (host, port) = s
            .rsplit_once(':')
            .ok_or_else(|| Error::Usage(format!("--{flag} {s}: HOST:PORT")))?;
        let port = port
            .parse()
            .map_err(|_| Error::Usage(format!("--{flag} {s}: HOST:PORT")))?;
        if host.is_empty() {
            return Err(Error::Usage(format!("--{flag} {s}: HOST:PORT")));
        }
        Ok(Self {
            host: host.to_owned(),
            port,
        })
    }

    /// Stream `i`'s `host:port+i`, refused in [`WILDCARD_PORTS`].
    fn of(&self, flag: &str, i: u32) -> Result<String, Error> {
        let port = u32::from(self.port) + i;
        if WILDCARD_PORTS.contains(&port) {
            return Err(Error::Usage(format!(
                "--{flag}: stream {i}'s port {port}: 20000-21999 are the media driver's wildcard ranges"
            )));
        }
        u16::try_from(port)
            .map(|p| format!("{}:{p}", self.host))
            .map_err(|_| Error::Usage(format!("--{flag}: port {port} past 65535")))
    }
}

/// The Aeron settings.
#[derive(Debug, Clone)]
struct Aeronish {
    mode: Mode,
    reliable: bool,
    endpoint: Option<Endpoint>,
    reply: Option<Endpoint>,
    interface: Option<String>,
    dir: String,
    driver: Driver,
    term_length: u32,
    mtu: u32,
    stream_id: i32,
    driver_busy_threads: u32,
    connect_timeout: Duration,
    /// `--stage-trace N`: every Nth message stamped (0: off).
    stage_every: u64,
    /// `--wire`: the wire the runner set the arm up on (UDP modes).
    wire: Option<Wire>,
}

/// `--wire`: kernel UDP or DPDK under the driver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Wire {
    Kernel,
    Dpdk,
}

impl Wire {
    fn parse(v: Option<&str>) -> Result<Option<Self>, Error> {
        match v {
            None => Ok(None),
            Some("kernel") => Ok(Some(Self::Kernel)),
            Some("dpdk") => Ok(Some(Self::Dpdk)),
            Some(other) => Err(Error::Usage(format!("--wire {other}: kernel or dpdk"))),
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Kernel => "kernel",
            Self::Dpdk => "dpdk",
        }
    }
}

impl Aeronish {
    fn parse(f: &mut Flags, c: &Common) -> Result<Self, Error> {
        let mode = match f.req("mode")?.as_str() {
            "ipc" => Mode::Ipc,
            "spy" => Mode::Spy,
            "spy-udp" => Mode::SpyUdp,
            "udp" => Mode::Udp,
            other => return Err(Error::Usage(format!("--mode {other}: ipc, spy, spy-udp or udp"))),
        };
        let endpoint = f
            .take("endpoint")
            .map(|e| Endpoint::parse("endpoint", &e))
            .transpose()?;
        let reply = f.take("reply").map(|e| Endpoint::parse("reply", &e)).transpose()?;
        let driver = match f.take("driver").as_deref() {
            None | Some("external") => Driver::External,
            Some("embedded") => Driver::Embedded,
            Some("invoker") => Driver::Invoker,
            Some(other) => return Err(Error::Usage(format!("--driver {other}: external, embedded or invoker"))),
        };
        let a = Self {
            mode,
            reliable: f.bool("reliable", true)?,
            endpoint,
            reply,
            interface: f.take("interface"),
            dir: f.req("aeron-dir")?,
            driver,
            term_length: f.num("term-length", 1_048_576)?,
            mtu: f.num("mtu", 1408)?,
            stream_id: f.num("stream-id", 1001)?,
            driver_busy_threads: f.num("driver-busy-threads", if driver == Driver::Invoker { 0 } else { 3 })?,
            connect_timeout: Duration::from_secs(f.num("connect-timeout-s", 30)?),
            stage_every: f.num("stage-trace", 0)?,
            wire: Wire::parse(f.take("wire").as_deref())?,
        };
        a.check_stage_trace(c)?;
        if a.wire.is_some() && !matches!(mode, Mode::Udp | Mode::SpyUdp) {
            return Err(Error::Usage(format!("--wire is UDP's, not --mode {}", a.mode_name())));
        }
        if matches!(mode, Mode::Udp | Mode::SpyUdp) {
            if a.endpoint.is_none() {
                return Err(Error::Usage(format!(
                    "--mode {} needs --endpoint HOST:PORT",
                    a.mode_name()
                )));
            }
            if c.is_round_trip() && a.reply.is_none() {
                return Err(Error::Usage(
                    "a UDP round trip needs --reply HOST:PORT for the echoes".into(),
                ));
            }
            a.check_spy_udp(c)?;
            a.check_invoker(c)?;
        } else {
            if !a.reliable {
                return Err(Error::Usage(format!(
                    "--reliable false is UDP only, not --mode {}",
                    a.mode_name()
                )));
            }
            if a.endpoint.is_some() || a.reply.is_some() || a.interface.is_some() {
                return Err(Error::Usage(format!(
                    "--endpoint, --reply and --interface are UDP only, not --mode {}",
                    a.mode_name()
                )));
            }
        }
        for i in 0..c.streams {
            if let Some(e) = &a.endpoint {
                e.of("endpoint", i)?;
            }
            if let Some(e) = &a.reply {
                e.of("reply", i)?;
            }
        }
        if !a.term_length.is_power_of_two() || a.term_length < 65_536 {
            return Err(Error::Usage(format!(
                "--term-length {}: a power of two of at least 64 KiB",
                a.term_length
            )));
        }
        if a.driver == Driver::Invoker && a.mode != Mode::Udp {
            return Err(Error::Usage(format!(
                "--driver invoker is --mode udp's alone, not --mode {}",
                a.mode_name()
            )));
        }
        if a.mtu < 64 + 32 {
            return Err(Error::Usage(format!(
                "--mtu {}: room for the message and its header",
                a.mtu
            )));
        }
        let last = i64::from(a.stream_id) + i64::from(ECHO_OFFSET) + i64::from(c.streams);
        if a.stream_id <= 0 || last > i64::from(i32::MAX) {
            return Err(Error::Usage(format!(
                "--stream-id {}: 1 and up, with room for the streams",
                a.stream_id
            )));
        }
        Ok(a)
    }

    /// `--stage-trace N`: a power of two up to 2^20, on an end that both
    /// offers and reads (ping, pong, `--role local --round-trip`), never the
    /// spy-udp sink.
    fn check_stage_trace(&self, c: &Common) -> Result<(), Error> {
        if self.stage_every == 0 {
            return Ok(());
        }
        if !self.stage_every.is_power_of_two() || self.stage_every > 1 << 20 {
            return Err(Error::Usage(format!(
                "--stage-trace {}: a power of two from 1 to 1048576",
                self.stage_every
            )));
        }
        if (c.role == Role::Local && !c.is_round_trip()) || (c.role == Role::Pong && self.mode == Mode::SpyUdp) {
            return Err(Error::Usage(
                "--stage-trace: ping, pong and --role local --round-trip (an end that offers and reads)".into(),
            ));
        }
        Ok(())
    }

    /// `--mode spy-udp`'s own refusals: reliable alone, `--role local`
    /// measuring and `--role pong` its sink (both channels).
    fn check_spy_udp(&self, c: &Common) -> Result<(), Error> {
        if self.mode != Mode::SpyUdp {
            return Ok(());
        }
        if !self.reliable {
            return Err(Error::Usage(
                "--mode spy-udp is reliable (Aeron's default): --reliable false is refused".into(),
            ));
        }
        if c.role == Role::Ping {
            return Err(Error::Usage(
                "--mode spy-udp measures with --role local; --role pong is its sink".into(),
            ));
        }
        if c.role == Role::Pong && self.reply.is_none() {
            return Err(Error::Usage(
                "the spy-udp sink needs --reply HOST:PORT (the echoes' channel) too".into(),
            ));
        }
        Ok(())
    }

    /// `--driver invoker`'s own refusals: one measuring thread a process
    /// (`--role ping` or `pong`; no spy-udp sink), and no driver thread.
    fn check_invoker(&self, c: &Common) -> Result<(), Error> {
        if self.driver != Driver::Invoker {
            return Ok(());
        }
        if c.role == Role::Local || self.mode != Mode::Udp {
            return Err(Error::Usage(format!(
                "--driver invoker runs the driver on the measuring thread, one a process: --role ping or pong \
                 over --mode udp, not --role {} --mode {}",
                c.role.name(),
                self.mode_name()
            )));
        }
        if self.driver_busy_threads != 0 {
            return Err(Error::Usage(format!(
                "--driver-busy-threads {}: an invoked driver has no thread of its own (0)",
                self.driver_busy_threads
            )));
        }
        Ok(())
    }

    const fn mode_name(&self) -> &'static str {
        match self.mode {
            Mode::Ipc => "ipc",
            Mode::Spy => "spy",
            Mode::SpyUdp => "spy-udp",
            Mode::Udp => "udp",
        }
    }

    /// Every publication's parameters.
    fn pub_params(&self) -> String {
        format!("term-length={}|mtu={}|sparse=false", self.term_length, self.mtu)
    }

    /// `endpoint=<e>` and, when given, `|interface=<--interface>`: a UDP
    /// publication's address (and its spy's: the same canonical form).
    fn udp_address(&self, endpoint: Option<&str>) -> String {
        let mut uri = format!("endpoint={}", endpoint.unwrap_or_default());
        if let Some(i) = &self.interface {
            uri.push_str("|interface=");
            uri.push_str(i);
        }
        uri
    }

    /// Stream id `id`'s publication: on `endpoint` (UDP), or the mode's
    /// channel. A spy's publication is tagged with its stream id: a
    /// manual-MDC channel with no endpoint or control gets a unique
    /// canonical form, so a spy finds it by its tag alone. A spy-udp
    /// publication sends to `endpoint` and is connected only by a receiver
    /// there (`ssc=false`).
    fn pub_uri(&self, endpoint: Option<&str>, id: i32) -> String {
        let p = self.pub_params();
        match (self.mode, endpoint) {
            (Mode::Ipc, _) => format!("aeron:ipc?{p}"),
            (Mode::Spy, _) => format!("aeron:udp?control-mode=manual|tags={id}|ssc=true|{p}"),
            (Mode::SpyUdp, e) => format!("aeron:udp?{}|ssc=false|{p}", self.udp_address(e)),
            (Mode::Udp, e) => format!("aeron:udp?{}|{p}", self.udp_address(e)),
        }
    }

    /// Stream id `id`'s subscription: on `endpoint` (UDP), or the mode's
    /// channel (a spy's by its publication's tag; a spy-udp spy by its
    /// publication's address).
    fn sub_uri(&self, endpoint: Option<&str>, id: i32) -> String {
        match (self.mode, endpoint) {
            (Mode::Ipc, _) => "aeron:ipc".to_owned(),
            (Mode::Spy, _) => format!("aeron-spy:aeron:udp?control-mode=manual|tags={id}"),
            (Mode::SpyUdp, e) => format!("aeron-spy:aeron:udp?{}", self.udp_address(e)),
            (Mode::Udp, e) => {
                let mut uri = format!("aeron:udp?endpoint={}", e.unwrap_or_default());
                if !self.reliable {
                    uri.push_str("|reliable=false|group=false|tether=false");
                }
                uri
            }
        }
    }

    /// Stream `i`'s ping channel (publication, subscription) and stream id.
    fn ping_channel(&self, i: u32) -> Result<(String, String, i32), Error> {
        let e = self.endpoint.as_ref().map(|e| e.of("endpoint", i)).transpose()?;
        let id = self.stream(i, 0);
        Ok((self.pub_uri(e.as_deref(), id), self.sub_uri(e.as_deref(), id), id))
    }

    /// Stream `i`'s echo channel (publication, subscription) and stream id.
    fn echo_channel(&self, i: u32) -> Result<(String, String, i32), Error> {
        let e = self.reply.as_ref().map(|e| e.of("reply", i)).transpose()?;
        let id = self.stream(i, ECHO_OFFSET);
        Ok((self.pub_uri(e.as_deref(), id), self.sub_uri(e.as_deref(), id), id))
    }

    fn stream(&self, i: u32, offset: i32) -> i32 {
        // checked in parse: base + offset + streams fits an i32
        self.stream_id + offset + i32::try_from(i).unwrap_or(0)
    }

    /// The spy-udp sink's subscription to stream `i` of `channel`
    /// (`endpoint` or `reply`): a plain UDP subscription on its address.
    fn sink_channel(&self, flag: &str, i: u32) -> Result<(String, i32), Error> {
        let (e, offset) = match flag {
            "endpoint" => (self.endpoint.as_ref(), 0),
            _ => (self.reply.as_ref(), ECHO_OFFSET),
        };
        let e = e
            .ok_or_else(|| Error::Usage(format!("the sink needs --{flag}")))?
            .of(flag, i)?;
        Ok((format!("aeron:udp?endpoint={e}"), self.stream(i, offset)))
    }

    const fn wire_bytes(&self) -> u64 {
        let frame = MESSAGE_LEN as u64 + DATA_HEADER;
        match self.mode {
            Mode::Ipc | Mode::Spy => frame,
            Mode::SpyUdp | Mode::Udp => frame + UDP_IP_ETH,
        }
    }
}

/// A client of the driver at `dir`, its memory mapped and touched up front;
/// with `invoker`, its conductor run by the caller ([`Client::duty`]), not a
/// thread of its own.
fn client(dir: &str, invoker: bool) -> Result<Aeron, Error> {
    let dir = CString::new(dir).map_err(|_| Error::Usage("--aeron-dir holds a NUL byte".into()))?;
    let ctx = AeronContext::new().map_err(aeron_err("the client's context"))?;
    ctx.set_dir(&dir).map_err(aeron_err("the client's directory"))?;
    ctx.set_pre_touch_mapped_memory(true)
        .map_err(aeron_err("pre-touching the client's mappings"))?;
    if invoker {
        ctx.set_use_conductor_agent_invoker(true)
            .map_err(aeron_err("the client's conductor invoker"))?;
    }
    let aeron = Aeron::new(&ctx).map_err(aeron_err("the client"))?;
    aeron.start().map_err(aeron_err("starting the client"))?;
    Ok(aeron)
}

/// How long an add (a publication, a subscription) may take.
const ADD_TIMEOUT: Duration = Duration::from_secs(5);

/// A measuring thread's Aeron: its client and, under `--driver invoker`,
/// the media driver it runs itself.
struct Client {
    // Dropped first: the client closes before its driver does.
    aeron: Aeron,
    invoked: Option<EnvMediaDriver>,
    /// Duty cycles run (an invoked driver's), and those that found work.
    duty_calls: Cell<u64>,
    duty_busy: Cell<u64>,
}

impl Client {
    /// A client of the driver at `dir`, its conductor on a thread of its own.
    fn of(dir: &str) -> Result<Self, Error> {
        Ok(Self {
            aeron: client(dir, false)?,
            invoked: None,
            duty_calls: Cell::new(0),
            duty_busy: Cell::new(0),
        })
    }

    /// `--driver invoker`: a media driver in Aeron's INVOKER mode at `dir`,
    /// configured from the environment as the bundled `media_driver` is, its
    /// conductor CPU `cpu` (Aeron pins this thread there as it starts the
    /// driver), never idling; then its client, whose conductor this thread
    /// runs too. Both run on this thread alone ([`Self::duty`]).
    fn invoker(dir: &str, cpu: Option<usize>) -> Result<Self, Error> {
        let ctx = AeronDriverContext::new().map_err(aeron_err("the driver's context"))?;
        let cdir = CString::new(dir).map_err(|_| Error::Usage("--aeron-dir holds a NUL byte".into()))?;
        ctx.set_dir(&cdir).map_err(aeron_err("the driver's directory"))?;
        ctx.set_dir_delete_on_start(true)
            .map_err(aeron_err("the driver's directory"))?;
        ctx.set_threading_mode(aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER)
            .map_err(aeron_err("the driver's threading mode"))?;
        let cpu = cpu
            .map_or(Ok(-1), i32::try_from)
            .map_err(|_| Error::Usage("--cpus: a CPU past i32".into()))?;
        ctx.set_conductor_cpu_affinity(cpu)
            .map_err(aeron_err("the driver's conductor CPU"))?;
        ctx.set_shared_idle_strategy_kind(AeronIdleStrategyKind::NoOp)
            .map_err(aeron_err("the driver's idle strategy"))?;
        let driver = ConfiguredDriver::from_env(ctx)
            .and_then(ConfiguredDriver::start)
            .map_err(|e| Error::Transport(format!("the invoked driver: {e}")))?;
        let aeron = client(driver.dir(), true)?;
        Ok(Self {
            aeron,
            invoked: Some(driver),
            duty_calls: Cell::new(0),
            duty_busy: Cell::new(0),
        })
    }

    /// One duty cycle of this thread's client conductor and invoked driver
    /// (every agent of it): the work they found. Nothing without an invoked
    /// driver.
    #[inline]
    fn duty(&self) -> Result<i32, Error> {
        let Some(driver) = &self.invoked else { return Ok(0) };
        let client = self.aeron.main_do_work().map_err(aeron_err("the client's conductor"))?;
        let work = client
            + driver
                .do_work()
                .map_err(|e| Error::Transport(format!("the invoked driver: {e}")))?;
        self.count(work);
        Ok(work)
    }

    /// Counts a duty cycle and whether it found work (an integer add each:
    /// what the invoking thread's share of the driver's work is read from).
    #[inline]
    fn count(&self, work: i32) {
        self.duty_calls.set(self.duty_calls.get() + 1);
        if work > 0 {
            self.duty_busy.set(self.duty_busy.get() + 1);
        }
    }

    /// One duty cycle of the invoked driver alone (after an offer: its
    /// sender sends the frame at once). Nothing without one.
    #[inline]
    fn drive(&self) -> Result<(), Error> {
        if let Some(driver) = &self.invoked {
            let work = driver
                .do_work()
                .map_err(|e| Error::Transport(format!("the invoked driver: {e}")))?;
            self.count(work);
        }
        Ok(())
    }

    /// Lets `period` pass outside the measured loop: asleep, or with an
    /// invoked driver running the duty (resting a millisecond when it finds
    /// nothing).
    fn rest(&self, period: Duration) -> Result<(), Error> {
        if self.invoked.is_none() {
            std::thread::sleep(period);
            return Ok(());
        }
        let until = Instant::now() + period;
        while Instant::now() < until {
            if self.duty()? == 0 {
                std::thread::sleep(Duration::from_millis(1).min(period));
            }
        }
        Ok(())
    }

    /// An add `poll` completes: within [`ADD_TIMEOUT`], the duty run while
    /// it waits.
    fn resolve<T>(&self, what: &str, mut poll: impl FnMut() -> Result<Option<T>, AeronCError>) -> Result<T, Error> {
        let deadline = Instant::now() + ADD_TIMEOUT;
        loop {
            if let Some(added) = poll().map_err(|e| Error::Transport(format!("adding the {what}: {e:?}")))? {
                return Ok(added);
            }
            if Instant::now() > deadline {
                return Err(Error::Timeout(format!(
                    "adding the {what}: not done within {} s",
                    ADD_TIMEOUT.as_secs()
                )));
            }
            self.rest(Duration::from_millis(1))?;
        }
    }
}

/// Retries `add` while the driver says a previous run's endpoint is still
/// closing ("please retry"), for 10 s.
fn retry_closing<T>(client: &Client, mut add: impl FnMut() -> Result<T, Error>) -> Result<T, Error> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match add() {
            Ok(added) => return Ok(added),
            Err(e) if e.to_string().contains("please retry") && Instant::now() < deadline => {
                client.rest(Duration::from_millis(20))?;
            }
            Err(e) => return Err(e),
        }
    }
}

fn publication(client: &Client, uri: &str, stream: i32) -> Result<AeronExclusivePublication, Error> {
    let uri = CString::new(uri).map_err(|_| Error::Usage("a channel with a NUL byte".into()))?;
    retry_closing(client, || {
        let added = |e: AeronCError| Error::Transport(format!("adding the publication: {e:?}"));
        let pending = client
            .aeron
            .async_add_exclusive_publication(&uri, stream)
            .map_err(added)?;
        if client.invoked.is_none() {
            return pending.poll_blocking(ADD_TIMEOUT).map_err(added);
        }
        client.resolve("publication", || pending.poll())
    })
}

fn subscription(client: &Client, uri: &str, stream: i32) -> Result<AeronSubscription, Error> {
    let uri = CString::new(uri).map_err(|_| Error::Usage("a channel with a NUL byte".into()))?;
    retry_closing(client, || {
        let added = |e: AeronCError| Error::Transport(format!("adding the subscription: {e:?}"));
        let pending = client
            .aeron
            .async_add_subscription(&uri, stream, Handlers::NONE, Handlers::NONE)
            .map_err(added)?;
        if client.invoked.is_none() {
            return pending.poll_blocking(ADD_TIMEOUT).map_err(added);
        }
        client.resolve("subscription", || pending.poll())
    })
}

/// The publications, one per stream.
struct AeronTx {
    pubs: Vec<AeronExclusivePublication>,
}

impl Tx for AeronTx {
    #[inline]
    fn try_send(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> Result<Sent, Error> {
        let p = self
            .pubs
            .get(stream as usize)
            .ok_or_else(|| Error::Transport(format!("no publication for stream {stream}")))?;
        match p.offer(msg) {
            Ok(_) => Ok(Sent::Sent),
            Err(e) if e.is_retryable() => Ok(Sent::Busy),
            Err(e) => Err(Error::Transport(format!("offer on stream {stream}: {e:?}"))),
        }
    }
}

/// The subscriptions' images, one per stream, read directly.
struct AeronRx {
    images: Vec<AeronImage>,
    // dropped after the images
    _subs: Vec<AeronSubscription>,
}

impl Rx for AeronRx {
    #[inline]
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let mut read = 0;
        for image in &self.images {
            let n = image
                .poll_fn(|payload: &[u8], _header| on(payload), POLL_LIMIT)
                .map_err(aeron_err("polling an image"))?;
            read += usize::try_from(n).unwrap_or(0);
        }
        Ok(read)
    }

    fn closed(&self) -> bool {
        self.images.iter().all(AeronImage::is_closed)
    }
}

/// Under `--driver invoker`, a transport end on the thread that runs the
/// driver: each poll first runs one duty cycle (the client's conductor and
/// every agent of the driver: what the receiver took off the wire lands in
/// the image the poll then reads), and each send attempt is followed by one
/// of the driver's (its sender sends the offered frame at once, and a
/// back-pressured echo, retried inside a poll, lets the driver go on).
struct Invoked<'a, T> {
    end: &'a mut T,
    client: &'a Client,
}

impl Tx for Invoked<'_, AeronTx> {
    #[inline]
    fn try_send(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> Result<Sent, Error> {
        let sent = self.end.try_send(stream, msg)?;
        self.client.drive()?;
        Ok(sent)
    }
}

impl Rx for Invoked<'_, AeronRx> {
    #[inline]
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        self.client.duty()?;
        self.end.poll(on)
    }

    fn closed(&self) -> bool {
        self.end.closed()
    }
}

/// One application stamp of the stage trace.
#[derive(Debug, Clone, Copy, Default)]
struct AppStamp {
    stage: u8,
    tsc: u64,
    stream_id: i32,
    seq: u64,
    value: u64,
}

const APP_OFFER: u8 = 1;
const APP_OFFERED: u8 = 2;
const APP_READ: u8 = 3;
/// The most stamps one log holds (32 MiB at 32 bytes): beyond, dropped.
const STAGE_LOG_MAX: usize = 1 << 20;

/// A measuring thread's stage log, one end's (its sends or its reads):
/// preallocated and written through, so recording never allocates.
#[derive(Debug)]
struct StageLog {
    mask: u64,
    /// The Aeron stream id of stream index 0 on this end.
    base: i32,
    stamps: Vec<AppStamp>,
    len: usize,
    dropped: u64,
}

impl StageLog {
    /// A log for every `every`th message of `c`'s schedule, its stream ids
    /// from `base`, with room for `per_message` stamps of each.
    fn new(c: &Common, every: u64, base: i32, per_message: u64) -> Self {
        let messages = c
            .rate
            .saturating_mul(c.warmup_s.saturating_add(c.duration_s).saturating_add(10))
            / every.max(1);
        let cap = usize::try_from(messages.saturating_add(1024).saturating_mul(per_message))
            .unwrap_or(STAGE_LOG_MAX)
            .min(STAGE_LOG_MAX);
        Self {
            mask: every.max(1) - 1,
            base,
            // written through now: no page faults while measuring
            stamps: vec![AppStamp::default(); cap],
            len: 0,
            dropped: 0,
        }
    }

    #[inline]
    const fn sampled(&self, seq: u64) -> bool {
        seq & self.mask == 0
    }

    #[inline]
    fn push(&mut self, stamp: AppStamp) {
        if let Some(slot) = self.stamps.get_mut(self.len) {
            *slot = stamp;
            self.len += 1;
        } else {
            self.dropped += 1;
        }
    }

    fn stream_id(&self, msg: &[u8]) -> i32 {
        let index = msg
            .get(24..28)
            .and_then(|b| b.try_into().ok())
            .map_or(0, u32::from_le_bytes);
        self.base.saturating_add(i32::try_from(index).unwrap_or(0))
    }
}

/// A message's seq (payload bytes 0..8), if it holds one.
#[inline]
fn msg_u64(msg: &[u8], at: usize) -> Option<u64> {
    msg.get(at..at + 8)
        .and_then(|b| b.try_into().ok())
        .map(u64::from_le_bytes)
}

/// A sending end that stamps its sampled messages' offers.
struct StagedTx<'a, T> {
    end: &'a mut T,
    clock: &'a TscClock,
    log: &'a mut StageLog,
    /// A sampled message whose offer was refused (busy): its stream, seq and
    /// first attempt's counter, kept for its `app-offer`.
    pending: Option<(u32, u64, u64)>,
}

impl<T: Tx> Tx for StagedTx<'_, T> {
    #[inline]
    fn try_send(&mut self, stream: u32, msg: &[u8; MESSAGE_LEN]) -> Result<Sent, Error> {
        let seq = msg_u64(msg, 0).unwrap_or(1);
        if !self.log.sampled(seq) {
            return self.end.try_send(stream, msg);
        }
        let t0 = self.clock.raw();
        let sent = self.end.try_send(stream, msg)?;
        match sent {
            Sent::Sent => {
                let t1 = self.clock.raw();
                let first = match self.pending.take() {
                    Some((s, q, t)) if s == stream && q == seq => t,
                    _ => t0,
                };
                let stream_id = self.log.stream_id(msg);
                for (stage, tsc) in [(APP_OFFER, first), (APP_OFFERED, t1)] {
                    self.log.push(AppStamp {
                        stage,
                        tsc,
                        stream_id,
                        seq,
                        value: 0,
                    });
                }
            }
            Sent::Busy => {
                if !matches!(self.pending, Some((s, q, _)) if s == stream && q == seq) {
                    self.pending = Some((stream, seq, t0));
                }
            }
        }
        Ok(sent)
    }
}

/// A receiving end that stamps its sampled messages as its poll hands them
/// over, before the measuring code sees them.
struct StagedRx<'a, R> {
    end: &'a mut R,
    clock: &'a TscClock,
    log: &'a mut StageLog,
}

impl<R: Rx> Rx for StagedRx<'_, R> {
    #[inline]
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let (clock, log) = (self.clock, &mut *self.log);
        self.end.poll(&mut |p: &[u8]| {
            if let Some(seq) = msg_u64(p, 0)
                && log.sampled(seq)
            {
                let tsc = clock.raw();
                let stream_id = log.stream_id(p);
                log.push(AppStamp {
                    stage: APP_READ,
                    tsc,
                    stream_id,
                    seq,
                    value: msg_u64(p, 8).unwrap_or(0),
                });
            }
            on(p);
        })
    }

    fn closed(&self) -> bool {
        self.end.closed()
    }
}

/// A measuring thread's two stage logs: its offers', its reads'.
#[derive(Debug)]
struct Stages {
    tx: StageLog,
    rx: StageLog,
    /// The schedule's start on a ping (raw counter), 0 elsewhere.
    start_tsc: u64,
}

impl Stages {
    /// `--stage-trace`'s logs for an end sending on stream ids from
    /// `tx_base` and reading from `rx_base`; `None` when off.
    fn new(c: &Common, a: &Aeronish, tx_base: i32, rx_base: i32) -> Option<Self> {
        (a.stage_every > 0).then(|| Self {
            tx: StageLog::new(c, a.stage_every, tx_base, 2),
            rx: StageLog::new(c, a.stage_every, rx_base, 1),
            start_tsc: 0,
        })
    }
}

/// `<out>/stages-app.csv` from every measuring thread's logs (thread k's
/// offers ring 2k, its reads 2k + 1).
fn write_stages(c: &Common, a: &Aeronish, clock: &TscClock, all: &[Stages]) -> Result<(), Error> {
    use std::fmt::Write as _;
    let tpn = clock.ticks_per_ns();
    let start_tsc = all.iter().map(|s| s.start_tsc).find(|&t| t != 0).unwrap_or(0);
    let measure_from = if start_tsc == 0 {
        0
    } else {
        start_tsc.saturating_add(clock.ns_to_ticks(c.warmup_s.saturating_mul(1_000_000_000)))
    };
    let dropped: u64 = all.iter().map(|s| s.tx.dropped + s.rx.dropped).sum();
    let stamps: usize = all.iter().map(|s| s.tx.len + s.rx.len).sum();
    let mut text = String::with_capacity(stamps * 48 + 512);
    let _ = writeln!(text, "# ergon aeron_lat stage trace v1");
    let _ = writeln!(
        text,
        "# role={} every={} streams={} rate={} warmup_s={} duration_s={} ticks_per_ns={tpn:.9} start_tsc={start_tsc} \
         measure_from_tsc={measure_from} stream_id={} echo_offset={ECHO_OFFSET} dropped={dropped}",
        c.role.name(),
        a.stage_every,
        c.streams,
        c.rate,
        c.warmup_s,
        c.duration_s,
        a.stream_id,
    );
    let _ = writeln!(text, "stage,tsc,stream_id,seq,term_offset,value,ring");
    for (k, st) in all.iter().enumerate() {
        for (ring, log) in [(2 * k, &st.tx), (2 * k + 1, &st.rx)] {
            for x in log.stamps.iter().take(log.len) {
                let name = match x.stage {
                    APP_OFFER => "app-offer",
                    APP_OFFERED => "app-offered",
                    _ => "app-read",
                };
                let _ = writeln!(text, "{name},{},{},{},0,{},{ring}", x.tsc, x.stream_id, x.seq, x.value);
            }
        }
    }
    std::fs::create_dir_all(&c.out).map_err(|source| Error::Io {
        path: c.out.clone(),
        source,
    })?;
    let path = c.out.join("stages-app.csv");
    std::fs::write(&path, text).map_err(|source| Error::Io { path, source })
}

/// The round trip's ping through `tx` and `rx`, each stamped into
/// `stages` when on.
fn ping_staged<T: Tx, R: Rx>(
    clock: &TscClock,
    c: &Common,
    t: u64,
    tx: &mut T,
    rx: &mut R,
    rec: &mut Recorder,
    stages: Option<&mut Stages>,
) -> Result<SendStats, Error> {
    match stages {
        None => point::ping(clock, c, t, tx, rx, rec),
        Some(st) => {
            st.start_tsc = t;
            let (mut tx, mut rx) = (
                StagedTx {
                    end: tx,
                    clock,
                    log: &mut st.tx,
                    pending: None,
                },
                StagedRx {
                    end: rx,
                    clock,
                    log: &mut st.rx,
                },
            );
            point::ping(clock, c, t, &mut tx, &mut rx, rec)
        }
    }
}

/// The echo through `rx` and `tx`, each stamped into `stages` when on.
fn echo_staged<R: Rx, T: Tx>(
    rx: &mut R,
    tx: &mut T,
    keep_going: &mut impl FnMut() -> bool,
    stages: Option<(&mut Stages, &TscClock)>,
) -> Result<u64, Error> {
    match stages {
        None => point::echo(rx, tx, keep_going),
        Some((st, clock)) => {
            let (mut rx, mut tx) = (
                StagedRx {
                    end: rx,
                    clock,
                    log: &mut st.rx,
                },
                StagedTx {
                    end: tx,
                    clock,
                    log: &mut st.tx,
                    pending: None,
                },
            );
            point::echo(&mut rx, &mut tx, keep_going)
        }
    }
}

/// Publications on `channel(i)` for every stream.
fn publications(
    client: &Client,
    streams: u32,
    channel: impl Fn(u32) -> Result<(String, String, i32), Error>,
) -> Result<AeronTx, Error> {
    let pubs = (0..streams)
        .map(|i| {
            let (uri, _, id) = channel(i)?;
            let p = publication(client, &uri, id)?;
            let max = p.max_payload_length().map_err(aeron_err("the max payload"))?;
            if max < MESSAGE_LEN {
                return Err(Error::Usage(format!("{uri}: max payload {max} under {MESSAGE_LEN}")));
            }
            Ok(p)
        })
        .collect::<Result<_, _>>()?;
    Ok(AeronTx { pubs })
}

/// Subscriptions on `channel(i)` for every stream; their images come later
/// ([`ready`]).
fn subscriptions(
    client: &Client,
    streams: u32,
    channel: impl Fn(u32) -> Result<(String, String, i32), Error>,
) -> Result<Vec<AeronSubscription>, Error> {
    (0..streams)
        .map(|i| {
            let (_, uri, id) = channel(i)?;
            subscription(client, &uri, id)
        })
        .collect()
}

/// Waits until every publication is connected and every subscription has
/// an image, then takes the images.
fn ready(
    client: &Client,
    tx: Option<&AeronTx>,
    subs: Vec<AeronSubscription>,
    timeout: Duration,
) -> Result<AeronRx, Error> {
    let deadline = Instant::now() + timeout;
    loop {
        let pubs_up = tx.is_none_or(|t| t.pubs.iter().all(AeronExclusivePublication::is_connected));
        let images: Option<Vec<AeronImage>> = subs.iter().map(|s| s.image_at_index(0)).collect();
        if let (true, Some(images)) = (pubs_up, images) {
            return Ok(AeronRx { images, _subs: subs });
        }
        if Instant::now() > deadline {
            return Err(Error::Timeout(format!(
                "the other end: publications connected {pubs_up}, images {}/{} within {} s",
                subs.iter().filter(|s| s.image_at_index(0).is_some()).count(),
                subs.len(),
                timeout.as_secs()
            )));
        }
        client.rest(Duration::from_millis(1))?;
    }
}

/// The driver's counters now: the loss counters and the traffic counters,
/// each summed by key, and whether the driver is md-dpdk's.
struct Counters {
    loss: BTreeMap<&'static str, i64>,
    wire: BTreeMap<&'static str, i64>,
    dpdk: bool,
}

fn counters_now(aeron: &Aeron) -> Counters {
    let mut c = Counters {
        loss: LOSS_COUNTERS.iter().map(|(_, k)| (*k, 0)).collect(),
        wire: WIRE_COUNTERS.iter().map(|(_, k)| (*k, 0)).collect(),
        dpdk: false,
    };
    aeron
        .counters_reader()
        .foreach_counter_fn(|value, _id, _type, _key, label| {
            if label.starts_with("rusteron-dpdk") {
                c.dpdk = true;
            }
            for (list, totals) in [(&LOSS_COUNTERS[..], &mut c.loss), (&WIRE_COUNTERS[..], &mut c.wire)] {
                if let Some((_, k)) = list.iter().find(|(prefix, _)| label.starts_with(prefix))
                    && let Some(t) = totals.get_mut(k)
                {
                    *t += value;
                }
            }
        });
    c
}

/// The embedded driver: started on a thread of its own, which runs its
/// first agent until stopped (its other agents on threads of their own, as
/// its threading mode has them).
struct Embedded {
    stop: std::sync::Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// The threading mode it started with (`AERON_THREADING_MODE`).
    threading: &'static str,
}

impl Embedded {
    fn start(dir: &str) -> Result<Self, Error> {
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel::<Result<&'static str, Error>>();
        let (dir, flag) = (dir.to_owned(), stop.clone());
        let thread = std::thread::Builder::new()
            .name("aeron-driver".into())
            .spawn(move || {
                let started = (|| {
                    let ctx = AeronDriverContext::new().map_err(aeron_err("the driver's context"))?;
                    let cdir = CString::new(dir).map_err(|_| Error::Usage("--aeron-dir holds a NUL byte".into()))?;
                    ctx.set_dir(&cdir).map_err(aeron_err("the driver's directory"))?;
                    ctx.set_dir_delete_on_start(true)
                        .map_err(aeron_err("the driver's directory"))?;
                    let driver = ConfiguredDriver::from_env(ctx)
                        .and_then(ConfiguredDriver::start)
                        .map_err(|e| Error::Transport(format!("the embedded driver: {e}")))?;
                    Ok(driver)
                })();
                let driver = match started {
                    Ok(d) => {
                        let _ = tx.send(Ok(threading_mode_name(d.threading_mode())));
                        d
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e));
                        return;
                    }
                };
                while !flag.load(Ordering::Acquire) {
                    match driver.do_work() {
                        Ok(work) => driver.idle(work),
                        Err(e) => {
                            eprintln!("aeron_lat: the embedded driver: {e}");
                            return;
                        }
                    }
                }
            })
            .map_err(|e| Error::Transport(format!("the driver's thread: {e}")))?;
        let threading = rx
            .recv()
            .map_err(|_| Error::Transport("the embedded driver's thread ended".into()))??;
        Ok(Self {
            stop,
            thread: Some(thread),
            threading,
        })
    }
}

impl Drop for Embedded {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// What a measuring end hands back.
struct Ends {
    rec: Recorder,
    send: SendStats,
    allocs: u64,
    start_realtime_ns: u128,
    /// The echoes the echo thread sent (local round trip).
    echoes: Option<u64>,
    /// `--stage-trace`'s logs: the ping's, and a local round trip's echo's.
    stages: Vec<Stages>,
}

fn joined<T>(r: std::thread::Result<Result<T, Error>>, who: &str) -> Result<T, Error> {
    r.map_err(|_| Error::Transport(format!("the {who} thread panicked")))?
}

fn local_oneway(c: &Common, a: &Aeronish, clock: &TscClock) -> Result<Ends, Error> {
    let (barrier, start, start_rt) = (Barrier::new(2), AtomicU64::new(0), AtomicU64::new(0));
    let progress = Progress::default();
    let (barrier, start, start_rt, progress) = (&barrier, &start, &start_rt, &progress);
    std::thread::scope(|s| {
        let sender = s.spawn(move || {
            let setup = (|| -> Result<_, Error> {
                let aeron = Client::of(&a.dir)?;
                let tx = publications(&aeron, c.streams, |i| a.ping_channel(i))?;
                ready(&aeron, Some(&tx), Vec::new(), a.connect_timeout)?;
                point::pin(c.cpu(0))?;
                Ok((aeron, tx))
            })();
            barrier.wait();
            let (t, rt) = point::start_at(clock, START_DELAY_NS);
            start.store(t, Ordering::Release);
            start_rt.store(u64::try_from(rt).unwrap_or(u64::MAX), Ordering::Release);
            barrier.wait();
            let (_aeron, mut tx) = setup.inspect_err(|_| progress.fail())?;
            let a0 = thread_allocs();
            let send = point::send_oneway(clock, c, t, &mut tx, progress)?;
            Ok((send, thread_allocs() - a0))
        });
        let receiver = s.spawn(move || {
            let setup = (|| -> Result<_, Error> {
                let aeron = Client::of(&a.dir)?;
                let subs = subscriptions(&aeron, c.streams, |i| a.ping_channel(i))?;
                let rx = ready(&aeron, None, subs, a.connect_timeout)?;
                point::pin(c.cpu(1))?;
                Ok((aeron, rx))
            })();
            barrier.wait();
            barrier.wait();
            let (_aeron, mut rx) = setup.inspect_err(|_| progress.fail())?;
            let t = start.load(Ordering::Acquire);
            let mut rec = Recorder::new(clock, t, c).inspect_err(|_| progress.fail())?;
            let a0 = thread_allocs();
            point::recv_oneway(clock, c, t, &mut rx, &mut rec, progress)?;
            Ok((rec, thread_allocs() - a0))
        });
        let ((send, sa), (rec, ra)) =
            point::both(joined(sender.join(), "sender"), joined(receiver.join(), "receiver"))?;
        Ok(Ends {
            rec,
            send,
            allocs: sa + ra,
            start_realtime_ns: u128::from(start_rt.load(Ordering::Acquire)),
            echoes: None,
            stages: Vec::new(),
        })
    })
}

/// The ping end on this thread: its client (`invoked`'s, under `--driver
/// invoker`, else one of its own), publications and echo subscriptions,
/// then (after `barrier`, when given; and with `gate`, once the mixed
/// load's background is ready, which then starts with it) the schedule.
fn ping_end(
    c: &Common,
    a: &Aeronish,
    clock: &TscClock,
    cpu: Option<usize>,
    barrier: Option<&Barrier>,
    invoked: Option<&Client>,
    gate: Option<&mixed::Gate>,
) -> Result<Ends, Error> {
    let setup = (|| -> Result<_, Error> {
        let own = if invoked.is_some() {
            None
        } else {
            Some(Client::of(&a.dir)?)
        };
        let client = invoked
            .or(own.as_ref())
            .ok_or_else(|| Error::Transport("no client".into()))?;
        let tx = publications(client, c.streams, |i| a.ping_channel(i))?;
        let subs = subscriptions(client, c.streams, |i| a.echo_channel(i))?;
        let rx = ready(client, Some(&tx), subs, a.connect_timeout)?;
        point::pin(cpu)?;
        Ok((own, tx, rx))
    })();
    if let Some(b) = barrier {
        b.wait();
    }
    let (own, mut tx, mut rx) = setup?;
    let client = invoked
        .or(own.as_ref())
        .ok_or_else(|| Error::Transport("no client".into()))?;
    let mut stages = Stages::new(c, a, a.stream(0, 0), a.stream(0, ECHO_OFFSET));
    let (t, rt) = match gate {
        Some(g) => mixed::start(client, clock, g)?,
        None => point::start_at(clock, START_DELAY_NS),
    };
    let mut rec = Recorder::new(clock, t, c)?;
    let a0 = thread_allocs();
    let send = if client.invoked.is_some() {
        let (mut tx, mut rx) = (Invoked { end: &mut tx, client }, Invoked { end: &mut rx, client });
        ping_staged(clock, c, t, &mut tx, &mut rx, &mut rec, stages.as_mut())?
    } else {
        ping_staged(clock, c, t, &mut tx, &mut rx, &mut rec, stages.as_mut())?
    };
    let allocs = thread_allocs() - a0;
    if let Some(g) = gate {
        mixed::drive_until_done(client, g)?;
    }
    Ok(Ends {
        rec,
        send,
        allocs,
        start_realtime_ns: rt,
        echoes: None,
        stages: stages.into_iter().collect(),
    })
}

/// The echo end on this thread: its client (`invoked`'s, under `--driver
/// invoker`, else one of its own), ping subscriptions and echo
/// publications (then `added`), then (after `barrier`, when given) echoes
/// until the pings go or `keep_going` says stop; with `gate` (the mixed
/// load's), under `--driver invoker`, it then runs the driver until the
/// background is done. Returns its hot-path allocations, the echoes it sent
/// and, with `--stage-trace` (`clock` given), its stage logs.
#[allow(clippy::too_many_arguments)]
fn echo_end(
    c: &Common,
    a: &Aeronish,
    cpu: Option<usize>,
    barrier: Option<&Barrier>,
    invoked: Option<&Client>,
    added: &dyn Fn(&Client) -> Result<(), Error>,
    keep_going: &mut impl FnMut() -> bool,
    clock: Option<&TscClock>,
    gate: Option<&mixed::Gate>,
) -> Result<(u64, u64, Option<Stages>), Error> {
    let setup = (|| -> Result<_, Error> {
        let own = if invoked.is_some() {
            None
        } else {
            Some(Client::of(&a.dir)?)
        };
        let client = invoked
            .or(own.as_ref())
            .ok_or_else(|| Error::Transport("no client".into()))?;
        let subs = subscriptions(client, c.streams, |i| a.ping_channel(i))?;
        let tx = publications(client, c.streams, |i| a.echo_channel(i))?;
        added(client)?;
        let rx = ready(client, Some(&tx), subs, a.connect_timeout)?;
        point::pin(cpu)?;
        Ok((own, tx, rx))
    })();
    if let Some(b) = barrier {
        b.wait();
    }
    let (own, mut tx, mut rx) = setup?;
    let client = invoked
        .or(own.as_ref())
        .ok_or_else(|| Error::Transport("no client".into()))?;
    let mut stages = clock.and_then(|_| Stages::new(c, a, a.stream(0, ECHO_OFFSET), a.stream(0, 0)));
    let logs = stages.as_mut().zip(clock);
    let a0 = thread_allocs();
    let echoes = if client.invoked.is_some() {
        let (mut rx, mut tx) = (Invoked { end: &mut rx, client }, Invoked { end: &mut tx, client });
        echo_staged(&mut rx, &mut tx, keep_going, logs)?
    } else {
        echo_staged(&mut rx, &mut tx, keep_going, logs)?
    };
    let allocs = thread_allocs() - a0;
    if let Some(g) = gate {
        mixed::drive_until_done(client, g)?;
    }
    Ok((allocs, echoes, stages))
}

fn local_round_trip(c: &Common, a: &Aeronish, clock: &TscClock) -> Result<Ends, Error> {
    let (barrier, stop) = (Barrier::new(2), AtomicBool::new(false));
    let (barrier, stop) = (&barrier, &stop);
    std::thread::scope(|s| {
        let echo = s.spawn(move || {
            echo_end(
                c,
                a,
                c.cpu(1),
                Some(barrier),
                None,
                &|_| Ok(()),
                &mut || !stop.load(Ordering::Acquire),
                Some(clock),
                None,
            )
        });
        let ping = s.spawn(move || {
            let r = ping_end(c, a, clock, c.cpu(0), Some(barrier), None, None);
            stop.store(true, Ordering::Release);
            r
        });
        let mut ends = joined(ping.join(), "ping")?;
        let (allocs, echoes, stages) = joined(echo.join(), "echo")?;
        ends.allocs += allocs;
        ends.echoes = Some(echoes);
        ends.stages.extend(stages);
        Ok(ends)
    })
}

fn manifest(
    c: &Common,
    a: &Aeronish,
    transport: &str,
    threading: Option<&str>,
    mixed: bool,
) -> Result<RunManifest, Error> {
    let mut m = RunManifest::new(transport, &template(), a.wire_bytes());
    let (ping_publication, ping_subscription, ping_id) = a.ping_channel(0)?;
    m.setting("transport", transport)
        .setting("mode", a.mode_name())
        .setting("reliable", &a.reliable.to_string())
        .setting("driver", a.driver.name())
        .setting("aeron_dir", &a.dir)
        .setting("term_length", &a.term_length.to_string())
        .setting("mtu", &a.mtu.to_string())
        .setting("stream_id", &ping_id.to_string())
        .setting("echo_stream_offset", &ECHO_OFFSET.to_string())
        .setting("ping_pub_uri", &ping_publication)
        .setting("ping_sub_uri", &ping_subscription)
        .setting("driver_busy_threads", &a.driver_busy_threads.to_string())
        .setting("client_pre_touch", "true")
        .setting("idle", "busy-spin (the client's measuring threads)")
        .setting("thread_order", thread_order(c, a.mode, mixed));
    // the threading mode of a driver this process runs (an external one's is
    // its process's: the runner records it)
    if let Some(t) = threading {
        m.setting("driver_threading", t);
    }
    if c.is_round_trip() {
        let (echo_publication, echo_subscription, _) = a.echo_channel(0)?;
        m.setting("echo_pub_uri", &echo_publication)
            .setting("echo_sub_uri", &echo_subscription);
    }
    if let Some(e) = &a.endpoint {
        m.setting("endpoint", &format!("{}:{}", e.host, e.port));
    }
    if let Some(e) = &a.reply {
        m.setting("reply", &format!("{}:{}", e.host, e.port));
    }
    if let Some(i) = &a.interface {
        m.setting("interface", i);
    }
    Ok(m)
}

fn thread_order(c: &Common, mode: Mode, mixed: bool) -> &'static str {
    if mixed {
        return "hot,background";
    }
    match (c.role, c.round_trip) {
        (Role::Local, false) => "sender,receiver",
        (Role::Local, true) => "ping,echo",
        (Role::Ping, _) => "ping",
        _ if mode == Mode::SpyUdp => "sink",
        _ => "echo",
    }
}

const fn client_threads(c: &Common, mixed: bool) -> u32 {
    match c.role {
        Role::Local => 2,
        // the hot stream's thread and the background's
        _ if mixed => 2,
        _ => 1,
    }
}

#[allow(clippy::too_many_lines)]
fn run(c: &Common, a: &Aeronish, profile: Option<&mixed::Profile>) -> Result<Option<String>, Error> {
    match c.role {
        Role::OnewaySend | Role::OnewayRecv => {
            return Err(Error::Usage(format!(
                "--role {}: aeron_lat measures one way with --role local (both ends in this process, one counter)",
                c.role.name()
            )));
        }
        Role::Local | Role::Ping | Role::Pong => {}
    }
    let is_mixed = profile.is_some();
    c.check_cpus(client_threads(c, is_mixed) as usize, thread_order(c, a.mode, is_mixed))?;
    let embedded = if a.driver == Driver::Embedded {
        Some(Embedded::start(&a.dir)?)
    } else {
        None
    };
    // under --driver invoker the measuring thread's own client (this one,
    // the driver started on it), which reads the counters too
    let counters = if a.driver == Driver::Invoker {
        Client::invoker(&a.dir, c.cpu(0))?
    } else {
        Client::of(&a.dir)?
    };
    let invoked = (a.driver == Driver::Invoker).then_some(&counters);
    let threading = match a.driver {
        Driver::External => None,
        Driver::Embedded => embedded.as_ref().map(|e| e.threading),
        Driver::Invoker => Some("INVOKER"),
    };
    let before = Before {
        counters: counters_now(&counters.aeron),
        extra: mixed::counters_now(&counters.aeron),
    };
    // provisional: settled once the point's channels exist (close)
    let transport = transport_name(a.mode, a.wire == Some(Wire::Dpdk));
    let mut m = manifest(c, a, transport, threading, is_mixed)?;
    if let Some(p) = profile {
        return run_mixed(c, a, p, &counters, m, &before);
    }
    if c.role == Role::Pong {
        stop_on_sigterm();
        let allocs = if a.mode == Mode::SpyUdp {
            sink(c, a, &mut m)?
        } else {
            // the stage trace's counter scale (a 20 ms calibration), before
            // the echo starts
            let clock = (a.stage_every > 0).then(TscClock::new);
            let (allocs, _, stages) = echo_end(
                c,
                a,
                c.cpu(0),
                None,
                invoked,
                &|_| {
                    println!("ready");
                    Ok(())
                },
                &mut || !STOP.load(Ordering::Relaxed),
                clock.as_ref(),
                None,
            )?;
            if let (Some(st), Some(clock)) = (stages, &clock) {
                write_stages(c, a, clock, &[st])?;
                m.setting("stage_trace", &a.stage_every.to_string());
            }
            allocs
        };
        close(&mut m, a, &counters, &before, false)?;
        m.setting("hot_path_allocs", &allocs.to_string()).setting(
            "busy_threads",
            &(client_threads(c, false) + a.driver_busy_threads).to_string(),
        );
        point::write_manifest(c, m)?;
        return Ok(None);
    }
    let clock = TscClock::new();
    let ends = match c.role {
        Role::Ping => ping_end(c, a, &clock, c.cpu(0), None, invoked, None)?,
        _ if c.round_trip => local_round_trip(c, a, &clock)?,
        _ => local_oneway(c, a, &clock)?,
    };
    let end = clock.raw();
    let retrans = close(&mut m, a, &counters, &before, false)?;
    if let Some(e) = ends.echoes {
        m.setting("echoes_sent", &e.to_string());
    }
    // the stage trace: the ping's log (a local round trip's echo's merged
    // into the same file, its rings 2 and 3)
    if !ends.stages.is_empty() {
        write_stages(c, a, &clock, &ends.stages)?;
        m.setting("stage_trace", &a.stage_every.to_string());
    }
    let row = point::finish(
        c,
        &clock,
        ends.rec,
        ends.send,
        ends.start_realtime_ns,
        end,
        Report {
            channel: if a.mode == Mode::Ipc { "ipc" } else { "udp" },
            retrans: Some(retrans),
            wire_bytes: a.wire_bytes(),
            busy_threads: client_threads(c, false) + a.driver_busy_threads,
            hot_path_allocs: ends.allocs,
            manifest: m,
        },
    )?;
    Ok(Some(row))
}

/// A `--load mixed` point (mixed.rs): the hot stream on this thread as any
/// ping or pong runs it, the background on a thread of its own with a
/// client of its own; their counts, the driver's counters around them, and
/// (the ping) the row: the hot stream's latency, every stream's rate.
#[allow(clippy::too_many_lines)]
fn run_mixed(
    c: &Common,
    a: &Aeronish,
    profile: &mixed::Profile,
    counters: &Client,
    mut m: RunManifest,
    before: &Before,
) -> Result<Option<String>, Error> {
    let invoked = (a.driver == Driver::Invoker).then_some(counters);
    let hot = Common {
        streams: 1,
        rate: profile.hot_rate,
        ..c.clone()
    };
    profile.settings(&mut m);
    let gate = mixed::Gate::default();
    let busy = client_threads(c, true) + a.driver_busy_threads;
    // the counters after (under DPDK once the PMD's statistics refreshed)
    let after = |m: &mut RunManifest| -> Result<u64, Error> {
        m.setting("mixed.invoker.duty_calls", &counters.duty_calls.get().to_string())
            .setting("mixed.invoker.busy_calls", &counters.duty_busy.get().to_string());
        close(m, a, counters, before, true)
    };
    if c.role == Role::Pong {
        stop_on_sigterm();
        // the stage trace's counter scale (a 20 ms calibration), before the
        // echo starts: the hot stream's
        let clock = (a.stage_every > 0).then(TscClock::new);
        let (hot_end, bg) = std::thread::scope(|s| {
            let bg = s.spawn(|| mixed::pong(c, a, profile, &gate));
            let hot_end = echo_end(
                &hot,
                a,
                c.cpu(0),
                None,
                invoked,
                &|client| {
                    while !gate.ready()? {
                        client.rest(Duration::from_millis(1))?;
                    }
                    println!("ready");
                    Ok(())
                },
                &mut || !STOP.load(Ordering::Relaxed),
                clock.as_ref(),
                Some(&gate),
            );
            (hot_end, joined(bg.join(), "background"))
        });
        let ((allocs, echoes, stages), bg) = point::both(hot_end, bg)?;
        after(&mut m)?;
        if let (Some(st), Some(clock)) = (stages, &clock) {
            write_stages(&hot, a, clock, &[st])?;
            m.setting("stage_trace", &a.stage_every.to_string());
        }
        mixed::put_all(&mut m, &bg.settings);
        let bg_allocs: u64 = bg
            .settings
            .get("mixed.bg.allocs")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        m.setting("mixed.hot_echoes", &echoes.to_string())
            .setting("mixed.hot_allocs", &allocs.to_string())
            .setting("hot_path_allocs", &(allocs + bg_allocs).to_string())
            .setting("busy_threads", &busy.to_string());
        point::write_manifest(c, m)?;
        return Ok(None);
    }
    let clock = TscClock::new();
    let (hot_end, bg) = std::thread::scope(|s| {
        let bg = s.spawn(|| mixed::ping(c, a, profile, &clock, &gate));
        let hot_end = ping_end(&hot, a, &clock, c.cpu(0), None, invoked, Some(&gate));
        if hot_end.is_err() {
            gate.fail();
        }
        (hot_end, joined(bg.join(), "background"))
    });
    let (ends, bg) = point::both(hot_end, bg)?;
    let end = clock.raw();
    let retrans = after(&mut m)?;
    mixed::put_all(&mut m, &bg.settings);
    let hot_sent = ends.send.pacer.stats().sent;
    let hot_bytes = hot_sent * mixed::frame_bytes(MESSAGE_LEN, a.mtu as usize) as u64;
    // the stage trace: the hot stream's
    if !ends.stages.is_empty() {
        write_stages(&hot, a, &clock, &ends.stages)?;
        m.setting("stage_trace", &a.stage_every.to_string());
    }
    m.setting("mixed.hot_sent", &hot_sent.to_string())
        .setting("mixed.hot_frame_bytes", &hot_bytes.to_string())
        .setting("mixed.hot_allocs", &ends.allocs.to_string());
    let bg_allocs: u64 = bg
        .settings
        .get("mixed.bg.allocs")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let row = point::finish_shown(
        &hot,
        &clock,
        ends.rec,
        ends.send,
        ends.start_realtime_ns,
        end,
        Report {
            channel: "udp",
            retrans: Some(retrans),
            wire_bytes: a.wire_bytes(),
            busy_threads: busy,
            hot_path_allocs: ends.allocs + bg_allocs,
            manifest: m,
        },
        Some(&point::Shown {
            rate: c.rate,
            streams: c.streams,
            achieved_extra: bg.achieved,
            others: &bg.hist,
        }),
    )?;
    Ok(Some(row))
}

/// The driver's counters as a point began: the loss and traffic ones, and
/// its agents' duty cycles and control traffic (mixed.rs `counters_now`).
struct Before {
    counters: Counters,
    extra: BTreeMap<String, i64>,
}

/// The transport's name for `mode`, on DPDK or not.
const fn transport_name(mode: Mode, dpdk: bool) -> &'static str {
    match mode {
        Mode::Ipc => "aeron-ipc",
        Mode::Spy => "aeron-spy",
        Mode::SpyUdp if dpdk => "aeron-spy-dpdk",
        Mode::SpyUdp => "aeron-spy-udp",
        Mode::Udp if dpdk => "aeron-dpdk",
        Mode::Udp => "aeron-udp",
    }
}

/// The point's end on the driver's counters: each counter's growth into
/// the manifest (the loss and traffic ones; every agent's duty cycles, a
/// max as it is after and before; flow control and control traffic), and
/// the transport settled. The driver is DPDK's when its counters show
/// md-dpdk's (read after the point too: a driver registers them once its
/// channels open, so at the start of a point a fresh one has none). A wire
/// the runner gave (`--wire`) that the driver does not show is refused, so
/// no row is labelled with a wire it did not run on. `wait_nic`: under
/// DPDK, wait for the PMD's once-a-second statistics first. Returns the
/// retransmits' growth.
fn close(m: &mut RunManifest, a: &Aeronish, client: &Client, before: &Before, wait_nic: bool) -> Result<u64, Error> {
    let mut after = counters_now(&client.aeron);
    let seen = before.counters.dpdk || after.dpdk;
    if seen && wait_nic {
        std::thread::sleep(Duration::from_millis(1200));
        after = counters_now(&client.aeron);
    }
    mixed::record_counters(m, &before.extra, &mixed::counters_now(&client.aeron));
    let retrans = record_counters(m, &before.counters, &after);
    if matches!(a.mode, Mode::Udp | Mode::SpyUdp) {
        let seen_name = if seen { Wire::Dpdk } else { Wire::Kernel }.name();
        if let Some(w) = a.wire
            && w.name() != seen_name
        {
            return Err(Error::Transport(format!(
                "--wire {} but the driver shows {}: refused rather than labelled with a wire it did not run on",
                w.name(),
                if seen {
                    "md-dpdk's DPDK counters"
                } else {
                    "no DPDK counters"
                }
            )));
        }
        let name = transport_name(a.mode, seen);
        name.clone_into(&mut m.arm);
        m.setting("transport", name)
            .setting("wire", a.wire.map_or("unstated", Wire::name))
            .setting("wire_seen", seen_name);
    }
    Ok(retrans)
}

/// Each loss counter's growth into the manifest (`aeron.<key>`), and each
/// traffic counter's (`driver.<key>`); returns the retransmits'.
fn record_counters(m: &mut RunManifest, before: &Counters, after: &Counters) -> u64 {
    let mut retrans = 0;
    for (k, v) in &after.loss {
        let grew = v - before.loss.get(k).copied().unwrap_or(0);
        m.setting(&format!("aeron.{k}"), &grew.to_string());
        if *k == "retransmits-sent" {
            retrans = u64::try_from(grew).unwrap_or(0);
        }
    }
    for (k, v) in &after.wire {
        let grew = v - before.wire.get(k).copied().unwrap_or(0);
        m.setting(&format!("driver.{k}"), &grew.to_string());
    }
    retrans
}

/// One sink subscription's count: frames, the next sequence number due,
/// and what came out of order.
#[derive(Default)]
struct SinkStream {
    frames: u64,
    next: u64,
    gaps: u64,
    behind: u64,
    refused: u64,
    seen: bool,
}

impl SinkStream {
    fn on(&mut self, payload: &[u8]) {
        self.frames += 1;
        match ergon_latency_core::message::Message::decode(payload) {
            Ok(msg) if msg.seq == self.next => self.next += 1,
            Ok(msg) if msg.seq > self.next => {
                self.gaps += msg.seq - self.next;
                self.next = msg.seq + 1;
            }
            Ok(_) => self.behind += 1,
            Err(_) => self.refused += 1,
        }
    }
}

/// How long a stopped sink keeps reading while frames still come (the
/// publications draining), and at most.
const SINK_QUIET: Duration = Duration::from_millis(500);
const SINK_DRAIN: Duration = Duration::from_secs(4);

/// The spy-udp sink: subscribes to every stream's ping channel (`--endpoint`)
/// and echo channel (`--reply`), says `ready`, then reads every frame until
/// SIGTERM (and the quiet after it) or until every ping channel's image has
/// come and gone. Its counts go to the manifest (`sink.*`); returns its
/// allocations while reading.
fn sink(c: &Common, a: &Aeronish, m: &mut RunManifest) -> Result<u64, Error> {
    let aeron = Client::of(&a.dir)?;
    let mut subs = Vec::new();
    for flag in ["endpoint", "reply"] {
        for i in 0..c.streams {
            let (uri, id) = a.sink_channel(flag, i)?;
            subs.push((flag, subscription(&aeron, &uri, id)?, SinkStream::default()));
        }
    }
    let (uri, _) = a.sink_channel("endpoint", 0)?;
    m.setting("sink_sub_uri", &uri);
    println!("ready");
    point::pin(c.cpu(0))?;
    let a0 = thread_allocs();
    let (mut stopped, mut last_read, mut idle) = (None::<Instant>, Instant::now(), 0u32);
    loop {
        let mut read = 0;
        for (_, sub, st) in &mut subs {
            let n = sub
                .poll_fn(|payload: &[u8], _header| st.on(payload), POLL_LIMIT)
                .map_err(aeron_err("polling a sink subscription"))?;
            read += usize::try_from(n).unwrap_or(0);
        }
        if read > 0 {
            idle = 0;
            continue;
        }
        idle = idle.wrapping_add(1);
        if !idle.is_multiple_of(1024) {
            continue;
        }
        let now = Instant::now();
        if idle == 1024 {
            last_read = now;
        }
        // every ping channel's image came, and every image went
        let mut gone = true;
        for (flag, sub, st) in &mut subs {
            let images = sub.image_count().unwrap_or(0);
            st.seen |= images > 0;
            if images > 0 || (*flag == "endpoint" && !st.seen) {
                gone = false;
            }
        }
        if gone {
            break;
        }
        if stopped.is_none() && STOP.load(Ordering::Relaxed) {
            stopped = Some(now);
        }
        if let Some(t) = stopped
            && (now.duration_since(last_read) >= SINK_QUIET || now.duration_since(t) >= SINK_DRAIN)
        {
            break;
        }
    }
    let allocs = thread_allocs() - a0;
    let sum = |dir: &str, f: fn(&SinkStream) -> u64| -> u64 {
        subs.iter().filter(|(d, _, _)| *d == dir).map(|(_, _, s)| f(s)).sum()
    };
    let all = |f: fn(&SinkStream) -> u64| sum("endpoint", f) + sum("reply", f);
    m.setting("sink.ping_frames", &sum("endpoint", |s| s.frames).to_string())
        .setting("sink.echo_frames", &sum("reply", |s| s.frames).to_string())
        .setting("sink.gaps", &all(|s| s.gaps).to_string())
        .setting("sink.behind", &all(|s| s.behind).to_string())
        .setting("sink.refused", &all(|s| s.refused).to_string())
        .setting(
            "sink.images",
            &subs.iter().filter(|(_, _, s)| s.seen).count().to_string(),
        );
    drop(subs);
    drop(aeron);
    Ok(allocs)
}

fn main() -> ExitCode {
    if std::env::args().skip(1).any(|a| a == "--help") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let go = || -> Result<Option<String>, Error> {
        let mut f = Flags::parse(std::env::args().skip(1))?;
        let c = Common::parse(&mut f)?;
        let a = Aeronish::parse(&mut f, &c)?;
        let profile = mixed::parse(&mut f, &c, &a)?;
        f.finish()?;
        run(&c, &a, profile.as_ref())
    };
    match go() {
        Ok(row) => {
            if let Some(row) = row {
                println!("{row}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("aeron_lat: {e}");
            ExitCode::FAILURE
        }
    }
}
