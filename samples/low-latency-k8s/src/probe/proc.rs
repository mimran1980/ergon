//! `/proc/<pid>/task/<tid>/{stat,status}`, `/proc/<pid>/environ` and
//! `/proc/<pid>/cgroup`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::LlError;
use crate::cpus::CpuList;

/// `PF_KTHREAD` in `stat`'s flags (field 9): a kernel thread.
pub const PF_KTHREAD: u64 = 0x0020_0000;

/// The fields of `stat` the probe uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskStat {
    /// Field 1.
    pub tid: u32,
    /// Field 2, without its parentheses (may hold spaces and `)`).
    pub comm: String,
    /// Field 3: `R`, `S`, `D`…
    pub state: char,
    /// Field 4: the parent process.
    pub ppid: u32,
    /// Field 9.
    pub flags: u64,
    /// Field 39: the CPU it last ran on.
    pub processor: usize,
}

impl TaskStat {
    /// Whether it is a kernel thread.
    #[must_use]
    pub const fn is_kthread(&self) -> bool {
        self.flags & PF_KTHREAD != 0
    }
}

/// Parses one `stat` line.
///
/// # Errors
///
/// [`LlError::BadValue`] when a field the probe needs is missing or does not
/// parse.
pub fn parse_stat(text: &str) -> Result<TaskStat, LlError> {
    let bad = |why: &str| LlError::bad("stat", text.trim_end(), why);
    let open = text.find(" (").ok_or_else(|| bad("no ( before comm"))?;
    let close = text.rfind(") ").ok_or_else(|| bad("no ) after comm"))?;
    if close < open {
        return Err(bad("comm parentheses out of order"));
    }
    let tid = text[..open]
        .trim()
        .parse()
        .map_err(|_| bad("pid is not a number"))?;
    let comm = text[open + 2..close].to_owned();
    // fields from 3 (state) on
    let rest: Vec<&str> = text[close + 2..].split_whitespace().collect();
    let field = |n: usize| {
        rest.get(n - 3)
            .copied()
            .ok_or_else(|| bad("too few fields"))
    };
    let state = field(3)?.chars().next().ok_or_else(|| bad("empty state"))?;
    let ppid = field(4)?.parse().map_err(|_| bad("ppid is not a number"))?;
    let flags = field(9)?
        .parse()
        .map_err(|_| bad("flags is not a number"))?;
    let processor = field(39)?
        .parse()
        .map_err(|_| bad("processor is not a number"))?;
    Ok(TaskStat {
        tid,
        comm,
        state,
        ppid,
        flags,
        processor,
    })
}

/// The fields of `status` the probe uses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskStatus {
    /// `Name:`.
    pub name: String,
    /// `Cpus_allowed_list:`.
    pub cpus_allowed: CpuList,
    /// `voluntary_ctxt_switches:`.
    pub voluntary: u64,
    /// `nonvoluntary_ctxt_switches:`.
    pub nonvoluntary: u64,
}

/// Parses `status`.
///
/// # Errors
///
/// [`LlError::BadValue`] when `Cpus_allowed_list` or a switch count is
/// missing or does not parse.
pub fn parse_status(text: &str) -> Result<TaskStatus, LlError> {
    let mut name = None;
    let mut allowed = None;
    let mut voluntary = None;
    let mut nonvoluntary = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match key {
            "Name" => name = Some(value.to_owned()),
            "Cpus_allowed_list" => allowed = Some(CpuList::parse(value)?),
            "voluntary_ctxt_switches" => voluntary = value.parse().ok(),
            "nonvoluntary_ctxt_switches" => nonvoluntary = value.parse().ok(),
            _ => {}
        }
    }
    let missing = |field: &str| LlError::bad("status", field, "missing or not a number");
    Ok(TaskStatus {
        name: name.unwrap_or_default(),
        cpus_allowed: allowed.ok_or_else(|| missing("Cpus_allowed_list"))?,
        voluntary: voluntary.ok_or_else(|| missing("voluntary_ctxt_switches"))?,
        nonvoluntary: nonvoluntary.ok_or_else(|| missing("nonvoluntary_ctxt_switches"))?,
    })
}

/// Parses `environ` (NUL-separated `KEY=value`); entries without `=` are
/// left out.
#[must_use]
pub fn parse_environ(bytes: &[u8]) -> BTreeMap<String, String> {
    bytes
        .split(|&b| b == 0)
        .filter_map(|entry| {
            let text = std::str::from_utf8(entry).ok()?;
            let (k, v) = text.split_once('=')?;
            (!k.is_empty()).then(|| (k.to_owned(), v.to_owned()))
        })
        .collect()
}

/// The cgroup v2 path in `/proc/<pid>/cgroup` (the `0::` line), else a v1
/// `cpuset` line's.
#[must_use]
pub fn cgroup_path(text: &str) -> Option<String> {
    let mut cpuset = None;
    for line in text.lines() {
        let mut parts = line.splitn(3, ':');
        let (Some(id), Some(controllers), Some(path)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        if id == "0" && controllers.is_empty() {
            return Some(path.to_owned());
        }
        if controllers.split(',').any(|c| c == "cpuset") {
            cpuset = Some(path.to_owned());
        }
    }
    cpuset
}

/// Whether a cgroup path belongs to the pod with `uid`: the cgroupfs driver
/// writes `pod<uid>` with dashes, the systemd driver
/// `kubepods-…-pod<uid_with_underscores>.slice`.
#[must_use]
pub fn path_has_pod(path: &str, uid: &str) -> bool {
    if uid.is_empty() {
        return false;
    }
    let dashed = format!("pod{uid}");
    let underscored = format!("pod{}", uid.replace('-', "_"));
    path.split('/').any(|part| {
        part == dashed
            || part.ends_with(&format!("-{underscored}.slice"))
            || part == format!("{underscored}.slice")
    })
}

/// Where the probe reads `/proc` and `/sys` (`/` live; a fixture tree in
/// tests).
#[derive(Clone, Debug)]
pub struct Root(pub PathBuf);

impl Root {
    /// The live system.
    #[must_use]
    pub fn system() -> Self {
        Self(PathBuf::from("/"))
    }

    /// `rel` (without a leading `/`) under the root.
    #[must_use]
    pub fn path(&self, rel: &str) -> PathBuf {
        self.0.join(rel.trim_start_matches('/'))
    }

    /// Reads `rel` as text.
    ///
    /// # Errors
    ///
    /// [`LlError::Io`] naming the file.
    pub fn read(&self, rel: &str) -> Result<String, LlError> {
        let p = self.path(rel);
        std::fs::read_to_string(&p).map_err(|e| LlError::io(format!("read {}", p.display()), e))
    }

    /// Reads `rel` as bytes.
    ///
    /// # Errors
    ///
    /// [`LlError::Io`] naming the file.
    pub fn read_bytes(&self, rel: &str) -> Result<Vec<u8>, LlError> {
        let p = self.path(rel);
        std::fs::read(&p).map_err(|e| LlError::io(format!("read {}", p.display()), e))
    }

    /// The numeric entries of directory `rel` (pids under `/proc`, tids
    /// under `/proc/<pid>/task`), ascending.
    ///
    /// # Errors
    ///
    /// [`LlError::Io`] when the directory cannot be listed.
    pub fn numeric_entries(&self, rel: &str) -> Result<Vec<u32>, LlError> {
        let p = self.path(rel);
        let mut out = Vec::new();
        for entry in
            std::fs::read_dir(&p).map_err(|e| LlError::io(format!("list {}", p.display()), e))?
        {
            let entry = entry.map_err(|e| LlError::io(format!("list {}", p.display()), e))?;
            if let Some(n) = entry.file_name().to_str().and_then(|s| s.parse().ok()) {
                out.push(n);
            }
        }
        out.sort_unstable();
        Ok(out)
    }

    /// Whether `rel` exists.
    #[must_use]
    pub fn exists(&self, rel: &str) -> bool {
        Path::new(&self.path(rel)).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A comm with a space and a `)`, as a thread can name itself.
    const STAT: &str = "4242 (ll hot) x) S 1 4242 4242 0 -1 4194368 120 0 0 0 5 1 0 0 20 0 7 0 \
        12345 104857600 2048 18446744073709551615 1 1 0 0 0 0 0 0 0 0 0 0 17 3 0 0 0 0 0 0 0 0 0 0 0 0 0\n";

    #[test]
    fn parses_stat_with_a_hostile_comm() -> Result<(), LlError> {
        let s = parse_stat(STAT)?;
        assert_eq!(s.tid, 4242);
        assert_eq!(s.comm, "ll hot) x");
        assert_eq!(s.state, 'S');
        assert_eq!(s.ppid, 1);
        assert_eq!(s.flags, 4_194_368);
        assert_eq!(s.processor, 3);
        assert!(!s.is_kthread());
        let k = parse_stat(
            "2 (kthreadd) S 0 0 0 0 -1 2129984 0 0 0 0 0 0 0 0 20 0 1 0 1 0 0 18446744073709551615 0 0 0 0 0 0 0 2147483647 0 0 0 0 0 1 0 0 0 0 0 0 0 0 0 0 0 0 0",
        )?;
        assert!(k.is_kthread());
        assert_eq!(k.processor, 1);
        Ok(())
    }

    #[test]
    fn refuses_a_truncated_stat() {
        assert!(parse_stat("4242 (x) S 1 2").is_err());
        assert!(parse_stat("4242 x S").is_err());
        assert!(parse_stat("").is_err());
    }

    #[test]
    fn parses_status() -> Result<(), LlError> {
        let s = parse_status(
            "Name:\taeron-client\nState:\tS (sleeping)\nCpus_allowed:\t02\nCpus_allowed_list:\t1\n\
             voluntary_ctxt_switches:\t150\nnonvoluntary_ctxt_switches:\t2\n",
        )?;
        assert_eq!(s.name, "aeron-client");
        assert_eq!(s.cpus_allowed, CpuList::one(1));
        assert_eq!((s.voluntary, s.nonvoluntary), (150, 2));
        assert!(parse_status("Name:\tx\n").is_err());
        Ok(())
    }

    #[test]
    fn parses_environ() {
        let env = parse_environ(b"ERGON_ISOLATED_CPUS=3\0ERGON_SHARED_CPUS=1\0junk\0A=b=c\0");
        assert_eq!(
            env.get("ERGON_ISOLATED_CPUS").map(String::as_str),
            Some("3")
        );
        assert_eq!(env.get("A").map(String::as_str), Some("b=c"));
        assert_eq!(env.len(), 3);
    }

    #[test]
    fn finds_the_cgroup_path_and_the_pod() {
        let v2 = "0::/kubepods.slice/kubepods-pod1a2b3c4d_0000_1111_2222_333344445555.slice/cri-containerd-abc.scope\n";
        let path = cgroup_path(v2).unwrap_or_default();
        assert!(path_has_pod(&path, "1a2b3c4d-0000-1111-2222-333344445555"));
        let fs = "0::/kubepods/pod1a2b3c4d-0000-1111-2222-333344445555/abc\n";
        assert!(path_has_pod(
            &cgroup_path(fs).unwrap_or_default(),
            "1a2b3c4d-0000-1111-2222-333344445555"
        ));
        assert!(!path_has_pod(
            &cgroup_path(fs).unwrap_or_default(),
            "ffffffff-0000-1111-2222-333344445555"
        ));
        let v1 = "12:cpuset:/kubepods/podx/abc\n11:memory:/kubepods/podx/abc\n";
        assert_eq!(cgroup_path(v1).as_deref(), Some("/kubepods/podx/abc"));
        assert_eq!(cgroup_path("").as_deref(), None);
    }

    proptest::proptest! {
        #[test]
        fn stat_parser_never_panics(text in ".{0,400}") {
            let _ = parse_stat(&text);
        }
    }
}
