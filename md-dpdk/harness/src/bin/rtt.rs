//! Cross-host round trip through an external media driver (`AERON_DIR`): the
//! kernel-UDP driver or the DPDK one, the client code identical. Ported from
//! rusteron's x86 lab (`scripts/x86-lab/harness/src/bin/rtt.rs`), cross-host
//! modes only, so the results compare with rusteron's published UDP table.
//!
//! Pong runs on the peer and ping here (endpoints are host:port, ping's on the
//! pong host and pong's on the ping host):
//! rtt xpong <ping endpoint> <pong endpoint> <pong cpu|->
//! rtt xping <ping endpoint> <pong endpoint> <messages> <warmup> <ping cpu|->
//! xping prints: rtt,<label>,udp,p50,p90,p99,p99.9,p99.99,max,mean (ns),samples
//!
//! Endpoints on 127.0.0.1 with both on one host measure a localhost round trip.
//! `rtt idle <endpoint> <seconds>` holds a subscription that receives nothing,
//! to measure what an idle host-local channel costs the driver's agents.
//!
//! A slow configuration stops early: warmup after a quarter of `RTT_SECONDS`
//! (default 20) and the measured run after all of it. `RTT_GAP_US=<us>` paces
//! the pings, one every that many microseconds (`roundtrip::Pace`), the
//! driver run and the echo's image polled in between.
//!
//! Round trips are stamped with quanta's clock, its ordered read at both ends
//! (`lab::Clock::stamp`), in nanoseconds. With
//! `RTT_HISTOGRAM=<path>` xping also writes the full histogram there, one
//! `value_ns,count` row per recorded value. With `LOAD_SPIKES=<path>` it writes
//! every measured round trip of at least `LOAD_SPIKE_NS` (default 100000) as
//! `stream,due_ns,latency_ns,ping` (`due_ns`: when it was sent, from the start of the
//! measured run, whose `CLOCK_REALTIME` the first line gives as
//! `# start_realtime_ns=`; `ping`: its index in the measured run, from 0), the
//! first 100,000 of them with `# spikes_total=`, as `load` does.
//! With `LOOP_STALLS_DIR=<dir>`, xping and xpong also write there every stall
//! of their own polling loop of at least `LOOP_STALL_NS` (default 20000), as
//! `stalls-ping.csv` / `stalls-pong.csv` (`end_ns,gap_ns` from the line
//! `# start_realtime_ns=`), so a spike can be told from a client that did not
//! run. xpong stops on SIGTERM, writing its stalls.
//!
//! Neither outlives its run. A back-pressured echo or ping retries only
//! while it can still make progress (`lab::Retry`): xpong gives the echo up,
//! and ends, on SIGTERM, once its ping's image closes, or once its
//! publication has been not connected for over a second; xping fails, its
//! pong gone, on the last two. xpong says on stderr why it ended.
//!
//! With a receive timestamp in each data frame header's reserved value
//! (`SUB_CHANNEL_PARAMS=media-rcv-ts-offset=reserved`: under the DPDK
//! transport's NIC RX timestamps, the NIC's arrival on `CLOCK_REALTIME`),
//! xpong echoes the ping's in the echo's bytes 16 to 23, and with
//! `RTT_ONE_WAY=<path>` xping writes there, for each spike (as `LOAD_SPIKES`
//! counts one), `ping,sent_realtime_ns,pong_rx_realtime_ns,ping_rx_realtime_ns,echoed_realtime_ns`
//! (the first 100,000, under `# start_realtime_ns=`): when it sent (its clock
//! taken onto `CLOCK_REALTIME` from the run's start), when the pong's host
//! and then its own received, and when the echo reached it. Two hosts'
//! clocks, so the lab's precision-time placement only (both on the PTP
//! hardware clock); a round trip without both stamps is not written.
//!
//! `HARNESS_DRIVER=invoker` embeds the media driver instead (`lab.rs`): the
//! measuring thread then also runs the client's conductor and the driver,
//! every pass of its loop. `rtt idle` measures an external driver's agents, so
//! it refuses it. `PUB_CHANNEL_PARAMS` is appended to every publication's
//! channel, `SUB_CHANNEL_PARAMS` to every subscription's.

#[path = "../lab.rs"]
mod lab;
#[path = "../roundtrip.rs"]
mod roundtrip;

use lab::{Clock, Duty, External, Halt, Lab, Retry, STOP, cpu_arg, pin, sub_channel};
use roundtrip::{Pace, Results, Stalls, env_u64, realtime_ns};
use rusteron_client::{AeronExclusivePublication, AeronImage};
use std::error::Error;
use std::ffi::{CStr, CString};
use std::fmt::Write as _;
use std::hint::spin_loop;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

const PING_STREAM_ID: i32 = 1002;
const PONG_STREAM_ID: i32 = 1003;
const MESSAGE_LENGTH: usize = 32;
const FRAGMENT_COUNT_LIMIT: usize = 10;
/// Where the pong puts the ping's receive timestamp in its echo: past the
/// send stamp and the sequence the ping reads.
const PONG_RX: usize = 16;
const ONE_WAY_CAPACITY: usize = 100_000;

/// What the ping knows of one echo: its send and echo stamps (the clients'
/// clock), and the receive timestamps of the ping on the pong's host and of
/// the echo on its own (`CLOCK_REALTIME` ns, 0 where none).
struct Echo {
    sent: u64,
    echoed: u64,
    pong_rx: i64,
    ping_rx: i64,
}

/// `RTT_ONE_WAY`: each spike's one-way split, preallocated.
struct OneWay {
    path: String,
    spike_ns: u64,
    t0: u64,
    t0_realtime: u128,
    rows: Vec<(usize, u128, i64, i64, u128)>,
}

impl OneWay {
    /// `None` unless `RTT_ONE_WAY` is set; starts the run's clock mapping.
    fn new(clock: &Clock) -> Result<Option<Self>, Box<dyn Error>> {
        let Some(path) = std::env::var("RTT_ONE_WAY").ok().filter(|p| !p.is_empty()) else {
            return Ok(None);
        };
        let spike_ns = env_u64("LOAD_SPIKE_NS", 100_000)?;
        Ok(Some(Self {
            path,
            spike_ns,
            t0: clock.now(),
            t0_realtime: realtime_ns(),
            rows: Vec::with_capacity(ONE_WAY_CAPACITY),
        }))
    }

    /// A stamp of the clients' clock on `CLOCK_REALTIME`.
    fn realtime(&self, stamp: u64) -> u128 {
        self.t0_realtime + u128::from(stamp.saturating_sub(self.t0))
    }

    #[inline]
    fn record(&mut self, ping: usize, echo: &Echo) {
        if echo.echoed.wrapping_sub(echo.sent) >= self.spike_ns
            && echo.pong_rx != 0
            && echo.ping_rx != 0
            && self.rows.len() < self.rows.capacity()
        {
            let (sent, echoed) = (self.realtime(echo.sent), self.realtime(echo.echoed));
            self.rows.push((ping, sent, echo.pong_rx, echo.ping_rx, echoed));
        }
    }

    fn write(&self) -> Result<(), Box<dyn Error>> {
        let mut rows = format!(
            "# start_realtime_ns={}\nping,sent_realtime_ns,pong_rx_realtime_ns,ping_rx_realtime_ns,echoed_realtime_ns\n",
            self.t0_realtime
        );
        for (ping, sent, pong_rx, ping_rx, echoed) in &self.rows {
            writeln!(rows, "{ping},{sent},{pong_rx},{ping_rx},{echoed}")?;
        }
        std::fs::write(&self.path, rows).map_err(|e| format!("RTT_ONE_WAY {}: {e}", self.path))?;
        Ok(())
    }
}

fn udp(endpoint: Option<String>) -> Result<CString, Box<dyn Error>> {
    let endpoint = endpoint.ok_or("an endpoint: host:port")?;
    Ok(CString::new(format!("aeron:udp?endpoint={endpoint}"))?)
}

#[allow(
    clippy::similar_names,
    reason = "ping and pong name the two directions of a round trip"
)]
fn main() -> Result<(), Box<dyn Error>> {
    lab::lock_memory()?;
    let mut args = std::env::args().skip(1);
    let mode = args.next().ok_or("xping|xpong|idle")?;
    if mode == "idle" {
        let channel = udp(args.next())?;
        let seconds = args.next().ok_or("seconds")?;
        return run_idle(
            &channel,
            seconds.parse().map_err(|e| format!("seconds {seconds:?}: {e}"))?,
        );
    }
    let (ping_channel, pong_channel) = (udp(args.next())?, udp(args.next())?);
    match mode.as_str() {
        "xpong" => {
            let cpu = cpu_arg(args.next())?;
            let lab = Lab::connect(cpu)?;
            lab.invoker().map_or_else(
                || run_pong(&lab, &External, &ping_channel, &pong_channel, cpu),
                |duty| run_pong(&lab, &duty, &ping_channel, &pong_channel, cpu),
            )
        }
        "xping" => {
            let number = |a: Option<String>| -> Result<usize, Box<dyn Error>> {
                let a = a.ok_or("messages and warmup")?;
                Ok(a.parse().map_err(|e| format!("{a:?}: a count: {e}"))?)
            };
            let (messages, warmup) = (number(args.next())?, number(args.next())?);
            let cpu = cpu_arg(args.next())?;
            let lab = Lab::connect(cpu)?;
            lab.invoker().map_or_else(
                || run_ping(&lab, &External, &ping_channel, &pong_channel, messages, warmup, cpu),
                |duty| run_ping(&lab, &duty, &ping_channel, &pong_channel, messages, warmup, cpu),
            )
        }
        other => Err(format!("unknown mode {other}: xping, xpong or idle").into()),
    }
}

#[allow(
    clippy::similar_names,
    reason = "ping and pong name the two directions of a round trip"
)]
fn run_ping(
    lab: &Lab,
    duty: &impl Duty,
    ping_channel: &CStr,
    pong_channel: &CStr,
    messages: usize,
    warmup: usize,
    cpu: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    let publication = lab.add_publication(ping_channel, PING_STREAM_ID)?;
    let subscription = lab.add_subscription(&sub_channel(pong_channel)?, PONG_STREAM_ID)?;
    lab.wait_until("pong", 30, || {
        publication.is_connected() && subscription.image_at_index(0).is_some()
    })?;
    let image = subscription.image_at_index(0).ok_or("the pong's image went")?;
    pin(cpu)?;
    let clock = Clock::calibrate();

    let budget = Duration::from_secs(env_u64("RTT_SECONDS", 20)?);
    let mut results = Results::new()?;
    let mut buffer = [0u8; MESSAGE_LENGTH];
    // RTT_LOST_TIMEOUT_US: on an unreliable channel a lost ping or echo is
    // never repaired, so after this long it is counted lost and the next sent
    let timeout_ns = env_u64("RTT_LOST_TIMEOUT_US", 0)?.saturating_mul(1000);
    let mut seq = 0_u64;
    let mut pace = Pace::from_env()?;
    let start = Instant::now();
    for i in 0..warmup {
        seq += 1;
        round_trip(
            duty,
            &clock,
            &publication,
            &image,
            &mut buffer,
            &mut None,
            &mut pace,
            seq,
            timeout_ns,
            |_: &Echo| {},
        )?;
        if (pace.paced() || i % 64 == 0) && start.elapsed() > budget / 4 {
            break;
        }
    }
    let start = Instant::now();
    results.start(&clock);
    let mut one_way = OneWay::new(&clock)?;
    let mut stalls = Stalls::new(&clock)?;
    let mut lost = 0_u64;
    for i in 0..messages {
        seq += 1;
        let echoed = round_trip(
            duty,
            &clock,
            &publication,
            &image,
            &mut buffer,
            &mut stalls,
            &mut pace,
            seq,
            timeout_ns,
            |echo: &Echo| {
                results.record(&clock, echo.echoed.wrapping_sub(echo.sent), i);
                if let Some(one_way) = one_way.as_mut() {
                    one_way.record(i, echo);
                }
            },
        )?;
        if !echoed {
            lost += 1;
        }
        if (pace.paced() || i % 64 == 0) && start.elapsed() > budget {
            break;
        }
    }

    results.report(lost, PING_STREAM_ID)?;
    if let Some(one_way) = &one_way {
        one_way.write()?;
    }
    if let Some(stalls) = &stalls {
        stalls.write("stalls-ping.csv")?;
    }
    Ok(())
}

/// Sends one stamped ping and polls until pong's echo of it is back.
#[inline]
/// One round trip of message `seq`: `true` once its echo is back, `false` if
/// none came within `timeout_ns` (0: wait for ever, as a reliable channel
/// repairs any loss). An echo of an earlier, timed-out message is skipped.
/// `duty` runs every pass (an embedded driver sends the ping and receives the
/// echo in it). A paced ping waits for its time (`pace`) first, the loop
/// running as it does for an echo, anything the image has drained.
#[allow(clippy::too_many_arguments)]
fn round_trip(
    duty: &impl Duty,
    clock: &Clock,
    publication: &AeronExclusivePublication,
    image: &AeronImage,
    buffer: &mut [u8; MESSAGE_LENGTH],
    stalls: &mut Option<Stalls>,
    pace: &mut Pace,
    seq: u64,
    timeout_ns: u64,
    mut record: impl FnMut(&Echo),
) -> Result<bool, Box<dyn Error>> {
    if let Some(s) = stalls.as_mut() {
        s.pass(clock.now());
    }
    while !pace.ready(clock.now()) {
        if let Some(s) = stalls.as_mut() {
            s.pass(clock.now());
        }
        let work = duty.run();
        let fragments = image.poll_fn(|_, _| {}, FRAGMENT_COUNT_LIMIT)?;
        if fragments == 0 && work == 0 {
            if image.is_closed() {
                return Err(pong_went_away());
            }
            spin_loop();
        }
    }
    buffer[8..16].copy_from_slice(&seq.to_le_bytes());
    let mut retry = Retry::default();
    let started = loop {
        // stamped on each attempt, so time back-pressured is not counted
        let now = clock.stamp();
        buffer[..8].copy_from_slice(&now.to_le_bytes());
        match publication.offer(buffer) {
            Ok(_) => {
                pace.sent(now);
                break now;
            }
            Err(e) if e.is_retryable() => {
                if let Err(halt) = retry.check(publication, image) {
                    return Err(pong_gone(&halt));
                }
                if duty.run() == 0 {
                    spin_loop();
                }
            }
            Err(e) => return Err(format!("offer: {e:?}").into()),
        }
    };
    let mut echoed = false;
    while !echoed {
        if let Some(s) = stalls.as_mut() {
            s.pass(clock.now());
        }
        let work = duty.run();
        let fragments = image.poll_fn(
            |reply, header| {
                if lab::le_u64(reply, 8) == seq {
                    let sent = lab::le_u64(reply, 0);
                    let at = clock.stamp();
                    record(&Echo {
                        sent,
                        echoed: at,
                        pong_rx: lab::le_u64(reply, PONG_RX).cast_signed(),
                        ping_rx: header.reserved_value().unwrap_or(0),
                    });
                    echoed = true;
                }
            },
            FRAGMENT_COUNT_LIMIT,
        )?;
        if fragments == 0 && work == 0 {
            if image.is_closed() {
                return Err(pong_went_away());
            }
            if timeout_ns != 0 && clock.now().wrapping_sub(started) > timeout_ns {
                return Ok(false);
            }
            spin_loop();
        }
    }
    Ok(true)
}

/// The ping's end when its pong's image closes while it waits.
#[cold]
#[inline(never)]
fn pong_went_away() -> Box<dyn Error> {
    "pong went away".into()
}

/// The ping's end when its pong is gone while it sends: it fails, as it
/// does when the pong's image closes while it waits for an echo.
#[cold]
#[inline(never)]
fn pong_gone(halt: &Halt) -> Box<dyn Error> {
    format!("pong went away: {halt} while the ping was back-pressured").into()
}

/// Echoes every ping until the ping side's image goes away, SIGTERM, or an
/// echo given up (`lab::echo`: SIGTERM, the ping's image closed, or this
/// publication not connected for over a second); says why on stderr.
#[allow(
    clippy::similar_names,
    reason = "ping and pong name the two directions of a round trip"
)]
fn run_pong(
    lab: &Lab,
    duty: &impl Duty,
    ping_channel: &CStr,
    pong_channel: &CStr,
    cpu: Option<usize>,
) -> Result<(), Box<dyn Error>> {
    let publication = lab.add_publication(pong_channel, PONG_STREAM_ID)?;
    let subscription = lab.add_subscription(&sub_channel(ping_channel)?, PING_STREAM_ID)?;
    lab.wait_until("ping", 60, || subscription.image_at_index(0).is_some())?;
    let image = subscription.image_at_index(0).ok_or("the ping's image went")?;
    pin(cpu)?;
    lab::stop_on_sigterm();
    let clock = Clock::calibrate();
    let mut stalls = Stalls::new(&clock)?;
    let mut halt = None;
    let mut stamped = [0u8; MESSAGE_LENGTH];
    while halt.is_none() && !image.is_closed() && !STOP.load(Ordering::Relaxed) {
        if let Some(s) = stalls.as_mut() {
            s.pass(clock.now());
        }
        let work = duty.run();
        let fragments = image.poll_fn(
            |ping, header| {
                // the ping's receive timestamp, where its frame has one,
                // into the echo (xping's one-way split)
                let echo = match header.reserved_value() {
                    Some(rx) if rx != 0 && ping.len() == MESSAGE_LENGTH => {
                        stamped.copy_from_slice(ping);
                        stamped[PONG_RX..PONG_RX + 8].copy_from_slice(&rx.to_le_bytes());
                        &stamped[..]
                    }
                    _ => ping,
                };
                if halt.is_none()
                    && let Err(why) = lab::echo(duty, &publication, &image, echo)
                {
                    halt = Some(why);
                }
            },
            FRAGMENT_COUNT_LIMIT,
        )?;
        if fragments == 0 && work == 0 {
            spin_loop();
        }
    }
    if let Some(stalls) = &stalls {
        stalls.write("stalls-pong.csv")?;
    }
    let why = match halt {
        Some(Halt::Failed(why)) => return Err(format!("rtt: pong failed: {why}").into()),
        Some(Halt::Stopped) => "SIGTERM",
        None if STOP.load(Ordering::Relaxed) => "SIGTERM",
        Some(Halt::ImageClosed) | None => "its ping's image closed",
        Some(Halt::NotConnected) => "its publication not connected for over 1 s, its echoes unread",
    };
    eprintln!("rtt: pong ended: {why}");
    Ok(())
}

/// Holds a subscription on `channel` that nobody publishes to, for `seconds`:
/// what an idle host-local channel costs an external driver's agents. An
/// embedded driver would measure nothing of the kind, so it is refused.
fn run_idle(channel: &CStr, seconds: u64) -> Result<(), Box<dyn Error>> {
    if lab::embedded() {
        return Err(
            "rtt idle measures an external driver's agents: HARNESS_DRIVER=invoker would embed one, \
                    which nothing else uses"
                .into(),
        );
    }
    let lab = Lab::connect(None)?;
    let _subscription = lab.add_subscription(channel, PONG_STREAM_ID + 100)?;
    eprintln!("rtt: idle subscription on {}", channel.to_string_lossy());
    std::thread::sleep(Duration::from_secs(seconds));
    Ok(())
}
