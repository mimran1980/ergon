//! Environment contract: selector, required values, and defaults.

use crate::config::{DpdkPortConfig, DpdkTransportConfig, MacAddr};
use crate::error::DpdkError;
use std::env;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::str::FromStr;

/// `RUSTERON_MEDIA_DRIVER_TRANSPORT` selector.
const SELECTOR: &str = "RUSTERON_MEDIA_DRIVER_TRANSPORT";

/// Parsed transport selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Selector {
    /// `default` (or absent) — unchanged socket-based behaviour.
    Default,
    /// `dpdk-ena` — use the DPDK ENA kernel-bypass transport.
    DpdkEna,
}

/// Parse the transport selector. Absent or `default` yields
/// [`Selector::Default`]; `dpdk-ena` yields [`Selector::DpdkEna`]; an empty or
/// unknown value is [`DpdkError::InvalidEnvironment`].
///
/// # Errors
///
/// [`DpdkError::InvalidEnvironment`] for an empty or unknown value.
pub fn selector() -> Result<Selector, DpdkError> {
    match env::var(SELECTOR) {
        Err(_) => Ok(Selector::Default),
        Ok(v) if v == "default" => Ok(Selector::Default),
        Ok(v) if v == "dpdk-ena" => Ok(Selector::DpdkEna),
        Ok(v) => Err(DpdkError::InvalidEnvironment(format!(
            "{SELECTOR}={v:?} must be `default` or `dpdk-ena`"
        ))),
    }
}

/// Parse a full transport configuration from the environment.
///
/// Required values are `RUSTERON_DPDK_FILE_PREFIX`, `RUSTERON_DPDK_SENDER_PCI`,
/// `RUSTERON_DPDK_SENDER_IPV4_CIDR`, `RUSTERON_DPDK_SENDER_GATEWAY`, and the
/// `RECEIVER_` twins. On Azure MANA, `RUSTERON_DPDK_SENDER_MAC` and
/// `RUSTERON_DPDK_RECEIVER_MAC` select each port on the shared PCI function.
/// Everything else falls back to the defaults below. `RUSTERON_DPDK_EAL_ARGS`
/// (read natively) appends EAL options such as log levels or lcore lists.
/// `RUSTERON_DPDK_RX_HW_TIMESTAMP=1` requests NIC hardware receive timestamps.
/// Read natively at start (see the README): `RUSTERON_DPDK_EPHEMERAL_PORT_RANGE`,
/// `RUSTERON_DPDK_{SENDER,RECEIVER}_EXTRA_IPV4`, `RUSTERON_DPDK_HOST_LOCAL`,
/// `RUSTERON_DPDK_WILDCARD_LOCAL`, `RUSTERON_DPDK_LOCAL_IO`, `RUSTERON_DPDK_LOCAL_HOT_US`, `RUSTERON_DPDK_LOCAL_UNWATCH`, `RUSTERON_DPDK_TX_RETRY_US`, `RUSTERON_DPDK_NIC_STATS_MS`, `RUSTERON_DPDK_RX_MAX_BURSTS`, `RUSTERON_DPDK_GAP_ATTRIB`, `RUSTERON_DPDK_TX_RATE_MBPS`, `RUSTERON_DPDK_TX_ROUTE_MBPS`,
/// `RUSTERON_DPDK_TX_BUCKET_BYTES`, `RUSTERON_DPDK_TX_PACE_SPIN_US`, `RUSTERON_DPDK_TX_LOW_STREAMS`,
/// `RUSTERON_DPDK_TX_LOW_RESERVE_BYTES`, `RUSTERON_DPDK_HOUSEKEEPING_CPU`,
/// `RUSTERON_DPDK_ENA_DEVARGS`, `RUSTERON_DPDK_ENA_LLQ_REQUIRED`,
/// `RUSTERON_DPDK_EAL_IN_MEMORY`, `RUSTERON_DPDK_EVENT_LOG` and
/// `RUSTERON_DPDK_VFIO_WC`. `RUSTERON_DPDK_CLOCK` is read by the install
/// ([`crate::clock`]), whatever made the configuration.
/// `RUSTERON_DPDK_RX_DESCRIPTORS` unset leaves
/// [`DpdkTransportConfig::rx_descriptors`] `None`: each port then takes
/// min(4096, the NIC's maximum) at start. `RUSTERON_DPDK_RX_FREE_THRESH` and
/// `RUSTERON_DPDK_TX_FREE_THRESH` unset keep the PMD's queue defaults, and
/// `RUSTERON_DPDK_TX_FAST_FREE` (0/1/true/false) is off unless set.
///
/// `RUSTERON_DPDK_TEST_VDEV=1` enables the test-only virtual-device mode: sender/receiver PCI may then be DPDK vdev names (e.g. `net_tap0`)
/// instead of ENA PCI BDFs. Production configurations never set it.
///
/// # Errors
///
/// [`DpdkError::MissingEnvironment`] for an absent required variable, and
/// [`DpdkError::InvalidEnvironment`] for a value that does not parse.
/// Cross-field validation happens later, in [`DpdkTransportConfig::validate`].
pub fn config_from_env() -> Result<DpdkTransportConfig, DpdkError> {
    let sender_cidr = required("RUSTERON_DPDK_SENDER_IPV4_CIDR")?;
    let receiver_cidr = required("RUSTERON_DPDK_RECEIVER_IPV4_CIDR")?;
    let (sender_addr, sender_prefix) = parse_cidr("RUSTERON_DPDK_SENDER_IPV4_CIDR", &sender_cidr)?;
    let (receiver_addr, receiver_prefix) = parse_cidr("RUSTERON_DPDK_RECEIVER_IPV4_CIDR", &receiver_cidr)?;
    let test_vdev = env::var("RUSTERON_DPDK_TEST_VDEV").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));

    Ok(DpdkTransportConfig {
        sender: DpdkPortConfig {
            pci_address: required("RUSTERON_DPDK_SENDER_PCI")?,
            mac: mac_or_none("RUSTERON_DPDK_SENDER_MAC")?,
            local_ipv4: sender_addr,
            prefix_len: sender_prefix,
            gateway_ipv4: ipv4(
                "RUSTERON_DPDK_SENDER_GATEWAY",
                &required("RUSTERON_DPDK_SENDER_GATEWAY")?,
            )?,
        },
        receiver: DpdkPortConfig {
            pci_address: required("RUSTERON_DPDK_RECEIVER_PCI")?,
            mac: mac_or_none("RUSTERON_DPDK_RECEIVER_MAC")?,
            local_ipv4: receiver_addr,
            prefix_len: receiver_prefix,
            gateway_ipv4: ipv4(
                "RUSTERON_DPDK_RECEIVER_GATEWAY",
                &required("RUSTERON_DPDK_RECEIVER_GATEWAY")?,
            )?,
        },
        file_prefix: required("RUSTERON_DPDK_FILE_PREFIX")?,
        test_vdev, // production selectors are PCI-only unless the test lever is set
        hugepage_dir: PathBuf::from(env::var("RUSTERON_DPDK_HUGE_DIR").unwrap_or_else(|_| "/dev/hugepages".into())),
        rx_descriptors: num_opt("RUSTERON_DPDK_RX_DESCRIPTORS")?, // unset: min(4096, the NIC's maximum) at start
        tx_descriptors: num_or("RUSTERON_DPDK_TX_DESCRIPTORS", 512)?, // an ENA's maximum
        mbufs_per_port: num_or("RUSTERON_DPDK_MBUFS_PER_PORT", 16384)?, // as the lab measured
        mempool_cache: num_or("RUSTERON_DPDK_MEMPOOL_CACHE", 256)?,
        burst_size: num_or("RUSTERON_DPDK_BURST_SIZE", 32)?,
        max_aeron_mtu: num_or("RUSTERON_DPDK_MAX_AERON_MTU", 1408)?,
        rx_hw_timestamp: flag_or("RUSTERON_DPDK_RX_HW_TIMESTAMP", false)?,
        rx_free_thresh: num_opt("RUSTERON_DPDK_RX_FREE_THRESH")?, // unset: the PMD's default
        tx_free_thresh: num_opt("RUSTERON_DPDK_TX_FREE_THRESH")?,
        tx_fast_free: flag_or("RUSTERON_DPDK_TX_FAST_FREE", false)?,
    })
}

/// An optional `aa:bb:cc:dd:ee:ff` value (the Azure MANA port selectors).
fn mac_or_none(name: &str) -> Result<Option<MacAddr>, DpdkError> {
    env::var(name).map_or(Ok(None), |v| {
        v.trim()
            .parse()
            .map(Some)
            .map_err(|_| DpdkError::InvalidEnvironment(format!("{name}={v:?} is not a MAC address aa:bb:cc:dd:ee:ff")))
    })
}

/// An optional boolean: `1`/`true` or `0`/`false` (any case).
fn flag_or(name: &str, default: bool) -> Result<bool, DpdkError> {
    env::var(name).map_or(Ok(default), |v| match v.trim().to_ascii_lowercase().as_str() {
        "1" | "true" => Ok(true),
        "0" | "false" => Ok(false),
        _ => Err(DpdkError::InvalidEnvironment(format!(
            "{name}={v:?} is not 0, 1, true or false"
        ))),
    })
}

fn required(name: &str) -> Result<String, DpdkError> {
    env::var(name).map_err(|_| DpdkError::MissingEnvironment(name.to_string()))
}

fn ipv4(name: &str, value: &str) -> Result<Ipv4Addr, DpdkError> {
    value
        .parse()
        .map_err(|_| DpdkError::InvalidEnvironment(format!("{name}={value:?} is not a valid IPv4 address")))
}

/// Parse `address/prefix` into the address and prefix length. `var` is the full
/// env var name (e.g. `RUSTERON_DPDK_SENDER_IPV4_CIDR`), used verbatim in errors
/// so a role typo can never produce a misleading name.
fn parse_cidr(var: &str, value: &str) -> Result<(Ipv4Addr, u8), DpdkError> {
    let (ip, prefix) = value.split_once('/').ok_or_else(|| {
        DpdkError::InvalidEnvironment(format!("{var}={value:?} must be address/prefix (e.g. 10.0.0.1/24)"))
    })?;
    let addr = ipv4(var, ip)?;
    let prefix_len: u8 = prefix
        .trim()
        .parse()
        .map_err(|_| DpdkError::InvalidEnvironment(format!("{var} prefix {prefix:?} is not a number")))?;
    if !(1..=32).contains(&prefix_len) {
        return Err(DpdkError::InvalidEnvironment(format!(
            "{var} prefix {prefix_len} must be in 1..=32"
        )));
    }
    Ok((addr, prefix_len))
}

/// Parse an optional numeric environment value: `None` when absent.
fn num_opt<T: FromStr>(name: &str) -> Result<Option<T>, DpdkError>
where
    T::Err: std::fmt::Display,
{
    env::var(name).map_or(Ok(None), |v| {
        v.trim()
            .parse()
            .map(Some)
            .map_err(|_| DpdkError::InvalidEnvironment(format!("{name}={v:?} is not a valid number")))
    })
}

/// Parse a numeric environment value, using `default` when absent.
fn num_or<T: FromStr>(name: &str, default: T) -> Result<T, DpdkError>
where
    T::Err: std::fmt::Display,
{
    env::var(name).map_or_else(
        |_| Ok(default),
        |v| {
            v.trim()
                .parse()
                .map_err(|_| DpdkError::InvalidEnvironment(format!("{name}={v:?} is not a valid number")))
        },
    )
}
