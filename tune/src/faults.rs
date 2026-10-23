//! [`FaultWatch`] (T19): a hot thread's page-fault counters, read again
//! while the loop runs. A thread whose memory was touched and locked before
//! the loop takes none; one that faults stops while the kernel maps the
//! page in.

use std::fs::File;

use crate::cpu::Thread;
use crate::finding::{CheckId, Finding, Role, Severity};
use crate::host::{Host, ReadError};
use crate::quota::read_from_start;

const T19_EVIDENCE: &str = "Linux proc(5) (/proc/<pid>/task/<tid>/stat: minflt, field 10; \
     majflt, field 12); not measured here";

/// How many bytes a `stat` line is read into: about 300, with a 64-byte
/// command name at most.
const BUF: usize = 1024;

/// A task's page faults since it started.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Faults {
    /// Faults served without I/O (`minflt`): a page allocated, or found
    /// in memory and mapped.
    pub minor: u64,
    /// Faults that waited for I/O (`majflt`).
    pub major: u64,
}

impl Faults {
    /// The faults between `earlier` and `self` (none where a counter went
    /// back: another task).
    #[must_use]
    pub const fn since(&self, earlier: &Self) -> Self {
        Self {
            minor: self.minor.saturating_sub(earlier.minor),
            major: self.major.saturating_sub(earlier.major),
        }
    }
}

/// `minflt` (field 10) and `majflt` (field 12) of a `stat` line, counted
/// after the command name (which may hold spaces and parentheses).
fn parse_faults(text: &str) -> Result<Faults, &'static str> {
    let (_, after_comm) = text.rsplit_once(')').ok_or("no command name")?;
    // Fields after the command start at field 3 (state).
    let mut f = after_comm.split_whitespace().skip(10 - 3);
    let mut number = || -> Result<u64, &'static str> {
        f.next()
            .ok_or("fewer fields than a stat line has")?
            .parse()
            .map_err(|_| "a fault counter that is not a number")
    };
    let minor = number()?;
    let _children_minor = number()?;
    let major = number()?;
    Ok(Faults { minor, major })
}

/// T19 while the loop runs: a hot thread's page faults since the last
/// check.
///
/// [`FaultWatch::open`] opens the thread's `stat` once, before the loop.
/// The open file stays bound to that thread, so open it with
/// [`Thread::Current`] on the hot thread itself, or by id from anywhere
/// ([`Thread::Task`], or [`FaultWatch::open_task`] for another process's
/// thread). [`FaultWatch::check`] reads it again from its start (`pread`)
/// into a buffer it owns: no open, no allocation unless it reports. Call it
/// from housekeeping or idle-deferred work, never inside the loop's hot
/// path: once every few seconds is enough.
#[derive(Debug)]
pub struct FaultWatch {
    path: String,
    file: File,
    buf: Vec<u8>,
    role: Role,
    last: Faults,
    failed: bool,
}

impl FaultWatch {
    /// Opens `thread`'s `stat` (a thread of this process, in `role`) and
    /// reads it once.
    ///
    /// # Errors
    /// The file cannot be opened or read, or does not parse.
    pub fn open(host: &Host, thread: Thread, role: Role) -> Result<Self, ReadError> {
        Self::open_path(host, thread.file("stat"), role)
    }

    /// Opens thread `tid` of process `pid` (`/proc/<pid>/task/<tid>/stat`),
    /// in `role`, and reads it once: the media driver's agents seen from
    /// another process.
    ///
    /// # Errors
    /// The file cannot be opened or read, or does not parse.
    pub fn open_task(host: &Host, pid: u32, tid: u32, role: Role) -> Result<Self, ReadError> {
        Self::open_path(host, format!("/proc/{pid}/task/{tid}/stat"), role)
    }

    fn open_path(host: &Host, path: String, role: Role) -> Result<Self, ReadError> {
        let file = host.open(&path)?;
        let mut watch = Self {
            path,
            file,
            buf: vec![0; BUF],
            role,
            last: Faults::default(),
            failed: false,
        };
        watch.last = watch.sample()?;
        Ok(watch)
    }

    /// The `stat` file watched, on the host.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The counters at the last read.
    #[must_use]
    pub const fn last(&self) -> Faults {
        self.last
    }

    /// The counters now, from the open file, with no allocation (but for
    /// an error).
    ///
    /// # Errors
    /// The read fails (the thread ended), or the line does not parse.
    pub fn sample(&mut self) -> Result<Faults, ReadError> {
        let n = read_from_start(&self.file, &mut self.buf)
            .map_err(|e| ReadError::io(&self.path, &e))?;
        let malformed = |detail| ReadError::Malformed {
            path: self.path.clone(),
            detail,
        };
        let text = std::str::from_utf8(&self.buf[..n]).map_err(|_| malformed("not text"))?;
        parse_faults(text).map_err(malformed)
    }

    /// T19: a warning when the thread took a page fault since the last call
    /// (or [`Self::open`]), saying how many of each kind; `None` when it did
    /// not. A read that fails gives one unknown finding, the first time
    /// only.
    pub fn check(&mut self) -> Option<Finding> {
        let now = match self.sample() {
            Ok(f) => f,
            Err(e) => {
                if self.failed {
                    return None;
                }
                self.failed = true;
                return Some(Finding::unknown(
                    CheckId::T19,
                    Severity::Warn,
                    T19_EVIDENCE,
                    format!("cannot read the {} thread's page faults", self.role.name()),
                    e,
                ));
            }
        };
        self.failed = false;
        let (before, d) = (self.last, now.since(&self.last));
        self.last = now;
        if d == Faults::default() {
            return None;
        }
        Some(Finding::found(
            CheckId::T19,
            Severity::Warn,
            T19_EVIDENCE,
            format!(
                "the {} thread ({}) took {} minor and {} major page faults since the last check \
                 (minflt {} → {}, majflt {} → {}): each stops the thread while the kernel maps \
                 the page in, and a major one waits for I/O too (not measured here). Fix: touch \
                 and lock its memory before the loop (`mlockall(MCL_CURRENT | MCL_FUTURE)`, every \
                 buffer written once), and keep the kernel from moving its pages: T3 (THP), T5 \
                 (NUMA balancing), T14 (compaction), T17 (KSM)",
                self.role.name(),
                self.path,
                d.minor,
                d.major,
                before.minor,
                now.minor,
                before.major,
                now.major
            ),
        ))
    }
}
