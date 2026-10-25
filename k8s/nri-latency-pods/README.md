# ergon-latency-pods

Latency-pod extensions for Kubernetes. One directory, three kinds of thing:

| kind | what | where the change lands |
|---|---|---|
| node | things a normal pod cannot do at all | nodeadm user data, systemd units, the plugin config |
| plugin | things an NRI plugin can grant on opt-in | `k8s/nri-shared-cpus` |
| admission | things a policy should refuse or strip | `admission/` here (ValidatingAdmissionPolicy) |

The grants are in the published `ergon-shared-cpus` image
(`ghcr.io/mimran1980/ergon-shared-cpus:0.1.2`). This directory is not a
second image. Its admission YAML and node units stay in the repo. Experimental.
EKS is not verified. Latency figures are not measured.

Kubernetes HFT tuning
([book](https://mimran1980.github.io/ergon/deploy/hft.html)) is the list of
node and pod settings. This directory is the admission half and the node
units. The shared-CPU half is the public chart. Apply only
`admission/priority-class.yaml` and
`admission/validating-admission-policy.yaml`. The kind sample
`k8s/nri-shared-cpus/examples/try` installs the chart, runs an unlabelled
pod that gets the shared CPU, shows the plugin refusing a hot pod (a kind
node has no `isolcpus`), and shows this policy denying an exec probe. It
does not grant hugepages, an isolated cpuset partition, io_uring seccomp,
or a PHC. Not yet run on kind. The chart writes `seccomp`, `ptp` and `schedFifo` as `"off"`.

## Why not AKS

**AKS cannot do this.** It has no supported way to change containerd (so an
NRI plugin cannot be installed) and no `reservedSystemCPUs` list. Everything
here needs one of those. **The target is EKS on AL2023**: containerd 2.3.5
with NRI on by default. The cloud lab
(`md-dpdk/benches/cloud/aws/eks-nri.sh`) runs on EKS with SPOT `c8g.xlarge`
nodes. If no enabled region has Spot capacity for that shape, it tries
`c9g.xlarge` and then `c7g.xlarge` (also 4 vCPU and no SMT, so the same CPU
pins) and only then on-demand, and the results name the type that ran.

## The five extensions

A pod author names **kinds and modes only**; values always come from the
plugin config or the node's facts. Every grant is opt-in by annotation, and a
grant that cannot be given is refused (fail closed): the kubelet shows the
text in the pod's events as `CreateContainerError` and it names the fix.

| # | extension | mechanism | pod opt-in | env the app sees |
|---|---|---|---|---|
| 1 | sized hugetlbfs mount for `AERON_DIR` | NRI `AddMount` `type: hugetlbfs` with `pagesize=2M,size=N` + `AddLinuxHugepageLimit`; N = the container's `hugepages-2Mi` limit | `hugepages.ergon.io/<ctr>: "2M"` + `hugepages-2Mi` (requests = limits) | `ERGON_HUGEPAGES_DIR`, `ERGON_HUGEPAGE_SIZE` |
| 2 | `cpuset.cpus.partition=isolated` on the exclusive CPUs | `createRuntime` hook makes a **child** cgroup holding only E, writes `cpuset.cpus.exclusive` down the hierarchy, then `cpuset.cpus.partition=isolated`, and reads it back | `cpu-partition.ergon.io/<ctr>: "isolated"` | `ERGON_ISOLATED_CGROUP`, `ERGON_ISOLATED_CPUS`, `ERGON_SHARED_CPUS` |
| 3 | io_uring seccomp profile | (a) NRI `SetLinuxSeccompPolicy`: a clone of the request's profile plus the `io_uring_*` allow rule; (b) fallback, pod field `seccompProfile: Localhost` | `seccomp.ergon.io/<ctr>: "io-uring"` (or none, for the Localhost fallback) | `ERGON_SECCOMP` |
| 4 | admission: refuse exec probes and always-restart sidecars | `ValidatingAdmissionPolicy` (CEL, GA 1.30) | pod label `ergon.io/latency: "hot"` | — |
| 5 | NRI device grants | NRI `AddDevice` + a read-only bind of the node facts file | `ptp.ergon.io/<ctr>: "auto"` (NIC PHC `/dev/ptp*`); `cpuqos.ergon.io/<ctr>` (`/dev/cpu_dma_latency`, x86 only) | `ERGON_PTP_DEVICE`, `ERGON_NODE_FACTS`, `ERGON_CPU_DMA_LATENCY` |

### Item 1: the app owns `AERON_DIR`

The plugin mounts a **sized** hugetlbfs at `/dev/hugepages` and exports
`ERGON_HUGEPAGES_DIR` and `ERGON_HUGEPAGE_SIZE`. It does **not** set
`AERON_DIR`. The app creates `$ERGON_HUGEPAGES_DIR/aeron`, points `AERON_DIR`
there, sets `AERON_FILE_PAGE_SIZE=$ERGON_HUGEPAGE_SIZE`, and does not set
`AERON_PERFORM_STORAGE_CHECKS=false`.

`emptyDir: {medium: HugePages-2Mi}` is the wrong shape: Kubernetes mounts it
without a `size=`, and Aeron refuses an unsized hugetlbfs. A sized hugetlbfs
on the node would need `hostPath`, which PSS Baseline forbids.

`size=` is a **capacity, not a reservation**: the mount succeeds with
`HugePages_Total=0` and `df` still shows `size`. Pages allocate on **fault**,
and a fault needs a page source. Measured in a Debian bookworm container,
2 MiB pagesize, `size=32M`, one page touched by `mmap`:

| node state | first fault | evidence |
|---|---|---|
| `nr_hugepages=0`, `nr_overcommit_hugepages=0` | `ENOMEM` | no page taken; `HugePages_Surp=0` |
| `nr_hugepages=0`, `nr_overcommit_hugepages=4` | allocates | surplus page; `HugePages_Surp=1` |
| `nr_hugepages=4` | allocates | pool page; `HugePages_Free` 4 → 2 |

So the node must carry a real pool (`hugepagesz=2M hugepages=N`, which is what
the `hugepages-2Mi` request draws from) or `nr_overcommit_hugepages`. K8s
accounts the preallocated pool, so size the request to the pool you actually
have. Allocation is **mmap-only** — `write()` on hugetlbfs returns `EINVAL`
even with a full pool, so the storage layer has to map and fault.

### Item 2: move-then-pin

The child cgroup is a **remote partition**. The kernel needs
`cpuset.cpus.exclusive` written down the hierarchy (outermost ancestor first)
before it accepts `cpuset.cpus.partition=isolated`; the plugin then reads the
file back and requires exactly `isolated`. The parent container cgroup keeps
E ∪ S and stays `member`/`root`, so the helpers keep load balancing.

The child is threaded, so it can hold single threads. At start-up the app
writes its pid to `$ERGON_ISOLATED_CGROUP/cgroup.procs`, moves each helper
thread back through the container cgroup's `cgroup.threads`, then pins the
hot threads to `$ERGON_ISOLATED_CPUS` (kernel lane:
`../nri-shared-cpus/test/cgroup/kernel_test.go`). The partition file itself
is parent-owned and not delegatable, so the plugin writes it. Needs
`cpuset.cpus.exclusive` (Linux 6.7 or a vendor backport); without it every
`cpu-partition` opt-in is refused, naming the cause. Do **not** use
`AddLinuxUnified("cpuset.cpus.partition")` on the container cgroup: that would
strip load balancing from the shared CPUs.

### Item 3: never Unconfined

containerd 2.x `RuntimeDefault` blocks `io_uring_*` (containerd PR #9320).
`Unconfined` works but PSS Baseline forbids it. Two shapes, one profile file
(`admission/profiles/ergon-io-uring.json`): RuntimeDefault plus
`io_uring_setup`/`io_uring_enter`/`io_uring_register`, nothing else.

1. **Plugin (primary):** `seccomp.ergon.io/<ctr>: "io-uring"`, no pod
   security field. List the plugin in `required-plugins.noderesource.dev` so
   the create fails closed if it is absent.
2. **No-plugin fallback:** pod field
   `seccompProfile: {type: Localhost, localhostProfile: ergon-io-uring.json}`.
   The file goes on the node at `/var/lib/kubelet/seccomp/`. PSS Baseline
   allows `Localhost`.

`seccomp.ergon.io/<ctr>: "unconfined"` is refused.

### Item 4: what the policy requires

For pods labelled `ergon.io/latency: "hot"` (a **pod** label, not a namespace
label):

- no `livenessProbe`, `readinessProbe` or `startupProbe` of type `exec`/`command`,
  and no lifecycle `postStart`/`preStop` exec handler (an exec starts a process
  on any CPU of the container's cpuset, the loop's included;
  `httpGet`/`tcpSocket`/`grpc` run from outside and are fine). Admission of
  the Pod does not control a later `kubectl exec` session;
- no container and no init container with `restartPolicy: Always` (an injected
  sidecar shares the pod's CPU budget and wakes on the hot core);
- `ergon.io/shared-millicpus` requested on every application container, and
  `required-plugins.noderesource.dev` listing the exact token
  `"ergon-shared-cpus"` (a substring does not count);
- a whole-number `cpu` request equal to its limit, and `resizePolicy:
  RestartContainer` for `cpu`;
- a `priorityClassName` set (`admission/priority-class.yaml` has `ergon-hot`
  and `ergon-helper` with `preemptionPolicy: Never`).

A `MutatingAdmissionPolicy` that strips `livenessProbe` from legacy manifests
is optional and off by default.

### Item 5: devices, not hostPath

PSS Baseline forbids `hostPath`, so the plugin is the path to a host device
node. The DaemonSet mounts the host's `/dev` read-only at `/host/dev` and the
config's `devRoot: /host/dev` is where the plugin stats the nodes; granted
paths stay the container's `/dev/...` names.

- **PHC** (`ptp.ergon.io`): read-only `/dev/ptp*`, plus a read-only bind of
  `/run/ergon/node.json` so the app can read the NIC's node and PHC name.
  The node keeps chrony on the PHC. Do not run a PTP daemon in a pod: a
  pod that steps the NIC clock affects every tenant.
- **PM QoS** (`cpuqos.ergon.io`, x86 only, default-off): writable
  `/dev/cpu_dma_latency`. The app writes the µs value and keeps the fd; the
  kernel aggregates the min of open requests. On arm64 the annotation is
  ignored. Honest status: a busy-spin loop never idles, so this cannot help
  the loop; it is plumbing for an interrupt-taking core. Keep it off.

### Allowed ride-along: plain rlimits

NRI `AddRlimit` may raise limits for an opted-in container. These are
**limits, not policies**:

| annotation | value | env the app sees |
|---|---|---|
| `rlimits.ergon.io/<ctr>` | a subset of `memlock,nofile,rt-prio,nice` | `ERGON_RLIMIT_MEMLOCK`, `ERGON_RLIMIT_NOFILE`, `ERGON_RLIMIT_RTPRIO` |

`memlock` is needed so the app can `mlockall` the hugepages. `rt-prio` raises
`RLIMIT_RTPRIO` as a **limit value only**. Nothing here takes a real-time
scheduling policy.

## Real-time scheduling: built, default off

`SCHED_FIFO` is not one of the five grants. It lives in
`k8s/nri-shared-cpus`, off unless a node turns it on:

| path | what it does | default |
|---|---|---|
| `ergon-cpus --fifo <prio>` | the exec wrapper takes `SCHED_FIFO` on the thread before exec | off (no flag) |
| `sched-fifo.ergon.io/<container>: "thread"` | the NRI plugin grants `RLIMIT_RTPRIO` and `ERGON_SCHED_FIFO`; the app or `prelock` takes the policy on the hot thread only | `schedFifo: "off"` — the node refuses the annotation |

Why it is off: its one committed A/B (run `070339Z`, `rt` arm) is mixed, and under it the loop's `nohz_full` CPU took timer interrupts again ([evidence](https://mimran1980.github.io/ergon/deploy/hft.html#sched_fifo)). RT throttling can take the CPU away. This project's latency-pods path never takes a policy: the e2e asserts PID 1 stays `SCHED_OTHER` and `ERGON_SCHED_FIFO` is empty.

## Deliberately not built

| item | why |
|---|---|
| SR-IOV VF per pod | Nitro guests have one ENA per ENI, no in-guest VFs |
| a netns-sysctl writer that pretends the knobs are per-netns | `busy_poll`/`busy_read` are global; `gro_flush_timeout` and `napi_defer_hard_irqs` are per-netdev sysfs; `rmem_max` is not writable per-netns. Node config instead (`node-checks.md`) |
| an IRQ DaemonSet that pins the latency pod's NIC IRQs to the SHARED CPUs | IRQs on a CPU that also runs other work are not measured from committed data; IRQs on a core of their own are ([evidence](https://mimran1980.github.io/ergon/deploy/hft.html#interrupt-placement)). The kept-CPU default stays |

## Contract summary

| annotation | value | env the app sees |
|---|---|---|
| `hugepages.ergon.io/<ctr>` | `2M` | `ERGON_HUGEPAGES_DIR`, `ERGON_HUGEPAGE_SIZE` (the app sets `AERON_DIR`) |
| `cpu-partition.ergon.io/<ctr>` | `isolated` | `ERGON_ISOLATED_CGROUP`, `ERGON_ISOLATED_CPUS`, `ERGON_SHARED_CPUS` |
| `seccomp.ergon.io/<ctr>` | `io-uring` or `runtime-default` | `ERGON_SECCOMP` |
| `ptp.ergon.io/<ctr>` | `off`, `auto`, or `/dev/ptpN` | `ERGON_PTP_DEVICE`, `ERGON_NODE_FACTS` |
| `cpuqos.ergon.io/<ctr>` | microsecond target (x86 only) | `ERGON_CPU_DMA_LATENCY` |
| `rlimits.ergon.io/<ctr>` | a subset of `memlock,nofile,rt-prio,nice` | `ERGON_RLIMIT_*` |
| `cpu-shared.ergon.io/<ctr>` / `ergon.io/shared-millicpus` | millicpus | `ERGON_ISOLATED_CPUS`, `ERGON_SHARED_CPUS`, … |

## What lives here

| path | what |
|---|---|
| `admission/validating-admission-policy.yaml` | VAP for pods labelled `ergon.io/latency: "hot"` (see item 4) |
| `admission/priority-class.yaml` | the `ergon-hot` / `ergon-helper` PriorityClasses |
| `admission/profiles/ergon-io-uring.json` | the shipped seccomp profile (RuntimeDefault + `io_uring_*`) |
| `admission/mutating-admission-policy.yaml` | optional MAP JSON-Patch `remove` of `livenessProbe`; off by default |
| `admission/examples/` | one good pod and the bad pods the policy must refuse |
| `tools/io_uring_probe.c` | `io_uring_setup(2)`: prints `fd=N` or `<errno name>`. The kind test uses it |
| `node/netns-sysctls.sh` | **node config helper.** Writes the node's `sysctl.d` and the per-netdev attributes; refuses to pretend a pod can set those knobs |
| `node/irq-affinity.service` | **node config helper.** Keeps the kept-CPU default for NIC IRQs (never the shared CPUs) |
| `node-checks.md` | the node checklist (becomes an ergon-tune check later) |
| `test/e2e.sh` | kind e2e, one check per extension, everything torn down in a trap |
| `test/manifests/` | one manifest per e2e case (`*-yes`, `*-no`, `*-nolimit`, `*-bad`, `seccomp-localhost`) |

The two `node/` files are node config, not pod extensions.

## Benchmark modes (M0–M6)

One entry point, one mode set: `md-dpdk/benches/cloud/aws/eks-nri.sh`.
Manifests live in `md-dpdk/benches/cloud/aws/latency-bench.d/`. No mode has
a committed result yet.

| mode | what it adds |
|---|---|
| M0 `baseline` | plain emptyDir `AERON_DIR`, default seccomp, no extension annotations, no probe |
| M1 `hugepages` | sized hugetlbfs on `/dev/hugepages` via NRI `AddMount`; the app points `AERON_DIR` there |
| M2 `partition` | `cpuset.cpus.partition=isolated` on the exclusive CPUs |
| M3 `seccomp` | RuntimeDefault plus the `io_uring_*` allow rule (never Unconfined) |
| M4 `devices` | PHC `/dev/ptp*` read-only via NRI `AddDevice`; `cpuqos` only on x86 |
| M5 `probe` | baseline plus a `livenessProbe.exec` every 1 s: the harm the admission extension removes |
| M6 `all` | M1+M2+M3+M4 together, no probe |

## Build and test

Nothing is installed on the host. Go, C and kind run in containers.

```sh
# plugin (includes the grants)
cd ../nri-shared-cpus && scripts/check.sh

# this directory: tools, manifests, e2e
test/e2e.sh
```

## Deploy (EKS AL2023)

1. Node (nodeadm `NodeConfig` + user data): `cpuManagerPolicy: static`,
   `reservedSystemCPUs: "0,1"` (0 = system/shared, 1 = NIC IRQs),
   `topologyManagerPolicy: single-numa-node`, `topologyManagerScope: pod`,
   boot `hugepagesz=2M hugepages=256 nohz_full rcu_nocbs isolcpus=managed_irq
   irqaffinity=1 transparent_hugepage=never` on CPUs 2,3.
   No `full-pcpus-only` (c8g has no SMT), no `strict-cpu-reservation`.
   Ship `admission/profiles/ergon-io-uring.json` to
   `/var/lib/kubelet/seccomp/` (and to the plugin config's
   `seccompProfilePath`) if the Localhost fallback is wanted.
2. Install the public plugin. No pull secret. `sharedCpus` is this node's
   housekeeping CPU (inside `reservedSystemCPUs`, not the IRQ CPU).

   ```sh
   helm install ergon-shared-cpus oci://ghcr.io/mimran1980/charts/ergon-shared-cpus \
     --version 0.1.2 \
     --namespace ergon-system --create-namespace \
     --set-string config.sharedCpus=0 \
     --set 'config.namespaces={trading}' \
     --set hostContract.kubeletConfig.enabled=true \
     --set hostContract.kubeletConfig.path=/etc/kubernetes/kubelet/config.json \
     --set hostContract.kubeletConfigDir.enabled=true \
     --set hostContract.sys.enabled=true \
     --set hostContract.proc.enabled=true
   ```

   nodeadm writes the
   base `/etc/kubernetes/kubelet/config.json` and the `NodeConfig` kubelet
   settings as the drop-in `config.json.d/40-nodeadm.conf`;
   `kubeletConfigDir` mounts that directory and the hot-pod check merges
   it as the kubelet does. Without `kubeletConfigDir` every hot pod is refused on AL2023 and unlabelled pods still get the
   grants.

   Image: `ghcr.io/mimran1980/ergon-shared-cpus:0.1.2` (the chart's
   `appVersion`). The chart writes
   `seccomp.ioUring` and `ptp` as `"off"`. A node that grants those writes them
   into the ConfigMap; the keys are in `k8s/nri-shared-cpus/README.md`.
3. `kubectl apply -f admission/priority-class.yaml -f admission/validating-admission-policy.yaml`
4. Label the node `ergon.io/shared-cpus=true`. The label
   `ergon.io/latency: "hot"` is what the admission policy and the plugin's
   hot-pod check match; without `kubeletConfigDir` that check refuses the
   pod on AL2023 (step 2).
5. Read `node-checks.md` and run those checks on every node before a run.

## Security cost, plainly

| item | what it gives away |
|---|---|
| NRI plugin | root on the node: anything on the NRI socket can change any container |
| host `/dev` mounted in the plugin | read-only, but the plugin can stat every device node on the node |
| `RLIMIT_MEMLOCK=inf` | pins node memory; cover it with the pod memory limit |
| `RLIMIT_RTPRIO` | a limit value only (no policy is taken); keep it paired with exclusive CPUs so a bug cannot starve a shared one |
| `RuntimeDefault + io_uring` | a larger kernel surface than RuntimeDefault; the small deliberate widening |
| `Unconfined` seccomp | the whole kernel surface; **refused** |
| `hostNetwork` (the lab) | the node's network namespace and its IAM role via IMDS |
| PM QoS fd in a pod | the pod can widen the node's C-state latency while it lives; the request dies with the fd. Keep the value off |

## Not verified

- NRI socket and nodeadm validator TOML merge on a real AL2023 node.
- That containerd 2.3.5's NRI has `SetLinuxSeccompPolicy` and
  `SetLinuxMemoryPolicy` (both are in the v0.12.3 proto).
- Whether c8g.xlarge exposes `/dev/ptp*`, and whether it is single-NUMA (the
  lab records both).
- `/dev/cpu_dma_latency` in an EC2 guest at all, and whether enabling it
  helps (the one committed C1-cap A/B, on a c7i VM, is mixed:
  [evidence](https://mimran1980.github.io/ergon/deploy/hft.html#c-states)).
- The latency of `cpuset.cpus.partition=isolated` alone (the boot-arg
  **bundle** is measured; the partition alone is "measure first", the lab's
  job).
- `AddMount` bind of `/run/ergon/node.json` into a pod (kind may not land the
  file; the e2e asserts the adjustment either way).
- MutatingAdmissionPolicy JSON-Patch removal on this k8s version.

Apache-2.0. The repo `LICENSE` applies.
