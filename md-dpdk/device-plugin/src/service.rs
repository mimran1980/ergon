//! Kubelet device-plugin v1beta1 service (plan §10.3). Advertises one device
//! per healthy ENA pair under `rusteron.io/dpdk-ena-pair`; `Allocate`
//! atomically injects the pair's sender/receiver DPDK env and each port's
//! device nodes (VFIO group or `/dev/uioN`) into the media driver pod. Both
//! read the inventory file as it is now: a bootstrap rerun that rewrites it is
//! followed without a restart.

use crate::inventory::{IGB_UIO, Inventory, InventoryFile, Pair, Port, VFIO_PCI};
use crate::{
    AllocateRequest, AllocateResponse, ContainerAllocateResponse, Device, DevicePluginOptions, DeviceSpec, Empty,
    ListAndWatchResponse, NumaNode, PreStartContainerRequest, PreStartContainerResponse, PreferredAllocationRequest,
    PreferredAllocationResponse, RegisterRequest, TopologyInfo, device_plugin_server, registration_server,
};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};

/// Kubelet accepts a Register call and the plugin then serves the device API.
/// We hold no state; a 200 is all the kubelet expects (plan §10.3).
#[derive(Clone, Default)]
pub struct RegistrationService;

#[tonic::async_trait]
impl registration_server::Registration for RegistrationService {
    async fn register(&self, _request: Request<RegisterRequest>) -> Result<Response<Empty>, Status> {
        Ok(Response::new(Empty {}))
    }
}

/// Host paths the live health probe inspects. Both are overridable so tests
/// can point at fixtures. `vfio_root` is the host's /dev/vfio, the only device
/// directory the plugin mounts; igb_uio liveness comes from sysfs alone.
#[derive(Clone, Debug)]
pub struct SysfsRoots {
    pub sys_root: PathBuf,
    pub vfio_root: PathBuf,
}

/// A pair is healthy only while both BDFs stay bound to their inventory
/// driver and their device nodes still exist (plan §10.3).
pub type HealthProbe = fn(&SysfsRoots, &Pair) -> bool;

pub fn sysfs_health(roots: &SysfsRoots, pair: &Pair) -> bool {
    [&pair.sender, &pair.receiver].iter().all(|p| port_health(roots, p))
}

/// The BDF's `driver` link must name exactly `port.driver`, and the port's
/// device node must be live. For vfio-pci, `/dev/vfio/<group>` must exist. For
/// igb_uio, the BDF's `uio/<uioN>/dev` attribute must exist: it is present only
/// while the uio class device is registered with a char device, which is when
/// devtmpfs provides `/dev/<uioN>`, and it pins that number to this BDF (UIO
/// numbers follow bind order, so a rebind can move them).
fn port_health(roots: &SysfsRoots, port: &Port) -> bool {
    let device = roots.sys_root.join("bus/pci/devices").join(&port.pci);
    let bound = device
        .join("driver")
        .read_link()
        .is_ok_and(|t| t.file_name().is_some_and(|name| *name == *port.driver));
    bound
        && match port.driver.as_str() {
            VFIO_PCI => roots.vfio_root.join(&port.iommu_group).exists(),
            IGB_UIO => device.join("uio").join(&port.uio).join("dev").exists(),
            _ => false,
        }
}

/// DevicePlugin service implementation.
#[derive(Clone)]
pub struct DevicePluginService {
    inventory: Arc<Mutex<InventoryFile>>,
    roots: SysfsRoots,
    health: HealthProbe,
    poll_interval: Duration,
}

impl DevicePluginService {
    pub fn new(inventory: InventoryFile, roots: SysfsRoots, health: HealthProbe, poll_interval: Duration) -> Self {
        Self {
            inventory: Arc::new(Mutex::new(inventory)),
            roots,
            health,
            poll_interval,
        }
    }

    /// The inventory as the file holds it now; `None` while it is missing or
    /// unsafe.
    fn inventory(&self) -> Option<Arc<Inventory>> {
        self.inventory.lock().unwrap_or_else(PoisonError::into_inner).current()
    }

    /// Current healthy devices: one per pair that passes the live probe.
    fn healthy_devices(&self) -> Vec<Device> {
        let Some(inventory) = self.inventory() else {
            return Vec::new();
        };
        inventory
            .pairs
            .iter()
            .filter(|p| (self.health)(&self.roots, p) && pair_numa_ok(p))
            .map(|p| Device {
                id: p.id.clone(),
                health: "Healthy".into(),
                topology: pair_numa_node(p).map(|id| TopologyInfo {
                    nodes: vec![NumaNode { id }],
                }),
            })
            .collect()
    }

    fn allocate_container(&self, devices_ids: &[String]) -> Result<ContainerAllocateResponse, Status> {
        let id = devices_ids
            .first()
            .ok_or_else(|| Status::invalid_argument("no device requested"))?;
        let inventory = self
            .inventory()
            .ok_or_else(|| Status::unavailable("no valid ENA pair inventory"))?;
        let pair = inventory
            .pairs
            .iter()
            .find(|p| &p.id == id)
            .ok_or_else(|| Status::not_found(format!("unknown device {id}")))?;
        if !(self.health)(&self.roots, pair) {
            return Err(Status::failed_precondition(format!("device {id} is not healthy")));
        }
        Ok(allocation_for(pair))
    }
}

#[tonic::async_trait]
impl device_plugin_server::DevicePlugin for DevicePluginService {
    type ListAndWatchStream = Pin<Box<dyn tokio_stream::Stream<Item = Result<ListAndWatchResponse, Status>> + Send>>;

    async fn get_device_plugin_options(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<DevicePluginOptions>, Status> {
        // No PreStartContainer, no preferred-allocation hinting.
        Ok(Response::new(DevicePluginOptions {
            pre_start_required: false,
            get_preferred_allocation_available: false,
        }))
    }

    async fn list_and_watch(&self, _request: Request<Empty>) -> Result<Response<Self::ListAndWatchStream>, Status> {
        let (tx, rx) = mpsc::channel::<Result<ListAndWatchResponse, Status>>(8);
        let this = self.clone();
        tokio::spawn(async move {
            let mut last: Option<Vec<Device>> = None;
            loop {
                let devices = this.healthy_devices();
                if last.as_ref() != Some(&devices) {
                    let next = ListAndWatchResponse {
                        devices: devices.clone(),
                    };
                    if tx.send(Ok(next)).await.is_err() {
                        break; // kubelet disconnected
                    }
                    last = Some(devices);
                }
                tokio::time::sleep(this.poll_interval).await;
            }
        });
        Ok(Response::new(Box::pin(ReceiverStream::new(rx))))
    }

    async fn get_preferred_allocation(
        &self,
        _request: Request<PreferredAllocationRequest>,
    ) -> Result<Response<PreferredAllocationResponse>, Status> {
        // Not advertised (get_preferred_allocation_available = false); the
        // kubelet falls back to round-robin. Empty per the v1beta1 contract.
        Ok(Response::new(PreferredAllocationResponse {
            container_responses: vec![],
        }))
    }

    async fn allocate(&self, request: Request<AllocateRequest>) -> Result<Response<AllocateResponse>, Status> {
        let mut container_responses = Vec::new();
        for cr in &request.into_inner().container_requests {
            container_responses.push(self.allocate_container(&cr.devices_ids)?);
        }
        Ok(Response::new(AllocateResponse { container_responses }))
    }

    async fn pre_start_container(
        &self,
        _request: Request<PreStartContainerRequest>,
    ) -> Result<Response<PreStartContainerResponse>, Status> {
        Ok(Response::new(PreStartContainerResponse {}))
    }
}

/// Both ports on one known node, or one known and the other unknown (`-1`).
/// Two different nodes are not one node: the topology manager would otherwise
/// place the pod from the sender's node alone.
fn pair_numa_ok(pair: &Pair) -> bool {
    let sender = pair.sender.numa_node;
    let receiver = pair.receiver.numa_node;
    sender < 0 || receiver < 0 || sender == receiver
}

/// The NUMA node to advertise for `pair`.
///
/// The known node when only one port reports one, that node when both report
/// the same, and none when both are unknown or they disagree.
fn pair_numa_node(pair: &Pair) -> Option<i64> {
    if !pair_numa_ok(pair) {
        return None;
    }
    let node = if pair.sender.numa_node >= 0 {
        pair.sender.numa_node
    } else {
        pair.receiver.numa_node
    };
    (node >= 0).then_some(i64::from(node))
}

/// Build the Allocate response for one pair: both roles' DPDK env plus the
/// device nodes. vfio-pci ports get the VFIO control device and each distinct
/// IOMMU group device (plan §10.3); igb_uio ports get their `/dev/uioN`.
fn allocation_for(pair: &Pair) -> ContainerAllocateResponse {
    let mut envs = HashMap::new();
    envs.insert("RUSTERON_MEDIA_DRIVER_TRANSPORT".to_string(), "dpdk-ena".to_string());
    envs.insert(
        "RUSTERON_DPDK_FILE_PREFIX".to_string(),
        format!("rusteron-dpdk-{}", pair.id),
    );
    put_role(&mut envs, "SENDER", &pair.sender);
    put_role(&mut envs, "RECEIVER", &pair.receiver);

    let mut devices = Vec::new();
    let mut groups = HashSet::new();
    for port in [&pair.sender, &pair.receiver] {
        match port.driver.as_str() {
            VFIO_PCI => {
                if groups.is_empty() {
                    devices.push(device_spec("/dev/vfio/vfio"));
                }
                if groups.insert(port.iommu_group.as_str()) {
                    devices.push(device_spec(&format!("/dev/vfio/{}", port.iommu_group)));
                }
            }
            IGB_UIO => devices.push(device_spec(&format!("/dev/{}", port.uio))),
            // Validation rejects any other driver before the plugin serves.
            _ => {}
        }
    }

    ContainerAllocateResponse {
        envs,
        mounts: vec![],
        devices,
        annotations: HashMap::new(),
        cdi_devices: vec![],
    }
}

// ponytail: role builds RUSTERON_DPDK_{SENDER|RECEIVER}_{PCI|IPV4_CIDR|GATEWAY}
// and must match the env.rs required() names; pinned by the
// allocation_injects_pair_env_and_vfio_nodes test.
fn put_role(envs: &mut HashMap<String, String>, role: &str, port: &Port) {
    let p = format!("RUSTERON_DPDK_{role}");
    envs.insert(format!("{p}_PCI"), port.pci.clone());
    envs.insert(format!("{p}_IPV4_CIDR"), cidr(port));
    envs.insert(format!("{p}_GATEWAY"), port.gateway.clone());
}

/// `address/prefix`, the format the DPDK env contract parses
/// (md-dpdk/src/env.rs `parse_cidr`).
fn cidr(port: &Port) -> String {
    format!("{}/{}", port.ipv4, port.prefix_len)
}

fn device_spec(path: &str) -> DeviceSpec {
    DeviceSpec {
        container_path: path.into(),
        host_path: path.into(),
        permissions: "rwm".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::fixtures::{ScratchDir, fixture, rewrite, uio_fixture};
    use std::path::Path;

    fn pair() -> Pair {
        fixture().pairs.remove(0)
    }

    fn service(health: HealthProbe) -> DevicePluginService {
        DevicePluginService::new(
            InventoryFile::fixed(fixture()),
            SysfsRoots {
                sys_root: PathBuf::from("/sys"),
                vfio_root: PathBuf::from("/dev/vfio"),
            },
            health,
            Duration::from_secs(5),
        )
    }

    fn always_healthy(_: &SysfsRoots, _: &Pair) -> bool {
        true
    }

    fn never_healthy(_: &SysfsRoots, _: &Pair) -> bool {
        false
    }

    #[test]
    fn allocation_injects_pair_env_and_vfio_nodes() {
        let resp = allocation_for(&pair());
        assert_eq!(resp.envs["RUSTERON_MEDIA_DRIVER_TRANSPORT"], "dpdk-ena");
        assert_eq!(resp.envs["RUSTERON_DPDK_SENDER_PCI"], "0000:00:06.0");
        assert_eq!(resp.envs["RUSTERON_DPDK_SENDER_IPV4_CIDR"], "10.0.0.1/24");
        assert_eq!(resp.envs["RUSTERON_DPDK_SENDER_GATEWAY"], "10.0.0.1");
        assert_eq!(resp.envs["RUSTERON_DPDK_RECEIVER_PCI"], "0000:00:07.0");
        assert_eq!(resp.envs["RUSTERON_DPDK_RECEIVER_IPV4_CIDR"], "10.0.0.1/24");
        let paths: Vec<&str> = resp.devices.iter().map(|d| d.host_path.as_str()).collect();
        assert!(paths.contains(&"/dev/vfio/vfio"));
        assert!(paths.contains(&"/dev/vfio/9"));
        assert!(paths.contains(&"/dev/vfio/10"));
        assert!(resp.devices.iter().all(|d| d.permissions == "rwm"));
    }

    #[test]
    fn igb_uio_allocation_injects_uio_nodes_only() {
        let resp = allocation_for(&uio_fixture().pairs.remove(0));
        assert_eq!(resp.envs["RUSTERON_DPDK_SENDER_PCI"], "0000:00:06.0");
        assert_eq!(resp.envs["RUSTERON_DPDK_RECEIVER_PCI"], "0000:00:07.0");
        let mut paths: Vec<(&str, &str)> = resp
            .devices
            .iter()
            .map(|d| (d.host_path.as_str(), d.container_path.as_str()))
            .collect();
        paths.sort_unstable();
        assert_eq!(paths, [("/dev/uio0", "/dev/uio0"), ("/dev/uio1", "/dev/uio1")]);
        assert!(resp.devices.iter().all(|d| d.permissions == "rwm"));
    }

    #[test]
    fn shared_iommu_group_is_deduplicated() {
        let mut p = pair();
        p.receiver.iommu_group = p.sender.iommu_group.clone();
        let resp = allocation_for(&p);
        // /dev/vfio/vfio + one group device.
        assert_eq!(resp.devices.len(), 2);
    }

    #[test]
    fn only_healthy_pairs_are_advertised() {
        assert_eq!(service(always_healthy).healthy_devices().len(), 1);
        assert!(service(never_healthy).healthy_devices().is_empty());
        let dev = service(always_healthy).healthy_devices().remove(0);
        assert_eq!(dev.id, "dpdk-pair-0");
        assert_eq!(dev.health, "Healthy");
        assert_eq!(dev.topology.unwrap().nodes[0].id, 0);
    }

    fn devices_of(inventory: Inventory) -> Vec<Device> {
        DevicePluginService::new(
            InventoryFile::fixed(inventory),
            SysfsRoots {
                sys_root: PathBuf::from("/sys"),
                vfio_root: PathBuf::from("/dev/vfio"),
            },
            always_healthy,
            Duration::from_secs(5),
        )
        .healthy_devices()
    }

    #[test]
    fn topology_uses_the_one_reported_numa_node() {
        let mut inventory = fixture();
        inventory.pairs[0].sender.numa_node = -1;
        inventory.pairs[0].receiver.numa_node = 1;
        let dev = devices_of(inventory).remove(0);
        assert_eq!(dev.topology.unwrap().nodes[0].id, 1);

        let mut same = fixture();
        same.pairs[0].sender.numa_node = 1;
        same.pairs[0].receiver.numa_node = 1;
        assert_eq!(devices_of(same).remove(0).topology.unwrap().nodes[0].id, 1);
    }

    #[test]
    fn a_split_numa_pair_is_not_advertised() {
        let mut inventory = fixture();
        inventory.pairs[0].receiver.numa_node = 1;
        assert!(devices_of(inventory).is_empty());
    }

    fn uio_nodes(resp: &ContainerAllocateResponse) -> Vec<&str> {
        let mut paths: Vec<&str> = resp.devices.iter().map(|d| d.host_path.as_str()).collect();
        paths.sort_unstable();
        paths
    }

    #[test]
    fn advertising_and_allocation_follow_a_rewritten_inventory() {
        let dir = ScratchDir::new("service");
        let path = dir.0.join("ena-pairs.json");
        rewrite(&path, &serde_json::to_string(&uio_fixture()).unwrap(), 1_000);
        let svc = DevicePluginService::new(
            InventoryFile::open(&path).unwrap(),
            SysfsRoots {
                sys_root: PathBuf::from("/sys"),
                vfio_root: PathBuf::from("/dev/vfio"),
            },
            always_healthy,
            Duration::from_secs(5),
        );
        let pair = ["dpdk-pair-0".to_string()];
        assert_eq!(
            uio_nodes(&svc.allocate_container(&pair).unwrap()),
            ["/dev/uio0", "/dev/uio1"]
        );

        // Rebound by a bootstrap rerun: the uio numbers moved.
        let mut moved = uio_fixture();
        moved.pairs[0].sender.uio = "uio2".into();
        moved.pairs[0].receiver.uio = "uio3".into();
        rewrite(&path, &serde_json::to_string(&moved).unwrap(), 2_000);
        assert_eq!(
            uio_nodes(&svc.allocate_container(&pair).unwrap()),
            ["/dev/uio2", "/dev/uio3"]
        );

        // Deleted (a bootstrap run in progress, or one that failed): nothing
        // advertised, nothing allocated.
        std::fs::remove_file(&path).unwrap();
        assert!(svc.healthy_devices().is_empty());
        assert!(svc.allocate_container(&pair).is_err());
        rewrite(&path, &serde_json::to_string(&moved).unwrap(), 3_000);
        assert_eq!(svc.healthy_devices().len(), 1);
    }

    #[test]
    fn allocate_rejects_unknown_and_unhealthy() {
        let svc = service(always_healthy);
        assert!(svc.allocate_container(&["nope".into()]).is_err());
        let bad = service(never_healthy);
        assert!(bad.allocate_container(&["dpdk-pair-0".into()]).is_err());
        let ok = svc.allocate_container(&["dpdk-pair-0".into()]).unwrap();
        assert!(!ok.envs.is_empty());
    }

    /// A throwaway host tree under the system temp dir: `sys/` and `dev/vfio/`,
    /// removed on drop.
    struct HostTree(PathBuf);

    impl HostTree {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!("rusteron-dpdk-plugin-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("dev/vfio")).unwrap();
            Self(root)
        }

        fn roots(&self) -> SysfsRoots {
            SysfsRoots {
                sys_root: self.0.join("sys"),
                vfio_root: self.0.join("dev/vfio"),
            }
        }

        fn device(&self, bdf: &str) -> PathBuf {
            self.0.join("sys/bus/pci/devices").join(bdf)
        }

        /// Bind `bdf` to `driver` the way sysfs shows it: a relative link.
        fn bind(&self, bdf: &str, driver: &str) {
            let dev = self.device(bdf);
            std::fs::create_dir_all(&dev).unwrap();
            let link = dev.join("driver");
            let _ = std::fs::remove_file(&link);
            std::os::unix::fs::symlink(format!("../../../bus/pci/drivers/{driver}"), link).unwrap();
        }

        /// Register `uio` under `bdf` the way sysfs shows a uio class device:
        /// `uio/<uioN>/dev` holds its char major:minor. There is deliberately
        /// no `dev/` node: liveness comes from sysfs alone.
        fn uio(&self, bdf: &str, uio: &str) {
            let dir = self.device(bdf).join("uio").join(uio);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("dev"), b"240:0\n").unwrap();
        }

        fn path(&self, rel: &str) -> PathBuf {
            self.0.join(rel)
        }
    }

    impl Drop for HostTree {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn remove(path: &Path) {
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn sysfs_health_igb_uio() {
        let host = HostTree::new("uio");
        let pair = uio_fixture().pairs.remove(0);
        let roots = host.roots();
        // Nothing bound yet.
        assert!(!sysfs_health(&roots, &pair));
        host.bind("0000:00:06.0", "igb_uio");
        host.uio("0000:00:06.0", "uio0");
        host.bind("0000:00:07.0", "igb_uio");
        host.uio("0000:00:07.0", "uio1");
        assert!(sysfs_health(&roots, &pair));

        // Rebound to the kernel driver: unhealthy.
        host.bind("0000:00:07.0", "ena");
        assert!(!sysfs_health(&roots, &pair));
        // A driver whose name merely contains the expected one is not it.
        host.bind("0000:00:07.0", "igb_uio_old");
        assert!(!sysfs_health(&roots, &pair));
        host.bind("0000:00:07.0", "igb_uio");
        assert!(sysfs_health(&roots, &pair));

        // The uio class device is still listed but no longer carries a char
        // device (no `dev` attribute, so devtmpfs has no /dev/uio1).
        let dev_attr = host.device("0000:00:07.0").join("uio/uio1/dev");
        remove(&dev_attr);
        assert!(!sysfs_health(&roots, &pair));
        std::fs::write(&dev_attr, b"240:1\n").unwrap();
        assert!(sysfs_health(&roots, &pair));

        // The uio number moved to another BDF (rebind order changed): the
        // inventory's /dev/uio1 is no longer this device.
        std::fs::remove_dir_all(host.device("0000:00:07.0").join("uio/uio1")).unwrap();
        std::fs::create_dir_all(host.device("0000:00:07.0").join("uio/uio2")).unwrap();
        assert!(!sysfs_health(&roots, &pair));
    }

    #[test]
    fn sysfs_health_vfio_pci() {
        let host = HostTree::new("vfio");
        let pair = pair();
        let roots = host.roots();
        host.bind("0000:00:06.0", "vfio-pci");
        host.bind("0000:00:07.0", "vfio-pci");
        std::fs::write(host.path("dev/vfio/9"), b"").unwrap();
        std::fs::write(host.path("dev/vfio/10"), b"").unwrap();
        assert!(sysfs_health(&roots, &pair));

        remove(&host.path("dev/vfio/10"));
        assert!(!sysfs_health(&roots, &pair));
        std::fs::write(host.path("dev/vfio/10"), b"").unwrap();

        host.bind("0000:00:06.0", "igb_uio");
        assert!(!sysfs_health(&roots, &pair));
    }
}
