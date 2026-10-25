# ergon-shared-cpus

A Guaranteed Kubernetes pod gets its exclusive CPUs **and** the node's
shared housekeeping CPUs, and every new thread starts on the shared ones.
The busy-spin loop keeps its exclusive core; only sleepy helpers (dial, TLS,
warnings, Aeron conductors) run on the shared CPUs.

One daemon (a containerd NRI plugin and a kubelet device plugin) plus a small
tool, `ergon-cpus`. Source:
<https://github.com/mimran1980/ergon/tree/main/k8s/nri-shared-cpus>.

**This is not a new idea.** [Prior art](#prior-art) names the projects that
already do parts of it and the open Kubernetes issue it answers. Read that
before you choose. What is here is the containerd path that needs no DRA
feature gate, plus the pieces those leave out.

## Status

| item | state |
|---|---|
| tag | **0.1.1** (and `latest`) |
| image | `ghcr.io/mimran1980/ergon-shared-cpus:0.1.1`. Anyone can pull it; no image pull secret |
| image digest | `sha256:69893e0c23ec6c3656053c0d6c4a06c676e33b726d56f362a30ce760908796c0` (`docker buildx imagetools inspect ghcr.io/mimran1980/ergon-shared-cpus:0.1.1`) |
| chart | `oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.1` |
| chart digest | `sha256:77a65305a45ea1b376ce80f9b1d5234b9061f4066a8c5baf5efdb892854fc0d8` |
| platforms | `linux/amd64`, `linux/arm64` |
| checked | `helm template` of the OCI chart with the [Quick start](#quick-start) flags: image `ghcr.io/mimran1980/ergon-shared-cpus:0.1.1`, args `--config`, `--kubelet-config`, `--host-sys`, `--host-proc`, every `config.File` field. `--version` on amd64 and arm64: `ergon-shared-cpus 0.1.1 515846b0201b5fd70a7a6cf3a88839b6180fa9c1 nri=v0.12.3` |
| not run | 0.1.1 not yet run on kind |
| in git, not published | 0.1.2, image and chart: `--kubelet-config-dir` / `hostContract.kubeletConfigDir` merge the kubelet's drop-in directory, so hot pods can pass on EKS AL2023 ([install](#eks-al2023)) |
| not verified | EKS |
| measured | one Azure Cobalt VM, helpers on the shared CPU, with the [node settings](#quiet-isolated-cpus): p50 0.02 µs, p99.99 0.03 µs, max 4.81 to 5.52 µs over 3 reps of 60 s, 0 involuntary switches; 3.5 to 5.5 µs is the hypervisor's floor. Shared-CPU scaling as designed ([BENCHMARKS.md](BENCHMARKS.md)) |
| not measured | a helper's wake-up wait on a shared CPU, the pin's cost, how often node events reach isolated CPUs in production ([BENCHMARKS.md](BENCHMARKS.md#remaining-risk-node-events)) |
| release gate | [`RELEASE.md`](RELEASE.md) |

`helm install` prints NOTES that say the chart is not published. It is
published; ignore that line.

## Quick start

The chart does not set the boot line, the kubelet or containerd. Do those
first. Kubernetes HFT tuning is that node setup plus this plugin plus the
admission policy, not another image:
[Kubernetes HFT tuning](https://mimran1980.github.io/ergon/deploy/hft.html).

1. **Prepare the node.**

   | setting | value | where it goes |
   |---|---|---|
   | Kubernetes | >= 1.33 | cluster |
   | kernel | >= 6.2, cgroup v2. The `cpu-partition` opt-in also needs `cpuset.cpus.exclusive`: Linux 6.7 or a vendor backport (the plugin reads the file, not the version) | node image |
   | CPU isolation | every CPU outside `reservedSystemCPUs` in `isolcpus` (`isolcpus=managed_irq,domain,<cpus>`), plus `nohz_full` and `rcu_nocbs` on them. Never a shared CPU | boot line |
   | isolated-CPU noise | THP `never`; the fair server's runtime 0 on each isolated CPU ([below](#quiet-isolated-cpus)). Not checked by the plugin | every boot |
   | `cpuManagerPolicy` | `static` | kubelet config |
   | `reservedSystemCPUs` | the IRQ CPU(s) plus the shared CPUs | kubelet config |
   | `cpuCFSQuota` | not `false`; feature gate `DisableCPUQuotaWithExclusiveCPUs` not `false` | kubelet config |
   | kubelet config file | `/var/lib/kubelet/config.yaml` (kubeadm, kind). The plugin pod does not start without it. Another path: `--set hostContract.kubeletConfig.path=<file>`. A kubelet started with `--config-dir` also needs `hostContract.kubeletConfigDir` (chart 0.1.2). EKS AL2023: [below](#eks-al2023) | node |
   | NIC IRQs | every hardware NIC IRQ, requested and effective affinity, on the IRQ CPU(s): `reservedSystemCPUs` minus `sharedCpus`. A Hyper-V synthetic NIC (Azure `hv_netvsc`) has no IRQ: each VMBus channel's `device/channels/<relid>/cpu` must be one of those CPUs | node (IRQ pinning) |
   | containerd | >= 2.2, NRI default validator on (block below) | containerd TOML |
   | `ergon.io/shared-cpus=true` | the DaemonSet schedules only on labelled nodes | node label |

   ```toml
   [plugins."io.containerd.nri.v1.nri".default_validator]
     enable = true
     # leave required_plugins empty: a node-wide list would also block
     # kube-system pods and this plugin's own pod while it is down
   ```

   A pod labelled `ergon.io/latency: hot` is checked against the CPU
   isolation, kubelet and NIC IRQ rows at every create, read from the
   kubelet file (merged with its drop-in directory when that is set),
   `/sys` and `/proc`. A miss refuses that create
   (`CreateContainerError`, naming the rule). Unlabelled pods get the same
   grants without the check.

2. **Install the chart.** `config.sharedCpus` must be inside this node's
   `reservedSystemCPUs` and must not be an IRQ CPU. The three
   `hostContract` flags mount the kubelet file, `/sys` and `/proc`
   read-only for the hot-pod check.

   ```sh
   helm install ergon-shared-cpus oci://ghcr.io/mimran1980/charts/ergon-shared-cpus \
     --version 0.1.1 \
     --namespace ergon-system --create-namespace \
     --set-string config.sharedCpus=0 \
     --set 'config.namespaces={trading}' \
     --set hostContract.kubeletConfig.enabled=true \
     --set hostContract.sys.enabled=true \
     --set hostContract.proc.enabled=true
   ```

   <a id="eks-al2023"></a>**EKS AL2023** (needs chart 0.1.2, not yet
   published; not verified on EKS). nodeadm writes the base
   `/etc/kubernetes/kubelet/config.json`, puts the `NodeConfig` kubelet
   settings in `/etc/kubernetes/kubelet/config.json.d/40-nodeadm.conf`, and
   starts the kubelet with `--config-dir` on that directory. The plugin
   merges the `*.conf` drop-ins over the file as the kubelet does:

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

   The drop-in directory exists only when `NodeConfig` sets
   `spec.kubelet.config` (it must: that is where `cpuManagerPolicy: static`
   goes); without it the plugin pod does not start. Chart 0.1.1 rejects
   `kubeletConfigDir` at install; with 0.1.1 on EKS every hot pod is
   refused.

3. **Apply the admission policy** (or set `admission.create=true`: [chart values](#chart-values)).
   Skip `admission/examples/` and the mutating policy.

   ```sh
   kubectl apply -f k8s/nri-latency-pods/admission/priority-class.yaml \
                 -f k8s/nri-latency-pods/admission/validating-admission-policy.yaml
   ```

   It is the HFT admission check. A pod labelled `ergon.io/latency: hot` is
   denied for:
   - an exec startup, readiness or liveness probe, or a lifecycle exec handler;
   - an always-restart sidecar;
   - a shared-millicpu request missing from any application container;
   - a `required-plugins` annotation that is not the exact plugin token;
   - a fractional CPU, or no `resizePolicy: RestartContainer` for `cpu`;
   - no priority class.

   Admission does not control a later `kubectl exec`.

4. **Run a Guaranteed pod** in an allowlisted namespace. It asks for integer
   CPUs and for `ergon.io/shared-millicpus` (millicpus, because extended
   resources are whole numbers and requests must equal limits).
   [`deploy/example-pod.yaml`](deploy/example-pod.yaml) is the full shape.

   ```yaml
   apiVersion: v1
   kind: Pod
   metadata:
     name: trader
     namespace: trading                       # must be in the plugin's allowlist
     annotations:
       required-plugins.noderesource.dev: '["ergon-shared-cpus"]'   # fail closed
   spec:
     nodeSelector: { ergon.io/shared-cpus: "true" }
     containers:
     - name: app
       image: my-app
       resources:
         requests: { cpu: "2", memory: 1Gi, ergon.io/shared-millicpus: "250" }
         limits:   { cpu: "2", memory: 1Gi, ergon.io/shared-millicpus: "250" }
   ```

5. **Verify.**

   ```sh
   kubectl -n ergon-system get pods -l app=ergon-shared-cpus   # Ready
   kubectl get node <node> -o jsonpath='{.status.allocatable.ergon\.io/shared-millicpus}'
   curl -s <pod-ip>:9466/readyz
   curl -s <pod-ip>:9466/metrics
   ```

   Inside the pod, check `cpuset.cpus.effective`, `ERGON_ISOLATED_CPUS`,
   `ERGON_SHARED_CPUS`, and PID 1 `Cpus_allowed_list` (shared CPUs only).
   These prove the plugin ran; [Did the isolation actually hold?](#did-the-isolation-actually-hold)
   proves the layout.

**Flux:** `HelmRepository` type `oci`, url `oci://ghcr.io/mimran1980/charts`,
chart `ergon-shared-cpus` version `0.1.1`, the same values (the three
`hostContract` ones included). Copy the two admission files into
the repo Flux applies. Do not point a Kustomization at all of `admission/`.

### Try it on kind

[examples/try](examples/try) is a throwaway kind cluster that pulls this
chart, runs a pod that gets the shared CPU, and shows the two hot-pod
checks refusing:

```sh
k8s/nri-shared-cpus/examples/try/run.sh
```

| | |
|---|---|
| needs | Docker (at least 6 CPUs), helm, kubectl, kind v0.33.0 |
| cleans up | writes its kubeconfig under `/tmp`; deletes the cluster after this run creates it. A preflight refusal, including a cluster that already has that name, does not delete it |
| passes when | the unlabelled pod's log shows `ERGON_SHARED_CPUS=0`, a non-empty `ERGON_ISOLATED_CPUS`, PID 1 `Cpus_allowed_list` of `0` and `nproc=1`; a hot pod is refused at create with `exclusive CPU not isolated`; the exec-probe pod is denied |
| does not | measure latency, or ask for hugepages, PTP or an isolated cpuset partition. A kind node shares the host kernel and has no `isolcpus`, so no hot pod can start there |
| status | not yet run on kind |

### Latency lab (one Azure VM)

[`lab/azure-vm.sh`](lab/azure-vm.sh) measures the claim on one Cobalt VM
(default `Standard_D8ps_v6`, Debian 13 arm64) with a real `isolcpus` boot
line, so a hot pod can start: kind on the VM, the chart and both images
built there from this checkout.

```sh
DRY_RUN=1 k8s/nri-shared-cpus/lab/azure-vm.sh   # plan, size, price; no cloud call
k8s/nri-shared-cpus/lab/azure-vm.sh             # the run; deletes its resource group on every exit
```

| item | value |
|---|---|
| layout | CPU 0: every NIC IRQ and VMBus channel. CPUs 1-2: housekeeping; `sharedCpus` 1 (e2e, bench) or 1-2 (scale). CPUs 3 to the last: `isolcpus=managed_irq,domain` + `nohz_full` + `rcu_nocbs`, handed out as exclusive (`reservedSystemCPUs: 0-2`) |
| node | with `LAB_QUIET=1` (default) the settings of `lib.sh` `lab_quiet_settings` (among them THP `never` and the fair server off on the exclusive CPUs), recorded in `results/quiet.txt` |
| e2e | a hot pod is admitted and starts; `ps -L` and `Cpus_allowed_list` show its spin thread on its isolated CPU alone and every helper on the shared CPU alone; an exec-probe hot pod is denied |
| probe | [`lab/gapprobe`](lab/gapprobe): spins reading the CPU counter, a histogram of every iteration (p50 to p99.99, exact max), every gap of 1 µs or more with its `CLOCK_REALTIME` end, the loop's context switches and page faults |
| load | `stress-ng --cpu 1 --vm 1` and a loopback UDP reader (20,000 datagrams/s) in a child process of the probe, started from the pod's main thread |
| bench | plugin-on (hot pod, helpers on the shared CPU), 5 reps. `LAB_ARMS="plugin-on plugin-off"` adds the plain static CPU manager arm (chart uninstalled: helpers on the pod's exclusive CPUs) |
| each max | with the `/proc/interrupts` and `/proc/softirqs` growth on its CPU in the 10 ms windows that overlap it, and the spin thread's involuntary context switches |
| trace | `LAB_TRACE=1` (default): one more plugin-on rep under kernel ftrace filtered to its spin CPU; `results/trace-cpu<N>.md` names every switch, IPI, timer and work item there and ties each to a gap |
| scale | `LAB_SCALE=1` (default): how much of two shared CPUs a pod's helpers get at 100 and 1,001 millicpus, idle, and when pods share one; `results/scale.md` ([result](BENCHMARKS.md#shared-cpu-scaling-run-20261024230400)) |
| results | `target/nri-lab/<stamp>/results/report.md`; committed runs in [`lab/results`](lab/results), summarised in [BENCHMARKS.md](BENCHMARKS.md) |
| cost guard | every resource tagged `owner=ergon-lab run=<stamp>`; the group is deleted on every exit and checked gone; at `LAB_MAX_HOURS` (4) a timer on the VM deletes it through the VM's identity, a watchdog here deletes it, and auto-shutdown deallocates it 30 minutes later (a deallocated VM cannot run its timer); runs over 2 hours detach |
| ssh | one multiplexed connection (`ControlMaster`): each new login's seccomp filter interrupts every CPU |
| tests | `just test-nri-lab` (fake `az`/`ssh`, fixtures, the probe's unit tests) |
| status | runs 20261024230400 (scale), 20261024233129 and 20261024234841 (bench, trace, after the fixes): [BENCHMARKS.md](BENCHMARKS.md) |

## What the app sees

For the pod in step 4, inside the container:

| item | value |
|---|---|
| `cpuset.cpus.effective` | the 2 exclusive CPUs plus 1 shared CPU |
| `ERGON_ISOLATED_CPUS` | the exclusive CPUs, sorted, comma list: `1,5` |
| `ERGON_SHARED_CPUS` | this container's shared CPUs: `4` |
| `ERGON_ISOLATED_CORES` | exclusive CPUs grouped by physical core: `1,5` |
| `ERGON_SHARED_MILLICPUS` | `250` |
| `ERGON_HELPER_CPUS` | same as `ERGON_SHARED_CPUS` |
| `OPENSHIFT_ISOLATED_CPUS` / `OPENSHIFT_SHARED_CPUS` | `1,5` / `4` (CRI-O style) |
| affinity of PID 1 and of every new thread | the shared CPUs only |

`250` is a quarter core of **scheduler accounting only**. The plugin adds
`ceil(M/1000)` shared CPUs (here 1). There is no hard cap: helpers may use
the whole shared CPU, shared with the kubelet, system daemons and other
pods.

### Shared CPUs: how much a pod gets

Measured on two shared CPUs, 4 busy helper threads a pod (run
20261024230400, [BENCHMARKS.md](BENCHMARKS.md#shared-cpu-scaling-run-20261024230400)):

| ask | gets | helpers used |
|---|---|---|
| 100 millicpus | 1 shared CPU (`ceil(M/1000)`, the least loaded at start) | 0.99 cores busy, 0.04 idle (1 % duty) |
| 1,001 millicpus | 2 shared CPUs | 1.99 cores |
| two pods on one shared CPU, `cpu: 1` and `cpu: 2`, 100 millicpus each | the same CPU | 0.33 and 0.66 cores: split by cgroup `cpu.weight`, which comes from the integer `cpu` request, not from M |

M is scheduler accounting only (allocatable `len(sharedCpus) x 1000 x
oversubscription`: 2000 here; a pod asking 2,100 stays Pending). No CPU
quota is set, so `nr_throttled` stays 0 and busy helpers take the whole of
their shared CPUs. The container's cpuset is its exclusive CPUs plus those.

## Pinning

Pin hot threads by index into `ERGON_ISOLATED_CPUS`. The app never needs
the real CPU numbers. Leave helpers alone: they start on the shared CPUs.

```c
/* Pin the calling thread to isolated CPU index i. Returns the CPU, or -1. */
int pin_isolated(int i);   /* full code in examples/pin.c */
```

See [examples/pin.c](examples/pin.c) (C) and
[examples/pin.rs](examples/pin.rs) (Rust, std only).

Rules:

- Never run a spinner or SQPOLL on a shared CPU.
- Create helpers from a thread that is still on the shared set.
- Each sleepy helper checks `sched_getcpu()` when it wakes and alarms if it
  is not on `ERGON_SHARED_CPUS`.

Why the second rule:

- The createRuntime hook pins the threads that exist at create onto the
  shared CPUs. A thread created later inherits its creator's mask, so a
  helper created by a pinned hot thread inherits the hot mask. The
  application can change affinity with `sched_setaffinity`.
- That inheritance was observed for a C `pthread_create`
  (`test/e2e/pin-test.c` mode `masks`) on the Docker Desktop linuxkit
  kernel, which has no `isolcpus`.
- A Go 1.27 OS thread started from the hot-pinned thread kept the shared CPU
  `PinProcess` had set; it did not inherit the hot mask.
- Aeron: the client conductor `aeron-client`, started by a thread before
  that thread pinned itself, kept the creator's mask
  (`samples/low-latency-k8s/tests/placement_aeron.rs`, in Docker without
  this hook). Aeron under the hook on a node, and the JVM, were not run.
  The sample that runs Aeron with this plugin is
  [`samples/README.md#low-latency-k8s`](../../samples/README.md#low-latency-k8s).

How the pool is granted:

- The configured pool is fixed. A container does not receive all of it
  unless it asks for it: the allocator grants `ceil(M/1000)` CPUs,
  preferring the assignment it recorded for that pod and otherwise the
  least-loaded CPUs.
- For one trading process, request `M = 1000 × |S|` with
  `oversubscription: 1`. That uses the existing interface and grants the
  whole helper set. A smaller `M` still grants `ceil(M/1000)` CPUs, which is
  scheduler accounting, not a fraction of a core.
- A reconnect rebuilds the table from the environment recorded at create. It
  does not choose replacement helper CPUs for a container that is already
  running. Drain trading before changing the reservation, the profile or the
  shared list.

## ergon-cpus

One tool, three jobs.

| command | what |
|---|---|
| `ergon-cpus hook --shared S` | OCI `createRuntime` hook. The plugin injects it; it sets every thread of the new container to `S` |
| `ergon-cpus [--env NAME=SPEC]... [--affinity shared\|keep] -- cmd` | exec wrapper. Turns `isolated:<i>` into a real CPU number, then execs |
| `ergon-cpus show` | prints the lists, cores, affinity and `cpuset.cpus.effective` as JSON |

SPEC forms for `--env`:

| SPEC | becomes |
|---|---|
| `isolated:<i>` | the i-th CPU of `ERGON_ISOLATED_CPUS` |
| `shared:<i>` | the i-th CPU of `ERGON_SHARED_CPUS` |
| `core:<j>` / `core:<j>:<t>` | CPU t (default 0) of core group j |
| `isolated` / `shared` | the whole list |

Aeron C media driver example:

```yaml
command: ["ergon-cpus",
          "--env", "AERON_SENDER_CPU_AFFINITY=isolated:0",
          "--env", "AERON_RECEIVER_CPU_AFFINITY=isolated:1",
          "--", "aeronmd"]
env: [{ name: AERON_THREADING_MODE, value: DEDICATED }]
```

Leave `AERON_CONDUCTOR_CPU_AFFINITY` and
`AERON_DRIVER_NATIVE_RESOURCE_AGENT_CPU_AFFINITY` unset: those agents start
on the shared CPUs. Keep `AERON_DRIVER_CPUSET_AFFINITY` false; the wrapper
refuses to translate Aeron variables when it is true.

## Chart values

The full list, with comments, is
[`values.yaml`](charts/ergon-shared-cpus/values.yaml); `values.schema.json`
rejects unknown keys. `config.*` keys become
`/etc/ergon-shared-cpus/config.yaml` ([Config](#config)).

| key | default | effect |
|---|---|---|
| `config.sharedCpus` | `"4"` | **required.** The housekeeping CPUs. Inside `reservedSystemCPUs`, not an IRQ CPU |
| `config.oversubscription` | `1` | capacity = `len(sharedCpus) × 1000 ×` this; 1–100, capacity at most 100,000 |
| `config.namespaces` | `[trading]` | allowlist; empty refuses every opt-in |
| `config.defaultAffinity` | `shared` | `shared`: inject the hook. `all`: no hook; use the wrapper |
| `config.hookPath` | `/opt/ergon/bin/ergon-cpus` | must end with `/ergon-cpus`; written as root on the node |
| `config.annotationOptIn` | `false` | allow `cpu-shared.ergon.io/<container>` without the device plugin |
| `config.metricsAddress` | `":9466"` | `/readyz`, `/healthz`, `/metrics` |
| `config.devRoot` | `/host/dev` | where the plugin sees the host's `/dev` (mounted read-only) to grant the PHC and PM QoS nodes |
| `config.rlimits.{memlock,nofile,rtPrio,nice}` | empty (block omitted) | ceilings a pod may ask for: `"inf"` or a positive count |
| `config.seccomp.ioUring` | `"off"` | opt-in grant: `off` or `allow` (`io_uring` seccomp) |
| `config.seccompProfilePath` | `/etc/ergon-shared-cpus/ergon-io-uring.json` | the profile for that grant |
| `config.schedFifo` | `"off"` | opt-in grant: `off` or `thread` |
| `config.ptp` | `"off"` | opt-in grant of the NIC PHC: `off`, `auto` or a `/dev/ptp*` path |
| `config.hugepagesAeronDir` | empty | opt-in hugepages: an absolute `AERON_DIR`; empty omits it |
| `config.nodeProfileLabel`, `config.profiles` | unset | named layouts, below |
| `image.repository` | `ghcr.io/mimran1980/ergon-shared-cpus` | |
| `image.tag` | empty: the chart's `appVersion` (`0.1.2` in git, `0.1.1` in published chart 0.1.1) | ignored when `image.digest` is set |
| `image.digest` | empty | `sha256:…` renders `repository@digest`; empty keeps `repository:tag` |
| `image.pullPolicy` | `IfNotPresent` | |
| `layoutRevision` | empty | a ConfigMap edit does not roll the DaemonSet. Drain trading workloads, then set a new revision, before changing `sharedCpus` |
| `admission.create` | `false` | `false`: a ConfigMap with `data.owner: external`; no policy and no PriorityClasses. `true`: installs `validating-admission-policy.yaml` (policy and binding) and `priority-class.yaml` (`ergon-hot`, `ergon-helper`) |
| `hostContract.{kubeletConfig,sys,proc}.enabled` | `true` in chart 0.1.2 (git), `false` in 0.1.1: the install command sets them | read-only host facts for the hot-pod check: `--kubelet-config`, `--host-sys`, `--host-proc`. With `kubeletConfig` off, every pod labelled `ergon.io/latency: hot` is refused (`kubelet configuration is not mounted`) |
| `hostContract.kubeletConfig.path` | `/var/lib/kubelet/config.yaml` | a `File` mount: the plugin pod does not start when it is missing. Never `FileOrCreate` |
| `hostContract.kubeletConfigDir.enabled`, `.path` | `false`, `/etc/kubernetes/kubelet/config.json.d` | chart 0.1.2. The kubelet's `--config-dir`, a read-only `Directory` mount passed as `--kubelet-config-dir`. Its `*.conf` drop-ins are merged over the kubelet file as the kubelet merges them (lexical order, subdirectories included, JSON merge patch: later fields win, lists replaced, maps merged by key). A missing directory keeps the plugin pod from starting; a drop-in the kubelet would reject (empty, unparsable, not `kubelet.config.k8s.io/v1beta1` `KubeletConfiguration`) refuses hot pods. Set it only when the kubelet runs with `--config-dir` at that path. Needs `kubeletConfig` on |
| `hostContract.{kubeletConfigZ,kubeletOverrides}.enabled` | `false` | optional: a copy of `/configz` that must agree with the merged kubelet config, or a kubelet argv file (or directory of them) whose checked flags must agree with it. Overrides are a conflict check, not a merge |
| `namespace` | `ergon-system` | the chart labels it Pod Security `privileged` |
| `nodeSelector` | `ergon.io/shared-cpus: "true"` | |
| `tolerations` | `[{operator: Exists}]` | |
| `resources` | requests `cpu: 10m`, `memory: 32Mi`; limit `memory: 64Mi` | |
| `metrics.port`, `metrics.service.enabled` | `9466`, `false` | |
| `extraHostPaths` | `[]` | extra host mounts |

- Opt-in grants in this image: hugepages, an isolated cpuset partition
  (`cpu-partition`), `io_uring` seccomp and the NIC PHC. All are off by
  default.
- `config.seccomp.ioUring`, `schedFifo` and `ptp` stay quoted `"off"`:
  unquoted `off` is a YAML 1.1 boolean.
- `cgroupRoot` is not a config field: the parser rejects it. The binary uses
  `/sys/fs/cgroup`; the DaemonSet mounts the host tree read-only there.

**Named layouts** (`config.profiles` and `config.nodeProfileLabel`):

- The chart then writes `profiles` and `nodeProfileLabel` and omits the flat
  `sharedCpus` key. The flat file stays the default. A file that sets both
  is rejected.
- The process reads the node once (`NODE_NAME`, a get-only ClusterRole on nodes) and
  picks the profile its `config.nodeProfileLabel` names. A missing label, an
  unknown profile or an empty `NODE_NAME` stops that process.
- It does not watch the node and does not change labels. Drain before
  changing a label or a profile. Running grants stay until those containers
  are recreated.
- A multi-node cluster was not started for this selection.

## Config

`/etc/ergon-shared-cpus/config.yaml`, from a ConfigMap. Strict YAML, no hot
reload.

```yaml
sharedCpus: "4"            # required. Inside reservedSystemCPUs
oversubscription: 1        # capacity = len(sharedCpus) x 1000 x this, max 100000
namespaces: [trading]      # allowlist; empty refuses every opt-in
defaultAffinity: shared    # shared (hook) | all (no hook; use the wrapper)
hookPath: /opt/ergon/bin/ergon-cpus   # must end with /ergon-cpus (written as root)
annotationOptIn: false     # allow cpu-shared.ergon.io/<container> without the device plugin
metricsAddress: ":9466"
```

## Failure modes

| event | effect |
|---|---|
| plugin down at create | `required-plugins` makes the create fail (containerd >= 2.2); the wrapper refuses without `ERGON_SHARED_CPUS`; the device plugin is down too, so the resource does not allocate. This does not protect a pod that is already running |
| kubelet restart, plugin up | the re-write goes through `UpdateContainer` and keeps the union. The plugin does not clear a CPU quota |
| kubelet restart, plugin down | cpuset shrinks to the exclusive CPUs; helpers fall onto them. `test/e2e.sh` step 9 demonstrates that outage. It is not a passing isolation claim. `Synchronize` re-widens on return and does not prove the contract held while the plugin was down |
| host drift (reserved set, quota, NIC affinity, isolation) | a new hot create is refused. The plugin stays connected: `/healthz` stays up, and this is not a liveness restart |
| kernel < 6.2 or cgroup v1 | the plugin exits with one clear line |
| kernel has no `cpuset.cpus.exclusive` | shared-only pods work; every `cpu-partition` opt-in is refused and the pod stays out of the partition (6.7 or a vendor backport of that file) |
| one physical core is split between the exclusive and the shared CPUs | the container is refused, naming the core: on SMT the hot thread and the helpers would share one core, which is the noise this removes |
| a CPU has no `thread_siblings_list` | the container is refused: without it the plugin cannot tell which CPUs share a core and will not guess |
| `numa.ergon.io` and the node's NUMA topology was never read | the container is refused, naming the cause: a binding is never chosen from a list nothing read |
| `numa.ergon.io: ""` on a machine with several NUMA nodes and no data NIC node recorded | the container is refused and told to name the node itself; node 0 would be a guess |

## Limits

- **AKS**: no supported way to enable containerd NRI plugins and no
  `reservedSystemCPUs`. Not supported.
- **CRI-O**: use its own `shared_cpuset` / `cpu-shared.crio.io` instead.
- **Bottlerocket**: whether runc may exec a hook binary that a pod wrote to
  a host path is not verified (read-only root, SELinux). Use
  `defaultAffinity: all` and the wrapper until checked.
- `kubectl exec` and exec probes start on any CPU of the cpuset, exclusive
  ones included. No exec probes or lifecycle exec handlers on hot containers.
  Admission of the Pod does not control a later exec session. Run debug
  commands as `ergon-cpus -- <cmd>`.
- Apps that read their CPUs from their own affinity (`nproc`,
  `sched_getaffinity`) see only the shared CPUs. Read
  `ERGON_ISOLATED_CPUS` or `cpuset.cpus.effective` instead.
- In-place CPU resize leaves the CPU env stale. Use
  `resizePolicy: RestartContainer` for `cpu`.
- **EKS AL2023**: the kubelet settings are in a drop-in, so hot pods need
  chart 0.1.2 with `kubeletConfigDir` ([install](#eks-al2023)). With chart
  0.1.1 every hot pod is refused there; unlabelled pods get the grants.
- The plugin cannot see the kubelet's command line. If
  `kubeletConfigDir` points at a directory the kubelet does not read (no
  `--config-dir`), the check trusts drop-ins the kubelet ignores. Turn on
  `kubeletConfigZ` with a fresh `/configz` copy to cross-check.
- Not verified on a real EKS node: the NRI socket, nodeadm TOML merge,
  the kubelet drop-in merge, `/opt` writable, `system-node-critical` outside kube-system.
- Measured on one Azure Cobalt VM: the exclusive core with the plugin on,
  helpers on the shared CPU and the [node settings](#quiet-isolated-cpus),
  p50 0.02 µs, p99.99 0.03 µs, max 4.81 to 5.52 µs, 0 involuntary switches
  a minute; the hypervisor's floor is 3.5 to 5.5 µs ([BENCHMARKS.md](BENCHMARKS.md)).
  Node events still reach isolated CPUs ([below](#quiet-isolated-cpus)).
  Not measured: a helper's wake-up wait on a shared CPU.

## Quiet isolated CPUs

`isolcpus` and `nohz_full` leave kernel work that is sent to every CPU.
The lab's trace named it ([BENCHMARKS.md](BENCHMARKS.md#what-reached-the-exclusive-cpu-and-the-fix)).
Set these on the node at every boot; the plugin does not:

| setting | value | why | note |
|---|---|---|---|
| `/sys/kernel/mm/transparent_hugepage/enabled` | `never`: that stops `khugepaged`; `madvise` still runs it (the lab's node was `madvise`) | `khugepaged` calls `lru_add_drain_all`, which runs a kworker on every CPU with queued pages | `transparent_hugepage=never` on the boot line also works. hugetlbfs pages are unaffected |
| `/sys/kernel/debug/sched/fair_server/cpu<N>/runtime`, each isolated N | `0` | the fair server's 1 s timer sends a nohz kick and a reschedule IPI, and keeps the tick on | kernel 6.12 and later; debugfs, lost at reboot (a oneshot unit after `sys-kernel-debug.mount`); safe only where nothing real-time runs on those CPUs |

`samples/low-latency-k8s/node/quiet-cpus.sh` and its unit
`ergon-quiet-cpus.service` apply both, with a read-only `--check`.

What still reaches every isolated CPU: anything that calls
`lru_add_drain_all` (block-device ioctls, `systemctl daemon-reload` and
generators, `mlock`, page migration) or loads a BPF program (systemd
units, a container start with seccomp, an SSH login). The lab caught a
systemd daily timer doing both: a 40.60 µs max and 1 involuntary switch.

| rule | why |
|---|---|
| move systemd daily timers out of trading hours | generators' block-device ioctls and BPF loads |
| no pod starts or restarts on the node during trading | seccomp filters are BPF programs |
| no SSH logins to the node during trading | each login installs a seccomp filter |

## Did the isolation actually hold?

The checks above only prove the plugin ran. This proves an IRQ did not land on
a hot core. Method from [Tal-or/rps-debug](https://github.com/Tal-or/rps-debug):

1. On one exclusive CPU run a latency probe that records the worst gap
   (`oslat`, or a loop that prints its max).
2. On the **same** CPU run something that takes network interrupts — a socket
   reading traffic sent from another host.
3. Send the traffic and watch the probe.

A spike means an IRQ or a softirq reached the exclusive core: IRQ affinity or
RPS is wrong, or `sharedCpus` is not where the interrupts are handled. No spike
means the layout is doing its job. Run it with and without the plugin.

## Prior art

The idea is not new. Several projects do part of it; one does most of it.

| project | what it does | state |
|---|---|---|
| [CRI-O `shared_cpuset`](https://github.com/cri-o/cri-o) | adds a node-wide shared CPU set to a Guaranteed container's cpuset on top of its exclusive CPUs | shipped, production, **CRI-O only** |
| [openshift-kni/mixed-cpu-node-plugin](https://github.com/openshift-kni/mixed-cpu-node-plugin) | NRI plugin plus device plugin, "populates shared-cpus as a device" | **deprecated** — merged into CRI-O |
| [kubernetes-sigs/dra-driver-cpu](https://github.com/kubernetes-sigs/dra-driver-cpu) | exclusive CPUs per container, shared pool for everything else, over DRA + CDI + NRI | active (★67), but exclusive ∪ shared in one container is **not landed** |
| [containers/nri-plugins](https://github.com/containers/nri-plugins) topology-aware | exclusive and shared cores in one workload, plus IRQ, frequency and NUMA policy | active (★117), containerd NRI |
| [antaloala/nri-cpuset-partition](https://github.com/antaloala/nri-cpuset-partition) | makes a container's cgroup a cgroup v2 `cpuset` partition | new (Oct 2026); covers the `cpu-partition` opt-in only |
| kubelet CPU Manager `static` + `reservedSystemCPUs` | exclusive CPUs only | upstream; we build on it |

On the Kubernetes SIG driver the same feature is an open ask and a draft:

| item | state | title |
|---|---|---|
| [#103](https://github.com/kubernetes-sigs/dra-driver-cpu/issues/103) | open, Mar 2026 | *Support mixed shared and isolated CPU allocation with CPU DRA* |
| [#149](https://github.com/kubernetes-sigs/dra-driver-cpu/pull/149) | **draft, "Do Not Merge"** | *POC for Mixed CPU Allocation* — sets the affinity to "the union of their dedicated exclusive CPUs and the node's shared CPU pool" |
| [#305](https://github.com/kubernetes-sigs/dra-driver-cpu/issues/305) | open, Aug 2026 | *Mixed CPU allocation via composable exclusive + shared devices* |

So this is [issue #103](https://github.com/kubernetes-sigs/dra-driver-cpu/issues/103)
working today, on plain containerd, with no DRA feature gate and without waiting
on [KEP-5517](https://github.com/kubernetes/enhancements/issues/5517).

### Why use this one

| | CRI-O | dra-driver-cpu | **here** |
|---|---|---|---|
| exclusive ∪ shared in one container | yes | draft POC only | yes |
| plain containerd, no feature gate | no | needs DRA + KEP-5517 | **yes** |
| every new thread starts on the shared CPUs | no (exec affinity only) | no | **yes** (OCI `createRuntime`) |
| pin by index, never a real CPU number | no | no | **yes** (`pin_isolated`, `ergon-cpus`) |
| refuses a split SMT core, naming it | no | partial | **yes** |
| refuses a NUMA binding from unread topology | no | picks best-fit | **yes** |
| capacity the scheduler can see | no | DRA consumable capacity | **yes** (`ergon.io/shared-millicpus`) |

### Why not wait for DRA

`dra-driver-cpu` is the right long-term vehicle and will probably reach this
behaviour eventually. It is not there now: it needs DRA (Kubernetes 1.34+) and a
feature gate, the mixed-allocation POC is a non-mergeable draft waiting on
KEP-5517, and its model treats a guaranteed container as exclusive-only, so its
helper threads have nowhere to go but the exclusive set. When it lands, move.

### Deliberate differences

| elsewhere | here | why |
|---|---|---|
| CRI-O raises `cpu.cfs_quota_us` / `cpu.max` when it widens the cpuset | a container with a CPU quota is **refused** | a quota means the pod is not an exclusive Guaranteed integer-CPU pod; lifting the quota would hide that |
| CRI-O takes the housekeeping CPUs from the SMT siblings of the container's first CPU | `sharedCpus` is set by the operator, and a split core is refused | on a trading node the housekeeping set is a choice, not a side effect |
| dra-driver-cpu shrinks the shared pool as exclusives are allocated | `sharedCpus` is fixed and must sit inside `reservedSystemCPUs` | exclusives can never overlap it, so there is nothing to subtract |
| `nri-plugins` topology-aware also sets IRQ affinity, governors and C-states | not here | that is a node policy product. This only places threads |

## Develop

### Build and test

Nothing is installed on the host. Go and builds run in containers.

```sh
scripts/check.sh          # gofmt, go vet, go test -race, static builds, then the cgroup kernel lane
scripts/check.sh --local  # the unit lane on the host (CI)
scripts/check-cgroup.sh   # the cgroup kernel lane on its own
docker build -t ergon-shared-cpus:dev .
test/e2e.sh               # the 18-step kind e2e (Docker only, needs >= 6 CPUs)
```

- **cgroup kernel lane:** runs `test/cgroup` against a real cgroup v2
  hierarchy, in a privileged container with the host's cgroup namespace. The
  unit fixtures in `internal/hook` are ordinary files, so they accept writes
  a kernel would reject (a bare name in `cgroup.subtree_control` is EINVAL,
  for example). This lane proves the partition is one a real kernel will
  take a task in, and that one thread of a process can sit on the exclusive
  CPUs while the others stay on the shared ones.
- **e2e:** builds both images, creates a kind cluster, runs every check and
  deletes every container, cluster, image and volume it created, on success,
  failure and interrupt. It uses `ergon-shared-cpus:dev` and does not pull
  the public chart.

### Local manifests

The manifests in `deploy/` are the local build path. Their image is
`ergon-shared-cpus:dev`. Build it with `docker build -t ergon-shared-cpus:dev .`,
load it onto the node, edit `deploy/configmap.yaml`, then
`kubectl apply -f deploy/namespace.yaml -f deploy/configmap.yaml -f deploy/daemonset.yaml`.

The `:dev` DaemonSet needs eight host paths:

| host path | in the pod | mode |
|---|---|---|
| `/var/run/nri` | same | read-write |
| `/var/lib/kubelet/device-plugins` | same | read-write |
| `/opt/ergon/bin` | same | read-write |
| `/dev` | `/host/dev` | read-only |
| the host cgroup v2 tree | `/sys/fs/cgroup` | read-only |
| `/var/lib/kubelet/config.yaml` | `/host/kubelet-config` | read-only |
| `/sys` | `/host/sys` | read-only |
| `/proc` | `/host/proc` | read-only |

- The kubelet file is a `File` mount, not `FileOrCreate`: the pod does not
  start when that path is missing. The last three are the hot-pod check's
  inputs, passed as `--kubelet-config`, `--host-sys` and `--host-proc`, as
  the chart does.
- The namespace needs Pod Security `privileged`. It is root on the node:
  anything on the NRI socket can change any container.

### Trust

Only a marked container is touched: the device plugin's container
annotation `ergon.io/shared-millicpus` (pods cannot set container
annotations), or the pod annotation `cpu-shared.ergon.io/<container>` when
`annotationOptIn` is on. The marker also names the millicpu request.

The `ERGON_*` env is set by the plugin and is not a marker: a pod can set
env by hand. A hand-set `ERGON_SHARED_CPUS` is ignored; only the shared
CPUs the plugin itself recorded are ever re-widened, and only inside
`sharedCpus`. A container that is no longer exclusive integer-CPU (CPU
quota or wrong shares) loses its shared CPUs.

### Contract

The resource name, the annotation keys and the env names are the contract.
Changes go in the repo `CHANGELOG.md`. `--version` prints the version, the
git describe string and the NRI module version.

Apache-2.0. The repo `LICENSE` applies.
