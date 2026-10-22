//! Pair inventory (`/var/lib/rusteron-dpdk/ena-pairs.json`, plan §10.2): the
//! exact shape written by `deploy/aws-eks/node/bootstrap-dpdk-ena.sh`, plus
//! strict validation.
//!
//! Each port names the userspace driver it is bound to: `vfio-pci` (the
//! default, so inventories written before the field still load) or `igb_uio`
//! (virtualised EC2, where it is the only binding that maps the LLQ BAR
//! write-combining). The plugin refuses any inventory with duplicate BDFs, an
//! unsupported driver, a vfio-pci port without a unique IOMMU group, an
//! igb_uio port without a unique `uioN` device, a member MAC equal to the
//! primary ENA, or incomplete network identity (plan §10.3) — it must never
//! advertise or allocate an unsafe pair.
//!
//! [`InventoryFile`] re-reads and revalidates the file whenever it changes, so
//! a bootstrap rerun (which deletes the inventory, rebinds the pair, possibly
//! under new uio numbers, and writes it again) reaches a running plugin.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

/// One node's pair inventory.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Inventory {
    #[serde(default)]
    pub generated_at: String,
    pub primary_ena_mac: String,
    pub pairs: Vec<Pair>,
}

/// One sender/receiver ENA pair.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Pair {
    pub id: String,
    pub sender: Port,
    pub receiver: Port,
}

/// Driver name of a VFIO-bound port; its device node is `/dev/vfio/<group>`.
pub const VFIO_PCI: &str = "vfio-pci";
/// Driver name of a UIO-bound port; its device node is `/dev/<uio>`.
pub const IGB_UIO: &str = "igb_uio";

/// Network identity of one ENA role. Missing identity fields default so that
/// validation can reject them with a precise reason instead of a parse error.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Port {
    pub pci: String,
    /// Kernel driver the BDF is bound to: [`VFIO_PCI`] or [`IGB_UIO`].
    #[serde(default = "default_driver")]
    pub driver: String,
    /// IOMMU group; required (and unique) only for [`VFIO_PCI`].
    #[serde(default)]
    pub iommu_group: String,
    /// UIO device name (`uioN`); required (and unique) only for [`IGB_UIO`].
    #[serde(default)]
    pub uio: String,
    pub eni_id: String,
    pub mac: String,
    pub ipv4: String,
    #[serde(default)]
    pub prefix_len: u8,
    #[serde(default)]
    pub subnet_cidr: String,
    #[serde(default)]
    pub gateway: String,
    #[serde(default = "default_numa")]
    pub numa_node: i32,
    #[serde(default = "default_health")]
    pub health: String,
}

fn default_driver() -> String {
    VFIO_PCI.into()
}

fn default_numa() -> i32 {
    -1
}

fn default_health() -> String {
    "healthy".into()
}

/// Why an inventory was rejected. Kubelet must never see a pair the plugin
/// cannot allocate safely.
#[derive(Debug)]
pub enum InventoryError {
    /// File could not be read.
    Io(String, std::io::Error),
    /// JSON did not parse.
    Parse(String),
    /// Validation rule violated (message is the reason).
    Validation(String),
}

impl fmt::Display for InventoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InventoryError::Io(path, e) => write!(f, "cannot read {path}: {e}"),
            InventoryError::Parse(e) => write!(f, "malformed inventory JSON: {e}"),
            InventoryError::Validation(msg) => write!(f, "unsafe inventory: {msg}"),
        }
    }
}

impl std::error::Error for InventoryError {}

impl Inventory {
    /// Load and validate the inventory at `path`.
    pub fn load(path: &Path) -> Result<Inventory, InventoryError> {
        let raw = std::fs::read_to_string(path).map_err(|e| InventoryError::Io(path.display().to_string(), e))?;
        let inv: Inventory = serde_json::from_str(&raw).map_err(|e| InventoryError::Parse(e.to_string()))?;
        inv.validate()?;
        Ok(inv)
    }

    /// Reject duplicate/unsafe inventories (plan §10.3).
    pub fn validate(&self) -> Result<(), InventoryError> {
        if self.primary_ena_mac.trim().is_empty() {
            return Err(InventoryError::Validation("missing primary_ena_mac".into()));
        }
        if self.pairs.is_empty() {
            return Err(InventoryError::Validation("no ENA pairs".into()));
        }
        let primary = self.primary_ena_mac.trim().to_ascii_lowercase();
        let mut ids = HashSet::new();
        let mut bdfs = HashSet::new();
        let mut groups = HashSet::new();
        let mut uios = HashSet::new();
        for (i, pair) in self.pairs.iter().enumerate() {
            if !ids.insert(pair.id.as_str()) {
                return Err(InventoryError::Validation(format!("duplicate pair id {:?}", pair.id)));
            }
            for (role, port) in [("sender", &pair.sender), ("receiver", &pair.receiver)] {
                validate_port(port)
                    .map_err(|reason| InventoryError::Validation(format!("pair {i} {role}: {reason}")))?;
                if !bdfs.insert(port.pci.as_str()) {
                    return Err(InventoryError::Validation(format!(
                        "duplicate BDF {} in pair {i} {role}",
                        port.pci
                    )));
                }
                if port.driver == VFIO_PCI && !groups.insert(port.iommu_group.as_str()) {
                    return Err(InventoryError::Validation(format!(
                        "duplicate IOMMU group {} in pair {i} {role}",
                        port.iommu_group
                    )));
                }
                if port.driver == IGB_UIO && !uios.insert(port.uio.as_str()) {
                    return Err(InventoryError::Validation(format!(
                        "duplicate uio device {} in pair {i} {role}",
                        port.uio
                    )));
                }
                if port.mac.trim().to_ascii_lowercase() == primary {
                    return Err(InventoryError::Validation(format!(
                        "pair {i} {role} {} is the primary ENA",
                        port.pci
                    )));
                }
            }
            if pair.sender.numa_node >= 0
                && pair.receiver.numa_node >= 0
                && pair.sender.numa_node != pair.receiver.numa_node
            {
                return Err(InventoryError::Validation(format!(
                    "pair {i} sender NUMA node {} and receiver NUMA node {} differ; both ENAs must be on one NUMA node",
                    pair.sender.numa_node, pair.receiver.numa_node
                )));
            }
        }
        Ok(())
    }
}

/// The inventory file as last read. It is read again, and validated again,
/// whenever its modification time or inode changes (the bootstrap renames a
/// new file over it), and while it is missing, unreadable or unsafe no pair is
/// served, never the one read before.
#[derive(Debug)]
pub struct InventoryFile {
    /// `None` for a fixed inventory that is never re-read (tests).
    path: Option<PathBuf>,
    stamp: Option<FileStamp>,
    current: Option<Arc<Inventory>>,
}

/// What tells one version of the file from the next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileStamp {
    modified: SystemTime,
    inode: u64,
}

fn stamp_of(path: &Path) -> Option<FileStamp> {
    let meta = std::fs::metadata(path).ok()?;
    Some(FileStamp {
        modified: meta.modified().ok()?,
        inode: meta.ino(),
    })
}

impl InventoryFile {
    /// Read and validate the inventory at `path`. The plugin refuses to start
    /// on an error.
    pub fn open(path: &Path) -> Result<Self, InventoryError> {
        let stamp = stamp_of(path);
        let inventory = Inventory::load(path)?;
        Ok(Self {
            path: Some(path.to_path_buf()),
            stamp,
            current: Some(Arc::new(inventory)),
        })
    }

    /// An inventory held in memory and never re-read.
    #[cfg(test)]
    pub fn fixed(inventory: Inventory) -> Self {
        Self {
            path: None,
            stamp: None,
            current: Some(Arc::new(inventory)),
        }
    }

    /// The inventory to serve, read and validated again first if the file
    /// changed since the last call. `None` while the file is missing,
    /// unreadable or unsafe.
    pub fn current(&mut self) -> Option<Arc<Inventory>> {
        if let Some(path) = &self.path {
            let stamp = stamp_of(path);
            if stamp != self.stamp {
                self.stamp = stamp;
                self.current = match Inventory::load(path) {
                    Ok(inventory) => {
                        log::info!("reloaded {} ENA pair(s) from {}", inventory.pairs.len(), path.display());
                        Some(Arc::new(inventory))
                    }
                    Err(e) => {
                        log::warn!("{e}: no ENA pair is advertised until {} is valid again", path.display());
                        None
                    }
                };
            }
        }
        self.current.clone()
    }
}

fn validate_port(port: &Port) -> Result<(), String> {
    if port.pci.trim().is_empty() {
        return Err("empty pci".into());
    }
    if !is_bdf(&port.pci) {
        return Err(format!("pci {:?} is not a BDF (DDDD:BB:DD.F)", port.pci));
    }
    match port.driver.as_str() {
        VFIO_PCI => {
            if port.iommu_group.trim().is_empty() {
                return Err("empty iommu_group".into());
            }
        }
        IGB_UIO => {
            if !is_uio_name(&port.uio) {
                return Err(format!("uio {:?} is not a uio device name (uioN)", port.uio));
            }
        }
        other => {
            return Err(format!(
                "unsupported driver {other:?} (expected {VFIO_PCI} or {IGB_UIO})"
            ));
        }
    }
    if port.eni_id.trim().is_empty() {
        return Err("empty eni_id".into());
    }
    if port.mac.trim().is_empty() {
        return Err("empty mac".into());
    }
    if port.ipv4.trim().is_empty() {
        return Err("empty ipv4".into());
    }
    if port.subnet_cidr.trim().is_empty() {
        return Err("empty subnet_cidr".into());
    }
    if port.gateway.trim().is_empty() {
        return Err("empty gateway".into());
    }
    if !(1..=32).contains(&port.prefix_len) {
        return Err(format!("prefix_len {} not in 1..=32", port.prefix_len));
    }
    if port.numa_node < -1 {
        return Err(format!("numa_node {} < -1", port.numa_node));
    }
    Ok(())
}

/// `uio<digits>`, the name the kernel gives a UIO device (the basename of
/// `/sys/bus/pci/devices/<bdf>/uio/uio*`, and of its `/dev` node).
fn is_uio_name(s: &str) -> bool {
    s.strip_prefix("uio")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

/// Loose check for the `DDDD:BB:DD.F` BDF layout the node bootstrap writes
/// (`basename` of a `/sys/bus/pci/devices/*` directory, e.g. `0000:00:06.0`).
///
/// Shape must match `valid_pci` in md-dpdk/src/config.rs —
/// keep the two in sync if either changes (the media driver is the canonical
/// `dddd:bb:ss.f` check, and DPDK itself parses the function slot as hex).
fn is_bdf(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 12
        && b[4] == b':'
        && b[7] == b':'
        && b[10] == b'.'
        && b[..4]
            .iter()
            .chain(&b[5..7])
            .chain(&b[8..10])
            .all(u8::is_ascii_hexdigit)
        && b[11].is_ascii_hexdigit()
}

/// Shared test fixtures (used by the inventory and service test suites).
#[cfg(test)]
pub(crate) mod fixtures {
    use super::{Inventory, Pair, Port};

    pub(crate) fn port(pci: &str, group: &str, mac: &str) -> Port {
        Port {
            pci: pci.into(),
            driver: "vfio-pci".into(),
            iommu_group: group.into(),
            uio: String::new(),
            eni_id: "eni-0123456789abcdef0".into(),
            mac: mac.into(),
            ipv4: "10.0.0.1".into(),
            prefix_len: 24,
            subnet_cidr: "10.0.0.0/24".into(),
            gateway: "10.0.0.1".into(),
            numa_node: 0,
            health: "healthy".into(),
        }
    }

    /// An igb_uio-bound port: no IOMMU group, a `/dev/uioN` node instead.
    pub(crate) fn uio_port(pci: &str, uio: &str, mac: &str) -> Port {
        Port {
            driver: "igb_uio".into(),
            iommu_group: String::new(),
            uio: uio.into(),
            ..port(pci, "", mac)
        }
    }

    pub(crate) fn fixture() -> Inventory {
        Inventory {
            generated_at: "2026-08-07T00:00:00Z".into(),
            primary_ena_mac: "0a:00:00:00:00:01".into(),
            pairs: vec![Pair {
                id: "dpdk-pair-0".into(),
                sender: port("0000:00:06.0", "9", "0a:00:00:00:00:02"),
                receiver: port("0000:00:07.0", "10", "0a:00:00:00:00:03"),
            }],
        }
    }

    /// The same pair as [`fixture`], bound to igb_uio as on virtualised EC2.
    pub(crate) fn uio_fixture() -> Inventory {
        Inventory {
            pairs: vec![Pair {
                id: "dpdk-pair-0".into(),
                sender: uio_port("0000:00:06.0", "uio0", "0a:00:00:00:00:02"),
                receiver: uio_port("0000:00:07.0", "uio1", "0a:00:00:00:00:03"),
            }],
            ..fixture()
        }
    }

    /// A scratch directory under the system temp dir, removed on drop.
    pub(crate) struct ScratchDir(pub(crate) std::path::PathBuf);

    impl ScratchDir {
        pub(crate) fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("rusteron-dpdk-inventory-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Replace `path` the way the node bootstrap does (a temporary file renamed
    /// over it), with its mtime `secs` seconds past the epoch.
    pub(crate) fn rewrite(path: &std::path::Path, json: &str, secs: u64) {
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, json).unwrap();
        let mtime = std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs);
        std::fs::File::options()
            .write(true)
            .open(&tmp)
            .unwrap()
            .set_modified(mtime)
            .unwrap();
        std::fs::rename(&tmp, path).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

    #[test]
    fn accepts_valid_inventory() {
        fixture().validate().unwrap();
    }

    #[test]
    fn accepts_fixture_json_from_bootstrap_shape() {
        let json = r#"{
          "generated_at": "2026-08-07T00:00:00Z",
          "primary_ena_mac": "0a:00:00:00:00:01",
          "pairs": [{
            "id": "dpdk-pair-0",
            "sender":  {"pci":"0000:00:06.0","iommu_group":"9","eni_id":"eni-a","mac":"0a:00:00:00:00:02","ipv4":"10.0.0.1","prefix_len":24,"subnet_cidr":"10.0.0.0/24","gateway":"10.0.0.1","numa_node":0,"health":"healthy"},
            "receiver":{"pci":"0000:00:07.0","iommu_group":"10","eni_id":"eni-b","mac":"0a:00:00:00:00:03","ipv4":"10.0.0.2","prefix_len":24,"subnet_cidr":"10.0.0.0/24","gateway":"10.0.0.1","numa_node":0,"health":"healthy"}
          }]
        }"#;
        let inv: Inventory = serde_json::from_str(json).unwrap();
        inv.validate().unwrap();
        // Inventories written before the `driver` field are vfio-pci.
        assert_eq!(inv.pairs[0].sender.driver, "vfio-pci");
        assert!(inv.pairs[0].sender.uio.is_empty());
    }

    #[test]
    fn accepts_igb_uio_json_without_iommu_group() {
        let json = r#"{
          "generated_at": "2026-10-14T00:00:00Z",
          "primary_ena_mac": "0a:00:00:00:00:01",
          "pairs": [{
            "id": "dpdk-pair-0",
            "sender":  {"pci":"0000:00:06.0","driver":"igb_uio","uio":"uio0","eni_id":"eni-a","mac":"0a:00:00:00:00:02","ipv4":"10.0.0.1","prefix_len":24,"subnet_cidr":"10.0.0.0/24","gateway":"10.0.0.1","numa_node":0,"health":"healthy"},
            "receiver":{"pci":"0000:00:07.0","driver":"igb_uio","uio":"uio1","iommu_group":"","eni_id":"eni-b","mac":"0a:00:00:00:00:03","ipv4":"10.0.0.2","prefix_len":24,"subnet_cidr":"10.0.0.0/24","gateway":"10.0.0.1","numa_node":0,"health":"healthy"}
          }]
        }"#;
        let inv: Inventory = serde_json::from_str(json).unwrap();
        inv.validate().unwrap();
        assert_eq!(inv.pairs[0].sender.driver, "igb_uio");
        assert_eq!(inv.pairs[0].receiver.uio, "uio1");
    }

    #[test]
    fn accepts_valid_igb_uio_inventory() {
        uio_fixture().validate().unwrap();
    }

    #[test]
    fn a_pair_on_two_numa_nodes_is_rejected() {
        let mut inv = fixture();
        inv.pairs[0].sender.numa_node = 0;
        inv.pairs[0].receiver.numa_node = 1;
        let err = inv.validate().unwrap_err().to_string();
        assert!(err.contains("NUMA node 0") && err.contains("NUMA node 1"), "{err}");
    }

    #[test]
    fn an_unknown_numa_node_does_not_reject_the_pair() {
        let mut inv = fixture();
        inv.pairs[0].receiver.numa_node = -1;
        inv.validate().unwrap();
    }

    #[test]
    fn vfio_pci_requires_iommu_group() {
        let mut inv = fixture();
        inv.pairs[0].sender.iommu_group.clear();
        let err = inv.validate().unwrap_err().to_string();
        assert!(err.contains("empty iommu_group"), "{err}");
    }

    #[test]
    fn igb_uio_ignores_iommu_group() {
        // igb_uio needs the IOMMU off, so a group is neither required nor
        // checked for uniqueness.
        let mut inv = uio_fixture();
        inv.pairs[0].sender.iommu_group = "9".into();
        inv.pairs[0].receiver.iommu_group = "9".into();
        inv.validate().unwrap();
    }

    #[test]
    fn igb_uio_requires_uio_device_name() {
        for bad in ["", "uio", "uioX", "uio-1", "/dev/uio0", "uio0 ", "UIO0"] {
            let mut inv = uio_fixture();
            inv.pairs[0].sender.uio = bad.into();
            let err = inv.validate().unwrap_err().to_string();
            assert!(err.contains("uio"), "{bad:?}: {err}");
        }
        let mut inv = uio_fixture();
        inv.pairs[0].sender.uio = "uio12".into();
        inv.pairs[0].receiver.uio = "uio3".into();
        inv.validate().unwrap();
    }

    #[test]
    fn rejects_duplicate_uio() {
        let mut inv = uio_fixture();
        inv.pairs[0].receiver.uio = "uio0".into();
        let err = inv.validate().unwrap_err().to_string();
        assert!(err.contains("duplicate uio device uio0"), "{err}");
    }

    #[test]
    fn rejects_unknown_driver() {
        for bad in ["", "ena", "uio_pci_generic", "vfio"] {
            let mut inv = fixture();
            inv.pairs[0].receiver.driver = bad.into();
            let err = inv.validate().unwrap_err().to_string();
            assert!(err.contains("unsupported driver"), "{bad:?}: {err}");
        }
    }

    #[test]
    fn mixed_drivers_keep_their_own_rules() {
        let mut inv = fixture();
        inv.pairs[0].receiver = uio_port("0000:00:07.0", "uio0", "0a:00:00:00:00:03");
        inv.validate().unwrap();
        inv.pairs[0].receiver.uio.clear();
        assert!(inv.validate().is_err());
    }

    #[test]
    fn rejects_duplicate_bdf() {
        let mut inv = fixture();
        inv.pairs[0].receiver.pci = "0000:00:06.0".into();
        let err = inv.validate().unwrap_err().to_string();
        assert!(err.contains("duplicate BDF"), "{err}");
    }

    #[test]
    fn rejects_duplicate_group() {
        let mut inv = fixture();
        inv.pairs[0].receiver.iommu_group = "9".into();
        let err = inv.validate().unwrap_err().to_string();
        assert!(err.contains("duplicate IOMMU group"), "{err}");
    }

    #[test]
    fn rejects_primary_ena_in_pair() {
        let mut inv = fixture();
        inv.pairs[0].sender.mac = inv.primary_ena_mac.clone();
        let err = inv.validate().unwrap_err().to_string();
        assert!(err.contains("primary ENA"), "{err}");
    }

    #[test]
    fn rejects_incomplete_identity() {
        let mut inv = fixture();
        inv.pairs[0].sender.gateway.clear();
        let err = inv.validate().unwrap_err().to_string();
        assert!(err.contains("empty gateway"), "{err}");
    }

    #[test]
    fn rejects_bad_bdf_and_prefix() {
        let mut inv = fixture();
        inv.pairs[0].sender.pci = "00:06.0".into();
        assert!(inv.validate().is_err());
        inv = fixture();
        inv.pairs[0].receiver.prefix_len = 33;
        assert!(inv.validate().is_err());
    }

    #[test]
    fn rejects_empty_pairs_and_duplicate_ids() {
        let mut inv = fixture();
        inv.pairs.clear();
        assert!(inv.validate().is_err());
        inv = fixture();
        inv.pairs.push(inv.pairs[0].clone());
        let err = inv.validate().unwrap_err().to_string();
        assert!(err.contains("duplicate pair id"), "{err}");
    }

    fn uio_numbers(inv: &Inventory) -> (String, String) {
        (inv.pairs[0].sender.uio.clone(), inv.pairs[0].receiver.uio.clone())
    }

    #[test]
    fn inventory_file_rereads_when_its_mtime_changes() {
        let dir = ScratchDir::new("reload");
        let path = dir.0.join("ena-pairs.json");
        rewrite(&path, &serde_json::to_string(&uio_fixture()).unwrap(), 1_000);
        let mut file = InventoryFile::open(&path).unwrap();
        let first = file.current().unwrap();
        assert_eq!(uio_numbers(&first), ("uio0".into(), "uio1".into()));
        // Unchanged: the inventory already read, not a fresh parse.
        assert!(std::sync::Arc::ptr_eq(&first, &file.current().unwrap()));

        // The bootstrap rebound the pair and the uio numbers moved.
        let mut moved = uio_fixture();
        moved.pairs[0].sender.uio = "uio2".into();
        moved.pairs[0].receiver.uio = "uio3".into();
        rewrite(&path, &serde_json::to_string(&moved).unwrap(), 2_000);
        assert_eq!(uio_numbers(&file.current().unwrap()), ("uio2".into(), "uio3".into()));

        // The bootstrap's first step deletes it: no pair until it is back.
        std::fs::remove_file(&path).unwrap();
        assert!(file.current().is_none());
        rewrite(&path, &serde_json::to_string(&uio_fixture()).unwrap(), 3_000);
        assert_eq!(uio_numbers(&file.current().unwrap()), ("uio0".into(), "uio1".into()));

        // An unsafe or malformed rewrite advertises nothing, never the old pair.
        let mut unsafe_inv = uio_fixture();
        unsafe_inv.pairs[0].receiver.pci = unsafe_inv.pairs[0].sender.pci.clone();
        rewrite(&path, &serde_json::to_string(&unsafe_inv).unwrap(), 4_000);
        assert!(file.current().is_none());
        rewrite(&path, "{", 5_000);
        assert!(file.current().is_none());
        rewrite(&path, &serde_json::to_string(&moved).unwrap(), 6_000);
        assert_eq!(uio_numbers(&file.current().unwrap()), ("uio2".into(), "uio3".into()));
    }

    #[test]
    fn inventory_file_refuses_an_unsafe_inventory_at_start() {
        let dir = ScratchDir::new("start");
        let path = dir.0.join("ena-pairs.json");
        assert!(matches!(InventoryFile::open(&path), Err(InventoryError::Io(..))));
        let mut inv = uio_fixture();
        inv.pairs[0].sender.mac = inv.primary_ena_mac.clone();
        rewrite(&path, &serde_json::to_string(&inv).unwrap(), 1_000);
        assert!(matches!(InventoryFile::open(&path), Err(InventoryError::Validation(_))));
    }
}
