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
//! (default 20) and the measured run after all of it.
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
use roundtrip::{Results, Stalls, realtime_ns};
use rusteron_client::*;
use std::ffi::{CStr, CString};
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
    fn new(clock: &Clock) -> Option<Self> {
        let path = std::env::var("RTT_ONE_WAY").ok().filter(|p| !p.is_empty())?;
        let spike_ns: u64 = std::env::var("LOAD_SPIKE_NS").map_or(100_000, |v| v.parse().expect("LOAD_SPIKE_NS"));
        Some(Self {
            path,
            spike_ns,
            t0: clock.now(),
            t0_realtime: realtime_ns(),
            rows: Vec::with_capacity(ONE_WAY_CAPACITY),
        })
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

    fn write(&self) {
        let mut rows = format!(
            "# start_realtime_ns={}\nping,sent_realtime_ns,pong_rx_realtime_ns,ping_rx_realtime_ns,echoed_realtime_ns\n",
            self.t0_realtime
        );
        for (ping, sent, pong_rx, ping_rx, echoed) in &self.rows {
            rows.push_str(&format!("{ping},{sent},{pong_rx},{ping_rx},{echoed}\n"));
        }
        std::fs::write(&self.path, rows).unwrap_or_else(|e| panic!("RTT_ONE_WAY {}: {e}", self.path));
    }
}

fn udp(endpoint: Option<String>) -> CString {
    CString::new(format!("aeron:udp?endpoint={}", endpoint.expect("host:port"))).unwrap()
}

fn main() {
    lab::lock_memory();
    let mut args = std::env::args().skip(1);
    let mode = args.next().expect("xping|xpong|idle");
    if mode == "idle" {
        run_idle(&udp(args.next()), args.next().expect("seconds").parse().unwrap());
        return;
    }
    let (ping_channel, pong_channel) = (udp(args.next()), udp(args.next()));
    match mode.as_str() {
        "xpong" => {
            let cpu = cpu_arg(args.next());
            let lab = Lab::connect(cpu);
            match lab.invoker() {
                Some(duty) => run_pong(&lab, &duty, &ping_channel, &pong_channel, cpu),
                None => run_pong(&lab, &External, &ping_channel, &pong_channel, cpu),
            }
        }
        "xping" => {
            let number = |a: Option<String>| -> usize { a.expect("messages and warmup").parse().unwrap() };
            let (messages, warmup) = (number(args.next()), number(args.next()));
            let cpu = cpu_arg(args.next());
            let lab = Lab::connect(cpu);
            match lab.invoker() {
                Some(duty) => run_ping(&lab, &duty, &ping_channel, &pong_channel, messages, warmup, cpu),
                None => run_ping(&lab, &External, &ping_channel, &pong_channel, messages, warmup, cpu),
            }
        }
        other => panic!("unknown mode {other}: xping, xpong or idle"),
    }
}

fn run_ping(
    lab: &Lab,
    duty: &impl Duty,
    ping_channel: &CStr,
    pong_channel: &CStr,
    messages: usize,
    warmup: usize,
    cpu: Option<usize>,
) {
    let publication = lab.add_publication(ping_channel, PING_STREAM_ID);
    let subscription = lab.add_subscription(&sub_channel(pong_channel), PONG_STREAM_ID);
    lab.wait_until("pong", 30, || {
        publication.is_connected() && subscription.image_at_index(0).is_some()
    });
    let image = subscription.image_at_index(0).unwrap();
    pin(cpu);
    let clock = Clock::calibrate();

    let budget = Duration::from_secs(std::env::var("RTT_SECONDS").map_or(20, |s| s.parse().unwrap()));
    let mut results = Results::new();
    let mut buffer = [0u8; MESSAGE_LENGTH];
    // RTT_LOST_TIMEOUT_US: on an unreliable channel a lost ping or echo is
    // never repaired, so after this long it is counted lost and the next sent
    let timeout_ns: u64 =
        std::env::var("RTT_LOST_TIMEOUT_US").map_or(0, |v| v.parse::<u64>().expect("RTT_LOST_TIMEOUT_US") * 1000);
    let mut seq = 0_u64;
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
            seq,
            timeout_ns,
            |_: &Echo| {},
        );
        if i % 64 == 0 && start.elapsed() > budget / 4 {
            break;
        }
    }
    let start = Instant::now();
    results.start(&clock);
    let mut one_way = OneWay::new(&clock);
    let mut stalls = Stalls::new(&clock);
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
            seq,
            timeout_ns,
            |echo: &Echo| {
                results.record(&clock, echo.echoed.wrapping_sub(echo.sent), i);
                if let Some(one_way) = one_way.as_mut() {
                    one_way.record(i, echo);
                }
            },
        );
        if !echoed {
            lost += 1;
        }
        if i % 64 == 0 && start.elapsed() > budget {
            break;
        }
    }

    results.report(lost, PING_STREAM_ID);
    if let Some(one_way) = &one_way {
        one_way.write();
    }
    if let Some(stalls) = &stalls {
        stalls.write("stalls-ping.csv");
    }
}

/// Sends one stamped ping and polls until pong's echo of it is back.
#[inline]
/// One round trip of message `seq`: `true` once its echo is back, `false` if
/// none came within `timeout_ns` (0: wait for ever, as a reliable channel
/// repairs any loss). An echo of an earlier, timed-out message is skipped.
/// `duty` runs every pass (an embedded driver sends the ping and receives the
/// echo in it).
#[allow(clippy::too_many_arguments)]
fn round_trip(
    duty: &impl Duty,
    clock: &Clock,
    publication: &AeronExclusivePublication,
    image: &AeronImage,
    buffer: &mut [u8; MESSAGE_LENGTH],
    stalls: &mut Option<Stalls>,
    seq: u64,
    timeout_ns: u64,
    mut record: impl FnMut(&Echo),
) -> bool {
    if let Some(s) = stalls.as_mut() {
        s.pass(clock.now());
    }
    buffer[8..16].copy_from_slice(&seq.to_le_bytes());
    let mut retry = Retry::default();
    let started = loop {
        // stamped on each attempt, so time back-pressured is not counted
        let now = clock.stamp();
        buffer[..8].copy_from_slice(&now.to_le_bytes());
        match publication.offer(buffer) {
            Ok(_) => break now,
            Err(e) if e.is_retryable() => {
                if let Err(halt) = retry.check(publication, image) {
                    pong_gone(halt);
                }
                if duty.run() == 0 {
                    spin_loop();
                }
            }
            Err(e) => panic!("offer: {e:?}"),
        }
    };
    let mut echoed = false;
    while !echoed {
        if let Some(s) = stalls.as_mut() {
            s.pass(clock.now());
        }
        let work = duty.run();
        let fragments = image
            .poll_fn(
                |reply, header| {
                    if u64::from_le_bytes(reply[8..16].try_into().unwrap()) == seq {
                        let sent = u64::from_le_bytes(reply[..8].try_into().unwrap());
                        let at = clock.stamp();
                        record(&Echo {
                            sent,
                            echoed: at,
                            pong_rx: i64::from_le_bytes(reply[PONG_RX..PONG_RX + 8].try_into().unwrap()),
                            ping_rx: header.reserved_value().unwrap_or(0),
                        });
                        echoed = true;
                    }
                },
                FRAGMENT_COUNT_LIMIT,
            )
            .unwrap();
        if fragments == 0 && work == 0 {
            assert!(!image.is_closed(), "pong went away");
            if timeout_ns != 0 && clock.now().wrapping_sub(started) > timeout_ns {
                return false;
            }
            spin_loop();
        }
    }
    true
}

/// The ping's end when its pong is gone while it sends: it fails, as it
/// does when the pong's image closes while it waits for an echo.
#[cold]
#[inline(never)]
fn pong_gone(halt: Halt) -> ! {
    panic!("pong went away: {halt} while the ping was back-pressured")
}

/// Echoes every ping until the ping side's image goes away, SIGTERM, or an
/// echo given up (`lab::echo`: SIGTERM, the ping's image closed, or this
/// publication not connected for over a second); says why on stderr.
fn run_pong(lab: &Lab, duty: &impl Duty, ping_channel: &CStr, pong_channel: &CStr, cpu: Option<usize>) {
    let publication = lab.add_publication(pong_channel, PONG_STREAM_ID);
    let subscription = lab.add_subscription(&sub_channel(ping_channel), PING_STREAM_ID);
    lab.wait_until("ping", 60, || subscription.image_at_index(0).is_some());
    let image = subscription.image_at_index(0).unwrap();
    pin(cpu);
    lab::stop_on_sigterm();
    let clock = Clock::calibrate();
    let mut stalls = Stalls::new(&clock);
    let mut halt = None;
    let mut stamped = [0u8; MESSAGE_LENGTH];
    while halt.is_none() && !image.is_closed() && !STOP.load(Ordering::Relaxed) {
        if let Some(s) = stalls.as_mut() {
            s.pass(clock.now());
        }
        let work = duty.run();
        let fragments = image
            .poll_fn(
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
            )
            .unwrap();
        if fragments == 0 && work == 0 {
            spin_loop();
        }
    }
    if let Some(stalls) = &stalls {
        stalls.write("stalls-pong.csv");
    }
    let why = match halt {
        Some(Halt::Stopped) => "SIGTERM",
        Some(Halt::ImageClosed) => "its ping's image closed",
        Some(Halt::NotConnected) => "its publication not connected for over 1 s, its echoes unread",
        None if STOP.load(Ordering::Relaxed) => "SIGTERM",
        None => "its ping's image closed",
    };
    eprintln!("rtt: pong ended: {why}");
}

/// Holds a subscription on `channel` that nobody publishes to, for `seconds`:
/// what an idle host-local channel costs an external driver's agents. An
/// embedded driver would measure nothing of the kind, so it is refused.
fn run_idle(channel: &CStr, seconds: u64) {
    assert!(
        !lab::embedded(),
        "rtt idle measures an external driver's agents: HARNESS_DRIVER=invoker would embed one, which nothing else uses"
    );
    let lab = Lab::connect(None);
    let _subscription = lab.add_subscription(channel, PONG_STREAM_ID + 100);
    eprintln!("rtt: idle subscription on {}", channel.to_string_lossy());
    std::thread::sleep(Duration::from_secs(seconds));
}
