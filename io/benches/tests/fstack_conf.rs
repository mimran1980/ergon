//! `fstack_lat`'s flags, the F-Stack config it writes and FreeBSD's
//! `tcp_info` decoding (`io_bench::transport::fstack_conf`), without
//! F-Stack. The arm end to end: io/benches/fstack/docker-test.sh.

use io_bench::transport::fstack_conf::{BsdTcpInfo, FstackArgs, netmask};
use io_bench::transport::point::{Flags, Role};

fn flags(s: &str) -> Flags {
    Flags::parse(s.split_whitespace().map(str::to_owned)).unwrap()
}

const PONG: &str =
    "--listen 10.30.1.12:22000 --prefix-len 24 --gateway 10.30.1.1 --pci 0000:00:06.0";
const PING: &str = "--source 10.30.1.11 --connect 10.30.1.12:22000 --prefix-len 24 --gateway 10.30.1.1 --pci 0000:00:06.0";

#[test]
fn pong_takes_its_address_from_listen_and_its_lcore_from_cpus() {
    let mut f = flags(PONG);
    let a = FstackArgs::parse(&mut f, Role::Pong, &[3]).unwrap();
    f.finish().unwrap();
    assert_eq!(a.conf.addr.to_string(), "10.30.1.12");
    assert_eq!(a.conf.lcore, 3);
    let conf = a.conf.render();
    for line in [
        "[dpdk]",
        "lcore_mask=8",
        "allow=0000:00:06.0",
        "pkt_tx_delay=0",
        "idle_sleep=0",
        "idle_loop_interval=0",
        "promiscuous=0",
        "tso=0",
        "lro=0",
        "thread_mode=0",
        "[port0]",
        "addr=10.30.1.12",
        "netmask=255.255.255.0",
        "broadcast=10.30.1.255",
        "gateway=10.30.1.1",
        "[kni]",
        "[freebsd.boot]",
        "hz=100",
        "net.inet.tcp.delayed_ack=1",
        "net.inet.tcp.sack.enable=1",
    ] {
        assert!(conf.lines().any(|l| l == line), "{line} not in\n{conf}");
    }
    assert!(!conf.contains("extra_eal_args"));
}

#[test]
fn ping_takes_its_address_from_source() {
    let mut f = flags(PING);
    let a = FstackArgs::parse(&mut f, Role::Ping, &[2]).unwrap();
    f.finish().unwrap();
    assert_eq!(a.conf.addr.to_string(), "10.30.1.11");
    assert_eq!(a.connect.unwrap().to_string(), "10.30.1.12:22000");
}

#[test]
fn every_rendered_key_is_a_manifest_setting() {
    let mut f = flags(&format!("{PONG} --hz 1000 --delayed-ack false"));
    let a = FstackArgs::parse(&mut f, Role::Pong, &[1]).unwrap();
    let s = a.conf.settings();
    let get = |k: &str| s.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
    assert_eq!(get("fstack.conf.dpdk.pkt_tx_delay"), Some("0"));
    assert_eq!(get("fstack.conf.freebsd.boot.hz"), Some("1000"));
    assert_eq!(
        get("fstack.conf.freebsd.sysctl.net.inet.tcp.delayed_ack"),
        Some("0")
    );
    let keys = a.conf.render().lines().filter(|l| l.contains('=')).count();
    assert_eq!(s.len(), keys);
}

#[test]
fn a_vdev_replaces_the_pci_port() {
    let mut f = flags(
        "--listen 10.9.0.2:22000 --prefix-len 24 --gateway 10.9.0.254 --vdev net_af_packet0,iface=vb --no-huge",
    );
    let a = FstackArgs::parse(&mut f, Role::Pong, &[1]).unwrap();
    let conf = a.conf.render();
    assert!(conf.contains("extra_eal_args=--no-pci --vdev=net_af_packet0,iface=vb\n"));
    assert!(conf.contains("no_huge=1\n"));
    assert!(!conf.contains("allow="));
}

#[test]
fn refuses_what_it_cannot_run() {
    let refused = |args: &str, role: Role, cpus: &[usize]| {
        let mut f = flags(args);
        FstackArgs::parse(&mut f, role, cpus)
            .unwrap_err()
            .to_string()
    };
    // one F-Stack per process: no local role
    assert!(refused(PONG, Role::Local, &[1, 2]).contains("no local"));
    // exactly one lcore
    assert!(refused(PONG, Role::Pong, &[]).contains("exactly one CPU"));
    assert!(refused(PONG, Role::Pong, &[1, 2]).contains("exactly one CPU"));
    // the connecting end's address is --source
    assert!(
        refused(
            "--connect 10.30.1.12:22000 --prefix-len 24 --gateway 10.30.1.1 --pci 0000:00:06.0",
            Role::Ping,
            &[2]
        )
        .contains("--source")
    );
    // a port, either PCI or virtual, not both
    assert!(
        refused(
            "--listen 10.30.1.12:22000 --prefix-len 24 --gateway 10.30.1.1",
            Role::Pong,
            &[1]
        )
        .contains("--pci")
    );
    assert!(refused(&format!("{PONG} --vdev net_ring0"), Role::Pong, &[1]).contains("one of"));
    // --nodelay is always on
    assert!(refused(&format!("{PONG} --nodelay false"), Role::Pong, &[1]).contains("always on"));
    // the media driver's wildcard ports
    assert!(
        refused(
            "--listen 10.30.1.12:20500 --prefix-len 24 --gateway 10.30.1.1 --pci 0000:00:06.0",
            Role::Pong,
            &[1]
        )
        .contains("20000-21999")
    );
    assert!(
        refused(
            "--listen 10.30.1.12:22000 --prefix-len 33 --gateway 10.30.1.1 --pci 0000:00:06.0",
            Role::Pong,
            &[1]
        )
        .contains("--prefix-len")
    );
}

#[test]
fn the_kernel_socket_flags_are_left_for_finish_to_refuse() {
    let mut f = flags(&format!("{PONG} --busy-read-us 50"));
    FstackArgs::parse(&mut f, Role::Pong, &[1]).unwrap();
    assert!(
        f.finish()
            .unwrap_err()
            .to_string()
            .contains("--busy-read-us")
    );
}

#[test]
fn netmasks() {
    assert_eq!(netmask(24).to_string(), "255.255.255.0");
    assert_eq!(netmask(20).to_string(), "255.255.240.0");
    assert_eq!(netmask(32).to_string(), "255.255.255.255");
}

/// FreeBSD 15's `struct tcp_info` laid out by hand from netinet/tcp.h
/// (F-Stack v2.0's freebsd/): 8 bytes of u8 fields, then u32s.
#[test]
fn tcp_info_offsets_follow_freebsd_15() {
    let mut b = [0u8; 512];
    b[0] = 4; // tcpi_state: ESTABLISHED
    b[5] = 0b111; // tcpi_options: timestamps, SACK, window scaling
    let mut put = |at: usize, v: u32| b[at..at + 4].copy_from_slice(&v.to_ne_bytes());
    put(8, 201_000); // tcpi_rto
    put(16, 1448); // tcpi_snd_mss
    put(68, 37); // tcpi_rtt
    put(80, 14_480); // tcpi_snd_cwnd
    put(120, 3); // tcpi_snd_rexmitpack
    put(124, 2); // tcpi_rcv_ooopack
    put(164, 1); // tcpi_total_tlp
    let i = BsdTcpInfo::parse(&b).unwrap();
    assert_eq!(
        i,
        BsdTcpInfo {
            state: 4,
            options: 7,
            rto_us: 201_000,
            snd_mss: 1448,
            rtt_us: 37,
            snd_cwnd: 14_480,
            rexmit_packets: 3,
            ooo_packets: 2,
            tlp: 1,
        }
    );
    assert!(BsdTcpInfo::parse(&b[..100]).is_err());
}

/// What transport.sh `tr_bin_args fstack_lat` and `tr_point` pass, flag for
/// flag (test-transport-runner.sh checks the runner's side of it): every
/// one taken, none left.
#[test]
fn takes_exactly_the_runners_flags() {
    use io_bench::transport::point::Common;
    let point = "--streams 8 --rate 50000 --warmup-s 5 --duration-s 30 --size 64 --label d8:dpdk-tcp@STREAMS=8 \
                 --spike-ns 100000 --stall-ns 10000 --cpus 2 --out /tmp/x";
    for (role, ends) in [
        (
            "ping",
            "--source 10.30.1.11 --connect 10.30.1.22:22000 --pci 0000:00:06.0 --prefix-len 24 --gateway 10.30.1.1 --nodelay --delayed-ack false --rcvbuf 2097152 --sndbuf 2097152",
        ),
        (
            "pong",
            "--listen 10.30.1.22:22000 --pci 0000:00:07.0 --prefix-len 24 --gateway 10.30.1.1 --nodelay --delayed-ack false --rcvbuf 2097152 --sndbuf 2097152",
        ),
    ] {
        let mut f = flags(&format!("--role {role} {point} {ends}"));
        let c = Common::parse(&mut f).unwrap();
        let a = FstackArgs::parse(&mut f, c.role, &c.cpus).unwrap();
        f.finish().unwrap();
        assert_eq!(a.conf.rcvbuf, 2_097_152);
        assert_eq!(a.conf.sndbuf, Some(2_097_152));
        assert_eq!(a.conf.lcore, 2);
        // the kernel TCP arms' quickack: every segment acknowledged at once
        assert!(!a.conf.delayed_ack);
        assert!(
            a.conf
                .render()
                .lines()
                .any(|l| l == "net.inet.tcp.delayed_ack=0")
        );
    }
}

/// The runner's `--impl ergon` line, through `spin_config`: fstack-spin,
/// `rexmit_min` 8 ms, the kernel RTO left unset.
#[test]
fn spin_config_follows_the_runner_ergon_line() {
    use io_bench::transport::fstack_conf::{FstackImpl, spin_config};
    use io_bench::transport::point::Common;
    let point = "--streams 8 --rate 50000 --warmup-s 5 --duration-s 30 --size 64 --label d8:io-tcp-fstack@STREAMS=8 \
                 --spike-ns 100000 --stall-ns 10000 --cpus 2 --out /tmp/x";
    let ends = "--source 10.30.1.11 --connect 10.30.1.22:22000 --pci 0000:00:06.0 --prefix-len 24 \
                --gateway 10.30.1.1 --nodelay --delayed-ack false --rcvbuf 2097152 --sndbuf 2097152 \
                --impl ergon --rto-min-us 8000";
    let mut f = flags(&format!("--role ping {point} {ends}"));
    let c = Common::parse(&mut f).unwrap();
    let a = FstackArgs::parse(&mut f, c.role, &c.cpus).unwrap();
    f.finish().unwrap();
    assert_eq!(a.imp, FstackImpl::Ergon);
    let cfg = spin_config(&a, std::path::Path::new("fstack-spin.conf")).unwrap();
    assert_eq!(cfg.backend.name(), "fstack-spin");
    assert!(cfg.validate().is_ok());
    assert!(
        cfg.fstack
            .sysctl
            .iter()
            .any(|(k, v)| k == "net.inet.tcp.rexmit_min" && v == "8")
    );
    assert_eq!(a.conf.hz, 1000);
    assert_eq!(cfg.fstack.hz, 1000);
    assert!(u64::from(8u32) * u64::from(cfg.fstack.hz) / 1000 >= 1);
    assert!(
        a.conf.render().lines().any(|l| l == "hz=1000"),
        "{}",
        a.conf.render()
    );
    assert_eq!(cfg.tcp_rto_min_us, None);
    assert_eq!(
        cfg.fstack.addr,
        Some(std::net::Ipv4Addr::new(10, 30, 1, 11))
    );
    assert!(!cfg.fstack.delayed_ack);
    assert_eq!(
        cfg.fstack.conf_path.as_deref(),
        Some(std::path::Path::new("fstack-spin.conf"))
    );
    let mut low = flags(&format!("--role ping {point} {ends} --hz 100"));
    let low_c = Common::parse(&mut low).unwrap();
    let low = FstackArgs::parse(&mut low, low_c.role, &low_c.cpus).unwrap();
    assert_eq!(low.conf.hz, 1000, "explicit --hz 100 cannot store 8 ms");
    assert_eq!(
        spin_config(&low, std::path::Path::new("fstack-spin.conf"))
            .unwrap()
            .fstack
            .hz,
        1000
    );
    let mut direct = flags(&format!("--role ping {PING}"));
    let direct = FstackArgs::parse(&mut direct, Role::Ping, &[2]).unwrap();
    let err = spin_config(&direct, std::path::Path::new("fstack-spin.conf")).unwrap_err();
    assert!(err.to_string().contains("fstack-spin"), "{err}");
}

#[test]
fn a_pci_port_needs_sys_admin_and_dac_override_before_ff_init() {
    let mut f = flags(PONG);
    let a = FstackArgs::parse(&mut f, Role::Pong, &[2]).unwrap();
    // a plain user (the AWS run's: the EAL stopped with "Cannot use IOVA as 'PA'")
    let e = a
        .conf
        .check_caps("Name:\tfstack_lat\nCapEff:\t0000000000000000\n")
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("CAP_SYS_ADMIN") && e.contains("CAP_DAC_OVERRIDE") && e.contains("0000:00:06.0"),
        "{e}"
    );
    // CAP_DAC_OVERRIDE alone: CAP_SYS_ADMIN still missing
    let e = a
        .conf
        .check_caps("CapEff:\t0000000000000002\n")
        .unwrap_err()
        .to_string();
    assert!(
        e.contains("CAP_SYS_ADMIN") && !e.contains("CAP_DAC_OVERRIDE"),
        "{e}"
    );
    // tr_fstack_priv's set (sys_admin, dac_override, ipc_lock, sys_rawio), and root's
    a.conf.check_caps("CapEff:\t0000000000224002\n").unwrap();
    a.conf.check_caps("CapEff:\t000001ffffffffff\n").unwrap();
    assert!(a.conf.check_caps("Name:\tx\n").is_err());
}

#[test]
fn a_vdev_or_no_huge_end_needs_no_capability() {
    let mut f = flags(
        "--listen 10.9.0.2:22000 --prefix-len 24 --gateway 10.9.0.254 --vdev net_af_packet0,iface=vb --no-huge",
    );
    let a = FstackArgs::parse(&mut f, Role::Pong, &[1]).unwrap();
    a.conf.check_caps("CapEff:\t0000000000000000\n").unwrap();
}
