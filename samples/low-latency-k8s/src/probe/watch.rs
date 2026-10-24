//! The probe's run: find the target processes, sample their threads every
//! interval, scan the node's tasks once a second, and turn it all into
//! checks ([`crate::probe::verdict`]).
//!
//! Everything reads through a [`Root`], so a fixture tree under a temporary
//! directory stands in for `/proc` and `/sys` in tests.

use std::collections::BTreeMap;

use crate::LlError;
use crate::cpus::CpuList;
use crate::probe::expect::{Expected, Plan, ThreadRole, role_of};
use crate::probe::interrupts::{IrqKind, Table, parse_interrupts, parse_softirqs};
use crate::probe::proc::{
    Root, cgroup_path, parse_environ, parse_stat, parse_status, path_has_pod,
};
use crate::probe::verdict::{Check, Contract, Outcome, ThreadSeries};

/// Which processes to watch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selector {
    /// Every process of the pod with this UID (its `pause` aside), found by
    /// the UID in `/proc/<pid>/cgroup`.
    Pod {
        /// `trading/ll-ping-x`.
        label: String,
        /// The pod's UID.
        uid: String,
    },
    /// This process and its descendants.
    Pid {
        /// A name for the output.
        label: String,
        /// The process.
        pid: u32,
    },
}

impl Selector {
    /// Parses `LABEL=UID` (`pod`) or `LABEL=PID` (`pid`).
    ///
    /// # Errors
    ///
    /// [`LlError::Usage`] without `=`, or a PID that is not a number.
    pub fn parse(kind: &str, text: &str) -> Result<Self, LlError> {
        let (label, value) = text
            .split_once('=')
            .ok_or_else(|| LlError::Usage(format!("--{kind} {text}: LABEL=VALUE")))?;
        match kind {
            "pod" => Ok(Self::Pod {
                label: label.to_owned(),
                uid: value.to_owned(),
            }),
            _ => Ok(Self::Pid {
                label: label.to_owned(),
                pid: value.parse().map_err(|_| {
                    LlError::Usage(format!("--target {text}: the PID is not a number"))
                })?,
            }),
        }
    }
}

/// How the probe runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Options {
    /// The checks that count.
    pub contract: Contract,
    /// The node's name, for the output.
    pub node: String,
    /// Whether P1 compares the container's `cpuset.cpus.effective` (off
    /// outside Kubernetes, where every process shares one cgroup).
    pub cpuset_check: bool,
    /// F3's limit on the per-CPU timer, interrupts a second.
    pub tick_limit: f64,
}

/// One watched process.
#[derive(Clone, Debug)]
struct Proc {
    pid: u32,
    plan: Result<Plan, String>,
    cgroup: Option<String>,
}

/// One target's processes.
#[derive(Clone, Debug)]
struct Target {
    label: String,
    procs: Vec<Proc>,
}

/// What `/proc/interrupts` and friends held at the start.
#[derive(Clone, Debug)]
struct Snapshot {
    interrupts: Table,
    softirqs: Table,
}

/// A hot thread's (voluntary, involuntary) switches: first and last seen.
type Switches = ((u64, u64), (u64, u64));

/// The run.
#[derive(Debug)]
pub struct Watch {
    root: Root,
    opts: Options,
    targets: Vec<Target>,
    series: BTreeMap<(String, u32), ThreadSeries>,
    ctxt: BTreeMap<(String, u32), Switches>,
    occupants: BTreeMap<usize, u32>,
    others: BTreeMap<usize, (u64, u64, Vec<String>)>,
    scans: u64,
    rounds: u64,
    start: Snapshot,
    quota_start: BTreeMap<String, u64>,
}

fn label_of(scope: &str) -> String {
    scope.to_owned()
}

impl Watch {
    /// Finds the targets' processes and takes the start snapshot.
    ///
    /// # Errors
    ///
    /// [`LlError::Io`] when `/proc` cannot be listed or the interrupt
    /// tables read; [`LlError::Usage`] when a selector finds no process.
    pub fn start(root: Root, opts: Options, selectors: &[Selector]) -> Result<Self, LlError> {
        let pids = root.numeric_entries("proc")?;
        let mut targets = Vec::new();
        for sel in selectors {
            let (label, found) = match sel {
                Selector::Pod { label, uid } => (label.clone(), pod_pids(&root, &pids, uid)),
                Selector::Pid { label, pid } => (label.clone(), descendants(&root, &pids, *pid)),
            };
            if found.is_empty() {
                return Err(LlError::Usage(format!("{label}: no process found")));
            }
            let procs = found
                .into_iter()
                .map(|pid| {
                    let plan = root
                        .read_bytes(&format!("proc/{pid}/environ"))
                        .map(|b| parse_environ(&b))
                        .and_then(|env| Plan::from_env(&env))
                        .map_err(|e| e.to_string());
                    let cgroup = root
                        .read(&format!("proc/{pid}/cgroup"))
                        .ok()
                        .and_then(|t| cgroup_path(&t));
                    Proc { pid, plan, cgroup }
                })
                .collect();
            targets.push(Target { label, procs });
        }
        let mut occupants = BTreeMap::new();
        for t in &targets {
            for p in &t.procs {
                if let Ok(plan) = &p.plan {
                    for cpu in plan.hot_cpus().iter() {
                        occupants.entry(cpu).or_insert(0);
                    }
                }
            }
        }
        let start = snapshot(&root)?;
        let mut quota_start = BTreeMap::new();
        for t in &targets {
            for p in &t.procs {
                if let Some(cg) = &p.cgroup
                    && let Some(n) = throttled(&root, cg)
                {
                    quota_start.insert(cg.clone(), n);
                }
            }
        }
        Ok(Self {
            root,
            opts,
            targets,
            series: BTreeMap::new(),
            ctxt: BTreeMap::new(),
            occupants,
            others: BTreeMap::new(),
            scans: 0,
            rounds: 0,
            start,
            quota_start,
        })
    }

    /// Sampling rounds taken.
    #[must_use]
    pub const fn rounds(&self) -> u64 {
        self.rounds
    }

    /// One round: every thread of every target, its mask and its CPU.
    pub fn sample(&mut self) {
        self.rounds += 1;
        for t in &self.targets {
            let scope = format!("pod={}", t.label);
            for p in &t.procs {
                let Ok(tids) = self.root.numeric_entries(&format!("proc/{}/task", p.pid)) else {
                    continue; // the process is gone: its threads stop sampling
                };
                for tid in tids {
                    let base = format!("proc/{}/task/{tid}", p.pid);
                    let (Ok(stat), Ok(status)) = (
                        self.root.read(&format!("{base}/stat")),
                        self.root.read(&format!("{base}/status")),
                    ) else {
                        continue;
                    };
                    let (Ok(stat), Ok(status)) = (parse_stat(&stat), parse_status(&status)) else {
                        continue;
                    };
                    let role = role_of(&stat.comm);
                    let key = (label_of(&scope), tid);
                    let entry = self.series.entry(key.clone()).or_insert_with(|| {
                        let expected = p
                            .plan
                            .as_ref()
                            .map_err(Clone::clone)
                            .and_then(|plan| plan.expect(role));
                        ThreadSeries::new(&scope, tid, &stat.comm, role, expected)
                    });
                    entry.sample(&status.cpus_allowed, stat.processor);
                    if role.is_hot() {
                        if let Ok(Expected::Exactly(cpu)) = &entry.expected {
                            self.occupants.insert(*cpu, tid);
                        }
                        let now = (status.voluntary, status.nonvoluntary);
                        self.ctxt
                            .entry(key)
                            .and_modify(|(_, last)| *last = now)
                            .or_insert((now, now));
                    }
                }
            }
        }
    }

    /// One scan of every task on the node: which ran on an exclusive CPU
    /// that is not its occupant (F1).
    pub fn scan_node(&mut self) {
        self.scans += 1;
        let Ok(pids) = self.root.numeric_entries("proc") else {
            return;
        };
        for pid in pids {
            let Ok(tids) = self.root.numeric_entries(&format!("proc/{pid}/task")) else {
                continue;
            };
            for tid in tids {
                let Ok(stat) = self.root.read(&format!("proc/{pid}/task/{tid}/stat")) else {
                    continue;
                };
                let Ok(stat) = parse_stat(&stat) else {
                    continue;
                };
                let Some(&occupant) = self.occupants.get(&stat.processor) else {
                    continue;
                };
                if tid == occupant {
                    continue;
                }
                let e = self
                    .others
                    .entry(stat.processor)
                    .or_insert((0, 0, Vec::new()));
                if stat.is_kthread() {
                    e.1 += 1;
                } else {
                    e.0 += 1;
                    let name = format!("{}/{tid}", stat.comm);
                    if e.2.len() < 5 && !e.2.contains(&name) {
                        e.2.push(name);
                    }
                }
            }
        }
    }

    /// The checks, after the window of `window_s` seconds.
    ///
    /// # Errors
    ///
    /// [`LlError::Io`] when the end snapshot cannot be read.
    pub fn finish(&self, window_s: f64) -> Result<Vec<Check>, LlError> {
        let c = self.opts.contract;
        let mut checks = Vec::new();
        let end = snapshot(&self.root)?;
        for t in &self.targets {
            let scope = format!("pod={}", t.label);
            self.process_checks(t, &scope, &mut checks);
        }
        for s in self.series.values() {
            checks.push(s.affinity_check(c));
            checks.push(s.ran_on_check(c));
        }
        let node = format!("node={}", self.opts.node);
        let exclusive: CpuList = self.occupants.keys().copied().collect();
        self.node_checks(&node, &exclusive, &end, window_s, &mut checks);
        for ((scope, tid), ((v0, n0), (v1, n1))) in &self.ctxt {
            let comm = self
                .series
                .get(&(scope.clone(), *tid))
                .map_or("?", |s| s.comm.as_str());
            let (dv, dn) = (v1.saturating_sub(*v0), n1.saturating_sub(*n0));
            checks.push(Check::judged(
                c,
                scope,
                "F4.ctxt_switches",
                &format!("{comm}/{tid}"),
                dv == 0 && dn == 0,
                format!("voluntary={dv} nonvoluntary={dn}"),
            ));
        }
        Ok(checks)
    }

    #[allow(clippy::too_many_lines)] // the P1, P2, P3.cores and F5 rules in table order
    fn process_checks(&self, t: &Target, scope: &str, checks: &mut Vec<Check>) {
        let c = self.opts.contract;
        let mut wants_hot = false;
        let mut wants_driver = false;
        for p in &t.procs {
            let subject = format!("pid/{}", p.pid);
            match &p.plan {
                Err(why) => checks.push(Check::judged(
                    c,
                    scope,
                    "P1.env",
                    &subject,
                    false,
                    why.clone(),
                )),
                Ok(plan) => {
                    wants_hot |= plan.hot.is_some();
                    wants_driver |= plan.sender.is_some() || plan.receiver.is_some();
                    checks.push(Check::judged(
                        c,
                        scope,
                        "P1.env",
                        &subject,
                        true,
                        format!(
                            "isolated={} shared={} helpers={}",
                            plan.env.isolated, plan.env.shared, plan.env.helpers
                        ),
                    ));
                    let want = plan.env.isolated.union(&plan.env.shared);
                    let effective = p
                        .cgroup
                        .as_ref()
                        .and_then(|cg| {
                            self.root
                                .read(&format!("sys/fs/cgroup{cg}/cpuset.cpus.effective"))
                                .ok()
                        })
                        .and_then(|t| CpuList::parse(&t).ok());
                    let held = effective.as_ref() == Some(&want);
                    let detail = format!(
                        "effective={} expected={want}",
                        effective.map_or_else(|| "unreadable".to_owned(), |e| e.to_string())
                    );
                    let mut check = Check::judged(c, scope, "P1.cpuset", &subject, held, detail);
                    if !self.opts.cpuset_check {
                        check.outcome = Outcome::NotInContract;
                    }
                    checks.push(check);
                    if let (Some(s), Some(r)) = (plan.sender, plan.receiver) {
                        let siblings = self
                            .root
                            .read(&format!(
                                "sys/devices/system/cpu/cpu{s}/topology/thread_siblings_list"
                            ))
                            .ok()
                            .and_then(|t| CpuList::parse(&t).ok())
                            .unwrap_or_else(|| CpuList::one(s));
                        checks.push(Check::judged(
                            c,
                            scope,
                            "P3.cores",
                            "sender,receiver",
                            s != r && !siblings.contains(r),
                            format!("sender={s} receiver={r} sender_core={siblings}"),
                        ));
                    }
                    if let Some(cg) = &p.cgroup {
                        let max = self.root.read(&format!("sys/fs/cgroup{cg}/cpu.max")).ok();
                        let max = max.as_deref().map_or("unreadable", str::trim);
                        let throttled = throttled(&self.root, cg)
                            .zip(self.quota_start.get(cg))
                            .map(|(now, then)| now.saturating_sub(*then));
                        checks.push(Check::judged(
                            c,
                            scope,
                            "F5.cpu_quota",
                            &subject,
                            max.starts_with("max") && throttled == Some(0),
                            format!(
                                "cpu.max={max} nr_throttled_delta={}",
                                throttled
                                    .map_or_else(|| "unreadable".to_owned(), |n| n.to_string())
                            ),
                        ));
                    }
                }
            }
        }
        let present = |role: ThreadRole| {
            self.series
                .values()
                .any(|s| s.scope == scope && s.role == role)
        };
        if wants_hot {
            checks.push(Check::judged(
                c,
                scope,
                "P2.present",
                "ll-hot",
                present(ThreadRole::Hot),
                "role=app".into(),
            ));
        }
        if wants_driver {
            for (role, name) in [
                (ThreadRole::Sender, "sender"),
                (ThreadRole::Receiver, "receiver"),
            ] {
                checks.push(Check::judged(
                    c,
                    scope,
                    "P2.present",
                    name,
                    present(role),
                    "role=driver".into(),
                ));
            }
        }
        if !wants_hot && !wants_driver {
            checks.push(Check::judged(
                c,
                scope,
                "P2.present",
                "-",
                false,
                "no process plans a hot thread".into(),
            ));
        }
    }

    #[allow(clippy::too_many_lines)] // the F0 to F3 rules in table order
    fn node_checks(
        &self,
        node: &str,
        exclusive: &CpuList,
        end: &Snapshot,
        window_s: f64,
        checks: &mut Vec<Check>,
    ) {
        let c = self.opts.contract;
        for (file, id) in [("isolated", "F0.isolated"), ("nohz_full", "F0.nohz_full")] {
            let list = self
                .root
                .read(&format!("sys/devices/system/cpu/{file}"))
                .ok()
                .and_then(|t| CpuList::parse(&t).ok());
            let held = list
                .as_ref()
                .is_some_and(|l| !exclusive.is_empty() && exclusive.is_subset(l));
            checks.push(Check::judged(
                c,
                node,
                id,
                "-",
                held,
                format!(
                    "{file}={} exclusive={exclusive}",
                    list.map_or_else(|| "unreadable".to_owned(), |l| l.to_string())
                ),
            ));
        }
        for cpu in exclusive.iter() {
            let subject = format!("cpu={cpu}");
            let (others, kthreads, names) = self.others.get(&cpu).cloned().unwrap_or_default();
            checks.push(Check::judged(
                c,
                node,
                "F1.other_tasks",
                &subject,
                self.scans > 0 && others == 0,
                format!(
                    "others={others} kthreads={kthreads} scans={} seen={}",
                    self.scans,
                    if names.is_empty() {
                        "-".to_owned()
                    } else {
                        names.join(",")
                    }
                ),
            ));
            let deltas = end.interrupts.deltas(&self.start.interrupts, cpu);
            let device: Vec<String> = deltas
                .iter()
                .filter(|(_, k, d)| *k == IrqKind::Device && *d > 0)
                .map(|(n, _, d)| format!("{n}:{d}"))
                .collect();
            let device_total: u64 = deltas
                .iter()
                .filter(|(_, k, _)| *k == IrqKind::Device)
                .map(|(_, _, d)| d)
                .sum();
            checks.push(Check::judged(
                c,
                node,
                "F2.device_irqs",
                &subject,
                device_total == 0 && !deltas.is_empty(),
                format!(
                    "delta={device_total} rows={}",
                    if device.is_empty() {
                        "-".to_owned()
                    } else {
                        device.join(",")
                    }
                ),
            ));
            let timer: Vec<(String, u64)> = deltas
                .iter()
                .filter(|(_, k, _)| *k == IrqKind::Timer)
                .map(|(n, _, d)| (n.clone(), *d))
                .collect();
            let timer_total: u64 = timer.iter().map(|(_, d)| d).sum();
            #[allow(clippy::cast_precision_loss)] // interrupt counts are far below 2^52
            let per_s = timer_total as f64 / window_s.max(0.001);
            checks.push(Check::judged(
                c,
                node,
                "F3.timer",
                &subject,
                !timer.is_empty() && per_s <= self.opts.tick_limit,
                format!(
                    "rows={} delta={timer_total} per_s={per_s:.2} limit={:.2}",
                    timer
                        .iter()
                        .map(|(n, d)| format!("{n}:{d}"))
                        .collect::<Vec<_>>()
                        .join(","),
                    self.opts.tick_limit
                ),
            ));
            let ipis: Vec<String> = deltas
                .iter()
                .filter(|(_, k, d)| *k == IrqKind::Ipi && *d > 0)
                .map(|(n, _, d)| format!("{n}:{d}"))
                .collect();
            let soft: Vec<String> = end
                .softirqs
                .deltas(&self.start.softirqs, cpu)
                .iter()
                .filter(|(_, _, d)| *d > 0)
                .map(|(n, _, d)| format!("{n}:{d}"))
                .collect();
            for (id, rows) in [("F3.ipis", ipis), ("F3.softirqs", soft)] {
                checks.push(Check {
                    scope: node.to_owned(),
                    id: id.to_owned(),
                    subject: subject.clone(),
                    outcome: Outcome::Info,
                    detail: if rows.is_empty() {
                        "none".to_owned()
                    } else {
                        rows.join(",")
                    },
                });
            }
        }
    }
}

fn snapshot(root: &Root) -> Result<Snapshot, LlError> {
    Ok(Snapshot {
        interrupts: parse_interrupts(&root.read("proc/interrupts")?)?,
        softirqs: parse_softirqs(&root.read("proc/softirqs")?)?,
    })
}

fn throttled(root: &Root, cgroup: &str) -> Option<u64> {
    let text = root.read(&format!("sys/fs/cgroup{cgroup}/cpu.stat")).ok()?;
    text.lines()
        .find_map(|l| l.strip_prefix("nr_throttled "))
        .and_then(|v| v.trim().parse().ok())
}

fn pod_pids(root: &Root, pids: &[u32], uid: &str) -> Vec<u32> {
    pids.iter()
        .copied()
        .filter(|pid| {
            root.read(&format!("proc/{pid}/cgroup"))
                .ok()
                .and_then(|t| cgroup_path(&t))
                .is_some_and(|p| path_has_pod(&p, uid))
        })
        .filter(|pid| {
            root.read(&format!("proc/{pid}/comm"))
                .map_or(true, |c| c.trim_end() != "pause")
        })
        .collect()
}

fn descendants(root: &Root, pids: &[u32], top: u32) -> Vec<u32> {
    let parents: BTreeMap<u32, u32> = pids
        .iter()
        .filter_map(|&pid| {
            let stat = parse_stat(&root.read(&format!("proc/{pid}/stat")).ok()?).ok()?;
            Some((pid, stat.ppid))
        })
        .collect();
    let mut out: Vec<u32> = Vec::new();
    if parents.contains_key(&top) {
        out.push(top);
    }
    let mut grew = true;
    while grew {
        grew = false;
        for (&pid, &ppid) in &parents {
            if out.contains(&ppid) && !out.contains(&pid) {
                out.push(pid);
                grew = true;
            }
        }
    }
    out.sort_unstable();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::probe::verdict::result;
    use std::path::Path;

    struct Tree(std::path::PathBuf);

    impl Tree {
        fn new(name: &str) -> Result<Self, LlError> {
            let dir = std::env::temp_dir().join(format!("ll-watch-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).map_err(|e| LlError::io("mkdir", e))?;
            Ok(Self(dir))
        }

        fn put(&self, rel: &str, text: &str) -> Result<(), LlError> {
            let p = self.0.join(rel);
            if let Some(parent) = Path::new(&p).parent() {
                std::fs::create_dir_all(parent).map_err(|e| LlError::io("mkdir", e))?;
            }
            std::fs::write(&p, text).map_err(|e| LlError::io("write", e))
        }

        fn task(
            &self,
            pid: u32,
            tid: u32,
            comm: &str,
            mask: &str,
            cpu: usize,
            ctxt: (u64, u64),
        ) -> Result<(), LlError> {
            // fields 3 to 52; field n is fields[n - 3]
            let mut fields = vec!["0".to_owned(); 50];
            fields[0] = "S".to_owned();
            fields[1] = "1".to_owned();
            fields[6] = "4194368".to_owned();
            fields[36] = cpu.to_string();
            let stat = format!("{tid} ({comm}) {}\n", fields.join(" "));
            let status = format!(
                "Name:\t{comm}\nCpus_allowed_list:\t{mask}\nvoluntary_ctxt_switches:\t{}\nnonvoluntary_ctxt_switches:\t{}\n",
                ctxt.0, ctxt.1
            );
            self.put(&format!("proc/{pid}/task/{tid}/stat"), &stat)?;
            self.put(&format!("proc/{pid}/task/{tid}/status"), &status)?;
            if pid == tid {
                self.put(&format!("proc/{pid}/stat"), &stat)?;
                self.put(&format!("proc/{pid}/comm"), &format!("{comm}\n"))?;
            }
            Ok(())
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    const UID: &str = "1a2b3c4d-0000-1111-2222-333344445555";

    fn node(t: &Tree, hot_mask_of_conductor: &str) -> Result<(), LlError> {
        let cg = "/kubepods.slice/kubepods-pod1a2b3c4d_0000_1111_2222_333344445555.slice/cri-containerd-x.scope";
        t.put(
            "proc/100/environ",
            "ERGON_ISOLATED_CPUS=3\0ERGON_SHARED_CPUS=1\0LL_ROLE=ping\0",
        )?;
        t.put("proc/100/cgroup", &format!("0::{cg}\n"))?;
        t.task(100, 100, "ll-app", "1", 1, (0, 0))?;
        t.task(100, 101, "ll-log", "1", 1, (5, 0))?;
        t.task(100, 102, "ll-hot", "3", 3, (2, 0))?;
        t.task(100, 103, "aeron-client", hot_mask_of_conductor, 1, (9, 1))?;
        // the pod's pause process is not a target
        t.put("proc/90/environ", "")?;
        t.put("proc/90/cgroup", &format!("0::{cg}\n"))?;
        t.task(90, 90, "pause", "1", 1, (0, 0))?;
        // a stranger off the exclusive CPU
        t.task(200, 200, "sshd", "0-1", 0, (0, 0))?;
        t.put(&format!("sys/fs/cgroup{cg}/cpuset.cpus.effective"), "1,3\n")?;
        t.put(&format!("sys/fs/cgroup{cg}/cpu.max"), "max 100000\n")?;
        t.put(
            &format!("sys/fs/cgroup{cg}/cpu.stat"),
            "usage_usec 1\nnr_throttled 0\n",
        )?;
        t.put("sys/devices/system/cpu/isolated", "2-7\n")?;
        t.put("sys/devices/system/cpu/nohz_full", "2-7\n")?;
        t.put(
            "proc/interrupts",
            "           CPU0       CPU1       CPU2       CPU3\n 24:  10  0  0  0  PCI-MSI eth0\nLOC: 100 100 100 100  Local timer interrupts\n",
        )?;
        t.put(
            "proc/softirqs",
            "            CPU0 CPU1 CPU2 CPU3\n  TIMER:  1 1 1 1\n",
        )?;
        Ok(())
    }

    fn opts(contract: Contract) -> Options {
        Options {
            contract,
            node: "n1".into(),
            cpuset_check: true,
            tick_limit: 2.0,
        }
    }

    fn run(t: &Tree, contract: Contract) -> Result<Vec<Check>, LlError> {
        let sel = [Selector::Pod {
            label: "trading/ll-ping".into(),
            uid: UID.into(),
        }];
        let mut w = Watch::start(Root(t.0.clone()), opts(contract), &sel)?;
        for _ in 0..3 {
            w.sample();
            w.scan_node();
        }
        // one more timer tick on CPU 3 over a 1 s window: inside the limit
        t.put(
            "proc/interrupts",
            "           CPU0       CPU1       CPU2       CPU3\n 24:  20  0  0  0  PCI-MSI eth0\nLOC: 200 200 100 101  Local timer interrupts\n",
        )?;
        w.finish(1.0)
    }

    #[test]
    fn a_good_node_passes_the_full_contract() -> Result<(), LlError> {
        let t = Tree::new("good")?;
        node(&t, "1")?;
        let checks = run(&t, Contract::Full)?;
        let failed: Vec<&Check> = checks
            .iter()
            .filter(|c| c.outcome == Outcome::Fail)
            .collect();
        assert!(failed.is_empty(), "{failed:#?}");
        let (outcome, counted, _) = result(&checks);
        assert_eq!(outcome, Outcome::Pass);
        assert!(counted >= 14, "{checks:#?}");
        assert!(
            checks
                .iter()
                .any(|c| c.id == "F3.timer" && c.detail.contains("LOC:1")),
            "{checks:#?}"
        );
        assert!(!checks.iter().any(|c| c.subject.starts_with("pause")));
        Ok(())
    }

    #[test]
    fn a_conductor_on_the_hot_cpu_fails() -> Result<(), LlError> {
        let t = Tree::new("bad")?;
        node(&t, "1,3")?;
        let checks = run(&t, Contract::Placement)?;
        let bad: Vec<&Check> = checks
            .iter()
            .filter(|c| c.outcome == Outcome::Fail)
            .collect();
        assert_eq!(bad.len(), 1, "{bad:#?}");
        assert_eq!(
            (bad[0].id.as_str(), bad[0].subject.as_str()),
            ("P5.affinity", "aeron-client/103")
        );
        // the F checks are printed, not counted
        assert!(
            checks
                .iter()
                .any(|c| c.id == "F2.device_irqs" && c.outcome == Outcome::NotInContract)
        );
        Ok(())
    }

    #[test]
    fn a_stranger_on_the_hot_cpu_fails_f1() -> Result<(), LlError> {
        let t = Tree::new("stranger")?;
        node(&t, "1")?;
        t.task(300, 300, "noisy", "0-7", 3, (0, 0))?;
        let checks = run(&t, Contract::Full)?;
        let f1 = checks
            .iter()
            .find(|c| c.id == "F1.other_tasks")
            .map(|c| (c.outcome, c.detail.clone()));
        assert_eq!(f1.as_ref().map(|f| f.0), Some(Outcome::Fail), "{f1:?}");
        assert!(f1.is_some_and(|f| f.1.contains("noisy/300")));
        Ok(())
    }

    #[test]
    fn a_missing_target_is_an_error() -> Result<(), LlError> {
        let t = Tree::new("missing")?;
        node(&t, "1")?;
        let sel = [Selector::Pid {
            label: "x".into(),
            pid: 4_000_000,
        }];
        assert!(Watch::start(Root(t.0.clone()), opts(Contract::Placement), &sel).is_err());
        let sel = [Selector::Pid {
            label: "app".into(),
            pid: 100,
        }];
        let w = Watch::start(Root(t.0.clone()), opts(Contract::Placement), &sel)?;
        assert_eq!(w.targets[0].procs.len(), 1);
        Ok(())
    }
}
