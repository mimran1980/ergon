//! The live runtime against a real media driver: messages, one-shot and
//! repeating timers, cancel, outputs, stop, and no allocation in a steady
//! duty cycle. Every client runs its Aeron conductor in the loop that owns
//! it: the runtime's duty cycle, or a test's own wait.
//!
//! Needs an Aeron media driver at `AERON_TEST_DIR` (default
//! `target/persist-test-aeron`); `just test` starts it. Fails, never skips,
//! without it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::error::Error;
use std::time::{Duration, Instant};

use ergon_runtime::Settings;
use ergon_runtime::bus::Bus;
use ergon_runtime::clock::{Clock, Nanos};
use ergon_runtime::event::codec::{TraceDecoder, TraceWhy};
use ergon_runtime::idle::Idle;
use ergon_runtime::publication::Publication;
use ergon_runtime::rt::{Agent, Config, Ctx, Expiry, FeedId, Out, Runtime};
use ergon_runtime::subscription::{Delivery, Subscription};
use ergon_runtime::timer::TimerId;

type TestResult = Result<(), Box<dyn Error>>;

thread_local! {
    /// Count this thread's allocations only: other tests' threads are not
    /// the duty cycle.
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    /// This thread's counted allocations.
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

fn count() {
    if COUNTING.with(Cell::get) {
        ALLOCS.with(|n| n.set(n.get() + 1));
    }
}

struct Counting;

#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: the system allocator is the delegate, with the caller's layout.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: as `alloc`.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from this allocator for `layout`.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        // SAFETY: `ptr` came from this allocator for `layout`.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

const IPC: &str = "aeron:ipc?term-length=1m";
const FRAME: usize = 32;
const MS: i64 = 1_000_000;

fn aeron_dir() -> String {
    std::env::var("AERON_TEST_DIR").unwrap_or_else(|_| env!("AERON_TEST_DIR").into())
}

/// Streams unique to this test in this run.
fn stream(test: i32, n: i32) -> i32 {
    40_000 + (std::process::id() % 20_000).cast_signed() * 16 + test * 4 + n
}

/// Echoes every message to `out`, and keeps what its timers did.
#[derive(Default)]
struct Probe {
    out: Option<Out>,
    seen: u64,
    one_shot: Vec<(u64, Nanos, Nanos)>,
    repeats: u64,
    repeating: Option<TimerId>,
    cancelled_fired: bool,
    stop_after: u64,
}

impl Probe {
    const fn maybe_stop(&self, ctx: &mut Ctx) {
        if self.stop_after > 0 && self.seen >= self.stop_after && self.repeats >= 3 {
            ctx.stop();
        }
    }
}

impl Agent for Probe {
    fn start(&mut self, ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        let bad = |e: ergon_runtime::timer::TimerError| ergon_runtime::Error::Config(e.to_string());
        ctx.after(2 * MS, 1).map_err(bad)?;
        ctx.after(MS, 2).map_err(bad)?;
        let doomed = ctx.after(MS, 3).map_err(bad)?;
        assert!(ctx.cancel(doomed));
        self.repeating = Some(ctx.every(MS, 4).map_err(bad)?);
        Ok(())
    }

    fn on_message(&mut self, ctx: &mut Ctx, _feed: FeedId, msg: &[u8], _d: Delivery) {
        // The publication's `Source` message comes first; only data counts.
        if msg.get(2..4) != Some(&1u16.to_le_bytes()[..]) {
            return;
        }
        self.seen += 1;
        if let Some(out) = self.out {
            let _ = ctx.send(out, 1, FRAME, |buf| {
                buf.copy_from_slice(&msg[..FRAME]);
                Ok::<_, std::convert::Infallible>(FRAME)
            });
        }
        self.maybe_stop(ctx);
    }

    fn on_timer(&mut self, ctx: &mut Ctx, timer: Expiry) {
        match timer.token {
            4 => {
                self.repeats += 1;
                if self.repeats == 3
                    && let Some(id) = self.repeating
                {
                    assert!(
                        ctx.cancel(id),
                        "a repeating timer cancels from its own firing"
                    );
                }
            }
            3 => self.cancelled_fired = true,
            token => self.one_shot.push((token, timer.deadline, ctx.now())),
        }
        self.maybe_stop(ctx);
    }
}

/// An agent with nothing to do: the runtime still runs its conductor.
struct Quiet;

impl Agent for Quiet {
    fn start(&mut self, _ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        Ok(())
    }

    fn on_message(&mut self, _ctx: &mut Ctx, _feed: FeedId, _msg: &[u8], _d: Delivery) {}

    fn on_timer(&mut self, _ctx: &mut Ctx, _timer: Expiry) {}
}

struct Rig {
    rt: Runtime,
    feed: Publication,
    echo: Subscription,
}

/// The test's feed and echo subscription on the runtime's bus, which the
/// runtime owns and drives from then on.
fn rig(test: i32) -> Result<Rig, Box<dyn Error>> {
    let settings = Settings {
        aeron_dir: Some(aeron_dir()),
        app: "runtime-live-test".into(),
        ..Settings::new("unused.yaml")
    };
    let bus = Bus::connect(&settings)?;
    let feed = bus.publication(IPC, stream(test, 0))?;
    let echo = bus.subscription(IPC, stream(test, 1));
    let mut rt = Runtime::new(Config {
        idle: Idle::Noop,
        ..Config::new(bus)
    })?;
    rt.ctx().subscribe_channel(IPC, stream(test, 0));
    Ok(Rig { rt, feed, echo })
}

fn publish(feed: &Publication, n: u64) {
    for i in 0..n {
        let _ = feed.record(1, FRAME, |buf| {
            buf[2..4].copy_from_slice(&1u16.to_le_bytes());
            buf[8..16].copy_from_slice(&i.to_le_bytes());
            Ok::<_, std::convert::Infallible>(FRAME)
        });
    }
}

/// Drive the runtime until `done`, publishing nothing.
fn drive(
    rig: &mut Rig,
    probe: &mut Probe,
    echoed: &mut u64,
    done: impl Fn(&Probe, u64) -> bool,
) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(probe, *echoed) {
        rig.rt.cycle(probe);
        *echoed += rig.echo.poll(|_, _| {}, 256) as u64;
        if Instant::now() > deadline {
            return Err(format!(
                "not done within 10 s: seen {}, echoed {echoed}, repeats {}",
                probe.seen, probe.repeats
            )
            .into());
        }
    }
    Ok(())
}

fn connect(rig: &mut Rig, probe: &mut Probe) -> TestResult {
    let echo_stream = rig.feed.stream_id() + 1;
    let out = rig.rt.ctx().publish_channel(IPC, echo_stream)?;
    probe.out = Some(out);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(rig.feed.is_connected() && rig.rt.ctx().is_connected(out) && rig.echo.is_connected()) {
        rig.rt.cycle(probe);
        rig.echo.poll(|_, _| {}, 256);
        if Instant::now() > deadline {
            return Err("the IPC streams did not connect within 10 s".into());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    // The echo stream starts with the runtime's `Source` message.
    while rig.echo.poll(|_, _| {}, 256) > 0 {}
    Ok(())
}

#[test]
fn messages_timers_outputs_and_stop() -> TestResult {
    let mut rig = rig(0)?;
    let mut probe = Probe {
        stop_after: 100,
        ..Probe::default()
    };
    rig.rt.start(&mut probe)?;
    connect(&mut rig, &mut probe)?;
    publish(&rig.feed, 100);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !rig.rt.is_stopping() && Instant::now() < deadline {
        rig.rt.cycle(&mut probe);
    }
    assert!(
        rig.rt.is_stopping(),
        "the agent's stop did not stop the loop"
    );
    assert_eq!(probe.seen, 100);
    assert!(!probe.cancelled_fired, "a cancelled one-shot fired");
    assert_eq!(probe.repeats, 3, "a cancelled repeating timer fired again");
    let tokens: Vec<u64> = probe.one_shot.iter().map(|t| t.0).collect();
    assert_eq!(tokens, [2, 1], "one-shots fire in deadline order");
    for (token, deadline, now) in &probe.one_shot {
        assert!(now >= deadline, "timer {token} fired before its deadline");
    }
    Ok(())
}

#[test]
fn a_steady_duty_cycle_does_not_allocate() -> TestResult {
    let mut rig = rig(2)?;
    let mut probe = Probe::default();
    rig.rt.start(&mut probe)?;
    connect(&mut rig, &mut probe)?;
    let mut echoed = 0;
    publish(&rig.feed, 500);
    drive(&mut rig, &mut probe, &mut echoed, |p, e| {
        p.seen >= 500 && e >= 500 && p.repeats >= 3
    })?;
    publish(&rig.feed, 1_000);
    ALLOCS.with(|n| n.set(0));
    COUNTING.with(|c| c.set(true));
    let result = drive(&mut rig, &mut probe, &mut echoed, |p, e| {
        p.seen >= 1_500 && e >= 1_500
    });
    COUNTING.with(|c| c.set(false));
    result?;
    assert_eq!(
        ALLOCS.with(Cell::get),
        0,
        "poll -> on_message -> send -> timers allocated"
    );
    Ok(())
}

#[test]
fn the_counter_sees_an_allocation_on_the_counted_thread() {
    COUNTING.with(|c| c.set(true));
    let before = ALLOCS.with(Cell::get);
    let boxed = std::hint::black_box(Box::new([0u8; 64]));
    let after = ALLOCS.with(Cell::get);
    COUNTING.with(|c| c.set(false));
    drop(boxed);
    assert!(after > before, "the allocation gate cannot fail");
}

/// Records what a replay delivers: `(feed, ctx.now, payload index)`.
#[derive(Default)]
struct Replayed {
    seen: Vec<(u32, i64, u64)>,
}

impl Agent for Replayed {
    fn start(&mut self, _ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        Ok(())
    }

    fn on_message(&mut self, ctx: &mut Ctx, feed: FeedId, msg: &[u8], _d: Delivery) {
        if msg.get(2..4) == Some(&1u16.to_le_bytes()[..]) {
            let i = u64::from_le_bytes(msg[8..16].try_into().unwrap_or_default());
            self.seen.push((feed.0, ctx.now().0, i));
        }
    }

    fn on_timer(&mut self, _ctx: &mut Ctx, _timer: Expiry) {}
}

#[test]
fn an_archive_replay_merges_recordings_by_their_publish_stamps() -> TestResult {
    use ergon_runtime::rt::ArchiveConfig;
    use ergon_runtime::rt::sim::{Sim, SimConfig};
    use rusteron_archive::{AeronArchiveAsyncConnect, AeronArchiveContext, IntoCString};

    const CONTROL: &str = "aeron:ipc?term-length=64k";
    let (a, b) = (stream(4, 0), stream(4, 1));
    let settings = Settings {
        aeron_dir: Some(aeron_dir()),
        app: "replay-test".into(),
        ..Settings::new("unused.yaml")
    };
    let bus = Bus::connect(&settings)?;
    // Record both streams, then publish them interleaved.
    let aeron = rusteron_archive::Aeron::new(&{
        let ctx = rusteron_archive::AeronContext::new()?;
        ctx.set_dir(&aeron_dir().into_c_string())?;
        ctx
    })?;
    aeron.start()?;
    let archive_ctx = AeronArchiveContext::new()?;
    archive_ctx.set_aeron(&aeron)?;
    archive_ctx.set_control_request_channel(&CONTROL.into_c_string())?;
    archive_ctx.set_control_response_channel(&CONTROL.into_c_string())?;
    let archive = AeronArchiveAsyncConnect::new_with_aeron(&archive_ctx, &aeron)?
        .poll_blocking(Duration::from_secs(10))?;
    for id in [a, b] {
        archive.start_recording(
            c"aeron:ipc",
            id,
            rusteron_archive::SOURCE_LOCATION_LOCAL,
            false,
        )?;
    }
    let (pa, pb) = (
        bus.publication("aeron:ipc", a)?,
        bus.publication("aeron:ipc", b)?,
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(pa.is_connected() && pb.is_connected()) {
        assert!(
            Instant::now() < deadline,
            "the archive did not record the streams"
        );
        let _ = bus.poll();
        std::thread::sleep(Duration::from_millis(1));
    }
    for i in 0..20u64 {
        let publication = if i % 2 == 0 { &pa } else { &pb };
        publish_one(publication, i);
        std::thread::sleep(Duration::from_micros(50));
    }
    // Let the archive write them.
    std::thread::sleep(Duration::from_millis(500));

    let config = SimConfig {
        directory: Box::new(Ids(vec![("rec-a", a), ("rec-b", b)])),
        from: Some(Nanos(0)),
        bus: Some(bus),
        archive: Some(ArchiveConfig {
            control: CONTROL.into(),
            response: CONTROL.into(),
            replay_stream: stream(4, 2),
        }),
        ..SimConfig::new()
    };
    let mut sim = Sim::new(config, Vec::new())?;
    sim.ctx().subscribe("rec-a", "md")?;
    sim.ctx().subscribe("rec-b", "md")?;
    let mut agent = Replayed::default();
    sim.run(&mut agent)?;
    let order: Vec<u64> = agent.seen.iter().map(|s| s.2).collect();
    assert_eq!(
        order,
        (0..20).collect::<Vec<_>>(),
        "merged by publish stamp"
    );
    assert!(
        agent
            .seen
            .iter()
            .all(|&(feed, _, i)| feed == u32::from(i % 2 == 1))
    );
    assert!(
        agent.seen.windows(2).all(|w| w[0].1 < w[1].1)
            && agent.seen[0].1 > 1_700_000_000_000_000_000,
        "the event time is each frame's epoch publish stamp: {:?}",
        agent.seen
    );
    Ok(())
}

/// A directory of IPC feeds by service name.
struct Ids(Vec<(&'static str, i32)>);

impl ergon_runtime::directory::Directory for Ids {
    fn feed(
        &self,
        service: &str,
        _kind: &str,
        _host_ip: &str,
    ) -> Result<ergon_runtime::directory::FeedAddr, ergon_runtime::Error> {
        let (_, stream_id) = self
            .0
            .iter()
            .find(|(name, _)| *name == service)
            .ok_or_else(|| ergon_runtime::Error::Config(format!("no feed {service}")))?;
        Ok(ergon_runtime::directory::FeedAddr {
            stream_id: *stream_id,
            live: "aeron:ipc".into(),
            archive: None,
        })
    }

    fn publication(
        &self,
        service: &str,
        _kind: &str,
        _host_ip: &str,
    ) -> Result<ergon_runtime::directory::PubAddr, ergon_runtime::Error> {
        Err(ergon_runtime::Error::Config(format!(
            "{service} publishes nothing here"
        )))
    }
}

#[test]
fn a_publication_names_its_source_before_its_first_frame() -> TestResult {
    let settings = Settings {
        aeron_dir: Some(aeron_dir()),
        app: "runtime-live-test".into(),
        ..Settings::new("unused.yaml")
    };
    let bus = Bus::connect(&settings)?;
    let channel = stream(6, 0);
    // Added once it is first polled, after the first frame.
    let mut subscriber = bus.subscription(IPC, channel);
    let mut rt = Runtime::new(Config {
        idle: Idle::Noop,
        ..Config::new(bus)
    })?;
    rt.start(&mut Quiet)?;
    let out = rt.ctx().publish_channel(IPC, channel)?;
    let send = |rt: &Runtime| {
        rt.ctx_ref().send(out, 1, FRAME, |buf| {
            buf[2..4].copy_from_slice(&1u16.to_le_bytes());
            Ok::<_, std::convert::Infallible>(FRAME)
        })
    };
    // No subscriber yet: neither its Source message nor this frame is taken.
    send(&rt)?;
    let drops = rt.ctx_ref().drops();
    assert!(drops.not_connected > 0, "{drops:?}");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(rt.ctx_ref().is_connected(out) && subscriber.is_connected()) {
        rt.cycle(&mut Quiet);
        subscriber.poll(|_, _| {}, 16);
        assert!(Instant::now() < deadline, "the subscriber did not connect");
        std::thread::sleep(Duration::from_millis(1));
    }
    send(&rt)?;
    let mut templates = Vec::new();
    while templates.len() < 2 {
        rt.cycle(&mut Quiet);
        subscriber.poll(
            |message, _| templates.push(u16::from_le_bytes([message[2], message[3]])),
            16,
        );
        assert!(Instant::now() < deadline, "only {templates:?} arrived");
    }
    assert_eq!(
        templates,
        [ergon_runtime::source::SOURCE_TEMPLATE_ID, 1],
        "its recording names its publisher before its first frame"
    );
    Ok(())
}

/// Persist's stream, read back: the `Source` message before anything, a
/// shape before its first row, and both again in the round `poll` starts
/// every 5 s. In simulation a row is stamped with the simulated time, and
/// `flush_sim` finishes the round under way.
#[test]
fn persist_sends_its_dictionary_ahead_of_rows_and_again_from_poll() -> TestResult {
    use ergon_runtime::event::{ROW_TEMPLATE_ID, SCHEMA_ID, SHAPE_TEMPLATE_ID, Shape};
    use ergon_runtime::persist::Persist;
    use ergon_runtime::source::SOURCE_TEMPLATE_ID;

    const SECOND: i64 = 1_000_000_000;
    let config = std::env::temp_dir().join(format!("runtime-live-{}.yaml", std::process::id()));
    std::fs::write(&config, "tables:\n  spread: { kind: dynamic }\n")?;
    for sim_start in [None, Some(Nanos(1_700_000_000_000_000_000))] {
        let stream_id = stream(7, i32::from(sim_start.is_some()));
        let settings = Settings {
            aeron_dir: Some(aeron_dir()),
            app: "runtime-live-test".into(),
            channel: IPC.into(),
            stream_id,
            subscriber_timeout: Duration::ZERO,
            sim_start,
            ..Settings::new(&config)
        };
        let bus = Bus::connect(&settings)?;
        let mut taken = bus.subscription(IPC, stream_id);
        let persist = Persist::connect(include_str!("../schema/events.xml"), &bus, settings)?;
        let deadline = Instant::now() + Duration::from_secs(10);
        while !(persist.is_connected() && taken.is_connected()) {
            let _ = bus.poll();
            taken.poll(|_, _| {}, 16);
            assert!(
                Instant::now() < deadline,
                "persist's stream did not connect"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        persist.record_value("spread", &(1.5_f64, 7_u64));
        if sim_start.is_some() {
            persist.flush_sim(Nanos(1_700_000_000_000_000_000))?;
        } else {
            // A second of the loop's time a poll: the fifth starts a round.
            let base = Clock::new().now().0;
            for k in 1..=5 {
                persist.poll(Nanos(base + k * SECOND));
            }
        }
        let expected = if sim_start.is_some() { 4 } else { 6 };
        let mut messages: Vec<Vec<u8>> = Vec::new();
        while messages.len() < expected {
            let _ = bus.poll();
            taken.poll(
                |message, _| {
                    let id = |at: usize| {
                        message
                            .get(at..at + 2)
                            .map(|b| u16::from_le_bytes([b[0], b[1]]))
                    };
                    let wanted = [SOURCE_TEMPLATE_ID, SHAPE_TEMPLATE_ID, ROW_TEMPLATE_ID];
                    if id(4) == Some(SCHEMA_ID) && id(2).is_some_and(|t| wanted.contains(&t)) {
                        messages.push(message.to_vec());
                    }
                },
                16,
            );
            assert!(
                Instant::now() < deadline,
                "only {} messages",
                messages.len()
            );
        }
        let templates: Vec<u16> = messages
            .iter()
            .map(|m| u16::from_le_bytes([m[2], m[3]]))
            .collect();
        let (source, shape, row) = (SOURCE_TEMPLATE_ID, SHAPE_TEMPLATE_ID, ROW_TEMPLATE_ID);
        // Then the round due at start, whose `Source` the row's went out
        // for; live, the fifth second's, all of it.
        let round: &[u16] = if sim_start.is_some() {
            &[shape]
        } else {
            &[shape, source, shape]
        };
        assert_eq!(templates[..3], [source, shape, row], "sim {sim_start:?}");
        assert_eq!(templates[3..], *round, "sim {sim_start:?}");
        let decoded = Shape::decode(&messages[1]).ok_or("undecodable shape")?;
        let ts = decoded
            .decode_row(&messages[2])
            .ok_or("undecodable row")?
            .ts;
        if let Some(start) = sim_start {
            assert_eq!(
                ts,
                start.0.cast_unsigned(),
                "stamped with the simulated time"
            );
        }
    }
    Ok(())
}

/// `Persist::poll` allocates nothing on the loop once warm while
/// `tables.yaml` is unchanged: its once-a-second re-read is one `stat`, and
/// applying the switches again copies nothing. Only the polls are counted;
/// the subscriber that keeps the stream connected is drained between them.
#[test]
fn a_steady_persist_poll_does_not_allocate() -> TestResult {
    use ergon_runtime::persist::Persist;

    const SECOND: i64 = 1_000_000_000;
    let config =
        std::env::temp_dir().join(format!("runtime-live-poll-{}.yaml", std::process::id()));
    std::fs::write(
        &config,
        "tables:\n  spread: { kind: dynamic }\n  otel_traces:\n    kind: static\n    traces: { t2t: { sample: 10 } }\n",
    )?;
    let stream_id = stream(9, 0);
    let settings = Settings {
        aeron_dir: Some(aeron_dir()),
        app: "runtime-live-test".into(),
        channel: IPC.into(),
        stream_id,
        subscriber_timeout: Duration::ZERO,
        ..Settings::new(&config)
    };
    let bus = Bus::connect(&settings)?;
    let mut taken = bus.subscription(IPC, stream_id);
    let persist = Persist::connect(include_str!("../schema/events.xml"), &bus, settings)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(persist.is_connected() && taken.is_connected()) {
        let _ = bus.poll();
        taken.poll(|_, _| {}, 16);
        assert!(
            Instant::now() < deadline,
            "persist's stream did not connect"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let drain = |taken: &mut Subscription| {
        let _ = bus.poll();
        while taken.poll(|_, _| {}, 256) > 0 {}
    };
    // A second of the loop's time a poll. The first ten read the file, start
    // two heartbeat rounds and publish the metrics twice.
    let base = Clock::new().now().0;
    for k in 1..=10 {
        persist.poll(Nanos(base + k * SECOND));
        drain(&mut taken);
    }
    ALLOCS.with(|n| n.set(0));
    for k in 11..=20 {
        COUNTING.with(|c| c.set(true));
        persist.poll(Nanos(base + k * SECOND));
        COUNTING.with(|c| c.set(false));
        drain(&mut taken);
    }
    assert_eq!(
        ALLOCS.with(Cell::get),
        0,
        "Persist::poll allocated in a second that changed nothing"
    );
    Ok(())
}

/// A simulation that records through persist must own the bus persist's
/// client runs on: nothing else would drive that client, and the driver
/// would close it after 10 s.
#[test]
fn a_simulation_recording_without_its_bus_is_refused() -> TestResult {
    use ergon_runtime::persist::Persist;
    use ergon_runtime::rt::sim::{Sim, SimConfig};

    let config = std::env::temp_dir().join(format!("runtime-live-sim-{}.yaml", std::process::id()));
    std::fs::write(&config, "tables:\n  spread: { kind: dynamic }\n")?;
    let settings = Settings {
        aeron_dir: Some(aeron_dir()),
        app: "runtime-live-test".into(),
        channel: IPC.into(),
        stream_id: stream(9, 1),
        subscriber_timeout: Duration::ZERO,
        ..Settings::new(&config)
    };
    let bus = Bus::connect(&settings)?;
    let persist = Persist::connect(include_str!("../schema/events.xml"), &bus, settings)?;
    let without = SimConfig {
        persist: Some(persist.clone()),
        ..SimConfig::new()
    };
    assert!(
        Sim::new(without, Vec::new()).is_err(),
        "persist without the bus that drives it"
    );
    let with = SimConfig {
        persist: Some(persist),
        bus: Some(bus),
        ..SimConfig::new()
    };
    Sim::new(with, Vec::new())?;
    Ok(())
}

/// Waits until `taken` has delivered `want` `Shape` messages of `table`,
/// counted in `seen`, running `bus`'s conductor.
fn shapes_of(
    bus: &Bus,
    taken: &mut Subscription,
    table: &str,
    want: usize,
    seen: &mut usize,
) -> TestResult {
    use ergon_runtime::event::{SCHEMA_ID, SHAPE_TEMPLATE_ID, Shape};

    let deadline = Instant::now() + Duration::from_secs(10);
    while *seen < want {
        let _ = bus.poll();
        taken.poll(
            |message, _| {
                let id = |at: usize| {
                    message
                        .get(at..at + 2)
                        .map(|b| u16::from_le_bytes([b[0], b[1]]))
                };
                if id(4) == Some(SCHEMA_ID)
                    && id(2) == Some(SHAPE_TEMPLATE_ID)
                    && Shape::decode(message).is_some_and(|shape| shape.table == table)
                {
                    *seen += 1;
                }
            },
            16,
        );
        if Instant::now() >= deadline {
            return Err(format!("{seen} of {want} {table} shapes arrived").into());
        }
    }
    Ok(())
}

thread_local! {
    /// This thread's errors saying a message is longer than a claim.
    static REFUSED: Cell<usize> = const { Cell::new(0) };
}

/// Counts [`REFUSED`].
struct Refusals;

impl log::Log for Refusals {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() == log::Level::Error
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata())
            && record
                .args()
                .to_string()
                .contains("more than one claim holds")
        {
            REFUSED.with(|n| n.set(n.get() + 1));
        }
    }

    fn flush(&self) {}
}

/// A shape too long for one claim never reaches the stream, and the round
/// `poll` starts every 5 s goes on past it: the shapes after it go again.
/// It is logged once, when it is made, not at each row or round.
#[test]
fn a_shape_aeron_refuses_does_not_hold_back_the_round() -> TestResult {
    use ergon_runtime::event::Value;
    use ergon_runtime::persist::Persist;

    const SECOND: i64 = 1_000_000_000;
    log::set_logger(&Refusals)?;
    log::set_max_level(log::LevelFilter::Error);
    let config =
        std::env::temp_dir().join(format!("runtime-live-round-{}.yaml", std::process::id()));
    std::fs::write(
        &config,
        "tables:\n  wide: { kind: dynamic }\n  narrow: { kind: dynamic }\n",
    )?;
    let stream_id = stream(8, 0);
    let settings = Settings {
        aeron_dir: Some(aeron_dir()),
        app: "runtime-live-test".into(),
        // No `mtu`: one claim holds about 1.4 KB.
        channel: IPC.into(),
        stream_id,
        subscriber_timeout: Duration::ZERO,
        ..Settings::new(&config)
    };
    let bus = Bus::connect(&settings)?;
    let mut taken = bus.subscription(IPC, stream_id);
    let persist = Persist::connect(include_str!("../schema/events.xml"), &bus, settings)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(persist.is_connected() && taken.is_connected()) {
        let _ = bus.poll();
        taken.poll(|_, _| {}, 16);
        assert!(
            Instant::now() < deadline,
            "persist's stream did not connect"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    // The round due at start, before either shape exists.
    let base = Clock::new().now().0;
    persist.poll(Nanos(base));
    let names: Vec<String> = (0..64)
        .map(|i| format!("a_field_name_of_forty_characters_{i:07}"))
        .collect();
    persist.record_row("wide", names.iter().map(|n| (n.as_str(), Value::I64(1))));
    assert_eq!(persist.drops().too_large, 1, "the wide shape fits a claim");
    persist.record_row("wide", names.iter().map(|n| (n.as_str(), Value::I64(2))));
    assert_eq!(persist.drops().too_large, 2, "each row of it is dropped");
    persist.record_row("narrow", [("x", Value::I64(1))]);
    let mut narrow = 0;
    shapes_of(&bus, &mut taken, "narrow", 1, &mut narrow)?;
    // A second of the loop's time a poll: the fifth starts a round.
    for k in 1..=5 {
        persist.poll(Nanos(base + k * SECOND));
    }
    shapes_of(&bus, &mut taken, "narrow", 2, &mut narrow)?;
    assert_eq!(
        REFUSED.with(Cell::get),
        1,
        "the wide shape is logged once, not at each row or round"
    );
    Ok(())
}

/// Once `Invoker::finish` closed persist's publication, a record is not
/// published and not a drop, whatever stopped it: a shape or a row longer
/// than a claim, as before, or the closed publication.
#[test]
fn a_record_after_finish_is_not_a_drop() -> TestResult {
    use ergon_runtime::event::Value;
    use ergon_runtime::persist::Persist;

    let config =
        std::env::temp_dir().join(format!("runtime-live-finish-{}.yaml", std::process::id()));
    std::fs::write(
        &config,
        "tables:\n  wide: { kind: dynamic }\n  long: { kind: dynamic }\n  narrow: { kind: dynamic }\n",
    )?;
    let stream_id = stream(3, 0);
    let settings = Settings {
        aeron_dir: Some(aeron_dir()),
        app: "runtime-live-test".into(),
        // No `mtu`: one claim holds about 1.4 KB.
        channel: IPC.into(),
        stream_id,
        subscriber_timeout: Duration::ZERO,
        ..Settings::new(&config)
    };
    let bus = Bus::connect(&settings)?;
    let mut taken = bus.subscription(IPC, stream_id);
    let persist = Persist::connect(include_str!("../schema/events.xml"), &bus, settings)?;
    let mut rt = Runtime::new(Config {
        persist: Some(persist.clone()),
        idle: Idle::Noop,
        ..Config::new(bus)
    })?;
    rt.start(&mut Quiet)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(persist.is_connected() && taken.is_connected()) {
        rt.cycle(&mut Quiet);
        taken.poll(|_, _| {}, 16);
        assert!(
            Instant::now() < deadline,
            "persist's stream did not connect"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let names: Vec<String> = (0..64)
        .map(|i| format!("a_field_name_of_forty_characters_{i:07}"))
        .collect();
    let wide = || names.iter().map(|n| (n.as_str(), Value::I64(1)));
    let text = "x".repeat(4096);
    // Before it: each is a drop, its `Source` message already out.
    persist.record_row("wide", wide());
    persist.record_row("long", [("text", Value::Str(&text))]);
    let before = persist.drops();
    assert_eq!(
        before.too_large, 2,
        "the wide shape, then the long row: {before:?}"
    );
    rt.finish(&mut Quiet)?;
    persist.record_row("wide", wide());
    persist.record_row("long", [("text", Value::Str(&text))]);
    persist.record_row("narrow", [("x", Value::I64(1))]);
    assert_eq!(
        persist.drops(),
        before,
        "a record after finish counted a drop"
    );
    Ok(())
}

/// A `Trace` message as the ingester reads it.
#[derive(Debug, PartialEq)]
struct Published {
    def: u64,
    id: u64,
    why: TraceWhy,
    marks: Vec<i64>,
}

impl Published {
    /// `message`, if it is a `Trace` of persist's schema.
    fn decode(message: &[u8]) -> Option<Self> {
        let trace = TraceDecoder::decode(message, 0).ok()?;
        Some(Self {
            def: trace.def(),
            id: trace.trace_lo(),
            why: trace.why(),
            marks: trace.marks().ok()?.map(|mark| mark.ns()).collect(),
        })
    }
}

/// Every `Trace` message `taken` delivers, running `bus`'s conductor, until
/// the one with trace definition `def` and id `last` arrives. Persist
/// publishes in order on one publication, so none before it is still to come.
fn traces_until(
    bus: &Bus,
    taken: &mut Subscription,
    (def, last): (u64, u64),
) -> Result<Vec<Published>, Box<dyn Error>> {
    let mut published: Vec<Published> = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !published.iter().any(|p| p.def == def && p.id == last) {
        let _ = bus.poll();
        taken.poll(
            |message, _| published.extend(Published::decode(message)),
            16,
        );
        if Instant::now() >= deadline {
            return Err(
                format!("trace {last} never arrived within 10 s; taken: {published:?}").into(),
            );
        }
    }
    Ok(published)
}

/// `tables.yaml`'s `sample` and `slower_than` reach the tracer persist makes
/// (what `Ctx::tracer` returns). Under `sample: 10`, 100 traces of no time at
/// all publish the 1st, 11th, ... 91st, with the watcher applying the rules
/// again every second meanwhile. Under `sample: 0, slower_than: 1ms`, a 2 ms
/// trace is published, as slow, and a 0 ms one is not.
#[test]
fn tables_yaml_sampling_and_slower_than_reach_a_tracer() -> TestResult {
    use ergon_runtime::persist::Persist;
    use ergon_runtime::trace::{TraceDef, TraceId, Tracer};

    const SECOND: i64 = 1_000_000_000;
    const NAMESPACE: u64 = TraceId::namespace("runtime-live");
    let config =
        std::env::temp_dir().join(format!("runtime-live-sampling-{}.yaml", std::process::id()));
    std::fs::write(
        &config,
        "tables:\n  otel_traces:\n    kind: static\n    traces:\n      t: { sample: 10 }\n      slow: { sample: 0, slower_than: 1ms }\n",
    )?;
    let stream_id = stream(1, 0);
    let settings = Settings {
        aeron_dir: Some(aeron_dir()),
        app: "runtime-live-test".into(),
        channel: IPC.into(),
        stream_id,
        subscriber_timeout: Duration::ZERO,
        ..Settings::new(&config)
    };
    let bus = Bus::connect(&settings)?;
    let mut taken = bus.subscription(IPC, stream_id);
    let persist = Persist::connect(include_str!("../schema/events.xml"), &bus, settings)?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while !(persist.is_connected() && taken.is_connected()) {
        let _ = bus.poll();
        taken.poll(|_, _| {}, 16);
        assert!(
            Instant::now() < deadline,
            "persist's stream did not connect"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    let stages = ["stage"];
    let head = persist.tracer("t", &stages, &[]);
    let slow = persist.tracer("slow", &stages, &[]);
    // A trace of exactly `total` ns, from a fixed time, under an id of its own.
    let base = Clock::new().now().0;
    let run = |tracer: &Tracer, id: u64, total: i64| {
        let mut trace = tracer.start(Nanos(base), TraceId::new(NAMESPACE, id));
        trace.mark(Nanos(base + total));
        trace.finish();
    };
    // A second of the loop's time after each 25: the watcher applies every
    // switch again in the middle of the countdown.
    for second in 1..=4_u64 {
        for id in (second - 1) * 25..second * 25 {
            run(&head, id, 0);
        }
        persist.poll(Nanos(base + second.cast_signed() * SECOND));
    }
    run(&slow, 1, 0);
    run(&slow, 2, 2 * MS);
    let head_def = TraceDef::new("t", &stages, &[]).def;
    let slow_def = TraceDef::new("slow", &stages, &[]).def;
    let (head_traces, slow_traces): (Vec<_>, Vec<_>) =
        traces_until(&bus, &mut taken, (slow_def, 2))?
            .into_iter()
            .partition(|trace| trace.def == head_def);
    let ids: Vec<u64> = head_traces.iter().map(|trace| trace.id).collect();
    assert_eq!(
        ids,
        [0, 10, 20, 30, 40, 50, 60, 70, 80, 90],
        "one in ten, the first included"
    );
    assert!(
        head_traces
            .iter()
            .all(|trace| trace.why == TraceWhy::Sampled && trace.marks == [0]),
        "{head_traces:?}"
    );
    assert_eq!(
        slow_traces,
        [Published {
            def: slow_def,
            id: 2,
            why: TraceWhy::Slow,
            marks: vec![2 * MS],
        }],
        "the 2 ms trace only, as slow"
    );
    assert_eq!(
        persist.dropped(),
        0,
        "a trace not taken is not one Aeron refused"
    );
    Ok(())
}

fn publish_one(publication: &Publication, i: u64) {
    let _ = publication.record(1, FRAME, |buf| {
        buf[2..4].copy_from_slice(&1u16.to_le_bytes());
        buf[8..16].copy_from_slice(&i.to_le_bytes());
        Ok::<_, std::convert::Infallible>(FRAME)
    });
}

#[test]
fn a_feed_whose_name_does_not_resolve_never_stalls_the_loop() -> TestResult {
    use ergon_runtime::directory::{ArchiveAddr, FeedAddr};

    let settings = Settings {
        aeron_dir: Some(aeron_dir()),
        app: "runtime-live-test".into(),
        ..Settings::new("unused.yaml")
    };
    let bus = Bus::connect(&settings)?;
    // A name that fails at once (`.invalid`, RFC 6761). The media driver
    // resolves the channels' names, and the shared Java test driver does it
    // on its conductor thread: a `.local` name, which macOS asks mDNS for
    // and gives up on after five seconds, would stall every other test on
    // that driver. The lab's C driver runs DEDICATED, where names resolve
    // on the native resource agent's own thread.
    let addr = FeedAddr {
        stream_id: stream(5, 0),
        live: "aeron:udp?endpoint=127.0.0.1:0|control=nowhere.invalid:40999|control-mode=dynamic"
            .into(),
        archive: Some(ArchiveAddr {
            host: "nowhere.invalid".into(),
            port: 8010,
            publisher_port: 40999,
        }),
    };
    let mut sub = bus.subscribe("nowhere/md", &addr)?;
    // However long the lookup takes, each poll returns at once.
    for _ in 0..200 {
        let started = Instant::now();
        sub.poll(|_, _| {}, 16);
        assert!(
            started.elapsed() < Duration::from_millis(50),
            "a poll blocked"
        );
        let _ = bus.poll();
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

/// A persistent feed whose archive never answers holds at most one connect
/// open, however often it retries. Each attempt runs to Aeron's own timeout,
/// which closes its request publication and response subscription. The
/// lookup used to give up first, on its own clock, and rusteron frees
/// nothing for an unfinished connect, so every retry left both open. On the
/// lab they piled up into the hundreds, and loaded a node's media driver
/// until its archive answered no one.
#[test]
fn a_feed_whose_archive_never_answers_leaves_no_connect_open() -> TestResult {
    use ergon_runtime::directory::{ArchiveAddr, FeedAddr};
    use rusteron_archive::{Aeron, AeronContext, IntoCString};

    /// Aeron's `AERON_COUNTER_PUBLISHER_LIMIT_TYPE_ID`: one per open publication.
    const PUBLISHER_LIMIT: i32 = 1;
    let settings = Settings {
        aeron_dir: Some(aeron_dir()),
        app: "runtime-live-test".into(),
        ..Settings::new("unused.yaml")
    };
    let bus = Bus::connect(&settings)?;
    // Nothing listens there: each connect waits out Aeron's 5 s timeout,
    // and the lookup tries again a second later.
    let archive = "127.0.0.1:40997";
    let addr = FeedAddr {
        stream_id: stream(7, 0),
        live: "aeron:udp?endpoint=127.0.0.1:0|control=127.0.0.1:40996|control-mode=dynamic".into(),
        archive: Some(ArchiveAddr {
            host: "127.0.0.1".into(),
            port: 40997,
            publisher_port: 40996,
        }),
    };
    let mut sub = bus.subscribe("silent/md", &addr)?;
    // Another client reads the driver's counters.
    let ctx = AeronContext::new()?;
    ctx.set_dir(&aeron_dir().into_c_string())?;
    let reader = Aeron::new(&ctx)?;
    reader.start()?;
    let counters = reader.counters_reader();
    let open = || {
        let mut n = 0;
        counters.foreach_counter_fn(|_, _, type_id, _, label| {
            if type_id == PUBLISHER_LIMIT && label.contains(archive) {
                n += 1;
            }
        });
        n
    };
    // Four attempts and more: a leak holds one publication per attempt.
    let (mut most, deadline) = (0, Instant::now() + Duration::from_secs(25));
    while Instant::now() < deadline {
        sub.poll(|_, _| {}, 16);
        let _ = bus.poll();
        most = most.max(open());
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        most > 0,
        "no connect was ever counted: the count reads the wrong counters"
    );
    // The attempt in flight, and the one before it while the driver lingers
    // on its closed publication (5 s).
    assert!(
        most <= 2,
        "{most} connects to the silent archive were open at once"
    );
    Ok(())
}
