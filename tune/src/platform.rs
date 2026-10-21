//! What kind of host this is, for the wording that differs per platform
//! (Graviton has no SMT).

use crate::host::{Host, first_line};

/// The CPU architecture, from `/proc/cpuinfo`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Arch {
    /// x86-64 (`vendor_id` lines).
    X86_64,
    /// 64-bit Arm (`CPU implementer` lines).
    Aarch64,
    /// Neither, or not readable.
    Unknown,
}

/// The cloud, from the DMI system vendor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Cloud {
    /// `Amazon EC2`.
    Aws,
    /// `Microsoft Corporation` (Azure, or Hyper-V elsewhere).
    Azure,
    /// Another vendor, or none readable.
    Other,
}

/// The host's architecture, cloud and instance type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Platform {
    /// The CPU architecture.
    pub arch: Arch,
    /// The cloud.
    pub cloud: Cloud,
    /// The DMI product name: the instance type on AWS Nitro
    /// (`c9g.xlarge`), `Virtual Machine` on Azure.
    pub product: Option<String>,
}

impl Platform {
    /// Reads `/proc/cpuinfo` and `/sys/class/dmi/id/{sys_vendor,product_name}`.
    /// What cannot be read is [`Arch::Unknown`] or [`Cloud::Other`]: the
    /// platform only changes wording, never whether a check fires.
    #[must_use]
    pub fn detect(host: &Host) -> Self {
        let arch = host
            .read("/proc/cpuinfo")
            .map_or(Arch::Unknown, |t| arch_of(&t));
        let vendor = host
            .read("/sys/class/dmi/id/sys_vendor")
            .map(|t| first_line(&t))
            .unwrap_or_default();
        let cloud = match vendor.as_str() {
            "Amazon EC2" => Cloud::Aws,
            "Microsoft Corporation" => Cloud::Azure,
            _ => Cloud::Other,
        };
        let product = host
            .read("/sys/class/dmi/id/product_name")
            .map(|t| first_line(&t))
            .ok()
            .filter(|p| !p.is_empty());
        Self {
            arch,
            cloud,
            product,
        }
    }

    /// An AWS Graviton instance: 64-bit Arm on EC2. One thread per core
    /// (no SMT).
    #[must_use]
    pub fn is_graviton(&self) -> bool {
        self.arch == Arch::Aarch64 && self.cloud == Cloud::Aws
    }
}

pub fn arch_of(cpuinfo: &str) -> Arch {
    for line in cpuinfo.lines() {
        let key = line.split(':').next().unwrap_or_default().trim();
        match key {
            "vendor_id" => return Arch::X86_64,
            "CPU implementer" => return Arch::Aarch64,
            _ => {}
        }
    }
    Arch::Unknown
}
