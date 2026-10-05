//! SIGTERM stops the loop through its pipe, and the process survives it.
//!
//! In a test binary of its own: the handler is the process's, and the
//! signal goes to the whole process. Needs an Aeron media driver at
//! `AERON_TEST_DIR` (default `/tmp/persist-test-aeron`); `just test` starts
//! it. Fails, never skips, without it.

use std::error::Error;
use std::time::Duration;

use ergon_runtime::Settings;
use ergon_runtime::bus::Bus;
use ergon_runtime::idle::Idle;
use ergon_runtime::rt::{self, Agent, Config, Ctx, Expiry, FeedId, Runtime, Stop};
use ergon_runtime::subscription::Delivery;

type TestResult = Result<(), Box<dyn Error>>;

/// An agent with nothing to do: only the runtime's housekeeping runs.
struct Quiet;

impl Agent for Quiet {
    fn start(&mut self, _ctx: &mut Ctx) -> Result<(), ergon_runtime::Error> {
        Ok(())
    }

    fn on_message(&mut self, _ctx: &mut Ctx, _feed: FeedId, _msg: &[u8], _d: Delivery) {}

    fn on_timer(&mut self, _ctx: &mut Ctx, _timer: Expiry) {}
}

/// A started runtime on a bus of its own, stopped from outside by `stop`.
fn runtime(stop: Stop) -> Result<Runtime, Box<dyn Error>> {
    let settings = Settings {
        aeron_dir: Some(
            std::env::var("AERON_TEST_DIR").unwrap_or_else(|_| "/tmp/persist-test-aeron".into()),
        ),
        app: "sigterm-test".into(),
        ..Settings::new("unused.yaml")
    };
    let mut rt = Runtime::new(Config {
        stop,
        idle: Idle::Noop,
        ..Config::new(Bus::connect(&settings)?)
    })?;
    rt.start(&mut Quiet)?;
    Ok(rt)
}

#[test]
fn sigterm_stops_the_loop_through_its_pipe_and_the_process_survives() -> TestResult {
    let mut signalled = runtime(rt::sigterm()?)?;
    let mut deaf = runtime(Stop::none())?;
    // Two housekeeping periods with no signal: neither stops.
    for _ in 0..2 {
        std::thread::sleep(Duration::from_millis(10));
        signalled.cycle(&mut Quiet);
        deaf.cycle(&mut Quiet);
    }
    assert!(!signalled.is_stopping(), "stopped with no signal");
    assert!(!deaf.is_stopping(), "Stop::none stopped with no signal");

    // The handler writes the pipe before `raise` returns. One 10 ms period
    // later, the first cycle with no work reads it.
    signal_hook::low_level::raise(signal_hook::consts::SIGTERM)?;
    std::thread::sleep(Duration::from_millis(20));
    for _ in 0..3 {
        signalled.cycle(&mut Quiet);
        deaf.cycle(&mut Quiet);
    }
    assert!(
        signalled.is_stopping(),
        "SIGTERM did not stop the loop within three cycles of its pipe's period"
    );
    assert!(!deaf.is_stopping(), "Stop::none stopped on SIGTERM");

    // Still running: the handler replaced SIGTERM's default action.
    signalled.finish(&mut Quiet)?;
    deaf.finish(&mut Quiet)?;
    Ok(())
}
