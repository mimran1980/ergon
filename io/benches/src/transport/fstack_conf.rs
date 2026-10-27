//! `fstack_lat`'s settings without F-Stack: its flags, the F-Stack config
//! file it writes, and FreeBSD's `struct tcp_info` read back.
//!
//! Pure Rust, so it builds and is tested without F-Stack; the arm itself
//! (`super::fstack`) needs it (`--features fstack`).
//!
//! The binary writes the whole config from its flags (`<out>/f-stack-<role>.conf`)
//! and hands it to `ff_init`, so every F-Stack setting a point ran with is
//! in its manifest (`fstack.conf.*`). Its fixed latency settings:
//!
//! | key | value | why |
//! |---|---|---|
//! | `pkt_tx_delay` | 0 | F-Stack's default holds a small TX burst up to 100 µs |
//! | `idle_sleep` | 0 | no sleep when a pass finds nothing |
//! | `idle_loop_interval` | 0 | (`ff_run`'s user loop only; the bench calls `ff_run_once`) |
//! | `tso`, `lro` | 0 | no segmentation or receive coalescing for 68-byte frames |
//! | `promiscuous` | 0 | the ENA has none (F-Stack #790) |
//! | `[kni] enable` | 0 | no kernel exception path |
//! | `[pcap] enable` | 0 | no capture |
//!
//! The FreeBSD stack's own settings are F-Stack v2.0's shipped `config.ini`
//! values (SACK, timestamps, CUBIC, the default `freebsd` TCP stack), with
//! `hz` and `delayed_ack` taken from flags.
#![deny(missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::fmt::Write as _;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use super::point::{Error, Flags, Role};

/// The F-Stack interface and EAL settings of one end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FstackConf {
    /// The interface's address: the listening end's `--listen` IP, the
    /// connecting end's `--source`.
    pub addr: Ipv4Addr,
    /// `--prefix-len`: the interface's subnet.
    pub prefix_len: u8,
    /// `--gateway`: the subnet's router (a peer off the subnet).
    pub gateway: Ipv4Addr,
    /// `--pci`: the one port's PCI address (`allow=`); `None` with
    /// `--vdev`.
    pub pci: Option<String>,
    /// `--vdev`: a virtual port in the PCI port's place, as DPDK's
    /// `--vdev` spells it (the Docker test's `net_af_packet0,iface=va`);
    /// written as `extra_eal_args=--no-pci --vdev=<spec>`.
    pub vdev: Option<String>,
    /// `--no-huge`: the EAL on anonymous memory (Docker, no hugepages).
    pub no_huge: bool,
    /// `--memory-mb`, default 1024: the EAL's memory.
    pub memory_mb: u32,
    /// `--file-prefix`, default `ergon-fstack`: the EAL's runtime files.
    pub file_prefix: String,
    /// The lcore: the thread's `--cpus` CPU.
    pub lcore: usize,
    /// `--hz`, default 100 (F-Stack's): the FreeBSD clock tick, TCP's
    /// timer granularity.
    pub hz: u32,
    /// `--delayed-ack`, default true (FreeBSD's and F-Stack's).
    pub delayed_ack: bool,
    /// `--rcvbuf`, bytes (`SO_RCVBUF` on every socket).
    pub rcvbuf: u32,
    /// `--sndbuf`, bytes; `None`: F-Stack's `sendspace` (16 KiB).
    pub sndbuf: Option<u32>,
}

/// `fstack_lat`'s own flags, after the common ones.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FstackArgs {
    /// The interface and EAL.
    pub conf: FstackConf,
    /// `--listen` (pong).
    pub listen: Option<SocketAddrV4>,
    /// `--connect` (ping).
    pub connect: Option<SocketAddrV4>,
    /// `--connect-timeout-s`, default 30.
    pub connect_timeout: Duration,
}

fn v4(f: &mut Flags, key: &str) -> Result<Option<SocketAddrV4>, Error> {
    f.take(key)
        .map(|v| match v.parse::<SocketAddr>() {
            Ok(SocketAddr::V4(a)) => Ok(a),
            _ => Err(Error::Usage(format!("--{key} {v}: IPv4:PORT"))),
        })
        .transpose()
}

fn ip(f: &mut Flags, key: &str) -> Result<Option<Ipv4Addr>, Error> {
    f.take(key)
        .map(|v| {
            v.parse()
                .map_err(|_| Error::Usage(format!("--{key} {v}: an IPv4 address")))
        })
        .transpose()
}

impl FstackArgs {
    /// Takes `fstack_lat`'s flags from `f` for `role` on `cpus`.
    ///
    /// `--nodelay` is always on (refused off), as in `tcp_lat`; the kernel's
    /// socket flags (`--busy-read-us`, `--epoll-events`,
    /// `--quickack-per-read`, `--rto-min-us`, `--delack-max-us`) are not
    /// taken: F-Stack has none of them, and `Flags::finish` refuses them.
    ///
    /// # Errors
    /// [`Error::Usage`] for a flag missing, malformed or refused.
    pub fn parse(f: &mut Flags, role: Role, cpus: &[usize]) -> Result<Self, Error> {
        if !f.bool("nodelay", true)? {
            return Err(Error::Usage("--nodelay is always on".into()));
        }
        let listen = v4(f, "listen")?;
        let connect = v4(f, "connect")?;
        let source = ip(f, "source")?;
        let addr = match (role, listen, connect, source) {
            (Role::Pong, Some(l), None, None) => *l.ip(),
            (Role::Ping, None, Some(_), Some(s)) => s,
            (Role::Ping, None, Some(_), None) => {
                return Err(Error::Usage(
                    "--source IP: the connecting end's F-Stack address".into(),
                ));
            }
            (Role::Ping | Role::Pong, ..) => {
                return Err(Error::Usage(
                    "pong: --listen IP:PORT; ping: --source IP --connect IP:PORT".into(),
                ));
            }
            (r, ..) => {
                return Err(Error::Usage(format!(
                    "--role {}: fstack_lat is ping or pong (one F-Stack per process, so no local)",
                    r.name()
                )));
            }
        };
        let &[lcore] = cpus else {
            return Err(Error::Usage(
                "--cpus: exactly one CPU, the F-Stack lcore (ping: ping; pong: echo)".into(),
            ));
        };
        let prefix_len: u8 = f.num("prefix-len", 0)?;
        if !(1..=32).contains(&prefix_len) {
            return Err(Error::Usage("--prefix-len 1..32 is required".into()));
        }
        let gateway = ip(f, "gateway")?.ok_or_else(|| Error::Usage("--gateway IP".into()))?;
        let pci = f.take("pci");
        let vdev = f.take("vdev");
        if pci.is_some() == vdev.is_some() {
            return Err(Error::Usage(
                "one of --pci BDF (the port) or --vdev SPEC (a virtual one)".into(),
            ));
        }
        let sndbuf = match f.take("sndbuf") {
            None => None,
            Some(v) => Some(
                v.parse()
                    .map_err(|_| Error::Usage(format!("--sndbuf {v}: bytes")))?,
            ),
        };
        let conf = FstackConf {
            addr,
            prefix_len,
            gateway,
            pci,
            vdev,
            no_huge: f.bool("no-huge", false)?,
            memory_mb: f.num("memory-mb", 1024)?,
            file_prefix: f
                .take("file-prefix")
                .unwrap_or_else(|| "ergon-fstack".to_owned()),
            lcore,
            hz: f.num("hz", 100)?,
            delayed_ack: f.bool("delayed-ack", true)?,
            rcvbuf: f.num("rcvbuf", super::DEFAULT_RCVBUF)?,
            sndbuf,
        };
        for a in listen.iter().chain(&connect) {
            super::port_plus(SocketAddr::V4(*a), 0)?;
        }
        Ok(Self {
            conf,
            listen,
            connect,
            connect_timeout: Duration::from_secs(f.num("connect-timeout-s", 30)?),
        })
    }
}

/// `prefix_len`'s netmask.
#[must_use]
pub const fn netmask(prefix_len: u8) -> Ipv4Addr {
    let bits = if prefix_len == 0 {
        0
    } else {
        u32::MAX << (32 - prefix_len as u32)
    };
    Ipv4Addr::from_bits(bits)
}

impl FstackConf {
    /// The F-Stack config file (`ff_init --conf`), every key this end runs
    /// with.
    #[must_use]
    pub fn render(&self) -> String {
        let mask = netmask(self.prefix_len);
        let bcast = Ipv4Addr::from_bits(self.addr.to_bits() | !mask.to_bits());
        let mut dpdk: Vec<(&str, String)> = vec![
            ("lcore_mask", format!("{:x}", 1u128 << self.lcore)),
            ("channel", "4".into()),
            ("memory", self.memory_mb.to_string()),
            ("no_huge", u8::from(self.no_huge).to_string()),
            ("file_prefix", self.file_prefix.clone()),
        ];
        if let Some(p) = &self.pci {
            dpdk.push(("allow", p.clone()));
        }
        if let Some(v) = &self.vdev {
            dpdk.push(("extra_eal_args", format!("--no-pci --vdev={v}")));
        }
        dpdk.extend(FIXED_DPDK.iter().map(|&(k, v)| (k, v.to_owned())));
        let port = [
            ("addr", self.addr.to_string()),
            ("netmask", mask.to_string()),
            ("broadcast", bcast.to_string()),
            ("gateway", self.gateway.to_string()),
        ];
        let mut boot = vec![("hz", self.hz.to_string())];
        boot.extend(FREEBSD_BOOT.iter().map(|&(k, v)| (k, v.to_owned())));
        let mut sysctl: Vec<(&str, String)> = FREEBSD_SYSCTL
            .iter()
            .map(|&(k, v)| (k, v.to_owned()))
            .collect();
        sysctl.push((
            "net.inet.tcp.delayed_ack",
            u8::from(self.delayed_ack).to_string(),
        ));
        let off = [("enable", "0".to_owned())];
        let mut s = String::new();
        for (section, keys) in [
            ("dpdk", dpdk.as_slice()),
            ("port0", port.as_slice()),
            ("kni", off.as_slice()),
            ("pcap", off.as_slice()),
            ("freebsd.boot", boot.as_slice()),
            ("freebsd.sysctl", sysctl.as_slice()),
        ] {
            let _ = writeln!(s, "[{section}]");
            for (k, v) in keys {
                let _ = writeln!(s, "{k}={v}");
            }
        }
        s
    }

    /// The manifest's `fstack.conf.<key>` settings: every line of
    /// [`FstackConf::render`], its section in front.
    #[must_use]
    pub fn settings(&self) -> Vec<(String, String)> {
        let mut out = Vec::new();
        let mut section = String::new();
        for line in self.render().lines() {
            if let Some(s) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                s.clone_into(&mut section);
            } else if let Some((k, v)) = line.split_once('=') {
                out.push((format!("fstack.conf.{section}.{k}"), v.to_owned()));
            }
        }
        out
    }
}

/// `[dpdk]` keys every end runs with (the module's table).
pub const FIXED_DPDK: [(&str, &str); 14] = [
    ("port_list", "0"),
    ("nb_vdev", "0"),
    ("nb_bond", "0"),
    ("promiscuous", "0"),
    ("numa_on", "1"),
    ("tx_csum_offoad_skip", "0"),
    ("tso", "0"),
    ("lro", "0"),
    ("vlan_strip", "1"),
    ("idle_sleep", "0"),
    ("pkt_tx_delay", "0"),
    ("idle_loop_interval", "0"),
    ("symmetric_rss", "0"),
    ("thread_mode", "0"),
];

/// `[freebsd.boot]`, F-Stack v2.0's `config.ini` (less `hz`, a flag).
pub const FREEBSD_BOOT: [(&str, &str); 7] = [
    ("fd_reserve", "1024"),
    ("kern.ipc.maxsockets", "262144"),
    ("net.inet.tcp.syncache.hashsize", "4096"),
    ("net.inet.tcp.syncache.bucketlimit", "100"),
    ("net.inet.tcp.tcbhashsize", "65536"),
    ("kern.ncallout", "262144"),
    ("kern.features.inet6", "1"),
];

/// `[freebsd.sysctl]`, F-Stack v2.0's `config.ini` (less `delayed_ack`, a
/// flag).
pub const FREEBSD_SYSCTL: [(&str, &str); 24] = [
    ("kern.ipc.somaxconn", "32768"),
    ("kern.ipc.maxsockbuf", "16777216"),
    ("net.add_addr_allfibs", "1"),
    ("net.link.ether.inet.maxhold", "5"),
    ("net.inet.tcp.fast_finwait2_recycle", "1"),
    ("net.inet.tcp.finwait2_timeout", "5000"),
    ("net.inet.tcp.sendspace", "16384"),
    ("net.inet.tcp.recvspace", "8192"),
    ("net.inet.tcp.cc.algorithm", "cubic"),
    ("net.inet.tcp.sendbuf_max", "16777216"),
    ("net.inet.tcp.recvbuf_max", "16777216"),
    ("net.inet.tcp.sendbuf_auto", "1"),
    ("net.inet.tcp.recvbuf_auto", "1"),
    ("net.inet.tcp.sendbuf_inc", "16384"),
    ("net.inet.tcp.sack.enable", "1"),
    ("net.inet.tcp.blackhole", "1"),
    ("net.inet.tcp.msl", "2000"),
    ("net.inet.tcp.rfc1323", "1"),
    ("net.inet.udp.blackhole", "1"),
    ("net.inet.ip.redirect", "0"),
    ("net.inet.ip.forwarding", "0"),
    ("net.inet.ip.portrange.randomized", "1"),
    ("net.inet.tcp.functions_default", "freebsd"),
    ("net.inet.tcp.hpts.minsleep", "250"),
];

/// FreeBSD's `TCP_INFO` option (`netinet/tcp.h`: 32; Linux's is 11).
pub const FREEBSD_TCP_INFO: i32 = 32;
/// Bytes asked for: FreeBSD 15's `struct tcp_info` and its padding fit.
pub const TCP_INFO_BUF: usize = 512;

/// The fields of FreeBSD 15's `struct tcp_info` the arm reads, by offset
/// (`freebsd/netinet/tcp.h` in F-Stack v2.0: seven `u_int8_t` and a byte
/// of window scales, then `u_int32_t`s).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BsdTcpInfo {
    /// `tcpi_state` (4: established).
    pub state: u8,
    /// `tcpi_options` (1: timestamps, 2: SACK, 4: window scaling).
    pub options: u8,
    /// `tcpi_rto`, µs.
    pub rto_us: u32,
    /// `tcpi_snd_mss`.
    pub snd_mss: u32,
    /// `tcpi_rtt`, µs (smoothed).
    pub rtt_us: u32,
    /// `tcpi_snd_cwnd`, bytes.
    pub snd_cwnd: u32,
    /// `tcpi_snd_rexmitpack`: segments retransmitted.
    pub rexmit_packets: u32,
    /// `tcpi_rcv_ooopack`: segments received out of order.
    pub ooo_packets: u32,
    /// `tcpi_total_tlp`: tail-loss probes sent.
    pub tlp: u32,
}

/// FreeBSD's `TCPI_OPT_TIMESTAMPS`.
pub const TCPI_OPT_TIMESTAMPS: u8 = 1;
/// FreeBSD's `TCPS_ESTABLISHED`.
pub const TCPS_ESTABLISHED: u8 = 4;

fn u32_at(b: &[u8], at: usize) -> Result<u32, Error> {
    b.get(at..at + 4)
        .and_then(|s| s.try_into().ok())
        .map(u32::from_ne_bytes)
        .ok_or_else(|| Error::Transport(format!("tcp_info: {} bytes, none at {at}", b.len())))
}

impl BsdTcpInfo {
    /// Decodes the bytes `getsockopt(TCP_INFO)` returned.
    ///
    /// # Errors
    /// Fewer bytes than the fields read need.
    pub fn parse(b: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            state: *b
                .first()
                .ok_or_else(|| Error::Transport("tcp_info: empty".into()))?,
            options: *b
                .get(5)
                .ok_or_else(|| Error::Transport("tcp_info: short".into()))?,
            rto_us: u32_at(b, 8)?,
            snd_mss: u32_at(b, 16)?,
            rtt_us: u32_at(b, 68)?,
            snd_cwnd: u32_at(b, 80)?,
            rexmit_packets: u32_at(b, 120)?,
            ooo_packets: u32_at(b, 124)?,
            tlp: u32_at(b, 164)?,
        })
    }
}

/// `fstack_lat`'s usage.
pub const FSTACK_USAGE: &str = "\
fstack_lat --role ping|pong --rate MSG_PER_S --label SET:ARM --out DIR --cpus CPU
        (pong: --listen IP:PORT | ping: --source IP --connect IP:PORT)
        --prefix-len N --gateway IP (--pci BDF | --vdev SPEC)
        [--streams 1] [--warmup-s 30] [--duration-s 30] [--size 64] [--spike-ns 100000]
        [--stall-ns 10000] [--rcvbuf 4194304] [--sndbuf BYTES] [--nodelay] [--delayed-ack true]
        [--hz 100] [--no-huge] [--memory-mb 1024] [--file-prefix ergon-fstack] [--connect-timeout-s 30]
TCP over F-Stack (FreeBSD's stack on DPDK), one F-Stack per process on the --cpus lcore; the
interface's address is --listen's (pong) or --source (ping). stream i: connection i.
Linux only, built with --features fstack (FSTACK_PREFIX: io/benches/fstack/build-fstack.sh).";
