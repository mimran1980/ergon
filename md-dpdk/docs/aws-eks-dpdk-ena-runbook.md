# AWS EKS DPDK ENA — rollout and rollback runbook

Operational guide for rolling the `ergon-md-dpdk` DPDK ENA
kernel-bypass transport out to production EKS cells and — when the acceptance
gates say so — back to the default kernel-UDP transport, with no binary
change. Covers §12 benchmarking prerequisites and the §13.2 rollback
rehearsal that the acceptance script automates.

Scope: Amazon Linux 2023, Linux x86_64 Nitro, one Media Driver pod per
dedicated node with its own ENA pair. The node's **primary ENA is never
touched** — it stays kernel-owned and carries EKS networking, DNS, telemetry,
and the kernel-UDP rollback path.

---

## 1. Prerequisites

- Cluster cell per §10 with `deploy/aws-eks/` applied
  (`kubectl apply -k deploy/aws-eks/`, through an overlay that points the
  images at your registry): device plugin, Media Driver DaemonSet and its
  ConfigMap. Every dedicated node is launched with the user data
  `deploy/aws-eks/node/userdata.sh` prints, which provisions it on first boot
  and installs the per-boot bootstrap unit (see `deploy/aws-eks/README.md`).
- Node label `rusteron-dpdk/ena=true` on the dedicated nodes and the
  `rusteron-dpdk` taint tolerated by the DaemonSet.
- Both dedicated nodes in the **same cluster placement group**. Placement group
  + ENA Express are the dominant inter-node latency levers on Nitro; without a
  shared placement group the §12 numbers are not representative. The acceptance
  script reads `placement/placement-group-name` from IMDS on both nodes,
  warns when it is absent, and fails the run when the two nodes differ.
- Node kernel cmdline carries `isolcpus`/`nohz_full`/`rcu_nocbs` on every CPU
  but the housekeeping core, `irqaffinity` on it, `iommu=pt`,
  `transparent_hugepage=never`, and no `max_cstate` cap.
  `provision-node.sh` sets them on first boot and reboots once; the per-boot
  bootstrap fails, leaving the node without a DPDK pair, when they are missing.
  A failed provision holds kubelet back (a drop-in asserting
  `/var/lib/rusteron-dpdk/provision-failed` is absent), so that node never
  joins; replace it.
  `chronyd` active: the transport's timestamps are software clock readings and
  need a disciplined host clock.
- The secondary ENAs bound to `igb_uio` loaded with `wc_activate=1` (the only
  write-combining binding on virtualised EC2), so the Media Driver pod runs
  privileged (`deploy/aws-eks/media-driver-daemonset.yaml` gives the reason).
- Built binaries (see §4 of the acceptance script header):
  - `dpdk-harness` with `--features dpdk` (release),
  - `media_driver` release binary,
  - and a machine with `aws`, `kubectl`, `ssh`/`scp`, `jq`.
- **No Media Driver pod on the test nodes.** The acceptance harness binds the
  ENA pairs itself, so it must not race the DaemonSet. A DaemonSet has no
  replica count to scale; park it on a node label nothing carries instead,
  and remove that selector afterwards:
  ```bash
  kubectl -n <cell-ns> patch ds rusteron-media-driver --type merge \
    -p '{"spec":{"template":{"spec":{"nodeSelector":{"rusteron-dpdk/parked":"true"}}}}}'
  kubectl -n <cell-ns> rollout status ds rusteron-media-driver
  # afterwards:
  kubectl -n <cell-ns> patch ds rusteron-media-driver --type json \
    -p '[{"op":"remove","path":"/spec/template/spec/nodeSelector/rusteron-dpdk~1parked"}]'
  ```
- **The acceptance script does not run end to end on these nodes as
  written:**
  - `rollback` runs its kernel-UDP scenario, then requires the secondaries to
    be bound to `vfio-pci` and fails on that check.
  - `bench` dies in its first step, `ena_bind_kernel`: it unbinds from
    `vfio-pci` (a no-op for a device on `igb_uio`), clears `driver_override`
    and probes, so the device stays on `igb_uio`, gets no netdev, and the run
    stops with `got no net iface after probe` on the sender, before the
    receiver is touched. The binding survives, but the sender's
    `driver_override` is left empty, and re-running the node bootstrap does
    not refill it (it leaves an already-bound device alone). Restore it before
    the next unbind or probe:
    `echo igb_uio | sudo tee /sys/bus/pci/devices/<sender-bdf>/driver_override`.

## 2. Acceptance run (§11.3 + §12)

```bash
# From the ergon repo root, after
# `cargo build -p ergon-md-dpdk-harness --features dpdk --release` and
# `cargo build -p ergon-md-dpdk --features dpdk --bin media_driver --release`:
scripts/aws-dpdk-acceptance.sh all
```

On igb_uio nodes `all` stops at `rollback` (§1); run the phases one by one
there.

Phases are rerunnable (`functional | bench | rollback | metadata`) and all
artifacts land in `$RUSTERON_ACCEPTANCE_ARTIFACTS/run-<timestamp>/` — a
versioned bundle with the per-scenario reports, raw latency samples, gate
verdict, and `env-metadata.json` (§12.2).

Key environment overrides (all optional):

| Variable | Default | Purpose |
|---|---|---|
| `RUSTERON_ACCEPTANCE_CELL` | `dpdk-test` | cell name for metadata |
| `RUSTERON_ACCEPTANCE_ARTIFACTS` | `/tmp/rusteron-aws-acceptance` | artifact root |
| `RUSTERON_HARNESS_BIN` | `target/release/dpdk-harness` | prebuilt harness |
| `RUSTERON_MEDIA_DRIVER_BIN` | `target/release/media_driver` | prebuilt driver |
| `RUSTERON_ACCEPTANCE_SSH_USER` | `ec2-user` | node SSH user |
| `RUSTERON_ACCEPTANCE_SSH_KEY` | (none) | optional `-i` key |
| `RUSTERON_ACCEPTANCE_WARMUP` | 30 | saturation warm-up (s) |
| `RUSTERON_ACCEPTANCE_RUNS` | 10 | runs per size/load |
| `RUSTERON_ACCEPTANCE_RUN_SECS` | 60 | run duration (s) |
| `RUSTERON_ACCEPTANCE_SIZES` | `64 256 1408 16384 1048576` | message sizes |
| `RUSTERON_ACCEPTANCE_COMMON_FRAC` | 0.5 | common-load fraction of saturation |
| `RUSTERON_ACCEPTANCE_STRESS_FRAC` | 0.7 | stress-load fraction |

The script picks the first two `rusteron-dpdk/ena=true` nodes, so the cell must
have at least two ready nodes. It verifies both nodes are in the same cluster
placement group (warn if absent, fail if they differ) and records each node's
placement group and chrony state in `env-metadata.json`, alongside the existing
per-node fields (instance type, AZ, kernel, DPDK/ENA versions).

## 3. Rollout (§13.1)

Sequence — non-prod first, then one prod AZ cell at a time:

1. **Non-prod cell.** Run the full acceptance bundle (§2). The bundle must end
   with `PRIMARY GATE: PASS` (see `gates-verdict.txt`) before anything ships.
   The ENA Express decision lands in `ena-express-decision.json`; **default:
   ENA Express stays OFF** unless the evidence says otherwise (see §12.3 of the
   benchmark report).
2. **One prod AZ cell.** Apply `deploy/aws-eks/` with the DaemonSet scheduling
   on that cell's dedicated nodes. Confirm per-pod health before advancing:
   ```bash
   kubectl -n <ns> get pods -o wide -l app=rusteron-media-driver
   kubectl -n <ns> logs -l app=rusteron-media-driver | grep -E 'transport backend|media driver started'
   ```
   Every pod must log `transport backend: dpdk-ena` and `media driver started`.
   Watch `rusteron-media-driver`-reported Aeron counters and the pod's
   `backpressure` / `gaps` (harness reports) for the first few hours.
3. **Advance one AZ cell at a time.** Leave a rollback window (e.g. two
   release cycles) between cells. There is no "flagship" toggle — the transport
   is selected per deployment by `RUSTERON_MEDIA_DRIVER_TRANSPORT`, so cells
   can hold different transports simultaneously. This is deliberate: a cell is
   the smallest revertible unit.

Never change the transport on more than one AZ cell in a single change. A
single failure domain failing is the intended blast radius.

## 4. Rollback (§13.2)

Rollback is a **config change, not a binary change**: the same image supports
both transports, selected at driver start from the environment.

```bash
# 1. Flip the DaemonSet's transport env to the default (kernel UDP), with
#    Aeron's own receiver IO vector capacity: the ConfigMap's 16 is for DPDK,
#    and kernel busy polling with 16 dropped frames at the NIC. A container
#    env entry overrides the ConfigMap's.
kubectl -n <ns> set env ds/rusteron-media-driver \
  RUSTERON_MEDIA_DRIVER_TRANSPORT=default AERON_RECEIVER_IO_VECTOR_CAPACITY=4
# 2. The DaemonSet's default updateStrategy is RollingUpdate
#    (maxUnavailable 1): it replaces one node's pod at a time.
kubectl -n <ns> rollout status ds/rusteron-media-driver
# 3. Clients reconnect to the *primary-ENA* endpoints. The DPDK harness's UDP
#    env is exactly this: MDC control + endpoints are the node private IPs.
# 4. Re-establish and check IRQ placement on every node: the kernel path now
#    carries the workload, and any ENA the kernel driver brought up again
#    (a reset, a ring change, a data ENA handed back) put its queue IRQs on
#    the driver's per-queue hints, mostly isolated CPUs. The watcher re-pins
#    them on the link event; this pass does it once more and fails, naming
#    the IRQ, if it cannot place one that can move.
for node in $(kubectl get nodes -l rusteron-dpdk/ena=true -o jsonpath='{.items[*].metadata.name}'); do
  ssh ec2-user@$node 'systemctl is-active rusteron-irq-affinity &&
    sudo /usr/local/sbin/irq-affinity.sh pin' || echo "IRQ placement FAILED on $node"
done
```

What must be true after rollback:

- Every pod logs `transport backend: socket` (not `dpdk-ena`).
- Step 4 passed on every node: `rusteron-irq-affinity` active and
  `irq-affinity.sh pin` ending in `irq-affinity ok: IRQs on CPUs ...`. A
  `warn` line names an IRQ the kernel keeps on an isolated CPU (a managed
  one, such as an NVMe queue serving isolated CPUs only, which fires only for
  I/O issued there), or one there `until its next interrupt`: on a VM, which
  has no interrupt remapping, the kernel carries out a move only in the
  IRQ's next interrupt, so an idle queue's IRQ stays listed until traffic
  reaches it, and then fires there once or twice more. Neither is a
  failure.
- Clients resume publishing/receiving over the nodes' private-IP endpoints
  (primary ENA, kernel UDP) — **no client change**.
- The secondary ENAs stay bound to their DPDK driver (`igb_uio`, or
  `vfio-pci` on a vfio node) on the host, so the DPDK path remains ready to
  re-enable. Rollback does **not** unbind them. Whoever hands one back to the
  kernel `ena` driver anyway (to carry kernel UDP on it, or the §12.1
  baseline in §5) gets a fresh probe with every queue IRQ on the driver's
  hints: after `ip link set <iface> up`, run
  `sudo /usr/local/sbin/irq-affinity.sh pin` on that node and start nothing
  latency-sensitive until it passes. Check the binding with:
  ```bash
  for node in $(kubectl get nodes -l rusteron-dpdk/ena=true -o jsonpath='{.items[*].metadata.name}'); do
    ssh ec2-user@$node 'inv=/var/lib/rusteron-dpdk/ena-pairs.json
      pci=$(jq -r ".pairs[0].sender.pci" $inv)
      echo "want $(jq -r ".pairs[0].sender.driver" $inv)," \
        "have $(basename "$(readlink /sys/bus/pci/devices/$pci/driver)")"'
  done
  ```
- The kernel-UDP path carries the workload at the §12.3 kernel baseline —
  which the acceptance run measured, so rollback performance is a known
  quantity, not a surprise.

The acceptance script rehearses this (§`phase_rollback`): it reruns a
bidirectional scenario with `RUSTERON_MEDIA_DRIVER_TRANSPORT` unset (default)
over the primary ENA and asserts both nodes still hold `vfio-pci` on their
secondaries afterwards. On igb_uio nodes the scenario runs and the phase then
fails on that assertion, so check the binding with the loop above instead.

**To roll forward again** after a rollback: remove both overrides
(`kubectl -n <ns> set env ds/rusteron-media-driver
RUSTERON_MEDIA_DRIVER_TRANSPORT- AERON_RECEIVER_IO_VECTOR_CAPACITY-`, so the
device plugin's `dpdk-ena` and the ConfigMap's 16 apply again), confirm the
backend log lines,
then watch the same counters as §3.

## 5. Operational notes

- **Node latency tuning is applied by the bootstrap at every boot**: every
  IRQ and the default IRQ affinity on the housekeeping CPUs, irqbalance
  disabled, transparent hugepages `never`, KSM and NUMA balancing off. The
  IRQs stay there after boot: `rusteron-irq-affinity` re-pins them on every
  link event, since the ENA driver puts its queue IRQs back on their
  per-queue hints whenever it brings an interface up (a device reset, a
  queue or ring change, a rebind to the kernel driver), and the bootstrap
  fails if the watcher is not running. Its journal
  (`journalctl -u rusteron-irq-affinity`) logs each IRQ it moves and where
  from; `sudo /usr/local/sbin/irq-affinity.sh pin` re-pins and checks on
  demand (`deploy/aws-eks/README.md`, IRQ placement after boot). The
  kernel keeps its own `vm.min_free_kbytes` and the primary ENA its interrupt
  coalescing, and busy polling stays off, unless
  `/etc/rusteron-dpdk/node.env` sets `RUSTERON_MIN_FREE_PCT` or
  `RUSTERON_BUSY_POLL_US` (`deploy/aws-eks/README.md`). The data ENAs are
  marked unmanaged for systemd-networkd
  (`/etc/systemd/network/00-rusteron-dpdk-data.network`) and their udev
  hotplug units masked for the boot, so an ENA handed back to the kernel is
  not re-addressed by DHCP: give it its address and routes yourself. The
  kernel command line comes from first boot; changing it needs a node
  replace. Verify with `cat /proc/cmdline /sys/devices/system/cpu/isolated
  /sys/devices/system/cpu/nohz_full`, `sysctl vm.min_free_kbytes
  net.core.busy_poll net.core.busy_read`, and `chronyc tracking` for clock
  offset. The preflight also warns when `chronyd` is not active.
- **Write-combining.** ENA's low-latency queues need the device's
  prefetchable BAR mapped write-combining. While the driver runs, the BAR's
  start address (third line of `/sys/bus/pci/devices/<bdf>/resource`) appears
  as `write-combining` in `/sys/kernel/debug/x86/pat_memtype_list`.
- **`RUSTERON_DPDK_TX_DESCRIPTORS` stays at most 256**: an ENA queue takes no
  more under the transport's large LLQ entries (`llq_policy=3`, the default;
  512 under `llq_policy=1`), and a larger ring fails the driver's start.
- **ENA Express** is toggled per-ENI with `scripts/toggle-ena-express.sh`
  (`on|off`). It requires SRD-capable instance types and an ENA that supports
  it; the script verifies the resulting `EnaSrdSpecification`. The acceptance
  gate keeps it OFF unless it delivers a ≥5% median-p99 improvement at every
  message size with no throughput regression.
- **The §12.1 kernel baseline temporarily returns the secondaries to the
  kernel `ena` driver**, which the acceptance script does from `vfio-pci` and
  then restores to `vfio-pci` (the driver_override + probe path the bootstrap
  uses). The rebind puts the data ENAs' queue IRQs on the driver's hints,
  across the isolated CPUs; the watcher moves them back on the link-up, and
  `sudo /usr/local/sbin/irq-affinity.sh pin` after the interfaces are up
  confirms it before the baseline is measured. If a run on a `vfio-pci` node
  fails during the kernel baseline,
  restore the pair by re-running the node bootstrap
  (`sudo systemctl restart rusteron-dpdk-ena`, with
  `RUSTERON_DPDK_DRIVER=vfio-pci` in `/etc/rusteron-dpdk/node.env`), which
  rebinds a device the kernel driver holds. On igb_uio nodes the baseline
  never unbinds the device; §1 says what it leaves behind. The primary ENA remains the §13.2 rollback path and
  is never touched.
- **Fragments**: the DPDK path never fragments (DF + MTU enforced in the
  DPDK transmit path, unit-tested). The kernel-UDP side is checked with
  tcpdump during acceptance; any `frag` capture fails the run.
- **A failed DPDK start is a loud failure.** `configure_media_transport_from_env`
  returns an error and the process exits nonzero — it never silently falls back
  to the socket driver. Treat a `transport backend: socket` line as an explicit
  operator decision, not a default.
