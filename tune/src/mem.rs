//! T3 (transparent hugepages), T4 (the Aeron directory's filesystem) and T5
//! (NUMA balancing).

use std::fmt;
use std::path::Path;

use crate::cpuset::CpuSet;
use crate::finding::{CheckId, Component, Finding, Severity};
use crate::host::{Host, ReadError};

const THP: &str = "/sys/kernel/mm/transparent_hugepage";

const T3_EVIDENCE: &str = "Linux Documentation/admin-guide/mm/transhuge.rst (khugepaged, \
     enabled: always/madvise/never); not measured with archived data";

/// The selected value of a sysfs choice (`always [madvise] never`).
fn selected(text: &str) -> Option<&str> {
    let start = text.find('[')? + 1;
    let end = start + text[start..].find(']')?;
    Some(&text[start..end])
}

/// The THP settings that are on, as (file, value).
type ThpOn = Vec<(String, String)>;

/// The top level's value and the settings that are on: the top level when
/// not `never`, and each size at `always` or `madvise`; `None` for a kernel
/// without THP.
fn thp_on(host: &Host) -> Result<Option<(String, ThpOn)>, ReadError> {
    let enabled = format!("{THP}/enabled");
    let top = match host.read(&enabled) {
        Ok(t) => t,
        Err(ReadError::Io {
            kind: std::io::ErrorKind::NotFound,
            ..
        }) if !host.exists(THP) && host.exists("/sys/kernel/mm") => return Ok(None),
        Err(e) => return Err(e),
    };
    let choice = |path: &str, text: &str| {
        selected(text)
            .map(str::to_owned)
            .ok_or_else(|| ReadError::Malformed {
                path: path.to_owned(),
                detail: "no [selected] value",
            })
    };
    let top = choice(&enabled, &top)?;
    let mut on: ThpOn = Vec::new();
    if top != "never" {
        on.push((enabled, top.clone()));
    }
    for size in host
        .list(THP)?
        .iter()
        .filter(|n| n.starts_with("hugepages-") && n.ends_with("kB"))
    {
        let path = format!("{THP}/{size}/enabled");
        // A size only shmem can use has `shmem_enabled` and no `enabled`.
        let Some(text) = host.read_optional(&path)? else {
            continue;
        };
        let v = choice(&path, &text)?;
        if v == "always" || v == "madvise" {
            on.push((path, v));
        }
    }
    Ok(Some((top, on)))
}

/// T3: transparent hugepages are off, at every size.
///
/// Fires when `transparent_hugepage/enabled` is not `never`, or a
/// multi-size size (`hugepages-<n>kB/enabled`, Linux 6.8 and later; 16 KiB
/// to 2 MiB on arm64) is `always` or `madvise` (a size at `inherit` follows
/// the top level). A warning for the processes that run Aeron (the media
/// driver, the runtime), information for ergon-io.
#[must_use]
pub fn t3_thp(host: &Host, component: Component) -> Vec<Finding> {
    let id = CheckId::T3;
    let sev = if component == Component::Io {
        Severity::Info
    } else {
        Severity::Warn
    };
    let (top, on) = match thp_on(host) {
        Ok(Some(found)) => found,
        Ok(None) => {
            return vec![Finding::ok(
                id,
                sev,
                T3_EVIDENCE,
                "the kernel has no transparent hugepages".to_owned(),
            )];
        }
        Err(e) => {
            return vec![Finding::unknown(
                id,
                sev,
                T3_EVIDENCE,
                "cannot tell whether transparent hugepages are on".to_owned(),
                e,
            )];
        }
    };
    if on.is_empty() {
        return vec![Finding::ok(
            id,
            sev,
            T3_EVIDENCE,
            "transparent hugepages are `never`, at every size".to_owned(),
        )];
    }
    let what: Vec<String> = on
        .iter()
        .map(|(path, v)| {
            path.strip_prefix(&format!("{THP}/"))
                .and_then(|p| p.strip_suffix("/enabled"))
                .map_or_else(|| format!("`{v}`"), |size| format!("{size} `{v}`"))
        })
        .collect();
    let fixes: Vec<String> = on
        .iter()
        .map(|(p, _)| format!("echo never > {p}"))
        .collect();
    let boot = if top == "never" {
        ""
    } else {
        ", or boot with `transparent_hugepage=never`"
    };
    let madvise = if on.iter().any(|(_, v)| v == "madvise") {
        "; under `madvise` khugepaged still collapses the regions that asked for it"
    } else {
        ""
    };
    let io = if component == Component::Io {
        "; ergon-io's own benchmarks ran under `madvise`"
    } else {
        ""
    };
    vec![Finding::found(
        id,
        sev,
        T3_EVIDENCE,
        format!(
            "transparent hugepages are {}: the kernel thread khugepaged scans and collapses \
             pages in the background, and its work can run on a busy-spinning thread's CPU (not \
             measured with archived data){madvise}{io}. Fix: `{}`{boot}",
            what.join(", "),
            fixes.join("; ")
        ),
    )]
}

/// A filesystem's type, as `statfs` gives it (`f_type`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FsMagic(pub u64);

impl FsMagic {
    /// `tmpfs` (what `/dev/shm` is).
    pub const TMPFS: Self = Self(0x0102_1994);
    /// `hugetlbfs`.
    pub const HUGETLBFS: Self = Self(0x9584_58f6);
    /// `ramfs`.
    pub const RAMFS: Self = Self(0x8584_58f6);

    /// Whether the filesystem keeps its files in memory only.
    #[must_use]
    pub const fn is_memory(self) -> bool {
        self.0 == Self::TMPFS.0 || self.0 == Self::HUGETLBFS.0 || self.0 == Self::RAMFS.0
    }

    /// The filesystem's name, if known.
    #[must_use]
    pub const fn name(self) -> Option<&'static str> {
        Some(match self.0 {
            0x0102_1994 => "tmpfs",
            0x9584_58f6 => "hugetlbfs",
            0x8584_58f6 => "ramfs",
            0xEF53 => "ext4",
            0x5846_5342 => "xfs",
            0x9123_683E => "btrfs",
            0x794c_7630 => "overlayfs",
            0x6969 => "nfs",
            0xFE53_4D42 => "smb2",
            0xFF53_4D42 => "cifs",
            0x6573_5546 => "fuse",
            0x0102_1997 => "9p",
            0x2fc1_2fc1 => "zfs",
            0x7371_7368 => "squashfs",
            _ => return None,
        })
    }
}

/// The name, or the number (`0x12345678`).
impl fmt::Display for FsMagic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            Some(n) => f.write_str(n),
            None => write!(f, "filesystem {:#x}", self.0),
        }
    }
}

/// The filesystem `path` is on, or, if it does not exist yet (Aeron
/// creates its directory), its nearest existing ancestor's.
///
/// # Errors
/// [`ReadError::NotLive`] on a tree standing for a host (its `statfs` would
/// describe the machine the tree is on), [`ReadError::NotLinux`] elsewhere
/// than Linux, or `statfs` fails.
pub fn fs_magic(host: &Host, path: &Path) -> Result<FsMagic, ReadError> {
    if !host.is_live() {
        return Err(ReadError::NotLive("statfs"));
    }
    // A relative path's last ancestor is empty: the working directory.
    let existing = path
        .ancestors()
        .map(|p| {
            if p.as_os_str().is_empty() {
                Path::new(".")
            } else {
                p
            }
        })
        .find(|p| p.exists())
        .unwrap_or_else(|| Path::new("/"));
    statfs_magic(existing)
}

#[cfg(target_os = "linux")]
fn statfs_magic(path: &Path) -> Result<FsMagic, ReadError> {
    let st = rustix::fs::statfs(path).map_err(|e| ReadError::Io {
        path: path.display().to_string(),
        kind: std::io::Error::from(e).kind(),
    })?;
    // Filesystem magic numbers are 32-bit; `f_type` is a signed word.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let magic = st.f_type as u32;
    Ok(FsMagic(u64::from(magic)))
}

#[cfg(not(target_os = "linux"))]
#[allow(clippy::unnecessary_wraps)] // the Linux version can fail
const fn statfs_magic(path: &Path) -> Result<FsMagic, ReadError> {
    let _ = path;
    Err(ReadError::NotLinux("statfs filesystem types"))
}

const T4_EVIDENCE: &str = "Aeron's documentation (AERON_DIR on /dev/shm on Linux); \
     k8s/nri-latency-pods/README.md (hugepages in a pod); not measured here";

/// T4: the Aeron directory is on a memory-backed filesystem (tmpfs,
/// hugetlbfs or ramfs). The media driver checks its own directory, a
/// runtime the directory it connects to.
#[must_use]
pub fn t4_aeron_dir(host: &Host, aeron_dir: &Path) -> Finding {
    t4_from(aeron_dir, fs_magic(host, aeron_dir))
}

/// T4 from a [`fs_magic`] result already taken.
#[must_use]
pub fn t4_from(aeron_dir: &Path, fs: Result<FsMagic, ReadError>) -> Finding {
    let id = CheckId::T4;
    let sev = Severity::Warn;
    let dir = aeron_dir.display();
    match fs {
        Ok(fs) if fs.is_memory() => Finding::ok(
            id,
            sev,
            T4_EVIDENCE,
            format!("the Aeron directory {dir} is on {fs}"),
        ),
        Ok(fs) => Finding::found(
            id,
            sev,
            T4_EVIDENCE,
            format!(
                "the Aeron directory {dir} is on {fs}: its log buffers are page-cache backed and \
                 written back to disk; Aeron puts it on /dev/shm on Linux. Fix: put it on tmpfs \
                 (/dev/shm, or in a pod a memory-backed emptyDir sized for Aeron's files), or a \
                 sized 2 MiB hugetlbfs: on the node `mount -t hugetlbfs -o pagesize=2M,size=1G \
                 none /mnt/huge`, in a pod the nri-latency-pods plugin's $ERGON_HUGEPAGES_DIR; \
                 with `AERON_FILE_PAGE_SIZE=2097152`"
            ),
        ),
        Err(e) => Finding::unknown(
            id,
            sev,
            T4_EVIDENCE,
            format!("cannot tell which filesystem the Aeron directory {dir} is on"),
            e,
        ),
    }
}

/// The NUMA nodes online, or `None` for a kernel without NUMA (no
/// `/sys/devices/system/node`).
pub fn nodes_online(host: &Host) -> Result<Option<CpuSet>, ReadError> {
    if !host.exists("/sys/devices/system/node") && host.exists("/sys/devices/system") {
        return Ok(None);
    }
    host.read_cpu_list("/sys/devices/system/node/online")
        .map(Some)
}

const T5_EVIDENCE: &str = "Linux Documentation/admin-guide/sysctl/kernel.rst (numa_balancing); \
     the lab sets 0 (md-dpdk/benches/cloud/vm.sh); not measured";

/// T5: automatic NUMA balancing is off, on a host with more than one node.
#[must_use]
pub fn t5_numa_balancing(host: &Host) -> Finding {
    let id = CheckId::T5;
    let sev = Severity::Warn;
    let nodes = match nodes_online(host) {
        Ok(Some(n)) if n.len() > 1 => n,
        Ok(_) => {
            return Finding::ok(
                id,
                sev,
                T5_EVIDENCE,
                "one NUMA node: NUMA balancing does not apply".to_owned(),
            );
        }
        Err(e) => {
            return Finding::unknown(
                id,
                sev,
                T5_EVIDENCE,
                "cannot tell how many NUMA nodes the host has".to_owned(),
                e,
            );
        }
    };
    let path = "/proc/sys/kernel/numa_balancing";
    match host.read_optional(path) {
        Ok(None) => Finding::ok(
            id,
            sev,
            T5_EVIDENCE,
            "the kernel has no NUMA balancing".to_owned(),
        ),
        Ok(Some(text)) => match crate::host::first_line(&text).parse::<i64>() {
            Ok(0) => Finding::ok(id, sev, T5_EVIDENCE, "NUMA balancing is off".to_owned()),
            Ok(v) => Finding::found(
                id,
                sev,
                T5_EVIDENCE,
                format!(
                    "automatic NUMA balancing is on (kernel.numa_balancing {v}) on a host with {} \
                     NUMA nodes ({nodes}): it unmaps pages to take hinting faults, pinned threads' \
                     pages included; the kernel's documentation says to turn it off for \
                     node-bound workloads. Fix (node; kernel.* sysctls are not set from a pod): \
                     `sysctl -w kernel.numa_balancing=0`",
                    nodes.len()
                ),
            ),
            Err(_) => Finding::unknown(
                id,
                sev,
                T5_EVIDENCE,
                "cannot tell whether NUMA balancing is on".to_owned(),
                ReadError::Malformed {
                    path: path.to_owned(),
                    detail: "not a number",
                },
            ),
        },
        Err(e) => Finding::unknown(
            id,
            sev,
            T5_EVIDENCE,
            "cannot tell whether NUMA balancing is on".to_owned(),
            e,
        ),
    }
}
