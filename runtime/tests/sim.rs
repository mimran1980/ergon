//! The simulation driver, with no Aeron and no `ClickHouse`.

use std::error::Error;

use ergon_runtime::clock::Nanos;
use ergon_runtime::frames::{self, FrameLog};
use ergon_runtime::rt::sim::{Sim, SimConfig};
use ergon_runtime::rt::{Agent, Ctx, Expiry, FeedId, Out};
use ergon_runtime::streams::Streams;
use ergon_runtime::subscription::Delivery;

type TestResult = Result<(), Box<dyn Error>>;

const MS: i64 = 1_000_000;
const T0: i64 = 1_700_000_000_000_000_000;

#[derive(Debug, PartialEq, Eq)]
enum Seen {
    Message {
        feed: u32,
        ts: i64,
        byte: u8,
        first: bool,
    },
    Timer {
        token: u64,
        ts: i64,
        deadline: i64,
        missed: u64,
    },
}

/// Logs what it sees; subscribes to `late` when timer 9 fires; echoes every
/// message to `out` with a fresh id.
#[derive(Default)]
struct Probe {
    seen: Vec<Seen>,
    out: Option<Out>,
    late: Option<FeedId>,
    timers: Vec<(i64, u64)>,
    stop_on: Option<u8>,
}

impl Agent for Probe {
    fn start(&mut self, ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        let bad = |e: ergon_runtime::timer::TimerError| ergon_runtime::Error::Config(e.to_string());
        for &(delay, token) in &self.timers {
            ctx.after(delay, token).map_err(bad)?;
        }
        Ok(())
    }

    fn on_message(&mut self, ctx: &mut Ctx, feed: FeedId, msg: &[u8], d: Delivery) {
        assert_eq!(
            ctx.read(),
            ctx.now(),
            "simulated time does not advance within an event"
        );
        assert_eq!(ctx.wall_ns(), ctx.now());
        assert!(d.is_live(), "a simulated message is live to the agent");
        self.seen.push(Seen::Message {
            feed: feed.0,
            ts: ctx.now().0,
            byte: msg[0],
            first: d.first,
        });
        if let Some(out) = self.out {
            let id = ctx.next_id();
            let _ = ctx.send(out, 0, 9, |buf| {
                buf[0] = msg[0];
                buf[1..9].copy_from_slice(&id.to_le_bytes());
                Ok::<_, std::convert::Infallible>(9)
            });
        }
        if self.stop_on == Some(msg[0]) {
            ctx.stop();
        }
    }

    fn on_timer(&mut self, ctx: &mut Ctx, timer: Expiry) {
        self.seen.push(Seen::Timer {
            token: timer.token,
            ts: ctx.now().0,
            deadline: timer.deadline.0,
            missed: timer.missed,
        });
        if timer.token == 9 {
            self.late = ctx.subscribe("late", "md").ok();
        }
    }
}

fn streams() -> Result<Streams, ergon_runtime::Error> {
    Streams::parse("services: {}\nkinds: {}\n")
}

/// `a/md` at T0, T0+1ms (twice) and T0+3ms; `b/md` at T0+1ms; `late/md` at
/// T0 and T0+4ms.
fn input() -> Vec<u8> {
    let mut log = FrameLog::new();
    let (a, b, late) = (
        log.stream("a/md"),
        log.stream("b/md"),
        log.stream("late/md"),
    );
    log.push(a, Nanos(T0), b"A");
    log.push(late, Nanos(T0), b"L");
    log.push(a, Nanos(T0 + MS), b"B");
    log.push(b, Nanos(T0 + MS), b"C");
    log.push(a, Nanos(T0 + MS), b"D");
    log.push(a, Nanos(T0 + 3 * MS), b"E");
    log.push(late, Nanos(T0 + 4 * MS), b"F");
    log.to_bytes()
}

fn run(probe: &mut Probe, config: SimConfig) -> Result<FrameLog, Box<dyn Error>> {
    let mut sim = Sim::new(config, vec![input()])?;
    let ctx = sim.ctx();
    ctx.subscribe("a", "md")?;
    ctx.subscribe("b", "md")?;
    probe.out = Some(ctx.publish("echo", "out")?);
    sim.run(probe)?;
    Ok(sim.ctx().captured())
}

#[test]
fn events_and_timers_merge_in_one_total_order() -> TestResult {
    let mut probe = Probe {
        // Strictly earlier than B, equal to B's time, and after E.
        timers: vec![(MS / 2, 1), (MS, 2), (3 * MS + 1, 3), (2 * MS, 9)],
        ..Probe::default()
    };
    run(&mut probe, SimConfig::new(streams()?))?;
    let message = |feed, ts, byte, first| Seen::Message {
        feed,
        ts,
        byte,
        first,
    };
    let timer = |token, ts| Seen::Timer {
        token,
        ts,
        deadline: ts,
        missed: 0,
    };
    assert_eq!(
        probe.seen,
        [
            message(0, T0, b'A', true),
            timer(1, T0 + MS / 2),
            // Events at an equal time go first, in feed order, then timers.
            message(0, T0 + MS, b'B', false),
            message(0, T0 + MS, b'D', false),
            message(1, T0 + MS, b'C', true),
            timer(2, T0 + MS),
            // Subscribed here: takes `late` from now on, not its earlier frame.
            timer(9, T0 + 2 * MS),
            message(0, T0 + 3 * MS, b'E', false),
            timer(3, T0 + 3 * MS + 1),
            message(2, T0 + 4 * MS, b'F', true),
        ]
    );
    assert_eq!(probe.late, Some(FeedId(2)));
    Ok(())
}

#[test]
fn two_runs_capture_the_same_bytes_and_ids_count_from_one() -> TestResult {
    let first = run(&mut Probe::default(), SimConfig::new(streams()?))?;
    let second = run(&mut Probe::default(), SimConfig::new(streams()?))?;
    assert_eq!(first.to_bytes(), second.to_bytes());
    let bytes = first.to_bytes();
    let parsed = frames::parse(&bytes)?;
    assert_eq!(parsed.names, ["echo/out"]);
    let echoed: Vec<(i64, u8, u64)> = parsed
        .records
        .map(|r| {
            let id = u64::from_le_bytes(r.frame[1..9].try_into().unwrap_or_default());
            (r.ts.0, r.frame[0], id)
        })
        .collect();
    assert_eq!(
        echoed,
        [
            (T0, b'A', 1),
            (T0 + MS, b'B', 2),
            (T0 + MS, b'D', 3),
            (T0 + MS, b'C', 4),
            (T0 + 3 * MS, b'E', 5),
        ]
    );
    Ok(())
}

#[test]
fn from_to_and_stop_bound_the_run() -> TestResult {
    let mut windowed = Probe::default();
    let config = SimConfig {
        from: Some(Nanos(T0 + MS)),
        to: Some(Nanos(T0 + 2 * MS)),
        ..SimConfig::new(streams()?)
    };
    run(&mut windowed, config)?;
    let bytes: Vec<u8> = windowed
        .seen
        .iter()
        .filter_map(|s| match s {
            Seen::Message { byte, .. } => Some(*byte),
            Seen::Timer { .. } => None,
        })
        .collect();
    assert_eq!(bytes, b"BDC");

    let mut stopped = Probe {
        stop_on: Some(b'B'),
        ..Probe::default()
    };
    run(&mut stopped, SimConfig::new(streams()?))?;
    assert_eq!(stopped.seen.len(), 2, "A, then B stops the run");
    Ok(())
}

#[test]
fn a_paced_run_takes_its_simulated_time_over_the_speed() -> TestResult {
    // Input spans 4 ms of simulated time; at 1x it takes at least that long.
    let started = std::time::Instant::now();
    let config = SimConfig {
        speed: Some(1.0),
        ..SimConfig::new(streams()?)
    };
    run(&mut Probe::default(), config)?;
    assert!(started.elapsed() >= std::time::Duration::from_millis(3));
    Ok(())
}

#[test]
fn route_delay_reorders_feeds_before_timer_merge() -> TestResult {
    let mut config = SimConfig::new(streams()?);
    config.route_delays.insert("a/md".into(), 2 * MS);
    let mut probe = Probe::default();
    run(&mut probe, config)?;
    let messages: Vec<_> = probe
        .seen
        .iter()
        .filter_map(|s| match s {
            Seen::Message { byte, ts, .. } => Some((*byte, *ts)),
            Seen::Timer { .. } => None,
        })
        .collect();
    assert_eq!(
        messages,
        [
            (b'C', T0 + MS),
            (b'A', T0 + 2 * MS),
            (b'B', T0 + 3 * MS),
            (b'D', T0 + 3 * MS),
            (b'E', T0 + 5 * MS)
        ]
    );
    Ok(())
}

// An echo agent publishes only on the recorded input, not its loopback.
struct Echo(Probe);
impl Agent for Echo {
    fn start(&mut self, ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        self.0.out = Some(ctx.publish("echo", "out")?);
        Ok(())
    }
    fn on_message(&mut self, ctx: &mut Ctx, feed: FeedId, msg: &[u8], d: Delivery) {
        self.0.stop_on = None;
        let out = self.0.out;
        if feed == FeedId(1) {
            self.0.out = None;
        }
        self.0.on_message(ctx, feed, msg, d);
        self.0.out = out;
    }
    fn on_timer(&mut self, _: &mut Ctx, _: Expiry) {}
}

#[test]
fn loopback_delivers_captured_output_after_configured_latency() -> TestResult {
    let mut config = SimConfig::new(streams()?);
    config.loopback.insert("echo/out".into(), MS / 2);
    config.to = Some(Nanos(T0 + MS));
    let mut log = FrameLog::new();
    let feed = log.stream("a/md");
    log.push(feed, Nanos(T0), b"A");
    let mut sim = Sim::new(config, vec![log.to_bytes()])?;
    sim.ctx().subscribe("a", "md")?;
    sim.ctx().subscribe("echo", "out")?;
    let mut probe = Probe {
        stop_on: Some(b'A'),
        ..Probe::default()
    };
    probe.stop_on = None;
    let mut echo = Echo(probe);
    sim.run(&mut echo)?;
    assert_eq!(
        echo.0.seen,
        [
            Seen::Message {
                feed: 0,
                ts: T0,
                byte: b'A',
                first: true
            },
            Seen::Message {
                feed: 1,
                ts: T0 + MS / 2,
                byte: b'A',
                first: true
            },
        ]
    );
    assert_eq!(sim.ctx().captured().records().count(), 1);
    Ok(())
}

#[test]
fn exact_journal_preserves_live_delivery_order_and_late_timer() -> TestResult {
    use ergon_runtime::journal::{Input, InputEvent, Journal};
    use ergon_runtime::subscription::Origin;
    let data = input();
    let mut parsed = frames::parse(&data)?;
    let a = parsed
        .records
        .clone()
        .find(|r| r.frame == b"B")
        .ok_or("B missing")?;
    let b = parsed
        .records
        .find(|r| r.frame == b"C")
        .ok_or("C missing")?;
    let message = |feed, record: frames::Record<'_>, ts, sequence| Input {
        sequence,
        wall_offset: 0,
        next_id: sequence,
        ts: Nanos(ts),
        event: InputEvent::Message {
            feed: FeedId(feed),
            recording: 0,
            position: i64::try_from(record.offset).unwrap_or_default(),
            session: 0,
            stream: 0,
            delivery: Delivery {
                first: true,
                origin: Origin::Live,
            },
        },
    };
    let config = SimConfig {
        from: Some(Nanos(T0)),
        journal: Some(Journal {
            inputs: vec![
                // A live journal opens with the context `start` saw.
                Input {
                    sequence: 0,
                    wall_offset: 0,
                    next_id: 0,
                    ts: Nanos(T0),
                    event: InputEvent::Start,
                },
                message(1, b, T0 + 2 * MS, 1),
                message(0, a, T0 + 2 * MS, 2),
                Input {
                    sequence: 3,
                    wall_offset: 0,
                    next_id: 3,
                    ts: Nanos(T0 + 3 * MS),
                    event: InputEvent::Timer {
                        token: 8,
                        deadline: Nanos(T0 + MS),
                        missed: 0,
                    },
                },
            ],
        }),
        ..SimConfig::new(streams()?)
    };
    let mut probe = Probe {
        timers: vec![(MS, 8)],
        ..Probe::default()
    };
    run(&mut probe, config)?;
    assert_eq!(
        probe.seen,
        [
            Seen::Message {
                feed: 1,
                ts: T0 + 2 * MS,
                byte: b'C',
                first: true
            },
            Seen::Message {
                feed: 0,
                ts: T0 + 2 * MS,
                byte: b'B',
                first: true
            },
            Seen::Timer {
                token: 8,
                ts: T0 + 3 * MS,
                deadline: T0 + MS,
                missed: 0
            },
        ]
    );
    Ok(())
}

#[test]
fn runtime_selects_the_replay_driver_from_mode() -> TestResult {
    use ergon_runtime::rt::{Mode, Runtime};
    let mut rt = Runtime::new(Mode::Replay {
        config: SimConfig::new(streams()?),
        logs: vec![input()],
    })?;
    rt.ctx().subscribe("a", "md")?;
    let probe = rt.run(Probe::default())?;
    assert_eq!(probe.seen.len(), 4);
    Ok(())
}

#[test]
fn malformed_simulated_output_fails_the_run_instead_of_losing_a_frame() -> TestResult {
    struct BadOutput;
    impl Agent for BadOutput {
        fn start(&mut self, ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
            let out = ctx.publish("bad", "output")?;
            let _: Result<(), std::convert::Infallible> = ctx.send(out, 0, 8, |_| Ok(7));
            Ok(())
        }
        fn on_message(&mut self, _: &mut Ctx, _: FeedId, _: &[u8], _: Delivery) {}
        fn on_timer(&mut self, _: &mut Ctx, _: Expiry) {}
    }
    let mut sim = Sim::new(SimConfig::new(streams()?), Vec::new())?;
    assert!(sim.run(&mut BadOutput).is_err());
    assert!(sim.ctx().captured().is_empty());
    Ok(())
}
