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
use std::path::Path;
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
    /// `--hz`. The direct path defaults to 100 (F-Stack's). The ergon path
    /// defaults to [`ERGON_FSTACK_HZ`]: FreeBSD stores `rexmit_min` as
    /// `ms * hz / 1000` ticks and rejects a write of 0 ticks, so 8 ms at
    /// 100 Hz never applies.
    pub hz: u32,
    /// `--delayed-ack`, default true (FreeBSD's and F-Stack's).
    pub delayed_ack: bool,
    /// `--rcvbuf`, bytes (`SO_RCVBUF` on every socket).
    pub rcvbuf: u32,
    /// `--sndbuf`, bytes; `None`: F-Stack's `sendspace` (16 KiB).
    pub sndbuf: Option<u32>,
    /// `--impl ergon`'s `--rto-min-us`, as FreeBSD's `net.inet.tcp.rexmit_min`
    /// in milliseconds. The direct path leaves this unset: it has no kernel
    /// `TCP_RTO_MIN_US`.
    pub rexmit_min_ms: Option<u32>,
}

/// Which F-Stack `fstack_lat` runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FstackImpl {
    /// The direct libfstack path (`ff_*`).
    Direct,
    /// ergon-io's `fstack-spin` backend.
    Ergon,
}

impl FstackImpl {
    /// The backend name the manifest and the runner record.
    #[must_use]
    pub const fn backend(self) -> &'static str {
        match self {
            Self::Direct => "fstack",
            Self::Ergon => "fstack-spin",
        }
    }
}

/// FreeBSD clock for the ergon `fstack-spin` path.
///
/// F-Stack's own default is 100 Hz. Its `sysctl_msec_to_ticks` stores
/// `ms * hz / 1000` ticks and returns `EINVAL` below one tick, so
/// `net.inet.tcp.rexmit_min=8` at 100 Hz is 0 ticks and the 8 ms floor
/// never applies. 1000 Hz stores 8 ms as 8 ticks (Linux's clock) and the
/// sysctl value stays `8`. The direct path keeps 100 Hz and does not set
/// `rexmit_min`.
pub const ERGON_FSTACK_HZ: u32 = 1000;

/// Clock for an ergon config that sets `net.inet.tcp.rexmit_min`.
///
/// `requested` is `--hz` when the flag was passed, otherwise
/// [`ERGON_FSTACK_HZ`]. A floor that would be 0 ticks at that clock is run
/// at [`ERGON_FSTACK_HZ`] instead. No floor: `requested` is kept.
#[must_use]
pub fn hz_for_rexmit(requested: u32, rexmit_min_ms: Option<u32>) -> u32 {
    match rexmit_min_ms {
        Some(ms) if u64::from(ms) * u64::from(requested) / 1000 < 1 => ERGON_FSTACK_HZ,
        _ => requested,
    }
}

fn default_hz(imp: FstackImpl) -> u32 {
    if imp == FstackImpl::Ergon {
        ERGON_FSTACK_HZ
    } else {
        100
    }
}

fn chosen_hz(imp: FstackImpl, requested: u32, rto_min_us: Option<u32>) -> u32 {
    let rexmit = rto_min_us.map(|us| us.div_ceil(1000));
    if imp == FstackImpl::Ergon {
        hz_for_rexmit(requested, rexmit)
    } else {
        requested
    }
}

/// ergon-io `Config` for [`FstackImpl::Ergon`].
///
/// `--rto-min-us` is FreeBSD's `net.inet.tcp.rexmit_min` (milliseconds,
/// already rounded in [`FstackConf::rexmit_min_ms`]). The kernel's
/// `tcp_rto_min_us` stays unset; `fstack-spin` refuses it. The clock is
/// [`hz_for_rexmit`], so an 8 ms floor is not 0 ticks.
/// `conf_path` is where `ff_init` reads the rendered file. `fstack_spin_lat`
/// builds its driver from this.
///
/// # Errors
/// [`Error::Usage`] when `a` is the direct path, the lcore does not fit, or
/// [`ergon_io::Config::validate`] refuses the settings.
pub fn spin_config(a: &FstackArgs, conf_path: &Path) -> Result<ergon_io::Config, Error> {
    if a.imp != FstackImpl::Ergon {
        return Err(Error::Usage(
            "spin_config is fstack-spin (--impl ergon); the direct path is fstack_lat".into(),
        ));
    }
    let lcore = u32::try_from(a.conf.lcore).map_err(|_| {
        Error::Usage(format!(
            "--cpus {}: the fstack-spin lcore does not fit",
            a.conf.lcore
        ))
    })?;
    let mut cfg = ergon_io::Config::new(ergon_io::Backend::FstackSpin);
    cfg.loop_cpu = Some(lcore);
    cfg.rcvbuf = Some(a.conf.rcvbuf);
    let ms = u32::try_from(a.connect_timeout.as_millis()).unwrap_or(u32::MAX);
    cfg.connect_timeout_ms = if ms == 0 { 1 } else { ms };
    let f = &mut cfg.fstack;
    f.addr = Some(a.conf.addr);
    f.prefix_len = a.conf.prefix_len;
    f.gateway = Some(a.conf.gateway);
    f.pci.clone_from(&a.conf.pci);
    f.vdev.clone_from(&a.conf.vdev);
    f.no_huge = a.conf.no_huge;
    f.memory_mb = a.conf.memory_mb;
    f.file_prefix.clone_from(&a.conf.file_prefix);
    f.hz = hz_for_rexmit(a.conf.hz, a.conf.rexmit_min_ms);
    f.delayed_ack = a.conf.delayed_ack;
    f.conf_path = Some(conf_path.to_path_buf());
    if let Some(ms) = a.conf.rexmit_min_ms {
        f.sysctl
            .push(("net.inet.tcp.rexmit_min".to_owned(), ms.to_string()));
    }
    cfg.validate()
        .map_err(|e| Error::Usage(format!("fstack-spin config: {e}")))?;
    Ok(cfg)
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
    /// `--impl`: absent or `fstack` is [`FstackImpl::Direct`]; `ergon` is
    /// [`FstackImpl::Ergon`] (`fstack-spin`).
    pub imp: FstackImpl,
    /// `--rto-min-us` when [`FstackImpl::Ergon`] took it. The direct path
    /// leaves the flag for [`Flags::finish`] to refuse.
    pub rto_min_us: Option<u32>,
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
    /// `--nodelay` is always on (refused off), as in `tcp_lat`. `--impl` is
    /// `fstack` (absent: the same) or `ergon` ([`FstackImpl::Ergon`], backend
    /// `fstack-spin`). The kernel's socket flags (`--busy-read-us`,
    /// `--epoll-events`, `--quickack-per-read`, `--delack-max-us`) are not
    /// taken. `--rto-min-us` is taken only for `--impl ergon`, as FreeBSD's
    /// `net.inet.tcp.rexmit_min` (milliseconds, rounded up); the direct path
    /// leaves it, and [`Flags::finish`] refuses it.
    ///
    /// # Errors
    /// [`Error::Usage`] for a flag missing, malformed or refused.
    pub fn parse(f: &mut Flags, role: Role, cpus: &[usize]) -> Result<Self, Error> {
        if !f.bool("nodelay", true)? {
            return Err(Error::Usage("--nodelay is always on".into()));
        }
        let imp = match f.take("impl").as_deref() {
            None | Some("fstack") => FstackImpl::Direct,
            Some("ergon") => FstackImpl::Ergon,
            Some(v) => {
                return Err(Error::Usage(format!("--impl {v}: fstack or ergon")));
            }
        };
        let rto_min_us = if imp == FstackImpl::Ergon {
            match f.take("rto-min-us") {
                None => None,
                Some(v) => Some(
                    v.parse::<u32>()
                        .map_err(|_| Error::Usage(format!("--rto-min-us {v}: microseconds")))?,
                ),
            }
        } else {
            None
        };
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
            hz: chosen_hz(imp, f.num("hz", default_hz(imp))?, rto_min_us),
            delayed_ack: f.bool("delayed-ack", true)?,
            rcvbuf: f.num("rcvbuf", super::DEFAULT_RCVBUF)?,
            sndbuf,
            rexmit_min_ms: rto_min_us.map(|us| us.div_ceil(1000)),
        };
        for a in listen.iter().chain(&connect) {
            super::port_plus(SocketAddr::V4(*a), 0)?;
        }
        Ok(Self {
            conf,
            listen,
            connect,
            connect_timeout: Duration::from_secs(f.num("connect-timeout-s", 30)?),
            imp,
            rto_min_us,
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
        if let Some(ms) = self.rexmit_min_ms {
            sysctl.push(("net.inet.tcp.rexmit_min", ms.to_string()));
        }
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

/// The capabilities a `--pci` port's EAL needs, with why.
///
/// `igb_uio` has no IOMMU, so the EAL takes IOVA as physical addresses, which
/// `/proc/self/pagemap` shows only to `CAP_SYS_ADMIN` (without it the EAL
/// stops with "Cannot use IOVA as 'PA'"); `/dev/uioN`, the BARs' sysfs
/// `resource` files and `/dev/hugepages` are root's (`CAP_DAC_OVERRIDE`).
pub const PCI_CAPS: [(u32, &str, &str); 2] = [
    (
        21,
        "CAP_SYS_ADMIN",
        "physical addresses from /proc/self/pagemap (IOVA as PA under igb_uio)",
    ),
    (
        1,
        "CAP_DAC_OVERRIDE",
        "/dev/uioN, the BARs' sysfs resource files, /dev/hugepages",
    ),
];

impl FstackConf {
    /// Checks `/proc/self/status` (`status`) has every capability
    /// [`PCI_CAPS`] lists in `CapEff`, for a `--pci` port with hugepages;
    /// a `--vdev` or `--no-huge` end needs none of them.
    ///
    /// # Errors
    /// [`Error::Transport`] naming each one missing (or `CapEff` unread),
    /// before `ff_init`, whose EAL would otherwise exit the process.
    pub fn check_caps(&self, status: &str) -> Result<(), Error> {
        if self.pci.is_none() || self.no_huge {
            return Ok(());
        }
        let eff = status
            .lines()
            .find_map(|l| l.strip_prefix("CapEff:"))
            .and_then(|v| u64::from_str_radix(v.trim(), 16).ok())
            .ok_or_else(|| Error::Transport("no CapEff in /proc/self/status".into()))?;
        let missing: Vec<String> = PCI_CAPS
            .iter()
            .filter(|(bit, ..)| eff & (1 << bit) == 0)
            .map(|(_, name, why)| format!("{name} ({why})"))
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        Err(Error::Transport(format!(
            "--pci {} needs {}: run it as root or with them ambient \
             (transport.sh: sudo setpriv, tr_fstack_priv)",
            self.pci.as_deref().unwrap_or(""),
            missing.join(", ")
        )))
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
        [--impl fstack|ergon] [--rto-min-us N]
TCP over F-Stack (FreeBSD's stack on DPDK), one F-Stack per process on the --cpus lcore; the
interface's address is --listen's (pong) or --source (ping). stream i: connection i.
--impl ergon selects ergon-io's fstack-spin backend and takes --rto-min-us as
net.inet.tcp.rexmit_min (milliseconds, rounded up). Its clock defaults to --hz 1000 so that
floor is at least one FreeBSD tick (8 ms at 100 Hz is 0 ticks and the sysctl is rejected).
Absent or fstack is the direct libfstack path, which keeps --hz 100 and refuses --rto-min-us.
Linux only, built with --features fstack (FSTACK_PREFIX: io/benches/fstack/build-fstack.sh).";
