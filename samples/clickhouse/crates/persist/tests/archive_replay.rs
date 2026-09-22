//! Real-Archive integration: launches the LATEST Aeron
//! (`aeron-all-1.53.2`) ArchivingMediaDriver with default archive
//! settings, records envelopes through a producer publication, replays
//! them back byte-for-byte, then exercises post-checkpoint pruning:
//! active recordings via `purge_segments`, stopped recordings via
//! `truncate_recording`.
//!
//! Requires a local JDK (>=17) and the pinned jar under `target/aeron/`.

#![allow(missing_docs)]

use std::io::Read;

use ergo_clickhouse_persist::ingest::archive::{
    ArchiveClient, ArchiveServer, prune_after_checkpoint, prune_position,
};
use ergo_clickhouse_persist::protocol::encode_data_record;
use ergo_clickhouse_persist::recording::RecordKind;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

const STREAM_ID: i32 = 42;
const SEGMENT_LENGTH: usize = 65536; // lab-sized segments
const N_RECORDS: usize = 500;
const RECORD_BYTES: usize = 64;

/// Resolve (downloading once into the user cache if needed) the pinned
/// LATEST Aeron jar. Cached outside `target/` so `cargo clean` cannot
/// delete it.
fn jar_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    const VERSION: &str = "1.53.2";
    let file_name = format!("aeron-all-{VERSION}.jar");
    let mut candidates = vec![
        PathBuf::from("target/aeron").join(&file_name),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/aeron")
            .join(&file_name),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/aeron")
            .join(&file_name),
    ];
    let cache = std::env::home_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(".cache/ergo/aeron");
    candidates.push(cache.join(&file_name));
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    // Download from Maven Central into the cache.
    std::fs::create_dir_all(&cache)?;
    let url = format!("https://repo1.maven.org/maven2/io/aeron/aeron-all/{VERSION}/{file_name}");
    let resp = ureq::get(&url).call()?;
    let mut bytes = Vec::new();
    resp.into_reader().read_to_end(&mut bytes)?;
    let dest = cache.join(&file_name);
    std::fs::write(&dest, &bytes)?;
    eprintln!(
        "downloaded {} ({} bytes) to {}",
        url,
        bytes.len(),
        dest.display()
    );
    Ok(dest)
}

#[test]
#[serial_test::serial]
fn driver_basic_ipc_pub_sub_works() -> Result<(), Box<dyn std::error::Error>> {
    // Sanity: can the C client do a basic IPC pub/sub on the archive's
    // driver at all? Isolates replay from transport problems.
    let jar = jar_path()?;
    let base = std::env::temp_dir().join(format!("ergo-archive-basic-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base)?;
    let mut server =
        ArchiveServer::launch(jar.to_str().unwrap(), &base, STREAM_ID, SEGMENT_LENGTH)?;
    std::thread::sleep(Duration::from_secs(3));
    if let Ok(Some(status)) = server.child.try_wait() {
        panic!("jvm exited early: {status}");
    }
    let driver_dir = server.aeron_dir.clone();
    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&rusteron_client::cformat!("{}", driver_dir.display()))?;
    let aeron = rusteron_client::Aeron::new(&ctx)?;
    aeron.start()?;

    let received: std::rc::Rc<std::cell::Cell<usize>> = Default::default();
    struct Count(Rc<std::cell::Cell<usize>>);
    impl rusteron_client::AeronFragmentHandlerCallback for Count {
        fn handle_aeron_fragment_handler(&mut self, _buf: &[u8], _h: rusteron_client::AeronHeader) {
            self.0.set(self.0.get() + 1);
        }
    }
    let handler = rusteron_client::Handler::new(Count(std::rc::Rc::clone(&received)));
    let sub = aeron.add_subscription(
        &rusteron_client::cformat!("aeron:ipc"),
        777,
        rusteron_client::Handlers::NONE,
        rusteron_client::Handlers::NONE,
        Duration::from_secs(5),
    )?;
    let pub1 = aeron.add_publication(
        &rusteron_client::cformat!("aeron:ipc"),
        777,
        Duration::from_secs(5),
    )?;

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut offered = 0;
    while offered < 20 && Instant::now() < deadline {
        let env = make_envelope(offered as u64 + 1);
        if pub1.offer_raw(&env, rusteron_client::Handlers::NONE) > 0 {
            offered += 1;
        }
        sub.poll(Some(&handler), 100)?;
        std::thread::sleep(Duration::from_millis(5));
    }
    eprintln!("basic ipc: offered={offered} received={}", received.get());
    assert_eq!(
        received.get(),
        20,
        "basic IPC pub/sub must work on the archive driver"
    );
    Ok(())
}

fn make_envelope(sequence: u64) -> Vec<u8> {
    let payload: Vec<u8> = (0..RECORD_BYTES)
        .map(|b| ((sequence + b as u64) % 251) as u8)
        .collect();
    let mut buf = vec![0u8; 4096];
    let len = encode_data_record(
        &mut buf,
        RecordKind::RawBytes,
        1,
        1,
        1,
        sequence,
        1_758_000_000_000_000_000 + sequence,
        2,
        &payload,
    )
    .expect("encode");
    buf[..len].to_vec()
}

#[test]
#[serial_test::serial]
fn archive_record_replay_and_prune() -> Result<(), Box<dyn std::error::Error>> {
    let jar = jar_path()?;
    let base = std::env::temp_dir().join(format!("ergo-archive-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base)?;

    let jvm_log = base.join("archive-jvm.log");
    let mut server = ArchiveServer::launch_with_log(
        jar.to_str().expect("jar path"),
        &base,
        STREAM_ID,
        SEGMENT_LENGTH,
        Some(&jvm_log),
    )?;
    std::thread::sleep(Duration::from_secs(3)); // JVM + driver + archive startup
    if let Ok(Some(status)) = server.child.try_wait() {
        let log = std::fs::read_to_string(&jvm_log).unwrap_or_default();
        panic!("archive JVM exited early ({status}): {log}");
    }

    // All clients point at the DRIVER's aeron dir (CnC file lives there).
    let driver_dir = server.aeron_dir.clone();
    let ctx = rusteron_client::AeronContext::new()?;
    ctx.set_dir(&rusteron_client::cformat!("{}", driver_dir.display()))?;
    let aeron = rusteron_client::Aeron::new(&ctx)?;
    aeron.start()?;

    // Archive client (separate Aeron handle; same driver dir).
    let client = ArchiveClient::connect(&server.endpoints, &driver_dir)
        .map_err(|e| format!("archive connect failed (is java >= 17 available?): {e}"))?;

    // Start recording the data channel.
    let subscription_id = client.archive.start_recording(
        &rusteron_archive::cformat!("{}", server.endpoints.recorded_channel),
        STREAM_ID,
        rusteron_archive::SOURCE_LOCATION_LOCAL,
        true,
    )?;
    assert!(subscription_id >= 0);

    // Producer publication + offer N envelopes.
    let publication = aeron.add_exclusive_publication(
        &rusteron_client::cformat!("{}", server.endpoints.recorded_channel),
        STREAM_ID,
        Duration::from_secs(30),
    )?;
    // The archive's recording subscription must present an image before
    // offers connect.
    let connect_deadline = Instant::now() + Duration::from_secs(30);
    while !publication.is_connected() && Instant::now() < connect_deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        publication.is_connected(),
        "publication never connected; recording info: {:?}; jvm log: {}",
        client.find_recording(&server.endpoints),
        std::fs::read_to_string(&jvm_log).unwrap_or_default()
    );
    let _ = &jvm_log;
    let envelopes: Vec<Vec<u8>> = (1..=N_RECORDS as u64).map(make_envelope).collect();
    let mut published = 0;
    let mut last_rc = 0i64;
    let deadline = Instant::now() + Duration::from_secs(30);
    while published < envelopes.len() && Instant::now() < deadline {
        for env in &envelopes[published..] {
            last_rc = publication.offer_raw(env, rusteron_client::Handlers::NONE);
            if last_rc > 0 {
                published += 1;
            } else {
                std::thread::sleep(Duration::from_millis(20));
                break;
            }
        }
    }
    assert_eq!(
        published,
        N_RECORDS,
        "all envelopes must be offered (last offer rc={last_rc}, errmsg={})",
        rusteron_client::Aeron::errmsg()
    );

    // Wait for the archive to record everything (recording position counter
    // or simply stop recording and read the stop position from the catalog).
    std::thread::sleep(Duration::from_millis(1500));
    client
        .archive
        .stop_recording_subscription(subscription_id)?;
    let mut info = None;
    for _ in 0..50 {
        if let Some(found) = client.find_recording(&server.endpoints) {
            info = Some(found);
            if !found.is_active() {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let info = info.ok_or("recording not found in archive catalog")?;
    assert!(
        !info.is_active(),
        "recording stopped: stop_position={}",
        info.stop_position
    );
    let recorded_bytes = info.stop_position - info.start_position;
    eprintln!(
        "recorded: start={} stop={} bytes={recorded_bytes} ({} envelopes worth)",
        info.start_position,
        info.stop_position,
        recorded_bytes / envelopes[0].len() as i64
    );
    assert!(
        recorded_bytes >= (envelopes[0].len() * N_RECORDS) as i64,
        "recorded {recorded_bytes} bytes — expected at least {} for {N_RECORDS} envelopes",
        envelopes[0].len() * N_RECORDS
    );

    // ── replay: start_replay from the recording start, verify bytes ────
    let replay_stream_id = STREAM_ID + 100;
    // Replay exactly the recorded range. Param order:
    // (bounding_limit_counter_id, file_io_max_length, position, length, ...).
    // bounding_limit_counter_id = -1 means UNBOUNDED — passing 0 binds the
    // replay to counter 0, which silently caps it (seen in the archive
    // event log as CMD_IN_START_BOUNDED_REPLAY limitCounterId=0).
    let replay_length = info.stop_position - info.start_position;
    let params = rusteron_archive::AeronArchiveReplayParams::new(
        -1,
        i32::MAX,
        info.start_position,
        replay_length,
        0,
        0,
    )?;
    let replay_session_id = client.archive.start_replay(
        info.recording_id,
        &rusteron_archive::cformat!("{}", server.endpoints.replay),
        replay_stream_id,
        &params,
    )?;
    assert!(replay_session_id > 0);

    let received: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    let _expected: Rc<Cell<usize>> = Rc::new(Cell::new(0));
    struct Verify {
        received: Rc<Cell<usize>>,
        total: Rc<Cell<usize>>,
        first_mismatch: Rc<Cell<i64>>,
        envelopes: Vec<Vec<u8>>,
    }
    impl rusteron_client::AeronFragmentHandlerCallback for Verify {
        fn handle_aeron_fragment_handler(
            &mut self,
            buffer: &[u8],
            _header: rusteron_client::AeronHeader,
        ) {
            self.total.set(self.total.get() + 1);
            let i = self.received.get();
            if i < self.envelopes.len() {
                if buffer == self.envelopes[i].as_slice() {
                    self.received.set(i + 1);
                } else if self.first_mismatch.get() < 0 {
                    self.first_mismatch.set(i as i64);
                }
            }
        }
    }
    let total = Rc::new(Cell::new(0));
    let first_mismatch = Rc::new(Cell::new(-1i64));
    let verifier = Verify {
        received: Rc::clone(&received),
        total: Rc::clone(&total),
        first_mismatch: Rc::clone(&first_mismatch),
        envelopes: envelopes.clone(),
    };
    let handler = rusteron_client::Handler::new(verifier);
    // Session-id binding: the archive derives the replay stream session
    // from the replay session id. ERGO_REPLAY_SESSION_ID=off disables the
    // binding (wildcard) for diagnosis.
    let bind = std::env::var("ERGO_REPLAY_SESSION_ID").as_deref() != Ok("off");
    let channel = if bind {
        let base = &server.endpoints.replay;
        let sep = if base.contains('?') { "|" } else { "?" };
        rusteron_client::cformat!("{}{}session-id={}", base, sep, replay_session_id as i32)
    } else {
        rusteron_client::cformat!("{}", server.endpoints.replay)
    };
    let sub = aeron.add_subscription(
        &channel,
        replay_stream_id,
        rusteron_client::Handlers::NONE,
        rusteron_client::Handlers::NONE,
        Duration::from_secs(10),
    )?;

    let replay_deadline = Instant::now() + Duration::from_secs(60);
    while received.get() < N_RECORDS && Instant::now() < replay_deadline {
        let n = sub.poll(Some(&handler), 100)?;
        if n == 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    eprintln!(
        "replay poll loop ended: received={} of {N_RECORDS}",
        received.get()
    );
    assert_eq!(
        received.get(),
        N_RECORDS,
        "replay delivered every recorded envelope byte-for-byte \
         (total fragments={}, first mismatch at={})",
        total.get(),
        first_mismatch.get(),
    );

    // ── prune the STOPPED recording (truncate) ──────────────────────────
    // Acknowledged checkpoint = everything ingested; keep one segment.
    let ack = info.stop_position;
    let target = prune_position(ack, SEGMENT_LENGTH, info.start_position);
    let segments = client.truncate_recording(info.recording_id, target)?;
    // After truncation the recording's stop position moved back.
    let mut after = None;
    for _ in 0..50 {
        if let Some(found) = client.find_recording(&server.endpoints) {
            after = Some(found);
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let after = after.ok_or("recording vanished after truncate")?;
    assert!(
        after.stop_position <= target,
        "truncated stop position {stop} <= target {target}",
        stop = after.stop_position,
    );
    let _ = (segments, target);

    // ── prune an ACTIVE recording (purge segments) ──────────────────────
    // Record a second stream: keep recording live and purge old segments.
    let sub2 = client.archive.start_recording(
        &rusteron_archive::cformat!("{}", server.endpoints.recorded_channel),
        STREAM_ID + 1,
        rusteron_archive::SOURCE_LOCATION_LOCAL,
        true,
    )?;
    let pub2 = aeron.add_exclusive_publication(
        &rusteron_client::cformat!("{}", server.endpoints.recorded_channel),
        STREAM_ID + 1,
        Duration::from_secs(10),
    )?;
    let mut pushed = 0;
    let purge_deadline = Instant::now() + Duration::from_secs(30);
    // Push well past two segment lengths so a purge can remove a segment.
    while pushed < 4096 && Instant::now() < purge_deadline {
        for env in &envelopes[pushed % envelopes.len()..pushed % envelopes.len() + 1] {
            if pub2.offer_raw(env, rusteron_client::Handlers::NONE) > 0 {
                pushed += 1;
            } else {
                std::thread::sleep(Duration::from_millis(10));
                break;
            }
        }
    }
    std::thread::sleep(Duration::from_millis(1500));
    let info2 = (0..50)
        .find_map(|_| {
            client
                .find_recording(&server.endpoints)
                .filter(|i| i.recording_id != info.recording_id || i.is_active())
        })
        .or_else(|| client.find_recording(&server.endpoints))
        .ok_or("second recording not found")?;
    let purge_from = prune_position(
        info2.stop_position.max(0),
        SEGMENT_LENGTH,
        info2.start_position,
    );
    if purge_from > info2.start_position {
        let purged = client.purge_segments(info2.recording_id, purge_from)?;
        let after2 = client
            .find_recording(&server.endpoints)
            .expect("recording still listed");
        assert!(
            after2.start_position >= purge_from,
            "active recording start advanced to {start} >= {purge_from}",
            start = after2.start_position,
        );
        assert!(after2.is_active(), "purged recording is still active");
        let _ = purged;
    }
    client.archive.stop_recording_subscription(sub2)?;

    // Prune helper on an already-pruned range is a no-op.
    let noop = prune_after_checkpoint(&client, &after, target, SEGMENT_LENGTH)?;
    assert_eq!(noop, 0, "no further segments to prune behind the window");
    Ok(())
}

/// The registration sink and the data writer must share ONE publication, so a
/// layout declaration and the rows that depend on it reach the ingester in
/// order. Two publications could not be ordered, and without the declaration
/// the ingester rejects every row with `MissingCatalog`.
///
/// This drives the real session (not a hand-built publication) so the wiring
/// itself is what is under test: `TransportConfig::AeronShared` plus
/// `AeronSink` over one `SharedPublication`.
#[test]
#[serial_test::serial]
fn declarations_and_rows_share_one_ordered_stream() -> Result<(), Box<dyn std::error::Error>> {
    use ergo_clickhouse_persist::persist::{EncodeError, Persistable, RecordOutcome, RowWriter};
    use ergo_clickhouse_persist::protocol::{Declaration, Policy};
    use ergo_clickhouse_persist::recorder::AeronPublication;
    use ergo_clickhouse_persist::registration::{
        AeronSink, RecorderConfig, RecorderSession, TransportConfig,
    };
    use ergo_clickhouse_persist::schema::{RowSchema, TypeCode, ValueSchema};

    #[derive(Clone, Copy)]
    struct Probe {
        instrument: u32,
        price: i64,
    }
    static PROBE: RowSchema = RowSchema {
        columns: &[
            ValueSchema::scalar(TypeCode::U32),
            ValueSchema::scalar(TypeCode::I64),
        ],
    };
    impl Persistable for Probe {
        fn schema() -> &'static RowSchema {
            &PROBE
        }
        fn encoded_len(&self) -> Result<usize, EncodeError> {
            Ok(12)
        }
        fn encode(&self, out: &mut RowWriter<'_>) -> Result<(), EncodeError> {
            out.set_raw(0, &self.instrument.to_le_bytes())?;
            out.set_raw(1, &self.price.to_le_bytes())?;
            Ok(())
        }
    }

    let jar = jar_path()?;
    let base = std::env::temp_dir().join(format!("ergo-shared-pub-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base)?;
    let jvm_log = base.join("archive-jvm.log");
    let mut server = ArchiveServer::launch_with_log(
        jar.to_str().expect("jar path"),
        &base,
        STREAM_ID,
        SEGMENT_LENGTH,
        Some(&jvm_log),
    )?;
    std::thread::sleep(Duration::from_secs(3));
    if let Ok(Some(status)) = server.child.try_wait() {
        panic!(
            "archive JVM exited early ({status}): {}",
            std::fs::read_to_string(&jvm_log).unwrap_or_default()
        );
    }

    let driver_dir = server.aeron_dir.clone();
    let client = ArchiveClient::connect(&server.endpoints, &driver_dir)?;
    let subscription_id = client.archive.start_recording(
        &rusteron_archive::cformat!("{}", server.endpoints.recorded_channel),
        STREAM_ID,
        rusteron_archive::SOURCE_LOCATION_LOCAL,
        true,
    )?;
    assert!(subscription_id >= 0);

    // `AeronPublication::connect` reads AERON_DIR; the session's shared
    // publication must attach to the driver the archive is recording.
    // SAFETY: `#[serial]` keeps this the only test touching the environment.
    unsafe { std::env::set_var("AERON_DIR", driver_dir.display().to_string()) };

    let publication = Rc::new(std::cell::RefCell::new(AeronPublication::connect(
        &server.endpoints.recorded_channel,
        STREAM_ID,
    )?));
    let sink = Box::new(AeronSink::new(Rc::clone(&publication)));
    let config = RecorderConfig {
        permanent: TransportConfig::AeronShared(Rc::clone(&publication)),
        // Diagnostics stay in memory: this test is about the permanent stream.
        diagnostics: TransportConfig::Memory {
            slots: 8,
            slot_bytes: 4096,
        },
        ..RecorderConfig::default()
    };

    // Connect while the recording subscription exists, then declare and write.
    let connect_deadline = Instant::now() + Duration::from_secs(30);
    while !publication.borrow().is_connected() && Instant::now() < connect_deadline {
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        publication.borrow().is_connected(),
        "shared publication never connected; jvm log: {}",
        std::fs::read_to_string(&jvm_log).unwrap_or_default()
    );

    let mut session = RecorderSession::connect_with(config, sink)?;
    session.declare_session_start()?;
    let table = session.table::<Probe>("shared_pub_probe", Policy::Permanent)?;
    table.slot().set_enabled(table.policy_id(), 1);
    let mut writer = session.writer()?;
    for i in 0..10u32 {
        assert_eq!(
            writer.record(
                &table,
                &Probe {
                    instrument: i,
                    price: i as i64 * 100,
                },
                u64::from(i),
            ),
            RecordOutcome::Published
        );
    }

    std::thread::sleep(Duration::from_millis(1500));

    // Replay while the recording is STILL ACTIVE. That is the live case — a
    // live recorder never stops — and refusing it made the whole live path
    // unreplayable: `--follow` died on "recording N is still active
    // (stop_position -1)" and CrashLooped, so no live byte ever reached
    // ClickHouse while the recorder pods published happily. An active
    // recording is bounded at wherever it has been written, so this must
    // deliver the declaration and the rows above without waiting for a stop.
    // Scoped: the source borrows `client`, which the stop below wants back.
    {
        let active = client
            .find_recording(&server.endpoints)
            .ok_or("recording not found while active")?;
        assert!(
            active.is_active(),
            "the recording must still be active or this assertion proves nothing"
        );
        let live_budget = ergo_clickhouse_persist::ingest::ReplayBudget::default();
        let mut live = ergo_clickhouse_persist::ingest::archive::ArchiveReplaySource::subscribe(
            &driver_dir,
            &server.endpoints.replay,
            STREAM_ID + 200,
            &client,
            &active,
            active.start_position,
        )?;
        use ergo_clickhouse_persist::ingest::IngestSource as _;
        let mut live_items = Vec::new();
        let live_deadline = Instant::now() + Duration::from_secs(30);
        while live_items.len() < 11 && Instant::now() < live_deadline {
            match live.next_batch(&live_budget)? {
                Some(batch) => live_items.extend(batch.items),
                None => std::thread::sleep(Duration::from_millis(20)),
            }
        }
        assert!(
            live_items.len() >= 11,
            "an active recording must replay what has been written so far, got {}",
            live_items.len()
        );
    }

    client
        .archive
        .stop_recording_subscription(subscription_id)?;
    let mut info = None;
    for _ in 0..50 {
        if let Some(found) = client.find_recording(&server.endpoints) {
            info = Some(found);
            if !found.is_active() {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let info = info.ok_or("recording not found in archive catalog")?;
    assert!(!info.is_active(), "recording must be stopped");

    // Replay through the production source: it owns the fragment assembler and
    // classifies each frame, so the assertion is about ORDER, not plumbing.
    let budget = ergo_clickhouse_persist::ingest::ReplayBudget::default();
    let mut source = ergo_clickhouse_persist::ingest::archive::ArchiveReplaySource::subscribe(
        &driver_dir,
        &server.endpoints.replay,
        STREAM_ID + 100,
        &client,
        &info,
        info.start_position,
    )?;

    use ergo_clickhouse_persist::ingest::IngestSource as _;
    let mut items = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    while items.len() < 11 && Instant::now() < deadline {
        match source.next_batch(&budget)? {
            Some(batch) => items.extend(batch.items),
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    assert!(
        items.len() >= 11,
        "expected the declaration plus 10 rows, got {}",
        items.len()
    );
    assert!(
        matches!(
            items[0],
            ergo_clickhouse_persist::ingest::SourceItem::Declaration(Declaration::SessionStart(_))
        ),
        "the session-start declaration must be the first item on the stream"
    );
    let rows = items
        .iter()
        .filter(|i| matches!(i, ergo_clickhouse_persist::ingest::SourceItem::Record(_)))
        .count();
    assert_eq!(rows, 10, "all ten rows must be on the same stream");

    // Leave no global state behind for the other serialized archive tests.
    // SAFETY: `#[serial]` keeps this the only test touching the environment.
    unsafe { std::env::remove_var("AERON_DIR") };
    Ok(())
}
