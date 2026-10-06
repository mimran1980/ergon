//! Replay this node's recordings into `ClickHouse`, then purge behind the checkpoint.
//!
//! The archive records the IPC persist stream and, through a spy, each feed
//! in [`Settings::feeds`](crate::Settings::feeds) ([`Ingester::record_feeds`]
//! adds more). Each Aeron session is its own recording and has its own
//! checkpoint. A stopped recording is deleted once it is fully ingested. A
//! live one has its inserted segments deleted.
//!
//! A crash before the checkpoint is saved replays that batch. The insert token
//! is the batch's recording positions, and `ClickHouse` drops the repeat. Each
//! table remembers the last 1000 inserts.
//!
//! An archive error from [`Ingester::tick`] means this connection is dead.
//! A new process resumes from the checkpoints. A crash drops at most the
//! open 5 s histogram window.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::{Duration, Instant};

use rusteron_archive::{
    Aeron, AeronArchive, AeronArchiveAsyncConnect, AeronArchiveContext, AeronArchiveErrorCode,
    AeronArchiveReplayParams, AeronAsyncAddSubscription, AeronContext, AeronImage,
    AeronSubscription, AeronUnavailableImageCallback, Handler, Handlers, IntoCString,
    SOURCE_LOCATION_LOCAL,
};

use crate::aeron_stats::AeronStats;
use ergon_runtime::clock::Nanos;
use ergon_runtime::frames::FrameRow;

use crate::{Error, RecordedFeed, Report, Settings, Writer};

/// The archive's local control channel: same host, no ports.
const CONTROL: &std::ffi::CStr = c"aeron:ipc?term-length=64k";

struct Replay {
    session: i64,
    /// Consumed position, including padding and a removed image's final position.
    position: Arc<AtomicI64>,
    subscription: Subscription,
    /// Called by the client while the subscription is open.
    _unavailable: Handler<ReplayPosition>,
    opened: Instant,
    connected: bool,
    /// A feed's recording: `tables.yaml` is applied when it is inserted.
    feed: bool,
    /// A feed's service and kind, and the recording's session and stream,
    /// for its `frame` rows.
    names: Option<(String, String)>,
    session_id: i32,
    stream_id: i32,
    /// A feed's frames carry their publish time in the reserved value, so
    /// its rows take the source the recording's `Source` message names.
    /// One exclusive publication is one session is one recording.
    source: u64,
    /// The recording's first position still on disk; advances as it is purged.
    start: i64,
    term_length: i32,
    segment_length: i32,
}

/// A replay's subscription, added without waiting for the driver.
enum Subscription {
    Adding(AeronAsyncAddSubscription),
    Added(AeronSubscription),
}

impl Subscription {
    /// The subscription once the driver has added it.
    fn added(&mut self) -> Result<Option<&AeronSubscription>, Error> {
        if let Self::Adding(adding) = self {
            match adding.poll().map_err(aeron)? {
                Some(added) => *self = Self::Added(added),
                None => return Ok(None),
            }
        }
        match self {
            Self::Added(subscription) => Ok(Some(subscription)),
            Self::Adding(_) => Ok(None),
        }
    }
}

struct ReplayPosition(Arc<AtomicI64>);

impl AeronUnavailableImageCallback for ReplayPosition {
    fn handle_aeron_on_unavailable_image(&mut self, _: AeronSubscription, image: AeronImage) {
        self.0.fetch_max(image.position(), Ordering::Relaxed);
    }
}

/// Moves recorded messages into `ClickHouse`. Call [`Ingester::tick`] about
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
    /// Feed streams recorded here, by stream id: their service and kind.
    feeds: BTreeMap<i32, (String, String)>,
    /// A `Frame` row being built, reused.
    frame_row: Vec<u8>,
    checkpoint_path: PathBuf,
    /// Positions of an insert that may already be in `ClickHouse`. Written
    /// before the insert and removed when the checkpoint passes it, so a
    /// restart replays this batch and no further, with the same token.
    pending_path: PathBuf,
    pending: Option<BTreeMap<i64, i64>>,
    /// Everything up to here is in `ClickHouse`, by recording id.
    checkpoints: BTreeMap<i64, i64>,
    /// Everything up to here has been handed to the writer, by recording id.
    polled: BTreeMap<i64, i64>,
    /// Each feed recording's source, saved beside the checkpoints so a
    /// resume mid-recording knows it before the next `Source` message.
    sources: BTreeMap<i64, i64>,
    sources_path: PathBuf,
    /// Where each tracked recording starts, from the last listing: a
    /// recording with no checkpoint is read, and its pieces measured, from
    /// there, which stays put until its first commit purges anything.
    starts: BTreeMap<i64, i64>,
    max_queued: usize,
    /// The driver's own statistics, and when they are next sampled.
    stats: Option<(AeronStats, Duration, Instant)>,
}

/// The most of the archive one batch reads, over all its recordings. Its
/// pieces are [`crate::PIECE_SPAN`] of it each, and `ClickHouse` drops a
/// piece it has seen only among the last `DEDUP_WINDOW` a table took, so a
/// batch, overshot by one poll (1024 fragments of at most 64 KiB), must have
/// fewer.
const MAX_BATCH_SPAN: i64 = 64 << 20;
const _: () = assert!(
    ((MAX_BATCH_SPAN + (1024 << 16)) / crate::PIECE_SPAN + 2).cast_unsigned()
        < crate::clickhouse::DEDUP_WINDOW
);

fn aeron(e: impl std::fmt::Display) -> Error {
    Error::Aeron(e.to_string())
}

/// Poll `connect` until Aeron finishes it: connected, or failed at the
/// context's message timeout, when Aeron closes the publication and
/// subscription it opened. Dropping an unfinished connect closes neither,
/// which `poll_blocking` does when its own timer runs out first.
fn finish(connect: &AeronArchiveAsyncConnect) -> Result<AeronArchive, String> {
    loop {
        if let Some(archive) = connect.poll().map_err(|e| e.to_string())? {
            return Ok(archive);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// One archive recording, copied out of the list callback.
struct Recording {
    id: i64,
    stream_id: i32,
    session_id: i32,
    start: i64,
    stop: i64,
    term_length: i32,
    segment_length: i32,
}

impl Ingester {
    /// Load each schema's XML and `tables.yaml`, connect to the archive, and
    /// make sure it records the persist stream and each feed in
    /// [`Settings::feeds`] ([`Ingester::record_feeds`] adds more).
    /// A frame is matched later by the schema id and template id in its
    /// header. This crate does not decode with generated codecs.
    ///
    /// # Errors
    ///
    /// A schema or `tables.yaml` could not be loaded, or the archive did not
    /// connect or accept a recording request.
    pub fn connect(schemas: &[&str], settings: Settings) -> Result<Self, Error> {
        // Raw feed frames: a `frame` table when `tables.yaml` lists it.
        let schemas: Vec<&str> = std::iter::once(ergon_runtime::frames::SCHEMA)
            .chain(std::iter::once(ergon_runtime::journal::SCHEMA))
            .chain(schemas.iter().copied())
            .collect();
        let mut writer = Writer::new(
            &schemas,
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
        // Until the archive answers, however long it takes to start: nothing
        // on this node is recorded before. Exiting instead left it to
        // Kubernetes' growing restart delay, minutes of every app's records
        // dropped as "not connected".
        let archive = loop {
            let connect =
                AeronArchiveAsyncConnect::new_with_aeron(&archive_ctx, &client).map_err(aeron)?;
            match finish(&connect) {
                Ok(archive) => break archive,
                Err(e) => log::warn!("connecting to the archive: {e}; retrying"),
            }
        };
        record(&archive, &settings.channel, settings.stream_id)?;
        let mut feeds = BTreeMap::new();
        record_feeds(&archive, &settings.feeds, &mut feeds)?;
        let checkpoints = load(&settings.checkpoint_path)?;
        let sources_path = settings.checkpoint_path.with_extension("sources");
        let sources = load(&sources_path)?;
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
            frame_row: Vec::new(),
            checkpoint_path: settings.checkpoint_path,
            pending_path,
            pending,
            polled: checkpoints.clone(),
            checkpoints,
            sources,
            sources_path,
            starts: BTreeMap::new(),
            max_queued: settings.max_queued_bytes,
            stats,
        })
    }

    /// Record `feeds` too, from now on: a feed added to the application's
    /// directory is recorded here from the next tick, with no restart, and
    /// so is one whose publisher moved to another port. A feed already
    /// recorded on the same spy is left as it is.
    ///
    /// # Errors
    ///
    /// The archive refused a recording.
    pub fn record_feeds(&mut self, feeds: &[RecordedFeed]) -> Result<(), Error> {
        record_feeds(&self.archive, feeds, &mut self.feeds)
    }

    /// Replay what was recorded since the last tick, insert it, then save the
    /// checkpoints and purge the archive behind them. An error means the
    /// archive can no longer be used: drop this ingester and connect again.
    ///
    /// # Errors
    ///
    /// The archive rejected a replay, poll, or purge. Drop this ingester and
    /// connect again.
    pub fn tick(&mut self) -> Result<Report, Error> {
        let mut report = Report::default();
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
        } else if !self.pending_reached() {
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
            .flush_elapsed_histograms(crate::unix_now_ns(), caught_up);
        if let Some(pending) = &self.pending {
            // Each recording's part of the batch, from where it starts to its
            // end: neither moves until the batch is committed, so its pieces
            // are the same after a restart.
            let spans = pending
                .iter()
                .map(|(&id, &end)| {
                    let start = batch_origin(&self.checkpoints, &self.starts, id);
                    (id, (start.unwrap_or(end), end))
                })
                .collect();
            self.writer.set_batch(&dedup_token(pending), &spans);
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
            let now = i64::try_from(crate::unix_now_ns()).unwrap_or(i64::MAX);
            let clients = self.writer.client_names();
            report
                .errors
                .extend(stats.sample(|t| self.writer.clickhouse_for(t), now, &clients));
        }
        self.writer.log(&report);
        Ok(report)
    }

    /// Replay every recording of a tracked stream that still holds records
    /// not in `ClickHouse`, deleting the ones that hold none.
    fn open_replays(&mut self, report: &mut Report) -> Result<(), Error> {
        let mut recordings = Vec::new();
        self.archive
            .list_recordings_fn(&mut 0, 0, i32::MAX, |d| {
                recordings.push(Recording {
                    id: d.recording_id(),
                    stream_id: d.stream_id(),
                    session_id: d.session_id(),
                    start: d.start_position(),
                    stop: d.stop_position(),
                    term_length: d.term_buffer_length(),
                    segment_length: d.segment_file_length(),
                });
            })
            .map_err(aeron)?;
        for d in recordings {
            let names = self.feeds.get(&d.stream_id).cloned();
            let feed = names.is_some();
            if d.stream_id != self.stream_id && !feed {
                continue;
            }
            self.starts.insert(d.id, d.start);
            if self.replays.contains_key(&d.id) {
                continue;
            }
            let committed = batch_origin(&self.checkpoints, &self.starts, d.id).unwrap_or(d.start);
            // A replay opened again (it ended, or lost its image) goes on from
            // what the writer already holds: from the checkpoint it would hand
            // the writer those records twice.
            let from = self
                .polled
                .get(&d.id)
                .map_or(committed, |&p| p.max(committed));
            // While an insert is uncommitted, replay that batch and stop.
            // A live replay would pull in messages that were not part of it.
            let length = if let Some(pending) = &self.pending {
                match pending.get(&d.id) {
                    Some(&end) if end > from => end - from,
                    // Nothing of the batch is left to read here: the writer
                    // holds it, or there was none (saved before its replay
                    // delivered anything, the batch ends at its start).
                    // Counted read, or the batch would wait for it for good.
                    Some(_) => {
                        self.polled.insert(d.id, from);
                        continue;
                    }
                    None => continue,
                }
            } else {
                -1
            };
            // A stopped recording (its application exited) that is all in
            // ClickHouse is no longer needed.
            if length < 0 && d.stop >= 0 && committed >= d.stop {
                self.purge(d.id, report)?;
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
            let session_id = ergon_runtime::subscription::replay_image_session(session);
            let subscription = self
                .aeron
                .async_add_subscription(
                    &format!("aeron:ipc?session-id={session_id}").into_c_string(),
                    replay_stream,
                    Handlers::NONE,
                    Some(&unavailable),
                )
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
                    position,
                    subscription: Subscription::Adding(subscription),
                    _unavailable: unavailable,
                    opened: Instant::now(),
                    connected: false,
                    feed,
                    names,
                    session_id: d.session_id,
                    stream_id: d.stream_id,
                    source: self.sources.get(&d.id).map_or(0, |s| s.cast_unsigned()),
                    start: d.start,
                    term_length: d.term_length,
                    segment_length: d.segment_length,
                },
            );
        }
        Ok(())
    }

    /// After a poll: save the feed sources learned, and stop replays that
    /// reached the end of a stopped recording or never started, to open them
    /// again next tick from where they got to.
    fn settle(&mut self, learned: Vec<(i64, u64)>) -> Result<(), Error> {
        let mut finished = Vec::new();
        for (&recording, replay) in &mut self.replays {
            if replay
                .subscription
                .added()?
                .is_some_and(AeronSubscription::is_connected)
            {
                replay.connected = true;
            } else if replay.connected || replay.opened.elapsed() > Duration::from_secs(10) {
                finished.push(recording);
            }
        }
        if !learned.is_empty() {
            for (recording, source) in learned {
                self.sources.insert(recording, source.cast_signed());
            }
            save(&self.sources_path, &self.sources)?;
        }
        for recording in finished {
            if let Some(replay) = self.replays.remove(&recording) {
                let _ = self.archive.stop_replay(replay.session);
            }
        }
        Ok(())
    }

    /// Delete a stopped recording that is all in `ClickHouse`, and forget it.
    fn purge(&mut self, id: i64, report: &mut Report) -> Result<(), Error> {
        self.archive.purge_recording(id).map_err(aeron)?;
        self.checkpoints.remove(&id);
        self.polled.remove(&id);
        self.starts.remove(&id);
        if self.sources.remove(&id).is_some()
            && let Err(e) = save(&self.sources_path, &self.sources)
        {
            report.errors.push(e.to_string());
        }
        if let Err(e) = save(&self.checkpoint_path, &self.checkpoints) {
            report.errors.push(e.to_string());
        }
        report.purged.push(format!(
            "recording {id}: all of it is in ClickHouse, deleted it"
        ));
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
        let frame_row = &mut self.frame_row;
        let mut learned = Vec::new();
        let mut caught_up = true;
        loop {
            let mut any = false;
            for (&recording, replay) in &mut self.replays {
                if capped
                    && (writer.queued_bytes() >= max_queued
                        || batch_span(&self.polled, &self.checkpoints, &self.starts)
                            >= MAX_BATCH_SPAN)
                {
                    caught_up = false;
                    break;
                }
                let replay_meta = Meta {
                    feed: replay.feed,
                    names: replay
                        .names
                        .as_ref()
                        .filter(|(service, kind)| writer.wants_frames(service, kind)),
                    session_id: replay.session_id,
                    stream_id: replay.stream_id,
                };
                let mut source = replay.source;
                let mut last = None;
                let Some(subscription) = replay.subscription.added()? else {
                    caught_up = false;
                    continue;
                };
                let polled = subscription
                    .poll_fn(
                        |message, header| {
                            let reserved = header.reserved_value().unwrap_or(0);
                            let at = (recording, header.position(), reserved);
                            take(writer, frame_row, replay_meta, at, &mut source, message);
                            last = Some(header.position());
                        },
                        1024,
                    )
                    .map_err(|e| Error::Aeron(format!("archive replay: {e}")))?;
                if let Some(position) = last {
                    self.polled.insert(recording, position);
                }
                if source != replay.source {
                    replay.source = source;
                    learned.push((recording, source));
                }
                // Padding advances the subscriber position without invoking
                // the fragment handler. Include it in catch-up/checkpoints.
                subscription.for_each_image(|image| {
                    replay
                        .position
                        .fetch_max(image.position(), Ordering::Relaxed);
                });
                // From its replay start once it is subscribed: an empty
                // recording never invokes a message callback.
                let position = replay.position.load(Ordering::Relaxed);
                self.polled
                    .entry(recording)
                    .and_modify(|last| *last = (*last).max(position))
                    .or_insert(position);
                any |= polled > 0;
            }
            if !any || (capped && writer.queued_bytes() >= max_queued) {
                if capped && writer.queued_bytes() >= max_queued {
                    caught_up = false;
                }
                break;
            }
        }
        self.settle(learned)?;
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

    /// Everything polled is in `ClickHouse`: save the checkpoints and purge
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

/// Where a batch starts in `recording`: its checkpoint, or its start in the
/// archive if that is later or it has none. Neither moves while a batch is
/// pending: only a commit moves the checkpoint, and segments are purged only
/// behind it.
fn batch_origin(
    checkpoints: &BTreeMap<i64, i64>,
    starts: &BTreeMap<i64, i64>,
    recording: i64,
) -> Option<i64> {
    checkpoints
        .get(&recording)
        .max(starts.get(&recording))
        .copied()
}

/// How much of the archive the batch being read spans, over every recording.
fn batch_span(
    polled: &BTreeMap<i64, i64>,
    checkpoints: &BTreeMap<i64, i64>,
    starts: &BTreeMap<i64, i64>,
) -> i64 {
    polled
        .iter()
        .map(|(&id, &p)| batch_origin(checkpoints, starts, id).map_or(0, |o| (p - o).max(0)))
        .sum()
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

/// What a replayed frame needs to know about its recording.
#[derive(Clone, Copy)]
struct Meta<'a> {
    feed: bool,
    /// A feed's service and kind, when its frames become `frame` rows.
    names: Option<&'a (String, String)>,
    session_id: i32,
    stream_id: i32,
}

/// One replayed frame to the writer: as its table's row and, for a feed
/// whose frames are kept, as a `frame` row. A persist frame's reserved value
/// is the recording app's source id; a feed frame's is its publish time, and
/// its source the recording's (`source`, learned from its `Source` message).
fn take(
    writer: &mut Writer,
    frame_row: &mut Vec<u8>,
    meta: Meta<'_>,
    (recording, position, reserved): (i64, i64, i64),
    source: &mut u64,
    message: &[u8],
) {
    let from = if meta.feed {
        if let Some(id) = source_id(message) {
            *source = id;
        }
        *source
    } else {
        reserved.cast_unsigned()
    };
    writer.push_at(message, from, meta.feed, (recording, position));
    if let Some((service, kind)) = meta.names {
        let row = FrameRow {
            ts: Nanos(reserved),
            recording,
            position,
            session: meta.session_id,
            stream: meta.stream_id,
            source: from,
            service,
            kind,
            message,
        };
        if row.encode(frame_row).is_ok() {
            writer.push_at(frame_row, from, true, (recording, position));
        }
    }
}

/// The source id a `Source` message names; `None` for any other message.
fn source_id(message: &[u8]) -> Option<u64> {
    if message.get(2..4) != Some(&ergon_runtime::source::SOURCE_TEMPLATE_ID.to_le_bytes()[..])
        || message.get(4..6) != Some(&ergon_runtime::event::SCHEMA_ID.to_le_bytes()[..])
    {
        return None;
    }
    ergon_runtime::source::Source::decode(message).map(|s| s.id)
}

/// The insert's identity: one token per batch, stable across the retry.
/// `ClickHouse` drops a later insert that repeats it, whichever rows it holds.
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
    use std::fmt::Write as _;

    let tmp = path.with_extension("tmp");
    let mut text = String::new();
    for (recording, position) in checkpoints {
        let _ = writeln!(text, "{recording} {position}");
    }
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

/// Record each of `feeds` through its spy, and track its stream id. The
/// archive keeps one recording per spy and stream, so a feed recorded
/// already is not recorded twice, and one whose spy changed (its publisher
/// moved to another port) is recorded again.
fn record_feeds(
    archive: &AeronArchive,
    feeds: &[RecordedFeed],
    tracked: &mut BTreeMap<i32, (String, String)>,
) -> Result<(), Error> {
    for feed in feeds {
        record(archive, &feed.spy, feed.stream_id)?;
        tracked.insert(feed.stream_id, (feed.service.clone(), feed.kind.clone()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_batch_starts_at_its_checkpoint_else_at_its_recording_start() {
        let checkpoints = BTreeMap::from([(1, 500), (2, 100)]);
        let starts = BTreeMap::from([(1, 0), (2, 300), (3, 700)]);
        assert_eq!(batch_origin(&checkpoints, &starts, 1), Some(500));
        // Purged past its checkpoint by something else: its start.
        assert_eq!(batch_origin(&checkpoints, &starts, 2), Some(300));
        assert_eq!(batch_origin(&checkpoints, &starts, 3), Some(700));
        assert_eq!(batch_origin(&checkpoints, &starts, 4), None);
        // Every recording's part counts toward the batch's span.
        let polled = BTreeMap::from([(1, 1500), (2, 300), (3, 900), (4, 50)]);
        assert_eq!(batch_span(&polled, &checkpoints, &starts), 1000 + 200);
    }

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
