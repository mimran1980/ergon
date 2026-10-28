//! The load point's own rules, without a transport: what a receiver counts,
//! what fails a point, and the spikes file's `# max=` line.

use ergon_latency_core::clock::TscClock;
use ergon_latency_core::manifest::RunManifest;
use ergon_latency_core::message::{Kind, Message, template};
use ergon_latency_core::pace::Pacer;
use io_bench::transport::point::{
    Common, Error, Flags, Max, Recorder, Report, SendStats, expected_per_stream, finish,
    with_max_line,
};
use std::path::{Path, PathBuf};

type R = Result<(), Box<dyn std::error::Error>>;

fn common(out: &Path, streams: u32) -> Result<Common, Error> {
    let args = [
        "--role",
        "local",
        "--rate",
        "1000",
        "--warmup-s",
        "0",
        "--duration-s",
        "1",
        "--label",
        "1:test",
        "--streams",
        &streams.to_string(),
        "--out",
        out.to_str().unwrap_or_default(),
    ];
    let mut f = Flags::parse(args.iter().map(ToString::to_string))?;
    let c = Common::parse(&mut f)?;
    f.finish()?;
    Ok(c)
}

fn scratch(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let dir = std::env::temp_dir().join(format!("point-{name}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(dir)
}

/// Sends `n` slots on `c`'s schedule from `start`, the messages they carry.
fn sent(
    clock: &TscClock,
    c: &Common,
    start: u64,
    n: u64,
) -> Result<(SendStats, Vec<Message>), Box<dyn std::error::Error>> {
    let mut pacer = Pacer::new(clock, start, c.rate, c.streams, 0, c.measured_slots()?)?;
    let mut msgs = Vec::new();
    for _ in 0..n {
        let slot = pacer.poll(u64::MAX).ok_or("a slot")?;
        let at = slot.intended_tsc;
        pacer.sent(clock, &slot, at)?;
        msgs.push(Message {
            seq: slot.seq,
            send_tsc: at,
            intended_tsc: slot.intended_tsc,
            stream: slot.stream,
            kind: Kind::Oneway,
        });
    }
    let end = pacer.due(n);
    Ok((
        SendStats {
            pacer,
            backpressured: 0,
            admin_actions: 0,
            send_end_tsc: end,
        },
        msgs,
    ))
}

fn report() -> Report<'static> {
    Report {
        channel: "ipc",
        retrans: None,
        wire_bytes: 64,
        busy_threads: 2,
        hot_path_allocs: 0,
        manifest: RunManifest::new("test", &template(), 64),
    }
}

/// A duplicate is the loss tracker's to count, never a second latency.
#[test]
fn a_duplicate_is_counted_once_and_adds_no_sample() -> R {
    let out = scratch("dup")?;
    let c = common(&out, 2)?;
    let clock = TscClock::new();
    let start = clock.raw();
    let (send, msgs) = sent(&clock, &c, start, 1000)?;
    let mut rec = Recorder::new(&clock, start, &c)?;
    for m in &msgs {
        rec.on_payload(&m.to_bytes(), m.intended_tsc + 100);
    }
    rec.on_payload(&msgs[10].to_bytes(), msgs[10].intended_tsc + 100);
    assert_eq!(rec.received(), 1000);
    let end = send.send_end_tsc;
    let row = finish(&c, &clock, rec, send, 0, end, report())?;
    let cols: Vec<&str> = row.split(',').collect();
    // samples (13), received (15), lost (23), dup (25)
    assert_eq!(
        (cols[13], cols[15], cols[23], cols[25]),
        ("1000", "1000", "0", "1"),
        "{row}"
    );
    std::fs::remove_dir_all(out)?;
    Ok(())
}

/// A payload that is not the expected message fails the point: no row, no
/// files.
#[test]
fn a_refused_payload_fails_the_point() -> R {
    let out = scratch("refused")?;
    let c = common(&out, 1)?;
    let clock = TscClock::new();
    let start = clock.raw();
    let (send, msgs) = sent(&clock, &c, start, 10)?;
    let mut rec = Recorder::new(&clock, start, &c)?;
    for m in &msgs {
        rec.on_payload(&m.to_bytes(), m.intended_tsc);
    }
    let echo = Message {
        kind: Kind::Echo,
        ..msgs[0]
    };
    rec.on_payload(&echo.to_bytes(), echo.intended_tsc);
    rec.on_payload(&[0u8; 63], 0);
    let end = send.send_end_tsc;
    match finish(&c, &clock, rec, send, 0, end, report()) {
        Err(Error::Payload { count: 2, .. }) => {}
        other => return Err(format!("{other:?}").into()),
    }
    assert!(!out.join("results.csv").exists());
    Ok(())
}

/// What the schedule owed and never sent is in the latency, censored, and
/// the max line names the worst of them.
#[test]
fn slots_never_sent_are_censored_and_can_be_the_max() -> R {
    let out = scratch("owed")?;
    let c = common(&out, 1)?;
    let clock = TscClock::new();
    let start = clock.raw();
    let (send, msgs) = sent(&clock, &c, start, 500)?;
    let mut rec = Recorder::new(&clock, start, &c)?;
    for m in &msgs {
        rec.on_payload(&m.to_bytes(), m.intended_tsc + clock.ns_to_ticks(1000));
    }
    // the run ends when the whole second is due: slots 500..1000 never went
    let end = send.pacer.due(1000);
    let row = finish(&c, &clock, rec, send, 0, end, report())?;
    let cols: Vec<&str> = row.split(',').collect();
    // samples (13), censored (21), lost (23)
    assert_eq!(
        (cols[13], cols[21], cols[23]),
        ("1000", "500", "0"),
        "{row}"
    );
    let spikes = std::fs::read_to_string(out.join("spikes-1:test.csv"))?;
    let max: u64 = cols[11].parse()?;
    assert!(max >= 499_000_000, "{row}");
    assert!(spikes.contains(&format!(",{max},0\n")), "{spikes}");
    std::fs::remove_dir_all(out)?;
    Ok(())
}

#[test]
fn the_max_line_follows_the_two_header_comments() {
    let csv = "# start_realtime_ns=5\n# spikes_total=0\nstream,due_ns,latency_ns,ping\n";
    let max = Max {
        due_ns: 7,
        latency_ns: 9,
        stream: 3,
    };
    assert_eq!(
        with_max_line(csv, Some(max)),
        "# start_realtime_ns=5\n# spikes_total=0\n# max=7,9,3\nstream,due_ns,latency_ns,ping\n"
    );
    assert_eq!(with_max_line(csv, None), csv);
}

#[test]
fn slots_share_out_round_robin() {
    assert_eq!(expected_per_stream(10, 4), [3, 3, 2, 2]);
    assert_eq!(expected_per_stream(0, 2), [0, 0]);
}
