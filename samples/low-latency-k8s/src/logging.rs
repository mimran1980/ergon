//! Logging: `tracing-subscriber`'s `fmt` to stdout through
//! `tracing-appender`'s non-blocking writer, lossy and bounded.
//!
//! ```text
//! caller thread                                   ll-log (shared CPU)
//! ─────────────                                   ───────────────────
//! level check (static max level, then filter)
//! fmt: format into a thread-local String
//!      (timestamp read, fields; reused, so it
//!      allocates only on the thread's first event)
//! make_writer_for: clone NonBlocking (Arc + Sender:
//!      atomic read-modify-write on shared lines)
//! write: copy the line into a new Vec (1 allocation,
//!      even when the queue is full)
//! try_send: full queue → drop, count, return     ──▶ recv, write(2) to stdout
//!      (a futex wake when ll-log sleeps)
//! drop the NonBlocking clone (atomics)
//! ```
//!
//! What this means for a hot thread:
//!
//! - It never blocks on stdout: a full queue drops the line and counts it
//!   ([`Logging::dropped_lines`]); only `ll-log` blocks in `write(2)`.
//! - A write that fails on `ll-log` (EPIPE, EIO) loses its line too.
//!   tracing-appender 0.2.5 discards that error without counting it, so the
//!   sink is wrapped in [`CountingWriter`] and the loss is reported apart
//!   ([`Logging::write_errors`]); `dropped_lines` counts only queue-full
//!   drops.
//! - It still pays to format the event, one allocation and the channel's
//!   atomics, and sometimes a futex wake (an IPI to the shared CPU). So the
//!   hot loop logs only rare events: rate-limited warnings
//!   ([`crate::ratelimit`]). `tests/logger_alloc.rs` pins the allocation
//!   count and `benches/log_event.rs` measures the cost.
//! - `log_internal_errors(false)`: a failed write never runs a blocking
//!   `eprintln!` on the caller.
//!
//! Where `ll-log` runs: `finish()` spawns it, and a new thread inherits its
//! creator's affinity, so it is spawned from `main` while `main` is on the
//! shared CPUs. [`place_worker`] then checks its mask and re-pins it by
//! thread id when it is not within the helper CPUs (the plugin's
//! `defaultAffinity: all`, or a caller that pinned first).
//!
//! Shutdown: dropping the `WorkerGuard` waits up to 100 ms to queue the
//! shutdown and up to 1 s for the flush; it does **not** join `ll-log`. When
//! the queue is full it prints a line with `println!`, which blocks if
//! stdout is wedged. So the app writes its report to `/dev/termination-log`
//! first, then drops the guard, then prints the report: late log lines can
//! be lost, the report is not.

use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::{ErrorCounter, NonBlocking, NonBlockingBuilder, WorkerGuard};

use crate::LlError;
use crate::cpus::CpuList;

/// How to log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogConfig {
    /// The most verbose level kept (`LL_LOG_LEVEL`); the build caps it at
    /// `info` (`release_max_level_info`).
    pub level: LevelFilter,
    /// Lines the queue holds before it drops (`LL_LOG_QUEUE_LINES`).
    pub queue_lines: usize,
    /// The worker thread's name (at most 15 bytes: the kernel's `comm`).
    pub thread_name: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LevelFilter::INFO,
            queue_lines: 4096,
            thread_name: "ll-log".to_owned(),
        }
    }
}

/// Parses `LL_LOG_LEVEL`: `off`, `error`, `warn`, `info`, `debug`,
/// `trace`.
///
/// # Errors
///
/// [`LlError::BadValue`] for anything else.
pub fn parse_level(text: &str) -> Result<LevelFilter, LlError> {
    text.parse().map_err(|_| {
        LlError::bad(
            "LL_LOG_LEVEL",
            text,
            "off, error, warn, info, debug or trace",
        )
    })
}

/// Failed writes and flushes on the worker, shared with whoever reports
/// them.
#[derive(Clone, Debug, Default)]
pub struct WriteErrors(Arc<AtomicU64>);

impl WriteErrors {
    /// How many so far.
    #[must_use]
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }

    fn bump(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

/// `inner`, counting each failed `write_all` (one log line lost) and each
/// failed `flush` in [`WriteErrors`]. Runs on the worker, never on the
/// caller.
#[derive(Debug)]
pub struct CountingWriter<W> {
    inner: W,
    errors: WriteErrors,
}

impl<W: Write> CountingWriter<W> {
    /// Wraps `inner`.
    pub const fn new(inner: W, errors: WriteErrors) -> Self {
        Self { inner, errors }
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.inner
            .write_all(buf)
            .inspect_err(|_| self.errors.bump())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush().inspect_err(|_| self.errors.bump())
    }
}

/// The non-blocking writer, its guard and its two loss counters.
pub struct Appender {
    /// The writer `fmt` writes to (cheap to clone).
    pub writer: NonBlocking,
    /// Keeps the worker alive; dropping it flushes (see the module docs).
    pub guard: WorkerGuard,
    /// Lines dropped because the queue was full.
    pub dropped: ErrorCounter,
    /// Lines (and flushes) lost to a failed write on the worker.
    pub write_errors: WriteErrors,
}

/// The lossy, bounded non-blocking writer over `sink` (wrapped in
/// [`CountingWriter`]). Spawns the worker thread, named `cfg.thread_name`,
/// from the calling thread (it inherits the caller's affinity).
pub fn non_blocking<W: Write + Send + 'static>(sink: W, cfg: &LogConfig) -> Appender {
    let write_errors = WriteErrors::default();
    let (writer, guard) = NonBlockingBuilder::default()
        .lossy(true)
        .buffered_lines_limit(cfg.queue_lines)
        .thread_name(&cfg.thread_name)
        .finish(CountingWriter::new(sink, write_errors.clone()));
    let dropped = writer.error_counter();
    Appender {
        writer,
        guard,
        dropped,
        write_errors,
    }
}

/// The `fmt` subscriber over `writer`: no ANSI, thread names, no target, the
/// configured level, internal errors not printed.
#[must_use]
pub fn subscriber(
    writer: NonBlocking,
    cfg: &LogConfig,
) -> impl tracing::Subscriber + Send + Sync + 'static {
    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_thread_names(true)
        .with_target(false)
        .with_max_level(cfg.level)
        .log_internal_errors(false)
        .finish()
}

/// Where the worker thread ended up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkerPlacement {
    /// Its thread id.
    pub tid: u32,
    /// The mask it inherited.
    pub inherited: CpuList,
    /// Its mask now.
    pub now: CpuList,
    /// Whether [`place_worker`] re-pinned it.
    pub repinned: bool,
}

/// Finds this process's thread named `name` (waiting up to a second for it
/// to name itself) and makes sure its affinity is within `helpers`,
/// re-pinning it by thread id when it is not.
///
/// # Errors
///
/// [`LlError::Timeout`] when no thread has that name, [`LlError::Io`] when
/// its mask cannot be read or set, [`LlError::Unsupported`] off Linux.
pub fn place_worker(name: &str, helpers: &CpuList) -> Result<WorkerPlacement, LlError> {
    let tid = find_thread(name)?;
    let inherited = crate::cpus::thread_affinity(tid)?;
    let repinned = !inherited.is_subset(helpers);
    if repinned {
        crate::cpus::pin_thread(tid, helpers)?;
    }
    let now = crate::cpus::thread_affinity(tid)?;
    Ok(WorkerPlacement {
        tid,
        inherited,
        now,
        repinned,
    })
}

/// The id of this process's thread named `name`: a new thread names itself
/// after it starts, so this looks for up to a second.
///
/// # Errors
///
/// [`LlError::Timeout`] when none has that name.
pub fn find_thread(name: &str) -> Result<u32, LlError> {
    let tasks = crate::probe::proc::Root::system();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    loop {
        for tid in tasks.numeric_entries("proc/self/task")? {
            if tasks
                .read(&format!("proc/self/task/{tid}/comm"))
                .is_ok_and(|comm| comm.trim_end() == name)
            {
                return Ok(tid);
            }
        }
        if std::time::Instant::now() > deadline {
            return Err(LlError::Timeout(format!(
                "no thread named {name} in /proc/self/task"
            )));
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// The process-wide logging: stdout through the non-blocking writer.
pub struct Logging {
    guard: Option<WorkerGuard>,
    dropped: ErrorCounter,
    write_errors: WriteErrors,
    /// Where `ll-log` runs (Linux).
    pub placement: Option<WorkerPlacement>,
}

impl Logging {
    /// Lines dropped so far because the queue was full.
    #[must_use]
    pub fn dropped_lines(&self) -> u64 {
        self.dropped.dropped_lines() as u64
    }

    /// Lines (and flushes) lost so far to a failed write on `ll-log`.
    #[must_use]
    pub fn write_errors(&self) -> u64 {
        self.write_errors.get()
    }

    /// Drops the guard: queues the shutdown (up to 100 ms) and waits for
    /// the flush (up to 1 s). It does not join the worker.
    pub fn shutdown(&mut self) {
        drop(self.guard.take());
    }
}

/// Installs the global subscriber on stdout and, when `helpers` is given,
/// places `ll-log` within them ([`place_worker`]).
///
/// # Errors
///
/// [`LlError::Logging`] when a global subscriber is already set, and the
/// errors of [`place_worker`].
pub fn init_stdout(cfg: &LogConfig, helpers: Option<&CpuList>) -> Result<Logging, LlError> {
    let Appender {
        writer,
        guard,
        dropped,
        write_errors,
    } = non_blocking(std::io::stdout(), cfg);
    tracing::subscriber::set_global_default(subscriber(writer, cfg))
        .map_err(|e| LlError::Logging(e.to_string()))?;
    let placement = match helpers {
        Some(h) => Some(place_worker(&cfg.thread_name, h)?),
        None => None,
    };
    Ok(Logging {
        guard: Some(guard),
        dropped,
        write_errors,
        placement,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_levels() -> Result<(), LlError> {
        assert_eq!(parse_level("info")?, LevelFilter::INFO);
        assert_eq!(parse_level("WARN")?, LevelFilter::WARN);
        assert_eq!(parse_level("off")?, LevelFilter::OFF);
        assert!(parse_level("loud").is_err());
        Ok(())
    }
}
