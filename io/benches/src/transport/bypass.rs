//! NIC and stack settings shared by the bypass arms.
//!
//! The runner passes `--source`, `--pci` or `--vdev`, `--prefix-len`,
//! `--gateway` and `--rto-min-us` on every `smoltcp-spin` and `fstack-spin`
//! line. This module turns those flags into smoltcp's [`StackConfig`] and
//! into the `fstack-spin` [`ergon_io::Config`] `spin_config` already builds
//! for TCP. It builds on macOS: the measuring loops that open a NIC are
//! Linux feature gates.

#![deny(missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use ergon_io::smoltcp_dpdk::StackConfig;

use super::fstack_conf::{FstackArgs, FstackConf, FstackImpl, spin_config};
use super::point::{Error, Flags, Role};

/// The data NIC a bypass ping binds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BypassNic {
    /// This stack's IPv4 address.
    pub ip: Ipv4Addr,
    /// Prefix length, 1 to 32.
    pub prefix_len: u8,
    /// Default gateway, another address inside the prefix.
    pub gateway: Ipv4Addr,
    /// PCI address (`-a`), set when [`Self::vdev`] is not.
    pub pci: Option<String>,
    /// DPDK `--vdev` spec, set when [`Self::pci`] is not.
    pub vdev: Option<String>,
    /// RTO floor, microseconds (8 ms in the bypass matrix).
    pub rto_min_us: u32,
}

/// What `fstack_spin_config` needs besides the NIC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FstackSpinIn {
    /// The data NIC.
    pub nic: BypassNic,
    /// The one lcore (`--cpus`).
    pub lcore: usize,
    /// FreeBSD `net.inet.tcp.delayed_ack`. The runner's quick-ACK is the
    /// opposite: delayed ACKs off.
    pub delayed_ack: bool,
    /// `SO_RCVBUF`.
    pub rcvbuf: u32,
    /// `SO_SNDBUF`.
    pub sndbuf: u32,
    /// How long ping retries the connect.
    pub connect_timeout: Duration,
}

/// What `udp_lat --backend` selects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UdpChoice {
    /// Kernel `epoll-spin`, including a smoltcp arm's pong (the kernel peer).
    Kernel,
    /// Userspace UDP on the polling thread. Only built with `smoltcp-dpdk`.
    #[cfg(all(target_os = "linux", feature = "smoltcp-dpdk"))]
    Smoltcp(BypassNic),
}

/// smoltcp's [`StackConfig`] for `nic`.
///
/// PCI is two EAL tokens (`-a`, the BDF) and the port name is that BDF.
/// A vdev's port name is the spec before the first comma. The RTO floor is
/// [`StackConfig::rto_min_us`], not the kernel's `tcp_rto_min_us`.
///
/// # Errors
/// [`Error::Usage`] when the NIC is not one port, or [`StackConfig::validate`]
/// refuses it.
pub fn stack_config(nic: &BypassNic) -> Result<StackConfig, Error> {
    let (eal_args, port) = match (&nic.pci, &nic.vdev) {
        (Some(pci), None) => (vec!["-a".to_owned(), pci.clone()], pci.clone()),
        (None, Some(vdev)) => {
            let port = vdev
                .split(',')
                .next()
                .filter(|name| !name.is_empty())
                .ok_or_else(|| Error::Usage(format!("--vdev {vdev}: a device name")))?
                .to_owned();
            (vec![format!("--vdev={vdev}")], port)
        }
        _ => {
            return Err(Error::Usage(
                "smoltcp-spin: exactly one of --pci and --vdev".into(),
            ));
        }
    };
    let cfg = StackConfig {
        eal_args,
        port,
        ip: nic.ip,
        prefix_len: nic.prefix_len,
        gateway: Some(nic.gateway),
        rto_min_us: nic.rto_min_us,
        tcp_timestamps: true,
        ..StackConfig::default()
    };
    cfg.validate()
        .map_err(|e| Error::Usage(format!("smoltcp-spin: {e}")))?;
    Ok(cfg)
}

/// ergon-io `Config` for an `fstack-spin` ping, via [`spin_config`].
///
/// `--rto-min-us` is FreeBSD's `net.inet.tcp.rexmit_min` in milliseconds,
/// rounded up. The clock is 1000 Hz: at F-Stack's 100 Hz an 8 ms floor is
/// 0 ticks and the sysctl is rejected. The kernel's `tcp_rto_min_us` stays
/// unset.
///
/// # Errors
/// [`Error::Usage`] when the lcore does not fit or [`spin_config`] refuses
/// the settings.
pub fn fstack_spin_config(inp: &FstackSpinIn, path: &Path) -> Result<ergon_io::Config, Error> {
    let rexmit_min_ms = inp.nic.rto_min_us.div_ceil(1000);
    let args = FstackArgs {
        conf: FstackConf {
            addr: inp.nic.ip,
            prefix_len: inp.nic.prefix_len,
            gateway: inp.nic.gateway,
            pci: inp.nic.pci.clone(),
            vdev: inp.nic.vdev.clone(),
            no_huge: false,
            memory_mb: 1024,
            file_prefix: "ergon-fstack".to_owned(),
            lcore: inp.lcore,
            hz: super::fstack_conf::hz_for_rexmit(
                super::fstack_conf::ERGON_FSTACK_HZ,
                Some(rexmit_min_ms),
            ),
            delayed_ack: inp.delayed_ack,
            rcvbuf: inp.rcvbuf,
            sndbuf: Some(inp.sndbuf),
            rexmit_min_ms: Some(rexmit_min_ms),
        },
        listen: None,
        connect: None,
        connect_timeout: inp.connect_timeout,
        imp: FstackImpl::Ergon,
        rto_min_us: Some(inp.nic.rto_min_us),
    };
    spin_config(&args, path)
}

/// Takes `udp_lat`'s `--backend`, `--sqpoll-cpu` and the bypass NIC flags.
///
/// `epoll-spin` (and a missing `--backend`) is the kernel. `fstack-spin`
/// has no UDP arm. `smoltcp-spin` on ping is userspace when this build has
/// `smoltcp-dpdk`, and a usage error that names the backend otherwise. The
/// pong of a smoltcp arm is the kernel peer: loss is applied there, and
/// this process does not start a stack.
///
/// # Errors
/// [`Error::Usage`] for a backend this binary will not run, or a malformed
/// NIC flag.
pub fn udp_choice(
    f: &mut Flags,
    role: Role,
    ergon: bool,
    local: SocketAddr,
) -> Result<UdpChoice, Error> {
    let backend = f.take("backend");
    let sqpoll = f.take("sqpoll-cpu");
    let raw = take_raw_nic(f)?;
    if !ergon && (backend.is_some() || sqpoll.is_some() || raw.any()) {
        return Err(Error::Usage(
            "--backend is ergon-io's (--impl ergon)".into(),
        ));
    }
    match backend.as_deref() {
        None | Some("epoll-spin") => {
            if sqpoll.is_some() {
                return Err(Error::Usage(
                    "--sqpoll-cpu is uring-sqpoll's, and udp_lat has no uring arm".into(),
                ));
            }
            Ok(UdpChoice::Kernel)
        }
        Some("uring-sqpoll") => Err(Error::Usage(
            "--backend uring-sqpoll: udp_lat has no uring arm".into(),
        )),
        Some("fstack-spin") => Err(Error::Usage(
            "--backend fstack-spin has no UDP arm. A kernel socket will not be labelled \
             fstack-spin"
                .into(),
        )),
        Some("smoltcp-spin") => smoltcp_udp(role, raw, local),
        Some(other) => Err(Error::Usage(format!(
            "--backend {other}: epoll-spin or smoltcp-spin"
        ))),
    }
}

fn smoltcp_udp(role: Role, raw: RawNic, local: SocketAddr) -> Result<UdpChoice, Error> {
    match role {
        Role::Pong => Ok(UdpChoice::Kernel),
        Role::Local | Role::OnewaySend | Role::OnewayRecv => Err(Error::Usage(format!(
            "--role {}: one smoltcp-spin stack per process",
            role.name()
        ))),
        Role::Ping => smoltcp_udp_ping(raw, local),
    }
}

#[cfg(all(target_os = "linux", feature = "smoltcp-dpdk"))]
fn smoltcp_udp_ping(raw: RawNic, local: SocketAddr) -> Result<UdpChoice, Error> {
    Ok(UdpChoice::Smoltcp(raw.require(local, "smoltcp-spin")?))
}

#[cfg(not(all(target_os = "linux", feature = "smoltcp-dpdk")))]
fn smoltcp_udp_ping(raw: RawNic, local: SocketAddr) -> Result<UdpChoice, Error> {
    let _ = (raw, local);
    Err(Error::Usage(
        "--backend smoltcp-spin is ergon-io's userspace UDP (StackConfig::rto_min_us). \
         A kernel socket will not be labelled smoltcp-spin"
            .into(),
    ))
}

struct RawNic {
    source: Option<IpAddr>,
    prefix_len: Option<u8>,
    gateway: Option<Ipv4Addr>,
    pci: Option<String>,
    vdev: Option<String>,
    rto_min_us: Option<u32>,
}

impl RawNic {
    const fn any(&self) -> bool {
        self.source.is_some()
            || self.prefix_len.is_some()
            || self.gateway.is_some()
            || self.pci.is_some()
            || self.vdev.is_some()
            || self.rto_min_us.is_some()
    }

    /// Called from the `smoltcp-dpdk` ping path. On a build without that
    /// feature the ping returns the usage error and never asks for the NIC,
    /// so a Mac test still sees the backend name when the flags are incomplete.
    #[cfg_attr(
        not(all(target_os = "linux", feature = "smoltcp-dpdk")),
        allow(dead_code)
    )]
    fn require(self, local: SocketAddr, backend: &str) -> Result<BypassNic, Error> {
        let ip = match self.source.unwrap_or_else(|| local.ip()) {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(ip) => {
                return Err(Error::Usage(format!("--source {ip}: IPv4 ({backend})")));
            }
        };
        let prefix_len = self
            .prefix_len
            .ok_or_else(|| Error::Usage(format!("--prefix-len is required for {backend}")))?;
        let gateway = self
            .gateway
            .ok_or_else(|| Error::Usage(format!("--gateway is required for {backend}")))?;
        if self.pci.is_some() == self.vdev.is_some() {
            return Err(Error::Usage(format!(
                "--pci or --vdev, not both, is required for {backend}"
            )));
        }
        let rto_min_us = self.rto_min_us.ok_or_else(|| {
            Error::Usage(format!(
                "--rto-min-us is required for {backend} (StackConfig::rto_min_us)"
            ))
        })?;
        Ok(BypassNic {
            ip,
            prefix_len,
            gateway,
            pci: self.pci,
            vdev: self.vdev,
            rto_min_us,
        })
    }
}

fn take_raw_nic(f: &mut Flags) -> Result<RawNic, Error> {
    let source = f.take("source").map_or(Ok(None), |v| {
        v.parse::<IpAddr>()
            .map(Some)
            .map_err(|_| Error::Usage(format!("--source {v}: an IP address")))
    })?;
    let prefix_len = f.take("prefix-len").map_or(Ok(None), |v| {
        v.parse::<u8>()
            .ok()
            .filter(|n| (1..=32).contains(n))
            .map(Some)
            .ok_or_else(|| Error::Usage(format!("--prefix-len {v}: 1 to 32")))
    })?;
    let gateway = f.take("gateway").map_or(Ok(None), |v| {
        v.parse::<Ipv4Addr>()
            .map(Some)
            .map_err(|_| Error::Usage(format!("--gateway {v}: an IPv4 address")))
    })?;
    let rto_min_us = f.take("rto-min-us").map_or(Ok(None), |v| {
        v.parse::<u32>()
            .map(Some)
            .map_err(|_| Error::Usage(format!("--rto-min-us {v}: microseconds")))
    })?;
    Ok(RawNic {
        source,
        prefix_len,
        gateway,
        pci: f.take("pci"),
        vdev: f.take("vdev"),
        rto_min_us,
    })
}
