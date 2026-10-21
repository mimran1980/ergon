//! Where the checks read from: a root directory standing for `/`, so tests
//! (and a captured host) run against fixture trees of `/sys` and `/proc` on
//! any OS.

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use crate::cpuset::{CpuSet, CpuSetError};

/// The host the checks read: its `/sys` and `/proc` under a root.
///
/// [`Host::system`] is the running host (`/`). [`Host::at`] is a tree that
/// stands for one: every absolute path a check reads is taken under it.
/// Kernel queries ([`fs_magic`](crate::fs_magic) and, with `nic`, the
/// ethtool requests) only go to the running host: on a fixture they report
/// unknown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Host {
    root: PathBuf,
    live: bool,
}

impl Default for Host {
    fn default() -> Self {
        Self::system()
    }
}

impl Host {
    /// The running host, read at `/`. On a system other than Linux every
    /// reader reports [`ReadError::NotLinux`].
    #[must_use]
    pub fn system() -> Self {
        Self {
            root: PathBuf::from("/"),
            live: true,
        }
    }

    /// A tree standing for a host's `/` (fixtures, a captured host).
    /// `Host::at("/")` is [`Host::system`].
    #[must_use]
    pub fn at(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let live = root == Path::new("/");
        Self { root, live }
    }

    /// The root the host is read under.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Whether this is the running host (root `/`), the only one kernel
    /// queries go to.
    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.live
    }

    /// `abs` (an absolute path on the host) under the root.
    #[must_use]
    pub fn path(&self, abs: &str) -> PathBuf {
        self.root.join(abs.trim_start_matches('/'))
    }

    fn not_linux(&self) -> Option<ReadError> {
        (self.live && !cfg!(target_os = "linux"))
            .then_some(ReadError::NotLinux("the host's /sys and /proc"))
    }

    /// The text of the file at `abs`.
    pub(crate) fn read(&self, abs: &str) -> Result<String, ReadError> {
        if let Some(e) = self.not_linux() {
            return Err(e);
        }
        std::fs::read_to_string(self.path(abs)).map_err(|e| ReadError::io(abs, &e))
    }

    /// The file at `abs`, open for reading (to read again later, with no
    /// further open).
    pub(crate) fn open(&self, abs: &str) -> Result<std::fs::File, ReadError> {
        if let Some(e) = self.not_linux() {
            return Err(e);
        }
        std::fs::File::open(self.path(abs)).map_err(|e| ReadError::io(abs, &e))
    }

    /// The text of the file at `abs`, or `None` when it does not exist but
    /// its directory does (a kernel without the feature). A missing
    /// directory is an error: the tree is not there to read.
    pub(crate) fn read_optional(&self, abs: &str) -> Result<Option<String>, ReadError> {
        match self.read(abs) {
            Ok(s) => Ok(Some(s)),
            Err(ReadError::Io {
                kind: io::ErrorKind::NotFound,
                ..
            }) => {
                let parent = Path::new(abs)
                    .parent()
                    .map_or("/", |p| p.to_str().unwrap_or("/"));
                if self.path(parent).is_dir() {
                    Ok(None)
                } else {
                    Err(ReadError::Io {
                        path: parent.to_owned(),
                        kind: io::ErrorKind::NotFound,
                    })
                }
            }
            Err(e) => Err(e),
        }
    }

    /// The first line of the file at `abs`, trimmed.
    pub(crate) fn read_line(&self, abs: &str) -> Result<String, ReadError> {
        Ok(first_line(&self.read(abs)?))
    }

    /// A CPU list file (`0-3,8`).
    pub(crate) fn read_cpu_list(&self, abs: &str) -> Result<CpuSet, ReadError> {
        let text = self.read(abs)?;
        CpuSet::parse_list(&text).map_err(|e| ReadError::cpus(abs, e))
    }

    /// A number file (`123`, or `-1`).
    pub(crate) fn read_i64(&self, abs: &str) -> Result<i64, ReadError> {
        let line = self.read_line(abs)?;
        line.parse().map_err(|_| ReadError::Malformed {
            path: abs.to_owned(),
            detail: "not a number",
        })
    }

    /// The last component of the symbolic link at `abs`'s target (a
    /// device's driver, its subsystem, a PCI address), or `None` when there
    /// is no link there.
    pub(crate) fn link_name(&self, abs: &str) -> Result<Option<String>, ReadError> {
        if let Some(e) = self.not_linux() {
            return Err(e);
        }
        match std::fs::read_link(self.path(abs)) {
            Ok(target) => Ok(target
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_owned)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ReadError::io(abs, &e)),
        }
    }

    /// Whether `abs` exists (following links).
    pub(crate) fn exists(&self, abs: &str) -> bool {
        self.not_linux().is_none() && self.path(abs).exists()
    }

    /// Whether `abs` is a directory (following links).
    pub(crate) fn is_dir(&self, abs: &str) -> bool {
        self.not_linux().is_none() && self.path(abs).is_dir()
    }

    /// The names in directory `abs`, sorted.
    pub(crate) fn list(&self, abs: &str) -> Result<Vec<String>, ReadError> {
        if let Some(e) = self.not_linux() {
            return Err(e);
        }
        let entries = std::fs::read_dir(self.path(abs)).map_err(|e| ReadError::io(abs, &e))?;
        let mut names = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| ReadError::io(abs, &e))?;
            if let Some(n) = entry.file_name().to_str() {
                names.push(n.to_owned());
            }
        }
        names.sort();
        Ok(names)
    }
}

/// The first line of `text`, trimmed.
pub fn first_line(text: &str) -> String {
    text.lines().next().unwrap_or_default().trim().to_owned()
}

/// Why something on the host could not be read. A check that meets one
/// reports [`Status::Unknown`](crate::Status::Unknown) with it as the
/// reason, never a warning.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReadError {
    /// The file or directory could not be read.
    Io {
        /// The path on the host.
        path: String,
        /// What went wrong.
        kind: io::ErrorKind,
    },
    /// Its contents did not parse.
    Malformed {
        /// The path on the host.
        path: String,
        /// What was wrong with it.
        detail: &'static str,
    },
    /// Only Linux has this (what is named).
    NotLinux(&'static str),
    /// Only the running host can answer this (what is named): the host is
    /// a tree standing for one ([`Host::at`]).
    NotLive(&'static str),
    /// A network interface whose NIC cannot be seen from here (a pod's
    /// veth, a `VMBus` device with no VF).
    NoDevice {
        /// The interface.
        interface: String,
        /// What is missing.
        detail: &'static str,
    },
}

impl ReadError {
    pub(crate) fn io(abs: &str, e: &io::Error) -> Self {
        Self::Io {
            path: abs.to_owned(),
            kind: e.kind(),
        }
    }

    pub(crate) fn cpus(abs: &str, e: CpuSetError) -> Self {
        Self::Malformed {
            path: abs.to_owned(),
            detail: match e {
                CpuSetError::Reversed => "a CPU range that ends before it starts",
                CpuSetError::TooLarge => "a CPU number beyond any kernel's limit",
                _ => "not a CPU list or mask",
            },
        }
    }
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, kind } => {
                let what = match kind {
                    io::ErrorKind::NotFound => "not present".to_owned(),
                    io::ErrorKind::PermissionDenied => "permission denied".to_owned(),
                    other => other.to_string(),
                };
                write!(f, "{path}: {what}")
            }
            Self::Malformed { path, detail } => write!(f, "{path}: {detail}"),
            Self::NotLinux(what) => write!(f, "{what}: only Linux has them"),
            Self::NotLive(what) => write!(f, "{what}: only the running host answers"),
            Self::NoDevice { interface, detail } => write!(f, "{interface}: {detail}"),
        }
    }
}

impl std::error::Error for ReadError {}
