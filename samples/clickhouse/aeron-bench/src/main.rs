//! Aeron round-trip latency under each driver threading mode, idle strategy
//! and CPU policy: `ping` sends quote-sized messages, `pong` echoes them, and
//! `ping` prints the round trip's percentiles as one JSON line.
//!
//! ```text
//! aeron-bench pong [options]
//! aeron-bench ping [options] [--count N] [--warmup N] [--rate MSGS_PER_S] [--size BYTES] [--record yes]
//! aeron-bench replay [options]
//! ```
//!
//! `--record yes` has the node's archive (on the driver at `AERON_DIR`, local
//! control channel) record the ping stream while it is timed: what recording
//! costs the hot path. `replay` then replays the newest recording of that
//! stream as fast as the archive serves it and prints the time to its first
//! message and the catch-up rate: what an engine's resync sees.
//!
//! Options (both):
//!
//! * `--ping CHANNEL` / `--pong CHANNEL`: where pings and echoes travel
//!   (default `aeron:ipc`, streams 1001 and 1002).
//! * `--idle spin|yield|backoff|sleep`: what the polling thread does when there
//!   is no work (default `spin`).
//! * `--driver external|dedicated|shared-network|shared|invoker`: `external`
//!   (default) uses the driver at `AERON_DIR`; the others start one in this
//!   process with that threading mode. `invoker` has no driver thread at all:
//!   this process's polling loop runs the driver's duty cycle. Idle
//!   strategies of a started driver come from `AERON_*_IDLE_STRATEGY`; it
//!   uses `AERON_DIR` when set, so the other side can join it as `external`.
//! * `--label TEXT`: copied into the JSON line.
//!
//! `--rate 0` (the default) is closed loop: one ping in flight, the pure round
//! trip. A positive rate is open loop: pings leave on a fixed schedule whether
//! or not echoes came back, and each is timed from when it was due, not when
//! it left, so a stall shows up in the tail instead of hiding (coordinated
//! omission).

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use std::ffi::CString;
use std::time::{Duration, Instant};

use hdrhistogram::Histogram;
use rusteron_archive::{
    Aeron, AeronArchive, AeronArchiveAsyncConnect, AeronArchiveContext, AeronArchiveReplayParams,
    AeronContext, AeronPublication, AeronSubscription, Handlers, SOURCE_LOCATION_LOCAL,
};
use rusteron_media_driver::bindings::aeron_threading_mode_t;
use rusteron_media_driver::{AeronDriver, AeronDriverContext};

type Error = Box<dyn std::error::Error>;

/// What the polling thread does with no work.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Idle {
    Spin,
    Yield,
    /// Spin, then yield, then sleep with exponential backoff up to 1 ms.
    Backoff,
    /// Sleep 1 ms: the lab's `IDLE=sleep`.
    Sleep,
}

impl Idle {
    fn parse(s: &str) -> Result<Self, Error> {
        Ok(match s {
            "spin" => Self::Spin,
            "yield" => Self::Yield,
            "backoff" => Self::Backoff,
            "sleep" => Self::Sleep,
            _ => return Err(format!("--idle {s}: expected spin, yield, backoff or sleep").into()),
        })
    }
}

/// The idle state kept between polls.
#[derive(Debug, Default)]
struct Waiter {
    misses: u32,
}

impl Waiter {
    fn work(&mut self) {
        self.misses = 0;
    }

    fn idle(&mut self, idle: Idle) {
        self.misses = self.misses.saturating_add(1);
        match idle {
            Idle::Spin => std::hint::spin_loop(),
            Idle::Yield => std::thread::yield_now(),
            Idle::Sleep => std::thread::sleep(Duration::from_millis(1)),
            Idle::Backoff => match self.misses {
                0..=100 => std::hint::spin_loop(),
                101..=200 => std::thread::yield_now(),
                n => std::thread::sleep(Duration::from_micros(
                    1u64 << (n - 200).min(10), // 2 µs .. ~1 ms
                )),
            },
        }
    }
}

#[derive(Debug)]
struct Options {
    ping: String,
    pong: String,
    idle: Idle,
    driver: String,
    label: String,
    count: u64,
    warmup: u64,
    rate: u64,
    size: usize,
    record: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, Error> {
        let mut o = Self {
            ping: "aeron:ipc".into(),
            pong: "aeron:ipc".into(),
            idle: Idle::Spin,
            driver: "external".into(),
            label: String::new(),
            count: 1_000_000,
            warmup: 100_000,
            rate: 0,
            size: 64,
            record: false,
        };
        let mut it = args.iter();
        while let Some(flag) = it.next() {
            let value = it.next().ok_or_else(|| format!("{flag} needs a value"))?;
            match flag.as_str() {
                "--ping" => o.ping.clone_from(value),
                "--pong" => o.pong.clone_from(value),
                "--idle" => o.idle = Idle::parse(value)?,
                "--driver" => o.driver.clone_from(value),
                "--label" => o.label.clone_from(value),
                "--count" => o.count = value.parse()?,
                "--warmup" => o.warmup = value.parse()?,
                "--rate" => o.rate = value.parse()?,
                "--size" => o.size = value.parse::<usize>()?.max(16),
                "--record" => o.record = value == "yes",
                _ => return Err(format!("unknown option {flag}").into()),
            }
        }
        Ok(o)
    }
}

/// A driver started in this process. For `invoker`, [`Driver::work`] runs its
/// duty cycle; the others run on their own threads.
struct Driver {
    invoked: Option<AeronDriver>,
    _threads: Option<rusteron_media_driver::EmbeddedMediaDriver>,
    dir: Option<CString>,
}

impl Driver {
    fn start(mode: &str) -> Result<Self, Error> {
        let threading = match mode {
            "external" => {
                return Ok(Self {
                    invoked: None,
                    _threads: None,
                    dir: None,
                });
            }
            "dedicated" => aeron_threading_mode_t::AERON_THREADING_MODE_DEDICATED,
            "shared-network" => aeron_threading_mode_t::AERON_THREADING_MODE_SHARED_NETWORK,
            "shared" => aeron_threading_mode_t::AERON_THREADING_MODE_SHARED,
            "invoker" => aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER,
            _ => return Err(format!(
                "--driver {mode}: expected external, dedicated, shared-network, shared or invoker"
            )
            .into()),
        };
        // Shared memory where there is one (Linux), else the temp directory.
        let base = if std::path::Path::new("/dev/shm").is_dir() {
            std::path::PathBuf::from("/dev/shm")
        } else {
            std::env::temp_dir()
        };
        let dir =
            CString::new(std::env::var("AERON_DIR").unwrap_or_else(|_| {
                format!("{}/aeron-bench-{}", base.display(), std::process::id())
            }))?;
        let context = AeronDriverContext::new()?;
        context.set_dir(&dir)?;
        context.set_dir_delete_on_start(true)?;
        context.set_dir_delete_on_shutdown(true)?;
        context.set_threading_mode(threading)?;
        if threading == aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER {
            let driver = AeronDriver::new(&context)?;
            driver.start(true)?;
            return Ok(Self {
                invoked: Some(driver),
                _threads: None,
                dir: Some(dir),
            });
        }
        Ok(Self {
            invoked: None,
            _threads: Some(AeronDriver::launch_embedded_guard(context, false)),
            dir: Some(dir),
        })
    }

    #[inline]
    fn work(&self) -> Result<(), Error> {
        if let Some(driver) = &self.invoked {
            driver.main_do_work()?;
        }
        Ok(())
    }
}

struct Link {
    aeron: Aeron,
    publication: AeronPublication,
    subscription: AeronSubscription,
}

fn connect(driver: &Driver, publish: (&str, i32), subscribe: (&str, i32)) -> Result<Link, Error> {
    let context = AeronContext::new()?;
    if let Some(dir) = &driver.dir {
        context.set_dir(dir)?;
    }
    let aeron = Aeron::new(&context)?;
    aeron.start()?;
    // Registered asynchronously while running the driver: an invoked driver
    // does nothing while this thread blocks, so a blocking add never returns.
    let sub_uri = CString::new(subscribe.0)?;
    let pending_sub =
        aeron.async_add_subscription(&sub_uri, subscribe.1, Handlers::NONE, Handlers::NONE)?;
    let pub_uri = CString::new(publish.0)?;
    let pending_pub = aeron.async_add_publication(&pub_uri, publish.1)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    let (mut subscription, mut publication) = (None, None);
    while subscription.is_none() || publication.is_none() {
        driver.work()?;
        if subscription.is_none() {
            subscription = pending_sub.poll()?;
        }
        if publication.is_none() {
            publication = pending_pub.poll()?;
        }
        if Instant::now() > deadline {
            return Err(
                "the driver did not register the publication and subscription in 10 s".into(),
            );
        }
        std::thread::yield_now();
    }
    let (Some(subscription), Some(publication)) = (subscription, publication) else {
        return Err("unreachable: both registered".into());
    };
    Ok(Link {
        aeron,
        publication,
        subscription,
    })
}

/// Offer until accepted, keeping an invoked driver running meanwhile.
#[inline]
fn send(link: &Link, driver: &Driver, message: &[u8]) -> Result<(), Error> {
    while link.publication.offer_raw(message, Handlers::NONE) < 0 {
        driver.work()?;
        std::hint::spin_loop();
    }
    Ok(())
}

/// The node archive's local control channel.
const CONTROL: &std::ffi::CStr = c"aeron:ipc?term-length=64k";
const PING_STREAM: i32 = 1001;
const REPLAY_STREAM: i32 = 2001;

fn archive(aeron: &Aeron) -> Result<AeronArchive, Error> {
    let context = AeronArchiveContext::new()?;
    context.set_aeron(aeron)?;
    context.set_control_request_channel(CONTROL)?;
    context.set_control_response_channel(CONTROL)?;
    Ok(AeronArchiveAsyncConnect::new_with_aeron(&context, aeron)?
        .poll_blocking(Duration::from_secs(10))?)
}

fn replay(o: &Options) -> Result<(), Error> {
    let driver = Driver::start(&o.driver)?;
    let link = connect(&driver, (&o.pong, 1002), (&o.ping, 1003))?;
    let archive = archive(&link.aeron)?;
    let mut newest = None;
    archive.list_recordings_fn(&mut 0, 0, i32::MAX, |d| {
        if d.stream_id() == PING_STREAM && d.stop_position() > d.start_position() {
            newest = Some((d.recording_id(), d.start_position(), d.stop_position()));
        }
    })?;
    let (id, start, stop) =
        newest.ok_or("no stopped recording of the ping stream: run ping --record yes first")?;
    let started = Instant::now();
    let params = AeronArchiveReplayParams::new(-1, -1, start, stop - start, -1, -1)?;
    let session = archive.start_replay(id, c"aeron:ipc", REPLAY_STREAM, &params)?;
    let channel = CString::new(format!("aeron:ipc?session-id={}", session as i32))?;
    let subscription = link.aeron.add_subscription(
        &channel,
        REPLAY_STREAM,
        Handlers::NONE,
        Handlers::NONE,
        Duration::from_secs(10),
    )?;
    let (mut messages, mut bytes, mut first) = (0u64, 0u64, None);
    // Padding advances the image position without reaching the fragment callback.
    // Retain the replay image before polling so its final position survives close.
    let mut image = None;
    let mut complete = false;
    let mut waiter = Waiter::default();
    while started.elapsed() < Duration::from_secs(120) {
        driver.work()?;
        if image.is_none() {
            image = subscription.image_by_session_id(session as i32);
        }
        let Some(image) = &image else {
            waiter.idle(o.idle);
            continue;
        };
        let n = image.poll_fn(
            |message, _| {
                first.get_or_insert_with(|| started.elapsed());
                messages += 1;
                // A message's frame: a 32-byte header, then the payload, padded to 32 bytes.
                bytes += u64::try_from((32 + message.len()).div_ceil(32) * 32).unwrap_or(0);
            },
            256,
        )?;
        if image.position() >= stop {
            complete = true;
            break;
        }
        if n > 0 {
            waiter.work();
        } else {
            waiter.idle(o.idle);
        }
    }
    if !complete {
        return Err("replay did not consume the full recording within 120 s".into());
    }
    let total = started.elapsed();
    println!(
        "{{\"kind\":\"replay\",\"idle\":\"{:?}\",\"messages\":{messages},\"bytes\":{bytes},\"first_ns\":{},\"total_ns\":{},\"msgs_per_s\":{:.0},\"mb_per_s\":{:.1}}}",
        o.idle,
        first.map_or(0, |f| f.as_nanos()),
        total.as_nanos(),
        messages as f64 / total.as_secs_f64(),
        bytes as f64 / total.as_secs_f64() / 1e6,
    );
    Ok(())
}

fn pong(o: &Options) -> Result<(), Error> {
    let driver = Driver::start(&o.driver)?;
    let link = connect(&driver, (&o.pong, 1002), (&o.ping, 1001))?;
    let mut echo = Vec::with_capacity(4096);
    let mut waiter = Waiter::default();
    eprintln!("pong ready");
    loop {
        driver.work()?;
        let mut got = false;
        link.subscription.poll_fn(
            |message, _| {
                echo.clear();
                echo.extend_from_slice(message);
                got = true;
            },
            1,
        )?;
        if got {
            send(&link, &driver, &echo)?;
            waiter.work();
        } else {
            waiter.idle(o.idle);
        }
    }
}

fn ping(o: &Options) -> Result<(), Error> {
    let driver = Driver::start(&o.driver)?;
    let link = connect(&driver, (&o.ping, 1001), (&o.pong, 1002))?;
    // Wait for the echo path: a closed loop stalls forever on a lost first ping.
    let started = Instant::now();
    while !link.publication.is_connected() || !link.subscription.is_connected() {
        driver.work()?;
        if started.elapsed() > Duration::from_secs(30) {
            return Err("pong never connected".into());
        }
        std::thread::yield_now();
    }
    // Recording runs beside the timed loop, from before the first ping.
    let archive = if o.record {
        let archive = archive(&link.aeron)?;
        let channel = CString::new(o.ping.as_str())?;
        // A recording a failed run left behind: stop it, so this run's is fresh.
        let _ = archive.stop_recording_channel_and_stream(&channel, PING_STREAM);
        archive.start_recording(&channel, PING_STREAM, SOURCE_LOCATION_LOCAL, false)?;
        std::thread::sleep(Duration::from_millis(500));
        Some(archive)
    } else {
        None
    };
    let mut histogram = Histogram::<u64>::new_with_bounds(1, 60_000_000_000, 3)?;
    let mut message = vec![0u8; o.size];
    let total = o.warmup + o.count;
    let interval = (o.rate > 0).then(|| Duration::from_nanos(1_000_000_000 / o.rate));
    let epoch = Instant::now();
    let (mut sent, mut received) = (0u64, 0u64);
    let mut waiter = Waiter::default();
    while received < total {
        driver.work()?;
        // Send: one in flight (closed loop), or everything now due (open loop).
        let due = match interval {
            None => (sent == received && sent < total).then(|| epoch.elapsed()),
            Some(every) => {
                let next = every * u32::try_from(sent).unwrap_or(u32::MAX);
                (sent < total && epoch.elapsed() >= next).then_some(next)
            }
        };
        let mut got = false;
        if let Some(at) = due {
            message[..8].copy_from_slice(&sent.to_le_bytes());
            message[8..16].copy_from_slice(&u64::try_from(at.as_nanos())?.to_le_bytes());
            // An overdue sender must still drain its reply queue. Blocking
            // here can fill both bounded channels and stall ping and pong.
            if link.publication.offer_raw(&message, Handlers::NONE) >= 0 {
                sent += 1;
                got = true;
            }
        }
        link.subscription.poll_fn(
            |echo, _| {
                let seq = u64::from_le_bytes(echo[..8].try_into().unwrap_or_default());
                let at = u64::from_le_bytes(echo[8..16].try_into().unwrap_or_default());
                // Timestamp receipt inside the callback: an echo may arrive
                // after the clock sampled before polling the subscription.
                let rtt = u64::try_from(epoch.elapsed().as_nanos())
                    .unwrap_or(u64::MAX)
                    .saturating_sub(at);
                if seq >= o.warmup {
                    histogram.saturating_record(rtt.max(1));
                }
                received += 1;
                got = true;
            },
            16,
        )?;
        if got {
            waiter.work();
        } else {
            waiter.idle(o.idle);
        }
    }
    // Stop recording, so the next run is neither recorded nor replayed with this one.
    if let Some(archive) = &archive {
        archive.stop_recording_channel_and_stream(&CString::new(o.ping.as_str())?, PING_STREAM)?;
    }
    let q = |p: f64| histogram.value_at_quantile(p);
    println!(
        "{{\"kind\":\"ping\",\"record\":{},\"label\":\"{}\",\"driver\":\"{}\",\"idle\":\"{:?}\",\"rate\":{},\"size\":{},\"count\":{},\"mean_ns\":{:.0},\"min_ns\":{},\"p50_ns\":{},\"p90_ns\":{},\"p99_ns\":{},\"p999_ns\":{},\"p9999_ns\":{},\"max_ns\":{}}}",
        o.record,
        o.label.replace('"', "'"),
        o.driver,
        o.idle,
        o.rate,
        o.size,
        histogram.len(),
        histogram.mean(),
        histogram.min(),
        q(0.5),
        q(0.9),
        q(0.99),
        q(0.999),
        q(0.9999),
        histogram.max()
    );
    Ok(())
}

fn main() -> Result<(), Error> {
    let args: Vec<String> = std::env::args().collect();
    let options = Options::parse(args.get(2..).unwrap_or_default())?;
    match args.get(1).map(String::as_str) {
        Some("ping") => ping(&options),
        Some("pong") => pong(&options),
        Some("replay") => replay(&options),
        _ => Err("usage: aeron-bench ping|pong|replay [options] (see the source's header)".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_default_to_a_closed_ipc_loop() -> Result<(), Error> {
        let o = Options::parse(&[])?;
        assert_eq!(
            (o.ping.as_str(), o.rate, o.idle),
            ("aeron:ipc", 0, Idle::Spin)
        );
        Ok(())
    }

    #[test]
    fn options_parse_and_reject() -> Result<(), Error> {
        let args: Vec<String> = ["--idle", "backoff", "--rate", "50000", "--size", "8"]
            .map(String::from)
            .into();
        let o = Options::parse(&args)?;
        assert_eq!((o.idle, o.rate, o.size), (Idle::Backoff, 50_000, 16));
        assert!(Options::parse(&["--idle".into(), "nap".into()]).is_err());
        assert!(Options::parse(&["--rate".into()]).is_err());
        Ok(())
    }

    #[test]
    fn backoff_reaches_its_one_millisecond_cap() {
        let mut waiter = Waiter { misses: 230 };
        let started = Instant::now();
        waiter.idle(Idle::Backoff);
        let slept = started.elapsed();
        assert!(slept >= Duration::from_micros(900), "{slept:?}");
    }
}
