//! DPDK transport error contract.

use std::fmt;

/// Errors from selecting, configuring, and installing the DPDK ENA transport.
///
/// Every variant carries enough context to identify the failed operation and
/// the exact invalid input — variable name, config field, PCI address, or port
/// role. No variant implies a silent fallback to the socket transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DpdkError {
    /// `dpdk-ena` was selected but the crate was not built with the `dpdk` feature.
    FeatureDisabled,
    /// [`DpdkTransport::close`](crate::DpdkTransport::close) was called while
    /// other handles to the same native instance were still alive, so nothing
    /// was freed. Close the last handle instead.
    StillHeld,
    /// A required environment variable is absent.
    MissingEnvironment(String),
    /// A selector or environment value cannot be parsed.
    InvalidEnvironment(String),
    /// A typed configuration violates a cross-field invariant.
    InvalidConfiguration(String),
    /// DPDK or the native transport returned an error code and message.
    Native(String),
    /// Installation through the Aeron context failed.
    Aeron(String),
    /// Under `ERGON_HOST_CHECKS=require`, a host check warned
    /// ([`crate::checks`]): the driver was not started. Names the warnings,
    /// which were printed in full.
    HostCheck(String),
}

impl fmt::Display for DpdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FeatureDisabled => write!(
                f,
                "the `dpdk-ena` transport was selected but ergon-md-dpdk was not built with the `dpdk` feature"
            ),
            Self::StillHeld => write!(
                f,
                "other handles to the native DPDK transport are still alive, so nothing was freed; close the last handle instead"
            ),
            Self::MissingEnvironment(var) => write!(f, "missing required environment variable {var}"),
            Self::InvalidEnvironment(msg) => write!(f, "invalid environment value: {msg}"),
            Self::InvalidConfiguration(msg) => write!(f, "invalid DPDK configuration: {msg}"),
            Self::Native(msg) => write!(f, "DPDK native error: {msg}"),
            Self::Aeron(msg) => write!(f, "Aeron installation error: {msg}"),
            Self::HostCheck(msg) => write!(f, "host check refused to start the driver: {msg}"),
        }
    }
}

impl std::error::Error for DpdkError {}
