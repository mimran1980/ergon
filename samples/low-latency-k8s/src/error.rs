//! The sample's one error type.

use std::fmt;
use std::io;

/// Everything the sample can fail with. Each variant names what to fix.
#[derive(Debug)]
#[non_exhaustive]
pub enum LlError {
    /// A required environment variable is unset (the plugin did not run, or
    /// the manifest lacks it).
    MissingEnv(&'static str),
    /// An environment variable or argument holds a value that does not parse.
    BadValue {
        /// The variable or argument.
        name: String,
        /// Its value.
        value: String,
        /// Why it was refused.
        why: String,
    },
    /// A kernel CPU list (`0-3,7`) that does not parse.
    CpuList {
        /// The text.
        text: String,
        /// Why it was refused.
        why: &'static str,
    },
    /// `isolated:<i>` or `shared:<i>` past the end of its list.
    NoSuchIndex {
        /// The spec.
        spec: String,
        /// How many CPUs the list has.
        len: usize,
    },
    /// A file or system call failed.
    Io {
        /// What was being done.
        what: String,
        /// The error.
        source: io::Error,
    },
    /// The thread is not where it was pinned: `sched_getcpu()` after the pin.
    NotOnCpu {
        /// The CPU it was pinned to.
        expected: usize,
        /// The CPU it runs on.
        got: usize,
    },
    /// An Aeron call failed.
    Aeron(String),
    /// Something did not happen in time (driver, connection, echo).
    Timeout(String),
    /// The logging setup failed.
    Logging(String),
    /// The in-app self-check failed: this many threads are misplaced.
    SelfCheck(usize),
    /// A thread could not be spawned or joined.
    Thread(String),
    /// A command line that does not parse.
    Usage(String),
    /// Not available on this operating system (pinning and `/proc` are
    /// Linux only).
    Unsupported(&'static str),
}

impl LlError {
    /// An [`LlError::Io`] for `what`.
    pub fn io(what: impl Into<String>, source: io::Error) -> Self {
        Self::Io {
            what: what.into(),
            source,
        }
    }

    /// An [`LlError::BadValue`].
    pub fn bad(name: impl Into<String>, value: impl Into<String>, why: impl Into<String>) -> Self {
        Self::BadValue {
            name: name.into(),
            value: value.into(),
            why: why.into(),
        }
    }
}

impl fmt::Display for LlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEnv(name) => write!(
                f,
                "{name} is unset: the ergon-shared-cpus plugin sets it in a pod that requests ergon.io/shared-millicpus"
            ),
            Self::BadValue { name, value, why } => write!(f, "{name}={value:?}: {why}"),
            Self::CpuList { text, why } => write!(f, "CPU list {text:?}: {why}"),
            Self::NoSuchIndex { spec, len } => {
                write!(f, "{spec}: the list has {len} CPU(s)")
            }
            Self::Io { what, source } => write!(f, "{what}: {source}"),
            Self::NotOnCpu { expected, got } => {
                write!(f, "pinned to CPU {expected} but sched_getcpu() says {got}")
            }
            Self::Aeron(m) => write!(f, "aeron: {m}"),
            Self::Timeout(m) => write!(f, "timed out: {m}"),
            Self::Logging(m) => write!(f, "logging: {m}"),
            Self::SelfCheck(n) => write!(f, "self-check: {n} thread(s) misplaced"),
            Self::Thread(m) => write!(f, "thread: {m}"),
            Self::Usage(m) => write!(f, "usage: {m}"),
            Self::Unsupported(what) => write!(f, "{what} needs Linux"),
        }
    }
}

impl std::error::Error for LlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
