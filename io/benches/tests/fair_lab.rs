//! The fair mode of the lab (`MODE=fair`) without any cloud: the host
//! settings it holds and checks (`vm-setup.sh fair`, `check`, `pin`) against a
//! fake `/proc` and `/sys` and a stand-in `ethtool`; the receiver's CPU roles;
//! a point that fails when a setting drifts and is measured again; every
//! planned point run once with each backend's own command; and the Azure
//! dry run, with `az`, `aws`, `ssh`, `scp` and `cargo` stand-ins that fail
//! the test if called and a `curl` that answers only Azure's public price
//! API, from a fixture. Needs bash 4 or later on `PATH`, as the lab does.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type R = Result<(), Box<dyn std::error::Error>>;

fn scripts() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts")
}

fn bash(code: &str, env: &[(&str, &str)]) -> Result<Output, std::io::Error> {
    let mut c = Command::new("bash");
    c.arg("-c").arg(code);
    for (k, v) in env {
        c.env(k, v);
    }
    c.output()
}

fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

fn tmp(tag: &str) -> Result<PathBuf, std::io::Error> {
    let d = std::env::temp_dir().join(format!("fair-lab-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

fn write_exec(path: &Path, body: &str) -> Result<(), std::io::Error> {
    std::fs::write(path, body)?;
    let mut perm = std::fs::metadata(path)?.permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perm, 0o755);
    std::fs::set_permissions(path, perm)
}

/// A fake root for `vm-setup.sh`: housekeeping CPU 0 (`irqaffinity=0`), four
/// NIC queue interrupts (IRQ 15 + q, named `<nic-irq-name><q>`) on CPU q, the
/// busy-poll sysctls at `busy_poll`, NIC `nic` with its NAPI settings, and
/// `/sys/devices/system/cpu/smt/active` 0.
fn fake_root(
    tag: &str,
    nic: &str,
    irq_name: &str,
    busy_poll: u32,
) -> Result<PathBuf, std::io::Error> {
    let root = tmp(tag)?;
    std::fs::create_dir_all(root.join("proc/sys/net/core"))?;
    std::fs::create_dir_all(root.join(format!("sys/class/net/{nic}/device")))?;
    std::fs::create_dir_all(root.join("sys/class/net/lo"))?;
    std::fs::create_dir_all(root.join("sys/devices/system/cpu/smt"))?;
    std::fs::create_dir_all(root.join("bin"))?;
    std::fs::write(root.join("sys/devices/system/cpu/smt/active"), "0\n")?;
    std::fs::write(
        root.join("proc/cmdline"),
        "BOOT_IMAGE=/vmlinuz isolcpus=managed_irq,domain,1-3 nohz_full=1-3 irqaffinity=0\n",
    )?;
    std::fs::write(
        root.join("proc/sys/net/core/busy_poll"),
        format!("{busy_poll}\n"),
    )?;
    std::fs::write(root.join("proc/sys/net/core/busy_read"), "0\n")?;
    std::fs::write(
        root.join(format!("sys/class/net/{nic}/napi_defer_hard_irqs")),
        "2\n",
    )?;
    std::fs::write(
        root.join(format!("sys/class/net/{nic}/gro_flush_timeout")),
        "0\n",
    )?;
    let mut interrupts = String::from("           CPU0       CPU1       CPU2       CPU3\n");
    for q in 0..4u32 {
        let irq = 15 + q;
        interrupts.push_str(&format!(
            " {irq}:          1          1          1          1  PCI-MSIX  {} Edge  {irq_name}{q}\n",
            81921 + q
        ));
        let d = root.join(format!("proc/irq/{irq}"));
        std::fs::create_dir_all(&d)?;
        std::fs::write(d.join("smp_affinity_list"), format!("{q}\n"))?;
    }
    std::fs::write(root.join("proc/interrupts"), interrupts)?;
    Ok(root)
}

/// A stand-in `ethtool` for an ENA-like NIC whose moderation (`adaptive
/// usecs cqe-frames`, `-` for no CQE coalescing) is kept in
/// `<root>/ethtool-state`.
fn ena_ethtool(root: &Path, state: &str) -> Result<(), std::io::Error> {
    std::fs::write(root.join("ethtool-state"), format!("{state}\n"))?;
    let st = root.join("ethtool-state");
    let st = st.display();
    write_exec(
        &root.join("bin/ethtool"),
        &format!(
            "#!/usr/bin/env bash\n\
             read -r a u c < {st}\n\
             case $1 in\n\
               -c) printf 'Coalesce parameters for %s:\\nAdaptive RX: %s  TX: off\\nrx-usecs: %s\\nrx-frames: 0\\n' \"$2\" \"$a\" \"$u\"\n\
                   [[ $c == - ]] || printf 'rx-cqe-frames: %s\\n' \"$c\" ;;\n\
               -C) shift 2; while (( $# )); do case $1 in adaptive-rx) a=$2 ;; rx-usecs) u=$2 ;; rx-cqe-frames) c=$2 ;; esac; shift 2; done\n\
                   echo \"$a $u $c\" > {st} ;;\n\
               -i) echo 'driver: ena' ;;\n\
               *) exit 1 ;;\n\
             esac\n"
        ),
    )
}

/// A stand-in `ethtool` for MANA on the kernel of the earlier Azure runs: no
/// coalescing operations at all.
fn mana_ethtool(root: &Path) -> Result<(), std::io::Error> {
    write_exec(
        &root.join("bin/ethtool"),
        "#!/usr/bin/env bash\n\
         case $1 in\n\
           -i) echo 'driver: mana' ;;\n\
           *) echo 'netlink error: Operation not supported' >&2; exit 1 ;;\n\
         esac\n",
    )
}

fn vm_setup(root: &Path, args: &str, arch: &str) -> Result<Output, std::io::Error> {
    let path = format!(
        "{}:{}",
        root.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    bash(
        &format!("bash {} {args}", scripts().join("vm-setup.sh").display()),
        &[
            ("VM_SETUP_ROOT", root.to_str().unwrap_or_default()),
            ("VM_SETUP_ARCH", arch),
            ("PATH", &path),
        ],
    )
}

fn read(p: &Path) -> String {
    std::fs::read_to_string(p)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn last_line(o: &Output) -> String {
    stdout(o).lines().last().unwrap_or_default().to_owned()
}

#[test]
fn fair_holds_every_setting_and_check_names_each_drift() -> R {
    let root = fake_root("hold", "ens5", "ens5-Tx-Rx-", 50)?;
    ena_ethtool(&root, "on 20 4")?;
    // Before the mode: settings not as the fair mode wants are not drift.
    let o = vm_setup(&root, "check", "x86_64")?;
    assert!(stdout(&o).contains("fair=off"), "{o:?}");
    let o = vm_setup(&root, "fair 1", "x86_64")?;
    assert!(o.status.success(), "{o:?}");
    let out = stdout(&o);
    for want in [
        "fair: ens5 (ena): interrupt moderation adaptive-rx on, rx-usecs 20, rx-frames 0, rx-cqe-frames 4 (before)",
        "pin: ens5 rx-cqe-frames was 4, set 1 again",
        "pin: ens5 adaptive-rx was on, set off again",
        "pin: ens5 rx-usecs was 20, set 0 again",
        "NIC queues to 1: the fair mode's IRQ core",
        "fair: net.core.busy_poll 0, busy_read 0; NIC queue IRQs on 1",
    ] {
        assert!(out.contains(want), "{want:?} not in {out}");
    }
    assert_eq!(read(&root.join("ethtool-state")), "off 0 1");
    assert_eq!(read(&root.join("proc/sys/net/core/busy_poll")), "0");
    assert_eq!(
        read(&root.join("sys/class/net/ens5/napi_defer_hard_irqs")),
        "0"
    );
    for irq in 15..19 {
        assert_eq!(
            read(&root.join(format!("proc/irq/{irq}/smp_affinity_list"))),
            "1"
        );
    }
    let o = vm_setup(&root, "check", "x86_64")?;
    assert!(o.status.success(), "{o:?}");
    let out = stdout(&o);
    for want in [
        "fair=on",
        "sysctl.net.core.busy_poll=0",
        "nic.ens5.driver=ena",
        "nic.ens5.adaptive-rx=off",
        "nic.ens5.rx-usecs=0",
        "nic.ens5.rx-cqe-frames=1",
        "nic.ens5.napi_defer_hard_irqs=0",
        "irq.15.ens5-Tx-Rx-0=1",
        "check: ok",
    ] {
        assert!(out.contains(want), "{want:?} not in {out}");
    }
    // The driver turns adaptive moderation back on, an IRQ moves, a sysctl
    // changes: each is named, and the check fails.
    std::fs::write(root.join("ethtool-state"), "on 20 1\n")?;
    std::fs::write(root.join("proc/irq/16/smp_affinity_list"), "3\n")?;
    std::fs::write(root.join("proc/sys/net/core/busy_poll"), "50\n")?;
    let o = vm_setup(&root, "check", "x86_64")?;
    assert_eq!(o.status.code(), Some(3), "{o:?}");
    let verdict = last_line(&o);
    for want in [
        "check: DRIFT 4:",
        "sysctl.net.core.busy_poll want 0 have 50",
        "nic.ens5.adaptive-rx want off have on",
        "nic.ens5.rx-usecs want 0 have 20",
        "irq.16.ens5-Tx-Rx-1 want 1 have 3",
    ] {
        assert!(verdict.contains(want), "{want:?} not in {verdict}");
    }
    // A pin puts all of it back.
    let o = vm_setup(&root, "pin", "x86_64")?;
    assert!(
        stdout(&o).contains("pin: net.core.busy_poll was 50, set 0 again"),
        "{o:?}"
    );
    let o = vm_setup(&root, "check", "x86_64")?;
    assert_eq!(last_line(&o), "check: ok", "{o:?}");
    // irqlocal: the NIC queues belong on the loop CPU while it is on.
    let o = vm_setup(&root, "cond irqlocal on 2", "x86_64")?;
    assert!(o.status.success(), "{o:?}");
    let o = vm_setup(&root, "check", "x86_64")?;
    assert!(stdout(&o).contains("irq.15.ens5-Tx-Rx-0=2"), "{o:?}");
    assert_eq!(last_line(&o), "check: ok", "{o:?}");
    std::fs::write(root.join("proc/irq/15/smp_affinity_list"), "1\n")?;
    let o = vm_setup(&root, "check", "x86_64")?;
    assert!(
        last_line(&o).contains("irq.15.ens5-Tx-Rx-0 want 2 have 1"),
        "{o:?}"
    );
    let o = vm_setup(&root, "cond irqlocal off", "x86_64")?;
    assert!(o.status.success(), "{o:?}");
    let o = vm_setup(&root, "pin", "x86_64")?;
    assert!(o.status.success(), "{o:?}");
    let o = vm_setup(&root, "check", "x86_64")?;
    assert_eq!(last_line(&o), "check: ok", "{o:?}");
    assert!(stdout(&o).contains("irq.15.ens5-Tx-Rx-0=1"), "{o:?}");
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn a_nic_without_moderation_is_recorded_not_failed() -> R {
    // MANA on 6.17.0-1022-azure: `ethtool -c` is not supported.
    let root = fake_root("mana", "ens1", "mana_q", 0)?;
    mana_ethtool(&root)?;
    let o = vm_setup(&root, "fair 1", "x86_64")?;
    assert!(o.status.success(), "{o:?}");
    assert!(
        stdout(&o).contains(
            "fair: ens1 (mana): no interrupt moderation setting (ethtool -c unsupported): nothing to turn off"
        ),
        "{o:?}"
    );
    let o = vm_setup(&root, "check", "x86_64")?;
    assert!(o.status.success(), "{o:?}");
    let out = stdout(&o);
    assert!(
        out.contains("nic.ens1.moderation=none (ethtool -c unsupported)"),
        "{out}"
    );
    assert!(out.contains("irq.15.mana_q0=1"), "{out}");
    assert_eq!(last_line(&o), "check: ok");
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn idle_cpus_poll_where_no_sibling_spins() -> R {
    let root = fake_root("pollidle", "ens5", "ens5-Tx-Rx-", 0)?;
    let o = vm_setup(&root, "bootline full-pollidle 0 1-3", "x86_64")?;
    assert!(o.status.success(), "{o:?}");
    assert_eq!(
        stdout(&o).trim(),
        "isolcpus=managed_irq,domain,1-3 nohz_full=1-3 rcu_nocbs=1-3 irqaffinity=0 idle=poll"
    );
    let o = vm_setup(&root, "bootline full-pollidle 0 1-3", "aarch64")?;
    assert!(stdout(&o).trim().ends_with("irqaffinity=0 nohlt"), "{o:?}");
    // With SMT, idle=poll would spin the loop's sibling: refused.
    std::fs::write(root.join("sys/devices/system/cpu/smt/active"), "1\n")?;
    let o = vm_setup(&root, "bootline full-pollidle 0 1-3", "x86_64")?;
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("has SMT"),
        "{o:?}"
    );
    // Set to poll but booted without it: the check names the drift.
    std::fs::create_dir_all(root.join("var/lib/ergon-lab"))?;
    std::fs::write(
        root.join("var/lib/ergon-lab/boot-profile"),
        "full-pollidle\n",
    )?;
    let o = vm_setup(&root, "check", "x86_64")?;
    assert_eq!(o.status.code(), Some(3), "{o:?}");
    assert!(last_line(&o).contains("idle want poll have halt"), "{o:?}");
    let cmdline = read(&root.join("proc/cmdline"));
    std::fs::write(root.join("proc/cmdline"), format!("{cmdline} idle=poll\n"))?;
    let o = vm_setup(&root, "check", "x86_64")?;
    assert!(stdout(&o).contains("idle=poll-idle=poll\n"), "{o:?}");
    assert!(!stdout(&o).contains("idle want"), "{o:?}");
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// The kernel trace of a point: every event, or (`noirq`, the irqlocal
/// points) every event but the interrupt and softirq ones, which there fire
/// on every packet of the interrupt-driven backends only.
#[test]
fn the_irqlocal_trace_leaves_out_per_packet_interrupt_events() -> R {
    let root = fake_root("trace", "ens5", "ens5-Tx-Rx-", 0)?;
    let t = root.join("sys/kernel/tracing");
    let irq = [
        "irq/irq_handler_entry",
        "irq/irq_handler_exit",
        "irq/softirq_entry",
        "irq/softirq_exit",
    ];
    let other = [
        "sched/sched_switch",
        "ipi",
        "workqueue/workqueue_execute_start",
    ];
    let reset = || -> Result<(), std::io::Error> {
        for e in irq.iter().chain(other.iter()) {
            std::fs::create_dir_all(t.join("events").join(e))?;
            std::fs::write(t.join("events").join(e).join("enable"), "0\n")?;
        }
        Ok(())
    };
    let enabled = |e: &str| read(&t.join("events").join(e).join("enable")) == "1";
    reset()?;
    let o = vm_setup(&root, "trace start 2 noirq", "x86_64")?;
    assert!(o.status.success(), "{o:?}");
    assert_eq!(read(&t.join("tracing_cpumask")), "4");
    assert_eq!(read(&t.join("tracing_on")), "1");
    for e in irq {
        assert!(!enabled(e), "{e} enabled with noirq");
    }
    for e in other {
        assert!(enabled(e), "{e} not enabled");
    }
    reset()?;
    let o = vm_setup(&root, "trace start 2", "x86_64")?;
    assert!(o.status.success(), "{o:?}");
    for e in irq.iter().chain(other.iter()) {
        assert!(enabled(e), "{e} not enabled");
    }
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// `fair_roles` (lib-cloud.sh) on a topology: the roles, or its error.
fn fair_roles(topo: &str) -> Result<String, std::io::Error> {
    let o = bash(
        &format!(
            "source {}; TOPO=$'{topo}'; if fair_roles; then \
             echo \"HK=$HK ISO=$ISO IRQ=$CPU_IRQ A=$CPU_A SQ=$CPU_SQ SIB=$CPU_SIB SPARE=$CPU_SPARE LAYOUT=$LAYOUT SMT=$SMT\"; fi",
            scripts().join("lib-cloud.sh").display()
        ),
        &[],
    )?;
    Ok(format!(
        "{}{}",
        stdout(&o).trim(),
        String::from_utf8_lossy(&o.stderr).trim()
    ))
}

#[test]
fn the_irqs_the_loop_and_sqpoll_each_get_a_core() -> R {
    // Standard_F4as_v6 / Standard_D4ps_v6: four cores, no SMT.
    assert_eq!(
        fair_roles("0,0\\n1,1\\n2,2\\n3,3")?,
        "HK=0 ISO=1,2,3 IRQ=1 A=2 SQ=3 SIB= SPARE= LAYOUT=own-cores SMT=no"
    );
    // Eight cores: the fifth is the spare.
    assert_eq!(
        fair_roles("0,0\\n1,1\\n2,2\\n3,3\\n4,4\\n5,5\\n6,6\\n7,7")?,
        "HK=0 ISO=1,2,3,4,5,6,7 IRQ=1 A=2 SQ=3 SIB= SPARE=4 LAYOUT=own-cores SMT=no"
    );
    // Standard_D4ds_v6: two cores with SMT: IRQs on housekeeping's sibling,
    // SQPOLL on the loop's.
    assert_eq!(
        fair_roles("0,0\\n1,0\\n2,1\\n3,1")?,
        "HK=0 ISO=1,2,3 IRQ=1 A=2 SQ=3 SIB=3 SPARE= LAYOUT=smt-shared SMT=yes"
    );
    // c7i.2xlarge numbering (CPU n and n+4 on core n): siblings stay idle.
    assert_eq!(
        fair_roles("0,0\\n1,1\\n2,2\\n3,3\\n4,0\\n5,1\\n6,2\\n7,3")?,
        "HK=0,4 ISO=1,5,2,6,3,7 IRQ=1 A=2 SQ=3 SIB=6 SPARE= LAYOUT=own-cores SMT=yes"
    );
    assert!(
        fair_roles("0,0\\n1,1\\n2,2")?
            .contains("needs four cores, or two with SMT; this receiver has 3"),
    );
    let boot = |smt: &str, arch: &str, idle: &str| -> Result<String, std::io::Error> {
        let o = bash(
            &format!(
                "source {}; fair_boot {smt} {arch}",
                scripts().join("lib-cloud.sh").display()
            ),
            &[("FAIR_IDLE", idle)],
        )?;
        Ok(stdout(&o).trim().to_owned())
    };
    assert_eq!(boot("no", "x86_64", "poll")?, "full-pollidle");
    assert_eq!(boot("yes", "x86_64", "poll")?, "full");
    assert_eq!(boot("no", "aarch64", "poll")?, "full-pollidle");
    assert_eq!(boot("no", "x86_64", "halt")?, "full");
    Ok(())
}

/// One `traced_run` with FAIR_CHECK, the receiver's job run here for real in
/// `<root>/host` with stand-ins for `sudo` and `vm-setup.sh` (whose `check`
/// prints `<root>/check-<n>`, n counting the calls, exit 3 on DRIFT), and the
/// sender's checks `snd_before`/`snd_after`. Returns the log, the calls and
/// whether the measured command ran.
fn fair_traced_run(
    root: &Path,
    checks: &[&str],
    snd_before: &str,
    snd_after: &str,
) -> Result<(String, String, bool), Box<dyn std::error::Error>> {
    let host = root.join("host");
    let bin = root.join("bin");
    std::fs::create_dir_all(&host)?;
    std::fs::create_dir_all(&bin)?;
    for (i, c) in checks.iter().enumerate() {
        std::fs::write(
            root.join(format!("check-{}", i + 1)),
            format!("fair=on\n{c}\n"),
        )?;
    }
    write_exec(&bin.join("sudo"), "#!/usr/bin/env bash\nexec \"$@\"\n")?;
    let t = root.display();
    std::fs::write(
        host.join("vm-setup.sh"),
        format!(
            "case $1 in\n\
               check) n=$(( $(cat {t}/checks 2>/dev/null || echo 0) + 1 )); echo $n > {t}/checks\n\
                      cat {t}/check-$n; grep -q DRIFT {t}/check-$n && exit 3; exit 0 ;;\n\
               pin) echo 'pin: 0 of 4 IRQs moved to 0' ;;\n\
               *) : ;;\n\
             esac\n"
        ),
    )?;
    let o = bash(
        &format!(
            r#"T={t}
log() {{ echo "$*" >> "$T/log"; }}
sssh() {{ echo "sender: $1" >> "$T/calls"; case $1 in *settings-before*) echo 'sender-before-{snd_before}' ;; *settings-after*) echo 'sender-after-{snd_after}' ;; esac; }}
rssh() {{ echo "receiver: $1" >> "$T/calls"; (cd "$T/host" && bash -c "$1"); }}
rrun() {{ (cd "$T/host" && PATH="$T/bin:$PATH" bash -c "$1"); }}
source {lib}
OUT=$T CPU_A=2 CPU_SPARE= S_CPU=1 FAIR_CHECK=1; mkdir -p "$T/logs"
traced_run f1-irqcore/md-epoll-spin-c1 "echo MEASURED > $T/measured" "" 10
echo "POINT_FAILED=$POINT_FAILED""#,
            lib = scripts().join("lib-matrix.sh").display()
        ),
        &[],
    )?;
    assert!(o.status.success(), "{o:?}");
    let log = std::fs::read_to_string(root.join("log")).unwrap_or_default() + &stdout(&o);
    let calls = std::fs::read_to_string(root.join("calls")).unwrap_or_default();
    Ok((log, calls, root.join("measured").exists()))
}

#[test]
fn a_point_whose_settings_drift_fails_and_says_why() -> R {
    let dir = "f1-irqcore/md-epoll-spin-c1";
    // Both hosts as wanted before and after: the point stands.
    let root = tmp("point-ok")?;
    let (log, calls, measured) =
        fair_traced_run(&root, &["check: ok", "check: ok"], "check: ok", "check: ok")?;
    assert!(measured);
    assert!(log.contains("POINT_FAILED=\n"), "{log}");
    assert!(!log.contains("!!! point"), "{log}");
    // The before check runs after the pin and before the measurement.
    let job = std::fs::read_to_string(root.join("host/res").join(dir).join("settings-before.txt"))?;
    assert!(job.contains("check: ok"));
    assert!(
        root.join("host/res")
            .join(dir)
            .join("settings-after.txt")
            .exists()
    );
    assert!(calls.contains(
        "vm-setup.sh check > srvres/f1-irqcore/md-epoll-spin-c1/settings-before-sender.txt"
    ));
    assert!(!root.join("host/res").join(dir).join("failed.txt").exists());
    std::fs::remove_dir_all(&root)?;

    // The receiver's moderation drifts during the run: failed, with why.
    let root = tmp("point-drift")?;
    let drift = "check: DRIFT 1: nic.ens5.adaptive-rx want off have on";
    let (log, _, measured) =
        fair_traced_run(&root, &["check: ok", drift], "check: ok", "check: ok")?;
    assert!(measured);
    assert!(
        log.contains(&format!(
            "!!! point {dir} FAILED: receiver-after: receiver-after-{drift}"
        )),
        "{log}"
    );
    let failed = std::fs::read_to_string(root.join("host/res").join(dir).join("failed.txt"))?;
    assert!(
        failed.contains("nic.ens5.adaptive-rx want off have on"),
        "{failed}"
    );
    assert!(std::fs::read_to_string(root.join("failed-points.txt"))?.contains(dir));
    std::fs::remove_dir_all(&root)?;

    // Not as wanted before: nothing is measured, and the point fails.
    let root = tmp("point-before")?;
    let (log, _, measured) = fair_traced_run(
        &root,
        &["check: DRIFT 1: irq.15.mana_q0 want 1 have 0", "check: ok"],
        "check: ok",
        "check: ok",
    )?;
    assert!(!measured, "{log}");
    assert!(
        log.contains("receiver-before: receiver-before-check: DRIFT 1"),
        "{log}"
    );
    assert!(log.contains("run exit 97"), "{log}");
    std::fs::remove_dir_all(&root)?;

    // The sender drifted.
    let root = tmp("point-sender")?;
    let (log, _, _) = fair_traced_run(
        &root,
        &["check: ok", "check: ok"],
        "check: ok",
        "check: DRIFT 1: sysctl.net.core.busy_poll want 0 have 50",
    )?;
    assert!(
        log.contains("sender-after: sender-after-check: DRIFT 1: sysctl.net.core.busy_poll"),
        "{log}"
    );
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// `run_fair` with the hosts stubbed: every traced run (dir | command |
/// server | seconds), server start and condition change, in order.
fn run_fair(
    root: &Path,
    env: &[(&str, &str)],
    fail_first: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let o = bash(
        &format!(
            r#"T={t}
log() {{ echo "log: $*" >> "$T/trace"; }}
sleep() {{ :; }}
sssh() {{ :; }}
rssh() {{ case $1 in cat*) cat > "$T/fair-env.txt" ;; mv*|rm*) echo "receiver: $1" >> "$T/trace" ;; esac; }}
relay() {{ :; }}
source {lib}
apply_conds() {{ echo "cond $1 $2" >> "$T/trace"; }}
fair_server() {{ echo "server $1 $2 $3" >> "$T/trace"; }}
traced_run() {{
  echo "run $1 | $2 | $3 | $4" >> "$T/trace"
  echo "noirq $1 ${{TRACE_NOIRQ:-}}" >> "$T/trace"
  POINT_FAILED=""
  if [[ $1 == "{fail_first}" && ! -e $T/failed-once ]]; then : > "$T/failed-once"; POINT_FAILED="drift"; fi
}}
OUT=$T CPU_A=2 CPU_SQ=3 CPU_SPARE= S_CPU=1 SND_PRIV=10.0.0.5 HK=0 CPU_IRQ=1 LAYOUT=own-cores RCV_BOOT=full-pollidle
run_fair
fair_points"#,
            t = root.display(),
            lib = scripts().join("lib-matrix.sh").display()
        ),
        env,
    )?;
    assert!(o.status.success(), "{o:?}");
    Ok((std::fs::read_to_string(root.join("trace"))?, stdout(&o)))
}

#[test]
fn every_planned_point_runs_once_with_its_backends_own_command() -> R {
    let root = tmp("run-fair")?;
    let env = [
        ("FAIR_ROUNDS", "2"),
        ("FAIR_MD_CONNS", "1 4"),
        ("FAIR_RT_CONNS", "1 4"),
        ("FAIR_HTTP_POOLS", "1 20"),
    ];
    let (trace, points) = run_fair(&root, &env, "f1-irqcore/rt-recv-busy-pq-n4-r10000")?;
    let runs: Vec<&str> = trace
        .lines()
        .filter_map(|l| l.strip_prefix("run "))
        .map(|l| l.split(" | ").next().unwrap_or_default())
        .collect();
    // fair_points' order, one run each, and the failed one again.
    let planned: Vec<String> = points
        .lines()
        .map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            let run = match f[2] {
                "md" => format!("md-{}-c{}", f[3], f[4]),
                "rt" => format!("rt-{}-n{}-r{}", f[3], f[4], f[5]),
                _ => format!("http-{}-p{}-r{}", f[3], f[4], f[5]),
            };
            format!("f{}-{}/{run}", f[0], f[1])
        })
        .collect();
    // 2 rounds x (irqcore: 6 backends x (2 md + 2 rt + 2 http) + irqlocal: 3 x 6).
    assert_eq!(planned.len(), 2 * (6 * 6 + 3 * 6));
    let mut expected: Vec<&str> = Vec::new();
    for p in &planned {
        expected.push(p);
        if p == "f1-irqcore/rt-recv-busy-pq-n4-r10000" {
            expected.push(p);
        }
    }
    assert_eq!(runs, expected);
    assert!(trace.contains(
        "receiver: rm -rf res/f1-irqcore/rt-recv-busy-pq-n4-r10000.attempt1; mv res/f1-irqcore/rt-recv-busy-pq-n4-r10000 res/f1-irqcore/rt-recv-busy-pq-n4-r10000.attempt1"
    ));
    // irqlocal: interrupt-driven backends only, between its on and off.
    let on = trace.find("cond irqlocal on").ok_or("no irqlocal on")?;
    let off = trace.find("cond irqlocal off").ok_or("no irqlocal off")?;
    let block = &trace[on..off];
    assert!(block.contains("f1-irqlocal/md-epoll-spin-c1"));
    assert!(block.contains("f1-irqlocal/md-mio-c1"));
    assert!(block.contains("f1-irqlocal/md-uring-spin-c1"));
    assert!(
        !block.contains("recv-busy") && !block.contains("sqpoll"),
        "{block}"
    );
    // The trace leaves out the per-packet interrupt events on irqlocal
    // points, and only there.
    for l in trace.lines().filter_map(|l| l.strip_prefix("noirq ")) {
        let (run, noirq) = l.split_once(' ').ok_or("noirq line")?;
        assert_eq!(noirq == "1", run.contains("-irqlocal/"), "{l}");
    }
    // Each backend's own client and settings.
    let line = |run: &str| -> String {
        trace
            .lines()
            .find(|l| l.starts_with(&format!("run {run} ")))
            .unwrap_or_default()
            .to_owned()
    };
    for (run, cmd) in [
        (
            "f1-irqcore/md-mio-c4",
            "sudo bin/mio_bench lab --server 10.0.0.5:9447 --ca cert-many.pem --sni lab.local --cpu 2 --conns 4",
        ),
        (
            "f1-irqcore/md-epoll-spin-c4",
            "sudo bin/io_bench lab --server 10.0.0.5:9447 --ca cert-many.pem --sni lab.local --variants epoll-spin  --cpus 2 --busy-poll 1 --wait-us 0 --conns 4",
        ),
        (
            "f1-irqcore/md-uring-sqpoll-c1",
            "--variants uring-sqpoll --sqpoll-cpu 3 --cpus 2",
        ),
        (
            "f1-irqcore/rt-recv-busy-pq-n4-r10000",
            "sudo bin/io_bench pingpong --server 10.0.0.5:9448 --ca cert-rt.pem --sni lab.local --variant recv-busy --per-queue --cpu 2 --busy-poll 1 --wait-us 0 --feeds 3 --rate 10000",
        ),
        (
            "f1-irqcore/rt-mio-n1-r10000",
            "sudo bin/mio_bench pingpong --server 10.0.0.5:9448 --ca cert-rt.pem --sni lab.local --cpu 2 --feeds 0 --rate 10000",
        ),
        (
            "f1-irqcore/http-uring-sqpoll-p20-r10000",
            "sudo bin/http_bench --client client --impl native --backend uring-sqpoll --sqpoll-cpu 3 --pool 20 --server 10.0.0.5:9445",
        ),
        (
            "f1-irqcore/http-mio-p20-r10000",
            "sudo bin/mio_bench http --server 10.0.0.5:9445 --ca cert-http.pem --sni lab.local --cpu 2 --pool 20 --rate 10000",
        ),
    ] {
        assert!(
            line(run).contains(cmd),
            "{run}: {cmd:?} not in {}",
            line(run)
        );
    }
    assert!(!line("f1-irqcore/http-epoll-spin-p1-r10000").contains("sqpoll-cpu"));
    assert!(line("f1-irqcore/rt-epoll-spin-n4-r10000").ends_with("| echo | 60"));
    assert!(line("f1-irqcore/http-recv-busy-p20-r10000").ends_with("| http | 60"));
    // The servers: market data and feeds the same traffic; the order
    // connection echoed by the same thread; HTTP on one thread.
    assert!(trace.contains("server 9447 cert-many.pem --clients 4 --mode mixed --trickle-min-us 10000 --trickle-max-us 100000 --burst-every-us 2000 --burst-slots 10 --burst 5 --seed 42 --streams 1024 --duration 120"));
    assert!(trace.contains("server 9448 cert-rt.pem --clients 4 --mode mixed --trickle-min-us 10000 --trickle-max-us 100000 --burst-every-us 2000 --burst-slots 10 --burst 5 --seed 42 --busy-poll 50 --stalls-dir $HOME/srvstalls --duration 120 --streams 1024"));
    assert!(trace.contains("server 9448 cert-rt.pem --clients 1 --mode mixed --trickle-min-us 10000 --trickle-max-us 100000 --burst-every-us 2000 --burst-slots 10 --burst 5 --seed 42 --busy-poll 50 --stalls-dir $HOME/srvstalls --duration 120\n"));
    assert!(trace.contains("server 9445 cert-http.pem --mode http --threads one --busy-poll 50"));
    // The report's environment: the layout and what each backend sets.
    let env = std::fs::read_to_string(root.join("fair-env.txt"))?;
    for want in [
        "layout=own-cores",
        "irq_cpu=1",
        "loop_cpu=2",
        "sqpoll_cpu=3",
        "receiver_boot=full-pollidle",
        "backend.mio=interrupt: mio + rustls",
        "backend.recv-busy-pq=polling:",
        "setting.1=net.core.busy_poll 0, net.core.busy_read 0",
    ] {
        assert!(env.contains(want), "{want:?} not in {env}");
    }
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// The Azure prices API's answer for one filter, shaped as the API answers
/// (each item's own fields first, then its nested savings plans).
fn price_items(items: &[(f64, &str, &str, &str)]) -> String {
    let body: Vec<String> = items
        .iter()
        .map(|(p, sku, product, unit)| {
            format!(
                "{{\"currencyCode\":\"USD\",\"tierMinimumUnits\":0.0,\"retailPrice\":{p},\"unitPrice\":{p},\"armRegionName\":\"eastasia\",\
                 \"skuName\":\"{sku}\",\"productName\":\"{product}\",\"serviceName\":\"x\",\"unitOfMeasure\":\"{unit}\",\"type\":\"Consumption\",\
                 \"savingsPlan\":[{{\"unitPrice\":0.01,\"retailPrice\":0.01,\"term\":\"3 Years\"}}]}}"
            )
        })
        .collect();
    format!(
        "{{\"BillingCurrency\":\"USD\",\"Items\":[{}],\"NextPageLink\":null,\"Count\":{}}}",
        body.join(","),
        items.len()
    )
}

/// Stand-ins first on PATH: `az`, `aws`, `ssh`, `scp` and `cargo` log their
/// call and fail; `curl` answers Azure's price API from `<root>/prices/` and
/// fails anything else.
fn stubs(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let bin = root.join("stubs");
    let prices = root.join("prices");
    std::fs::create_dir_all(&bin)?;
    std::fs::create_dir_all(&prices)?;
    let calls = root.join("forbidden");
    for tool in ["az", "aws", "ssh", "scp", "cargo", "rsync"] {
        write_exec(
            &bin.join(tool),
            &format!(
                "#!/usr/bin/env bash\necho \"{tool} $*\" >> {}\nexit 99\n",
                calls.display()
            ),
        )?;
    }
    let vm = |sku: &str, short: &str, family: &str, regular: f64, spot: f64| {
        price_items(&[
            (
                regular,
                short,
                &format!("Virtual Machines {family} Series"),
                "1 Hour",
            ),
            (
                regular * 1.5,
                short,
                &format!("Virtual Machines {family} Series Windows"),
                "1 Hour",
            ),
            (
                spot,
                &format!("{short} Spot"),
                &format!("Virtual Machines {family} Series"),
                "1 Hour",
            ),
            (
                spot * 1.5,
                &format!("{short} Spot"),
                &format!("Virtual Machines {family} Series Windows"),
                "1 Hour",
            ),
        ])
        .replace(
            "\"armRegionName\"",
            &format!("\"armSkuName\":\"{sku}\",\"armRegionName\""),
        )
    };
    for (sku, short, family, regular, spot) in [
        ("Standard_F4as_v6", "F4as v6", "Fasv6", 0.4, 0.08),
        ("Standard_F2as_v6", "F2as v6", "Fasv6", 0.2, 0.04),
        ("Standard_D4ps_v6", "D4ps v6", "Dpsv6", 0.2, 0.04),
        ("Standard_D2ps_v6", "D2ps v6", "Dpsv6", 0.1, 0.02),
    ] {
        std::fs::write(prices.join(sku), vm(sku, short, family, regular, spot))?;
    }
    // As the API lists a disk SKU: its "Disk Mount" meter (shared disks) in
    // the same unit and often first, and a page-blob product with the same
    // meter; only the managed disk's own meter is the OS disk's price.
    let disk = |p: f64, sku: &str, product: &str, meter: &str| {
        format!(
            "{{\"currencyCode\":\"USD\",\"retailPrice\":{p},\"unitPrice\":{p},\"armRegionName\":\"eastasia\",\
             \"meterName\":\"{meter}\",\"skuName\":\"{sku}\",\"productName\":\"{product}\",\"serviceName\":\"Storage\",\
             \"unitOfMeasure\":\"1/Month\",\"type\":\"Consumption\"}}"
        )
    };
    let disks = [
        disk(
            1.41,
            "P10 LRS",
            "Premium SSD Managed Disks",
            "P10 LRS Disk Mount",
        ),
        disk(99.0, "P10 LRS", "Premium Page Blob", "P10 LRS Disk"),
        disk(73.0, "P10 LRS", "Premium SSD Managed Disks", "P10 LRS Disk"),
        disk(
            0.403,
            "P4 LRS",
            "Premium SSD Managed Disks",
            "P4 LRS Disk Mount",
        ),
        disk(21.9, "P4 LRS", "Premium SSD Managed Disks", "P4 LRS Disk"),
    ];
    std::fs::write(
        prices.join("disk"),
        format!(
            "{{\"BillingCurrency\":\"USD\",\"Items\":[{}],\"NextPageLink\":null,\"Count\":{}}}",
            disks.join(","),
            disks.len()
        ),
    )?;
    std::fs::write(
        prices.join("ip"),
        price_items(&[(0.005, "Standard", "IP Addresses", "1 Hour")]),
    )?;
    write_exec(
        &bin.join("curl"),
        &format!(
            "#!/usr/bin/env bash\n\
             url=\"\" filter=\"\"\n\
             while (( $# )); do case $1 in https://*) url=$1 ;; --data-urlencode) filter=$2; shift ;; esac; shift; done\n\
             if [[ $url != https://prices.azure.com/api/retail/prices ]]; then echo \"curl $url\" >> {calls}; exit 99; fi\n\
             case $filter in\n\
               *\"armSkuName eq '\"*) sku=${{filter#*armSkuName eq \\'}}; cat {p}/${{sku%%\\'*}} ;;\n\
               *Storage*) cat {p}/disk ;;\n\
               *IPv4*) cat {p}/ip ;;\n\
               *) exit 22 ;;\n\
             esac\n",
            calls = calls.display(),
            p = prices.display()
        ),
    )?;
    Ok(format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    ))
}

fn dry_run(
    root: &Path,
    script: &str,
    env: &[(&str, &str)],
) -> Result<Output, Box<dyn std::error::Error>> {
    let path = stubs(root)?;
    let tmpdir = root.join("tmp");
    std::fs::create_dir_all(&tmpdir)?;
    let mut c = Command::new("bash");
    c.arg(scripts().join(script));
    c.env("PATH", &path)
        .env("TMPDIR", &tmpdir)
        .env("STAMP", "20261017T120000Z")
        .env("DRY_RUN", "1")
        .env_remove("ERGON_LAB_HOME")
        .env_remove("RCV_SIZE")
        .env_remove("SND_SIZE")
        .env_remove("IMAGE")
        .env_remove("PRIORITY")
        .env_remove("MAX_S");
    for (k, v) in env {
        c.env(k, v);
    }
    let o = c.output()?;
    let forbidden = std::fs::read_to_string(root.join("forbidden")).unwrap_or_default();
    assert!(forbidden.is_empty(), "called in a dry run: {forbidden}");
    Ok(o)
}

#[test]
fn the_dry_run_prints_the_plan_and_the_estimate_and_touches_no_cloud() -> R {
    let root = tmp("dry-x86")?;
    let o = dry_run(
        &root,
        "azure-flags.sh",
        &[("MODE", "fair"), ("ARCH", "x86")],
    )?;
    assert!(o.status.success(), "{o:?}");
    let out = stdout(&o);
    for want in [
        "# Fair run plan, Azure x86 (run 20261017T120000Z)",
        "resource group ergon-lab-io-fair-x86-20261017T120000Z",
        "tags owner=ergon-lab run=20261017T120000Z platform=x86",
        "receiver Standard_F4as_v6, sender Standard_F2as_v6, one proximity placement group, accelerated networking (MANA), image Canonical:ubuntu-24_04-lts:server:latest",
        "priority Spot, eviction policy Delete",
        "quota needed: 6 vCPUs (regional spot quota, lowPriorityCores)",
        "MANA interrupt moderation: kernel 6.17.0-1022-azure",
        "Standard_F4as_v6: 4 vCPUs, SMT no, layout own-cores",
        "NIC interrupts CPU 1; loop CPU 2; SQPOLL CPU 3",
        "receiver boot profile full-pollidle",
        "== points: 198 (2 rounds; conditions irqcore irqlocal)",
        "irqcore md         60 points",
        "irqlocal md        30 points",
        "net.core.busy_poll 0, net.core.busy_read 0",
        "mio           interrupt mio + rustls",
        "uring-sqpoll  polling   ergon-io: io_uring with its SQPOLL kernel thread",
        "1 irqcore md epoll-spin 1 - 60",
        "2 irqcore http epoll-spin 20 10000 60",
        "total about 373 min (6.22 h)",
        "the run detaches (nohup) when its deadline is over 2 h",
        // 6.22 h x (0.08 + 0.04 + 73/730 + 21.9/730 + 2 x 0.005) = 6.22 x 0.26:
        // the managed disks' own meters, not the mount meters or page blobs.
        "prices (USD/h, Linux, retail): Standard_F4as_v6 spot 0.0800, regular 0.4000; Standard_F2as_v6 spot 0.0400, regular 0.2000; OS disks: receiver (P10 LRS, 128 GB) 0.1000, sender (P4 LRS, 30 GB) 0.0300; 2 public IPs 0.0100",
        "cost for 6.22 h: spot 1.62 USD (0.2600/h), regular 4.60 USD (0.7400/h); this run: Spot",
        // The deadline, 1.5 x 373 min = 9.34 h.
        "at most (stopped by the deadline, 9.34 h): spot 2.43 USD, regular 6.91 USD",
    ] {
        assert!(out.contains(want), "{want:?} not in\n{out}");
    }
    // irqlocal: the interrupt-driven backends only.
    let points: Vec<&str> = out
        .lines()
        .filter(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            f.len() == 7 && f[1] == "irqlocal"
        })
        .collect();
    assert_eq!(points.len(), 2 * 3 * (5 + 4 + 2), "{out}");
    for l in points {
        assert!(
            ["epoll-spin", "mio", "uring-spin"]
                .iter()
                .any(|b| l.contains(&format!(" {b} "))),
            "{l}"
        );
    }
    // Nothing was created locally either.
    assert!(
        !Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("results/fair-azure-x86-20261017T120000Z")
            .exists()
    );
    std::fs::remove_dir_all(&root)?;

    // The x86 fallback: two cores with SMT, the layout said.
    let root = tmp("dry-fallback")?;
    let o = dry_run(
        &root,
        "azure-flags.sh",
        &[
            ("MODE", "fair"),
            ("ARCH", "x86"),
            ("RCV_SIZE", "Standard_D4ds_v6"),
            ("SND_SIZE", "Standard_D2ds_v6"),
            ("PRIORITY", "Regular"),
        ],
    )?;
    assert!(o.status.success(), "{o:?}");
    let out = stdout(&o);
    assert!(
        out.contains("Standard_D4ds_v6: 4 vCPUs, SMT yes, layout smt-shared"),
        "{out}"
    );
    assert!(
        out.contains("the SQPOLL thread shares the loop's (CPU 2's sibling)"),
        "{out}"
    );
    assert!(
        out.contains("receiver boot profile full (isolated: 1,2,3)"),
        "{out}"
    );
    assert!(out.contains("priority Regular"), "{out}");
    // No price fixture for these sizes: said, not guessed.
    assert!(out.contains("cost: prices not available"), "{out}");
    std::fs::remove_dir_all(&root)?;

    // Both platforms: Arm on Cobalt with the arm64 image, and the total.
    let root = tmp("dry-both")?;
    let o = dry_run(&root, "azure-fair.sh", &[])?;
    assert!(o.status.success(), "{o:?}");
    let out = stdout(&o);
    for want in [
        "# Fair run plan, Azure x86 (run 20261017T120000Z)",
        "# Fair run plan, Azure arm (run 20261017T120000Z)",
        "receiver Standard_D4ps_v6, sender Standard_D2ps_v6, one proximity placement group, accelerated networking (MANA), image Canonical:ubuntu-24_04-lts:server-arm64:latest",
        "resource group ergon-lab-io-fair-arm-20261017T120000Z",
        "x86:   cost for 6.22 h: spot 1.62 USD",
        // 6.22 h x (0.04 + 0.02 + 0.14) = 1.24 spot; x (0.2 + 0.1 + 0.14) = 2.74
        // regular; with x86's 1.62 and 4.60.
        "arm:   cost for 6.22 h: spot 1.24 USD (0.2000/h), regular 2.74 USD (0.4400/h)",
        "together: spot 2.86 USD, regular 7.34 USD",
    ] {
        assert!(out.contains(want), "{want:?} not in\n{out}");
    }
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn an_unknown_architecture_or_priority_stops_before_anything() -> R {
    let root = tmp("dry-bad")?;
    let o = dry_run(
        &root,
        "azure-flags.sh",
        &[("MODE", "fair"), ("ARCH", "riscv")],
    )?;
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(stdout(&o).contains("ARCH=riscv: x86 or arm"), "{o:?}");
    let o = dry_run(
        &root,
        "azure-flags.sh",
        &[("MODE", "fair"), ("PRIORITY", "Cheap")],
    )?;
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(
        stdout(&o).contains("PRIORITY=Cheap: Spot or Regular"),
        "{o:?}"
    );
    std::fs::remove_dir_all(&root)?;
    Ok(())
}
