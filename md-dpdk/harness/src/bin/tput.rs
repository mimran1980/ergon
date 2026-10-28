//! Cross-host throughput through an external media driver (`AERON_DIR`): the
//! kernel-UDP driver or the DPDK one, the client code identical. Ported from
//! rusteron's x86 lab (`scripts/x86-lab/harness/src/bin/tput.rs`), cross-host
//! modes only.
//!
//! The subscriber binds <endpoint> (host:port) on its host; the publisher sends
//! 32-byte messages to it flat out for <seconds>:
//! tput xsub <endpoint> <seconds> <cpu|->
//! tput xpub <endpoint> <seconds> <cpu|-> [size]   (default 32 bytes)
//! xsub prints, over one-second samples after a one-second warmup:
//! tput,<label>,udp,<median msgs/s>,<min>,<max>,<received>,<lost>,<window start ns>,<window end ns>
//! the window being the samples' span on the wall clock (`CLOCK_REALTIME`, ns
//! since the epoch), so the lab can check that several subscribers' measured
//! seconds overlapped (`md-dpdk/benches/cloud/lib.sh` `multi_flow_max`)
//!
//! `HARNESS_DRIVER=invoker` embeds the media driver instead (`lab.rs`): the
//! measuring thread (xpub's main thread then, while another only times the
//! run) also runs the client's conductor and the driver, every pass of its
//! loop. `PUB_CHANNEL_PARAMS` is appended to the publication's channel,
//! `SUB_CHANNEL_PARAMS` to the subscription's.

#[path = "../lab.rs"]
mod lab;

use lab::{Clock, Duty, External, Invoker, Lab, cpu_arg, label, pin, sub_channel};
use rusteron_client::{AeronCError, AeronSubscription};
use std::error::Error;
use std::ffi::{CStr, CString};
use std::hint::spin_loop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STREAM_ID: i32 = 1001;
const MESSAGE_LENGTH: usize = 32;
const FRAGMENT_COUNT_LIMIT: usize = 256;

fn main() -> Result<(), Box<dyn Error>> {
    lab::lock_memory()?;
    let mut args = std::env::args().skip(1);
    let mode = args.next().ok_or("xsub or xpub")?;
    let endpoint = args.next().ok_or("an endpoint: host:port")?;
    let channel = CString::new(format!("aeron:udp?endpoint={endpoint}"))?;
    let seconds = args.next().ok_or("seconds")?;
    let seconds: u64 = seconds.parse().map_err(|e| format!("seconds {seconds:?}: {e}"))?;
    let cpu = cpu_arg(args.next())?;
    let size = args.next().map_or(Ok(MESSAGE_LENGTH), |s| {
        s.parse::<usize>().map_err(|e| format!("size {s:?}: {e}"))
    })?;
    match mode.as_str() {
        "xpub" => {
            let running = AtomicBool::new(true);
            let stop_after = || {
                std::thread::sleep(Duration::from_secs(seconds));
                running.store(false, Ordering::Release);
            };
            std::thread::scope(|scope| {
                if lab::embedded() {
                    // the driver's thread publishes, the main one, as a lab
                    // client's measuring thread always is under INVOKER
                    scope.spawn(stop_after);
                    publish(&running, &channel, cpu, size)
                } else {
                    // its error as text: a boxed error does not cross threads
                    let publisher = scope.spawn(|| publish(&running, &channel, cpu, size).map_err(|e| e.to_string()));
                    stop_after();
                    let outcome = publisher.join().map_err(|_| "the publisher's thread failed")?;
                    Ok(outcome?)
                }
            })
        }
        "xsub" => {
            let lab = Lab::connect(cpu)?;
            let subscription = lab.add_subscription(&sub_channel(&channel)?, STREAM_ID)?;
            lab.wait_until("publisher", 60, || subscription.image_at_index(0).is_some())?;
            pin(cpu)?;
            let seconds = usize::try_from(seconds)?;
            let measured = match lab.invoker() {
                Some(duty) => measure(
                    &Driven {
                        subscription: &subscription,
                        duty: &duty,
                    },
                    seconds,
                )?,
                None => measure(&subscription, seconds)?,
            };
            println!("{}", measured.line(&label()));
            Ok(())
        }
        other => Err(format!("unknown mode {other}: xsub or xpub").into()),
    }
}

/// What `measure` polls: the subscription, or a test's stand-in. Generic, so
/// the subscription's handler inlines into the measuring loop as before.
trait Poll {
    /// Hands up to `FRAGMENT_COUNT_LIMIT` messages to `on_message`; how many.
    fn poll(&self, on_message: impl FnMut(&[u8])) -> Result<i32, AeronCError>;
}

impl Poll for AeronSubscription {
    #[allow(
        clippy::inline_always,
        reason = "called in every pass of a measured loop: kept inline whatever the inliner decides"
    )]
    #[inline(always)]
    fn poll(&self, mut on_message: impl FnMut(&[u8])) -> Result<i32, AeronCError> {
        self.poll_fn(|message, _header| on_message(message), FRAGMENT_COUNT_LIMIT)
    }
}

/// The subscription with an embedded driver's duty, run before each poll: the
/// messages read, plus the work the duty found, so an idle pass is one that
/// found neither.
struct Driven<'a> {
    subscription: &'a AeronSubscription,
    duty: &'a Invoker<'a>,
}

impl Poll for Driven<'_> {
    #[allow(
        clippy::inline_always,
        reason = "called in every pass of a measured loop: kept inline whatever the inliner decides"
    )]
    #[inline(always)]
    fn poll(&self, on_message: impl FnMut(&[u8])) -> Result<i32, AeronCError> {
        let work = self.duty.run();
        Ok(Poll::poll(self.subscription, on_message)? + work)
    }
}

/// A subscriber's measurement.
struct Measured {
    /// Messages per second over each one-second sample, sorted.
    samples: Vec<f64>,
    /// Messages received over the samples.
    received: u64,
    /// Messages the publisher's sequence numbers say never arrived (on an
    /// unreliable channel, where a gap is filled rather than repaired).
    lost: u64,
    /// The samples' span on the wall clock (`CLOCK_REALTIME`, ns since the epoch).
    window: (u128, u128),
}

impl Measured {
    /// `tput,<label>,udp,<median msgs/s>,<min>,<max>,<received>,<lost>,<window start ns>,<window end ns>`:
    /// the window last, so every reader of the fields before it reads them as it did
    fn line(&self, label: &str) -> String {
        let samples = &self.samples;
        format!(
            "tput,{label},udp,{:.0},{:.0},{:.0},{},{},{},{}",
            samples[samples.len() / 2],
            samples[0],
            samples[samples.len() - 1],
            self.received,
            self.lost,
            self.window.0,
            self.window.1
        )
    }
}

/// Messages per second over `seconds` one-second samples after a one-second
/// warmup, sorted, then the messages received over those samples, how many
/// never arrived, and the samples' span on the wall clock: read once as the
/// warmup ends and once after the last sample, nothing more in the loop.
fn measure(subscription: &impl Poll, seconds: usize) -> Result<Measured, AeronCError> {
    const SECOND_NS: u64 = 1_000_000_000;
    let clock = Clock::calibrate();
    let mut count = 0u64;
    // sequence numbers seen while measuring: the first, the highest, how many
    let (mut first, mut highest, mut received) = (u64::MAX, 0u64, 0u64);
    let mut measuring = false;
    let mut samples = Vec::with_capacity(seconds);
    let mut window_start = clock.now();
    let warmup_start = window_start;
    let mut warming = true;
    let mut start_realtime = 0;
    while samples.len() < seconds {
        let fragments = subscription.poll(|message| {
            count += 1;
            if measuring {
                let seq = lab::le_u64(message, 0);
                first = first.min(seq);
                highest = highest.max(seq);
                received += 1;
            }
        })?;
        if fragments == 0 {
            spin_loop();
        }
        let now = clock.now();
        if warming {
            if now - warmup_start >= SECOND_NS {
                warming = false;
                measuring = true;
                count = 0;
                window_start = now;
                start_realtime = realtime_ns();
            }
        } else if now - window_start >= SECOND_NS {
            #[allow(
                clippy::cast_precision_loss,
                reason = "a rate to report: 2^52 messages or ns is far past a second's"
            )]
            samples.push(count as f64 * 1e9 / (now - window_start) as f64);
            count = 0;
            window_start = now;
        }
    }
    let end_realtime = realtime_ns();
    samples.sort_by(f64::total_cmp);
    let lost = if received == 0 {
        0
    } else {
        (highest - first + 1).saturating_sub(received)
    };
    Ok(Measured {
        samples,
        received,
        lost,
        window: (start_realtime, end_realtime),
    })
}

/// `CLOCK_REALTIME` in ns since the epoch: one clock for every subscriber on a
/// host, so their windows compare (0 for a clock before the epoch).
fn realtime_ns() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos())
}

/// Publishes on the driver `HARNESS_DRIVER` picks until `running` clears.
fn publish(running: &AtomicBool, channel: &CStr, cpu: Option<usize>, size: usize) -> Result<(), Box<dyn Error>> {
    let lab = Lab::connect(cpu)?;
    lab.invoker().map_or_else(
        || publish_on(&lab, &External, running, channel, cpu, size),
        |duty| publish_on(&lab, &duty, running, channel, cpu, size),
    )
}

fn publish_on(
    lab: &Lab,
    duty: &impl Duty,
    running: &AtomicBool,
    channel: &CStr,
    cpu: Option<usize>,
    size: usize,
) -> Result<(), Box<dyn Error>> {
    let publication = lab.add_publication(channel, STREAM_ID)?;
    lab.wait_until("subscriber", 60, || publication.is_connected())?;
    let max_payload = publication.max_payload_length()?;
    if size > max_payload {
        return Err(format!("size {size} > max payload {max_payload}: it would fragment").into());
    }
    pin(cpu)?;
    if size < 8 {
        return Err(format!("size {size} < 8: the sequence number needs 8 bytes").into());
    }
    let mut message = vec![0u8; size];
    let mut seq = 0_u64;
    while running.load(Ordering::Relaxed) {
        message[..8].copy_from_slice(&seq.to_le_bytes());
        // an embedded driver sends what was offered, every pass
        let work = duty.run();
        match publication.offer(&message) {
            Ok(_) => seq += 1,
            Err(e) if e.is_retryable() => {
                if work == 0 {
                    spin_loop();
                }
            }
            Err(e) => return Err(format!("offer: {e:?}").into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    /// A publisher that never stops: one message a poll, numbered from 0.
    struct Endless(Cell<u64>);

    impl Poll for Endless {
        fn poll(&self, mut on_message: impl FnMut(&[u8])) -> Result<i32, AeronCError> {
            let seq = self.0.get();
            self.0.set(seq + 1);
            on_message(&seq.to_le_bytes());
            Ok(1)
        }
    }

    /// Slack for `CLOCK_REALTIME` against the clock the samples are timed by:
    /// chrony slews it by up to 83,333 ppm while correcting a large offset,
    /// 250 ms over the three seconds here. Still far from the second a
    /// missing warm-up or an extra sample would make.
    const SLACK_NS: u128 = 250_000_000;

    #[test]
    fn the_window_is_the_samples_span_on_the_wall_clock_after_the_warm_up() -> Result<(), AeronCError> {
        let before = realtime_ns();
        let measured = measure(&Endless(Cell::new(0)), 2)?;
        let after = realtime_ns();
        let (start, end) = measured.window;
        assert!(
            start + SLACK_NS >= before + 1_000_000_000,
            "the window starts after the warm-up second: {start} against {before}"
        );
        assert!(
            end <= after,
            "the window ends by the time measure returns: {end} against {after}"
        );
        // two one-second samples; the warm-up second too would make it three
        // (half a second of slack above for a busy machine's scheduling)
        assert!(
            end - start + SLACK_NS >= 2_000_000_000 && end - start <= 2_500_000_000,
            "the window spans the two one-second samples: {} ns",
            end - start
        );
        assert_eq!(measured.samples.len(), 2);
        assert!(measured.received > 0);
        assert_eq!(measured.lost, 0);
        Ok(())
    }

    #[test]
    fn the_line_appends_the_window_after_the_fields_readers_index() {
        let measured = Measured {
            samples: vec![1_000_000.0, 2_000_000.0, 3_000_000.0],
            received: 6_000_000,
            lost: 2,
            window: (1_792_032_877_000_000_000, 1_792_032_880_000_000_123),
        };
        assert_eq!(
            measured.line("isolated:dpdk"),
            "tput,isolated:dpdk,udp,2000000,1000000,3000000,6000000,2,1792032877000000000,1792032880000000123"
        );
    }
}
