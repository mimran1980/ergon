//! Aeron Archive -> ClickHouse: replay the recorded stream from the
//! checkpoint, insert, save the checkpoint, purge what is behind it.
//!
//! Every run of an application is its own recording (a new Aeron session).
//! Recordings are ingested oldest first; a stopped one is deleted once all of
//! it is in ClickHouse, and the active one is followed live and has its
//! inserted segments purged.
//!
//! Delivery is at least once: a crash between an insert and the checkpoint
//! write replays those records, so ClickHouse can hold them twice.
//!
//! A failed archive request is returned from [`Ingester::tick`]: the archive,
//! or this client's session with it, is gone, and only a new connection
//! recovers. The checkpoint makes exiting and starting again safe.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rusteron_archive::{
    Aeron, AeronArchive, AeronArchiveAsyncConnect, AeronArchiveContext, AeronArchiveErrorCode,
    AeronArchiveReplayParams, AeronContext, AeronSubscription, Handlers, IntoCString,
    SOURCE_LOCATION_LOCAL,
};

use crate::{Error, Report, Settings, Writer};

/// The archive's local control channel: same host, no ports.
const CONTROL: &std::ffi::CStr = c"aeron:ipc?term-length=64k";

/// A position in one recording.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Position {
    recording: i64,
    position: i64,
}

struct Replay {
    recording: i64,
    session: i64,
    subscription: AeronSubscription,
    opened: Instant,
    connected: bool,
    /// The recording's first position still on disk; advances as it is purged.
    start: i64,
    term_length: i32,
    segment_length: i32,
}

/// Moves recorded messages into ClickHouse. Call [`Ingester::tick`] about
/// once a second.
pub struct Ingester {
    writer: Writer,
    // Field order is drop order: the archive client before the Aeron client.
    replay: Option<Replay>,
    archive: AeronArchive,
    aeron: Aeron,
    _ctx: AeronContext,
    channel: String,
    stream_id: i32,
    checkpoint_path: PathBuf,
    /// Everything up to here is in ClickHouse.
    checkpoint: Option<Position>,
    /// Everything up to here has been handed to the writer.
    polled: Option<Position>,
    max_queued: usize,
}

fn aeron(e: impl std::fmt::Display) -> Error {
    Error::Aeron(e.to_string())
}

impl Ingester {
    /// Load the schemas and `tables.yaml`, connect to the archive, and make
    /// sure it records the stream.
    pub fn connect(schemas: &[&str], settings: Settings) -> Result<Self, Error> {
        let mut writer = Writer::new(
            schemas,
            settings.clickhouse,
            &settings.config_path,
            settings.recheck,
        )?;
        writer.keep_shapes(settings.checkpoint_path.with_extension("shapes"))?;
        let ctx = AeronContext::new().map_err(aeron)?;
        if let Some(dir) = &settings.aeron_dir {
            ctx.set_dir(&dir.as_str().into_c_string()).map_err(aeron)?;
        }
        let client = Aeron::new(&ctx).map_err(aeron)?;
        client.start().map_err(aeron)?;
        let archive_ctx = AeronArchiveContext::new().map_err(aeron)?;
        archive_ctx.set_aeron(&client).map_err(aeron)?;
        archive_ctx
            .set_control_request_channel(CONTROL)
            .map_err(aeron)?;
        archive_ctx
            .set_control_response_channel(CONTROL)
            .map_err(aeron)?;
        let archive = AeronArchiveAsyncConnect::new_with_aeron(&archive_ctx, &client)
            .map_err(aeron)?
            .poll_blocking(Duration::from_secs(10))
            .map_err(|e| Error::Aeron(format!("connecting to the archive: {e}")))?;
        // The recording outlives this process: after a restart it is kept.
        let channel = settings.channel.as_str().into_c_string();
        match archive.start_recording(&channel, settings.stream_id, SOURCE_LOCATION_LOCAL, false) {
            Ok(_) => log::info!(
                "archive: recording {} stream {}",
                settings.channel,
                settings.stream_id
            ),
            Err(e) if e.code == AeronArchiveErrorCode::ActiveSubscription => {}
            Err(e) => return Err(aeron(e)),
        }
        let checkpoint = load(&settings.checkpoint_path)?;
        Ok(Self {
            writer,
            replay: None,
            archive,
            aeron: client,
            _ctx: ctx,
            channel: settings.channel,
            stream_id: settings.stream_id,
            checkpoint_path: settings.checkpoint_path,
            checkpoint,
            polled: checkpoint,
            max_queued: settings.max_queued_bytes,
        })
    }

    /// Replay what was recorded since the last tick, insert it, then save the
    /// checkpoint and purge the archive behind it. An error means the archive
    /// can no longer be used: drop this ingester and connect again.
    pub fn tick(&mut self) -> Result<Report, Error> {
        let mut report = Report::default();
        if self.replay.is_none() {
            self.open_replay(&mut report)?;
        }
        self.poll()?;
        self.writer.run(&mut report);
        if self.writer.queued_bytes() == 0 {
            self.commit(&mut report)?;
        }
        self.writer.log(&report);
        Ok(report)
    }

    /// Replay the oldest recording that still holds records not in
    /// ClickHouse, deleting the ones that hold none.
    fn open_replay(&mut self, report: &mut Report) -> Result<(), Error> {
        struct Descriptor {
            id: i64,
            start: i64,
            stop: i64,
            term_length: i32,
            segment_length: i32,
        }
        let mut recordings = Vec::new();
        let mut count = 0;
        let fragment = self.channel.split('?').next().unwrap_or_default();
        self.archive
            .list_recordings_for_uri_fn(
                &mut count,
                self.checkpoint.map_or(0, |c| c.recording),
                1000,
                &fragment.into_c_string(),
                self.stream_id,
                |d| {
                    recordings.push(Descriptor {
                        id: d.recording_id(),
                        start: d.start_position(),
                        stop: d.stop_position(),
                        term_length: d.term_buffer_length(),
                        segment_length: d.segment_file_length(),
                    });
                },
            )
            .map_err(aeron)?;
        for d in recordings {
            let from = match self.checkpoint {
                Some(c) if c.recording == d.id => c.position.max(d.start),
                _ => d.start,
            };
            // A stopped recording (its application exited) that is all in
            // ClickHouse is no longer needed.
            if d.stop >= 0 && from >= d.stop {
                self.archive.purge_recording(d.id).map_err(aeron)?;
                report.purged.push(format!(
                    "recording {}: all of it is in ClickHouse, deleted it",
                    d.id
                ));
                continue;
            }
            // Length -1: to the end, and on live if it is still recording.
            let params = AeronArchiveReplayParams::new(-1, -1, from, -1, -1, -1).map_err(aeron)?;
            let replay_stream = self.stream_id + 1;
            let session = self
                .archive
                .start_replay(d.id, c"aeron:ipc", replay_stream, &params)
                .map_err(aeron)?;
            let subscription = self
                .aeron
                .async_add_subscription(
                    &format!("aeron:ipc?session-id={}", session as i32).into_c_string(),
                    replay_stream,
                    Handlers::NONE,
                    Handlers::NONE,
                )
                .map_err(aeron)?
                .poll_blocking(Duration::from_secs(10))
                .map_err(aeron)?;
            log::info!("replaying recording {} from position {from}", d.id);
            self.replay = Some(Replay {
                recording: d.id,
                session,
                subscription,
                opened: Instant::now(),
                connected: false,
                start: d.start,
                term_length: d.term_length,
                segment_length: d.segment_length,
            });
            return Ok(());
        }
        Ok(())
    }

    /// Hand replayed messages to the writer until it holds `max_queued`
    /// bytes; the rest waits in the archive.
    fn poll(&mut self) -> Result<(), Error> {
        let Some(replay) = &mut self.replay else {
            return Ok(());
        };
        let writer = &mut self.writer;
        let mut last = None;
        while writer.queued_bytes() < self.max_queued {
            let polled = replay.subscription.poll_fn(
                |message, header| {
                    writer.push(message);
                    last = Some(header.position());
                },
                1024,
            );
            if polled.map_err(|e| Error::Aeron(format!("archive replay: {e}")))? == 0 {
                break;
            }
        }
        if let Some(position) = last {
            self.polled = Some(Position {
                recording: replay.recording,
                position,
            });
        }
        if replay.subscription.is_connected() {
            replay.connected = true;
        } else if replay.connected || replay.opened.elapsed() > Duration::from_secs(10) {
            // The replay reached the end of a stopped recording, or never
            // started: open the next one.
            if let Some(replay) = self.replay.take() {
                let _ = self.archive.stop_replay(replay.session);
            }
        }
        Ok(())
    }

    /// Everything polled is in ClickHouse: save the checkpoint and purge the
    /// archive segments before it.
    fn commit(&mut self, report: &mut Report) -> Result<(), Error> {
        let Some(polled) = self.polled.filter(|p| Some(*p) != self.checkpoint) else {
            return Ok(());
        };
        if let Err(e) = save(&self.checkpoint_path, polled) {
            report.errors.push(e.to_string());
            return Ok(());
        }
        self.checkpoint = Some(polled);
        let Some(replay) = self
            .replay
            .as_mut()
            .filter(|r| r.recording == polled.recording)
        else {
            return Ok(());
        };
        let base = AeronArchive::segment_file_base_position(
            replay.start,
            polled.position,
            replay.term_length,
            replay.segment_length,
        );
        if base <= replay.start {
            return Ok(());
        }
        self.archive
            .purge_segments(replay.recording, base)
            .map_err(|e| {
                Error::Aeron(format!(
                    "archive: purging recording {}: {e}",
                    replay.recording
                ))
            })?;
        report.purged.push(format!(
            "recording {}: deleted the segments before position {base}",
            replay.recording
        ));
        replay.start = base;
        Ok(())
    }
}

impl Drop for Ingester {
    /// Stop the replay now rather than when the archive notices: a replay
    /// left running blocks purging the segments it has not reached.
    fn drop(&mut self) {
        if let Some(replay) = self.replay.take() {
            let _ = self.archive.stop_replay(replay.session);
        }
    }
}

/// `recording position`, or `None` when nothing was ever inserted.
fn load(path: &Path) -> Result<Option<Position>, Error> {
    let fail = |e: &dyn std::fmt::Display| Error::Checkpoint(format!("{}: {e}", path.display()));
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(fail(&e)),
    };
    let mut numbers = text.split_whitespace().map(str::parse::<i64>);
    match (numbers.next(), numbers.next()) {
        (Some(Ok(recording)), Some(Ok(position))) => Ok(Some(Position {
            recording,
            position,
        })),
        _ => Err(fail(&"expected `recording position`")),
    }
}

/// Write the checkpoint beside itself, then rename, so a crash never leaves
/// half a file.
fn save(path: &Path, at: Position) -> Result<(), Error> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, format!("{} {}\n", at.recording, at.position))
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| Error::Checkpoint(format!("{}: {e}", path.display())))
}
