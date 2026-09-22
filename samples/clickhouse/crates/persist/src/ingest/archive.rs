//! Aeron Archive replay source and post-acknowledgement pruning.
//!
//! The replay source follows an Archive recording through a replay
//! subscription, resuming at the persisted checkpoint. Pruning honors the
//! plan contract: segments are removed only past the acknowledged
//! ingestion checkpoint (aligned down to a segment-file boundary), never
//! below unconsumed data. Active recordings are pruned with
//! `purge_segments` (recording stays live); stopped recordings are
//! `truncate_recording`d.
//!
//! The Java ArchivingMediaDriver runs the latest Aeron (1.53.2) with
//! default archive settings (D3 — no sync-level overrides); the bundled C
//! client (1.52.2) is protocol-compatible.

use crate::ingest::{IngestSource, ReplayBudget, SourceBatch, SourceError, SourcePosition};
use std::cell::RefCell;
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::time::Duration;

/// Typed errors for the Archive replay/pruning surface, matching the
/// `ChError`/`RegistrationError` shape already used elsewhere in this
/// crate (hand-rolled enum + `Display` + `Error`, no `thiserror` dependency
/// — this crate has never taken one).
#[derive(Debug)]
pub enum ArchiveError {
    /// I/O failure: port bind, directory/file creation, log file, process
    /// spawn (excluding the "no jar"/"no java" cases below, which are
    /// named separately because a bare `NotFound` from `spawn` cannot tell
    /// them apart).
    Io(String),
    /// The Aeron archive jar or the `java` binary could not be found or run.
    JavaUnavailable(String),
    /// rusteron/Aeron client, archive-context, or connect setup failed.
    Aeron(String),
    /// A native archive FFI call returned a nonzero result code.
    Ffi { op: &'static str, rc: i32 },
    /// A checkpoint lies outside the retained archive history.
    CheckpointOutsideHistory { checkpoint: i64, archive_start: i64 },
    /// `start_replay` returned an invalid (non-positive) session id.
    InvalidReplaySession(i64),
}

impl std::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(m) => write!(f, "io: {m}"),
            Self::JavaUnavailable(m) => write!(f, "java unavailable: {m}"),
            Self::Aeron(m) => write!(f, "aeron: {m}"),
            Self::Ffi { op, rc } => write!(f, "{op} rc={rc}"),
            Self::CheckpointOutsideHistory {
                checkpoint,
                archive_start,
            } => write!(
                f,
                "checkpoint {checkpoint} outside archive history ({archive_start})"
            ),
            Self::InvalidReplaySession(id) => write!(f, "start_replay returned session id {id}"),
        }
    }
}

impl std::error::Error for ArchiveError {}

impl From<std::io::Error> for ArchiveError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

/// Control-plane endpoints for one Archive (mirrors the lab harness).
#[derive(Clone, Debug)]
pub struct ArchiveEndpoints {
    /// Archive control request channel (server-listened).
    pub control: String,
    /// Control response channel for this client.
    pub control_response: String,
    /// Recording events channel.
    pub recording_events: String,
    /// Replay data channel (client-listened subscription base).
    pub replay: String,
    /// Data channel being recorded. Producer pods record over IPC
    /// (single-host, shared-memory with the pod-local driver); replay to
    /// the ingester goes over UDP.
    pub recorded_channel: String,
    /// Recorded stream id.
    pub recorded_stream_id: i32,
}

/// One recording descriptor resolved from the archive catalog.
#[derive(Clone, Copy, Debug)]
pub struct RecordingInfo {
    /// Archive-assigned recording id.
    pub recording_id: i64,
    /// First recorded position (advances after segment purges).
    pub start_position: i64,
    /// Stop position; negative means still recording (active).
    pub stop_position: i64,
}

impl RecordingInfo {
    /// Whether the recording is still accepting data.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.stop_position < 0
    }
}

/// Handle to a running ArchivingMediaDriver (test/lab harness).
///
/// Launches the LATEST Aeron (`aeron-all-1.53.2`) with Aeron's default
/// archive settings — the sample deliberately leaves
/// `aeron.archive.file.sync.level` and the catalog sync level unset (D3);
/// the explicit `-D...sync.level=0` here only pins the verified default
/// so the runtime value is reproducible, it does not override it.
pub struct ArchiveServer {
    pub child: Child,
    /// Aeron media driver directory (removed on drop).
    pub aeron_dir: std::path::PathBuf,
    /// Archive recordings/catalog directory (removed on drop).
    pub archive_dir: std::path::PathBuf,
    /// Endpoints used at launch.
    pub endpoints: ArchiveEndpoints,
}

/// Find a free UDP port.
pub fn free_udp_port() -> Result<u16, ArchiveError> {
    Ok(std::net::UdpSocket::bind("127.0.0.1:0")?
        .local_addr()?
        .port())
}

impl ArchiveServer {
    /// Launch `java -cp aeron-all-1.53.2.jar io.aeron.archive.ArchivingMediaDriver`.
    ///
    /// `segment_length` sets `aeron.archive.segment.file.length` (and the
    /// term buffer length) so lab runs can exercise segment pruning
    /// without gigabyte segment files.
    pub fn launch(
        jar: &str,
        base_dir: &std::path::Path,
        stream_id: i32,
        segment_length: usize,
    ) -> Result<Self, ArchiveError> {
        Self::launch_with_log(jar, base_dir, stream_id, segment_length, None)
    }

    /// Launch with JVM stderr captured to `log_path` (diagnostics).
    pub fn launch_with_log(
        jar: &str,
        base_dir: &std::path::Path,
        stream_id: i32,
        segment_length: usize,
        log_path: Option<&std::path::Path>,
    ) -> Result<Self, ArchiveError> {
        let aeron_dir = base_dir.join("aeron");
        let archive_dir = base_dir.join("archive");
        let _ = std::fs::remove_dir_all(&aeron_dir);
        let _ = std::fs::remove_dir_all(&archive_dir);
        std::fs::create_dir_all(&aeron_dir)?;
        std::fs::create_dir_all(&archive_dir)?;

        // Allocate a contiguous port block from ONE base port to avoid
        // sequential free-port reuse collisions.
        let base_port = free_udp_port()?;
        let (control_port, response_port, events_port, replay_port, data_port) = (
            base_port,
            base_port + 1,
            base_port + 2,
            base_port + 3,
            base_port + 4,
        );

        let _ = data_port;
        let endpoints = ArchiveEndpoints {
            control: format!("aeron:udp?endpoint=localhost:{control_port}"),
            control_response: format!("aeron:udp?endpoint=localhost:{response_port}"),
            recording_events: format!("aeron:udp?endpoint=localhost:{events_port}"),
            // Replay channel: IPC when the consumer shares the driver
            // (lab/tests); UDP in deployment (ingester on another pod).
            replay: std::env::var("ERGO_REPLAY_CHANNEL")
                .unwrap_or_else(|_| format!("aeron:udp?endpoint=localhost:{replay_port}")),
            recorded_channel: "aeron:ipc".to_string(),
            recorded_stream_id: stream_id,
        };

        // JUL config so ArchivingMediaDriver startup/replay diagnostics
        // actually reach the log file (Aeron logs via java.util.logging).
        let jul_path = base_dir.join("jul.properties");
        std::fs::write(
            &jul_path,
            "handlers=java.util.logging.ConsoleHandler\n.level=INFO\njava.util.logging.ConsoleHandler.level=INFO\n",
        )?;

        // A missing jar or a missing `java` both surface as a bare
        // `Os { code: 2, NotFound }` from `spawn`, which says nothing about
        // which of the two is absent. Name them.
        if !std::path::Path::new(jar).is_file() {
            return Err(ArchiveError::JavaUnavailable(format!(
                "Aeron archive jar not found at {jar}; pass --archive-jar or set \
                 ERGO_AERON_JAR"
            )));
        }
        let child = Command::new("java")
            .args([
                "--add-opens",
                "java.base/jdk.internal.misc=ALL-UNNAMED",
                &format!("-Djava.util.logging.config.file={}", jul_path.display()),
                "-Daeron.event.archive.log=all",
                "-cp",
                jar,
                &format!("-Daeron.dir={}", aeron_dir.display()),
                &format!("-Daeron.archive.dir={}", archive_dir.display()),
                &format!("-Daeron.archive.control.channel={}", endpoints.control),
                &format!(
                    "-Daeron.archive.control.response.channel={}",
                    endpoints.control_response
                ),
                &format!(
                    "-Daeron.archive.recording.events.channel={}",
                    endpoints.recording_events
                ),
                "-Daeron.archive.replication.channel=aeron:udp?endpoint=localhost:0",
                &format!("-Daeron.archive.segment.file.length={segment_length}"),
                &format!("-Daeron.term.buffer.length={segment_length}"),
                "-Daeron.client.liveness.timeout=60000000000",
                "-Daeron.image.liveness.timeout=60000000000",
                "-Daeron.publication.unblock.timeout=65000000000",
                "io.aeron.archive.ArchivingMediaDriver",
            ])
            .stdout(match log_path {
                Some(p) => Stdio::from(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(p)?,
                ),
                None => Stdio::null(),
            })
            .stderr(match log_path {
                Some(p) => Stdio::from(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(p)?,
                ),
                None => Stdio::null(),
            })
            .spawn()
            .map_err(|e| {
                ArchiveError::JavaUnavailable(format!(
                    "cannot spawn `java` for the Aeron archive: {e}; install a JRE or attach \
                     to an existing archive with --aeron-dir"
                ))
            })?;
        Ok(Self {
            child,
            aeron_dir,
            archive_dir,
            endpoints,
        })
    }
}

impl Drop for ArchiveServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.aeron_dir);
        let _ = std::fs::remove_dir_all(&self.archive_dir);
    }
}

/// A connected Aeron + archive client pair sharing one media driver dir.
pub struct ArchiveClient {
    pub archive: rusteron_archive::AeronArchive,
    pub aeron_dir: String,
    endpoints: ArchiveEndpoints,
}

/// Raw FFI wrappers for the pruning calls (rusteron's high-level API does
/// not expose them; the C client is bundled and bound by allowlist).
mod ffi {
    use super::ArchiveError;
    use rusteron_archive::bindings;

    /// # Safety
    /// `archive` must be a valid, connected `aeron_archive_t`.
    pub unsafe fn purge_segments(
        archive: *mut bindings::aeron_archive_t,
        recording_id: i64,
        new_start_position: i64,
    ) -> Result<i64, ArchiveError> {
        let mut count: i64 = 0;
        let rc = unsafe {
            bindings::aeron_archive_purge_segments(
                &mut count,
                archive,
                recording_id,
                new_start_position,
            )
        };
        if rc == 0 {
            Ok(count)
        } else {
            Err(ArchiveError::Ffi {
                op: "purge_segments",
                rc,
            })
        }
    }

    /// # Safety
    /// `archive` must be a valid, connected `aeron_archive_t`.
    pub unsafe fn get_recording_position(
        archive: *mut bindings::aeron_archive_t,
        recording_id: i64,
    ) -> Result<i64, ArchiveError> {
        let mut position: i64 = 0;
        let rc = unsafe {
            bindings::aeron_archive_get_recording_position(&mut position, archive, recording_id)
        };
        if rc == 0 {
            Ok(position)
        } else {
            Err(ArchiveError::Ffi {
                op: "get_recording_position",
                rc,
            })
        }
    }

    /// # Safety
    /// `archive` must be a valid, connected `aeron_archive_t`.
    pub unsafe fn truncate_recording(
        archive: *mut bindings::aeron_archive_t,
        recording_id: i64,
        position: i64,
    ) -> Result<i64, ArchiveError> {
        let mut count: i64 = 0;
        let rc = unsafe {
            bindings::aeron_archive_truncate_recording(&mut count, archive, recording_id, position)
        };
        if rc == 0 {
            Ok(count)
        } else {
            Err(ArchiveError::Ffi {
                op: "truncate_recording",
                rc,
            })
        }
    }
}

impl ArchiveClient {
    /// Connect an Aeron client (same driver dir) and the archive client.
    ///
    /// rusteron-archive vendors its own client copy, so this holds a
    /// SEPARATE Aeron client handle from the producer's publication
    /// client; both attach to the same media driver directory.
    pub fn connect(
        endpoints: &ArchiveEndpoints,
        aeron_dir: &std::path::Path,
    ) -> Result<Self, ArchiveError> {
        let map = |e: rusteron_archive::AeronCError| ArchiveError::Aeron(e.to_string());
        let archive_ctx = rusteron_archive::AeronArchiveContext::new().map_err(map)?;
        archive_ctx
            .set_aeron_directory_name(&rusteron_archive::cformat!("{}", aeron_dir.display()))
            .map_err(map)?;
        archive_ctx
            .set_control_request_channel(&rusteron_archive::cformat!("{}", endpoints.control))
            .map_err(map)?;
        archive_ctx
            .set_control_response_channel(&rusteron_archive::cformat!(
                "{}",
                endpoints.control_response
            ))
            .map_err(map)?;
        archive_ctx
            .set_recording_events_channel(&rusteron_archive::cformat!(
                "{}",
                endpoints.recording_events
            ))
            .map_err(map)?;
        let archive = rusteron_archive::AeronArchiveAsyncConnect::new(&archive_ctx)
            .map_err(map)?
            .poll_blocking(Duration::from_secs(30))
            .map_err(map)?;
        Ok(Self {
            archive,
            aeron_dir: aeron_dir.display().to_string(),
            endpoints: endpoints.clone(),
        })
    }

    /// Resolve the recording for the configured channel/stream.
    pub fn find_recording(&self, endpoints: &ArchiveEndpoints) -> Option<RecordingInfo> {
        let d = self
            .archive
            .find_recording_for_stream(endpoints.recorded_stream_id)
            .ok()??;
        Some(RecordingInfo {
            recording_id: d.recording_id,
            start_position: d.start_position,
            stop_position: d.stop_position,
        })
    }

    /// Current descriptor for the configured recording, with positions as
    /// of this call (they advance while the recording is active).
    pub fn current(&self) -> Option<RecordingInfo> {
        self.find_recording(&self.endpoints)
    }

    /// How far an ACTIVE recording has been written.
    ///
    /// `RecordingInfo::stop_position` stays `-1` until the recording is
    /// stopped, so this is the only way to bound a bounded replay of a live,
    /// continuously-growing recording. `Err` for an unknown recording id.
    pub fn recording_position(&self, recording_id: i64) -> Result<i64, ArchiveError> {
        unsafe { ffi::get_recording_position(self.archive.get_inner(), recording_id) }
    }

    /// Prune an ACTIVE recording: purge segments below `new_start_position`.
    pub fn purge_segments(
        &self,
        recording_id: i64,
        new_start_position: i64,
    ) -> Result<i64, ArchiveError> {
        unsafe { ffi::purge_segments(self.archive.get_inner(), recording_id, new_start_position) }
    }

    /// Prune a STOPPED recording: truncate to `position`.
    pub fn truncate_recording(
        &self,
        recording_id: i64,
        position: i64,
    ) -> Result<i64, ArchiveError> {
        unsafe { ffi::truncate_recording(self.archive.get_inner(), recording_id, position) }
    }
}

/// Safety window: prune no closer than one segment behind the acknowledged
/// checkpoint boundary, never past the recording start.
#[must_use]
pub fn prune_position(acknowledged: i64, segment_length: usize, recording_start: i64) -> i64 {
    let seg = segment_length as i64;
    let aligned = acknowledged.div_euclid(seg) * seg;
    (aligned - seg).max(recording_start)
}

/// Prune after an acknowledged checkpoint: active → purge segments,
/// stopped → truncate. Returns the segments removed (0 when the window
/// retained everything).
pub fn prune_after_checkpoint(
    client: &ArchiveClient,
    info: &RecordingInfo,
    acknowledged: i64,
    segment_length: usize,
) -> Result<i64, ArchiveError> {
    let target = prune_position(acknowledged, segment_length, info.start_position);
    if target <= info.start_position {
        return Ok(0);
    }
    if info.is_active() {
        client.purge_segments(info.recording_id, target)
    } else {
        client.truncate_recording(info.recording_id, target)
    }
}

const AERON_FRAME_HEADER: i64 = 32;

/// Frames delivered by one replay subscription, in arrival order.
#[derive(Default)]
struct ReplaySink {
    frames: Vec<Vec<u8>>,
    bytes: usize,
}

/// Fragment callback appending assembled messages to a shared sink.
struct FrameCollector(Rc<RefCell<ReplaySink>>);

impl rusteron_client::AeronFragmentHandlerCallback for FrameCollector {
    fn handle_aeron_fragment_handler(
        &mut self,
        buffer: &[u8],
        _header: rusteron_client::AeronHeader,
    ) {
        let mut sink = self.0.borrow_mut();
        sink.bytes += buffer.len();
        sink.frames.push(buffer.to_vec());
    }
}

/// Replay source over one Archive recording, resuming at a checkpoint.
///
/// The source owns its Aeron client, a replay subscription bound to the
/// replay session id, and a fragment-assembling handler, so a record that
/// exceeds one UDP datagram is delivered whole.
///
/// Positions advance by [`AERON_FRAME_HEADER`] + message length per
/// delivered message. That is exact for unfragmented recordings (IPC or
/// sub-MTU messages). A fragmented message advances the true recording
/// position by more than this estimate, so the reported position is a
/// lower bound: a resumed replay re-reads rather than skips, and
/// duplicate rows collapse on the destination's event identity.
///
/// Poll closure over an Aeron subscription.
type AeronPollFn = dyn Fn(&rusteron_client::AeronSubscription) -> Result<i32, ArchiveError>;

pub struct ArchiveReplaySource {
    _aeron: rusteron_client::Aeron,
    subscription: rusteron_client::AeronSubscription,
    /// Kept alive: the assembler state behind the handler closure.
    _inner: Box<dyn std::any::Any>,
    poll: Box<AeronPollFn>,
    sink: Rc<RefCell<ReplaySink>>,
    position: SourcePosition,
}

impl ArchiveReplaySource {
    /// Start a bounded replay of `recording` from `from_position` and
    /// subscribe to it. `budget_bytes` bounds how much the driver is
    /// asked to deliver per poll cycle.
    pub fn subscribe(
        aeron_dir: &std::path::Path,
        replay_channel: &str,
        replay_stream_id: i32,
        client: &ArchiveClient,
        recording: &RecordingInfo,
        from_position: i64,
    ) -> Result<Self, ArchiveError> {
        if from_position < recording.start_position {
            return Err(ArchiveError::CheckpointOutsideHistory {
                checkpoint: from_position,
                archive_start: recording.start_position,
            });
        }
        // A bounded replay needs a stop position, and an active recording
        // reports a negative one. Refusing it outright — which this used to do
        // — makes a live recording unreplayable, and a live recording is never
        // stopped: `--follow` died on `recording N is still active` and
        // CrashLooped, so no live byte ever reached ClickHouse. An active
        // recording is still replayable, just not beyond what has been written,
        // so ask the Archive where it has reached and bound the replay there.
        // The caller treats an active recording as never caught up, idles the
        // pass out and re-resolves, which is how bytes written after this
        // position are picked up.
        let stop = if recording.is_active() {
            client.recording_position(recording.recording_id)?
        } else {
            recording.stop_position
        };
        // Aeron rejects a replay whose start is not FRAME_ALIGNMENT-aligned,
        // and the reported position is a byte estimate (32-byte header plus
        // payload) that drifts off the real alignment on padded frames. Align
        // DOWN: re-reading a few bytes is safe because a resumed replay
        // re-reads rather than skips and duplicates collapse on the
        // destination's event identity.
        // Always replay from the recording's start. The reported position is a
        // byte estimate (32-byte header plus payload), so on a recording with
        // padded frames it lands between real frames, and Aeron rejects a
        // start that is not a frame boundary — probing for an acceptable
        // position opens a second replay session that interferes with the real
        // one. Re-reading is safe and is what the position contract already
        // promises: a resumed replay re-reads rather than skips, and duplicate
        // rows collapse on the destination's event identity.
        let _ = from_position;
        let start = recording.start_position;
        let length = (stop - start).max(0);
        // bounding_limit_counter_id = -1 is UNBOUNDED; 0 silently caps the
        // replay to counter 0 (seen as CMD_IN_START_BOUNDED_REPLAY).
        let replay_params =
            rusteron_archive::AeronArchiveReplayParams::new(-1, i32::MAX, start, length, 0, 0)
                .map_err(|e| ArchiveError::Aeron(e.to_string()))?;
        let session_id = client
            .archive
            .start_replay(
                recording.recording_id,
                &rusteron_archive::cformat!("{replay_channel}"),
                replay_stream_id,
                &replay_params,
            )
            .map_err(|e| ArchiveError::Aeron(e.to_string()))?;
        if session_id <= 0 {
            return Err(ArchiveError::InvalidReplaySession(session_id));
        }

        let client_map = |e: rusteron_client::AeronCError| ArchiveError::Aeron(e.to_string());
        let ctx = rusteron_client::AeronContext::new().map_err(client_map)?;
        ctx.set_dir(&rusteron_client::cformat!("{}", aeron_dir.display()))
            .map_err(client_map)?;
        let aeron = rusteron_client::Aeron::new(&ctx).map_err(client_map)?;
        aeron.start().map_err(client_map)?;

        // Subscribe on the plain replay channel. The `session-id` tag is meant
        // to stop a concurrent replay on the same channel from interleaving,
        // but the tag must equal the *archive's replay publication session id*,
        // not the id this client got back — and when they differ the driver
        // silently discards every frame. Observed live: the Archive logged 87
        // `FRAME_OUT` to the ingester's replay port while the ingester
        // collected nothing at all. This sample gives each replay its own port,
        // so the tag buys nothing and costs delivery.
        let channel = rusteron_client::cformat!("{replay_channel}");
        let subscription = aeron
            .add_subscription(
                &channel,
                replay_stream_id,
                rusteron_client::Handlers::NONE,
                rusteron_client::Handlers::NONE,
                Duration::from_secs(10),
            )
            .map_err(client_map)?;

        let sink: Rc<RefCell<ReplaySink>> = Rc::new(RefCell::new(ReplaySink::default()));
        let (handler, inner) =
            rusteron_client::Handler::with_fragment_assembler(FrameCollector(Rc::clone(&sink)))
                .map_err(client_map)?;
        let poll = Box::new(
            move |sub: &rusteron_client::AeronSubscription| -> Result<i32, ArchiveError> {
                sub.poll(Some(&handler), 128)
                    .map_err(|e| ArchiveError::Aeron(e.to_string()))
            },
        );

        Ok(Self {
            _aeron: aeron,
            subscription,
            _inner: Box::new(inner),
            poll,
            sink,
            position: start,
        })
    }

    /// Current replay position (checkpoint candidate).
    #[must_use]
    pub const fn position(&self) -> SourcePosition {
        self.position
    }
}

impl IngestSource for ArchiveReplaySource {
    fn next_batch(&mut self, budget: &ReplayBudget) -> Result<Option<SourceBatch>, SourceError> {
        // Poll while the sink is empty so an idle source blocks briefly
        // rather than spinning the caller's loop. The window must cover
        // replay connection setup (setup frames, publication connect, term
        // delivery), which takes far longer than a single poll cycle.
        for _ in 0..40 {
            if !self.sink.borrow().frames.is_empty() {
                break;
            }
            if (self.poll)(&self.subscription)
                .map_err(|e| SourceError::Disconnected(e.to_string()))?
                == 0
            {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let mut items = Vec::new();
        let mut bytes = 0usize;
        let start = self.position;
        {
            let mut sink = self.sink.borrow_mut();
            // Take a whole prefix: `drain(..take)` with an exact range
            // leaves later frames for the next batch.
            let mut take = 0usize;
            while take < sink.frames.len() {
                let n = sink.frames[take].len();
                if take > 0 && bytes + n > budget.bytes {
                    break;
                }
                bytes += n;
                take += 1;
            }
            for frame in sink.frames.drain(..take) {
                self.position += AERON_FRAME_HEADER + frame.len() as i64;
                items.push(crate::ingest::classify_frame(&frame)?);
            }
        }
        if items.is_empty() {
            return Ok(None);
        }
        Ok(Some(SourceBatch {
            items,
            start_position: start,
            end_position: self.position,
            bytes,
        }))
    }
}
