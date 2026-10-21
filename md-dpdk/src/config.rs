//! Typed DPDK transport configuration and validation.

use crate::error::DpdkError;
use std::fmt;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;

/// An Ethernet MAC address, written `aa:bb:cc:dd:ee:ff` (either case on input,
/// lower case on output).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct MacAddr(pub [u8; 6]);

impl MacAddr {
    /// Not multicast (and so not broadcast) and not all zero: an address a
    /// single port can own.
    #[must_use]
    pub const fn is_unicast(self) -> bool {
        self.0[0] & 1 == 0 && !matches!(self.0, [0, 0, 0, 0, 0, 0])
    }
}

impl FromStr for MacAddr {
    type Err = DpdkError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || DpdkError::InvalidConfiguration(format!("{s:?} is not a MAC address aa:bb:cc:dd:ee:ff"));
        let mut mac = [0_u8; 6];
        let mut octets = s.split(':');
        for byte in &mut mac {
            let octet = octets.next().ok_or_else(invalid)?;
            if octet.len() != 2 {
                return Err(invalid());
            }
            *byte = u8::from_str_radix(octet, 16).map_err(|_| invalid())?;
        }
        if octets.next().is_some() {
            return Err(invalid());
        }
        Ok(Self(mac))
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, octet) in self.0.iter().enumerate() {
            if i > 0 {
                f.write_str(":")?;
            }
            write!(f, "{octet:02x}")?;
        }
        Ok(())
    }
}

/// Per-port DPDK configuration for one ENA device.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DpdkPortConfig {
    /// Canonical `dddd:bb:ss.f` PCI BDF of the device: the ENA on AWS, or the
    /// MANA function on Azure (one function serves every accelerated NIC).
    pub pci_address: String,
    /// The port's MAC. Required for Azure MANA, where it selects the NIC's
    /// vport on the shared function; `None` for an ENA, which is one port per
    /// PCI function.
    pub mac: Option<MacAddr>,
    /// IPv4 address assigned to this ENA on its subnet.
    pub local_ipv4: Ipv4Addr,
    /// Subnet prefix length (`1..=32`).
    pub prefix_len: u8,
    /// VPC router address for this subnet.
    pub gateway_ipv4: Ipv4Addr,
}

/// The receive ring a port takes when [`DpdkTransportConfig::rx_descriptors`]
/// is `None`.
///
/// Fewer only where the NIC reports a lower maximum (Azure MANA's 256).
/// 4096 against ENA's 1024 did not separate at idle (md-dpdk/BENCHMARKS.md,
/// "Measured, but no setting recommended": round 6 and G1), and round 6's
/// loss rows for 1024 were not recovered, so the default rests on room for a
/// burst, not on a measured gain.
pub const DEFAULT_RX_DESCRIPTORS: u16 = 4096;

/// Azure MANA's queue limit (the native `RUSTERON_DPDK_MANA_MAX_DESCRIPTORS`):
/// what an unset receive ring takes there, and what the pool validation's
/// error suggests for a pool sized for it.
const MANA_MAX_DESCRIPTORS: u16 = 256;

/// Validated transport configuration covering both ENA roles and the EAL.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DpdkTransportConfig {
    /// The ENA the driver's sender agent transmits on.
    pub sender: DpdkPortConfig,
    /// The ENA the driver's receiver agent receives on.
    pub receiver: DpdkPortConfig,
    /// DPDK EAL file prefix, unique per process (`[A-Za-z0-9_-]{1,64}`).
    pub file_prefix: String,
    /// Accept DPDK virtual-device names (e.g. `net_tap0`) instead of PCI BDFs.
    ///
    /// Test lever: the environment parser sets it only from
    /// `RUSTERON_DPDK_TEST_VDEV=1`, so production configurations stay PCI-only.
    /// When set, the two selectors are each validated as a canonical PCI BDF
    /// *or* a vdev name, and the native EAL emits `--vdev=<name>` instead of
    /// `-a <bdf>`. Not a user setting; hidden from the API.
    #[doc(hidden)]
    pub test_vdev: bool,
    /// Hugepage mount. Must be absolute; hugetlbfs backing is verified natively
    /// at EAL initialization.
    pub hugepage_dir: PathBuf,
    /// Receive ring descriptors per port (`64..=8192`). `None` (the default)
    /// takes min([`DEFAULT_RX_DESCRIPTORS`], the NIC's reported maximum) on
    /// each port at start: 4096 on an ENA, 256 on Azure MANA. An explicit
    /// count is used as given, and start fails on a NIC that cannot take it.
    pub rx_descriptors: Option<u16>,
    /// Transmit ring descriptors per port (`64..=8192`).
    pub tx_descriptors: u16,
    /// Mbufs in each port's mempool; at least both rings, four bursts and two
    /// per-lcore mempool caches (the receive ring at its most, 4096, when
    /// `rx_descriptors` is `None`).
    pub mbufs_per_port: u32,
    /// Per-lcore mempool cache size.
    pub mempool_cache: u16,
    /// Packets per receive/transmit burst (`1..=256`).
    pub burst_size: u16,
    /// Largest Aeron MTU the transport carries: non-zero, 32-aligned, at most
    /// 8960 (jumbo frames: an AWS 9001-byte VPC MTU less IPv4+UDP; 8448 for an
    /// 8500-byte path such as inter-region VPC peering), and it must fit the
    /// NIC's maximum MTU and its mbufs (checked when the ports start: each port
    /// takes it plus 28 as its MTU, and every frame is one mbuf). Every peer's Aeron MTU must
    /// be at most this: IPv4 fragments are never accepted.
    /// The context's MTU must not exceed it.
    pub max_aeron_mtu: usize,
    /// Request the NIC's hardware receive timestamps (AWS ENA in a
    /// `precision-time` placement group, DPDK >= 25.11). Each port then
    /// reports the delay from NIC arrival to the receive burst that carried
    /// the frame: `CLOCK_REALTIME` is read once a burst, before any frame of
    /// it is dispatched, so a frame after the burst's first is measured
    /// before its own dispatch. Initialization fails on a port whose driver
    /// does not offer RX timestamps.
    pub rx_hw_timestamp: bool,
}

impl DpdkTransportConfig {
    /// Validate every cross-field invariant below.
    ///
    /// Returns [`DpdkError::InvalidConfiguration`] with the failing field (and
    /// port role where applicable) named in the message.
    ///
    /// # Errors
    ///
    /// The first violated invariant, as [`DpdkError::InvalidConfiguration`].
    pub fn validate(&self) -> Result<(), DpdkError> {
        validate_port("sender", &self.sender, self.test_vdev)?;
        validate_port("receiver", &self.receiver, self.test_vdev)?;

        // Two ports on one PCI function (Azure MANA) are told apart by MAC; two
        // ENAs by their functions. Either way the two roles must not collide.
        let distinct_macs = matches!((self.sender.mac, self.receiver.mac), (Some(s), Some(r)) if s != r);
        if self.sender.pci_address == self.receiver.pci_address && !distinct_macs {
            return Err(DpdkError::InvalidConfiguration(format!(
                "sender.pci_address and receiver.pci_address must differ (both are {}) unless both ports \
                 name distinct MACs on that function (Azure MANA)",
                self.sender.pci_address
            )));
        }
        if self.sender.mac.is_some() && self.sender.mac == self.receiver.mac {
            return Err(DpdkError::InvalidConfiguration(format!(
                "sender.mac and receiver.mac must differ (both are {})",
                self.sender.mac.map_or_else(String::new, |m| m.to_string())
            )));
        }

        if !valid_ident(&self.file_prefix, 64) {
            return Err(DpdkError::InvalidConfiguration(format!(
                "file_prefix {:?} must match [A-Za-z0-9_-]{{1,64}}",
                self.file_prefix
            )));
        }

        if !self.hugepage_dir.is_absolute() {
            return Err(DpdkError::InvalidConfiguration(format!(
                "hugepage_dir \"{}\" must be an absolute path to the hugetlbfs mount",
                self.hugepage_dir.display()
            )));
        }

        if let Some(rx) = self.rx_descriptors
            && !(64..=8192).contains(&rx)
        {
            return Err(DpdkError::InvalidConfiguration(format!(
                "rx_descriptors {rx} must be in 64..=8192"
            )));
        }
        if !(64..=8192).contains(&self.tx_descriptors) {
            return Err(DpdkError::InvalidConfiguration(format!(
                "tx_descriptors {} must be in 64..=8192",
                self.tx_descriptors
            )));
        }
        if !(1..=256).contains(&self.burst_size) {
            return Err(DpdkError::InvalidConfiguration(format!(
                "burst_size {} must be in 1..=256",
                self.burst_size
            )));
        }

        // DPDK's per-core mempool cache: none (0) up to 512 entries.
        if self.mempool_cache > 512 {
            return Err(DpdkError::InvalidConfiguration(format!(
                "mempool_cache {} is above DPDK's 512 (0 means no per-core cache)",
                self.mempool_cache
            )));
        }

        // The mbuf pool must cover both descriptor rings, four bursts of
        // headroom and what two lcores' mempool caches may hold (the port's
        // agent, and the EAL main lcore, which starts and stops the port),
        // each up to DPDK's flush threshold of 1.5 times the cache size.
        // Unset, the receive ring is sized at start: DEFAULT_RX_DESCRIPTORS
        // at most.
        let caches = 3 * u64::from(self.mempool_cache);
        let required =
            |rx: u16| u64::from(rx) + u64::from(self.tx_descriptors) + 4 * u64::from(self.burst_size) + caches;
        let rx = self.rx_descriptors.unwrap_or(DEFAULT_RX_DESCRIPTORS);
        let need = required(rx);
        if u64::from(self.mbufs_per_port) < need {
            let parts = format!(
                "rx descriptors {rx} + tx {} + 4 bursts of {} + two mempool caches of up to {} each",
                self.tx_descriptors,
                self.burst_size,
                caches / 2
            );
            return Err(DpdkError::InvalidConfiguration(match self.rx_descriptors {
                // A NIC capped lower (Azure MANA's 256) takes less at start,
                // but the pool is checked here for the most it can take.
                None => format!(
                    "mbufs_per_port {} is below the required {need} for a receive ring of up to {rx} \
                     (RUSTERON_DPDK_RX_DESCRIPTORS unset: min({rx}, the NIC's maximum)): {parts}. Enlarge the \
                     pool (RUSTERON_DPDK_MBUFS_PER_PORT={need}), or on a NIC capped at {MANA_MAX_DESCRIPTORS} \
                     descriptors (Azure MANA) set RUSTERON_DPDK_RX_DESCRIPTORS={MANA_MAX_DESCRIPTORS}, which \
                     needs {}",
                    self.mbufs_per_port,
                    required(MANA_MAX_DESCRIPTORS)
                ),
                Some(_) => format!(
                    "mbufs_per_port {} is below the required {need}: {parts}. Enlarge the pool \
                     (RUSTERON_DPDK_MBUFS_PER_PORT={need}), or set RUSTERON_DPDK_RX_DESCRIPTORS lower",
                    self.mbufs_per_port
                ),
            }));
        }

        if self.max_aeron_mtu == 0 || !self.max_aeron_mtu.is_multiple_of(32) || self.max_aeron_mtu > 8960 {
            return Err(DpdkError::InvalidConfiguration(format!(
                "max_aeron_mtu {} must be non-zero, 32-aligned and at most 8960",
                self.max_aeron_mtu
            )));
        }

        Ok(())
    }
}

fn validate_port(role: &str, port: &DpdkPortConfig, test_vdev: bool) -> Result<(), DpdkError> {
    let vdev = test_vdev && valid_ident(&port.pci_address, 15);
    if !(valid_pci(&port.pci_address) || vdev) {
        return Err(DpdkError::InvalidConfiguration(format!(
            "{role}.pci_address {:?} is not canonical dddd:bb:ss.f{}",
            port.pci_address,
            if test_vdev { " nor a DPDK vdev name" } else { "" }
        )));
    }
    if let Some(mac) = port.mac
        && !mac.is_unicast()
    {
        return Err(DpdkError::InvalidConfiguration(format!(
            "{role}.mac {mac} is not a unicast MAC address"
        )));
    }
    if !is_unicast(port.local_ipv4) {
        return Err(DpdkError::InvalidConfiguration(format!(
            "{role}.local_ipv4 {ip} is not a unicast IPv4 address",
            ip = port.local_ipv4
        )));
    }
    if !is_unicast(port.gateway_ipv4) {
        return Err(DpdkError::InvalidConfiguration(format!(
            "{role}.gateway_ipv4 {ip} is not a unicast IPv4 address",
            ip = port.gateway_ipv4
        )));
    }
    if !(1..=32).contains(&port.prefix_len) {
        return Err(DpdkError::InvalidConfiguration(format!(
            "{role}.prefix_len {} must be in 1..=32",
            port.prefix_len
        )));
    }

    // Gateway must be inside the configured local subnet.
    // `u32::from(Ipv4Addr)` keeps the first octet in the high byte, so the
    // prefix mask must set the *network* bits, i.e. `u32::MAX << (32 - len)`.
    let shift = 32 - u32::from(port.prefix_len);
    let mask = u32::MAX << shift;
    if u32::from(port.local_ipv4) & mask != u32::from(port.gateway_ipv4) & mask {
        return Err(DpdkError::InvalidConfiguration(format!(
            "{role}.gateway_ipv4 {gw} is not in the {ip}/{prefix} subnet of {role}.local_ipv4",
            gw = port.gateway_ipv4,
            ip = port.local_ipv4,
            prefix = port.prefix_len,
        )));
    }

    Ok(())
}

/// `std`'s `Ipv4Addr` has no stable `is_unicast`, so spell it out: not
/// multicast, not the limited broadcast, not `0.0.0.0`.
const fn is_unicast(addr: Ipv4Addr) -> bool {
    !addr.is_multicast() && !addr.is_broadcast() && !addr.is_unspecified()
}

/// Canonical PCI BDF syntax `dddd:bb:ss.f` (4-2-2 hex, dot, 1 hex).
///
/// Shape must match `is_bdf` in md-dpdk/device-plugin/src/inventory.rs —
/// the plugin refuses any inventory whose BDF this check would reject. (No
/// shared crate dependency; this comment is the sync contract.)
fn valid_pci(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 12 {
        return false;
    }
    let hex = |i: usize| b[i].is_ascii_hexdigit();
    hex(0)
        && hex(1)
        && hex(2)
        && hex(3)
        && b[4] == b':'
        && hex(5)
        && hex(6)
        && b[7] == b':'
        && hex(8)
        && hex(9)
        && b[10] == b'.'
        && hex(11)
}

/// `[A-Za-z0-9_-]` name of `1..=max_len` bytes. Shared by DPDK vdev names
/// (max 15, the 16-byte native selector buffer; no `:` so the EAL can tell a
/// vdev from a PCI BDF by shape) and EAL file prefixes (max 64).
fn valid_ident(s: &str, max_len: usize) -> bool {
    let n = s.len();
    n >= 1 && n <= max_len && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
