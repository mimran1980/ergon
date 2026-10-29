//! `aeron_lat --load mixed`: a latency-measured hot stream beside background
//! streams of a realistic mix, over two hosts (`--role ping` and `--role
//! pong`, `--mode udp`).
//!
//! Stream 0 is the hot stream: the 64-byte message at `--hot-rate`, paced,
//! echoed and recorded by the same code as every other arm's (`point.rs`),
//! on its own channel (`--endpoint`/`--reply` port + 0) and its own busy
//! thread at each end. Streams 1 to `--streams` - 1 are the background,
//! together `--rate` less `--hot-rate` msg/s, on a busy thread of their own
//! at each end (`--cpus hot,background`), four streams to a channel (port +
//! 1 + (i - 1) / 4). Stream `i`'s class is `(i - 1) mod 8`'s:
//!
//! | `(i - 1) mod 8` | class | publishers | burst | sizes (‰ of 64 / 288 / 1344 / 4096 B) |
//! |---|---|---|---|---|
//! | 0-3 | steady, weight 1, 2, 4 or 8 by `(i - 1) / 8 mod 4` | 1 | 1 | 550 / 300 / 130 / 20 |
//! | 4-5 | bursty, weight 4 | 1 | 8 at once | 600 / 300 / 100 / 0 |
//! | 6 | idle, then a burst, weight 1 | 1 | 128 at once | 700 / 300 / 0 / 0 |
//! | 7 | two publishers, weight 4 (half each) | 2 | 1 | 550 / 300 / 130 / 20 |
//!
//! A stream's rate is its weight's share of the background's; each
//! publisher sends its bursts every `burst / rate` seconds from a phase of
//! its own (so the bursts of different streams do not line up). A message's
//! size is a hash of its stream, publisher and sequence number, so both
//! ends know it: 4096 B fragments into three frames at the lab's MTU of
//! 1408 (Aeron's frame payload 1376 B), 1344 B is the largest that does
//! not. On the pong, stream `i` has `1 + i mod 4` subscriptions (fan-out 1
//! to 4); the first echoes every message back on stream `i`'s echo, the
//! rest read and check it. Every subscription checks every message (its
//! length, stream, publisher, sequence and tail) and counts; both ends
//! write the counts per stream into their manifests (`mixed.*`), which
//! fairness.py holds against each other.
//!
//! Background message, little-endian, `len` bytes:
//!
//! | bytes | field |
//! |---|---|
//! | 0..8 | `seq`, per publisher from 0 |
//! | 8..16 | `send_tsc` |
//! | 16..24 | `intended_tsc` |
//! | 24..28 | stream |
//! | 28..30 | kind (1 ping, 3 echo) |
//! | 30..32 | publisher |
//! | 32..36 | `len` |
//! | 36..40 | [`MAGIC`] |
//! | 40..len-8 | zero |
//! | len-8..len | `seq ^` [`TAIL`] |

use super::{
    Aeronish, Client, Common, ECHO_OFFSET, Error, Mode, POLL_LIMIT, START_DELAY_NS, aeron_err, publication,
    subscription,
};
use ergon_latency_core::clock::TscClock;
use ergon_latency_core::hist::Latency;
use ergon_latency_core::manifest::RunManifest;
use point::{Role, thread_allocs};
use rusteron_client::{
    Aeron, AeronExclusivePublication, AeronFragmentClosureAssembler, AeronHeader, AeronOfferError, AeronSubscription,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use super::point;

/// The profile's name, in the manifest (`mixed.profile`): a change to the
/// table above is a new one.
pub const PROFILE: &str = "mixed-v1";
/// The background's message sizes.
pub const SIZES: [usize; 4] = [64, 288, 1344, 4096];
const MAX_SIZE: usize = 4096;
const HEADER: usize = 40;
/// Bytes 36..40 of every background message.
pub const MAGIC: u32 = 0x4D49_5844;
/// A background message's last eight bytes: its `seq` XOR this.
pub const TAIL: u64 = 0xA5A5_5A5A_C3C3_3C3C;
const PING: u16 = 1;
const ECHO: u16 = 3;
/// Background streams to a channel (a port).
pub const STREAMS_PER_CHANNEL: u32 = 4;
/// Aeron's data frame header.
const FRAME_HEADER: usize = 32;
/// Frames are aligned to this.
const FRAME_ALIGN: usize = 32;
/// How long a stopped pong reads on while frames still come, and at most.
const QUIET: Duration = Duration::from_millis(200);
const DRAIN_MAX: Duration = Duration::from_secs(4);

/// A background stream's class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// One message at a time, evenly paced.
    Steady,
    /// Eight at once.
    Bursty,
    /// 128 at once, idle between.
    IdleBurst,
    /// Two publishers, each steady.
    MultiPub,
}

impl Class {
    const fn of(i: u32) -> Self {
        match (i - 1) % 8 {
            0..=3 => Self::Steady,
            4 | 5 => Self::Bursty,
            6 => Self::IdleBurst,
            _ => Self::MultiPub,
        }
    }

    /// Its letter in `mixed.classes`.
    pub const fn letter(self) -> char {
        match self {
            Self::Steady => 'S',
            Self::Bursty => 'B',
            Self::IdleBurst => 'I',
            Self::MultiPub => 'M',
        }
    }

    const fn weight(self, i: u32) -> u64 {
        match self {
            Self::Steady => 1 << ((i - 1) / 8 % 4),
            Self::Bursty | Self::MultiPub => 4,
            Self::IdleBurst => 1,
        }
    }

    const fn burst(self) -> u64 {
        match self {
            Self::Steady | Self::MultiPub => 1,
            Self::Bursty => 8,
            Self::IdleBurst => 128,
        }
    }

    const fn publishers(self) -> u32 {
        if matches!(self, Self::MultiPub) { 2 } else { 1 }
    }

    /// Per mille of [`SIZES`].
    const fn sizes(self) -> [u64; 4] {
        match self {
            Self::Steady | Self::MultiPub => [550, 300, 130, 20],
            Self::Bursty => [600, 300, 100, 0],
            Self::IdleBurst => [700, 300, 0, 0],
        }
    }
}

const fn mix(mut z: u64) -> u64 {
    // splitmix64's finaliser
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// One background stream.
#[derive(Debug, Clone)]
pub struct Stream {
    /// Its index, 1 and up (0 is the hot stream).
    pub index: u32,
    /// Its class.
    pub class: Class,
    /// Its publishers.
    pub publishers: u32,
    /// The pong's subscriptions to it.
    pub fanout: u32,
    /// Its channel (port + 1 + channel).
    pub channel: u32,
    /// Messages a burst.
    pub burst: u64,
    /// Between a publisher's bursts, ns.
    pub period_ns: u64,
}

impl Stream {
    /// Publisher `p`'s first burst, ns after the schedule's start: a hash of
    /// the stream and publisher within the period.
    pub const fn phase_ns(&self, p: u32) -> u64 {
        mix(((self.index as u64) << 8) ^ p as u64 ^ 0x5EED) % self.period_ns
    }

    /// Message `seq` of publisher `p`: when the schedule owes it, ns after
    /// its start.
    pub const fn due_ns(&self, p: u32, seq: u64) -> u64 {
        self.phase_ns(p) + seq / self.burst * self.period_ns
    }

    /// The messages publisher `p` owes in a schedule of `window_ns`.
    pub const fn total(&self, p: u32, window_ns: u64) -> u64 {
        let phase = self.phase_ns(p);
        if phase >= window_ns {
            return 0;
        }
        ((window_ns - phase - 1) / self.period_ns + 1) * self.burst
    }

    /// Message `seq` of publisher `p`: its size.
    pub fn size(&self, p: u32, seq: u64) -> usize {
        let roll = mix((u64::from(self.index) << 40) ^ (u64::from(p) << 32) ^ seq) % 1000;
        let mut acc = 0;
        for (w, s) in self.class.sizes().iter().zip(SIZES) {
            acc += w;
            if roll < acc {
                return s;
            }
        }
        SIZES[0]
    }

    /// msg/s the stream sends, every publisher together.
    #[allow(clippy::cast_precision_loss)] // a rate to print
    pub fn rate(&self) -> f64 {
        f64::from(self.publishers) * self.burst as f64 * 1e9 / self.period_ns as f64
    }
}

const fn align(n: usize) -> usize {
    n.div_ceil(FRAME_ALIGN) * FRAME_ALIGN
}

/// Bytes on the wire (Aeron frames, headers and alignment) for a message of
/// `len` bytes at `mtu`.
pub const fn frame_bytes(len: usize, mtu: usize) -> usize {
    let room = mtu - FRAME_HEADER;
    if len <= room {
        return align(len + FRAME_HEADER);
    }
    let full = len / room;
    let rest = len % room;
    full * align(room + FRAME_HEADER) + if rest == 0 { 0 } else { align(rest + FRAME_HEADER) }
}

/// The frames a message of `len` bytes takes at `mtu`.
pub const fn frames(len: usize, mtu: usize) -> usize {
    len.div_ceil(mtu - FRAME_HEADER)
}

/// The point's mix: `--streams`, `--rate` (every stream) and `--hot-rate`.
#[derive(Debug, Clone)]
pub struct Profile {
    /// The hot stream's msg/s.
    pub hot_rate: u64,
    /// Every stream's.
    pub rate: u64,
    /// The background streams, 1 and up.
    pub streams: Vec<Stream>,
    /// The background's channels.
    pub channels: u32,
}

impl Profile {
    /// The mix of `streams` streams at `rate` msg/s, `hot_rate` of them the
    /// hot stream's.
    ///
    /// # Errors
    /// [`Error::Usage`] for fewer than two streams, a hot rate not under the
    /// total, or a background too slow to send in a second.
    pub fn new(streams: u32, rate: u64, hot_rate: u64) -> Result<Self, Error> {
        if streams < 2 {
            return Err(Error::Usage(format!(
                "--load mixed --streams {streams}: the hot stream and at least one background stream"
            )));
        }
        if hot_rate == 0 || hot_rate >= rate {
            return Err(Error::Usage(format!(
                "--hot-rate {hot_rate}: at least 1 and under --rate {rate} (the background takes the rest)"
            )));
        }
        let bg = rate - hot_rate;
        let weights: u64 = (1..streams).map(|i| Class::of(i).weight(i)).sum();
        let mut out = Vec::new();
        for i in 1..streams {
            let class = Class::of(i);
            let publishers = class.publishers();
            // msg/s of each publisher, its share of the background's
            let per_pub = u128::from(bg) * u128::from(class.weight(i)) / u128::from(publishers);
            let period = u128::from(class.burst()) * 1_000_000_000 * u128::from(weights) / per_pub.max(1);
            let period_ns = u64::try_from(period).unwrap_or(u64::MAX);
            if period_ns > 1_000_000_000 * 3600 {
                return Err(Error::Usage(format!(
                    "--rate {rate}: background stream {i} would send a burst an hour at most"
                )));
            }
            out.push(Stream {
                index: i,
                class,
                publishers,
                fanout: 1 + i % 4,
                channel: (i - 1) / STREAMS_PER_CHANNEL,
                burst: class.burst(),
                period_ns: period_ns.max(1),
            });
        }
        let channels = (streams - 1).div_ceil(STREAMS_PER_CHANNEL);
        Ok(Self {
            hot_rate,
            rate,
            streams: out,
            channels,
        })
    }

    /// The background's msg/s as scheduled (the periods' rounding apart, its
    /// share of `--rate`).
    pub fn bg_rate(&self) -> f64 {
        self.streams.iter().map(Stream::rate).sum()
    }

    /// Each list's entries joined by commas, one a background stream.
    fn list<T: ToString>(&self, f: impl Fn(&Stream) -> T) -> String {
        self.streams
            .iter()
            .map(|s| f(s).to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// The profile's settings in the manifest.
    pub fn settings(&self, m: &mut RunManifest) {
        m.setting("load", "mixed")
            .setting("mixed.profile", PROFILE)
            .setting("hot_rate", &self.hot_rate.to_string())
            .setting("mixed.rate", &self.rate.to_string())
            .setting("mixed.bg_rate", &format!("{:.1}", self.bg_rate()))
            .setting("mixed.bg_streams", &self.streams.len().to_string())
            .setting("mixed.channels", &self.channels.to_string())
            .setting("mixed.streams_per_channel", &STREAMS_PER_CHANNEL.to_string())
            .setting(
                "mixed.classes",
                &self.streams.iter().map(|s| s.class.letter()).collect::<String>(),
            )
            .setting("mixed.publishers", &self.list(|s| s.publishers))
            .setting("mixed.fanout", &self.list(|s| s.fanout))
            .setting("mixed.burst", &self.list(|s| s.burst))
            .setting("mixed.period_ns", &self.list(|s| s.period_ns))
            .setting("mixed.stream_rate", &self.list(|s| format!("{:.1}", s.rate())))
            .setting("mixed.sizes", &SIZES.map(|s| s.to_string()).join(","));
    }
}

/// `--load uniform|mixed` and `--hot-rate`: the mix, or none.
///
/// # Errors
/// [`Error::Usage`] for a mix the arm cannot run: `--mode udp`, `--role
/// ping` or `pong`, a term that holds the largest message.
pub fn parse(f: &mut point::Flags, c: &Common, a: &Aeronish) -> Result<Option<Profile>, Error> {
    let load = f.take("load");
    let hot = f.take("hot-rate");
    match load.as_deref() {
        None | Some("uniform") => {
            if hot.is_some() {
                return Err(Error::Usage("--hot-rate is --load mixed's".into()));
            }
            return Ok(None);
        }
        Some("mixed") => {}
        Some(other) => return Err(Error::Usage(format!("--load {other}: uniform or mixed"))),
    }
    if a.mode != Mode::Udp || !matches!(c.role, Role::Ping | Role::Pong) {
        return Err(Error::Usage(
            "--load mixed is a cross-host round trip: --role ping or pong, --mode udp".into(),
        ));
    }
    let hot: u64 = hot
        .ok_or_else(|| Error::Usage("--load mixed needs --hot-rate MSG_PER_S".into()))?
        .parse()
        .map_err(|_| Error::Usage("--hot-rate: msg/s".into()))?;
    // the pong's --rate is unused elsewhere (0): its mix needs the ping's
    if c.rate == 0 {
        return Err(Error::Usage(
            "--load mixed: both ends need --rate (the mix is drawn from it)".into(),
        ));
    }
    // a term must hold a message of 4096 B (Aeron: an eighth of the term)
    if (a.term_length as usize) / 8 < MAX_SIZE {
        return Err(Error::Usage(format!(
            "--term-length {}: --load mixed sends {MAX_SIZE}-byte messages (a term of 32 KiB at least)",
            a.term_length
        )));
    }
    Profile::new(c.streams, c.rate, hot).map(Some)
}

/// What the hot thread and the background thread of one process share.
#[derive(Debug, Default)]
pub struct Gate {
    ready: AtomicBool,
    failed: AtomicBool,
    done: AtomicBool,
    start: AtomicU64,
    start_rt: AtomicU64,
}

impl Gate {
    /// The background is set up (its channels added; on the ping, connected).
    pub fn ready(&self) -> Result<bool, Error> {
        if self.failed.load(Ordering::Acquire) {
            return Err(Error::Peer("background"));
        }
        Ok(self.ready.load(Ordering::Acquire))
    }

    /// The hot thread failed: the background stops waiting for it.
    pub fn fail(&self) {
        self.failed.store(true, Ordering::Release);
    }

    /// The hot thread's schedule starts at raw `tsc` (realtime `rt` ns).
    pub fn start(&self, tsc: u64, rt: u128) {
        self.start_rt
            .store(u64::try_from(rt).unwrap_or(u64::MAX), Ordering::Release);
        self.start.store(tsc, Ordering::Release);
    }

    /// The background is done (or failed).
    pub fn done(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    fn wait_start(&self, stop: &dyn Fn() -> bool) -> Option<u64> {
        loop {
            let t = self.start.load(Ordering::Acquire);
            if t != 0 {
                return Some(t);
            }
            if stop() {
                return None;
            }
            std::hint::spin_loop();
        }
    }
}

/// Marks the gate done when dropped, failed unless told it succeeded.
struct Finished<'a> {
    gate: &'a Gate,
    ok: bool,
}

impl Drop for Finished<'_> {
    fn drop(&mut self) {
        if !self.ok {
            self.gate.failed.store(true, Ordering::Release);
        }
        self.gate.done.store(true, Ordering::Release);
    }
}

/// A background message's header fields.
#[derive(Debug, Clone, Copy)]
struct Header {
    seq: u64,
    intended: u64,
    stream: u32,
    publisher: u32,
}

fn u64_at(b: &[u8], at: usize) -> u64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[at..at + 8]);
    u64::from_le_bytes(a)
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    let mut a = [0u8; 4];
    a.copy_from_slice(&b[at..at + 4]);
    u32::from_le_bytes(a)
}

const fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

/// Writes message `seq` of `stream`'s publisher `publisher` into `buf`
/// (its tail at `len`); its length.
fn encode(buf: &mut [u8; MAX_SIZE], h: Header, send_tsc: u64, len: usize) -> usize {
    buf[0..8].copy_from_slice(&h.seq.to_le_bytes());
    buf[8..16].copy_from_slice(&send_tsc.to_le_bytes());
    buf[16..24].copy_from_slice(&h.intended.to_le_bytes());
    buf[24..28].copy_from_slice(&h.stream.to_le_bytes());
    buf[28..30].copy_from_slice(&PING.to_le_bytes());
    buf[30..32].copy_from_slice(&u16::try_from(h.publisher).unwrap_or(u16::MAX).to_le_bytes());
    buf[32..36].copy_from_slice(&u32::try_from(len).unwrap_or(u32::MAX).to_le_bytes());
    buf[36..40].copy_from_slice(&MAGIC.to_le_bytes());
    buf[len - 8..len].copy_from_slice(&(h.seq ^ TAIL).to_le_bytes());
    len
}

/// What is wrong with a background payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Bad {
    Short,
    Magic,
    Length,
    Kind,
    Stream,
    Publisher,
    Tail,
    Size,
}

/// Checks a background payload of `kind` on `stream` against the profile:
/// its header.
fn check(p: &[u8], kind: u16, stream: &Stream) -> Result<Header, Bad> {
    if p.len() < HEADER + 8 {
        return Err(Bad::Short);
    }
    if u32_at(p, 36) != MAGIC {
        return Err(Bad::Magic);
    }
    if u32_at(p, 32) as usize != p.len() {
        return Err(Bad::Length);
    }
    if u16_at(p, 28) != kind {
        return Err(Bad::Kind);
    }
    let h = Header {
        seq: u64_at(p, 0),
        intended: u64_at(p, 16),
        stream: u32_at(p, 24),
        publisher: u32::from(u16_at(p, 30)),
    };
    if h.stream != stream.index {
        return Err(Bad::Stream);
    }
    if h.publisher >= stream.publishers {
        return Err(Bad::Publisher);
    }
    if u64_at(p, p.len() - 8) != h.seq ^ TAIL {
        return Err(Bad::Tail);
    }
    if stream.size(h.publisher, h.seq) != p.len() {
        return Err(Bad::Size);
    }
    Ok(h)
}

/// One reader's count of a stream: per publisher the next sequence number
/// due; what came, what was skipped, what came behind, what was refused.
#[derive(Debug, Default, Clone)]
struct Count {
    next: [u64; 2],
    received: u64,
    /// Aeron frame bytes of what was received.
    bytes: u64,
    gaps: u64,
    behind: u64,
    refused: u64,
    first_bad: Option<Bad>,
}

impl Count {
    /// A checked message of publisher `p`, sequence `seq`, `bytes` in
    /// frames: true when new and in order (not behind).
    const fn on(&mut self, p: u32, seq: u64, bytes: u64) -> bool {
        let next = &mut self.next[p as usize];
        if seq == *next {
            *next += 1;
        } else if seq > *next {
            self.gaps += seq - *next;
            *next = seq + 1;
        } else {
            self.behind += 1;
            return false;
        }
        self.received += 1;
        self.bytes += bytes;
        true
    }

    const fn refuse(&mut self, why: Bad) {
        self.refused += 1;
        if self.first_bad.is_none() {
            self.first_bad = Some(why);
        }
    }
}

/// The background channels of `a` for stream `s`: (ping publication or
/// subscription URI, echo URI, ping stream id, echo stream id), on its
/// channel's port.
fn channel(a: &Aeronish, s: &Stream) -> Result<(String, String, String, String, i32, i32), Error> {
    let port = 1 + s.channel;
    let ep = a.endpoint.as_ref().map(|e| e.of("endpoint", port)).transpose()?;
    let rp = a.reply.as_ref().map(|e| e.of("reply", port)).transpose()?;
    let (id, eid) = (a.stream(s.index, 0), a.stream(s.index, ECHO_OFFSET));
    Ok((
        a.pub_uri(ep.as_deref(), id),
        a.sub_uri(ep.as_deref(), id),
        a.pub_uri(rp.as_deref(), eid),
        a.sub_uri(rp.as_deref(), eid),
        id,
        eid,
    ))
}

fn largest_message(p: &AeronExclusivePublication) -> Result<usize, Error> {
    Ok(p.get_constants()
        .map_err(aeron_err("a publication's constants"))?
        .max_message_length())
}

/// Waits until every publication is connected and every subscription has
/// its images (`want[i]`), the duty run while waiting (`rest`).
fn connected(
    client: &Client,
    pubs: &[&AeronExclusivePublication],
    subs: &[(&AeronSubscription, i32)],
    timeout: Duration,
    gate: &Gate,
) -> Result<(), Error> {
    let deadline = Instant::now() + timeout;
    loop {
        // connected, and its first status message in (a limit to offer to)
        let up = pubs.iter().filter(|p| p.is_ready()).count();
        let imaged = subs
            .iter()
            .filter(|(s, want)| s.image_count().unwrap_or(0) >= *want)
            .count();
        if up == pubs.len() && imaged == subs.len() {
            return Ok(());
        }
        if gate.failed.load(Ordering::Acquire) {
            return Err(Error::Peer("hot stream"));
        }
        if Instant::now() > deadline {
            return Err(Error::Timeout(format!(
                "the background: {up}/{} publications connected, {imaged}/{} subscriptions with their images within {} s",
                pubs.len(),
                subs.len(),
                timeout.as_secs()
            )));
        }
        client.rest(Duration::from_millis(1))?;
    }
}

/// One background publisher on the ping.
struct Pub {
    stream: usize,
    publisher: u32,
    publication: AeronExclusivePublication,
    next: u64,
    total: u64,
    sent: u64,
    bp: u64,
    bp_measured: u64,
    bp_seq: u64,
    admin: u64,
    admin_measured: u64,
    admin_seq: u64,
    bytes: u64,
    frames: u64,
    frame_bytes: u64,
    measured_sent: u64,
}

/// One background stream's echo subscription on the ping.
struct EchoRx {
    sub: AeronSubscription,
    asm: AeronFragmentClosureAssembler,
    count: Count,
}

/// What the ping's echo callback records into.
struct PingCtx<'a> {
    clock: &'a TscClock,
    stream: &'a Stream,
    count: &'a mut Count,
    hist: &'a mut Latency,
    measured_from: u64,
    max: &'a mut (u64, u32),
    mtu: usize,
}

fn on_echo(ctx: &mut PingCtx<'_>, payload: &[u8], _header: AeronHeader) {
    let recv = ctx.clock.raw();
    match check(payload, ECHO, ctx.stream) {
        Ok(h) => {
            let fb = frame_bytes(payload.len(), ctx.mtu) as u64;
            if ctx.count.on(h.publisher, h.seq, fb) && h.intended >= ctx.measured_from {
                let ns = ctx.clock.delta_ns(h.intended, recv);
                ctx.hist.record(ns);
                if ns > ctx.max.0 {
                    *ctx.max = (ns, h.stream);
                }
            }
        }
        Err(why) => ctx.count.refuse(why),
    }
}

/// The ping's background, as it ended.
pub struct PingBackground {
    /// Latency of every measured background message, from its intended send.
    pub hist: Latency,
    /// Its manifest settings.
    pub settings: BTreeMap<String, String>,
    /// msg/s it achieved over the measured window.
    pub achieved: f64,
}

/// Each stream's sum over `f` of its entries, a comma list.
fn per_stream<T>(n: usize, items: &[T], stream: impl Fn(&T) -> usize, f: impl Fn(&T) -> u64) -> String {
    let mut v = vec![0u64; n];
    for it in items {
        v[stream(it)] += f(it);
    }
    v.iter().map(ToString::to_string).collect::<Vec<_>>().join(",")
}

/// The ping's background thread: its own client of the driver at `a.dir`,
/// every background stream's publications and echo subscriptions, then
/// (once `gate` says the hot thread's schedule starts) the schedule, then
/// up to [`point::DRAIN_NS`] for what is still in flight.
///
/// # Errors
/// The transport's, a payload refused, or [`Error::Timeout`].
#[allow(clippy::too_many_lines, clippy::cast_precision_loss)]
pub fn ping(
    c: &Common,
    a: &Aeronish,
    profile: &Profile,
    clock: &TscClock,
    gate: &Gate,
) -> Result<PingBackground, Error> {
    let mut finished = Finished { gate, ok: false };
    let client = Client::of(&a.dir)?;
    let mtu = a.mtu as usize;
    let window_ns = c
        .warmup_s
        .checked_add(c.duration_s)
        .and_then(|s| s.checked_mul(1_000_000_000))
        .ok_or_else(|| Error::Usage("--warmup-s and --duration-s overflow".into()))?;
    let warm_ns = c.warmup_s * 1_000_000_000;
    let mut pubs = Vec::new();
    let mut echoes = Vec::new();
    for (k, s) in profile.streams.iter().enumerate() {
        let (pub_uri, _, _, echo_sub, id, eid) = channel(a, s)?;
        for p in 0..s.publishers {
            let publication = publication(&client, &pub_uri, id)?;
            if largest_message(&publication)? < MAX_SIZE {
                return Err(Error::Usage(format!(
                    "{pub_uri}: its largest message under {MAX_SIZE} B"
                )));
            }
            pubs.push(Pub {
                stream: k,
                publisher: p,
                publication,
                next: 0,
                total: s.total(p, window_ns),
                sent: 0,
                bp: 0,
                bp_measured: 0,
                bp_seq: u64::MAX,
                admin: 0,
                admin_measured: 0,
                admin_seq: u64::MAX,
                bytes: 0,
                frames: 0,
                frame_bytes: 0,
                measured_sent: 0,
            });
        }
        let sub = subscription(&client, &echo_sub, eid)?;
        let asm = AeronFragmentClosureAssembler::new().map_err(aeron_err("a fragment assembler"))?;
        echoes.push(EchoRx {
            sub,
            asm,
            count: Count::default(),
        });
    }
    {
        let ps: Vec<&AeronExclusivePublication> = pubs.iter().map(|p| &p.publication).collect();
        let ss: Vec<(&AeronSubscription, i32)> = echoes.iter().map(|e| (&e.sub, 1)).collect();
        connected(&client, &ps, &ss, a.connect_timeout, gate)?;
    }
    point::pin(c.cpu(1))?;
    gate.ready.store(true, Ordering::Release);
    let Some(start) = gate.wait_start(&|| gate.failed.load(Ordering::Acquire)) else {
        return Err(Error::Peer("hot stream"));
    };
    let measured_from = start.saturating_add(clock.ns_to_ticks(warm_ns));
    let deadline = c.deadline(clock, start);
    let mut hist = Latency::new()?;
    let mut buf = [0u8; MAX_SIZE];
    let mut max = (0u64, 0u32);
    let due = |p: &Pub| -> u64 {
        let s = &profile.streams[p.stream];
        start.saturating_add(clock.ns_to_ticks(s.due_ns(p.publisher, p.next)))
    };
    let mut due_at: Vec<u64> = pubs
        .iter()
        .map(|p| if p.total == 0 { u64::MAX } else { due(p) })
        .collect();
    let (mut passes, mut busy_passes, mut busy_ticks) = (0u64, 0u64, 0u64);
    let a0 = thread_allocs();
    let loop_start = clock.raw();
    let all_sent = |pubs: &[Pub]| pubs.iter().all(|p| p.next >= p.total);
    let mut last = loop_start;
    let send_end = loop {
        let now = clock.raw();
        passes += 1;
        let mut work = 0usize;
        for (i, p) in pubs.iter_mut().enumerate() {
            while p.next < p.total && due_at[i] <= now {
                let stream = &profile.streams[p.stream];
                let size = stream.size(p.publisher, p.next);
                let header = Header {
                    seq: p.next,
                    intended: due_at[i],
                    stream: stream.index,
                    publisher: p.publisher,
                };
                let send_tsc = clock.raw();
                let size = encode(&mut buf, header, send_tsc, size);
                let offered = p.publication.offer(&buf[..size]);
                // the body back to zero where the tail went
                buf[size - 8..size].fill(0);
                match offered {
                    Ok(_) => {
                        p.sent += 1;
                        p.bytes += size as u64;
                        p.frames += frames(size, mtu) as u64;
                        p.frame_bytes += frame_bytes(size, mtu) as u64;
                        if header.intended >= measured_from {
                            p.measured_sent += 1;
                        }
                        p.next += 1;
                        due_at[i] = if p.next < p.total { due(p) } else { u64::MAX };
                        work += 1;
                    }
                    // a term rotation, not back-pressure: counted apart,
                    // retried at once
                    Err(AeronOfferError::AdminAction) => {
                        if p.admin_seq != p.next {
                            p.admin_seq = p.next;
                            p.admin += 1;
                            if header.intended >= measured_from {
                                p.admin_measured += 1;
                            }
                        }
                    }
                    Err(e) if e.is_retryable() => {
                        if p.bp_seq != p.next {
                            p.bp_seq = p.next;
                            p.bp += 1;
                            if header.intended >= measured_from {
                                p.bp_measured += 1;
                            }
                        }
                        break;
                    }
                    Err(e) => {
                        return Err(Error::Transport(format!(
                            "background offer on stream {}: {e:?}",
                            stream.index
                        )));
                    }
                }
            }
        }
        work += poll_echoes(&mut echoes, profile, clock, &mut hist, measured_from, &mut max, mtu)?;
        let after = clock.raw();
        if work > 0 {
            busy_passes += 1;
            busy_ticks += after.saturating_sub(last.max(now));
        }
        last = after;
        if all_sent(&pubs) {
            break after;
        }
        if now > deadline {
            return Err(Error::Timeout(
                "the background: the schedule ran past its deadline".into(),
            ));
        }
    };
    let sent: u64 = pubs.iter().map(|p| p.sent).sum();
    let until = clock.raw().saturating_add(clock.ns_to_ticks(point::DRAIN_NS));
    while echoes.iter().map(|e| e.count.received).sum::<u64>() < sent && clock.raw() < until {
        poll_echoes(&mut echoes, profile, clock, &mut hist, measured_from, &mut max, mtu)?;
    }
    let allocs = thread_allocs() - a0;
    let loop_ticks = send_end.saturating_sub(loop_start);
    let refused: u64 = echoes.iter().map(|e| e.count.refused).sum();
    if refused > 0 {
        let first = echoes.iter().find_map(|e| e.count.first_bad);
        return Err(Error::Payload {
            count: refused,
            first: format!("a background echo refused: {first:?}"),
        });
    }
    let n = profile.streams.len();
    let mut st = BTreeMap::new();
    let mut put = |k: &str, v: String| {
        st.insert(k.to_owned(), v);
    };
    put("mixed.scheduled", per_stream(n, &pubs, |p| p.stream, |p| p.total));
    put("mixed.sent", per_stream(n, &pubs, |p| p.stream, |p| p.sent));
    put("mixed.bp", per_stream(n, &pubs, |p| p.stream, |p| p.bp));
    put(
        "mixed.bp_measured",
        per_stream(n, &pubs, |p| p.stream, |p| p.bp_measured),
    );
    put("mixed.admin", per_stream(n, &pubs, |p| p.stream, |p| p.admin));
    put(
        "mixed.admin_measured",
        per_stream(n, &pubs, |p| p.stream, |p| p.admin_measured),
    );
    put(
        "mixed.position",
        per_stream(
            n,
            &pubs,
            |p| p.stream,
            |p| u64::try_from(p.publication.position()).unwrap_or(0),
        ),
    );
    put("mixed.payload_bytes", per_stream(n, &pubs, |p| p.stream, |p| p.bytes));
    put(
        "mixed.echoes",
        echoes
            .iter()
            .map(|e| e.count.received.to_string())
            .collect::<Vec<_>>()
            .join(","),
    );
    put(
        "mixed.echo_gaps",
        echoes
            .iter()
            .map(|e| e.count.gaps.to_string())
            .collect::<Vec<_>>()
            .join(","),
    );
    put(
        "mixed.echo_behind",
        echoes
            .iter()
            .map(|e| e.count.behind.to_string())
            .collect::<Vec<_>>()
            .join(","),
    );
    let total = |f: &dyn Fn(&Pub) -> u64| pubs.iter().map(f).sum::<u64>();
    put("mixed.scheduled_total", total(&|p| p.total).to_string());
    put("mixed.sent_total", sent.to_string());
    put("mixed.bp_total", total(&|p| p.bp).to_string());
    put("mixed.admin_total", total(&|p| p.admin).to_string());
    put(
        "mixed.echoes_total",
        echoes.iter().map(|e| e.count.received).sum::<u64>().to_string(),
    );
    put("mixed.frames_sent", total(&|p| p.frames).to_string());
    put("mixed.frame_bytes_sent", total(&|p| p.frame_bytes).to_string());
    put(
        "mixed.echo_frame_bytes_received",
        echoes.iter().map(|e| e.count.bytes).sum::<u64>().to_string(),
    );
    put("mixed.bg.passes", passes.to_string());
    put("mixed.bg.busy_passes", busy_passes.to_string());
    put("mixed.bg.busy_ns", clock.delta_ns(0, busy_ticks).to_string());
    put("mixed.bg.loop_ns", clock.delta_ns(0, loop_ticks).to_string());
    put("mixed.bg.allocs", allocs.to_string());
    put("mixed.bg.max_stream", max.1.to_string());
    let measured: u64 = total(&|p| p.measured_sent);
    let achieved = if c.duration_s == 0 {
        0.0
    } else {
        measured as f64 / c.duration_s as f64
    };
    finished.ok = true;
    Ok(PingBackground {
        hist,
        settings: st,
        achieved,
    })
}

fn poll_echoes(
    echoes: &mut [EchoRx],
    profile: &Profile,
    clock: &TscClock,
    hist: &mut Latency,
    measured_from: u64,
    max: &mut (u64, u32),
    mtu: usize,
) -> Result<usize, Error> {
    let mut read = 0usize;
    for (k, e) in echoes.iter_mut().enumerate() {
        let mut ctx = PingCtx {
            clock,
            stream: &profile.streams[k],
            count: &mut e.count,
            hist,
            measured_from,
            max,
            mtu,
        };
        let n = e
            .asm
            .poll(&e.sub, &mut ctx, on_echo, POLL_LIMIT)
            .map_err(aeron_err("polling a background echo"))?;
        read += usize::try_from(n).unwrap_or(0);
    }
    Ok(read)
}

/// One subscription of the pong's.
struct SubRx {
    sub: AeronSubscription,
    asm: AeronFragmentClosureAssembler,
    count: Count,
}

/// One background stream on the pong: its subscriptions (the first
/// echoes), its echo publication.
struct PongStream {
    subs: Vec<SubRx>,
    echo: AeronExclusivePublication,
    echoed: u64,
    echo_frames: u64,
    echo_frame_bytes: u64,
    echo_bp_spins: u64,
}

/// What the pong's callback records into, and echoes with.
struct PongCtx<'a> {
    stream: &'a Stream,
    count: &'a mut Count,
    echo: Option<(&'a AeronExclusivePublication, &'a mut [u8; MAX_SIZE])>,
    echoed: &'a mut u64,
    echo_frames: &'a mut u64,
    echo_frame_bytes: &'a mut u64,
    bp_spins: &'a mut u64,
    mtu: usize,
    failed: &'a mut Option<Error>,
}

fn on_ping(ctx: &mut PongCtx<'_>, payload: &[u8], _header: AeronHeader) {
    if ctx.failed.is_some() {
        return;
    }
    let h = match check(payload, PING, ctx.stream) {
        Ok(h) => h,
        Err(why) => return ctx.count.refuse(why),
    };
    ctx.count
        .on(h.publisher, h.seq, frame_bytes(payload.len(), ctx.mtu) as u64);
    let Some((publication, buf)) = ctx.echo.as_mut() else {
        return;
    };
    let n = payload.len();
    buf[..n].copy_from_slice(payload);
    buf[28..30].copy_from_slice(&ECHO.to_le_bytes());
    let mut spins = 0u64;
    loop {
        match publication.offer(&buf[..n]) {
            Ok(_) => break,
            Err(e) if e.is_retryable() => {
                spins += 1;
                // the ping has gone: give up on an echo nobody will read
                if spins.is_multiple_of(1 << 20) && super::STOP.load(Ordering::Relaxed) {
                    *ctx.failed = Some(Error::Transport(
                        "stopped while a background echo was back-pressured".into(),
                    ));
                    return;
                }
                std::hint::spin_loop();
            }
            Err(e) => {
                *ctx.failed = Some(Error::Transport(format!(
                    "background echo on stream {}: {e:?}",
                    h.stream
                )));
                return;
            }
        }
    }
    *ctx.bp_spins += spins;
    *ctx.echoed += 1;
    *ctx.echo_frames += frames(n, ctx.mtu) as u64;
    *ctx.echo_frame_bytes += frame_bytes(n, ctx.mtu) as u64;
}

/// The pong's background, as it ended.
pub struct PongBackground {
    /// Its manifest settings.
    pub settings: BTreeMap<String, String>,
}

/// The pong's background thread: its own client of the driver at `a.dir`,
/// every background stream's subscriptions (fan-out) and echo publication;
/// `gate` ready once they are added (the hot thread then says `ready`);
/// then, pinned, reads every subscription and echoes the first's, until
/// SIGTERM and [`QUIET`] after it.
///
/// # Errors
/// The transport's, a payload refused, or [`Error::Timeout`].
#[allow(clippy::too_many_lines)]
pub fn pong(c: &Common, a: &Aeronish, profile: &Profile, gate: &Gate) -> Result<PongBackground, Error> {
    let mut finished = Finished { gate, ok: false };
    let client = Client::of(&a.dir)?;
    let mtu = a.mtu as usize;
    let mut streams = Vec::new();
    for s in &profile.streams {
        let (_, sub_uri, echo_pub, _, id, eid) = channel(a, s)?;
        let mut subs = Vec::new();
        for _ in 0..s.fanout {
            subs.push(SubRx {
                sub: subscription(&client, &sub_uri, id)?,
                asm: AeronFragmentClosureAssembler::new().map_err(aeron_err("a fragment assembler"))?,
                count: Count::default(),
            });
        }
        let echo = publication(&client, &echo_pub, eid)?;
        if largest_message(&echo)? < MAX_SIZE {
            return Err(Error::Usage(format!(
                "{echo_pub}: its largest message under {MAX_SIZE} B"
            )));
        }
        streams.push(PongStream {
            subs,
            echo,
            echoed: 0,
            echo_frames: 0,
            echo_frame_bytes: 0,
            echo_bp_spins: 0,
        });
    }
    gate.ready.store(true, Ordering::Release);
    {
        let ps: Vec<&AeronExclusivePublication> = streams.iter().map(|s| &s.echo).collect();
        let ss: Vec<(&AeronSubscription, i32)> = streams
            .iter()
            .zip(&profile.streams)
            .flat_map(|(st, s)| {
                st.subs
                    .iter()
                    .map(move |r| (&r.sub, i32::try_from(s.publishers).unwrap_or(1)))
            })
            .collect();
        connected(&client, &ps, &ss, a.connect_timeout, gate)?;
    }
    point::pin(c.cpu(1))?;
    let mut buf = [0u8; MAX_SIZE];
    let mut failed = None;
    let a0 = thread_allocs();
    let (mut stopped, mut last_read) = (None::<Instant>, Instant::now());
    let mut idle = 0u32;
    loop {
        let mut read = 0usize;
        for (k, st) in streams.iter_mut().enumerate() {
            let PongStream {
                subs,
                echo,
                echoed,
                echo_frames,
                echo_frame_bytes,
                echo_bp_spins,
            } = st;
            for (j, r) in subs.iter_mut().enumerate() {
                let mut ctx = PongCtx {
                    stream: &profile.streams[k],
                    count: &mut r.count,
                    echo: (j == 0).then_some((&*echo, &mut buf)),
                    echoed,
                    echo_frames,
                    echo_frame_bytes,
                    bp_spins: echo_bp_spins,
                    mtu,
                    failed: &mut failed,
                };
                let n = r
                    .asm
                    .poll(&r.sub, &mut ctx, on_ping, POLL_LIMIT)
                    .map_err(aeron_err("polling a background subscription"))?;
                read += usize::try_from(n).unwrap_or(0);
            }
        }
        if let Some(e) = failed.take() {
            return Err(e);
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
        if stopped.is_none() && super::STOP.load(Ordering::Relaxed) {
            stopped = Some(now);
        }
        if let Some(t) = stopped
            && (now.duration_since(last_read) >= QUIET || now.duration_since(t) >= DRAIN_MAX)
        {
            break;
        }
    }
    let allocs = thread_allocs() - a0;
    let refused: u64 = streams
        .iter()
        .flat_map(|s| s.subs.iter())
        .map(|r| r.count.refused)
        .sum();
    let mut st = BTreeMap::new();
    let mut put = |k: &str, v: String| {
        st.insert(k.to_owned(), v);
    };
    let join = |f: &dyn Fn(&PongStream) -> String| streams.iter().map(f).collect::<Vec<_>>().join(",");
    put(
        "mixed.recv",
        join(&|s| {
            s.subs
                .iter()
                .map(|r| r.count.received.to_string())
                .collect::<Vec<_>>()
                .join("/")
        }),
    );
    put(
        "mixed.recv_gaps",
        join(&|s| s.subs.iter().map(|r| r.count.gaps).sum::<u64>().to_string()),
    );
    put(
        "mixed.recv_behind",
        join(&|s| s.subs.iter().map(|r| r.count.behind).sum::<u64>().to_string()),
    );
    put(
        "mixed.recv_refused",
        join(&|s| s.subs.iter().map(|r| r.count.refused).sum::<u64>().to_string()),
    );
    put("mixed.echoed", join(&|s| s.echoed.to_string()));
    put(
        "mixed.echoed_total",
        streams.iter().map(|s| s.echoed).sum::<u64>().to_string(),
    );
    put(
        "mixed.recv_total",
        streams
            .iter()
            .flat_map(|s| s.subs.iter())
            .map(|r| r.count.received)
            .sum::<u64>()
            .to_string(),
    );
    put(
        "mixed.frame_bytes_sent",
        streams.iter().map(|s| s.echo_frame_bytes).sum::<u64>().to_string(),
    );
    put(
        "mixed.frames_sent",
        streams.iter().map(|s| s.echo_frames).sum::<u64>().to_string(),
    );
    // what crossed the wire: each stream once (its first subscription's),
    // however many read it here
    put(
        "mixed.frame_bytes_received",
        streams
            .iter()
            .filter_map(|s| s.subs.first())
            .map(|r| r.count.bytes)
            .sum::<u64>()
            .to_string(),
    );
    put(
        "mixed.echo_bp_spins",
        streams.iter().map(|s| s.echo_bp_spins).sum::<u64>().to_string(),
    );
    put("mixed.bg.allocs", allocs.to_string());
    put(
        "mixed.recv_first_refused",
        streams
            .iter()
            .flat_map(|s| s.subs.iter())
            .find_map(|r| r.count.first_bad)
            .map_or_else(|| "none".to_owned(), |b| format!("{b:?}")),
    );
    put("mixed.recv_refused_total", refused.to_string());
    finished.ok = true;
    Ok(PongBackground { settings: st })
}

/// The driver's counters a mixed point reads beyond the loss and traffic
/// ones (by label prefix): its agents' duty cycles (their longest, a max
/// since the driver started; and the cycles over the driver's threshold,
/// `AERON_DRIVER_*_CYCLE_THRESHOLD`), flow control and control traffic.
const DRIVER_COUNTERS: [(&str, &str); 15] = [
    (
        "Conductor max cycle time doing its work in ns",
        "cycle.conductor.max_ns",
    ),
    ("Sender max cycle time doing its work in ns", "cycle.sender.max_ns"),
    ("Receiver max cycle time doing its work in ns", "cycle.receiver.max_ns"),
    (
        "Conductor work cycle exceeded threshold count",
        "cycle.conductor.exceeded",
    ),
    ("Sender work cycle exceeded threshold count", "cycle.sender.exceeded"),
    (
        "Receiver work cycle exceeded threshold count",
        "cycle.receiver.exceeded",
    ),
    ("Sender flow control limits", "sender-flow-control-limits"),
    ("Status Messages sent", "status-messages-sent"),
    ("Status Messages received", "status-messages-received"),
    ("Heartbeats sent", "heartbeats-sent"),
    ("Heartbeats received", "heartbeats-received"),
    ("Flow control under runs", "flow-control-under-runs"),
    ("Flow control over runs", "flow-control-over-runs"),
    ("Invalid packets", "invalid-packets"),
    ("Retransmitted bytes", "retransmitted-bytes"),
];

/// md-dpdk's transport and NIC counters a mixed point reads (DPDK only):
/// a label suffix and its key. The transport's frames and bytes each way
/// (live), its drops, and the PMD's ENA allowance counters (refreshed
/// about once a second: the point waits that long before its last read).
const NIC_COUNTERS: [(&str, &str); 9] = [
    (" dir=tx pkts", "tx_pkts"),
    (" dir=tx bytes", "tx_bytes"),
    (" dir=rx pkts", "rx_pkts"),
    (" dir=rx bytes", "rx_bytes"),
    (" nic-rx-missed", "rx_missed"),
    (" nic-rx-nombuf", "rx_nombuf"),
    (" queue-drop", "queue_drop"),
    (" tx-eagain", "tx_eagain"),
    ("allowance_exceeded", "allowance_exceeded"),
];

/// The extra counters now, summed by key ([`DRIVER_COUNTERS`]; under DPDK
/// [`NIC_COUNTERS`] as `nic.<key>`).
pub fn counters_now(aeron: &Aeron) -> BTreeMap<String, i64> {
    let mut out: BTreeMap<String, i64> = BTreeMap::new();
    aeron
        .counters_reader()
        .foreach_counter_fn(|value, _id, _type, _key, label| {
            if let Some((_, k)) = DRIVER_COUNTERS.iter().find(|(prefix, _)| label.starts_with(prefix)) {
                let e = out.entry((*k).to_owned()).or_insert(0);
                // a max is one counter, never summed
                if k.ends_with("max_ns") {
                    *e = (*e).max(value);
                } else {
                    *e += value;
                }
            }
            if let Some((_, k)) = NIC_COUNTERS.iter().find(|(suffix, _)| label.ends_with(suffix)) {
                // the transport's own (`rusteron-dpdk ...`) or the PMD's xstats (bare names)
                if label.starts_with("rusteron-dpdk") || !label.contains(' ') {
                    *out.entry(format!("nic.{k}")).or_insert(0) += value;
                }
            }
        });
    out
}

/// Each extra counter into the manifest: a max as it is after (and before,
/// `<key>_before`: the driver's since it started), every other its growth.
pub fn record_counters(m: &mut RunManifest, before: &BTreeMap<String, i64>, after: &BTreeMap<String, i64>) {
    for (k, v) in after {
        let was = before.get(k).copied().unwrap_or(0);
        let key = if k.starts_with("nic.") {
            k.clone()
        } else {
            format!("driver.{k}")
        };
        if k.ends_with("max_ns") {
            m.setting(&key, &v.to_string())
                .setting(&format!("{key}_before"), &was.to_string());
        } else {
            m.setting(&key, &(v - was).to_string());
        }
    }
}

/// Settings into the manifest.
pub fn put_all(m: &mut RunManifest, settings: &BTreeMap<String, String>) {
    for (k, v) in settings {
        m.setting(k, v);
    }
}

/// The schedule's start for the hot thread: once `gate`'s background is
/// ready (the duty run while waiting), `START_DELAY_NS` ahead; told to the
/// background.
///
/// # Errors
/// [`Error::Peer`] when the background failed.
pub fn start(client: &Client, clock: &TscClock, gate: &Gate) -> Result<(u64, u128), Error> {
    while !gate.ready()? {
        client.rest(Duration::from_millis(1))?;
    }
    let (t, rt) = point::start_at(clock, START_DELAY_NS);
    gate.start(t, rt);
    Ok((t, rt))
}

/// Under `--driver invoker` the hot thread runs the driver: it goes on
/// running it, without resting, until the background is done.
///
/// # Errors
/// The driver's.
pub fn drive_until_done(client: &Client, gate: &Gate) -> Result<(), Error> {
    if client.invoked.is_none() {
        return Ok(());
    }
    while !gate.done() {
        client.duty()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn the_mix_shares_the_background_rate_by_weight() -> Result<(), Error> {
        for (streams, rate) in [(32, 50_000), (32, 200_000), (128, 50_000), (128, 200_000)] {
            let p = Profile::new(streams, rate, 10_000)?;
            assert_eq!(p.streams.len(), streams as usize - 1);
            let want = (rate - 10_000) as f64;
            assert!(
                (p.bg_rate() - want).abs() / want < 0.001,
                "{streams} {rate}: {}",
                p.bg_rate()
            );
            assert_eq!(p.channels, (streams - 1).div_ceil(4));
            // every class and fan-out present
            for l in ['S', 'B', 'I', 'M'] {
                assert!(p.streams.iter().any(|s| s.class.letter() == l));
            }
            for f in 1..=4 {
                assert!(p.streams.iter().any(|s| s.fanout == f));
            }
        }
        Ok(())
    }

    #[test]
    fn sizes_follow_the_class_mix() -> Result<(), Error> {
        let p = Profile::new(32, 50_000, 10_000)?;
        for s in &p.streams {
            let mut n = [0u64; 4];
            for seq in 0..20_000 {
                let len = s.size(0, seq);
                n[SIZES.iter().position(|&x| x == len).unwrap_or(0)] += 1;
            }
            for (got, want) in n.iter().zip(s.class.sizes()) {
                let want = want * 20;
                assert!(got.abs_diff(want) <= want / 10 + 40, "{:?}: {n:?}", s.class);
            }
        }
        Ok(())
    }

    #[test]
    fn frames_at_the_lab_mtu() {
        assert_eq!(frame_bytes(64, 1408), 96);
        assert_eq!(frame_bytes(288, 1408), 320);
        assert_eq!(frame_bytes(1344, 1408), 1376);
        assert_eq!(frames(1344, 1408), 1);
        assert_eq!(frame_bytes(4096, 1408), 1408 + 1408 + 1376);
        assert_eq!(frames(4096, 1408), 3);
    }

    #[test]
    fn schedule_counts_what_falls_in_the_window() -> Result<(), Error> {
        let p = Profile::new(32, 50_000, 10_000)?;
        for s in &p.streams {
            for pb in 0..s.publishers {
                let total = s.total(pb, 10_000_000_000);
                assert!(total.is_multiple_of(s.burst));
                assert!(total == 0 || s.due_ns(pb, total - 1) < 10_000_000_000);
                assert!(s.due_ns(pb, total) >= 10_000_000_000);
            }
        }
        Ok(())
    }

    #[test]
    fn a_message_checks_and_a_corrupt_one_does_not() -> Result<(), Error> {
        let p = Profile::new(32, 50_000, 10_000)?;
        let s = &p.streams[7];
        let mut buf = [0u8; MAX_SIZE];
        let len = s.size(1, 5);
        let h = Header {
            seq: 5,
            intended: 99,
            stream: s.index,
            publisher: 1,
        };
        let n = encode(&mut buf, h, 7, len);
        let got = check(&buf[..n], PING, s).map_err(|b| Error::Transport(format!("{b:?}")))?;
        assert_eq!((got.seq, got.intended, got.publisher), (5, 99, 1));
        assert_eq!(check(&buf[..n], ECHO, s).err(), Some(Bad::Kind));
        assert_eq!(check(&buf[..n - 1], PING, s).err(), Some(Bad::Length));
        let mut bad = buf;
        bad[n - 1] ^= 1;
        assert_eq!(check(&bad[..n], PING, s).err(), Some(Bad::Tail));
        assert_eq!(check(&buf[..n], PING, &p.streams[0]).err(), Some(Bad::Stream));
        Ok(())
    }
}
