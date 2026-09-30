//! Replay this node's recordings into ClickHouse, then purge behind the checkpoint.
//!
//! The archive records the IPC persist stream and, through a spy, every feed
//! published on this node. Each Aeron session is its own recording and has
//! its own checkpoint. A stopped recording is deleted once it is fully
//! ingested. A live one has its inserted segments deleted.
//!
//! A crash before the checkpoint is saved replays that batch. The insert token
//! is the batch's recording positions, and ClickHouse drops the repeat. Each
//! table remembers the last 1000 inserts.
//!
//! An archive error from [`Ingester::tick`] means this connection is dead.
//! A new process resumes from the checkpoints. A crash drops at most the
//! open 5 s histogram window.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

use rusteron_archive::{
    Aeron, AeronArchive, AeronArchiveAsyncConnect, AeronArchiveContext, AeronArchiveErrorCode,
    AeronArchiveReplayParams, AeronContext, AeronImage, AeronSubscription,
    AeronUnavailableImageCallback, Handler, Handlers, IntoCString, SOURCE_LOCATION_LOCAL,
};

use crate::aeron_stats::AeronStats;
use crate::{Error, Report, Settings, Writer};

/// The archive's local control channel: same host, no ports.
const CONTROL: &std::ffi::CStr = c"aeron:ipc?term-length=64k";

struct Replay {
    session: i64,
    /// Consumed position, including padding and a removed image's final position.
    position: Arc<AtomicI64>,
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

struct ReplayPosition(Arc<AtomicI64>);

impl AeronUnavailableImageCallback for ReplayPosition {
    fn handle_aeron_on_unavailable_image(&mut self, _: AeronSubscription, image: AeronImage) {
        self.0.fetch_max(image.position(), Ordering::Relaxed);
    }
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
    /// This node's IP: its feeds' spies bind it.
    host_ip: String,
    /// New versions of `streams.yaml`, once [`Ingester::follow`] is called.
    watch: Option<persist_client::streams::Watch>,
    checkpoint_path: PathBuf,
    /// Positions of an insert that may already be in ClickHouse. Written
    /// before the insert and removed when the checkpoint passes it, so a
    /// restart replays this batch and no further, with the same token.
    pending_path: PathBuf,
    pending: Option<BTreeMap<i64, i64>>,
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
    /// Load each schema's XML and `tables.yaml`, connect to the archive, and
    /// make sure it records the persist stream and every feed published here.
    /// A frame is matched later by the schema id and template id in its
    /// header. This crate does not decode with generated codecs.
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
        record(&archive, &settings.channel, settings.stream_id)?;
        let mut feeds = BTreeSet::new();
        if let Some(streams) = &settings.streams {
            record_feeds(&archive, streams, &settings.host_ip, &mut feeds)?;
        }
        let checkpoints = load(&settings.checkpoint_path)?;
        let pending_path = settings.checkpoint_path.with_extension("pending");
        let pending = load(&pending_path)?;
        let pending = in_flight(&checkpoints, pending);
        if pending.is_none() {
            // Already checkpointed, or nothing was ever in flight.
            if pending_path.exists() {
                std::fs::remove_file(&pending_path)
                    .map_err(|e| Error::Checkpoint(format!("{}: {e}", pending_path.display())))?;
            }
        }
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
            host_ip: settings.host_ip,
            watch: None,
            checkpoint_path: settings.checkpoint_path,
            pending_path,
            pending,
            polled: checkpoints.clone(),
            checkpoints,
            max_queued: settings.max_queued_bytes,
            stats,
        })
    }

    /// Follow `streams.yaml` at `path` from now on: a service added to it has
    /// its feeds recorded here from the next tick, with no restart.
    pub fn follow(&mut self, path: impl Into<PathBuf>) -> Result<(), Error> {
        self.watch = Some(persist_client::streams::Watch::spawn(path)?);
        Ok(())
    }

    /// Replay what was recorded since the last tick, insert it, then save the
    /// checkpoints and purge the archive behind them. An error means the
    /// archive can no longer be used: drop this ingester and connect again.
    pub fn tick(&mut self) -> Result<Report, Error> {
        let mut report = Report::default();
        if let Some(streams) = self.watch.as_ref().and_then(|w| w.changed()) {
            record_feeds(&self.archive, &streams, &self.host_ip, &mut self.feeds)?;
        }
        self.open_replays(&mut report)?;
        let mut caught_up = false;
        if self.pending.is_none() {
            caught_up = self.poll(true)?;
            if self.polled != self.checkpoints {
                // The token is this file. An insert before it is durable can
                // be replayed as a different batch and land twice.
                if let Err(e) = save(&self.pending_path, &self.polled) {
                    report.errors.push(e.to_string());
                    self.writer.log(&report);
                    return Ok(report);
                }
                self.pending = Some(self.polled.clone());
            }
        } else if self.writer.queued_bytes() == 0 && !self.pending_reached() {
            // The restart replay is bounded by `pending`, so this reads the
            // uncommitted batch and stops. Insert once it is all in hand:
            // a prefix would consume the token and ClickHouse would drop the rest.
            self.poll(false)?;
            if !self.pending_reached() {
                self.writer.log(&report);
                return Ok(report);
            }
            // Reaching this retry's bounded endpoint does not prove we have
            // read the rest of a historical histogram window in the archive.
        }
        self.writer
            .flush_elapsed_histograms(Self::unix_now_ns(), caught_up);
        if let Some(pending) = &self.pending {
            self.writer.set_dedup_token(&dedup_token(pending));
        } else {
            self.writer.set_dedup_token("");
        }
        self.writer.run(&mut report);
        if self.writer.queued_bytes() == 0 && self.pending_reached() {
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
            // While an insert is uncommitted, replay that batch and stop.
            // A live replay would pull in messages that were not part of it.
            let length = if let Some(pending) = &self.pending {
                match pending.get(&d.id) {
                    Some(&end) if end > from => end - from,
                    _ => continue,
                }
            } else {
                -1
            };
            // A stopped recording (its application exited) that is all in
            // ClickHouse is no longer needed.
            if length < 0 && d.stop >= 0 && from >= d.stop {
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
            // Length -1 replays to the end and follows a live recording.
            // A positive length replays the uncommitted batch and stops.
            let params =
                AeronArchiveReplayParams::new(-1, -1, from, length, -1, -1).map_err(aeron)?;
            let replay_stream = self.stream_id + 1;
            let session =
                match self
                    .archive
                    .start_replay(d.id, c"aeron:ipc", replay_stream, &params)
                {
                    Ok(session) => session,
                    // Listed as recording, it stopped before the request with
                    // nothing past `from` (a publisher restarted before it sent
                    // anything): the next listing shows it stopped, and purges it.
                    Err(e) if e.code == AeronArchiveErrorCode::InvalidPosition => {
                        log::info!("recording {}: not replayable yet ({e}); next tick", d.id);
                        continue;
                    }
                    Err(e) => return Err(aeron(e)),
                };
            let position = Arc::new(AtomicI64::new(from));
            let unavailable = Handler::new(ReplayPosition(Arc::clone(&position)));
            let subscription = self
                .aeron
                .async_add_subscription(
                    &format!("aeron:ipc?session-id={}", session as i32).into_c_string(),
                    replay_stream,
                    Handlers::NONE,
                    Some(&unavailable),
                )
                .map_err(aeron)?
                .poll_blocking(Duration::from_secs(10))
                .map_err(aeron)?;
            log::info!(
                "replaying recording {} (stream {}) from position {from}",
                d.id,
                d.stream_id
            );
            // An empty recording is already consumed through its replay start,
            // even though it will never invoke a message callback.
            self.polled.entry(d.id).or_insert(from);
            self.replays.insert(
                d.id,
                Replay {
                    session,
                    position,
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
    /// `true` after reading through each recording's current archive position.
    /// An empty transport poll alone does not prove a replay has caught up.
    fn poll(&mut self, capped: bool) -> Result<bool, Error> {
        let recordings: Vec<_> = self.replays.keys().copied().collect();
        let max_queued = self.max_queued;
        let writer = &mut self.writer;
        let mut finished = Vec::new();
        let mut caught_up = true;
        loop {
            let mut any = false;
            for (&recording, replay) in &mut self.replays {
                if capped && writer.queued_bytes() >= max_queued {
                    caught_up = false;
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
                // Padding advances the subscriber position without invoking
                // the fragment handler. Include it in catch-up/checkpoints.
                replay.subscription.for_each_image(|image| {
                    replay
                        .position
                        .fetch_max(image.position(), Ordering::Relaxed);
                });
                let position = replay.position.load(Ordering::Relaxed);
                self.polled
                    .entry(recording)
                    .and_modify(|last| *last = (*last).max(position));
                any |= polled > 0;
            }
            if !any || (capped && writer.queued_bytes() >= max_queued) {
                if capped && writer.queued_bytes() >= max_queued {
                    caught_up = false;
                }
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
        if caught_up {
            let mut ends = BTreeMap::new();
            for recording in recordings {
                let end = self
                    .archive
                    .get_max_recorded_position(recording)
                    .map_err(aeron)?;
                ends.insert(recording, end);
            }
            caught_up = replay_caught_up(&self.polled, &ends);
        }
        Ok(caught_up)
    }

    fn unix_now_ns() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| {
                u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX)
            })
    }

    fn clear_pending(&mut self, report: &mut Report) {
        if self.pending.take().is_none() {
            return;
        }
        if let Err(e) = std::fs::remove_file(&self.pending_path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            report
                .errors
                .push(format!("{}: {e}", self.pending_path.display()));
        }
    }

    /// The uncommitted batch, if there is one, has been read back.
    fn pending_reached(&self) -> bool {
        let Some(pending) = &self.pending else {
            return true;
        };
        pending.iter().all(|(id, end)| {
            self.checkpoints.get(id).is_some_and(|c| c >= end)
                || self.polled.get(id).is_some_and(|p| p >= end)
        })
    }

    /// Everything polled is in ClickHouse: save the checkpoints and purge
    /// each recording's segments before its own.
    fn commit(&mut self, report: &mut Report) -> Result<(), Error> {
        let target = self.pending.clone().unwrap_or_else(|| self.polled.clone());
        if target == self.checkpoints {
            self.clear_pending(report);
            return Ok(());
        }
        if let Err(e) = save(&self.checkpoint_path, &target) {
            report.errors.push(e.to_string());
            return Ok(());
        }
        self.checkpoints.clone_from(&target);
        self.clear_pending(report);
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

/// A replay is caught up only after it has delivered all archived messages.
fn replay_caught_up(polled: &BTreeMap<i64, i64>, ends: &BTreeMap<i64, i64>) -> bool {
    ends.iter()
        .all(|(id, end)| polled.get(id).is_some_and(|position| position >= end))
}

/// `pending` when one of its positions is still ahead of `checkpoints`.
fn in_flight(
    checkpoints: &BTreeMap<i64, i64>,
    pending: BTreeMap<i64, i64>,
) -> Option<BTreeMap<i64, i64>> {
    if pending.is_empty()
        || pending
            .iter()
            .all(|(id, pos)| checkpoints.get(id).is_some_and(|c| c >= pos))
    {
        None
    } else {
        Some(pending)
    }
}

/// The insert's identity: one token per batch, stable across the retry.
/// ClickHouse drops a later insert that repeats it, whichever rows it holds.
fn dedup_token(positions: &BTreeMap<i64, i64>) -> String {
    positions
        .iter()
        .map(|(id, pos)| format!("{id}:{pos}"))
        .collect::<Vec<_>>()
        .join(",")
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

/// Have the archive record `channel` and `stream_id`. Recordings outlive
/// this process: one already running is left as it is.
fn record(archive: &AeronArchive, channel: &str, stream_id: i32) -> Result<(), Error> {
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
}

/// Record every archived feed of `streams` published on this node, through
/// a spy on its publication, and track its stream id.
fn record_feeds(
    archive: &AeronArchive,
    streams: &persist_client::streams::Streams,
    host_ip: &str,
    feeds: &mut BTreeSet<i32>,
) -> Result<(), Error> {
    for (service, _, stream_id) in streams.archived() {
        record(archive, &streams.spy(service, host_ip)?, stream_id)?;
        feeds.insert(stream_id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_partial_replay_batch_does_not_close_historical_histograms() {
        let archive_ends = BTreeMap::from([(3, 8192), (7, 128)]);
        let pending_end = BTreeMap::from([(3, 4096), (7, 128)]);
        assert!(!replay_caught_up(&pending_end, &archive_ends));
        assert!(!replay_caught_up(
            &BTreeMap::from([(3, 8192)]),
            &archive_ends
        ));
        assert!(replay_caught_up(&archive_ends, &archive_ends));
        assert!(replay_caught_up(
            &BTreeMap::from([(3, 9000), (7, 128)]),
            &archive_ends
        ));
    }

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

    #[test]
    fn a_checkpointed_batch_is_not_still_in_flight() {
        let checkpoints = BTreeMap::from([(3, 4096), (7, 128)]);
        assert!(in_flight(&checkpoints, BTreeMap::new()).is_none());
        assert!(in_flight(&checkpoints, checkpoints.clone()).is_none());
        assert!(in_flight(&checkpoints, BTreeMap::from([(3, 1000)])).is_none());
        let ahead = BTreeMap::from([(3, 4096), (7, 256)]);
        assert_eq!(
            in_flight(&checkpoints, ahead.clone()).as_ref(),
            Some(&ahead)
        );
    }

    #[test]
    fn the_dedup_token_is_the_positions_in_recording_order() {
        let positions = BTreeMap::from([(7, 128), (3, 4096)]);
        assert_eq!(dedup_token(&positions), "3:4096,7:128");
    }
}
