//! The bypass matrix, from the shipped parsers and the runner's real arm
//! lines (`transport.sh` `tr_bin_args`, `tr_plan`, the sets file). macOS
//! builds this; `tests/transport.rs` is Linux only.
//!
//! `wstls_lat` on `epoll-spin` also runs a one-second loopback point twice.
//! `fstack_lat --impl ergon` is the parser: the binary links F-Stack and is
//! not built here.

use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::Command;

use io_bench::transport::bypass::{self, BypassNic, UdpChoice};
use io_bench::transport::fstack_conf::{FstackArgs, FstackImpl};
use io_bench::transport::point::{Common, Error, Flags};
use io_bench::transport::wstls::{self, WstlsArgs};

fn cloud() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../md-dpdk/benches/cloud")
}

fn runner_lines() -> String {
    let script = r#"
set -euo pipefail
cloud=$1
sets=$cloud/transport-aws-io-bypass.tsv
. "$cloud/transport.sh"
dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
printf 'sender ens6 ens6 0a:00:00:00:00:01 10.30.1.11 24 0000:00:06.0\nreceiver ens7 ens7 0a:00:00:00:00:02 10.30.1.21 24 0000:00:07.0\n' >"$dir/nics-a.txt"
printf 'sender ens6 ens6 0a:00:00:00:00:03 10.30.1.12 24 0000:00:06.0\nreceiver ens7 ens7 0a:00:00:00:00:04 10.30.1.22 24 0000:00:07.0\n' >"$dir/nics-b.txt"
sets_file=$sets tr_sets_file=$sets arms_file='' tr_arms_file=''
echo "LOSS $(tr_plan loss)"
echo "RTO $(tr_knob rto_min_us)"
if tr_wants_fstack; then echo "WANTS yes"; else echo "WANTS no"; fi
sets_file='' tr_sets_file=''
if tr_wants_fstack; then echo "MATRIX yes"; else echo "MATRIX no"; fi
sets_file=$sets tr_sets_file=$sets
tr_plan rounds a 1 1 | awk -F'\t' '{print "ROUND", $5, $7, $8}'
common="--streams 1 --rate 10000 --warmup-s 0 --duration-s 1 --size $TR_PAYLOAD --label bypass --spike-ns $TR_SPIKE_NS --stall-ns $TR_STALL_NS --cpus 2 --out $dir/out"
for arm in io-tcp-epoll io-tcp-smol io-tcp-fstack io-ws-epoll io-ws-smol io-ws-fstack io-udp-epoll io-udp-smol; do
    spec=$(tr_plan arm "$arm")
    echo "ARM $arm $spec"
    read -r bin setup _ rest <<<"$spec"
    if [[ -n ${rest:-} ]]; then read -ra args <<<"$rest"; else args=(); fi
    echo "FULL $arm --role ping $common $(tr_bin_args "$bin" ping 10.30.1.11 10.30.1.22 "${args[@]}") $(tr_alt_args a sender b receiver)"
done
args=() bin=fstack_lat setup=fstack
echo "DIRECT --role ping $common $(tr_bin_args fstack_lat ping 10.30.1.11 10.30.1.22)"
"#;
    let out = Command::new("bash")
        .arg("-c")
        .arg(script)
        .arg("io-bypass")
        .arg(cloud())
        .output()
        .expect("bash");
    assert!(
        out.status.success(),
        "runner: {}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("utf8")
}

fn line<'a>(text: &'a str, prefix: &str) -> &'a str {
    text.lines()
        .find(|l| l.starts_with(prefix))
        .unwrap_or_else(|| panic!("missing {prefix} in runner output:\n{text}"))
}

fn argv(line: &str) -> Vec<String> {
    // "FULL <arm> <argv...>" or "DIRECT <argv...>"
    let rest = line.split_once(' ').expect("prefix").1;
    let rest = rest
        .strip_prefix("io-")
        .map_or(rest, |r| r.split_once(' ').expect("arm").1);
    rest.split_whitespace().map(str::to_owned).collect()
}

fn parse_fstack(args: &[String]) -> FstackArgs {
    let mut f = Flags::parse(args.iter().cloned()).expect("flags");
    let c = Common::parse(&mut f).expect("common");
    let a = FstackArgs::parse(&mut f, c.role, &c.cpus).expect("fstack");
    f.finish().expect("no flag left");
    a
}

fn parse_wstls(args: &[String]) -> WstlsArgs {
    let mut f = Flags::parse(args.iter().cloned()).expect("flags");
    let _c = Common::parse(&mut f).expect("common");
    let a = WstlsArgs::parse(&mut f).expect("wstls");
    f.finish().expect("no flag left");
    a
}

#[test]
fn the_runner_selects_fstack_spin_wstls_and_the_bypass_matrix() {
    let text = runner_lines();
    assert_eq!(line(&text, "LOSS "), "LOSS kernel-peer");
    assert_eq!(line(&text, "RTO "), "RTO 8000");
    assert_eq!(line(&text, "WANTS "), "WANTS yes");
    assert_eq!(line(&text, "MATRIX "), "MATRIX no");
    let rounds: Vec<&str> = text.lines().filter(|l| l.starts_with("ROUND ")).collect();
    assert_eq!(rounds.len(), 3, "{text}");
    for r in &rounds {
        let mut p = r.split_whitespace();
        assert_eq!(p.next(), Some("ROUND"));
        let netem = p.next().unwrap();
        assert!(netem == "0" || netem == "0.1" || netem == "1", "{r}");
        assert_eq!(p.next(), Some("10000,50000"), "{r}");
        assert_eq!(p.next(), Some("1,8"), "{r}");
    }

    assert!(
        line(&text, "ARM io-tcp-fstack ").contains("fstack_spin_lat"),
        "{}",
        line(&text, "ARM io-tcp-fstack ")
    );
    let fstack = parse_fstack(&argv(line(&text, "FULL io-tcp-fstack ")));
    assert_eq!(fstack.imp, FstackImpl::Ergon);
    assert_eq!(fstack.imp.backend(), "fstack-spin");
    assert_eq!(fstack.rto_min_us, Some(8000));
    assert_eq!(fstack.conf.rexmit_min_ms, Some(8));
    let cfg = io_bench::transport::fstack_conf::spin_config(
        &fstack,
        std::path::Path::new("fstack-spin.conf"),
    )
    .expect("spin_config");
    assert_eq!(cfg.backend.name(), "fstack-spin");
    assert!(cfg.validate().is_ok());
    assert!(
        cfg.fstack
            .sysctl
            .iter()
            .any(|(k, v)| k == "net.inet.tcp.rexmit_min" && v == "8"),
        "{:?}",
        cfg.fstack.sysctl
    );
    assert_eq!(cfg.tcp_rto_min_us, None);
    assert_eq!(
        cfg.fstack.addr,
        Some(std::net::Ipv4Addr::new(10, 30, 1, 11))
    );
    assert!(!cfg.fstack.delayed_ack);
    assert_eq!(fstack.conf.hz, 1000, "8 ms is 0 ticks at 100 Hz");
    assert!(
        u64::from(8u32) * u64::from(cfg.fstack.hz) / 1000 >= 1,
        "hz {} does not store rexmit_min=8",
        cfg.fstack.hz
    );
    assert_eq!(cfg.fstack.hz, 1000);
    assert!(
        fstack
            .conf
            .render()
            .lines()
            .any(|l| l == "net.inet.tcp.rexmit_min=8"),
        "{}",
        fstack.conf.render()
    );
    assert!(
        fstack.conf.render().lines().any(|l| l == "hz=1000"),
        "{}",
        fstack.conf.render()
    );

    let direct = parse_fstack(&argv(line(&text, "DIRECT ")));
    assert_eq!(direct.imp, FstackImpl::Direct);
    assert_eq!(direct.imp.backend(), "fstack");
    assert_eq!(direct.rto_min_us, None);
    assert_eq!(direct.conf.rexmit_min_ms, None);

    for (arm, echo, backend) in [
        ("io-tcp-epoll", "tcp", "epoll-spin"),
        ("io-ws-epoll", "wstls", "epoll-spin"),
        ("io-tcp-smol", "tcp", "smoltcp-spin"),
        ("io-ws-smol", "wstls", "smoltcp-spin"),
        ("io-ws-fstack", "wstls", "fstack-spin"),
    ] {
        let args = argv(line(&text, &format!("FULL {arm} ")));
        let a = parse_wstls(&args);
        assert_eq!(a.echo.name(), echo, "{arm}");
        assert_eq!(a.backend.name(), backend, "{arm}");
        assert_eq!(a.rto_min_us, Some(8000), "{arm}");
        if backend == "epoll-spin" {
            assert!(
                !args.iter().any(|s| s == "--pci" || s == "--source"),
                "{arm} must stay a kernel line: {args:?}"
            );
        } else {
            let err = wstls::run(args).expect_err(arm);
            assert!(
                err.to_string().contains(backend),
                "{arm}: {err} does not name {backend}"
            );
        }
    }

    let udp: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("ARM io-udp"))
        .collect();
    assert_eq!(udp.len(), 2, "{text}");
    assert!(
        udp.iter().any(|l| l.contains("--backend epoll-spin")),
        "{udp:?}"
    );
    assert!(
        udp.iter().any(|l| l.contains("--backend smoltcp-spin")),
        "{udp:?}"
    );
    assert!(udp.iter().all(|l| !l.contains("fstack")), "{udp:?}");
}

#[test]
fn runner_lines_build_the_bypass_stack_configs() {
    let text = runner_lines();
    for arm in ["io-tcp-smol", "io-ws-smol"] {
        let args = argv(line(&text, &format!("FULL {arm} ")));
        let mut f = Flags::parse(args.iter().cloned()).expect("flags");
        let _c = Common::parse(&mut f).expect("common");
        let a = WstlsArgs::parse(&mut f).expect("wstls");
        f.finish().expect("no flag left");
        let cfg = bypass::stack_config(&a.nic().expect("nic")).expect("stack");
        assert_eq!(cfg.rto_min_us, 8000, "{arm}");
        assert_eq!(cfg.ip, Ipv4Addr::new(10, 30, 1, 11), "{arm}");
        assert_eq!(cfg.gateway, Some(Ipv4Addr::new(10, 30, 1, 1)), "{arm}");
        assert_eq!(
            cfg.eal_args,
            vec!["-a".to_owned(), "0000:00:06.0".to_owned()],
            "{arm}"
        );
        assert_eq!(cfg.port, "0000:00:06.0", "{arm}");
        assert!(cfg.tcp_timestamps, "{arm}");
    }

    let args = argv(line(&text, "FULL io-ws-fstack "));
    let mut f = Flags::parse(args.iter().cloned()).expect("flags");
    let c = Common::parse(&mut f).expect("common");
    let a = WstlsArgs::parse(&mut f).expect("wstls");
    f.finish().expect("no flag left");
    let inp = a.fstack_in(&c.cpus).expect("fstack_in");
    let cfg = bypass::fstack_spin_config(&inp, std::path::Path::new("fstack-spin.conf"))
        .expect("fstack config");
    assert_eq!(cfg.backend.name(), "fstack-spin");
    assert_eq!(cfg.tcp_rto_min_us, None);
    assert!(!cfg.fstack.delayed_ack);
    assert_eq!(cfg.fstack.addr, Some(Ipv4Addr::new(10, 30, 1, 11)));
    assert!(
        cfg.fstack
            .sysctl
            .iter()
            .any(|(k, v)| k == "net.inet.tcp.rexmit_min" && v == "8"),
        "{:?}",
        cfg.fstack.sysctl
    );
    assert_eq!(cfg.fstack.hz, 1000);
    assert!(u64::from(8u32) * u64::from(cfg.fstack.hz) / 1000 >= 1);

    let smol = udp_choice_of(&argv(line(&text, "FULL io-udp-smol ")));
    let err = smol.expect_err("smoltcp udp measures only with smoltcp-dpdk");
    assert!(err.to_string().contains("smoltcp-spin"), "{err}");
    let epoll = udp_choice_of(&argv(line(&text, "FULL io-udp-epoll "))).expect("kernel udp");
    assert_eq!(epoll, UdpChoice::Kernel);
}

#[test]
fn stack_config_names_a_vdev_port() {
    let nic = BypassNic {
        ip: Ipv4Addr::new(10, 0, 0, 2),
        prefix_len: 24,
        gateway: Ipv4Addr::new(10, 0, 0, 1),
        pci: None,
        vdev: Some("net_af_packet0,iface=eth1".to_owned()),
        rto_min_us: 8000,
    };
    let cfg = bypass::stack_config(&nic).expect("vdev");
    assert_eq!(cfg.port, "net_af_packet0");
    assert_eq!(
        cfg.eal_args,
        vec!["--vdev=net_af_packet0,iface=eth1".to_owned()]
    );
    assert_eq!(cfg.rto_min_us, 8000);
}

#[test]
fn the_bench_tls_client_passes_the_websocket_dial_check() {
    let cfg = wstls::bench_client_config().expect("bench client");
    assert!(cfg.enable_secret_extraction);
    assert!(
        cfg.alpn_protocols.is_empty(),
        "the kernel peer offers no ALPN: {:?}",
        cfg.alpn_protocols
    );
    let dial = ergon_io::WsDial::<std::net::TcpStream>::on_stack(
        ergon_io::Target::new("localhost", 22000, "/"),
        std::sync::Arc::clone(&cfg),
        ergon_io::WsOptions::default(),
    );
    let err = dial
        .as_ref()
        .err()
        .map(ToString::to_string)
        .unwrap_or_default();
    assert!(dial.is_ok(), "{err}");
    assert!(!err.contains("enable_secret_extraction"), "{err}");

    let provider = std::sync::Arc::new(rustls::crypto::ring::default_provider());
    let bare = rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .expect("tls 1.3")
        .with_root_certificates(rustls::RootCertStore::empty())
        .with_no_client_auth();
    let rejected = ergon_io::WsDial::<std::net::TcpStream>::on_stack(
        ergon_io::Target::new("localhost", 22000, "/"),
        std::sync::Arc::new(bare),
        ergon_io::WsOptions::default(),
    );
    let rejected = match rejected {
        Ok(_) => panic!("a client without secret extraction was accepted"),
        Err(e) => e.to_string(),
    };
    assert!(rejected.contains("enable_secret_extraction"), "{rejected}");
}

fn udp_choice_of(args: &[String]) -> Result<UdpChoice, Error> {
    let mut f = Flags::parse(args.iter().cloned())?;
    let c = Common::parse(&mut f)?;
    let imp = f.req("impl")?;
    let local = f.req("local")?.parse().expect("--local from the runner");
    bypass::udp_choice(&mut f, c.role, imp == "ergon", local)
}

fn p50_max(csv: &str) -> (u64, u64) {
    let row = csv
        .lines()
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .expect("results.csv row");
    let f: Vec<&str> = row.split(',').collect();
    assert!(f.len() > 11, "{row}");
    let p50: u64 = f[6].parse().expect("p50");
    let max: u64 = f[11].parse().expect("max");
    (p50, max)
}

fn launch(echo: &str, out: &std::path::Path) -> (u64, u64) {
    let bin = env!("CARGO_BIN_EXE_wstls_lat");
    let output = Command::new(bin)
        .args([
            "--impl",
            "ergon",
            "--echo",
            echo,
            "--backend",
            "epoll-spin",
            "--role",
            "local",
            "--round-trip",
            "--local",
            "127.0.0.1:9",
            "--peer",
            "127.0.0.1:9",
            "--rate",
            "200",
            "--warmup-s",
            "0",
            "--duration-s",
            "1",
            "--size",
            "64",
            "--label",
            "smoke",
            "--out",
        ])
        .arg(out)
        .output()
        .expect("wstls_lat");
    assert!(
        output.status.success(),
        "wstls_lat {echo}: {}\n{}{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let csv = std::fs::read_to_string(out.join("results.csv")).expect("results.csv");
    let (p50, max) = p50_max(&csv);
    assert!(p50 > 0 && max >= p50, "{echo}: p50 {p50} max {max}\n{csv}");
    (p50, max)
}

#[test]
fn wstls_lat_writes_a_row_twice_for_tcp_and_websocket() {
    let root = std::env::temp_dir().join(format!("wstls-lat-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    for echo in ["tcp", "wstls"] {
        let mut rows = Vec::new();
        for n in 0..2 {
            let dir = root.join(format!("{echo}-{n}"));
            rows.push(launch(echo, &dir));
        }
        assert!(rows[0].0 > 0 && rows[1].0 > 0 && rows[0].1 >= rows[0].0 && rows[1].1 >= rows[1].0);
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Ping and pong are separate processes, as the lab runs them. The
/// WebSocket arm's certificate is the same in both, so the handshake
/// completes and the ping writes a row.
#[test]
fn wstls_lat_ping_and_pong_write_a_row_for_tcp_and_websocket() {
    struct Stop(Option<std::process::Child>);
    impl Drop for Stop {
        fn drop(&mut self) {
            if let Some(mut child) = self.0.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }

    let root = std::env::temp_dir().join(format!("wstls-pair-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let bin = env!("CARGO_BIN_EXE_wstls_lat");
    for echo in ["tcp", "wstls"] {
        let port = 41_000 + (std::process::id() % 2_000) * 2 + u32::from(echo == "wstls");
        let addr = format!("127.0.0.1:{port}");
        let dir = root.join(echo);
        std::fs::create_dir_all(&dir).unwrap();
        let mut child = Command::new(bin)
            .args([
                "--impl",
                "ergon",
                "--echo",
                echo,
                "--backend",
                "epoll-spin",
                "--role",
                "pong",
                "--local",
                &addr,
                "--peer",
                &addr,
                "--rate",
                "200",
                "--warmup-s",
                "0",
                "--duration-s",
                "1",
                "--size",
                "64",
                "--label",
                "smoke",
                "--out",
            ])
            .arg(&dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("pong");
        let mut stdout = std::io::BufReader::new(child.stdout.take().expect("pong stdout"));
        let mut stderr = child.stderr.take().expect("pong stderr");
        let stop = Stop(Some(child));
        let mut line = String::new();
        std::io::BufRead::read_line(&mut stdout, &mut line).expect("pong ready");
        assert!(line.contains("ready"), "pong {echo} said {line:?}");
        let ping = Command::new(bin)
            .args([
                "--impl",
                "ergon",
                "--echo",
                echo,
                "--backend",
                "epoll-spin",
                "--role",
                "ping",
                "--local",
                &addr,
                "--peer",
                &addr,
                "--rate",
                "200",
                "--warmup-s",
                "0",
                "--duration-s",
                "1",
                "--size",
                "64",
                "--label",
                "smoke",
                "--out",
            ])
            .arg(&dir)
            .output()
            .expect("ping");
        if !ping.status.success() {
            let mut err = String::new();
            let _ = std::io::Read::read_to_string(&mut stderr, &mut err);
            panic!(
                "ping {echo}: {}\n{}{}\npong: {err}",
                ping.status,
                String::from_utf8_lossy(&ping.stderr),
                String::from_utf8_lossy(&ping.stdout)
            );
        }
        let csv = std::fs::read_to_string(dir.join("results.csv")).expect("results.csv");
        let (p50, max) = p50_max(&csv);
        assert!(p50 > 0 && max >= p50, "{echo}: p50 {p50} max {max}\n{csv}");
        drop(stop);
    }
    let _ = std::fs::remove_dir_all(&root);
}
