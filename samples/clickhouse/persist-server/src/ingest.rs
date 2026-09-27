//! Aeron Archive -> ClickHouse: replay every recording on this node from its
//! checkpoint, insert, save the checkpoints, purge what is behind them.
//!
//! The node's archive records two kinds of stream:
//!
//! * the applications' IPC persist stream (tables, events, metrics, traces);
//! * every feed in `config/streams.yaml` that is published on this node,
//!   through a spy on its UDP publication: the archive reads the
//!   publication's buffers in shared memory, and the publisher does no
//!   extra work. A feed that moves to another node is recorded there.
//!
//! Every run of an application is its own recording (a new Aeron session),
//! and several are live at once (one per feed on the node, and the persist
//! stream), so all of them are replayed together, each with its own
//! checkpoint. A stopped recording is deleted once all of it is in
//! ClickHouse; a live one has its inserted segments purged.
//!
//! Delivery is at least once: a crash between an insert and the checkpoint
//! write replays those records, so ClickHouse can hold them twice.
//!
//! A failed archive request is returned from [`Ingester::tick`]: the archive,
//! or this client's session with it, is gone, and only a new connection
//! recovers. The checkpoints make exiting and starting again safe.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rusteron_archive::{
    Aeron, AeronArchive, AeronArchiveAsyncConnect, AeronArchiveContext, AeronArchiveErrorCode,
    AeronArchiveReplayParams, AeronContext, AeronSubscription, Handlers, IntoCString,
    SOURCE_LOCATION_LOCAL,
};

use crate::aeron_stats::AeronStats;
use crate::{Error, Report, Settings, Writer};

/// The archive's local control channel: same host, no ports.
const CONTROL: &std::ffi::CStr = c"aeron:ipc?term-length=64k";

struct Replay {
    session: i64,
    subscription: AeronSubscription,
    opened: Instant,
    connected: bool,
    /// A feed's recording: `tables.yaml` is applied when it is inserted.
    feed: bool,
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
    /// By recording id.
    replays: BTreeMap<i64, Replay>,
    archive: AeronArchive,
    aeron: Aeron,
    _ctx: AeronContext,
    /// The persist stream; replays use the next id.
    stream_id: i32,
    /// Feed streams recorded here, by stream id.
    feeds: BTreeSet<i32>,
    checkpoint_path: PathBuf,
    /// Everything up to here is in ClickHouse, by recording id.
    checkpoints: BTreeMap<i64, i64>,
    /// Everything up to here has been handed to the writer, by recording id.
    polled: BTreeMap<i64, i64>,
    max_queued: usize,
    /// The driver's own statistics, and when they are next sampled.
    stats: Option<(AeronStats, Duration, Instant)>,
}

fn aeron(e: impl std::fmt::Display) -> Error {
    Error::Aeron(e.to_string())
}

impl Ingester {
    /// Load the schemas and `tables.yaml`, connect to the archive, and make
    /// sure it records the persist stream and every feed published here.
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
        ctx.set_client_name(c"ingester").map_err(aeron)?;
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
        // Recordings outlive this process: after a restart they are kept.
        let record = |channel: &str, stream_id: i32| -> Result<(), Error> {
            match archive.start_recording(
                &channel.into_c_string(),
                stream_id,
                SOURCE_LOCATION_LOCAL,
                false,
            ) {
                Ok(_) => {
                    log::info!("archive: recording {channel} stream {stream_id}");
                    Ok(())
                }
                Err(e) if e.code == AeronArchiveErrorCode::ActiveSubscription => Ok(()),
                Err(e) => Err(Error::Aeron(format!(
                    "recording {channel} {stream_id}: {e}"
                ))),
            }
        };
        record(&settings.channel, settings.stream_id)?;
        let mut feeds = BTreeSet::new();
        if let Some(streams) = &settings.streams {
            for (service, _, stream_id) in streams.archived() {
                record(&streams.spy(service, &settings.host_ip)?, stream_id)?;
                feeds.insert(stream_id);
            }
        }
        let checkpoints = load(&settings.checkpoint_path)?;
        let stats = if settings.aeron_stats_interval.is_zero() {
            None
        } else {
            Some((
                AeronStats::open(ctx.get_dir())?,
                settings.aeron_stats_interval,
                Instant::now(),
            ))
        };
        Ok(Self {
            writer,
            replays: BTreeMap::new(),
            archive,
            aeron: client,
            _ctx: ctx,
            stream_id: settings.stream_id,
            feeds,
            checkpoint_path: settings.checkpoint_path,
            polled: checkpoints.clone(),
            checkpoints,
            max_queued: settings.max_queued_bytes,
            stats,
        })
    }

    /// Replay what was recorded since the last tick, insert it, then save the
    /// checkpoints and purge the archive behind them. An error means the
    /// archive can no longer be used: drop this ingester and connect again.
    pub fn tick(&mut self) -> Result<Report, Error> {
        let mut report = Report::default();
        self.open_replays(&mut report)?;
        self.poll()?;
        self.writer.run(&mut report);
        if self.writer.queued_bytes() == 0 {
            self.commit(&mut report)?;
        }
        if let Some((stats, every, next)) = &mut self.stats
            && Instant::now() >= *next
        {
            *next = Instant::now() + *every;
            let now = persist_client::clock::Clock::new().now().epoch_ns();
            let clients = self.writer.client_names();
            report
                .errors
                .extend(stats.sample(self.writer.clickhouse(), now, &clients));
        }
        self.writer.log(&report);
        Ok(report)
    }

    /// Replay every recording of a tracked stream that still holds records
    /// not in ClickHouse, deleting the ones that hold none.
    fn open_replays(&mut self, report: &mut Report) -> Result<(), Error> {
        struct Descriptor {
            id: i64,
            stream_id: i32,
            start: i64,
            stop: i64,
            term_length: i32,
            segment_length: i32,
        }
        let mut recordings = Vec::new();
        self.archive
            .list_recordings_fn(&mut 0, 0, i32::MAX, |d| {
                recordings.push(Descriptor {
                    id: d.recording_id(),
                    stream_id: d.stream_id(),
                    start: d.start_position(),
                    stop: d.stop_position(),
                    term_length: d.term_buffer_length(),
                    segment_length: d.segment_file_length(),
                });
            })
            .map_err(aeron)?;
        for d in recordings {
            let feed = self.feeds.contains(&d.stream_id);
            if (d.stream_id != self.stream_id && !feed) || self.replays.contains_key(&d.id) {
                continue;
            }
            let from = self
                .checkpoints
                .get(&d.id)
                .map_or(d.start, |&c| c.max(d.start));
            // A stopped recording (its application exited) that is all in
            // ClickHouse is no longer needed.
            if d.stop >= 0 && from >= d.stop {
                self.archive.purge_recording(d.id).map_err(aeron)?;
                self.checkpoints.remove(&d.id);
                self.polled.remove(&d.id);
                if let Err(e) = save(&self.checkpoint_path, &self.checkpoints) {
                    report.errors.push(e.to_string());
                }
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
            log::info!(
                "replaying recording {} (stream {}) from position {from}",
                d.id,
                d.stream_id
            );
            self.replays.insert(
                d.id,
                Replay {
                    session,
                    subscription,
                    opened: Instant::now(),
                    connected: false,
                    feed,
                    start: d.start,
                    term_length: d.term_length,
                    segment_length: d.segment_length,
                },
            );
        }
        Ok(())
    }

    /// Hand replayed messages to the writer, a batch from each recording in
    /// turn, until it holds `max_queued` bytes; the rest waits in the archive.
    fn poll(&mut self) -> Result<(), Error> {
        let writer = &mut self.writer;
        let mut finished = Vec::new();
        loop {
            let mut any = false;
            for (&recording, replay) in &mut self.replays {
                if writer.queued_bytes() >= self.max_queued {
                    break;
                }
                let feed = replay.feed;
                let mut last = None;
                let polled = replay
                    .subscription
                    .poll_fn(
                        |message, header| {
                            // The frame's reserved value is the recording app's source id.
                            writer.push_from(
                                message,
                                header.reserved_value().unwrap_or(0) as u64,
                                feed,
                            );
                            last = Some(header.position());
                        },
                        1024,
                    )
                    .map_err(|e| Error::Aeron(format!("archive replay: {e}")))?;
                if let Some(position) = last {
                    self.polled.insert(recording, position);
                }
                any |= polled > 0;
            }
            if !any || writer.queued_bytes() >= self.max_queued {
                break;
            }
        }
        for (&recording, replay) in &mut self.replays {
            if replay.subscription.is_connected() {
                replay.connected = true;
            } else if replay.connected || replay.opened.elapsed() > Duration::from_secs(10) {
                // The replay reached the end of a stopped recording, or never
                // started: open it again next tick, from its checkpoint.
                finished.push(recording);
            }
        }
        for recording in finished {
            if let Some(replay) = self.replays.remove(&recording) {
                let _ = self.archive.stop_replay(replay.session);
            }
        }
        Ok(())
    }

    /// Everything polled is in ClickHouse: save the checkpoints and purge
    /// each recording's segments before its own.
    fn commit(&mut self, report: &mut Report) -> Result<(), Error> {
        if self.polled == self.checkpoints {
            return Ok(());
        }
        if let Err(e) = save(&self.checkpoint_path, &self.polled) {
            report.errors.push(e.to_string());
            return Ok(());
        }
        self.checkpoints.clone_from(&self.polled);
        for (&recording, replay) in &mut self.replays {
            let Some(&position) = self.checkpoints.get(&recording) else {
                continue;
            };
            let base = AeronArchive::segment_file_base_position(
                replay.start,
                position,
                replay.term_length,
                replay.segment_length,
            );
            if base <= replay.start {
                continue;
            }
            self.archive.purge_segments(recording, base).map_err(|e| {
                Error::Aeron(format!("archive: purging recording {recording}: {e}"))
            })?;
            report.purged.push(format!(
                "recording {recording}: deleted the segments before position {base}"
            ));
            replay.start = base;
        }
        Ok(())
    }
}

impl Drop for Ingester {
    /// Stop the replays now rather than when the archive notices: a replay
    /// left running blocks purging the segments it has not reached.
    fn drop(&mut self) {
        for replay in std::mem::take(&mut self.replays).into_values() {
            let _ = self.archive.stop_replay(replay.session);
        }
    }
}

/// `recording position` per line; none when nothing was ever inserted.
fn load(path: &Path) -> Result<BTreeMap<i64, i64>, Error> {
    let fail = |e: &dyn std::fmt::Display| Error::Checkpoint(format!("{}: {e}", path.display()));
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(e) => return Err(fail(&e)),
    };
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| {
            let mut numbers = line.split_whitespace().map(str::parse::<i64>);
            match (numbers.next(), numbers.next()) {
                (Some(Ok(recording)), Some(Ok(position))) => Ok((recording, position)),
                _ => Err(fail(&"expected `recording position` lines")),
            }
        })
        .collect()
}

/// Write the checkpoints beside the file, then rename, so a crash never
/// leaves half a file.
fn save(path: &Path, checkpoints: &BTreeMap<i64, i64>) -> Result<(), Error> {
    let tmp = path.with_extension("tmp");
    let text: String = checkpoints
        .iter()
        .map(|(recording, position)| format!("{recording} {position}\n"))
        .collect();
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| Error::Checkpoint(format!("{}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoints_round_trip_one_line_per_recording() -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!("ingest-checkpoint-{}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("checkpoint");
        assert!(load(&path)?.is_empty(), "no file: nothing inserted yet");
        let checkpoints = BTreeMap::from([(3, 4096), (7, 128)]);
        save(&path, &checkpoints)?;
        assert_eq!(std::fs::read_to_string(&path)?, "3 4096\n7 128\n");
        assert_eq!(load(&path)?, checkpoints);
        std::fs::write(&path, "3 x\n")?;
        assert!(load(&path).is_err());
        std::fs::remove_file(&path)?;
        std::fs::remove_dir(&dir)?;
        Ok(())
    }
}
