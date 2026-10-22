# ergon-shared-cpus

A containerd NRI plugin and a kubelet device plugin in one daemon, plus a
small tool. Together they give a Kubernetes pod its exclusive CPUs **and**
the node's shared housekeeping CPUs, and start every thread on the shared
ones. The busy-spin loop keeps its exclusive core; only sleepy helpers
(dial, TLS, warnings, Aeron conductors) run on the shared CPUs.

Source: <https://github.com/mimran1980/ergon/tree/main/k8s/nri-shared-cpus>.

**Published, tag 0.1.1.** Anyone can pull
`ghcr.io/mimran1980/ergon-shared-cpus:0.1.1` (and `:latest`) and the chart
`oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.1`.
Tag `0.1.0` remains. The EKS lane and the latency benchmarks are still open.
[`RELEASE.md`](RELEASE.md) names what is not verified. The kind sample below
ran on chart `0.1.0`, not on EKS.

**This is not a new idea.** [Prior art](#prior-art) names the projects that
already do parts of it and the open Kubernetes issue it answers. Read that
before you choose between the options. What is here is the containerd path
that needs no DRA feature gate, plus the pieces those leave out.

## What the app sees

A Guaranteed pod asks for integer CPUs and for
`ergon.io/shared-millicpus` (millicpus, because extended resources are
whole numbers and requests must equal limits):

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

Inside the container:

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

- never run a spinner or SQPOLL on a shared CPU;
- the createRuntime hook pins the threads that exist at create onto the
  shared CPUs. A thread created later inherits its creator's mask. A helper
  created by a pinned hot thread inherits the hot mask. The application can
  change affinity with `sched_setaffinity`. That inheritance was observed for
  a C `pthread_create` (`test/e2e/pin-test.c` mode `masks`) on the Docker
  Desktop linuxkit kernel, which has no `isolcpus`. A Go 1.27 OS thread
  started from the hot-pinned thread kept the shared CPU `PinProcess` had set;
  it did not inherit the hot mask. JVM and Aeron were not run;
- create helpers from a thread that is still on the shared set;
- each sleepy helper checks `sched_getcpu()` when it wakes and alarms if it
  is not on `ERGON_SHARED_CPUS`.

The configured pool is fixed. A container does not receive all of it unless
it asks for it: the allocator grants `ceil(M/1000)` CPUs, preferring the
assignment it recorded for that pod and otherwise the least-loaded CPUs.
For one trading process, request `M = 1000 × |S|` with `oversubscription: 1`.
That uses the existing interface and grants the whole helper set. A smaller
`M` still grants `ceil(M/1000)` CPUs, which is scheduler accounting, not a
fraction of a core. A reconnect rebuilds the table from the environment
recorded at create. It does not choose replacement helper CPUs for a
container that is already running. Drain trading before changing the
reservation, the profile or the shared list.

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

## Build and test

Nothing is installed on the host. Go and builds run in containers.

```sh
scripts/check.sh          # gofmt, go vet, go test -race, static builds, then the cgroup kernel lane
scripts/check.sh --local  # the unit lane on the host (CI)
scripts/check-cgroup.sh   # the cgroup kernel lane on its own
docker build -t ergon-shared-cpus:dev .
test/e2e.sh               # the 18-step kind e2e (Docker only, needs >= 6 CPUs)
```

The cgroup kernel lane runs `test/cgroup` against a real cgroup v2
hierarchy, in a privileged container with the host's cgroup namespace. The
unit fixtures in `internal/hook` are ordinary files, so they accept writes a
kernel would reject — a bare name in `cgroup.subtree_control` is EINVAL, for
example. This lane is what proves the partition is one a real kernel will
take a task in, and that one thread of a process can sit on the exclusive
CPUs while the others stay on the shared ones.

The e2e builds both images, creates a kind cluster, runs every check and
deletes every container, cluster, image and volume it created, on success,
failure and interrupt.

## Install from GHCR

Anyone can pull these. No image pull secret.

| what | address |
|---|---|
| image | `ghcr.io/mimran1980/ergon-shared-cpus:0.1.1` (and `:latest`) |
| digest of `0.1.1` and `latest` | `sha256:69893e0c23ec6c3656053c0d6c4a06c676e33b726d56f362a30ce760908796c0` |
| chart | `oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.1` |
| chart digest | `sha256:77a65305a45ea1b376ce80f9b1d5234b9061f4066a8c5baf5efdb892854fc0d8` |

```sh
helm install ergon-shared-cpus oci://ghcr.io/mimran1980/charts/ergon-shared-cpus \
  --version 0.1.1 \
  --namespace ergon-system --create-namespace \
  --set-string config.sharedCpus=0 \
  --set 'config.namespaces={trading}'
```

`config.sharedCpus` must be inside this node's `reservedSystemCPUs`, minus
the IRQ CPU. Label each node `ergon.io/shared-cpus=true` before the
DaemonSet can schedule. The chart does not set the boot line, the kubelet,
or containerd.

Node requirements: Kubernetes >= 1.33, `cpuManagerPolicy: static`,
`reservedSystemCPUs` = the IRQ CPU(s) + the shared CPUs, kernel >= 6.2,
cgroup v2, containerd >= 2.2 with the NRI default validator on. A
`cpu-partition` opt-in also needs `cpuset.cpus.exclusive`, which is Linux
6.7 or a vendor kernel that backported it. The plugin reads the file rather
than the version.

Chart 0.1.0 writes `sharedCpus`, `oversubscription`, `namespaces`,
`defaultAffinity`, `hookPath`, `annotationOptIn`, `metricsAddress` and
`devRoot`. It does not write `seccomp`, `ptp` or `schedFifo`, so `--set`
of those keys does nothing on this package and the plugin leaves them off.
`helm install` of this package prints "This chart is NOT released". That
note was packed before the push. The tag is the public chart.

`docker buildx imagetools inspect ghcr.io/mimran1980/ergon-shared-cpus:0.1.0`
prints digest `sha256:d28fea2f95ebce3e53d33f8770f3ba69be75c76ec7437ea2b92c1329759e07e9`.
The published chart image is `repository:tag`. It cannot take `@sha256`.
Chart digest `sha256:7347131d7d87caee5cf6de11b2a11e2b62454da59c41092b3805bc2834795dbd`.

Chart 0.1.1 is the package above. `helm template` of that OCI chart with
`--set-string config.sharedCpus=0` renders image
`ghcr.io/mimran1980/ergon-shared-cpus:0.1.1` and args `--config` only.
It renders every `config.File` field (`seccomp.ioUring`, `ptp` and
`schedFifo` stay quoted `"off"`). `image.digest` renders
`repository@digest`; empty keeps `repository:tag`. `admission.create: false`
(the default) renders a ConfigMap with `data.owner: external` and does not
create the policy or the PriorityClasses. `admission.create: true` installs
`validating-admission-policy.yaml` (policy and binding) and
`priority-class.yaml` (`ergon-hot` and `ergon-helper`). A ConfigMap edit
does not roll the DaemonSet. Drain trading workloads, then set
`layoutRevision`, before changing `sharedCpus`. `cgroupRoot` is not a
config field. Host-fact mounts stay off. Image `0.1.1` `--help` on
linux/amd64 and linux/arm64 lists `--kubelet-config`, `--kubelet-configz`,
`--kubelet-overrides`, `--host-sys` and `--host-proc`. The chart does not
pass them. Image `0.1.0` has none of those flags. `helm install` of 0.1.1
prints a NOTES text that still says the chart is not published and that the
OCI chart is 0.1.0. That text was packed with the push. The tag is not
republished to change it. `--version` printed
`ergon-shared-cpus 0.1.1 515846b0201b5fd70a7a6cf3a88839b6180fa9c1 nri=v0.12.3`.

Image `0.1.0` rejects `nodeProfileLabel` and exits with
`unknown field "nodeProfileLabel"`. Do not set `config.profiles` or
`config.nodeProfileLabel` on that image or on chart 0.1.0. Chart 0.1.1 can
write those keys. The default render is the flat `sharedCpus` file. The
process picks one named configuration from `config.nodeProfileLabel` on the
node (`NODE_NAME`, a get-only Node role).
A missing label, an unknown profile, or an empty `NODE_NAME` stops that
process. It does not watch the node and it does not change labels. Legacy
`sharedCpus` stays the default. A file that sets both is rejected. Drain
before changing a label or a profile. Running grants stay until those
containers are recreated. A multi-node cluster was not started for this
selection.

### Steps on a cluster you run

1. Prepare the node (the chart does not): Kubernetes >= 1.33, cgroup v2,
   kernel >= 6.2, containerd >= 2.2 with the NRI block below,
   `cpuManagerPolicy: static`, `reservedSystemCPUs` = the IRQ CPU(s) plus
   the shared CPUs, boot line isolating the exclusive CPUs only, label
   `ergon.io/shared-cpus=true`.
2. `helm install` as above. `config.sharedCpus` must be inside
   `reservedSystemCPUs` and must not be an IRQ CPU.
3. Apply `k8s/nri-latency-pods/admission/priority-class.yaml` and
   `validating-admission-policy.yaml`. Skip `admission/examples/` and the
   mutating policy. That policy is the HFT admission check: a pod labelled
   `ergon.io/latency: hot` is denied for an exec startup, readiness or
   liveness probe, a lifecycle exec handler, an always-restart sidecar, a
   shared-millicpu request missing from any application container, a
   `required-plugins` annotation that is not the exact plugin token, a
   fractional CPU, a missing `resizePolicy: RestartContainer` for `cpu`, or
   a missing priority class. Admission does not control a later `kubectl exec`.
4. Run a Guaranteed pod in an allowlisted namespace (`deploy/example-pod.yaml`
   is the shape). Check `ERGON_ISOLATED_CPUS`, `ERGON_SHARED_CPUS`, and
   PID 1 `Cpus_allowed_list` (shared CPUs only).

Kubernetes HFT tuning is that node setup plus this plugin plus the
admission policy. It is not another image. The book chapter is
<https://mimran1980.github.io/ergon/deploy/hft.html>. Hugepages, an
isolated cpuset partition, io_uring seccomp and the NIC PHC are opt-in
grants in this image. Chart 0.1.1 writes `seccomp`, `ptp` and `schedFifo`
as `"off"`. Chart 0.1.0 does not write those keys.
EKS is not verified. Latency figures are not measured.

Flux: `HelmRepository` type `oci`, url `oci://ghcr.io/mimran1980/charts`,
chart `ergon-shared-cpus` version `0.1.1`. Copy the two admission files
into the repo Flux applies. Do not point a Kustomization at all of
`admission/`.

### Sample

[examples/try](examples/try) is a throwaway kind cluster that pulls this
chart and runs the hot pod and the admission denial:

```sh
k8s/nri-shared-cpus/examples/try/run.sh
```

Needs Docker (at least 6 CPUs), helm, kubectl and kind v0.33.0. It writes
its kubeconfig under `/tmp` and deletes the cluster after this run creates
it. A preflight refusal, including a cluster that already has that name,
does not delete it. The pod log
must show `ERGON_SHARED_CPUS=0`, a non-empty `ERGON_ISOLATED_CPUS`, PID 1
`Cpus_allowed_list` of `0`, and `nproc=1`. The exec-probe pod must be
denied. The sample does not measure latency and does not ask for
hugepages, PTP, or an isolated cpuset partition.

On 2026-10-22 that script passed on chart `0.1.0` and
`kindest/node:v1.33.0` (Docker Desktop, linux aarch64, 10 CPUs):
`ERGON_ISOLATED_CPUS=2,3`, `ERGON_SHARED_CPUS=0`,
`cpuset.cpus.effective=0,2-3`, `nproc=1`, allocatable
`ergon.io/shared-millicpus=1k`, and `ValidatingAdmissionPolicy`
`ergon-latency-pods` denied the exec probe. The lines are in
[examples/try/README.md](examples/try/README.md).

The manifests in `deploy/` are the local build path. Their image is
`ergon-shared-cpus:dev`. Build it with `docker build -t ergon-shared-cpus:dev .`,
load it onto the node, edit `deploy/configmap.yaml`, then
`kubectl apply -f deploy/namespace.yaml -f deploy/configmap.yaml -f deploy/daemonset.yaml`.
`test/e2e.sh` uses that image. It does not pull the public chart.

containerd:

```toml
[plugins."io.containerd.nri.v1.nri".default_validator]
  enable = true
  # leave required_plugins empty: a node-wide list would also block
  # kube-system pods and this plugin's own pod while it is down
```

The `:dev` DaemonSet needs six host paths: `/var/run/nri`,
`/var/lib/kubelet/device-plugins` and `/opt/ergon/bin` read-write, plus
`/dev` at `/host/dev`, the host cgroup v2 tree at `/sys/fs/cgroup`, and
`/var/lib/kubelet/config.yaml` at `/host/kubelet-config`, the last three
read-only. The kubelet file is a `File` mount, not `FileOrCreate`: the pod
does not start when that path is missing. The `:dev` image passes it as
`--kubelet-config`. Chart 0.1.1 leaves `hostContract` off, so its
DaemonSet does not pass that flag. Image 0.1.1 accepts it. Image 0.1.0
rejects it. The namespace needs Pod Security `privileged`. It is
root on the node: anything on the NRI socket can change any container.

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

## Verify

```sh
kubectl -n ergon-system get pods -l app=ergon-shared-cpus   # Ready
kubectl get node <node> -o jsonpath='{.status.allocatable.ergon\.io/shared-millicpus}'
curl -s <pod-ip>:9466/readyz
curl -s <pod-ip>:9466/metrics
```

Then run a pod with the example above and check
`cpuset.cpus.effective` and the `ERGON_*` variables.

### Did the isolation actually hold?

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
  `sched_getaffinity`) now see only the shared CPUs. Read
  `ERGON_ISOLATED_CPUS` or `cpuset.cpus.effective` instead.
- In-place CPU resize leaves the CPU env stale. Use
  `resizePolicy: RestartContainer` for `cpu`.
- Not verified on a real EKS node: the NRI socket, nodeadm TOML merge,
  `/opt` writable, `system-node-critical` outside kube-system.
- Not measured: latency of helpers on shared CPUs, and of the exclusive
  cores with the plugin on.

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
| CRI-O raises `cpu.cfs_quota_us` / `cpu.max` when it widens the cpuset | a container with a CPU quota is **refused** (rule 3) | a quota means the pod is not an exclusive Guaranteed integer-CPU pod; lifting the quota would hide that |
| CRI-O takes the housekeeping CPUs from the SMT siblings of the container's first CPU | `sharedCpus` is set by the operator, and a split core is refused | on a trading node the housekeeping set is a choice, not a side effect |
| dra-driver-cpu shrinks the shared pool as exclusives are allocated | `sharedCpus` is fixed and must sit inside `reservedSystemCPUs` (rule 5) | exclusives can never overlap it, so there is nothing to subtract |
| `nri-plugins` topology-aware also sets IRQ affinity, governors and C-states | not here | that is a node policy product. This only places threads |

## Trust

Only a marked container is touched: the device plugin's container
annotation `ergon.io/shared-millicpus` (pods cannot set container
annotations), or the pod annotation `cpu-shared.ergon.io/<container>` when
`annotationOptIn` is on. The marker also names the millicpu request.

The `ERGON_*` env is set by the plugin and is not a marker: a pod can set
env by hand. A hand-set `ERGON_SHARED_CPUS` is ignored; only the shared
CPUs the plugin itself recorded are ever re-widened, and only inside
`sharedCpus`. A container that is no longer exclusive integer-CPU (CPU
quota or wrong shares) loses its shared CPUs.

## Contract

The resource name, the annotation keys and the env names are the contract.
Changes go in the repo `CHANGELOG.md`. `--version` prints the version, the
git describe string and the NRI module version.

Apache-2.0. The repo `LICENSE` applies.
