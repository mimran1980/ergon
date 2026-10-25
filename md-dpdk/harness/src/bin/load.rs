//! Round-trip latency under a fixed offered load, open loop, through an
//! external media driver (`AERON_DIR`). The far end echoes each message; this
//! side publishes on a fixed schedule regardless of replies and stamps each
//! message with its *intended* send time, so time spent back-pressured or
//! queued counts against the latency (no coordinated omission).
//!
//! load xping <ping endpoint> <pong endpoint> <msgs/s> <seconds> <size> <cpu|-> [burst]
//!     one stream, echoed by `rtt xpong` (streams 1002 -> 1003)
//! load xmix <ping endpoint> <pong endpoint> <seconds> <cpu|-> <name:stream:msgs/s:size[:burst]>...
//!     several streams at once on the same channels, each on its own schedule
//!     (a mix of rates, sizes and bursts), echoed by `load xecho` on stream + 1000
//! load xecho <ping endpoint> <pong endpoint> <cpu|-> <stream>...
//!     echoes each stream back on stream + 1000 until every ping image goes,
//!     or an echo is given up (`lab::echo`: a ping's image closed, or its
//!     publication not connected for over a second, while back-pressured),
//!     saying why on stderr; SIGTERM kills it
//!
//! `burst` messages (default 1) are due together, `burst / rate` seconds
//! apart. The first second is warm-up, not recorded. Prints one line per stream:
//! <load|mix>,<label>,<udp|udp-name>,<size>,<rate>,<burst>,p50,p90,p99,p99.9,p99.99,max,mean (ns),
//! <samples>,<sent>,<received>,<back-pressured messages>,<achieved msgs/s>,<max send lag ns>,
//! <scheduled>,<late>,<censored>
//! `achieved` is measured over that stream's own schedule window, not the
//! run's: a mixed run's streams finish at different times, and charging them
//! all the longest would report the fast ones at the slowest one's rate. Below
//! `rate`, the generator or the path could not keep the schedule. `scheduled`
//! is what the run owed; `sent` below it is work never offered. `late` counts
//! the sends that found the following slot already due (the run owed more than
//! one), beside `max send lag`'s worst slip. The percentiles cover every
//! measured message the schedule owed and was due by the run's end, not only
//! the echoed ones: a message sent without an echo, or never offered, is
//! recorded at what it is known to be at least, the run's end minus its due
//! time. `censored` counts those plus round trips at the histogram's bound:
//! values only known to be at least what was recorded, so a non-zero
//! `censored` makes the max (and any percentile it reaches) a floor.
//! `samples` is the echoes after warm-up plus the messages owed without one.
//! `max send lag` is the longest a measured message waited past
//! its due time to be offered, over the measured messages that were offered
//! (warm-up is in neither it nor the histogram). `received < sent` means replies never came back
//! within two seconds of the end (a stall: Aeron itself does not lose
//! messages), and a run is cut short at twice its schedule if the peer stalls
//! or goes away. With `RTT_HISTOGRAM=<path>` the first stream's full histogram
//! is written there.
//!
//! With `LOAD_SPIKES=<path>`, every round trip of at least `LOAD_SPIKE_NS`
//! (default 100000) after warm-up is written there as
//! `stream,due_ns,latency_ns` (`due_ns` from the start of the schedule, whose
//! `CLOCK_REALTIME` the first line gives as `# start_realtime_ns=`), the first
//! 100,000 of them (`# spikes_total=` says how many there were: past capacity
//! nearly every message is one), so spikes can be lined up against periodic work.
//!
//! `HARNESS_DRIVER=invoker` embeds the media driver instead (`lab.rs`): the
//! measuring thread then also runs the client's conductor and the driver,
//! every pass of its loop. `PUB_CHANNEL_PARAMS` is appended to every
//! publication's channel, `SUB_CHANNEL_PARAMS` to every subscription's.

#[path = "../lab.rs"]
mod lab;

use hdrhistogram::Histogram;
use lab::{Clock, Duty, External, Halt, Lab, cpu_arg, label, pin, sub_channel};
use rusteron_client::*;
use std::ffi::CString;
use std::hint::spin_loop;

// rtt's streams, so `rtt xpong` echoes for xping
const PING_STREAM_ID: i32 = 1002;
const PONG_STREAM_ID: i32 = 1003;
// xmix/xecho: the echo of stream s is on s + ECHO_OFFSET
const ECHO_OFFSET: i32 = 1000;
const HEADER: usize = 16; // intended send time (ns), sequence
const SECOND_NS: u64 = 1_000_000_000;
const FRAGMENT_COUNT_LIMIT: usize = 64;
const SPIKE_CAPACITY: usize = 100_000;
/// The histogram's highest trackable latency. A longer round trip is recorded
/// at this bound and counted as censored: its true value is only known to be
/// at least this, so a max at the bound is a floor, not the slowest round trip.
const HISTOGRAM_MAX_NS: u64 = 60_000_000_000;

fn udp(endpoint: Option<String>) -> CString {
    CString::new(format!("aeron:udp?endpoint={}", endpoint.expect("host:port"))).unwrap()
}

/// One offered stream: what to send, and on which streams.
struct Spec {
    name: Option<String>,
    ping_stream: i32,
    pong_stream: i32,
    rate: u64,
    size: usize,
    burst: u64,
}

impl Spec {
    /// `name:stream:msgs/s:size[:burst]`
    fn parse(arg: &str) -> Self {
        let parts: Vec<&str> = arg.split(':').collect();
        assert!(
            (4..=5).contains(&parts.len()),
            "stream spec {arg}: name:stream:msgs/s:size[:burst]"
        );
        let stream: i32 = parts[1].parse().expect("stream id");
        Self {
            name: Some(parts[0].to_owned()),
            ping_stream: stream,
            pong_stream: stream + ECHO_OFFSET,
            rate: parts[2].parse().expect("msgs/s"),
            size: parts[3].parse().expect("size"),
            burst: parts.get(4).map_or(1, |b| b.parse::<u64>().expect("burst")).max(1),
        }
    }
}

fn main() {
    lab::lock_memory();
    let mut args = std::env::args().skip(1);
    let usage = "load xping <ping> <pong> <rate> <seconds> <size> <cpu> [burst] | \
                 load xmix <ping> <pong> <seconds> <cpu> <name:stream:rate:size[:burst]>... | \
                 load xecho <ping> <pong> <cpu> <stream>...";
    let mode = args.next().unwrap_or_else(|| panic!("{usage}"));
    let (ping_channel, pong_channel) = (udp(args.next()), udp(args.next()));
    let number = |a: Option<String>, what: &str| -> u64 { a.unwrap_or_else(|| panic!("{what}")).parse().unwrap() };
    match mode.as_str() {
        "xping" => {
            let rate = number(args.next(), "msgs/s");
            let seconds = number(args.next(), "seconds");
            let size = usize::try_from(number(args.next(), "size")).unwrap();
            let cpu = cpu_arg(args.next());
            let burst = args.next().map_or(1, |b| b.parse::<u64>().unwrap()).max(1);
            let spec = Spec {
                name: None,
                ping_stream: PING_STREAM_ID,
                pong_stream: PONG_STREAM_ID,
                rate,
                size,
                burst,
            };
            offer(&ping_channel, &pong_channel, vec![spec], seconds, cpu);
        }
        "xmix" => {
            let seconds = number(args.next(), "seconds");
            let cpu = cpu_arg(args.next());
            let specs: Vec<Spec> = args.map(|a| Spec::parse(&a)).collect();
            assert!(!specs.is_empty(), "{usage}");
            offer(&ping_channel, &pong_channel, specs, seconds, cpu);
        }
        "xecho" => {
            let cpu = cpu_arg(args.next());
            let streams: Vec<i32> = args.map(|a| a.parse().expect("stream id")).collect();
            assert!(!streams.is_empty(), "{usage}");
            let lab = Lab::connect(cpu);
            match lab.invoker() {
                Some(duty) => echo(&lab, &duty, &ping_channel, &pong_channel, &streams, cpu),
                None => echo(&lab, &External, &ping_channel, &pong_channel, &streams, cpu),
            }
        }
        other => panic!("unknown mode {other}: {usage}"),
    }
}

/// A stream being offered: its schedule and what came back.
struct Offered {
    spec: Spec,
    publication: AeronExclusivePublication,
    image: AeronImage,
    message: Vec<u8>,
    interval_fp: u128,
    total: u64,
    warmup: u64,
    sent: u64,
    backpressured: u64,
    stalled: bool,
    max_lag: u64,
    /// Sends of a measured message that found the following slot already due:
    /// the run owed more than one. The count, beside `max_lag`'s worst slip.
    late: u64,
    /// Measured echoes whose latency reached the histogram's bound, so their
    /// true value is only known to be at least that. `saturating_record` keeps
    /// them at the bound; this says how many are not their own value.
    censored: u64,
    /// When this stream finished its own schedule, so its rate is over its own
    /// window and not the window of whatever stream ran longest. 0 while the
    /// schedule is unfinished (a stall, a lost peer, the run's deadline).
    send_end: u64,
    histogram: Histogram<u64>,
    received: u64,
}

/// When message `n` was due: `start` plus `n / burst` burst intervals, in
/// 32.32 fixed point so a long run does not drift.
#[inline]
fn due(start: u64, interval_fp: u128, burst: u64, n: u64) -> u64 {
    start + ((interval_fp * u128::from(n / burst)) >> 32) as u64
}

/// Whether the offer of message `n` at `now` found the *following* slot already
/// due: the run owed more than one. A run keeping its schedule reports 0. The
/// sends of a catch-up burst after a stall report as late, the one that caught
/// up not, because it went out before the following slot was due. This is the
/// count behind `max send lag`: the worst slip and how often it happened.
#[inline]
fn is_late(burst: u64, interval_fp: u128, start: u64, n: u64, now: u64) -> bool {
    now >= due(start, interval_fp, burst, (n / burst + 1) * burst)
}

/// The messages per second a stream offered after warm-up, over the window its
/// own schedule occupied: from when its measured schedule was due to begin, to
/// when it stopped offering (`send_end`), or to the run's end when it never
/// finished. Below the requested rate, the generator or the path could not
/// keep the schedule.
///
/// Each stream is measured over its own window. A mixed run's streams finish
/// at different times; charging them all the run's end would report the fast
/// ones at the slowest one's rate.
#[inline]
fn achieved_rate(sent: u64, warmup: u64, due_start: u64, send_end: u64, run_end: u64) -> f64 {
    if sent <= warmup {
        return 0.0;
    }
    let end = if send_end != 0 { send_end } else { run_end };
    let window = end.saturating_sub(due_start);
    if window == 0 {
        // no window to divide by: no rate, not a measured zero and not an
        // infinity from a one-nanosecond floor
        return 0.0;
    }
    (sent - warmup) as f64 * 1e9 / window as f64
}

/// Records each measured message whose echo never came at what it is known
/// to be at least: `end` minus when it was due. Echoes arrive in order (one
/// Aeron image each way), so the echoed messages are `0..received` and the
/// rest, `received..total`, were owed without one: sent and unanswered, or
/// never offered. Warm-up stays out, as it does for an echo. A message not
/// yet due at `end` (the peer went away early) has no bound above zero and
/// is not recorded. Returns how many were recorded; each is censored.
fn record_incomplete(
    histogram: &mut Histogram<u64>,
    due_of: impl Fn(u64) -> u64,
    warmup: u64,
    received: u64,
    total: u64,
    end: u64,
) -> u64 {
    let mut recorded = 0;
    for n in received.max(warmup)..total {
        let due = due_of(n);
        if due >= end {
            // due no earlier than the end: neither are the later ones
            break;
        }
        histogram.saturating_record(end - due);
        recorded += 1;
    }
    recorded
}

impl Offered {
    #[inline]
    fn due(&self, start: u64, n: u64) -> u64 {
        due(start, self.interval_fp, self.spec.burst, n)
    }
}

/// Where spikes go: preallocated, so recording never allocates.
struct Spikes {
    threshold_ns: u64,
    rows: Vec<(i32, u64, u64)>,
    dropped: u64,
}

/// Offers `specs` on the driver `HARNESS_DRIVER` picks.
fn offer(ping_channel: &CString, pong_channel: &CString, specs: Vec<Spec>, seconds: u64, cpu: Option<usize>) {
    for spec in &specs {
        assert!(spec.rate > 0 && spec.size >= HEADER, "rate > 0, size >= {HEADER}");
    }
    let lab = Lab::connect(cpu);
    match lab.invoker() {
        Some(duty) => run(&lab, &duty, ping_channel, pong_channel, specs, seconds, cpu),
        None => run(&lab, &External, ping_channel, pong_channel, specs, seconds, cpu),
    }
}

fn run(
    lab: &Lab,
    duty: &impl Duty,
    ping_channel: &CString,
    pong_channel: &CString,
    specs: Vec<Spec>,
    seconds: u64,
    cpu: Option<usize>,
) {
    let mut pending = Vec::new();
    for spec in specs {
        let publication = lab.add_publication(ping_channel, spec.ping_stream);
        let subscription = lab.add_subscription(&sub_channel(pong_channel), spec.pong_stream);
        pending.push((spec, publication, subscription));
    }
    lab.wait_until("pong", 30, || {
        pending
            .iter()
            .all(|(_, p, s)| p.is_connected() && s.image_at_index(0).is_some())
    });
    pin(cpu);
    let clock = Clock::calibrate();
    let mut streams: Vec<Offered> = pending
        .into_iter()
        .map(|(spec, publication, subscription)| {
            let image = subscription.image_at_index(0).unwrap();
            let max_payload = publication.max_payload_length().unwrap();
            assert!(
                spec.size <= max_payload,
                "size {} > max payload {max_payload}: it would fragment",
                spec.size
            );
            // ns per burst interval, in 32.32 fixed point so long runs do not drift
            let interval_fp = (u128::from(SECOND_NS) << 32) * u128::from(spec.burst) / u128::from(spec.rate);
            Offered {
                message: vec![0u8; spec.size],
                interval_fp,
                total: spec.rate * (seconds + 1), // the first second is warm-up
                warmup: spec.rate,
                sent: 0,
                backpressured: 0,
                stalled: false,
                max_lag: 0,
                late: 0,
                censored: 0,
                send_end: 0,
                histogram: Histogram::<u64>::new_with_bounds(1, HISTOGRAM_MAX_NS, 3).unwrap(),
                received: 0,
                spec,
                publication,
                image,
            }
        })
        .collect();
    let mut spikes = std::env::var("LOAD_SPIKES").ok().filter(|p| !p.is_empty()).map(|path| {
        let threshold_ns = std::env::var("LOAD_SPIKE_NS").map_or(100_000, |v| v.parse().expect("LOAD_SPIKE_NS"));
        (
            path,
            Spikes {
                threshold_ns,
                rows: Vec::with_capacity(SPIKE_CAPACITY),
                dropped: 0,
            },
        )
    });

    let start = clock.now();
    let start_realtime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    // A hard bound: the schedule plus as long again, so a stalled or lost
    // peer ends the run (reported as sent < total) instead of hanging it.
    let send_deadline = start + 2 * (seconds + 1) * SECOND_NS;
    let mut gone = false;
    'send: while streams.iter().any(|s| s.sent < s.total) {
        let now = clock.now();
        if now > send_deadline {
            break;
        }
        for s in &mut streams {
            // the messages now due, each stamped with when it was due; at
            // most a poll's worth per pass, so replies are read between them
            let mut budget = FRAGMENT_COUNT_LIMIT;
            while budget > 0 && s.sent < s.total && s.due(start, s.sent) <= now {
                let intended = s.due(start, s.sent);
                s.message[..8].copy_from_slice(&intended.to_le_bytes());
                s.message[8..16].copy_from_slice(&s.sent.to_le_bytes());
                match s.publication.offer(&s.message) {
                    Ok(_) => {
                        // the lag of the measured run alone, as the
                        // histogram is: warm-up messages are not in it, so a
                        // warm-up that fell behind must not raise this past
                        // the slowest recorded round trip
                        if s.sent >= s.warmup {
                            s.max_lag = s.max_lag.max(clock.stamp().saturating_sub(intended));
                            if is_late(s.spec.burst, s.interval_fp, start, s.sent, now) {
                                s.late += 1;
                            }
                        }
                        s.sent += 1;
                        budget -= 1;
                        s.stalled = false;
                        // this stream's schedule is done: its window ends
                        // here, not whenever a slower stream's does
                        if s.sent == s.total {
                            s.send_end = clock.now();
                        }
                    }
                    Err(e) if e.is_retryable() => {
                        if !s.publication.is_connected() || s.image.is_closed() {
                            eprintln!(
                                "load: stream {}'s pong went away after {} messages",
                                s.spec.ping_stream, s.sent
                            );
                            gone = true;
                            break 'send;
                        }
                        // messages that met back-pressure at least once; an
                        // admin action (once per term rotation) is not
                        if matches!(e, AeronOfferError::BackPressured) && !s.stalled {
                            s.backpressured += 1;
                            s.stalled = true;
                        }
                        break;
                    }
                    Err(e) => panic!("offer: {e:?}"),
                }
            }
        }
        let work = duty.run();
        if poll_all(&mut streams, &clock, start, spikes.as_mut().map(|(_, s)| s)) == 0 && work == 0 {
            spin_loop();
        }
    }
    let send_end = clock.now();
    // The rate each stream actually offered after warm-up, over its own
    // schedule's window. Below `rate`, the generator or the path could not
    // keep the schedule. A mixed run's streams finish at different times, so
    // each is measured to its own end (or to the run's, when it never
    // finished): one shared end would report the fast streams at the slowest
    // one's rate.
    let achieved: Vec<f64> = streams
        .iter()
        .map(|s| achieved_rate(s.sent, s.warmup, s.due(start, s.warmup), s.send_end, send_end))
        .collect();
    // drain: replies still in flight, for up to two seconds
    let drain_until = clock.now() + 2 * SECOND_NS;
    while !gone && streams.iter().any(|s| s.received < s.sent) && clock.now() < drain_until {
        let work = duty.run();
        if poll_all(&mut streams, &clock, start, spikes.as_mut().map(|(_, s)| s)) == 0 && work == 0 {
            spin_loop();
        }
    }

    // What the schedule owed and never got back is in the percentiles: each
    // measured message without an echo at its lower bound, counted censored.
    // Left out, a stall that lost the rest of the run would report only the
    // round trips that got through it.
    let end = clock.stamp();
    for s in &mut streams {
        let (interval_fp, burst) = (s.interval_fp, s.spec.burst);
        s.censored += record_incomplete(
            &mut s.histogram,
            |n| due(start, interval_fp, burst, n),
            s.warmup,
            s.received,
            s.total,
            end,
        );
    }

    for (s, achieved) in streams.iter().zip(&achieved) {
        let q = |quantile| s.histogram.value_at_quantile(quantile);
        let (kind, name) = match &s.spec.name {
            Some(n) => ("mix", format!("udp-{n}")),
            None => ("load", "udp".to_owned()),
        };
        println!(
            "{kind},{},{name},{},{},{},{},{},{},{},{},{},{:.0},{},{},{},{},{achieved:.0},{},{},{},{}",
            label(),
            s.spec.size,
            s.spec.rate,
            s.spec.burst,
            q(0.5),
            q(0.9),
            q(0.99),
            q(0.999),
            q(0.9999),
            s.histogram.max(),
            s.histogram.mean(),
            s.histogram.len(),
            s.sent,
            s.received,
            s.backpressured,
            s.max_lag,
            // the work behind the percentiles: what the schedule owed, how
            // often the run fell more than one slot behind, and how many
            // recorded latencies are only known to be at least the histogram's
            // bound. `sent` below `scheduled` is work never offered; a
            // `censored` count above zero beside a max at the bound means that
            // max is a floor, not the slowest round trip.
            s.total,
            s.late,
            s.censored
        );
    }
    if let Some(path) = std::env::var("RTT_HISTOGRAM").ok().filter(|p| !p.is_empty()) {
        let mut rows = String::from("value_ns,count\n");
        for v in streams[0].histogram.iter_recorded() {
            rows.push_str(&format!("{},{}\n", v.value_iterated_to(), v.count_at_value()));
        }
        std::fs::write(&path, rows).unwrap_or_else(|e| panic!("RTT_HISTOGRAM {path}: {e}"));
    }
    if let Some((path, spikes)) = spikes {
        // the wall-clock time due_ns counts from, to line spikes up with traces
        let total = spikes.rows.len() as u64 + spikes.dropped;
        let mut rows =
            format!("# start_realtime_ns={start_realtime}\n# spikes_total={total}\nstream,due_ns,latency_ns\n");
        for (stream, due, latency) in &spikes.rows {
            rows.push_str(&format!("{stream},{due},{latency}\n"));
        }
        std::fs::write(&path, rows).unwrap_or_else(|e| panic!("LOAD_SPIKES {path}: {e}"));
    }
}

/// Reads every stream's echoes once; the fragments read.
#[inline]
fn poll_all(streams: &mut [Offered], clock: &Clock, start: u64, mut spikes: Option<&mut Spikes>) -> i32 {
    let mut fragments = 0;
    for s in streams.iter_mut() {
        let (histogram, received, censored, warmup, stream) = (
            &mut s.histogram,
            &mut s.received,
            &mut s.censored,
            s.warmup,
            s.spec.ping_stream,
        );
        let mut spikes = spikes.as_deref_mut();
        fragments += s
            .image
            .poll_fn(
                |reply, _header| {
                    let intended = u64::from_le_bytes(reply[..8].try_into().unwrap());
                    let sequence = u64::from_le_bytes(reply[8..16].try_into().unwrap());
                    if sequence >= warmup {
                        let latency = clock.stamp().wrapping_sub(intended);
                        histogram.saturating_record(latency);
                        // `saturating_record` keeps a longer round trip at the
                        // bound: its true value is only known to be at least
                        // that, so it is counted, not left to read as the max
                        if latency >= HISTOGRAM_MAX_NS {
                            *censored += 1;
                        }
                        if let Some(spikes) = spikes.as_deref_mut()
                            && latency >= spikes.threshold_ns
                        {
                            if spikes.rows.len() < SPIKE_CAPACITY {
                                spikes.rows.push((stream, intended.wrapping_sub(start), latency));
                            } else {
                                spikes.dropped += 1;
                            }
                        }
                    }
                    *received += 1;
                },
                FRAGMENT_COUNT_LIMIT,
            )
            .unwrap();
    }
    fragments
}

/// Echoes each stream back on stream + ECHO_OFFSET until every ping image
/// goes, or an echo is given up (`lab::echo`); says why on stderr.
fn echo(
    lab: &Lab,
    duty: &impl Duty,
    ping_channel: &CString,
    pong_channel: &CString,
    streams: &[i32],
    cpu: Option<usize>,
) {
    let mut pairs = Vec::new();
    for &stream in streams {
        let publication = lab.add_publication(pong_channel, stream + ECHO_OFFSET);
        let subscription = lab.add_subscription(&sub_channel(ping_channel), stream);
        pairs.push((publication, subscription));
    }
    lab.wait_until("ping", 60, || pairs.iter().all(|(_, s)| s.image_at_index(0).is_some()));
    let pairs: Vec<(AeronExclusivePublication, AeronImage)> = pairs
        .into_iter()
        .map(|(p, s)| {
            let image = s.image_at_index(0).unwrap();
            (p, image)
        })
        .collect();
    pin(cpu);
    let mut halt = None;
    while halt.is_none() && pairs.iter().any(|(_, image)| !image.is_closed()) {
        let mut fragments = 0;
        let work = duty.run();
        for (publication, image) in &pairs {
            fragments += image
                .poll_fn(
                    |ping, _header| {
                        if halt.is_none()
                            && let Err(why) = lab::echo(duty, publication, image, ping)
                        {
                            halt = Some(why);
                        }
                    },
                    FRAGMENT_COUNT_LIMIT,
                )
                .unwrap();
        }
        if fragments == 0 && work == 0 {
            spin_loop();
        }
    }
    let why = match halt {
        None => "every ping's image closed",
        Some(Halt::ImageClosed) => "a ping's image closed while its echo was back-pressured",
        Some(Halt::NotConnected) => "an echo's publication not connected for over 1 s, its echoes unread",
        Some(Halt::Stopped) => "SIGTERM",
    };
    eprintln!("load: echo ended: {why}");
}

#[cfg(test)]
mod tests {
    use super::{HISTOGRAM_MAX_NS, SECOND_NS, achieved_rate, due, is_late, record_incomplete};
    use hdrhistogram::Histogram;

    fn histogram() -> Histogram<u64> {
        Histogram::<u64>::new_with_bounds(1, HISTOGRAM_MAX_NS, 3).unwrap()
    }

    /// A measured message whose echo never came is in the distribution at
    /// what it is known to be at least: the run's end minus when it was due.
    /// Sent with no echo and never offered at all are the same: both were
    /// owed. Echoed messages and warm-up are not added again.
    #[test]
    fn a_message_whose_echo_never_came_is_recorded_at_its_lower_bound() {
        // 10 messages, one a second from 0; warm-up 2, echoes for 0..6
        let interval_fp = (SECOND_NS as u128) << 32;
        let mut h = histogram();
        let end = 20 * SECOND_NS;
        let n = record_incomplete(&mut h, |n| due(0, interval_fp, 1, n), 2, 6, 10, end);
        assert_eq!(n, 4, "messages 6..10");
        assert_eq!(h.len(), 4);
        assert!(h.equivalent(h.min(), 11 * SECOND_NS), "message 9: {}", h.min());
        assert!(h.equivalent(h.max(), 14 * SECOND_NS), "message 6: {}", h.max());
    }

    /// Warm-up that never came back is not measured, so it stays out.
    #[test]
    fn warm_up_without_an_echo_stays_out() {
        let interval_fp = (SECOND_NS as u128) << 32;
        let mut h = histogram();
        let n = record_incomplete(&mut h, |n| due(0, interval_fp, 1, n), 5, 3, 7, 20 * SECOND_NS);
        assert_eq!(n, 2, "messages 5 and 6 only");
    }

    /// A message not yet due when the run ended (the peer went away early)
    /// has no lower bound above zero, so it is not recorded as a fast trip.
    #[test]
    fn a_message_not_yet_due_at_the_end_is_not_recorded() {
        let interval_fp = (SECOND_NS as u128) << 32;
        let mut h = histogram();
        let n = record_incomplete(&mut h, |n| due(0, interval_fp, 1, n), 0, 0, 10, 4 * SECOND_NS);
        assert_eq!(n, 4, "messages 0..4 were due before the end");
        assert!(h.equivalent(h.min(), SECOND_NS), "message 3: {}", h.min());
    }

    /// Every echo came back: nothing to add.
    #[test]
    fn a_complete_run_adds_nothing() {
        let interval_fp = (SECOND_NS as u128) << 32;
        let mut h = histogram();
        assert_eq!(
            record_incomplete(&mut h, |n| due(0, interval_fp, 1, n), 2, 10, 10, 20 * SECOND_NS),
            0
        );
        assert_eq!(h.len(), 0);
    }

    /// A stream's own send end, not the run's. A finished its 1000 messages
    /// after a second; B dragged on for two. Reporting both against the run's
    /// end would give each 500/s and hide that A kept its schedule.
    #[test]
    fn a_stream_that_finished_early_is_not_charged_the_laggards_window() {
        let run_end = 2 * SECOND_NS;
        assert_eq!(achieved_rate(1000, 0, 0, SECOND_NS, run_end), 1000.0);
        assert_eq!(achieved_rate(1000, 0, 0, 2 * SECOND_NS, run_end), 500.0);
    }

    /// A stream that never finished its schedule (a stall, or a peer that went
    /// away) has no end of its own and is reported to the run's end, so it is
    /// never read as having kept the schedule it did not finish.
    #[test]
    fn a_stream_that_never_finished_is_reported_to_the_runs_end() {
        assert_eq!(achieved_rate(1000, 0, 0, 0, 2 * SECOND_NS), 500.0);
    }

    /// Warm-up is in neither the histogram nor the rate.
    #[test]
    fn the_rate_covers_the_measured_messages_alone() {
        assert_eq!(achieved_rate(1100, 100, 0, SECOND_NS, SECOND_NS), 1000.0);
        assert_eq!(achieved_rate(100, 100, 0, SECOND_NS, SECOND_NS), 0.0);
        assert_eq!(achieved_rate(0, 100, 0, SECOND_NS, SECOND_NS), 0.0);
    }

    /// A stream whose window is empty would divide by zero; it reports 0
    /// rather than an infinity that would read as a spectacular rate.
    #[test]
    fn an_empty_window_reports_no_rate() {
        assert_eq!(achieved_rate(1000, 0, 5, 5, 5), 0.0);
    }

    /// Late means the *following* slot was already due when the send went out:
    /// the run owed more than one. A run keeping its schedule reports 0.
    #[test]
    fn a_run_keeping_its_schedule_reports_nothing_late() {
        // 1000 msgs/s, one per slot: interval 1 ms, 32.32 fixed point.
        let interval_fp = (SECOND_NS as u128) << 32;
        let start = 100;
        // on time, and a little past its own slot but before the next is due
        assert!(!is_late(1, interval_fp, start, 0, due(start, interval_fp, 1, 0)));
        assert!(!is_late(1, interval_fp, start, 0, due(start, interval_fp, 1, 1) - 1));
    }

    /// The sends of a catch-up burst after a stall report as late; the one that
    /// caught up does not, because it went out before the following slot was due.
    #[test]
    fn a_catch_up_burst_reports_the_overtaken_slots_as_late() {
        let interval_fp = (SECOND_NS as u128) << 32;
        let start = 100;
        // a stall until three intervals past the start, then four sends
        let now = start + 3 * SECOND_NS;
        // messages 0 and 1: their following slot was already due
        assert!(is_late(1, interval_fp, start, 0, now));
        assert!(is_late(1, interval_fp, start, 1, now));
        assert!(is_late(1, interval_fp, start, 2, now));
        // message 3 is the one that caught up: its following slot (message 4's)
        // is a second away, so it is not late
        assert!(!is_late(1, interval_fp, start, 3, now));
    }

    /// A burst of `b` messages shares one slot, so a burst sent a little after
    /// its slot is not late until the *next* burst's slot is due.
    #[test]
    fn a_burst_is_late_only_once_the_next_bursts_slot_is_due() {
        let interval_fp = (SECOND_NS as u128) << 32; // 1 ms per burst
        let start = 0;
        // messages 0 and 1 are one burst; the next burst (2,3) is due at 1 ms
        assert!(!is_late(2, interval_fp, start, 0, SECOND_NS - 1));
        assert!(!is_late(2, interval_fp, start, 1, SECOND_NS - 1));
        assert!(is_late(2, interval_fp, start, 0, SECOND_NS));
        assert!(is_late(2, interval_fp, start, 1, SECOND_NS));
    }
}
