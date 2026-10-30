//! T24: the NIC pair the device plugin grants, reported as the two shortest
//! lines ergon-io's DPDK backends take. The pair is copied, never guessed,
//! and nothing here binds a NIC.

use std::net::Ipv4Addr;

use crate::finding::{Finding, Severity};

const T24_EVIDENCE: &str = "md-dpdk/device-plugin (it injects \
     RUSTERON_DPDK_{SENDER,RECEIVER}_{PCI,IPV4_CIDR,GATEWAY} with the \
     rusteron.io/dpdk-ena-pair it allocates) and md-dpdk/src/env.rs (the sender trio is required \
     at start); book/src/io/backends.md (the two shortest lines); not measured here";

/// The three sender values a granted pair always sets, in the order the
/// checks below validate them: PCI address, IPv4 cidr, gateway.
const SENDER: [&str; 3] = [
    "RUSTERON_DPDK_SENDER_PCI",
    "RUSTERON_DPDK_SENDER_IPV4_CIDR",
    "RUSTERON_DPDK_SENDER_GATEWAY",
];

/// Why a PCI value cannot be spliced into a line: not a
/// `[domain:]bus:dev.func` address and not a DPDK device name.
const PCI_BAD: &str = "is not a PCI address dddd:bb:dd.d or a DPDK device name";
/// Why a cidr value cannot be spliced into a line.
const CIDR_BAD: &str = "is not an IPv4 cidr a.b.c.d/len with a prefix of 0 to 32";
/// Why a gateway value cannot be spliced into a line.
const GATEWAY_BAD: &str = "is not an IPv4 address";

/// T24: a process on DPDK has the device plugin's NIC pair complete and
/// consistent.
///
/// `on_dpdk` is whether this process runs on DPDK: the media driver's
/// `dpdk-ena` transport, or ergon-io's `fstack-spin` or `smoltcp-spin`.
/// `env` looks a variable up (`|n| std::env::var(n).ok()` for this process).
/// Off the pair, or with all three sender values unset, the check is ok and
/// the message says the device plugin has not granted a pair, not a guessed
/// address. An empty or unparsable value warns with the exact key. All
/// three set and usable, the message carries the shortest `fstack-spin` and
/// `smoltcp-spin` lines built from them.
///
/// The check reads only: it exports nothing, binds no NIC, sets no
/// `AERON_DIR`, and never returns [`Status::Unknown`](crate::Status::Unknown),
/// so it never joins
/// [`unknown_stops_require`](crate::unknown_stops_require).
#[must_use]
pub fn t24_dpdk_nic_pair(on_dpdk: bool, env: impl Fn(&str) -> Option<String>) -> Finding {
    let id = "T24";
    let sev = Severity::Warn;
    if !on_dpdk {
        return Finding::ok(
            id,
            sev,
            T24_EVIDENCE,
            "this process is not an ergon-io DPDK backend and not a media driver with dpdk-ena, \
             so the NIC pair does not apply here"
                .to_owned(),
        );
    }
    let pair = match DpdkNicPair::from_env(env) {
        Ok(Some(pair)) => pair,
        Ok(None) => {
            return Finding::ok(
                id,
                sev,
                T24_EVIDENCE,
                format!(
                    "{} are unset: the device plugin has not granted a pair, so nothing here \
                     guesses an address, a gateway or a PCI device",
                    SENDER.join(", ")
                ),
            );
        }
        Err(problems) => {
            return Finding::found(
                id,
                sev,
                T24_EVIDENCE,
                format!(
                    "the NIC pair is not usable: {problems}. Effect: no fstack-spin or \
                     smoltcp-spin line can be copied from it. Fix: set every empty key and \
                     correct every bad one from the pair the device plugin granted \
                     (rusteron.io/dpdk-ena-pair); this check copies nothing, binds no NIC and \
                     sets no AERON_DIR"
                ),
            );
        }
    };
    Finding::found(
        id,
        Severity::Info,
        T24_EVIDENCE,
        format!(
            "the NIC pair is set; the two shortest lines built from it (copied, not guessed): \
             {}; {}. A bare fstack-spin or smoltcp-spin takes the same values itself. This \
             check copies nothing, binds no NIC and sets no AERON_DIR",
            pair.fstack_line(),
            pair.smoltcp_line()
        ),
    )
}

/// The NIC the ENA device plugin granted this container.
///
/// `rusteron.io/dpdk-ena-pair` injects its sender PCI address, IPv4 address
/// and prefix, and gateway as [`DpdkNicPair::ENV`]. ergon-io's `fstack-spin`
/// and `smoltcp-spin` take it when their configuration names no NIC; T24
/// reports it. Copied from what the kubelet allocated, never guessed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DpdkNicPair {
    /// `RUSTERON_DPDK_SENDER_PCI`: `[domain:]bus:dev.func`, or a DPDK device
    /// name (`net_tap0`) in the plugin's test mode.
    pub pci: String,
    /// The address of `RUSTERON_DPDK_SENDER_IPV4_CIDR`.
    pub addr: Ipv4Addr,
    /// The prefix length of `RUSTERON_DPDK_SENDER_IPV4_CIDR`, 0 to 32.
    pub prefix_len: u8,
    /// `RUSTERON_DPDK_SENDER_GATEWAY`.
    pub gateway: Ipv4Addr,
}

impl DpdkNicPair {
    /// The three variables the device plugin injects for the sender NIC,
    /// in the order they are checked.
    pub const ENV: [&'static str; 3] = SENDER;

    /// Reads the pair through `env` (`|n| std::env::var(n).ok()` for this
    /// process). `Ok(None)` when all three variables are unset: no pair was
    /// granted.
    ///
    /// # Errors
    /// Every empty or unparsable variable, by name and value, joined with
    /// `; `.
    pub fn from_env(env: impl Fn(&str) -> Option<String>) -> Result<Option<Self>, String> {
        // the trimmed values, in SENDER's order; None only where the variable is unset.
        let raw: [Option<String>; 3] = SENDER.map(|name| env(name).map(|v| v.trim().to_owned()));
        if raw.iter().all(Option::is_none) {
            return Ok(None);
        }
        let mut problems: Vec<String> = Vec::new();
        for (i, name) in SENDER.iter().enumerate() {
            match raw[i].as_deref() {
                None | Some("") => problems.push(format!("{name} is still empty")),
                Some(value) => {
                    let why = match i {
                        0 => pci_why(value),
                        1 => cidr_why(value),
                        _ => address_why(value),
                    };
                    if let Some(why) = why {
                        problems.push(format!("{name}={value} {why}"));
                    }
                }
            }
        }
        if !problems.is_empty() {
            return Err(problems.join("; "));
        }
        let [Some(pci), Some(cidr), Some(gateway)] = raw else {
            return Err("the NIC pair is incomplete".to_owned());
        };
        let (addr, prefix) = cidr.split_once('/').unwrap_or((cidr.as_str(), ""));
        match (addr.parse(), prefix.parse(), gateway.parse()) {
            (Ok(addr), Ok(prefix_len), Ok(gateway)) => Ok(Some(Self {
                pci,
                addr,
                prefix_len,
                gateway,
            })),
            _ => Err(format!("{} {CIDR_BAD}", SENDER[1])),
        }
    }

    /// Whether [`DpdkNicPair::pci`] is a PCI address (a NIC bound to
    /// `vfio-pci` or `igb_uio`) rather than a DPDK device name.
    #[must_use]
    pub fn is_pci_address(&self) -> bool {
        self.pci.contains(':')
    }

    /// The EAL arguments that give DPDK this one port: `-a <pci>`, or
    /// `--vdev=<name>` for a device name.
    #[must_use]
    pub fn eal_args(&self) -> [String; 2] {
        if self.is_pci_address() {
            ["-a".to_owned(), self.pci.clone()]
        } else {
            ["--vdev".to_owned(), self.pci.clone()]
        }
    }

    /// The shortest `fstack-spin` configuration string for this NIC.
    #[must_use]
    pub fn fstack_line(&self) -> String {
        let prefix = if self.prefix_len == 24 {
            String::new()
        } else {
            format!(",fstack_prefix_len={}", self.prefix_len)
        };
        let port = if self.is_pci_address() {
            "fstack_pci"
        } else {
            "fstack_vdev"
        };
        format!(
            "fstack-spin,fstack_addr={},fstack_gateway={},{port}={}{prefix}",
            self.addr, self.gateway, self.pci
        )
    }

    /// The shortest `smoltcp-spin` stack line for this NIC.
    #[must_use]
    pub fn smoltcp_line(&self) -> String {
        format!(
            "port={},ip={}/{},eal={}",
            self.pci,
            self.addr,
            self.prefix_len,
            self.eal_args().join(" ")
        )
    }
}

/// `Some(reason)` when `value` is not `[domain:]bus:dev.func` and not a
/// DPDK device name (`net_tap0`, `net_af_packet0`).
fn pci_why(value: &str) -> Option<&'static str> {
    if value.split_whitespace().count() != 1 || value.contains(',') || value.contains('=') {
        return Some(PCI_BAD);
    }
    if !value.contains(':') {
        // a DPDK device name, which the test vdev mode uses in place of a PCI address.
        return None;
    }
    let fields: Vec<&str> = value.split([':', '.']).collect();
    let bdf = match fields.as_slice() {
        [domain, bus, dev, func] => {
            is_hex(domain, 1, 4) && is_hex(bus, 2, 2) && is_hex(dev, 2, 2) && is_func(func)
        }
        [bus, dev, func] => is_hex(bus, 2, 2) && is_hex(dev, 2, 2) && is_func(func),
        _ => false,
    };
    if bdf { None } else { Some(PCI_BAD) }
}

/// `Some(reason)` when `value` is not `a.b.c.d/len` with a prefix of 0 to 32.
fn cidr_why(value: &str) -> Option<&'static str> {
    let Some((addr, prefix)) = value.split_once('/') else {
        return Some(CIDR_BAD);
    };
    if addr.parse::<Ipv4Addr>().is_err() {
        return Some(CIDR_BAD);
    }
    match prefix.parse::<u8>() {
        Ok(len) if len <= 32 => None,
        _ => Some(CIDR_BAD),
    }
}

/// `Some(reason)` when `value` is not an IPv4 address.
fn address_why(value: &str) -> Option<&'static str> {
    if value.parse::<Ipv4Addr>().is_ok() {
        None
    } else {
        Some(GATEWAY_BAD)
    }
}

/// A field of a PCI address: 1 to 4 hex digits (`min` to `max`).
fn is_hex(field: &str, min: usize, max: usize) -> bool {
    field.len() >= min && field.len() <= max && field.chars().all(|c| c.is_ascii_hexdigit())
}

/// The function number of a PCI address: one hex digit, 0 to 7.
fn is_func(field: &str) -> bool {
    field.len() == 1 && field.chars().all(|c| c.is_ascii_hexdigit()) && field <= "7"
}
