//! The lab's host scripts (`scripts/lib-cloud.sh`, `scripts/vm-setup.sh`) on
//! x86 and Graviton layouts, against a fake `/proc` where they touch the
//! host. Needs bash 4 or later on `PATH`, as the lab scripts do.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type R = Result<(), Box<dyn std::error::Error>>;

fn script(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join(name)
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

/// The CPU roles `lib-cloud.sh` derives from `lscpu -p=CPU,CORE` on a
/// receiver of architecture `arch` (`uname -m`).
fn roles(topo: &str, arch: &str) -> Result<String, Box<dyn std::error::Error>> {
    let o = bash(
        &format!(
            "source {}; TOPO=$'{topo}'; RARCH={arch}; cpu_roles; \
             echo \"HK=$HK ISO=$ISO A=$CPU_A SIB=$CPU_SIB SPARE=$CPU_SPARE SQ=$CPU_SQ SMT=$SMT\"",
            script("lib-cloud.sh").display()
        ),
        &[],
    )?;
    assert!(o.status.success(), "{o:?}");
    Ok(stdout(&o).trim().to_owned())
}

#[test]
fn an_smt_receiver_keeps_its_layout() -> R {
    // c7i.2xlarge: 4 cores, CPU n and n+4 on core n.
    let topo = "0,0\\n1,1\\n2,2\\n3,3\\n4,0\\n5,1\\n6,2\\n7,3";
    assert_eq!(
        roles(topo, "x86_64")?,
        "HK=0,4 ISO=1,5,2,6,3,7 A=1 SIB=5 SPARE=2 SQ=3 SMT=yes"
    );
    Ok(())
}

#[test]
fn an_x86_receiver_without_smt_keeps_its_layout() -> R {
    // c7a.2xlarge (AMD, one thread per core): measured with core 0 alone for
    // housekeeping (BENCHMARK.md, instance types); the two-core housekeeping
    // is Graviton's.
    let topo = "0,0\\n1,1\\n2,2\\n3,3\\n4,4\\n5,5\\n6,6\\n7,7";
    assert_eq!(
        roles(topo, "x86_64")?,
        "HK=0 ISO=1,2,3,4,5,6,7 A=1 SIB= SPARE=2 SQ=3 SMT=no"
    );
    Ok(())
}

#[test]
fn a_graviton_receiver_has_two_housekeeping_cores_and_no_sibling() -> R {
    // c7g.2xlarge: 8 cores, one CPU each.
    let topo = "0,0\\n1,1\\n2,2\\n3,3\\n4,4\\n5,5\\n6,6\\n7,7";
    assert_eq!(
        roles(topo, "aarch64")?,
        "HK=0,1 ISO=2,3,4,5,6,7 A=2 SIB= SPARE=3 SQ=4 SMT=no"
    );
    // c7g.xlarge: too few cores to give housekeeping two.
    assert_eq!(
        roles("0,0\\n1,1\\n2,2\\n3,3", "aarch64")?,
        "HK=0 ISO=1,2,3 A=1 SIB= SPARE=2 SQ=3 SMT=no"
    );
    Ok(())
}

#[test]
fn rust_build_flags_follow_the_graviton_generation() -> R {
    let flags = |arch: &str, midr: &str, over: Option<&str>| -> Result<String, std::io::Error> {
        let mut env = vec![];
        if let Some(f) = over {
            env.push(("BUILD_RUSTFLAGS", f));
        }
        let o = bash(
            &format!(
                "source {}; build_rustflags {arch} {midr}",
                script("lib-cloud.sh").display()
            ),
            &env,
        )?;
        Ok(stdout(&o).trim().to_owned())
    };
    // MIDR_EL1: implementer 0x41 (Arm), part in bits 15:4.
    assert_eq!(
        flags("aarch64", "0x00000000413fd0c1", None)?,
        "-Ctarget-cpu=neoverse-n1"
    );
    assert_eq!(
        flags("aarch64", "0x00000000411fd401", None)?,
        "-Ctarget-cpu=neoverse-v1"
    );
    assert_eq!(
        flags("aarch64", "0x00000000410fd4f1", None)?,
        "-Ctarget-cpu=neoverse-v2"
    );
    assert_eq!(
        flags("aarch64", "0x00000000410fd840", None)?,
        "-Ctarget-cpu=neoverse-v3"
    );
    assert_eq!(flags("aarch64", "-", None)?, "-Ctarget-cpu=native");
    // Another implementer's core (0xc0, Ampere) whose part number is a
    // Neoverse one: built for itself.
    assert_eq!(
        flags("aarch64", "0x00000000c00fd400", None)?,
        "-Ctarget-cpu=native"
    );
    // x86 builds stay generic, as every published x86 run was built.
    assert_eq!(flags("x86_64", "-", None)?, "");
    assert_eq!(
        flags("x86_64", "-", Some("-Ctarget-cpu=native"))?,
        "-Ctarget-cpu=native"
    );
    Ok(())
}

/// A fake root: `/proc/cmdline` with `irqaffinity=0,1`, eight ENA queue
/// interrupts plus the management one and a timer PPI, every queue where
/// the ENA driver's affinity hint puts it after the interface comes up
/// (queue i on CPU i).
fn fake_root(tag: &str) -> Result<PathBuf, std::io::Error> {
    let root = std::env::temp_dir().join(format!("vm-setup-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("proc"))?;
    std::fs::write(
        root.join("proc/cmdline"),
        "BOOT_IMAGE=/vmlinuz isolcpus=managed_irq,domain,2-7 nohz_full=2-7 irqaffinity=0,1\n",
    )?;
    let mut interrupts = String::from("           CPU0       CPU1\n");
    interrupts.push_str(" 11:  9 9  GICv3  27 Level  arch_timer\n");
    interrupts.push_str(" 14:  5 0  ITS-MSI 81920 Edge  ena-mgmnt@pci:0000:00:05.0\n");
    for q in 0..8 {
        let irq = 15 + q;
        interrupts.push_str(&format!(
            " {irq}:  1 1  ITS-MSI {} Edge  ens5-Tx-Rx-{q}\n",
            81921 + q
        ));
        let d = root.join(format!("proc/irq/{irq}"));
        std::fs::create_dir_all(&d)?;
        std::fs::write(d.join("smp_affinity_list"), format!("{q}\n"))?;
    }
    for (irq, aff) in [(11, "0-7"), (14, "0-1")] {
        let d = root.join(format!("proc/irq/{irq}"));
        std::fs::create_dir_all(&d)?;
        std::fs::write(d.join("smp_affinity_list"), format!("{aff}\n"))?;
    }
    std::fs::write(root.join("proc/interrupts"), interrupts)?;
    Ok(root)
}

fn affinity(root: &Path, irq: u32) -> Result<String, std::io::Error> {
    Ok(
        std::fs::read_to_string(root.join(format!("proc/irq/{irq}/smp_affinity_list")))?
            .trim()
            .to_owned(),
    )
}

fn vm_setup(root: &Path, args: &str, arch: &str) -> Result<Output, std::io::Error> {
    // A stand-in ethtool on PATH first: it keeps one NIC's interrupt
    // moderation in <root>/ethtool-state and logs every change.
    let path = format!(
        "{}:{}",
        root.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    bash(
        &format!("bash {} {args}", script("vm-setup.sh").display()),
        &[
            ("VM_SETUP_ROOT", root.to_str().unwrap_or_default()),
            ("VM_SETUP_ARCH", arch),
            ("PATH", &path),
        ],
    )
}

/// One ENA NIC (`ens5`) under `<root>/sys/class/net` and a stand-in
/// `ethtool` that reports and changes its interrupt moderation, and its
/// eight combined queues: a queue change (`-L`) brings the interface down and
/// up as `ena_up` does, each queue's interrupt back on CPU i and adaptive
/// moderation back on.
fn fake_nic(root: &Path, adaptive: &str, rx_usecs: u32) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(root.join("sys/class/net/ens5/device"))?;
    std::fs::create_dir_all(root.join("sys/class/net/lo"))?;
    std::fs::write(
        root.join("ethtool-state"),
        format!("{adaptive} {rx_usecs}\n"),
    )?;
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin)?;
    let st = root.join("ethtool-state");
    let st = st.display();
    let irq = root.join("proc/irq");
    let irq = irq.display();
    std::fs::write(
        bin.join("ethtool"),
        format!(
            "#!/usr/bin/env bash\n\
             read -r a u < {st}\n\
             case $1 in\n\
               -c) printf 'Coalesce parameters for %s:\\nAdaptive RX: %s  TX: off\\nrx-usecs: %s\\n' \"$2\" \"$a\" \"$u\" ;;\n\
               -C) d=$2; shift 2; echo \"-C $d $*\" >> {st}.log\n\
                   while (( $# )); do case $1 in adaptive-rx) a=$2 ;; rx-usecs) u=$2 ;; esac; shift 2; done\n\
                   echo \"$a $u\" > {st} ;;\n\
               -l) printf 'Channel parameters for %s:\\nPre-set maximums:\\nCombined:\\t8\\nCurrent hardware settings:\\nCombined:\\t8\\n' \"$2\" ;;\n\
               -L) echo \"$*\" >> {st}.log\n\
                   for q in 0 1 2 3 4 5 6 7; do echo $q > {irq}/$((15 + q))/smp_affinity_list; done\n\
                   echo \"on $u\" > {st} ;;\n\
               *) exit 1 ;;\n\
             esac\n"
        ),
    )?;
    let mut perm = std::fs::metadata(bin.join("ethtool"))?.permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perm, 0o755);
    std::fs::set_permissions(bin.join("ethtool"), perm)
}

fn ethtool_state(root: &Path) -> Result<String, std::io::Error> {
    Ok(std::fs::read_to_string(root.join("ethtool-state"))?
        .trim()
        .to_owned())
}

#[test]
fn pin_turns_adaptive_moderation_off_again_after_the_driver_turned_it_on() -> R {
    let root = fake_root("nic")?;
    fake_nic(&root, "on", 20)?;
    // The run asked for adaptive receive moderation off (`tune nic`)...
    let o = vm_setup(&root, "cond nodim on", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    assert_eq!(ethtool_state(&root)?, "off 20");
    // ...then the interface came up again (ethtool -L, a reset): ena_up
    // turns it back on. The next pin turns it off.
    std::fs::write(root.join("ethtool-state"), "on 20\n")?;
    let o = vm_setup(&root, "pin", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    assert_eq!(ethtool_state(&root)?, "off 20");
    assert!(
        stdout(&o).contains("pin: ens5 adaptive-rx was on, set off again"),
        "{}",
        stdout(&o)
    );
    // With the condition off, the NIC keeps whatever it has.
    let o = vm_setup(&root, "cond nodim off", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    assert_eq!(ethtool_state(&root)?, "on 20");
    let o = vm_setup(&root, "pin", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    assert_eq!(ethtool_state(&root)?, "on 20");
    // nocoalesce holds rx-usecs 0 as well.
    let o = vm_setup(&root, "cond nocoalesce on", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    std::fs::write(root.join("ethtool-state"), "on 20\n")?;
    let o = vm_setup(&root, "pin", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    assert_eq!(ethtool_state(&root)?, "off 0");
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn pin_moves_nic_queues_back_to_housekeeping_after_a_reset() -> R {
    let root = fake_root("pin")?;
    let o = vm_setup(&root, "pin", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    for irq in 15..23 {
        assert_eq!(affinity(&root, irq)?, "0,1", "irq {irq}");
    }
    assert_eq!(affinity(&root, 11)?, "0,1");
    // Already on the housekeeping CPUs (as the kernel prints them): not
    // written again.
    assert_eq!(affinity(&root, 14)?, "0-1");
    assert!(
        stdout(&o).contains("pin: 9 of 10 IRQs moved to 0,1"),
        "{}",
        stdout(&o)
    );
    // Nothing drifted since: nothing moves.
    let o = vm_setup(&root, "pin", "aarch64")?;
    assert!(
        stdout(&o).contains("pin: 0 of 10 IRQs moved"),
        "{}",
        stdout(&o)
    );
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn pin_keeps_the_nic_queues_on_the_loop_cpu_while_irqlocal_is_on() -> R {
    let root = fake_root("irqlocal")?;
    let o = vm_setup(&root, "cond irqlocal on 3", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    // A queue reset (ethtool -L) re-applies the driver's hints...
    std::fs::write(root.join("proc/irq/16/smp_affinity_list"), "1\n")?;
    // ...and the next pin puts the queues back on the loop CPU, the rest on
    // housekeeping.
    let o = vm_setup(&root, "pin", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    for irq in 15..23 {
        assert_eq!(affinity(&root, irq)?, "3", "irq {irq}");
    }
    assert_eq!(affinity(&root, 11)?, "0,1");
    let o = vm_setup(&root, "cond irqlocal off", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    let o = vm_setup(&root, "pin", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    for irq in 15..23 {
        assert_eq!(affinity(&root, irq)?, "0,1", "irq {irq}");
    }
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn a_queue_change_puts_the_irqs_and_the_moderation_back() -> R {
    let root = fake_root("onequeue")?;
    fake_nic(&root, "on", 20)?;
    let o = vm_setup(&root, "cond nodim on", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    // One queue (ethtool -L): the driver puts the queues' interrupts back on
    // CPU i, the isolated CPUs among them, and adaptive moderation back on.
    // The condition pins both again before anything is measured.
    let o = vm_setup(&root, "cond onequeue on", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    assert!(
        std::fs::read_to_string(root.join("ethtool-state.log"))?.contains("-L ens5 combined 1"),
        "{o:?}"
    );
    for irq in 15..23 {
        assert_eq!(affinity(&root, irq)?, "0,1", "irq {irq}");
    }
    assert_eq!(ethtool_state(&root)?, "off 20");
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn the_idle_poll_boot_profile_is_arm64_only() -> R {
    let root = fake_root("bootline")?;
    let o = vm_setup(&root, "bootline full-nohlt 0,1 2-7", "aarch64")?;
    assert!(o.status.success(), "{o:?}");
    assert_eq!(
        stdout(&o).trim(),
        "isolcpus=managed_irq,domain,2-7 nohz_full=2-7 rcu_nocbs=2-7 irqaffinity=0,1 nohlt"
    );
    let o = vm_setup(&root, "bootline full 0,1 2-7", "aarch64")?;
    assert_eq!(
        stdout(&o).trim(),
        "isolcpus=managed_irq,domain,2-7 nohz_full=2-7 rcu_nocbs=2-7 irqaffinity=0,1"
    );
    // x86 has no `nohlt` (it is idle=poll there, which would spin the loop's
    // hyperthread sibling): refused with the reason.
    let o = vm_setup(&root, "bootline full-nohlt 0,4 1-3,5-7", "x86_64")?;
    assert_eq!(o.status.code(), Some(2), "{o:?}");
    assert!(
        String::from_utf8_lossy(&o.stderr).contains("arm64 only"),
        "{o:?}"
    );
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// `prepare_hosts` with stand-ins for ssh and scp: an aarch64 receiver
/// (Graviton4) and the x86 rig, so the sender builds its own programs. A
/// build job's exit code is `<root>/rc-r` (receiver) or `<root>/rc-s`
/// (sender), absent while it runs; every poll for one appends its host to
/// `<root>/polls`. The first reboot exits the script with 7.
fn prepare_hosts(root: &Path) -> Result<Output, std::io::Error> {
    let t = root.display();
    bash(
        &format!(
            r#"source {lib}; T={t}
log() {{ echo "$*" >> "$T/log"; }}
wait_ssh() {{ :; }}
scp() {{ :; }}
sleep() {{ command sleep 0.05; }}
host() {{
  case $2 in
    *'uname -m'*) if [[ $1 == r ]]; then echo "aarch64 0x00000000410fd4f1 Neoverse-V2"; else echo "x86_64 - Intel(R) Xeon(R) Platinum 8488C"; fi ;;
    *lscpu*) if [[ $1 == r ]]; then printf '0,0\n1,1\n2,2\n3,3\n4,4\n5,5\n6,6\n7,7\n'; else printf '0,0\n1,1\n2,0\n3,1\n'; fi ;;
    *'cat ~/jobs/'*.rc*) echo "$1" >> "$T/polls"; cat "$T/rc-$1" 2> /dev/null ;;
    *'cat ~/jobs/'*.log*) echo "build output $1" ;;
    *reboot*) exit 7 ;;
    *) : ;;
  esac
}}
rssh() {{ host r "$1"; }}
sssh() {{ host s "$1"; }}
OUT=$T SSHO=() RUSER=u RCV=r SND=s
prepare_hosts"#,
            lib = script("lib-cloud.sh").display()
        ),
        &[],
    )
}

fn polls(root: &Path, host: &str) -> usize {
    std::fs::read_to_string(root.join("polls"))
        .unwrap_or_default()
        .lines()
        .filter(|l| *l == host)
        .count()
}

/// A run that stops during the build (the receiver's failed, or an
/// interrupt) leaves nothing behind still polling the sender's build: the
/// teardown deletes the hosts, and a poller left running would retry its
/// ssh to them for ever.
#[test]
fn a_failed_receiver_build_leaves_no_poller_behind() -> R {
    let root = std::env::temp_dir().join(format!("prepare-fail-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    std::fs::write(root.join("rc-r"), "1\n")?;
    let o = prepare_hosts(&root)?;
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let at_exit = polls(&root, "s");
    std::thread::sleep(std::time::Duration::from_millis(500));
    let later = polls(&root, "s");
    // A poller left behind finishes (and the directory goes) before the
    // verdict.
    std::fs::write(root.join("rc-s"), "0\n")?;
    let mut n = polls(&root, "s");
    for _ in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(200));
        let m = polls(&root, "s");
        if m == n {
            break;
        }
        n = m;
    }
    std::fs::remove_dir_all(&root)?;
    assert_eq!(
        later, at_exit,
        "the sender's build was still being polled after the run stopped"
    );
    Ok(())
}

/// Both builds run at once; the sender's is waited for before the reboot,
/// and its failure stops the run.
#[test]
fn the_senders_own_build_is_waited_for() -> R {
    let root = std::env::temp_dir().join(format!("prepare-ok-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    std::fs::write(root.join("rc-r"), "0\n")?;
    std::fs::write(root.join("rc-s"), "0\n")?;
    let o = prepare_hosts(&root)?;
    assert_eq!(o.status.code(), Some(7), "{o:?}");
    let log = std::fs::read_to_string(root.join("log"))?;
    assert!(
        log.contains("receiver build RUSTFLAGS='-Ctarget-cpu=neoverse-v2'; sender ''"),
        "{log}"
    );
    assert!(
        log.contains("and, for its own architecture or core, the sender"),
        "{log}"
    );
    assert!(log.contains("rebooting for CPU isolation"), "{log}");
    assert_eq!(
        std::fs::read_to_string(root.join("build-snd.txt"))?,
        "build output s\n"
    );
    std::fs::remove_dir_all(&root)?;

    std::fs::create_dir_all(&root)?;
    std::fs::write(root.join("rc-r"), "0\n")?;
    std::fs::write(root.join("rc-s"), "1\n")?;
    let o = prepare_hosts(&root)?;
    assert_eq!(o.status.code(), Some(1), "{o:?}");
    let log = std::fs::read_to_string(root.join("log"))?;
    assert!(log.contains("sender build failed"), "{log}");
    assert!(!log.contains("rebooting"), "{log}");
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// One `traced_run` (`lib-matrix.sh`) with stand-ins for the hosts: what each
/// was asked to run, in order (`<root>/calls`), and the run's log. Each
/// host's `vm-setup.sh pin` prints `rcv_pin` or `snd_pin`.
fn traced_run(
    root: &Path,
    rcv_pin: &str,
    snd_pin: &str,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let o = bash(
        &format!(
            r#"T={t}
log() {{ echo "$*" >> "$T/log"; }}
sssh() {{ echo "sender: $1" >> "$T/calls"; if [[ $1 == *'vm-setup.sh pin'* ]]; then echo '{snd_pin}'; fi; }}
rrun() {{ echo "receiver: $1" >> "$T/calls"; if [[ $1 == *'vm-setup.sh pin'* ]]; then echo '{rcv_pin}'; fi; }}
source {lib}
OUT=$T CPU_A=2 CPU_SPARE=3 S_CPU=1; mkdir -p "$T/logs"
traced_run r1-base-0/pp-recv-busy-0 'MEASURED --cpu 2' echo 10"#,
            t = root.display(),
            lib = script("lib-matrix.sh").display()
        ),
        &[],
    )?;
    assert!(o.status.success(), "{o:?}");
    Ok((
        std::fs::read_to_string(root.join("calls"))?,
        std::fs::read_to_string(root.join("log")).unwrap_or_default(),
    ))
}

#[test]
fn every_run_pins_both_hosts_first_and_logs_any_drift() -> R {
    let root = std::env::temp_dir().join(format!("traced-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    let quiet = "pin: 0 of 12 IRQs moved to 0,1; 2 cannot move (per-CPU or kernel-managed)";
    let (calls, log) = traced_run(&root, quiet, quiet)?;
    // The sender is pinned before the receiver's job starts, and the
    // receiver's job pins before it measures.
    let snd =
        calls.find("sender: mkdir -p srvres/r1-base-0/pp-recv-busy-0; sudo bash vm-setup.sh pin");
    let rcv = calls.find("receiver: ");
    assert!(snd.is_some() && rcv.is_some() && snd < rcv, "{calls}");
    let job = &calls[rcv.unwrap_or_default()..];
    let (pin, measured) = (
        job.find("sudo bash vm-setup.sh pin"),
        job.find("MEASURED --cpu 2"),
    );
    assert!(
        pin.is_some() && measured.is_some() && pin < measured,
        "{job}"
    );
    assert!(!log.contains("drifted"), "{log}");
    std::fs::remove_dir_all(&root)?;

    std::fs::create_dir_all(&root)?;
    let (_, log) = traced_run(
        &root,
        "pin: 8 of 12 IRQs moved to 0,1; 2 cannot move (per-CPU or kernel-managed)",
        "pin: ens5 adaptive-rx was on, set off again",
    )?;
    assert!(
        log.contains("run r1-base-0/pp-recv-busy-0: the receiver's IRQs or NIC moderation had drifted and were pinned again: pin: 8 of 12"),
        "{log}"
    );
    assert!(
        log.contains("run r1-base-0/pp-recv-busy-0: the sender's IRQs or NIC moderation had drifted and were pinned again: pin: ens5 adaptive-rx was on"),
        "{log}"
    );
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

/// A stand-in `chronyc` on `<root>/bin`: `-n tracking` and `-n sources` as
/// chrony 4 prints them.
fn fake_chronyc(root: &Path) -> Result<(), std::io::Error> {
    let bin = root.join("bin");
    std::fs::create_dir_all(&bin)?;
    std::fs::write(
        bin.join("chronyc"),
        "#!/usr/bin/env bash\n\
         case \"$*\" in\n\
           '-n tracking') printf 'Reference ID    : A9FEA97B (169.254.169.123)\\n\
System time     : 0.000000728 seconds slow of NTP time\\n\
Root delay      : 0.000171096 seconds\\n\
Root dispersion : 0.000031781 seconds\\n\
Leap status     : Normal\\n' ;;\n\
           '-n sources') printf '^* 169.254.169.123   1   4   77   2   -10us[  -13us] +/-  105us\\n' ;;\n\
           *) exit 1 ;;\n\
         esac\n",
    )?;
    let mut perm = std::fs::metadata(bin.join("chronyc"))?.permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perm, 0o755);
    std::fs::set_permissions(bin.join("chronyc"), perm)
}

#[test]
fn the_clock_command_keeps_chronys_tracking_and_sources() -> R {
    let root = std::env::temp_dir().join(format!("vm-clock-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    fake_chronyc(&root)?;
    let out = root.join("clock-before.txt");
    let o = vm_setup(&root, &format!("clock {}", out.display()), "x86_64")?;
    assert!(o.status.success(), "{o:?}");
    let text = std::fs::read_to_string(&out)?;
    for part in [
        "== tracking",
        "System time     : 0.000000728",
        "== sources",
        "169.254.169.123",
    ] {
        assert!(text.contains(part), "{part:?} not in {text}");
    }
    // What the analysis reads back: |System time| + ½ root delay + root
    // dispersion.
    assert_eq!(
        io_bench::stalls::chrony_bound_ns(&text),
        Some(728 + 85_548 + 31_781)
    );
    std::fs::remove_dir_all(&root)?;
    Ok(())
}

#[test]
fn every_run_keeps_both_hosts_chrony_logs() -> R {
    let root = std::env::temp_dir().join(format!("traced-clock-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root)?;
    let quiet = "pin: 0 of 12 IRQs moved to 0,1; 2 cannot move (per-CPU or kernel-managed)";
    let (calls, _) = traced_run(&root, quiet, quiet)?;
    let at = |s: &str| calls.find(s);
    let dir = "r1-base-0/pp-recv-busy-0";
    let snd_before = at(&format!(
        "vm-setup.sh clock srvres/{dir}/clock-before-sender.txt"
    ));
    let rcv = at("receiver: ");
    let snd_after = at(&format!(
        "vm-setup.sh clock srvres/{dir}/clock-after-sender.txt"
    ));
    assert!(snd_before.is_some() && snd_before < rcv, "{calls}");
    assert!(snd_after.is_some() && snd_after > rcv, "{calls}");
    let job = &calls[rcv.unwrap_or_default()..];
    let before = job.find(&format!("vm-setup.sh clock res/{dir}/clock-before.txt"));
    let measured = job.find("MEASURED --cpu 2");
    let after = job.find(&format!("vm-setup.sh clock res/{dir}/clock-after.txt"));
    assert!(before.is_some() && before < measured, "{job}");
    assert!(after.is_some() && after > measured, "{job}");
    std::fs::remove_dir_all(&root)?;
    Ok(())
}
