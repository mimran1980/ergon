//! An Aeron Archive as a simulation source: every recording of a subscribed
//! feed is replayed, and each is one head of the merge.
//!
//! A feed frame's reserved value is its publish time (see
//! [`Ctx::send`](super::Ctx::send)), which becomes the message's event time.
//! A recording is replayed from its start to its stop position, or to where
//! it had reached when the feed was opened if it is still recording. Each
//! head holds one frame, copied into a reused buffer.
//!
//! The simulation's [`Bus`] has no conductor thread: connecting to the
//! archive and waiting for a replay drive its conductor as they wait.
//!
//! ponytail: replays from each recording's start and skips frames before
//! `from`; seek by stamp (or the `frames` table) if long recordings make
//! that slow.

use std::time::{Duration, Instant};

use rusteron_archive::{
    Aeron, AeronArchive, AeronArchiveAsyncConnect, AeronArchiveContext, AeronArchiveReplayParams,
    AeronAsyncAddSubscription, AeronSubscription, Handlers, IntoCString,
};

use crate::Error;
use crate::bus::Bus;
use crate::clock::Nanos;
use crate::directory::Directory;
use crate::subscription::replay_image_session;

use super::FeedId;

/// Where the archive is, and the stream its replays use.
#[derive(Clone, Debug)]
pub struct ArchiveConfig {
    /// Its control request channel (`aeron:udp?endpoint=<host>:8010`).
    pub control: String,
    /// The channel it answers on, at this process.
    pub response: String,
    /// The stream every replay is sent on; replays are told apart by session.
    pub replay_stream: i32,
}

fn fail(e: impl std::fmt::Display) -> Error {
    Error::Aeron(e.to_string())
}

/// How long one replay may go without delivering before the run fails.
const STALL: Duration = Duration::from_secs(10);

enum Sub {
    Adding(AeronAsyncAddSubscription),
    Added(AeronSubscription),
}

/// One recording being replayed: its next frame, when it has one.
struct Head {
    recording: i64,
    session: i64,
    end: i64,
    sub: Sub,
    frame: Vec<u8>,
    ts: Nanos,
    /// Position after the frame held, or after the last one taken.
    position: i64,
    has: bool,
    done: bool,
}

/// The heads, by recording, and what each feeds.
pub(super) struct ArchiveSource {
    aeron: Aeron,
    /// This node, as the directory is asked from it.
    host_ip: String,
    archive: AeronArchive,
    replay_stream: i32,
    pub(super) names: Vec<String>,
    heads: Vec<Head>,
    pub(super) feeds: Vec<Vec<FeedId>>,
    /// Feed names already looked up.
    opened: Vec<String>,
}

impl ArchiveSource {
    pub(super) fn connect(bus: &Bus, config: &ArchiveConfig) -> Result<Self, Error> {
        let aeron = bus.aeron().clone();
        let ctx = AeronArchiveContext::new().map_err(fail)?;
        ctx.set_aeron(&aeron).map_err(fail)?;
        ctx.set_control_request_channel(&config.control.as_str().into_c_string())
            .map_err(fail)?;
        ctx.set_control_response_channel(&config.response.as_str().into_c_string())
            .map_err(fail)?;
        // Aeron's timeout, not ours: when it expires, Aeron closes what the
        // connect opened, and dropping an unfinished one closes nothing.
        ctx.set_message_timeout_ns(10_000_000_000).map_err(fail)?;
        let connect = AeronArchiveAsyncConnect::new_with_aeron(&ctx, &aeron).map_err(fail)?;
        // Not `poll_blocking`, which never runs the conductor the connect
        // waits on.
        let archive = loop {
            if let Some(archive) = connect
                .poll()
                .map_err(|e| Error::Aeron(format!("connecting to the archive: {e}")))?
            {
                break archive;
            }
            crate::bus::pause(&aeron);
        };
        Ok(Self {
            aeron,
            host_ip: bus.host_ip().to_owned(),
            archive,
            replay_stream: config.replay_stream,
            names: Vec::new(),
            heads: Vec::new(),
            feeds: Vec::new(),
            opened: Vec::new(),
        })
    }

    /// Replay every recording of each subscribed feed not yet opened, from
    /// `now` for one subscribed mid-run, and bind every head to its feeds.
    pub(super) fn bind(
        &mut self,
        subscribed: &[String],
        directory: &dyn Directory,
        now: Nanos,
        started: bool,
    ) -> Result<(), Error> {
        for name in subscribed {
            if self.opened.contains(name) {
                continue;
            }
            self.opened.push(name.clone());
            let Some((service, kind)) = name.split_once('/') else {
                continue;
            };
            // A raw channel (`subscribe_channel`), or a name the directory
            // does not know, has no recording to find by stream id.
            let stream_id = match directory.feed(service, kind, &self.host_ip) {
                Ok(addr) => addr.stream_id,
                Err(e) => {
                    log::warn!("archive replay: nothing to replay for {name}: {e}");
                    continue;
                }
            };
            let before = self.heads.len();
            self.open(name, stream_id)?;
            if started {
                for head in before..self.heads.len() {
                    while self.fill(head)? && self.heads[head].ts < now {
                        self.heads[head].has = false;
                    }
                }
            }
        }
        for (head, name) in self.names.iter().enumerate() {
            self.feeds[head] = subscribed
                .iter()
                .enumerate()
                .filter(|(_, s)| *s == name)
                .map(|(id, _)| FeedId(u32::try_from(id).unwrap_or(u32::MAX)))
                .collect();
        }
        Ok(())
    }

    fn open(&mut self, name: &str, stream_id: i32) -> Result<(), Error> {
        let mut found = Vec::new();
        self.archive
            .list_recordings_for_uri_fn(&mut 0, 0, i32::MAX, c"aeron", stream_id, |d| {
                found.push((d.recording_id(), d.start_position(), d.stop_position()));
            })
            .map_err(fail)?;
        for (recording, start, stop) in found {
            let end = if stop >= 0 {
                stop
            } else {
                self.archive
                    .get_max_recorded_position(recording)
                    .map_err(fail)?
            };
            if end <= start {
                continue;
            }
            let params =
                AeronArchiveReplayParams::new(-1, -1, start, end - start, -1, -1).map_err(fail)?;
            let session = self
                .archive
                .start_replay(recording, c"aeron:ipc", self.replay_stream, &params)
                .map_err(fail)?;
            let channel = format!("aeron:ipc?session-id={}", replay_image_session(session));
            let sub = self
                .aeron
                .async_add_subscription(
                    &channel.into_c_string(),
                    self.replay_stream,
                    Handlers::NONE,
                    Handlers::NONE,
                )
                .map_err(fail)?;
            log::info!("replaying {name}: recording {recording}, positions {start}..{end}");
            self.names.push(name.to_owned());
            self.feeds.push(Vec::new());
            self.heads.push(Head {
                recording,
                session,
                end,
                sub: Sub::Adding(sub),
                frame: Vec::new(),
                ts: Nanos(0),
                position: start,
                has: false,
                done: false,
            });
        }
        Ok(())
    }

    /// Fill every bound head that has no frame, waiting for the replay.
    pub(super) fn fill_all(&mut self) -> Result<(), Error> {
        for head in 0..self.heads.len() {
            if !self.feeds[head].is_empty() {
                self.fill(head)?;
            }
        }
        Ok(())
    }

    /// Make head `i` hold its next frame; `false` when its recording is done.
    fn fill(&mut self, i: usize) -> Result<bool, Error> {
        let head = &mut self.heads[i];
        let deadline = Instant::now() + STALL;
        while !head.has && !head.done {
            if head.position >= head.end {
                head.done = true;
                let _ = self.archive.stop_replay(head.session);
                break;
            }
            if let Sub::Adding(adding) = &head.sub
                && let Some(sub) = adding.poll().map_err(fail)?
            {
                head.sub = Sub::Added(sub);
            }
            if let Sub::Added(sub) = &head.sub {
                let (frame, ts, position, has) = (
                    &mut head.frame,
                    &mut head.ts,
                    &mut head.position,
                    &mut head.has,
                );
                sub.poll_fn(
                    |message, header| {
                        frame.clear();
                        frame.extend_from_slice(message);
                        *ts = Nanos(header.reserved_value().unwrap_or(0));
                        *position = header.position();
                        *has = true;
                    },
                    1,
                )
                .map_err(fail)?;
            }
            if !head.has {
                if Instant::now() > deadline {
                    return Err(Error::Aeron(format!(
                        "recording {}: no frame for {STALL:?} at position {} of {}",
                        head.recording, head.position, head.end
                    )));
                }
                // The subscription's add and the replay's image arrive
                // through the conductor.
                let _ = crate::bus::poll(&self.aeron);
            }
        }
        Ok(head.has)
    }

    /// Head `i`'s frame: `(ts, recording, position, frame)`.
    pub(super) fn head(&self, i: usize) -> Option<(Nanos, i64, i64, &[u8])> {
        let h = self.heads.get(i)?;
        h.has
            .then_some((h.ts, h.recording, h.position, h.frame.as_slice()))
    }

    pub(super) fn advance(&mut self, i: usize) {
        if let Some(h) = self.heads.get_mut(i) {
            h.has = false;
        }
    }

    pub(super) const fn len(&self) -> usize {
        self.heads.len()
    }
}
