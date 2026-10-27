//! T13-T18, the node's background noise on the busy-spinning CPUs, and
//! `FaultWatch` (T19), a hot thread's page faults read while a loop runs:
//! each against fixture trees that fire it, find it fine, and cannot read it.
#![allow(missing_docs)]

mod common;

use common::{Fixture, c9g_tuned, debian_default};
use ergon_tune::{
    CpuSteal, FaultWatch, Faults, HostCheckPolicy, HotSet, Role, Severity, Status, Thread,
    cpu_steal, t13_workqueue_cpumask, t14_compaction, t15_host_tasks, t16_instrumentation, t17_ksm,
    t18_steal, unknown_stops_require,
};

type R = Result<(), Box<dyn std::error::Error>>;

fn loop_on(cpu: u32) -> HotSet {
    HotSet::new().with(cpu, Role::Loop)
}

/// A `/proc/<pid>/stat` line: `flags` (field 9), `minflt` (10), `majflt`
/// (12), the rest zero. The command holds a space and a parenthesis, as
/// a thread's name may.
fn stat_line(pid: u32, comm: &str, state: char, flags: u64, minflt: u64, majflt: u64) -> String {
    let mut fields: Vec<String> = vec![state.to_string()];
    for field in 4..=52 {
        fields.push(match field {
            9 => flags.to_string(),
            10 => minflt.to_string(),
            12 => majflt.to_string(),
            _ => "0".to_owned(),
        });
    }
    format!("{pid} ({comm}) {}\n", fields.join(" "))
}

const PF_KTHREAD: u64 = 0x0020_0000;

/// A task: its `stat` and its `status` with `Cpus_allowed_list`.
fn task(f: &Fixture, pid: u32, tid: u32, name: &str, flags: u64, cpus: &str) -> R {
    let d = format!("/proc/{pid}/task/{tid}");
    f.file(
        &format!("{d}/stat"),
        &stat_line(tid, name, 'S', flags, 0, 0),
    )?;
    f.file(
        &format!("{d}/status"),
        &format!("Name:\t{name}\nState:\tS (sleeping)\nCpus_allowed_list:\t{cpus}\n"),
    )?;
    Ok(())
}

// ---------------------------------------------------------------- T13

#[test]
fn t13_warns_when_unbound_workqueues_may_run_on_a_hot_cpu() -> R {
    let f = debian_default("t13-warn")?;
    f.file("/sys/devices/virtual/workqueue/cpumask", "ff\n")?;
    let v = t13_workqueue_cpumask(&f.host(), &loop_on(3));
    assert_eq!(v.id, "T13");
    assert!(v.is_warning(), "{v}");
    for part in [
        "CPU 3 (loop)",
        "/sys/devices/virtual/workqueue/cpumask",
        // The fix writes the mask of every online CPU but the hot one.
        "echo f7 > /sys/devices/virtual/workqueue/cpumask",
        "node",
    ] {
        assert!(v.message.contains(part), "{part:?} not in {v}");
    }
    Ok(())
}

#[test]
fn t13_is_ok_off_the_hot_cpus_and_unknown_when_unreadable() -> R {
    let f = c9g_tuned("t13-ok")?;
    f.file("/sys/devices/virtual/workqueue/cpumask", "01\n")?;
    let v = t13_workqueue_cpumask(&f.host(), &loop_on(3));
    assert_eq!(v.status, Status::Ok, "{v}");
    // A mask wider than 32 CPUs, in the kernel's comma-separated words.
    f.file(
        "/sys/devices/virtual/workqueue/cpumask",
        "00000000,00000001\n",
    )?;
    assert_eq!(
        t13_workqueue_cpumask(&f.host(), &loop_on(3)).status,
        Status::Ok
    );
    // Only busy-spinning CPUs are asked.
    f.file("/sys/devices/virtual/workqueue/cpumask", "ff\n")?;
    let conductor = HotSet::new().with(3, Role::Conductor);
    assert_eq!(
        t13_workqueue_cpumask(&f.host(), &conductor).status,
        Status::Ok
    );
    f.remove("/sys/devices/virtual/workqueue/cpumask")?;
    let v = t13_workqueue_cpumask(&f.host(), &loop_on(3));
    assert!(v.is_unknown(), "{v}");
    assert!(!v.is_warning());
    f.file("/sys/devices/virtual/workqueue/cpumask", "not a mask\n")?;
    assert!(t13_workqueue_cpumask(&f.host(), &loop_on(3)).is_unknown());
    Ok(())
}

// ---------------------------------------------------------------- T14

#[test]
fn t14_informs_while_compaction_may_move_a_hot_threads_pages() -> R {
    let f = debian_default("t14-info")?;
    f.file("/proc/sys/vm/compaction_proactiveness", "20\n")?
        .file("/proc/sys/vm/compact_unevictable_allowed", "1\n")?;
    let v = t14_compaction(&f.host());
    assert_eq!(v.id, "T14");
    assert!(v.is_finding(), "{v}");
    assert_eq!(v.severity, Severity::Info);
    for part in [
        "vm.compaction_proactiveness 20",
        "vm.compact_unevictable_allowed 1",
        "sysctl -w vm.compaction_proactiveness=0 vm.compact_unevictable_allowed=0",
    ] {
        assert!(v.message.contains(part), "{part:?} not in {v}");
    }
    // One of the two is enough, and only it is named in the fix.
    f.file("/proc/sys/vm/compaction_proactiveness", "0\n")?;
    let v = t14_compaction(&f.host());
    assert!(v.is_finding(), "{v}");
    assert!(!v.message.contains("compaction_proactiveness=0"), "{v}");
    Ok(())
}

#[test]
fn t14_is_ok_when_off_or_absent_and_unknown_when_unreadable() -> R {
    let f = c9g_tuned("t14-ok")?;
    f.file("/proc/sys/vm/compaction_proactiveness", "0\n")?
        .file("/proc/sys/vm/compact_unevictable_allowed", "0\n")?;
    assert_eq!(t14_compaction(&f.host()).status, Status::Ok);
    // A kernel before 5.9 has no proactive compaction.
    f.remove("/proc/sys/vm/compaction_proactiveness")?;
    assert_eq!(t14_compaction(&f.host()).status, Status::Ok);
    f.remove("/proc/sys/vm/compact_unevictable_allowed")?;
    let v = t14_compaction(&f.host());
    assert_eq!(v.status, Status::Ok, "{v}");
    f.file("/proc/sys/vm/compact_unevictable_allowed", "yes\n")?;
    assert!(t14_compaction(&f.host()).is_unknown());
    f.remove("/proc/sys/vm")?;
    let v = t14_compaction(&f.host());
    assert!(v.is_unknown(), "{v}");
    assert!(!v.is_finding());
    Ok(())
}

// ---------------------------------------------------------------- T15

/// A host's `/proc` with this process (4242), kthreadd, a per-CPU kernel
/// thread on the hot CPU, an unbound kernel worker, and sshd pinned to the
/// housekeeping CPU.
fn tasks(name: &str) -> Result<Fixture, Box<dyn std::error::Error>> {
    let f = c9g_tuned(name)?;
    f.link("/proc/self", "4242")?;
    task(&f, 4242, 4242, "ergon-loop", 0, "3")?;
    task(&f, 4242, 4243, "ergon-helper", 0, "0-7")?;
    task(&f, 2, 2, "kthreadd", PF_KTHREAD, "0-7")?;
    task(&f, 31, 31, "ksoftirqd/3", PF_KTHREAD, "3")?;
    task(&f, 90, 90, "kworker/u16:1", PF_KTHREAD, "0-7")?;
    task(&f, 812, 812, "sshd", 0, "0")?;
    Ok(f)
}

#[test]
fn t15_warns_for_another_processs_task_allowed_on_a_hot_cpu() -> R {
    let f = tasks("t15-warn")?;
    task(&f, 700, 700, "systemd-journal", 0, "0-7")?;
    task(&f, 900, 900, "app", 0, "0")?;
    task(&f, 900, 901, "app worker", 0, "2-4")?;
    let v = t15_host_tasks(&f.host(), &loop_on(3));
    assert_eq!(v.id, "T15");
    assert!(v.is_warning(), "{v}");
    for part in [
        "2 processes",
        "systemd-journal (pid 700)",
        "app (pid 900)",
        "CPU 3 (loop)",
        "taskset",
        "CPUAffinity",
        "reservedSystemCPUs",
    ] {
        assert!(v.message.contains(part), "{part:?} not in {v}");
    }
    // Its own threads, kernel threads and tasks elsewhere are not named.
    for name in ["ergon-helper", "kworker", "ksoftirqd", "sshd"] {
        assert!(!v.message.contains(name), "{name:?} in {v}");
    }
    Ok(())
}

#[test]
fn t15_skips_zombies_and_vanished_tasks() -> R {
    let f = tasks("t15-zombie")?;
    f.file(
        "/proc/950/task/950/stat",
        &stat_line(950, "defunct", 'Z', 0, 0, 0),
    )?
    .file(
        "/proc/950/task/950/status",
        "Name:\tdefunct\nState:\tZ (zombie)\nCpus_allowed_list:\t0-7\n",
    )?;
    // Listed, then gone before its task directory is read.
    f.dir("/proc/960")?;
    let v = t15_host_tasks(&f.host(), &loop_on(3));
    assert_eq!(v.status, Status::Ok, "{v}");
    assert!(v.message.contains("CPU 3 (loop)"), "{v}");
    Ok(())
}

#[test]
fn t15_is_unknown_in_a_pid_namespace_or_when_proc_cannot_be_read() -> R {
    // A pod's own PID namespace: no kthreadd, so the host's tasks are not
    // visible, and none of those visible fires.
    let f = c9g_tuned("t15-pod")?;
    f.link("/proc/self", "7")?;
    task(&f, 1, 1, "pause", 0, "0")?;
    task(&f, 7, 7, "ergon-loop", 0, "3")?;
    let v = t15_host_tasks(&f.host(), &loop_on(3));
    assert!(v.is_unknown(), "{v}");
    assert!(
        matches!(&v.status, Status::Unknown(r) if r.contains("PID namespace")),
        "{v}"
    );
    // But one it can see still fires.
    task(&f, 9, 9, "sidecar", 0, "0-7")?;
    assert!(t15_host_tasks(&f.host(), &loop_on(3)).is_warning());
    let f = Fixture::empty("t15-noproc")?;
    let v = t15_host_tasks(&f.host(), &loop_on(3));
    assert!(v.is_unknown(), "{v}");
    // No busy-spinning CPU: nothing to check.
    let f = tasks("t15-none")?;
    task(&f, 700, 700, "systemd-journal", 0, "0-7")?;
    let v = t15_host_tasks(&f.host(), &HotSet::new().with(3, Role::Housekeeping));
    assert_eq!(v.status, Status::Ok, "{v}");
    Ok(())
}

// ---------------------------------------------------------------- T16

const KPROBES: &str = "ffffffff8110f0a0  k  do_sys_openat2+0x0    [FTRACE]\n\
                       ffffffff81234560  r  tcp_sendmsg+0x0    [DISABLED][FTRACE]\n";

const KALLSYMS_ROOT: &str = "ffffffff81000000 T _stext\n\
     ffffffffc0123000 t bpf_prog_6deef7357e7b4530_sd_fw_ingress\t[bpf]\n\
     ffffffffc0125000 t bpf_prog_6deef7357e7b4531_sd_devices\t[bpf]\n\
     ffffffffc0200000 t bpf_trampoline_6442\t[bpf]\n";

/// The same, as an unprivileged reader sees it: every address zero.
const KALLSYMS_UNPRIV: &str = "0000000000000000 T _stext\n\
     0000000000000000 t bpf_prog_6deef7357e7b4530_sd_fw_ingress\t[bpf]\n\
     0000000000000000 t bpf_prog_6deef7357e7b4531_sd_devices\t[bpf]\n";

/// Every source readable: kprobes, ftrace hooks and the BPF programs.
fn instrumented(name: &str) -> Result<Fixture, Box<dyn std::error::Error>> {
    let f = c9g_tuned(name)?;
    f.file("/sys/kernel/debug/kprobes/list", KPROBES)?
        .file("/sys/kernel/debug/kprobes/enabled", "1\n")?
        .file(
            "/sys/kernel/tracing/enabled_functions",
            "schedule (1)\tR\ttramp: 0xffffffffc0300000\n",
        )?
        .file("/proc/kallsyms", KALLSYMS_ROOT)?
        .file("/proc/sys/net/core/bpf_jit_enable", "1\n")?
        .file("/proc/sys/net/core/bpf_jit_kallsyms", "1\n")?
        .file("/proc/sys/net/core/bpf_jit_harden", "0\n")?;
    Ok(f)
}

#[test]
fn t16_informs_of_kprobes_ftrace_hooks_and_bpf_programs() -> R {
    let f = instrumented("t16-info")?;
    let v = t16_instrumentation(&f.host());
    assert_eq!(v.id, "T16");
    assert!(v.is_finding(), "{v}");
    assert_eq!(v.severity, Severity::Info);
    for part in [
        "1 kprobe",
        "do_sys_openat2",
        "1 ftrace-hooked function",
        "2 BPF programs",
        "sd_fw_ingress",
        "bpftool prog show",
    ] {
        assert!(v.message.contains(part), "{part:?} not in {v}");
    }
    // A disabled probe is not counted.
    assert!(!v.message.contains("tcp_sendmsg"), "{v}");
    Ok(())
}

#[test]
fn t16_is_ok_only_when_every_source_is_proven_empty() -> R {
    let f = instrumented("t16-ok")?;
    f.file("/sys/kernel/debug/kprobes/list", "")?
        .file("/sys/kernel/tracing/enabled_functions", "")?
        .file("/proc/kallsyms", "ffffffff81000000 T _stext\n")?;
    let v = t16_instrumentation(&f.host());
    assert_eq!(v.status, Status::Ok, "{v}");
    // Kprobes disarmed globally: listed, but none fires.
    let f = instrumented("t16-disarmed")?;
    f.file("/sys/kernel/debug/kprobes/enabled", "0\n")?
        .file("/sys/kernel/tracing/enabled_functions", "")?
        .file("/proc/kallsyms", "ffffffff81000000 T _stext\n")?;
    assert_eq!(t16_instrumentation(&f.host()).status, Status::Ok);
    Ok(())
}

#[test]
fn t16_is_unknown_without_privilege_never_a_warning() -> R {
    // What an unprivileged process sees: no debugfs or tracefs, kallsyms
    // with its addresses zeroed and no BPF symbol.
    let f = c9g_tuned("t16-unpriv")?;
    f.file(
        "/proc/kallsyms",
        "0000000000000000 T _stext\n0000000000000000 T schedule\n",
    )?
    .file("/proc/sys/net/core/bpf_jit_kallsyms", "1\n")?;
    let v = t16_instrumentation(&f.host());
    assert!(v.is_unknown(), "{v}");
    assert!(!v.is_finding());
    let Status::Unknown(reason) = &v.status else {
        return Err("not unknown".into());
    };
    for part in ["kprobes/list", "enabled_functions", "kallsyms"] {
        assert!(reason.contains(part), "{part:?} not in {reason}");
    }
    // A BPF program it can see still informs, with what it could not read.
    f.file("/proc/kallsyms", KALLSYMS_UNPRIV)?;
    let v = t16_instrumentation(&f.host());
    assert!(v.is_finding(), "{v}");
    assert!(v.message.contains("2 BPF programs"), "{v}");
    assert!(v.message.contains("kprobes/list"), "{v}");
    // JIT symbols not exported: none seen proves nothing.
    let f = instrumented("t16-nojitsyms")?;
    f.file("/sys/kernel/debug/kprobes/list", "")?
        .file("/sys/kernel/tracing/enabled_functions", "")?
        .file("/proc/kallsyms", "ffffffff81000000 T _stext\n")?
        .file("/proc/sys/net/core/bpf_jit_kallsyms", "0\n")?;
    assert!(t16_instrumentation(&f.host()).is_unknown());
    // JIT hardening on: the kernel exports no BPF symbol whatever
    // bpf_jit_kallsyms says (net.rst), so none seen proves nothing.
    f.file("/proc/sys/net/core/bpf_jit_kallsyms", "1\n")?
        .file("/proc/sys/net/core/bpf_jit_harden", "1\n")?;
    let v = t16_instrumentation(&f.host());
    assert!(v.is_unknown(), "{v}");
    assert!(
        matches!(&v.status, Status::Unknown(r) if r.contains("bpf_jit_harden")),
        "{v}"
    );
    f.remove("/proc/sys/net/core/bpf_jit_harden")?;
    assert!(t16_instrumentation(&f.host()).is_unknown());
    Ok(())
}

// ---------------------------------------------------------------- T17

#[test]
fn t17_warns_while_ksm_runs() -> R {
    let f = debian_default("t17-warn")?;
    f.file("/sys/kernel/mm/ksm/run", "1\n")?
        .file("/sys/kernel/mm/ksm/pages_sharing", "5120\n")?;
    let v = t17_ksm(&f.host());
    assert_eq!(v.id, "T17");
    assert!(v.is_warning(), "{v}");
    for part in ["ksmd", "5120 pages", "echo 2 > /sys/kernel/mm/ksm/run"] {
        assert!(v.message.contains(part), "{part:?} not in {v}");
    }
    Ok(())
}

#[test]
fn t17_is_ok_stopped_or_absent_and_unknown_when_unreadable() -> R {
    let f = c9g_tuned("t17-ok")?;
    for stopped in ["0\n", "2\n"] {
        f.file("/sys/kernel/mm/ksm/run", stopped)?;
        assert_eq!(t17_ksm(&f.host()).status, Status::Ok, "{stopped}");
    }
    f.file("/sys/kernel/mm/ksm/run", "on\n")?;
    assert!(t17_ksm(&f.host()).is_unknown());
    // A kernel without KSM has no ksm directory.
    f.remove("/sys/kernel/mm/ksm")?;
    let v = t17_ksm(&f.host());
    assert_eq!(v.status, Status::Ok, "{v}");
    assert!(v.message.contains("no KSM"), "{v}");
    f.remove("/sys/kernel/mm")?;
    let v = t17_ksm(&f.host());
    assert!(v.is_unknown(), "{v}");
    Ok(())
}

// ---------------------------------------------------------------- T18

const PROC_STAT: &str = "cpu  4000 0 2000 90000 10 0 5 70 0 0\n\
     cpu0 1000 0 500 22000 10 0 5 0 0 0\n\
     cpu1 1000 0 500 22500 0 0 0 0 0 0\n\
     cpu3 1000 0 500 22430 0 0 0 70 0 0\n\
     intr 1234 0 0\nctxt 5678\nbtime 1761200000\n";

#[test]
fn t18_informs_of_time_stolen_from_a_hot_cpu() -> R {
    let f = c9g_tuned("t18-info")?;
    f.file("/proc/stat", PROC_STAT)?;
    let v = t18_steal(&f.host(), &loop_on(3));
    assert_eq!(v.id, "T18");
    assert!(v.is_finding(), "{v}");
    assert_eq!(v.severity, Severity::Info);
    for part in ["CPU 3 (loop)", "700 ms", "since boot", "0.3%"] {
        assert!(v.message.contains(part), "{part:?} not in {v}");
    }
    let all = cpu_steal(&f.host())?;
    assert_eq!(
        all.iter().find(|s| s.cpu == 3),
        Some(&CpuSteal {
            cpu: 3,
            steal_ticks: 70,
            total_ticks: 24_000,
        })
    );
    assert_eq!(all.len(), 3, "the summary line is not a CPU");
    Ok(())
}

#[test]
fn t18_is_ok_without_steal_and_unknown_when_unreadable() -> R {
    let f = c9g_tuned("t18-ok")?;
    f.file("/proc/stat", PROC_STAT)?;
    let v = t18_steal(&f.host(), &loop_on(1));
    assert_eq!(v.status, Status::Ok, "{v}");
    // A hot CPU /proc/stat does not list (offline) is not known.
    assert!(t18_steal(&f.host(), &loop_on(2)).is_unknown());
    // A kernel line without the steal column.
    f.file("/proc/stat", "cpu  1 2 3 4\ncpu3 1 2 3 4\n")?;
    assert!(t18_steal(&f.host(), &loop_on(3)).is_unknown());
    f.remove("/proc/stat")?;
    let v = t18_steal(&f.host(), &loop_on(3));
    assert!(v.is_unknown(), "{v}");
    assert!(!v.is_finding());
    Ok(())
}

// ---------------------------------------------------------------- T19

#[test]
fn fault_watch_warns_when_a_hot_thread_faults_and_is_silent_otherwise() -> R {
    let f = c9g_tuned("t19-watch")?;
    let path = "/proc/self/task/4250/stat";
    f.file(path, &stat_line(4250, "ergon (loop) x", 'R', 0, 100, 2))?;
    let mut w = FaultWatch::open(&f.host(), Thread::Task(4250), Role::Loop)?;
    assert_eq!(
        w.last(),
        Faults {
            minor: 100,
            major: 2
        }
    );
    assert_eq!(w.check(), None);
    f.file(path, &stat_line(4250, "ergon (loop) x", 'R', 0, 112, 3))?;
    let v = w.check().ok_or("no finding")?;
    assert_eq!(v.id, "T19");
    assert!(v.is_warning(), "{v}");
    for part in [
        "loop thread",
        "12 minor and 1 major",
        "minflt 100 → 112",
        "majflt 2 → 3",
        "mlockall",
    ] {
        assert!(v.message.contains(part), "{part:?} not in {v}");
    }
    // Reported once: the counters did not move since.
    assert_eq!(w.check(), None);
    assert_eq!(
        Faults {
            minor: 112,
            major: 3
        }
        .since(&Faults {
            minor: 100,
            major: 2
        }),
        Faults {
            minor: 12,
            major: 1
        }
    );
    Ok(())
}

#[test]
fn fault_watch_reads_another_process_and_reports_a_lost_task_once() -> R {
    let f = c9g_tuned("t19-task")?;
    let path = "/proc/77/task/78/stat";
    f.file(path, &stat_line(78, "receiver", 'R', 0, 5, 0))?;
    let mut w = FaultWatch::open_task(&f.host(), 77, 78, Role::Receiver)?;
    f.file(path, "garbage\n")?;
    let v = w.check().ok_or("no finding")?;
    assert!(v.is_unknown(), "{v}");
    assert_eq!(w.check(), None, "an unknown is reported once");
    f.file(path, &stat_line(78, "receiver", 'R', 0, 6, 0))?;
    let v = w.check().ok_or("no finding")?;
    assert!(v.message.contains("receiver thread"), "{v}");
    assert!(v.message.contains("1 minor"), "{v}");
    // Nothing to open: an error, not a watch.
    assert!(FaultWatch::open_task(&f.host(), 77, 79, Role::Receiver).is_err());
    Ok(())
}

// ---------------------------------------------------------------- policy

#[test]
fn an_unknown_new_check_does_not_stop_a_require_start() {
    for id in ["T13", "T14", "T15", "T16", "T17", "T18", "T19"] {
        assert!(!unknown_stops_require(id), "{id}");
    }
    let f = ergon_tune::Finding {
        id: "T16",
        severity: Severity::Info,
        status: Status::Unknown("no debugfs".to_owned()),
        message: "cannot tell".to_owned(),
        evidence: "kprobes.rst",
    };
    assert_eq!(HostCheckPolicy::Require.refusal(&[f]), None);
}
