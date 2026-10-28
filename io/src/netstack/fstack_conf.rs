//! F-Stack's settings without F-Stack: the `fstack_*` configuration keys,
//! the config file `ff_init` reads, FreeBSD's errno numbers and its
//! `struct tcp_info`. Pure, so it builds and is tested on every platform;
//! the stack itself (`super::fstack`) needs Linux and the `fstack` feature.
//!
//! The config file is written whole from [`FstackConf`] and the polling
//! CPU, so every F-Stack setting a process ran with is known
//! (`ergon_io::fstack::init` keeps the text). Fixed settings:
//!
//! | key | value | why |
//! |---|---|---|
//! | `pkt_tx_delay` | 0 | F-Stack's default holds a small TX burst up to 100 µs |
//! | `idle_sleep` | 0 | no sleep when a pass finds nothing |
//! | `tso`, `lro` | 0 | no segmentation or receive coalescing for small messages |
//! | `promiscuous` | 0 | the ENA has none |
//! | `[kni] enable`, `[pcap] enable` | 0 | no kernel exception path, no capture |
//! | `net.inet.tcp.delayed_ack` | [`FstackConf::delayed_ack`], 0 by default | what a quick-ACK route is to the kernel |
//!
//! The FreeBSD stack's other settings are F-Stack v2.0's shipped
//! `config.ini` values (SACK, timestamps, CUBIC, the `freebsd` TCP stack);
//! [`FstackConf::sysctl`] overrides any of them.
// The errno and `tcp_info` parts serve the stack, built on Linux with the
// feature only; they are tested everywhere.
#![cfg_attr(not(all(target_os = "linux", feature = "fstack")), allow(dead_code))]

use std::fmt::Write as _;
use std::net::Ipv4Addr;
use std::path::PathBuf;

use crate::error::{Error, Result};

/// F-Stack's interface and EAL settings: the `fstack_*` keys of
/// `Config` (`fstack-spin` only; every other backend refuses them).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FstackConf {
    /// `fstack_addr`: the interface's IPv4 address. Required.
    pub addr: Option<Ipv4Addr>,
    /// `fstack_prefix_len`: the subnet, 1 to 32. Default 24.
    pub prefix_len: u8,
    /// `fstack_gateway`: the subnet's router. Required.
    pub gateway: Option<Ipv4Addr>,
    /// `fstack_pci`: the port's PCI address (a NIC bound to `vfio-pci` or
    /// `igb_uio`). Exactly one of this and `vdev`.
    pub pci: Option<String>,
    /// `fstack_vdev`: a DPDK virtual port instead, as `--vdev` spells it
    /// (`net_af_packet0,iface=eth1`). In a configuration string its commas
    /// are written `;` (`fstack_vdev=net_af_packet0;iface=eth1`).
    pub vdev: Option<String>,
    /// `fstack_no_huge`: the EAL on anonymous memory instead of hugepages
    /// (tests, containers without hugepages). Default false.
    pub no_huge: bool,
    /// `fstack_memory_mb`: the EAL's memory. Default 1024.
    pub memory_mb: u32,
    /// `fstack_file_prefix`: the EAL's runtime-file prefix, one per
    /// process on a host. Default `ergon-io`.
    pub file_prefix: String,
    /// `fstack_hz`: FreeBSD's clock tick, TCP's timer granularity. Default
    /// 100 (F-Stack's).
    pub hz: u32,
    /// `fstack_delayed_ack`: FreeBSD's `net.inet.tcp.delayed_ack`. Default
    /// false: every segment is acknowledged at once, as a quick-ACK route
    /// does for the kernel.
    pub delayed_ack: bool,
    /// `fstack_sysctl=name=value` (repeatable): FreeBSD sysctls set after
    /// the defaults, overriding them.
    pub sysctl: Vec<(String, String)>,
    /// `fstack_conf_path`: where the config file is written. Default
    /// `<temp dir>/ergon-io-fstack-<pid>.conf`.
    pub conf_path: Option<PathBuf>,
}

impl Default for FstackConf {
    fn default() -> Self {
        Self::new()
    }
}

impl FstackConf {
    /// The defaults: no address, gateway or port yet.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            addr: None,
            prefix_len: 24,
            gateway: None,
            pci: None,
            vdev: None,
            no_huge: false,
            memory_mb: 1024,
            file_prefix: String::new(),
            hz: 100,
            delayed_ack: false,
            sysctl: Vec::new(),
            conf_path: None,
        }
    }

    /// Whether every setting is the default (a backend other than
    /// `fstack-spin` refuses anything else).
    #[must_use]
    pub fn is_default(&self) -> bool {
        *self == Self::new()
    }

    /// Takes one `fstack_*` key from a configuration string: `Ok(false)`
    /// when `key` is not one of them.
    ///
    /// # Errors
    /// A malformed value.
    pub(crate) fn set(&mut self, key: &str, v: &str) -> Result<bool> {
        let bad = |what: &str| Error::Config(format!("{key}: {what}, got {v:?}"));
        let ip = || v.parse::<Ipv4Addr>().map_err(|_| bad("an IPv4 address"));
        let num = || v.parse::<u32>().map_err(|_| bad("a number"));
        let flag = || v.parse::<bool>().map_err(|_| bad("true or false"));
        match key {
            "fstack_addr" => self.addr = Some(ip()?),
            "fstack_prefix_len" => {
                self.prefix_len = v.parse().map_err(|_| bad("1 to 32"))?;
            }
            "fstack_gateway" => self.gateway = Some(ip()?),
            "fstack_pci" => self.pci = Some(v.to_owned()),
            "fstack_vdev" => self.vdev = Some(v.replace(';', ",")),
            "fstack_no_huge" => self.no_huge = flag()?,
            "fstack_memory_mb" => self.memory_mb = num()?,
            "fstack_file_prefix" => v.clone_into(&mut self.file_prefix),
            "fstack_hz" => self.hz = num()?,
            "fstack_delayed_ack" => self.delayed_ack = flag()?,
            "fstack_sysctl" => {
                let (k, val) = v.split_once('=').ok_or_else(|| bad("name=value"))?;
                self.sysctl
                    .push((k.trim().to_owned(), val.trim().to_owned()));
            }
            "fstack_conf_path" => self.conf_path = Some(PathBuf::from(v)),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Checks the settings `ff_init` needs.
    ///
    /// # Errors
    /// [`Error::Config`] naming the key: no address or gateway, not exactly
    /// one of `fstack_pci` and `fstack_vdev`, a prefix outside 1 to 32, a
    /// zero `fstack_hz` or `fstack_memory_mb`, or a value with a newline
    /// (it would end its line of the file).
    pub fn check(&self) -> Result<()> {
        let need = |k: &str| Error::Config(format!("fstack-spin needs {k}"));
        if self.addr.is_none() {
            return Err(need("fstack_addr"));
        }
        if self.gateway.is_none() {
            return Err(need("fstack_gateway"));
        }
        if self.pci.is_some() == self.vdev.is_some() {
            return Err(Error::Config(
                "fstack-spin needs exactly one of fstack_pci (the port) and fstack_vdev".into(),
            ));
        }
        if !(1..=32).contains(&self.prefix_len) {
            return Err(Error::Config(format!(
                "fstack_prefix_len {}: 1 to 32",
                self.prefix_len
            )));
        }
        for (k, v) in [("fstack_hz", self.hz), ("fstack_memory_mb", self.memory_mb)] {
            if v == 0 {
                return Err(Error::Config(format!("{k} 0: at least 1")));
            }
        }
        let texts = self
            .pci
            .iter()
            .chain(&self.vdev)
            .chain(std::iter::once(&self.file_prefix))
            .chain(self.sysctl.iter().flat_map(|(k, v)| [k, v]));
        for t in texts {
            if t.contains(['\n', '\r', '[', ']']) {
                return Err(Error::Config(format!(
                    "fstack setting {t:?}: no newline or bracket"
                )));
            }
        }
        Ok(())
    }

    /// The EAL's runtime-file prefix: [`FstackConf::file_prefix`], or
    /// `ergon-io` when empty.
    #[must_use]
    pub fn eal_prefix(&self) -> &str {
        if self.file_prefix.is_empty() {
            "ergon-io"
        } else {
            &self.file_prefix
        }
    }

    /// The config file `ff_init` reads, for the polling thread on CPU
    /// `lcore` (F-Stack's one lcore).
    ///
    /// # Errors
    /// As [`FstackConf::check`]; `lcore` above 127.
    pub fn render(&self, lcore: usize) -> Result<String> {
        self.check()?;
        if lcore > 127 {
            return Err(Error::Config(format!(
                "F-Stack lcore {lcore}: CPUs 0 to 127"
            )));
        }
        let (Some(addr), Some(gateway)) = (self.addr, self.gateway) else {
            return Err(Error::Config("fstack-spin needs fstack_addr".into()));
        };
        let mask = netmask(self.prefix_len);
        let bcast = Ipv4Addr::from_bits(addr.to_bits() | !mask.to_bits());
        let mut dpdk: Vec<(&str, String)> = vec![
            ("lcore_mask", format!("{:x}", 1u128 << lcore)),
            ("channel", "4".into()),
            ("memory", self.memory_mb.to_string()),
            ("no_huge", u8::from(self.no_huge).to_string()),
            ("file_prefix", self.eal_prefix().to_owned()),
        ];
        if let Some(p) = &self.pci {
            dpdk.push(("allow", p.clone()));
        }
        if let Some(v) = &self.vdev {
            dpdk.push(("extra_eal_args", format!("--no-pci --vdev={v}")));
        }
        dpdk.extend(FIXED_DPDK.iter().map(|&(k, v)| (k, v.to_owned())));
        let port = [
            ("addr", addr.to_string()),
            ("netmask", mask.to_string()),
            ("broadcast", bcast.to_string()),
            ("gateway", gateway.to_string()),
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
        // Overrides replace a default in place, else come last.
        for (k, v) in &self.sysctl {
            match sysctl.iter_mut().find(|(d, _)| d == k) {
                Some(slot) => slot.1.clone_from(v),
                None => sysctl.push((k.as_str(), v.clone())),
            }
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
        Ok(s)
    }
}

/// `prefix_len`'s netmask.
const fn netmask(prefix_len: u8) -> Ipv4Addr {
    let bits = if prefix_len == 0 {
        0
    } else {
        u32::MAX << (32 - prefix_len as u32)
    };
    Ipv4Addr::from_bits(bits)
}

/// `[dpdk]` keys every process runs with (the module's table).
const FIXED_DPDK: [(&str, &str); 14] = [
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

/// `[freebsd.boot]`, F-Stack v2.0's `config.ini` (less `hz`, a key).
const FREEBSD_BOOT: [(&str, &str); 7] = [
    ("fd_reserve", "1024"),
    ("kern.ipc.maxsockets", "262144"),
    ("net.inet.tcp.syncache.hashsize", "4096"),
    ("net.inet.tcp.syncache.bucketlimit", "100"),
    ("net.inet.tcp.tcbhashsize", "65536"),
    ("kern.ncallout", "262144"),
    ("kern.features.inet6", "1"),
];

/// `[freebsd.sysctl]`, F-Stack v2.0's `config.ini` (less `delayed_ack`, a
/// key).
const FREEBSD_SYSCTL: [(&str, &str); 24] = [
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

/// FreeBSD errno numbers and the Linux ones they stand for, where they
/// differ (`sys/errno.h` against `asm-generic/errno*.h`). F-Stack's `ff_*`
/// calls translate `errno` themselves (`ff_os_errno`); a value F-Stack
/// hands back as data does not: `SO_ERROR` holds FreeBSD's.
const BSD_TO_LINUX: [(i32, i32); 48] = [
    (11, 35),  // EDEADLK
    (35, 11),  // EAGAIN, EWOULDBLOCK
    (36, 115), // EINPROGRESS
    (37, 114), // EALREADY
    (38, 88),  // ENOTSOCK
    (39, 89),  // EDESTADDRREQ
    (40, 90),  // EMSGSIZE
    (41, 91),  // EPROTOTYPE
    (42, 92),  // ENOPROTOOPT
    (43, 93),  // EPROTONOSUPPORT
    (44, 94),  // ESOCKTNOSUPPORT
    (45, 95),  // EOPNOTSUPP, ENOTSUP
    (46, 96),  // EPFNOSUPPORT
    (47, 97),  // EAFNOSUPPORT
    (48, 98),  // EADDRINUSE
    (49, 99),  // EADDRNOTAVAIL
    (50, 100), // ENETDOWN
    (51, 101), // ENETUNREACH
    (52, 102), // ENETRESET
    (53, 103), // ECONNABORTED
    (54, 104), // ECONNRESET
    (55, 105), // ENOBUFS
    (56, 106), // EISCONN
    (57, 107), // ENOTCONN
    (58, 108), // ESHUTDOWN
    (59, 109), // ETOOMANYREFS
    (60, 110), // ETIMEDOUT
    (61, 111), // ECONNREFUSED
    (62, 40),  // ELOOP
    (63, 36),  // ENAMETOOLONG
    (64, 112), // EHOSTDOWN
    (65, 113), // EHOSTUNREACH
    (66, 39),  // ENOTEMPTY
    (68, 87),  // EUSERS
    (69, 122), // EDQUOT
    (70, 116), // ESTALE
    (71, 66),  // EREMOTE
    (77, 37),  // ENOLCK
    (78, 38),  // ENOSYS
    (82, 43),  // EIDRM
    (83, 42),  // ENOMSG
    (84, 75),  // EOVERFLOW
    (85, 125), // ECANCELED
    (86, 84),  // EILSEQ
    (89, 74),  // EBADMSG
    (90, 72),  // EMULTIHOP
    (91, 67),  // ENOLINK
    (92, 71),  // EPROTO
];

/// The Linux errno for FreeBSD's `bsd` (unchanged where they agree, or
/// where Linux has no counterpart).
#[must_use]
pub fn bsd_errno_to_linux(bsd: i32) -> i32 {
    BSD_TO_LINUX
        .iter()
        .find(|&&(b, _)| b == bsd)
        .map_or(bsd, |&(_, l)| l)
}

/// FreeBSD's `TCP_INFO` option (`netinet/tcp.h`: 32; Linux's is 11).
pub const FREEBSD_TCP_INFO: i32 = 32;
/// Bytes asked for: FreeBSD 15's `struct tcp_info` and its padding fit.
pub const TCP_INFO_BUF: usize = 512;
/// FreeBSD's `TCPS_ESTABLISHED`.
pub const TCPS_ESTABLISHED: u8 = 4;

/// The fields of FreeBSD 15's `struct tcp_info` read, by offset (F-Stack
/// v2.0's `freebsd/netinet/tcp.h`: eight bytes of state and options, then
/// `u_int32_t`s).
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
    /// `tcpi_rttvar`, µs.
    pub rttvar_us: u32,
    /// `tcpi_snd_cwnd`, bytes.
    pub snd_cwnd: u32,
    /// `tcpi_snd_rexmitpack`: segments retransmitted.
    pub rexmit_packets: u32,
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .and_then(|s| s.try_into().ok())
        .map(u32::from_ne_bytes)
}

impl BsdTcpInfo {
    /// Decodes what `getsockopt(TCP_INFO)` returned; `None` when it is too
    /// short for the fields read.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        Some(Self {
            state: *b.first()?,
            options: *b.get(5)?,
            rto_us: u32_at(b, 8)?,
            snd_mss: u32_at(b, 16)?,
            rtt_us: u32_at(b, 68)?,
            rttvar_us: u32_at(b, 72)?,
            snd_cwnd: u32_at(b, 80)?,
            rexmit_packets: u32_at(b, 120)?,
        })
    }

    /// As ergon-io's [`crate::TcpInfo`] (`at_ns` 0): FreeBSD keeps no
    /// windowed minimum RTT and no count of segments now lost.
    #[must_use]
    pub const fn to_tcp_info(self) -> crate::TcpInfo {
        crate::TcpInfo {
            at_ns: 0,
            rtt_us: self.rtt_us,
            rttvar_us: self.rttvar_us,
            min_rtt_us: None,
            total_retrans: self.rexmit_packets,
            lost: 0,
            rto_us: self.rto_us,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type R<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    fn lab() -> R<FstackConf> {
        let mut c = FstackConf::new();
        for (k, v) in [
            ("fstack_addr", "10.9.0.1"),
            ("fstack_gateway", "10.9.0.254"),
            ("fstack_vdev", "net_af_packet0;iface=va"),
            ("fstack_no_huge", "true"),
            ("fstack_memory_mb", "512"),
            ("fstack_file_prefix", "client"),
            ("fstack_sysctl", "net.inet.tcp.blackhole=0"),
            ("fstack_sysctl", "net.inet.tcp.keepinit=5000"),
        ] {
            assert!(c.set(k, v)?, "{k} not a key");
        }
        Ok(c)
    }

    #[test]
    fn the_file_has_every_key_once() -> R {
        let c = lab()?;
        let s = c.render(3)?;
        for want in [
            "[dpdk]\nlcore_mask=8\n",
            "memory=512\nno_huge=1\nfile_prefix=client\n",
            "extra_eal_args=--no-pci --vdev=net_af_packet0,iface=va\n",
            "pkt_tx_delay=0\n",
            "idle_sleep=0\n",
            "[port0]\naddr=10.9.0.1\nnetmask=255.255.255.0\nbroadcast=10.9.0.255\ngateway=10.9.0.254\n",
            "[kni]\nenable=0\n",
            "[freebsd.boot]\nhz=100\n",
            "net.inet.tcp.delayed_ack=0\n",
            "net.inet.tcp.blackhole=0\n",
            "net.inet.tcp.keepinit=5000\n",
        ] {
            assert!(s.contains(want), "{want:?} missing from\n{s}");
        }
        // An override replaces the default in place: one line per key.
        let mut keys: Vec<&str> = s
            .lines()
            .filter(|l| !l.starts_with('['))
            .filter_map(|l| l.split_once('=').map(|(k, _)| k))
            .collect();
        let n = keys.len();
        keys.sort_unstable();
        keys.dedup();
        // `enable` is in two sections ([kni], [pcap]).
        assert_eq!(keys.len(), n - 1, "a key twice in\n{s}");
        assert!(!s.contains("allow="), "a PCI port with a vdev");
        Ok(())
    }

    #[test]
    fn missing_or_conflicting_settings_are_refused() -> R {
        assert!(FstackConf::new().render(0).is_err(), "no address");
        let mut c = lab()?;
        c.pci = Some("0000:00:06.0".into());
        let e = c.render(0).err().ok_or("a port and a vdev")?;
        assert!(e.to_string().contains("exactly one"), "{e}");
        let mut c = lab()?;
        c.gateway = None;
        assert!(c.render(0).is_err(), "no gateway");
        let mut c = lab()?;
        c.prefix_len = 33;
        assert!(c.render(0).is_err(), "/33");
        assert!(lab()?.render(128).is_err(), "lcore 128");
        let mut c = lab()?;
        c.sysctl.push(("x".into(), "1\n[dpdk]".into()));
        assert!(c.render(0).is_err(), "a newline in a value");
        let mut c = FstackConf::new();
        assert!(c.set("fstack_addr", "10.0.0").is_err());
        assert!(c.set("fstack_sysctl", "novalue").is_err());
        assert!(!c.set("fstack_nope", "1")?, "an unknown key taken");
        assert!(c.is_default());
        Ok(())
    }

    #[test]
    fn a_pci_port_is_allowed_by_address() -> R {
        let mut c = lab()?;
        c.vdev = None;
        c.pci = Some("0000:00:06.0".into());
        c.no_huge = false;
        let s = c.render(1)?;
        assert!(
            s.contains("allow=0000:00:06.0\n") && s.contains("no_huge=0\n"),
            "{s}"
        );
        assert!(!s.contains("--no-pci"), "{s}");
        Ok(())
    }

    /// The table's FreeBSD side against this host's errno where it is a
    /// BSD (macOS keeps FreeBSD's numbers for every entry here), and its
    /// Linux side against Linux's.
    #[test]
    fn errno_numbers_match_each_system() {
        #[cfg(target_os = "macos")]
        for (bsd, name) in [
            (libc::EAGAIN, "EAGAIN"),
            (libc::EINPROGRESS, "EINPROGRESS"),
            (libc::ECONNREFUSED, "ECONNREFUSED"),
            (libc::ECONNRESET, "ECONNRESET"),
            (libc::ETIMEDOUT, "ETIMEDOUT"),
            (libc::EHOSTUNREACH, "EHOSTUNREACH"),
            (libc::ENETUNREACH, "ENETUNREACH"),
            (libc::ENOTCONN, "ENOTCONN"),
            (libc::EADDRINUSE, "EADDRINUSE"),
            (libc::EDEADLK, "EDEADLK"),
            (libc::ELOOP, "ELOOP"),
        ] {
            assert!(
                BSD_TO_LINUX.iter().any(|&(b, _)| b == bsd),
                "{name} ({bsd}) not in the table"
            );
        }
        #[cfg(target_os = "linux")]
        for (bsd, linux) in [
            (35, libc::EAGAIN),
            (36, libc::EINPROGRESS),
            (61, libc::ECONNREFUSED),
            (54, libc::ECONNRESET),
            (60, libc::ETIMEDOUT),
            (65, libc::EHOSTUNREACH),
            (51, libc::ENETUNREACH),
            (57, libc::ENOTCONN),
            (48, libc::EADDRINUSE),
            (11, libc::EDEADLK),
            (62, libc::ELOOP),
        ] {
            assert_eq!(bsd_errno_to_linux(bsd), linux, "FreeBSD {bsd}");
        }
        assert_eq!(bsd_errno_to_linux(61), 111, "ECONNREFUSED");
        assert_eq!(bsd_errno_to_linux(22), 22, "EINVAL is the same");
        assert_eq!(bsd_errno_to_linux(9999), 9999, "unknown is kept");
        // One Linux number per FreeBSD one, and no FreeBSD number twice.
        let mut b: Vec<i32> = BSD_TO_LINUX.iter().map(|p| p.0).collect();
        let mut l: Vec<i32> = BSD_TO_LINUX.iter().map(|p| p.1).collect();
        b.sort_unstable();
        b.dedup();
        l.sort_unstable();
        l.dedup();
        assert_eq!((b.len(), l.len()), (BSD_TO_LINUX.len(), BSD_TO_LINUX.len()));
    }

    #[test]
    fn tcp_info_offsets() {
        let mut b = [0u8; TCP_INFO_BUF];
        b[0] = TCPS_ESTABLISHED;
        b[5] = 7;
        for (at, v) in [
            (8, 201_000u32),
            (16, 1448),
            (68, 55),
            (72, 9),
            (80, 14_480),
            (120, 3),
        ] {
            b[at..at + 4].copy_from_slice(&v.to_ne_bytes());
        }
        let i = BsdTcpInfo::parse(&b);
        assert_eq!(
            i,
            Some(BsdTcpInfo {
                state: 4,
                options: 7,
                rto_us: 201_000,
                snd_mss: 1448,
                rtt_us: 55,
                rttvar_us: 9,
                snd_cwnd: 14_480,
                rexmit_packets: 3,
            })
        );
        let t = i.map(BsdTcpInfo::to_tcp_info).unwrap_or_default();
        assert_eq!(
            (t.rtt_us, t.rttvar_us, t.total_retrans, t.rto_us),
            (55, 9, 3, 201_000)
        );
        assert_eq!(t.min_rtt_us, None);
        assert!(BsdTcpInfo::parse(&b[..100]).is_none(), "short buffer read");
    }
}
