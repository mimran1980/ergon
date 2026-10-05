//! A feed handler added to the registry while the engine runs live is
//! subscribed to, with no restart: the new version is the runtime's
//! directory from then on, not only the engine's list of venues.

use std::time::Duration;

use engine::{agent::Engine, replay};
use ergon_runtime::Settings;
use ergon_runtime::bus::Bus;
use ergon_runtime::clock::Clock;
use ergon_runtime::rt::{Config, Invoker};
use lab::Watch;
use rusteron_media_driver::bindings::aeron_threading_mode_t;
use rusteron_media_driver::{AeronDriver, AeronDriverContext};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The runtime's open feeds, as its `Debug` shows them.
fn feeds(runtime: &Invoker) -> Result<usize, Box<dyn std::error::Error>> {
    let shown = format!("{runtime:?}");
    let count = shown
        .split("feeds: ")
        .nth(1)
        .and_then(|rest| rest.split(',').next())
        .ok_or_else(|| format!("no feed count in {shown}"))?;
    Ok(count.trim().parse()?)
}

#[test]
fn a_feed_handler_added_to_the_registry_is_subscribed_with_no_restart() -> TestResult {
    let directory = std::env::temp_dir().join(format!("engine-registry-{}", std::process::id()));
    std::fs::create_dir_all(&directory)?;
    let driver_path = directory.join("driver");
    let context = AeronDriverContext::new()?;
    context.set_dir(&rusteron_media_driver::cformat!(
        "{}",
        driver_path.display()
    ))?;
    context.set_dir_delete_on_start(true)?;
    context.set_dir_delete_on_shutdown(true)?;
    context.set_threading_mode(aeron_threading_mode_t::AERON_THREADING_MODE_SHARED)?;
    let _driver = AeronDriver::launch_embedded_guard(context, false);
    let settings = Settings {
        aeron_dir: Some(driver_path.to_string_lossy().into_owned()),
        ..Settings::new(directory.join("tables.yaml"))
    };
    let bus = Bus::connect(&settings)?;
    let path = directory.join("streams.yaml");
    std::fs::write(&path, replay::STREAMS)?;
    let (streams, watch) = Watch::start(&path)?;
    let mut runtime = Invoker::new(Config {
        region: replay::REGION.into(),
        directory: Box::new(streams.clone()),
        ..Config::new(bus)
    })?;
    let mut engine = Engine::new(runtime.ctx(), streams)?.watching(watch);
    runtime.start(&mut engine)?;
    let before = feeds(&runtime)?;
    // A third venue's feed handler, in the far region.
    std::fs::write(
        &path,
        replay::STREAMS.replace(
            "kinds:",
            "  md-gamma: { port: 41005, region: r2, streams: { md: 108, tob: 109 } }\nkinds:",
        ),
    )?;
    let clock = Clock::new();
    let deadline = clock.now().0 + 10_000_000_000;
    while feeds(&runtime)? == before {
        runtime.cycle(&mut engine);
        assert!(
            clock.now().0 < deadline,
            "md-gamma was never subscribed to: its name did not resolve"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(feeds(&runtime)?, before + 2, "its md and tob feeds");
    runtime.finish(&mut engine)?;
    Ok(())
}
