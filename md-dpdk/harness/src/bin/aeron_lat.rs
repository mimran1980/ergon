//! The transport latency suite's Aeron arms: IPC, spy, spy on a live UDP
//! stream, kernel UDP and DPDK.
//!
//! One binary against a media driver: the external one at `--aeron-dir`
//! (the runner starts it `DEDICATED`, every agent busy-spinning; the DPDK
//! arm is this binary against a DPDK driver), or with `--driver embedded`
//! one this process starts on a thread of its own (local tests; configured
//! from the environment as the bundled `media_driver` is, through
//! `ergon_md_dpdk::driver::ConfiguredDriver`). Every measurement, the echo
//! and the files are `io_bench::transport::point`'s, taken by path, so the
//! Aeron arms measure exactly as the others do.
//!
//! ```text
//! aeron_lat --role <ping|pong|local> [--round-trip] --mode <ipc|spy|spy-udp|udp> --rate MSG_PER_S
//!           --label TEXT --out DIR --aeron-dir PATH [--driver external|embedded]
//!           [--reliable true|false] [--endpoint HOST:PORT] [--reply HOST:PORT]
//!           [--interface IP] [--term-length 1048576] [--mtu 1408] [--stream-id 1001]
//!           [--driver-busy-threads 3] [--connect-timeout-s 30]
//!           [--streams N] [--warmup-s 30] [--duration-s 30] [--size 64] [--cpus CSV]
//!           [--spike-ns 100000] [--stall-ns 10000]
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
//! affinity (the runner starts the binary on a housekeeping CPU).
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
#![deny(unsafe_code, missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

// io/benches formats it (rustfmt's defaults), not this crate (max_width 120).
#[path = "../../../../io/benches/src/transport/point.rs"]
#[rustfmt::skip]
mod point;

use ergon_latency_core::clock::TscClock;
use ergon_latency_core::manifest::RunManifest;
use ergon_latency_core::message::{MESSAGE_LEN, template};
use ergon_md_dpdk::driver::ConfiguredDriver;
use point::{
    Common, CountingAlloc, Error, Flags, POLL_LIMIT, Progress, Recorder, Report, Role, Rx, START_DELAY_NS, SendStats,
    Sent, Tx, thread_allocs,
};
use rusteron_client::{
    Aeron, AeronCError, AeronContext, AeronExclusivePublication, AeronImage, AeronSubscription, Handlers,
};
use rusteron_media_driver::AeronDriverContext;
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
          --aeron-dir PATH [--driver external|embedded] [--driver-busy-threads 3] [--reliable true|false]
          [--endpoint HOST:PORT] [--reply HOST:PORT] [--interface IP] [--term-length 1048576] [--mtu 1408]
          [--stream-id 1001] [--connect-timeout-s 30] [--streams 1] [--warmup-s 30] [--duration-s 30]
          [--size 64] [--cpus CSV] [--spike-ns 100000] [--stall-ns 10000]
--mode udp: stream i's pings go to --endpoint port + i (stream id S+i), its echoes to --reply port + i
  (S+10000+i); ping and pong take the same --endpoint (the pong's address) and --reply (the ping's), and
  --interface is this end's own address. --endpoint, --reply and --interface are UDP only. Ports
  20000-21999 (the media driver's wildcard ranges) are refused.
--mode spy-udp: --role local measures as --mode spy does, its pings published to --endpoint and its echoes
  to --reply, both a sink's addresses on another host (reliable, ssc=false: the sink connects them);
  --role pong is that sink: it reads every frame of both channels and writes its counts (sink.*).
--cpus: ping: ping; pong: echo; local: sender,receiver (--round-trip: ping,echo).
--driver-busy-threads: the external driver's busy threads (DEDICATED: 3), counted in the row's busy_threads.
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
    embedded: bool,
    term_length: u32,
    mtu: u32,
    stream_id: i32,
    driver_busy_threads: u32,
    connect_timeout: Duration,
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
        let a = Self {
            mode,
            reliable: f.bool("reliable", true)?,
            endpoint,
            reply,
            interface: f.take("interface"),
            dir: f.req("aeron-dir")?,
            embedded: match f.take("driver").as_deref() {
                None | Some("external") => false,
                Some("embedded") => true,
                Some(other) => return Err(Error::Usage(format!("--driver {other}: external or embedded"))),
            },
            term_length: f.num("term-length", 1_048_576)?,
            mtu: f.num("mtu", 1408)?,
            stream_id: f.num("stream-id", 1001)?,
            driver_busy_threads: f.num("driver-busy-threads", 3)?,
            connect_timeout: Duration::from_secs(f.num("connect-timeout-s", 30)?),
        };
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

/// A client of the driver at `dir`, its memory mapped and touched up front.
fn client(dir: &str) -> Result<Aeron, Error> {
    let dir = CString::new(dir).map_err(|_| Error::Usage("--aeron-dir holds a NUL byte".into()))?;
    let ctx = AeronContext::new().map_err(aeron_err("the client's context"))?;
    ctx.set_dir(&dir).map_err(aeron_err("the client's directory"))?;
    ctx.set_pre_touch_mapped_memory(true)
        .map_err(aeron_err("pre-touching the client's mappings"))?;
    let aeron = Aeron::new(&ctx).map_err(aeron_err("the client"))?;
    aeron.start().map_err(aeron_err("starting the client"))?;
    Ok(aeron)
}

/// Retries `add` while the driver says a previous run's endpoint is still
/// closing ("please retry"), for 10 s.
fn retry_closing<T>(what: &str, mut add: impl FnMut() -> Result<T, AeronCError>) -> Result<T, Error> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match add() {
            Ok(added) => return Ok(added),
            Err(e) if format!("{e:?}").contains("please retry") && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => return Err(Error::Transport(format!("adding the {what}: {e:?}"))),
        }
    }
}

fn publication(aeron: &Aeron, uri: &str, stream: i32) -> Result<AeronExclusivePublication, Error> {
    let uri = CString::new(uri).map_err(|_| Error::Usage("a channel with a NUL byte".into()))?;
    retry_closing("publication", || {
        aeron
            .async_add_exclusive_publication(&uri, stream)?
            .poll_blocking(Duration::from_secs(5))
    })
}

fn subscription(aeron: &Aeron, uri: &str, stream: i32) -> Result<AeronSubscription, Error> {
    let uri = CString::new(uri).map_err(|_| Error::Usage("a channel with a NUL byte".into()))?;
    retry_closing("subscription", || {
        aeron
            .async_add_subscription(&uri, stream, Handlers::NONE, Handlers::NONE)?
            .poll_blocking(Duration::from_secs(5))
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

/// Publications on `channel(i)` for every stream.
fn publications(
    aeron: &Aeron,
    streams: u32,
    channel: impl Fn(u32) -> Result<(String, String, i32), Error>,
) -> Result<AeronTx, Error> {
    let pubs = (0..streams)
        .map(|i| {
            let (uri, _, id) = channel(i)?;
            let p = publication(aeron, &uri, id)?;
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
    aeron: &Aeron,
    streams: u32,
    channel: impl Fn(u32) -> Result<(String, String, i32), Error>,
) -> Result<Vec<AeronSubscription>, Error> {
    (0..streams)
        .map(|i| {
            let (_, uri, id) = channel(i)?;
            subscription(aeron, &uri, id)
        })
        .collect()
}

/// Waits until every publication is connected and every subscription has
/// an image, then takes the images.
fn ready(tx: Option<&AeronTx>, subs: Vec<AeronSubscription>, timeout: Duration) -> Result<AeronRx, Error> {
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
        std::thread::sleep(Duration::from_millis(1));
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
/// first agent until stopped.
struct Embedded {
    stop: std::sync::Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Embedded {
    fn start(dir: &str) -> Result<Self, Error> {
        let stop = std::sync::Arc::new(AtomicBool::new(false));
        let (tx, rx) = std::sync::mpsc::channel::<Result<(), Error>>();
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
                        let _ = tx.send(Ok(()));
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
        rx.recv()
            .map_err(|_| Error::Transport("the embedded driver's thread ended".into()))??;
        Ok(Self {
            stop,
            thread: Some(thread),
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
                let aeron = client(&a.dir)?;
                let tx = publications(&aeron, c.streams, |i| a.ping_channel(i))?;
                ready(Some(&tx), Vec::new(), a.connect_timeout)?;
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
                let aeron = client(&a.dir)?;
                let subs = subscriptions(&aeron, c.streams, |i| a.ping_channel(i))?;
                let rx = ready(None, subs, a.connect_timeout)?;
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
        })
    })
}

/// The ping end on this thread: its client, publications and echo
/// subscriptions, then (after `barrier`, when given) the schedule.
fn ping_end(
    c: &Common,
    a: &Aeronish,
    clock: &TscClock,
    cpu: Option<usize>,
    barrier: Option<&Barrier>,
) -> Result<Ends, Error> {
    let setup = (|| -> Result<_, Error> {
        let aeron = client(&a.dir)?;
        let tx = publications(&aeron, c.streams, |i| a.ping_channel(i))?;
        let subs = subscriptions(&aeron, c.streams, |i| a.echo_channel(i))?;
        let rx = ready(Some(&tx), subs, a.connect_timeout)?;
        point::pin(cpu)?;
        Ok((aeron, tx, rx))
    })();
    if let Some(b) = barrier {
        b.wait();
    }
    let (_aeron, mut tx, mut rx) = setup?;
    let (t, rt) = point::start_at(clock, START_DELAY_NS);
    let mut rec = Recorder::new(clock, t, c)?;
    let a0 = thread_allocs();
    let send = point::ping(clock, c, t, &mut tx, &mut rx, &mut rec)?;
    Ok(Ends {
        rec,
        send,
        allocs: thread_allocs() - a0,
        start_realtime_ns: rt,
        echoes: None,
    })
}

/// The echo end on this thread: its client, ping subscriptions and echo
/// publications (then `added`), then (after `barrier`, when given) echoes
/// until the pings go or `keep_going` says stop. Returns its hot-path
/// allocations and the echoes it sent.
fn echo_end(
    c: &Common,
    a: &Aeronish,
    cpu: Option<usize>,
    barrier: Option<&Barrier>,
    added: &dyn Fn(),
    keep_going: &mut impl FnMut() -> bool,
) -> Result<(u64, u64), Error> {
    let setup = (|| -> Result<_, Error> {
        let aeron = client(&a.dir)?;
        let subs = subscriptions(&aeron, c.streams, |i| a.ping_channel(i))?;
        let tx = publications(&aeron, c.streams, |i| a.echo_channel(i))?;
        added();
        let rx = ready(Some(&tx), subs, a.connect_timeout)?;
        point::pin(cpu)?;
        Ok((aeron, tx, rx))
    })();
    if let Some(b) = barrier {
        b.wait();
    }
    let (_aeron, mut tx, mut rx) = setup?;
    let a0 = thread_allocs();
    let echoes = point::echo(&mut rx, &mut tx, keep_going)?;
    Ok((thread_allocs() - a0, echoes))
}

fn local_round_trip(c: &Common, a: &Aeronish, clock: &TscClock) -> Result<Ends, Error> {
    let (barrier, stop) = (Barrier::new(2), AtomicBool::new(false));
    let (barrier, stop) = (&barrier, &stop);
    std::thread::scope(|s| {
        let echo = s.spawn(move || {
            echo_end(c, a, c.cpu(1), Some(barrier), &|| {}, &mut || {
                !stop.load(Ordering::Acquire)
            })
        });
        let ping = s.spawn(move || {
            let r = ping_end(c, a, clock, c.cpu(0), Some(barrier));
            stop.store(true, Ordering::Release);
            r
        });
        let mut ends = joined(ping.join(), "ping")?;
        let (allocs, echoes) = joined(echo.join(), "echo")?;
        ends.allocs += allocs;
        ends.echoes = Some(echoes);
        Ok(ends)
    })
}

fn manifest(c: &Common, a: &Aeronish, transport: &str) -> Result<RunManifest, Error> {
    let mut m = RunManifest::new(transport, &template(), a.wire_bytes());
    let (ping_publication, ping_subscription, ping_id) = a.ping_channel(0)?;
    m.setting("transport", transport)
        .setting("mode", a.mode_name())
        .setting("reliable", &a.reliable.to_string())
        .setting("driver", if a.embedded { "embedded" } else { "external" })
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
        .setting("thread_order", thread_order(c, a.mode));
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

fn thread_order(c: &Common, mode: Mode) -> &'static str {
    match (c.role, c.round_trip) {
        (Role::Local, false) => "sender,receiver",
        (Role::Local, true) => "ping,echo",
        (Role::Ping, _) => "ping",
        _ if mode == Mode::SpyUdp => "sink",
        _ => "echo",
    }
}

const fn client_threads(c: &Common) -> u32 {
    match c.role {
        Role::Local => 2,
        _ => 1,
    }
}

fn run(c: &Common, a: &Aeronish) -> Result<Option<String>, Error> {
    match c.role {
        Role::OnewaySend | Role::OnewayRecv => {
            return Err(Error::Usage(format!(
                "--role {}: aeron_lat measures one way with --role local (both ends in this process, one counter)",
                c.role.name()
            )));
        }
        Role::Local | Role::Ping | Role::Pong => {}
    }
    c.check_cpus(client_threads(c) as usize, thread_order(c, a.mode))?;
    let _driver = if a.embedded {
        Some(Embedded::start(&a.dir)?)
    } else {
        None
    };
    let counters = client(&a.dir)?;
    let before = counters_now(&counters);
    let transport = match a.mode {
        Mode::Ipc => "aeron-ipc",
        Mode::Spy => "aeron-spy",
        Mode::SpyUdp if before.dpdk => "aeron-spy-dpdk",
        Mode::SpyUdp => "aeron-spy-udp",
        Mode::Udp if before.dpdk => "aeron-dpdk",
        Mode::Udp => "aeron-udp",
    };
    let mut m = manifest(c, a, transport)?;
    if c.role == Role::Pong {
        stop_on_sigterm();
        let allocs = if a.mode == Mode::SpyUdp {
            sink(c, a, &mut m)?
        } else {
            echo_end(c, a, c.cpu(0), None, &|| println!("ready"), &mut || {
                !STOP.load(Ordering::Relaxed)
            })?
            .0
        };
        record_counters(&mut m, &before, &counters_now(&counters));
        m.setting("hot_path_allocs", &allocs.to_string())
            .setting("busy_threads", &(client_threads(c) + a.driver_busy_threads).to_string());
        point::write_manifest(c, m)?;
        return Ok(None);
    }
    let clock = TscClock::new();
    let ends = match c.role {
        Role::Ping => ping_end(c, a, &clock, c.cpu(0), None)?,
        _ if c.round_trip => local_round_trip(c, a, &clock)?,
        _ => local_oneway(c, a, &clock)?,
    };
    let end = clock.raw();
    let retrans = record_counters(&mut m, &before, &counters_now(&counters));
    if let Some(e) = ends.echoes {
        m.setting("echoes_sent", &e.to_string());
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
            busy_threads: client_threads(c) + a.driver_busy_threads,
            hot_path_allocs: ends.allocs,
            manifest: m,
        },
    )?;
    Ok(Some(row))
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
    let aeron = client(&a.dir)?;
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
        f.finish()?;
        run(&c, &a)
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
