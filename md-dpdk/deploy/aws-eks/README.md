# AWS EKS DPDK ENA deployment

Deployment assets for running the `ergon-md-dpdk` DPDK ENA kernel-bypass
transport on Amazon EKS (Amazon Linux 2023 on Nitro: Graviton4/5 arm64 or
Intel x86_64). One Media Driver pod per dedicated node owns two secondary ENAs
(sender and receiver). The node's primary ENA stays kernel-owned for EKS, DNS
and telemetry, and is the kernel-UDP rollback path.

```
md-dpdk/deploy/aws-eks/
├── kustomization.yaml            # kustomize base: both DaemonSets + the ConfigMap
├── media-driver-daemonset.yaml   # one privileged driver pod per DPDK node
├── media-driver-config.yaml      # ConfigMap: DPDK rings, Aeron dir on hugetlbfs, ports
├── device-plugin-daemonset.yaml  # advertises rusteron.io/dpdk-ena-pair
├── Dockerfile.media-driver       # AL2023 image: media_driver (static,dpdk), rtt, counters
├── Dockerfile.device-plugin      # AL2023 image: rusteron-dpdk-device-plugin
├── entrypoint.sh                 # container entrypoint: cpuset -> CPU roles, Aeron/EAL env
└── node/
    ├── userdata.sh               # prints the node's gzipped MIME user data (scripts + NodeConfig)
    ├── provision-node.sh         # first boot: kernel args, igb_uio build, unit install, reboot
    ├── bootstrap-dpdk-ena.sh     # every boot: bind the pair, write the inventory, tune the host
    ├── preflight.sh              # driver/IOMMU/hugepages/NUMA/ENA identity checks
    ├── irq-affinity.sh           # every movable IRQ on the housekeeping CPUs: one pass, or a watcher
    ├── rusteron-dpdk-ena.service # systemd oneshot: run the bootstrap before kubelet
    ├── rusteron-irq-affinity.service # systemd: irq-affinity.sh watch, re-pins on each link event
    └── test/
        └── test-bootstrap.sh     # fixture tests for the node scripts and the entrypoint
```

## 1. Images

Build both from the ergon repo root (`.dockerignore` keeps `target/`, `.git`
and the upstream submodules out of the context):

```bash
docker buildx build --platform linux/arm64 --load -f md-dpdk/deploy/aws-eks/Dockerfile.media-driver -t rusteron-media-driver:dpdk .
docker buildx build --platform linux/arm64 --load -f md-dpdk/deploy/aws-eks/Dockerfile.device-plugin -t rusteron-dpdk-device-plugin:latest .
docker run --rm rusteron-media-driver:dpdk ldd /usr/local/bin/media_driver
docker run --rm --entrypoint /usr/local/bin/media_driver rusteron-media-driver:dpdk --version
```

The media driver image compiles DPDK 24.11.4 from a pinned, checksummed
tarball (ENA PMD only; the version the VM lab runs) for DPDK's generic
platform (`-Dplatform=generic`: `armv8-a` on arm64, `corei7` on x86_64, as
Debian's package the lab ran is built), never for the CPU that happens to
build the image. It builds
`media_driver` with `--features static,dpdk`, and adds the benchmark clients
`rtt` and `counters` (`ergon-md-dpdk-harness --features static`), which run
inside the driver's pod against its Aeron directory. It also carries
`taskset`, `nsenter` and `pkill`. It records the DPDK/ENA/kernel/Aeron
versions it was built against, and the instruction set DPDK was built for, in
`/etc/rusteron-dpdk-versions.txt`.

Both images build for the platform buildx is given: `linux/arm64` for
Graviton nodes, `linux/amd64` for Intel x86 (the `dpdk` feature rejects any
other target). On an Arm Mac arm64 builds natively and amd64 under emulation,
which is slow. Pick the image with the node's architecture
(`kubernetes.io/arch`).

| Build argument | Default | Effect |
|---|---|---|
| `RUST_TARGET_CPU` | empty: generic | `-Ctarget-cpu` for the Rust; `build.rs` gives md-dpdk's C the same `-mcpu` (arm64) or `-march` (x86_64), which `cc` does not pass on. The image then runs only on that CPU family (SIGILL elsewhere): select its nodes by instance family too. The C compiler must know the name, or the build stops. |
| `CARGO_BUILD_JOBS` | empty: cargo's | caps the build's parallel jobs on a small Docker VM |

`media_driver --version` prints the architecture, Rust's target CPU and the
native C's CPU flag.

## 2. Nodes

The node path has two stages, both driven by the user data that
`node/userdata.sh` prints for a self-managed AL2023 EKS node:

```bash
node/userdata.sh <cluster> <apiServerEndpoint> <ca-b64> <service-cidr> <reserved-cpus> > userdata.gz
```

The output is a gzip-compressed MIME multipart document (pass it as
`--user-data fileb://userdata.gz`; cloud-init and nodeadm both read gzip user
data). The first part is a shell script that writes `/etc/rusteron-dpdk/node.env`
and the six node files, each from a quoted here-document, to
`/opt/rusteron-dpdk/node`, then runs `provision-node.sh`. Each file goes in as
its header (the leading comment block) and its code; the comment lines in
between stay in the repo, which is how the scripts fit. The second is the
nodeadm `NodeConfig`: the cluster fields (nothing is merged for a stock AMI
launched outside a managed node group), the node label
`rusteron-dpdk/ena=true`, the taint `rusteron-dpdk=true:NoSchedule`, and the
kubelet latency settings: CPU Manager `static` with `full-pcpus-only`,
`reservedSystemCPUs` (the housekeeping core, one CPU: e.g. `0`) with the
reserved-cgroup settings cleared so kubelet accepts it, and Topology Manager
`single-numa-node` at pod scope. `cpuCFSQuota` is not set, so it stays
`true` and every pod's CPU limit holds. The driver still gets no CPU quota:
kubelet 1.33 and later (use 1.36 or later) sets none on a container with
exclusive CPUs (static policy, Guaranteed pod, whole-number CPU request), nor
on its pod. Do not set `cpuCFSQuota: false`: it removes the CPU limit of
every pod on the node
([checklist](https://mimran1980.github.io/ergon/deploy/kubernetes.html#latency-checklist)).
The Memory Manager stays at `None`: every
c7i size below `metal-48xl` has one NUMA node, so `Static` would pin nothing,
and kubelet refuses to start under `Static` unless `reservedMemory` exactly
equals kubeReserved, systemReserved and the hard eviction threshold together.
The driver refuses to start when the two ENAs report different NUMA nodes,
and the device plugin does not advertise that pair. Queues and mbuf pools
use the node the PMD reports, not a hardcoded node 0.
`userdata.sh` fails if its gzipped output exceeds EC2's 16384-byte user-data
limit.

### SMT off (one CPU is one physical core)

Hyper-threading (SMT) is off on every x86 latency node, so one Kubernetes CPU
is one physical core, the same as on Graviton. A pod's CPU request is a whole
number of cores with no twin sharing its execution resources.

- **Where EC2 takes CPU options** (every VM type that has SMT, e.g. `c7i`):
  `node/launch-template.py` puts `CpuOptions` `ThreadsPerCore=1` and the
  `CoreCount` EC2 requires alongside it in the node group's launch template,
  so the node boots with half the type's vCPUs and no SMT. There is no extra
  charge. EC2 takes no CPU options on bare metal. It will not change the
  number of threads per core on T2, C7a, M7a, R7a or Graviton. C7a, M7a, R7a
  and Graviton already run one thread per core. Those virtual-machine types
  remain in the supported CPU options table, with a core count and threads
  per core fixed at 1.
- **Bare metal** (no CPU options): `provision-node.sh` turns SMT off on the
  first boot — the sysfs `smt/control` write is the last resort before kubelet
  starts, and `nosmt` on the kernel command line holds every later boot. It
  then computes the CPU lists from one CPU per physical core: the kernel keeps
  listing the offline twins in the topology, so a list that names a twin fails
  the node (`housekeeping CPU N will not run with SMT off`).
- The live check (`benches/cloud/aws/eks.sh`) fails a node whose
  `/sys/devices/system/cpu/smt/active` is 1.

Kubelet's `full-pcpus-only` stays on as a safety net: with SMT off every CPU
is already a whole core, so it only matters if a node somehow runs with SMT.

### First boot (`provision-node.sh`)

Runs from user data, before kubelet has ever started (nodeadm starts kubelet
after cloud-init). It does nothing on a node whose kernel command line already
has `isolcpus=`, so it can never loop. Otherwise it:

1. turns SMT off on a node that still has it (bare metal: EC2's CPU options
   cover a VM only), then checks the node: Intel x86 or Graviton4/5, housekeeping CPUs
   (`RUSTERON_HOUSEKEEPING_CPUS`, default cpu0's core, one CPU per core) that
   run with SMT off, and at least four physical cores outside them (the
   driver's housekeeping, sender and receiver, and an application);
2. waits (up to `RUSTERON_ENI_WAIT_SECS`, default 600) for the secondary ENAs
   (IMDS `device-number` 1 and 2 by default, `RUSTERON_DPDK_DEVICE_NUMBERS`)
   to be attached;
3. installs the build tools and kernel headers with `dnf`, clones
   `dpdk-kmods` at a pinned commit (both tried five times, 5, 10, 20 and 40 s
   apart), builds `igb_uio` against the running kernel and installs it to
   `/opt/rusteron-dpdk/`;
4. installs `bootstrap-dpdk-ena.sh`, `preflight.sh`, `irq-affinity.sh` and
   the two systemd units (the per-boot bootstrap and the IRQ watcher), and
   enables both;
5. sets the kernel command line with `grubby --update-kernel=ALL --args=...`
   and checks the default entry took it: `nosmt` (SMT stays off on every later
   boot), `isolcpus`
   (`nohz,domain,managed_irq`), `nohz_full` and `rcu_nocbs` on every CPU but
   the housekeeping core, `irqaffinity` on it, `iommu=pt`, `transparent_hugepage=never`, and
   boot-time 2 MiB hugepages (so kubelet's first start advertises
   `hugepages-2Mi`). The clocksource is left to the kernel. Bare metal may
   add `clocksource=tsc tsc=reliable` with `RUSTERON_FORCE_TSC=1`, which is
   the operator saying this machine's TSC has been measured as the same
   across cores against a PTP/PHC reference: `tsc=reliable` turns the
   kernel's TSC checks off. A VM never gets it. The choice is written to
   `/opt/rusteron-dpdk/clocksource-policy`;
6. clears kubelet's CPU and memory manager state and reboots once.

If any step fails, it writes `/var/lib/rusteron-dpdk/provision-failed` and a
kubelet drop-in (`kubelet.service.d/90-rusteron-provision-failed.conf`) that
asserts that file is absent, so kubelet never starts and the node never
registers with the DPDK label and taint. Its output is in
`/var/log/cloud-init-output.log`; replace the node.

### Every boot (`bootstrap-dpdk-ena.sh`)

The systemd unit (`Type=oneshot`, `RemainAfterExit=yes`, ordered before
kubelet, `Restart=on-failure` every 15 s, about 20 attempts) runs it at every
boot, so a reboot restores the bindings and regenerates the inventory before
kubelet schedules pods. It first deletes the old inventory, so a failed run
leaves none. Then, in order:

1. **Driver.** Loads `uio` and `igb_uio` with `wc_activate=1`
   (`RUSTERON_DPDK_DRIVER`, default `igb_uio`), or `vfio-pci`.
2. **Preflight and discovery** (`preflight.sh`). For igb_uio: fails if the
   module is not loaded, or if an IOMMU translates DMA (igb_uio programs
   none): IOMMU groups are allowed only when every one is in passthrough
   (`identity`, from `iommu=pt`). For vfio-pci: needs IOMMU groups, `vfio_iommu_type1`, and
   `RUSTERON_DPDK_VFIO_WC=1` (a write-combining vfio-pci; mainline maps none,
   and ENA's low-latency queues need it). The driver pod's transport takes
   that confirmation from the ConfigMap's `RUSTERON_DPDK_VFIO_WC=1`: vfio-pci
   maps the LLQ BAR through its device file, which `/proc/self/maps` cannot
   show write-combined, so its start check accepts a vfio-pci port on it,
   without a warning (see [the ConfigMap](#the-configmap)). Always: 2 MiB hugepages reserved at
   `/dev/hugepages`, NUMA topology present, at least three ENA PCI functions,
   and complete IMDS identity for the selected ENAs. The sender and receiver
   are the ENAs at the configured IMDS device numbers (first = sender); the
   primary ENA and any VPC CNI ENI are never selected, whatever their MAC
   order.
3. **Kernel command line.** Fails if it lacks the provisioned isolation flags
   (with `iommu=pt` wherever IOMMU groups exist),
   or if `irqaffinity` is not the housekeeping CPUs.
4. **Release and bind.** First takes both ENAs from the OS's network
   configuration, so that one handed back to the kernel (a rollback, a kernel
   baseline) is never re-addressed behind the operator's back, as cloud-init's
   hotplug did in the Debian lab. AL2023 has the
   same path through udev: on the EKS AMI, `90-eks.rules` starts
   `udev-net-manager@<iface>.service` for every ENA that appears, which writes
   `/run/systemd/network/70-eks-<iface>.network` (DHCP) and reloads networkd;
   stock AL2023's amazon-ec2-net-utils does the same through
   `policy-routes@<iface>.service`, and its `80-ec2.network` runs DHCP on every
   ENA. The bootstrap writes
   `/etc/systemd/network/00-rusteron-dpdk-data.network` (`Unmanaged=yes` for
   the two MACs; it sorts before both, so networkd leaves them alone, reloading
   networkd only when the file changes), and masks each ENA's
   `udev-net-manager@`, `policy-routes@` and `refresh-policy-routes@` units
   until the next boot. It also records each ENA's MAC and PCI BDF in
   `/var/lib/rusteron-dpdk/bound-enas`, then unbinds it from the kernel `ena`
   driver and binds it to the target driver through `driver_override`; a port
   already on the target driver is left alone. A bound ENA has no netdev, so
   its MAC no longer leads to its BDF: a rerun (the next boot's, or a restart
   after a run that failed between binding and the inventory) finds the BDF in
   `bound-enas`, and uses it only while that BDF is still on the target
   driver.
5. **Tune.** Every IRQ's affinity and the default affinity to the housekeeping
   CPUs (`irq-affinity.sh pin`, which fails on an IRQ that can move but
   that it cannot place there, and names without failing one the kernel
   refuses to move, such as a managed NVMe queue IRQ, and one the kernel
   moves only at its next interrupt), the IRQ watcher
   started (see [IRQ placement after boot](#irq-placement-after-boot)),
   irqbalance disabled, transparent hugepages `never`, KSM and NUMA
   balancing off, `net.core.rmem_max`/`wmem_max` 16 MiB, and Aeron's
   hugetlbfs mounted at `/mnt/huge-aeron` (`RUSTERON_AERON_HUGE_SIZE`, 1G),
   which fails if it is larger than the free 2 MiB pool. The kernel keeps its
   own `vm.min_free_kbytes` and the primary ENA its interrupt coalescing (no
   committed run measures a change to either), and busy polling stays off
   unless asked:
   `busy_read` reaches every blocking socket read on the node, kubelet's
   included, and was never measured on the primary ENA's path.
6. **Inventory.** Writes `/var/lib/rusteron-dpdk/ena-pairs.json` atomically,
   recording per role: PCI BDF, driver, uio device (igb_uio) or IOMMU group
   (vfio-pci), ENI id, MAC, IPv4, prefix length, subnet CIDR, gateway, NUMA
   node and health.

A bootstrap failure leaves the node usable for kernel-UDP workloads (visible
with `systemctl status rusteron-dpdk-ena`); the device plugin then advertises
no pair.

### IRQ placement after boot

The kernel's ENA driver sets each queue IRQ's affinity to its per-queue hint
(queue *i* on CPU *i*, so mostly isolated CPUs) whenever it brings an
interface up: after a device reset (its own recovery included), a queue or
ring change (`ethtool -L`, `-G`), an `ifdown`/`ifup`, and a data ENA handed
back to the kernel driver. The boot-time pinning cannot prevent that, so the
interrupts then land on the isolated CPUs.
`rusteron-irq-affinity.service` (`irq-affinity.sh watch`) keeps them off:
it follows the kernel's netlink link events (`ip -o monitor link`), and every
one of those paths ends in one (the interface's flags or carrier), so it
re-pins within milliseconds (a carrier change may reach it up to a second
later). It sleeps between events, runs on the housekeeping CPUs, and logs
each IRQ it moves with the CPUs it came from
(`journalctl -u rusteron-irq-affinity`). On an EC2 VM, which has no
interrupt remapping, the kernel carries out an MSI's move, the driver's hint
as much as the watcher's re-pin, only in that IRQ's next interrupt: a re-pin
that wins the race leaves the IRQ on the housekeeping CPUs, and one that
comes after the hint took effect costs the isolated CPU the interrupt that
moves it (two, if the hint's own move had not yet seen one there) and the
old vector's clean-up. irqbalance stays off: with the isolated CPUs banned
it still rewrites an IRQ's affinity only after moving that IRQ itself, so an
interface brought up again in place keeps its hints. `irq-affinity.sh` gives
the full reasoning.

`sudo /usr/local/sbin/irq-affinity.sh pin` re-pins and checks on demand:
it exits 1 naming any IRQ that can move but whose requested affinity it
cannot make the housekeeping CPUs, and warns about each one the kernel keeps
on an isolated CPU (a managed IRQ whose queue serves isolated CPUs only fires
only for I/O issued on them) and each one still there `until its next
interrupt`, whose move the kernel has taken but carries out only then (an
idle queue's IRQ can show there for minutes; it fires there once or twice
more).

Optional settings, read from `/etc/rusteron-dpdk/node.env` (the unit's
`EnvironmentFile`):

| Variable | Default | Effect |
|---|---|---|
| `RUSTERON_MIN_FREE_PCT` | unset: the kernel's | `vm.min_free_kbytes` at 1-5% of memory (2% caused NAK storms and a 12 s stall) |
| `RUSTERON_BUSY_POLL_US` | `0` (off) | `net.core.busy_poll` and `busy_read` in µs for the kernel sockets (the primary ENA, the rollback path); 50 is the lab's kernel busy-polling value |
| `RUSTERON_AERON_HUGE_SIZE` | `1G` | the size of Aeron's hugetlbfs; see [Hugepage budget](#hugepage-budget) |
| `RUSTERON_THP` | `never` | transparent hugepages at each boot (the kernel command line already says `never`) |

### Node requirements

- AL2023 Nitro instance with at least 5 physical cores, which
  `provision-node.sh` checks: one reserved for the system and kubelet, three
  for the driver pod (housekeeping, sender, receiver) and one for an
  application. SMT is off ([above](#smt-off-one-cpu-is-one-physical-core)),
  so one CPU is one core.

  | CPU | `provision-node.sh` accepts | Smallest size | Kernel command line differs by |
  |---|---|---|---|
  | Graviton4 (c8g, Neoverse V2) | `CPU part 0xd4f` | c8g.2xlarge (8 cores) | no `nosmt` (no SMT); `iommu.passthrough=1` in place of `iommu=pt` |
  | Graviton5 (c9g, Neoverse V3) | `CPU part 0xd84` | c9g.2xlarge (8 cores) | as Graviton4 |
  | Intel x86 (c7i) | `vendor_id GenuineIntel` | c7i.4xlarge (8 cores) | `nosmt`, `iommu=pt` |

  Any other CPU (Graviton2/3, AMD) fails closed, as does a Graviton kernel
  without 4 KiB pages and 2 MiB hugepages (`getconf PAGESIZE`: only 4 KiB
  pages are qualified, as in the VM lab). Graviton bare metal is not
  supported yet: `bootstrap-dpdk-ena.sh` still requires `iommu=pt` wherever
  there are IOMMU groups, and `eks.sh` refuses a Graviton `*.metal` type.
  These are node settings (user data), not pod settings.
- Three ENAs: the primary plus the sender and receiver, created with the tag
  `node.k8s.amazonaws.com/no_manage=true` so the VPC CNI never claims them.
- The ENAs bind to igb_uio with `wc_activate=1`, the only write-combining
  binding on virtualised EC2, and on `*.metal` too (see
  [Bare metal](#bare-metal)). A write-combining vfio-pci
  (`RUSTERON_DPDK_DRIVER=vfio-pci RUSTERON_DPDK_VFIO_WC=1` in the node's
  environment) is the alternative on metal: the bootstrap and preflight
  support it, the device plugin hands out its VFIO devices, and the
  ConfigMap's `RUSTERON_DPDK_VFIO_WC=1` lets the transport's start check
  accept its LLQ BAR mapping; the DaemonSet stays privileged as for igb_uio.
- Both nodes of a cell in the same cluster placement group.
- `chronyd` active: DPDK timestamps here are software clock readings and need
  a disciplined host clock.

### Bare metal

No run compared metal with a VM
([BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#open)).
Round 4 measured `c7i.metal-24xl` alone: DPDK's idle p99.99 38.72 µs, max
71 (48–845) µs, the median of the reps' maxima and their range. The same node
path serves it, with two differences:

- Metal has a real IOMMU, which igb_uio does not program. `iommu=pt` (set by
  `provision-node.sh` on every node; a VM has no IOMMU, so it changes nothing
  there) puts every group in passthrough, and the preflight checks that each
  one is.
- No C-state cap: on metal, C1 did not separate at idle, and at 288 B 1 M
  msgs/s at least one rep of each capped arm had a p50 above 4 ms
  ([data](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#measured-but-no-setting-recommended)),
  and the one committed VM A/B was mixed
  ([C-states](https://mimran1980.github.io/ergon/deploy/hft.html#c-states)),
  so `provision-node.sh` sets it on no node.
- SMT off at boot, not by EC2: bare metal takes no CPU options, so
  `provision-node.sh` writes sysfs's `smt/control` before kubelet's first
  start and `nosmt` holds every later boot
  ([above](#smt-off-one-cpu-is-one-physical-core)). A metal `c7i` still
  occupies the type's full vCPU count for the EC2 quota: `nosmt` takes half
  the threads offline, it does not give them back to the pool.

## 3. Media driver pod

`media-driver-daemonset.yaml` runs one pod per `rusteron-dpdk/ena=true` node,
requesting one `rusteron.io/dpdk-ena-pair`, 6 CPUs (six physical cores: SMT is
off), 2 GiB of memory and 2 GiB of 2 MiB hugepages, with requests equal to
limits (Guaranteed QoS, so the CPU Manager gives it exclusive cores).

The container is **privileged**. With igb_uio the DPDK EAL runs in
IOVA-as-PA mode: it reads `/proc/self/pagemap` (`CAP_SYS_ADMIN`,
`CAP_DAC_READ_SEARCH`), locks hugepages (`CAP_IPC_LOCK`), and maps the ENA's
BARs through their sysfs resource files, which needs the read-write `/sys`
only a privileged container gets.

kubelet stops the pod with SIGTERM. The driver stops on it as on SIGINT
(ctrlc's `termination` feature), closing the driver and then the DPDK
transport; without a handler, the container's PID 1 would ignore SIGTERM until
the SIGKILL that ends the grace period (`terminationGracePeriodSeconds: 15`).

### The ConfigMap

`media-driver-config.yaml` sets what is the same on every node. It leaves
`RUSTERON_DPDK_RX_DESCRIPTORS` unset, so each port takes min(4096, the NIC's
maximum), 4096 on an ENA: room for a burst; against 1024 it did not separate
at idle (`BENCHMARKS.md`, "Measured, but no setting recommended"). The pool
holds that ring.

| Setting | Value | Why |
|---|---|---|
| `AERON_RECEIVER_IO_VECTOR_CAPACITY` | `16` | Aeron's maximum (its default is 4) and `BENCHMARKS.md`'s recommended configuration: with DPDK at 1,344 B 600 k msgs/s no NAK against 16,247 at the default (one run). The DPDK receive poll drains whole bursts whatever this is; it still sizes each receive from a host-local socket. The media driver also defaults to 16 under `dpdk-ena` when it is unset, never under the socket transport; a rollback sets it back to 4 (the runbook's §4) |
| `RUSTERON_DPDK_TX_DESCRIPTORS` | `256` | An ENA queue takes at most 256 transmit descriptors under the transport's large LLQ entries (`llq_policy=3`, the default; 512 under `llq_policy=1`); a larger ring fails the port's start |
| `RUSTERON_DPDK_MBUFS_PER_PORT` | `16384` | The count every lab measurement used (`benches/cloud/vm.sh`): about 37 MiB of hugepages a port; see [Hugepage budget](#hugepage-budget). It holds the 4096-descriptor receive ring, the transmit ring, four bursts of 32 and two mempool caches (4096 + 256 + 128 + 768 = 5,248) with room to spare |
| `RUSTERON_DPDK_VFIO_WC` | `1` | A vfio-pci node's write-combining, as its preflight confirmed: the bootstrap binds the ENAs to vfio-pci only after `preflight.sh` passed with the operator's `RUSTERON_DPDK_VFIO_WC=1`, and a failed run leaves no inventory, so the device plugin hands out no pair. vfio-pci maps the LLQ BAR through its device file, so `/proc/self/maps` cannot show it write-combined; with this the transport's start check accepts a port bound to vfio-pci, with no warning. Inert for igb_uio ports, which must still show `resource2_wc` |
| `AERON_DIR`, `RUSTERON_MEDIA_DRIVER_DIR` | `/mnt/huge-aeron/aeron` | Aeron's directory on a hugetlbfs of its own that the node bootstrap mounts with a size (an unsized hugetlbfs reports no free space, and Aeron's storage check then refuses every log buffer) |
| `AERON_FILE_PAGE_SIZE`, `AERON_TERM_BUFFER_SPARSE_FILE`, `AERON_DIR_DELETE_ON_START` | `2097152`, `false`, `true` | Log buffers in whole 2 MiB pages, filled when created, as in the VM lab |
| `AERON_RCV_INITIAL_WINDOW_LENGTH`, `AERON_SOCKET_SO_RCVBUF`, `AERON_SOCKET_SO_SNDBUF` | `2097152` | The VM lab's window; the driver refuses a window larger than its receive buffer, and the bootstrap raises `rmem_max`/`wmem_max` to allow it |
| `RUSTERON_DPDK_EPHEMERAL_PORT_RANGE` | `61000 63999` | The transport's own fallback for a bind Aeron leaves at port 0. With the entrypoint's wildcard ranges, Aeron gives publications without a fixed port a source port in 20000-20999 and receiver-side wildcard binds one in 21000-21999, so peers need UDP 20000-21999 open between them |

The image names in the manifests are bare; an overlay rewrites them to a
registry with kustomize `images:`.

### Hugepage budget

The node reserves `RUSTERON_HUGEPAGES` 2 MiB pages at boot (2048: 4 GiB), and
kubelet advertises all of them as `hugepages-2Mi`. Hugetlb pages are charged
to the cgroup of the process that allocates them, and the driver allocates
both kinds it uses:

- the DPDK EAL's mempools: two ports × `RUSTERON_DPDK_MBUFS_PER_PORT` × about
  2.3 KiB (a 2 KiB data room, the headroom and the mbuf's headers), about
  75 MiB at 16384, plus the EAL's rings and memzones;
- every log buffer in Aeron's directory, which it creates and touches page by
  page (non-sparse): up to `RUSTERON_AERON_HUGE_SIZE` (1 GiB), the size of
  the hugetlbfs the bootstrap mounts, and refuses to mount larger than the
  free pool.

So the mempools plus the Aeron size must fit in the driver pod's
`hugepages-2Mi` (2 GiB, which is also its limit), and every pod's `hugepages-2Mi` on the node must fit in the pool.
Raise the mbuf count or the Aeron size only with the pod's request, and the
pod's request only with `RUSTERON_HUGEPAGES` (`userdata.sh`'s environment).

### Client pods

The latency was measured with the clients inside the driver's pod (`rtt`, on
its spare core). An application in a pod of its own on the node reaches the
driver through the same directory and needs the same treatment:

- the `/mnt/huge-aeron` hostPath (`type: Directory`) mounted at
  `/mnt/huge-aeron`, and `AERON_DIR=/mnt/huge-aeron/aeron` (not
  `/dev/hugepages`, which holds the EAL's memory);
- `AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY=true`, so its mappings are faulted
  in before the first message rather than on the hot path (the driver's
  `AERON_TERM_BUFFER_SPARSE_FILE=false` already fills the log buffers it
  creates);
- the `rusteron-dpdk/ena=true` node selector and the `rusteron-dpdk` taint's
  toleration;
- Guaranteed QoS with whole cores: requests equal to limits on every
  resource, so the static CPU Manager (`full-pcpus-only`) gives it physical
  cores of its own (SMT is off, so one CPU is one core);
- each busy-spinning thread pinned to a core of its own from the pod's cpuset
  (`Cpus_allowed_list` in `/proc/self/status`, as the entrypoint reads it),
  one logical CPU a core, and Aeron's client conductor off those cores.

```yaml
spec:
  nodeSelector:
    rusteron-dpdk/ena: "true"
  tolerations:
    - key: rusteron-dpdk
      operator: Exists
      effect: NoSchedule
  containers:
    - name: app
      env:
        - name: AERON_DIR
          value: /mnt/huge-aeron/aeron
        - name: AERON_CLIENT_PRE_TOUCH_MAPPED_MEMORY
          value: "true"
      resources:
        requests: {cpu: "4", memory: 1Gi}
        limits: {cpu: "4", memory: 1Gi}
      volumeMounts:
        - name: aeron-huge
          mountPath: /mnt/huge-aeron
  volumes:
    - name: aeron-huge
      hostPath:
        path: /mnt/huge-aeron
        type: Directory
```

### Throughput per channel

One UDP flow carried at most 7.75–7.77 Gbit/s of Aeron frames on
`c7i.4xlarge` (round 7), kernel UDP and DPDK alike; AWS documents up to
10 Gbit/s within a cluster placement group. An Aeron channel is one flow whatever its stream
ids, so one channel carries at most about 0.7 M msgs/s of 1,344 B or 15 M of
32 B. Spread a heavier stream over several channels on different ports and
divide the traffic in the application. The transport's transmit cap
(`RUSTERON_DPDK_TX_RATE_MBPS`) is per NIC, not per flow: it cannot raise this,
and it is off unless you set it. Measurements:
[`BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/md-dpdk/BENCHMARKS.md#one-flows-ceiling-on-aws).

## 4. Entrypoint

`entrypoint.sh` reads the pod's cpuset from `Cpus_allowed_list` in
`/proc/self/status`, not from `/sys/fs/cgroup/cpuset.cpus.effective`:
containerd gives a privileged container no cgroup namespace, so in this pod
that file is the host's cgroup root and lists every online CPU. It keeps one
logical CPU per physical core, requires at least three, and assigns them in
order:

| Core | Role | Settings |
|---|---|---|
| 1st | housekeeping: the Aeron conductor (Aeron's default backoff idle, not busy-spinning), the EAL main lcore, the transport's housekeeping and local-I/O threads | `AERON_CONDUCTOR_CPU_AFFINITY`, `RUSTERON_DPDK_HOUSEKEEPING_CPU`, `RUSTERON_DPDK_EAL_ARGS=-l <cpu>` |
| 2nd | sender, busy-spinning | `AERON_SENDER_CPU_AFFINITY`, `AERON_SENDER_IDLE_STRATEGY=noop` |
| 3rd | receiver, busy-spinning | `AERON_RECEIVER_CPU_AFFINITY`, `AERON_RECEIVER_IDLE_STRATEGY=noop` |
| rest | spare: unused by the driver (a client such as `rtt` can take one) | |

It also sets `AERON_THREADING_MODE=DEDICATED`, the wildcard port ranges
`20000 20999` and `21000 21999` (Aeron's format is `"<low> <high>"`; a dashed
range fails driver context creation), `AERON_MTU_LENGTH=1408` and
`RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena`. Any variable already in the
environment wins. It logs one line,
`rusteron entrypoint: transport=… cpuset=… housekeeping=… conductor=… sender=… receiver=… spare=…`,
then `exec`s the media driver under `taskset -c <housekeeping>`, so every
thread that is not an agent stays on the housekeeping core; the agents pin
themselves.

The `RUSTERON_DPDK_SENDER/RECEIVER_{PCI,IPV4_CIDR,GATEWAY}` values are injected
by the device plugin at allocation time; the media driver exits nonzero if any
are missing and never falls back to the socket driver.

## 5. Device plugin

`rusteron-dpdk-device-plugin` reads `/var/lib/rusteron-dpdk/ena-pairs.json`,
advertises one `rusteron.io/dpdk-ena-pair` per healthy pair (both BDFs still
bound to their inventory driver and their device nodes live), and at
`Allocate` injects the pair's `RUSTERON_DPDK_*` variables and device nodes
(`/dev/uioN` for igb_uio, the VFIO group nodes for vfio-pci) into the driver
pod. It reads the inventory again, and validates it again, whenever the file's
modification time or inode changes, so a bootstrap rerun (which deletes it,
rebinds the pair, perhaps under new uio numbers, and writes it again) reaches
it without a restart; while the file is missing or unsafe it advertises and
allocates no pair. At start it still refuses an unsafe inventory and exits.
It stops on SIGTERM or SIGINT.

## 6. Fixture tests

The node scripts and the entrypoint are tested against fake sysfs, /proc and
IMDS trees, with no node:

```bash
bash node/test/test-bootstrap.sh
```

It covers preflight for both drivers (igb_uio with and without IOMMU
passthrough), the bootstrap's selection, its release of the ENAs from
systemd-networkd and their hotplug units, inventory, host tuning and its
optional settings, the Aeron hugetlbfs against the free pool, the kernel
command line on a VM and on bare metal, IRQ placement (the pass and its
check, an IRQ the kernel keeps, one it moves only at its next interrupt,
and the watcher re-pinning after a link event), and idempotent rerun (dry
run: no
host sysfs, /proc or networkd directory is touched), provisioning (SMT off
on a node that still has it and CPU lists of one CPU per physical core, the
no C-state cap, the kubelet hold on failure), the user data's size
with an incompressible CA, its kubelet settings and a dry run of the scripts
it embeds, `launch-template.py`'s `CpuOptions` (present on x86 with SMT,
absent on Graviton, bare metal and 1-thread types), and the entrypoint's CPU
layout. It needs bash and python3; in a container, from the
ergon repo root:

```bash
docker run --rm -v "$PWD":/src:ro -w /src rust:1-slim-bookworm bash -c \
  'apt-get update -qq && apt-get install -qq -y python3 >/dev/null && bash md-dpdk/deploy/aws-eks/node/test/test-bootstrap.sh'
```

## 7. Live test

`md-dpdk/benches/cloud/aws/eks.sh` builds both images, creates a cluster and
two nodes (launched with this directory's user data; `EKS_TYPE` may be a
`*.metal` type) tagged `owner=ergon-lab`, deploys this base through an
overlay (images in ECR, the driver pod on four CPUs so `rtt` has a
spare one), checks the settings the latency depends on (SMT off
(`/sys/devices/system/cpu/smt/active` 0), the kernel command line
(`nosmt` and the isolation flags), the per-boot tuning, write-combining on
both data BARs, kubelet's CPU
Manager, the driver pod's cpuset and thread placement, and the CPU quota:
kubelet 1.36 or later (`EKS_K8S`, default 1.36), `cpuCFSQuota` true,
`DisableCPUQuotaWithExclusiveCPUs` on, `cpu.max` `max` on the driver's
container and pod, and the device plugin's 100m limit still a quota; it reports
`vm.min_free_kbytes` and the primary ENA's coalescing without judging them),
runs `rtt` across the DPDK ENAs, and deletes and verifies every resource on
exit. `EKS_KEEP=1` keeps the run (and its bill) for work by hand;
`EKS_DELETE=<stamp>` deletes it. The script's header lists every setting.
