# Shared CPUs for Helper Threads (`k8s/nri-shared-cpus`)

How to install and use `k8s/nri-shared-cpus` on a Kubernetes node.

A busy-spinning pod gets whole cores of its own from the kubelet. Its
helpers (dial, TLS, warnings, Aeron conductors) do not need a whole core,
and running them on the loop's core is what this layout forbids (its
latency cost is **not measured**). This is a containerd
NRI plugin and a kubelet device plugin that give the pod the node's shared
housekeeping CPUs as well, and start every thread there.

The busy-spin loop keeps its exclusive core. Only sleepy helpers go to the
shared CPUs. Never a spinner or SQPOLL there.

Source: `k8s/nri-shared-cpus/`.

Status: published as tag `0.1.1`. Anyone can pull the image and the chart.
Not yet run on kind; **not verified on a real EKS node** (see
[Limits](#limits-and-what-is-not-verified)). The latency numbers are not
measured. AKS cannot run it.
Prior art: `k8s/nri-shared-cpus/README.md`.

## Use the public chart

Do these in order. The chart installs the DaemonSet. It does not prepare
the node, and it does not install the hot-pod admission policy.

1. Prepare the node. [Node setup](#node-setup) below. Label it
   `ergon.io/shared-cpus=true`.
2. Install. `config.sharedCpus` is the housekeeping CPU, inside
   `reservedSystemCPUs`, and not the IRQ CPU. For the 8-CPU split in that
   section the value is `0`. One Helm release per shared-CPU layout. The
   three `hostContract` flags mount the node's kubelet config, `/sys` and
   `/proc` read-only; the plugin checks every pod labelled
   `ergon.io/latency: hot` against them ([Node setup](#node-setup)).
   Chart 0.1.1 needs them.

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

**EKS AL2023** (needs chart 0.1.2, not yet published; not verified on
EKS). nodeadm writes the base `/etc/kubernetes/kubelet/config.json`, puts
the `NodeConfig` kubelet settings in the drop-in
`/etc/kubernetes/kubelet/config.json.d/40-nodeadm.conf`, and runs the
kubelet with `--config-dir` on that directory. `kubeletConfigDir` mounts
it read-only and the hot-pod check merges its `*.conf` files as the
kubelet does (lexical order, JSON merge patch):

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

Chart 0.1.1 rejects `kubeletConfigDir`; without it every hot pod is
refused on AL2023. The drop-in directory exists only when `NodeConfig`
sets `spec.kubelet.config`; without it the plugin pod does not start.

Check the pulled image. This prints the index digest
`sha256:69893e0c23ec6c3656053c0d6c4a06c676e33b726d56f362a30ce760908796c0`:

```sh
docker buildx imagetools inspect ghcr.io/mimran1980/ergon-shared-cpus:0.1.1
```

Chart digest
`sha256:77a65305a45ea1b376ce80f9b1d5234b9061f4066a8c5baf5efdb892854fc0d8`.
`image.digest` renders `repository@digest`; an empty digest keeps
`repository:tag`, and the default tag is image `0.1.1`.

The chart renders every field `config.File` accepts, with
`seccomp.ioUring`, `ptp` and `schedFifo` quoted `"off"`. The plugin leaves
them off. A later edit of the ConfigMap can turn a key on, and the pod
still has to opt in. Aeron's C media driver does not use io_uring.
`SCHED_FIFO` stays off.

`admission.create` defaults to false and renders a ConfigMap whose
`data.owner` is `external`. Set it true to install the two named admission
files (the policy, its binding, and the `ergon-hot` and `ergon-helper`
PriorityClasses). A ConfigMap edit does not roll the DaemonSet. Drain
first, then set `layoutRevision`.

`helm install` prints NOTES that say the chart is not published. It is
published; ignore that line.

3. Apply the hot-pod policy. These two files only, from
   `k8s/nri-latency-pods/admission/`:

```sh
kubectl apply -f priority-class.yaml -f validating-admission-policy.yaml
```

Leave `examples/` and `mutating-admission-policy.yaml` unapplied.
A pod labelled `ergon.io/latency: hot` is refused when it has an exec
probe, a container with `restartPolicy: Always`, no
`ergon.io/shared-millicpus` request, no
`required-plugins.noderesource.dev` annotation naming `ergon-shared-cpus`,
or no `priorityClassName`.

4. Run a Guaranteed pod in an allowlisted namespace. Whole-number `cpu`,
   requests equal limits, `ergon.io/shared-millicpus`. The pod spec is
   under [Pod](#pod). Inside it, `ERGON_SHARED_CPUS` is the housekeeping
   list, `ERGON_ISOLATED_CPUS` is the exclusive list, PID 1
   `Cpus_allowed_list` is the shared CPUs only, and
   `cpuset.cpus.effective` is the union.

Flux: a `HelmRepository` of type `oci` at
`oci://ghcr.io/mimran1980/charts`, chart `ergon-shared-cpus` version
`0.1.1`, `targetNamespace: ergon-system`, and the same `config` and
`hostContract` values.
Copy the two admission files into the Git repo Flux already applies.
Do not point Flux at the whole `admission/` directory: that directory
also holds pods the policy is meant to refuse.

### See it on kind

`k8s/nri-shared-cpus/examples/try/run.sh` creates a throwaway kind cluster
(`kindest/node:v1.33.0`, static CPU manager, `reservedSystemCPUs: "0,1"`),
installs this public chart with `sharedCpus: "0"` and the three
`hostContract` flags, and applies the two admission files. A kind node
shares the host kernel and has no `isolcpus`, so no hot pod can start
there.

| pod | expected |
|---|---|
| unlabelled, `cpu: "2"`, `ergon.io/shared-millicpus: "250"` | Ready. Log: `ERGON_SHARED_CPUS=0`, non-empty `ERGON_ISOLATED_CPUS`, PID 1 `Cpus_allowed_list` of `0`, `nproc=1` |
| the same, labelled `ergon.io/latency: hot` | admitted, then refused by the plugin: `exclusive CPU not isolated` |
| the hot pod with an exec liveness probe | denied by the admission policy |

The script deletes the cluster on the way out and uses a kubeconfig under
`/tmp`.

That run shows the shared-CPU grant and the HFT admission check. It does
not measure latency. Kind on Docker Desktop has no hugepage pool, no NIC
PHC and no `cpuset.cpus.exclusive` partition to demonstrate, and this
sample does not ask for those grants.

Not yet run on kind.

The 18-step `test/e2e.sh` is a different run. It builds
`ergon-shared-cpus:dev` from this tree. It does not pull the public chart.

## What it gives

| item | value |
|---|---|
| exclusive CPUs | the kubelet's static CPU manager, `cpu: N` |
| shared CPUs | `ergon.io/shared-millicpus: M` adds `ceil(M/1000)` shared CPUs |
| start affinity | the hook pins threads that exist at create onto the shared CPUs. A later thread inherits its creator's mask. A helper created by a pinned hot thread inherits the hot mask. The application can change affinity. A C `pthread_create` did that on linuxkit with no `isolcpus`. A Go 1.27 OS thread started from the hot thread stayed on the shared CPU. JVM and Aeron were not run |
| timer slack | 1 ns on every thread, inherited by every thread and child the program makes |
| pinning | the app pins hot threads by index into `ERGON_ISOLATED_CPUS` |

`M` is millicpus because extended resources are whole numbers and requests
must equal limits. `250` is a quarter core of **scheduler accounting only**:
the plugin still adds one whole shared CPU, and there is no hard cap.

### Timer slack

The kernel's default timer slack is 50,000 ns. It is added to every sleep and
to every `epoll`/`io_uring` timeout. The hook sets 1 ns, the kernel's floor,
on every thread the container has when it starts; the kernel copies it to
every thread and child after that. A container that cannot have it does not
start. The OCI hook runs as root, so it can write
`/proc/<tid>/timerslack_ns`; a program can always set its own slack with
`prctl(PR_SET_TIMERSLACK)`.

What the 1 ns slack is worth in latency is **not measured**. The loop
spins and never sleeps; the slack applies to sleeping helpers such as
Aeron's conductor.

## Node setup

Do this before installing the plugin. Everything here is per node. The rest
of the node checklist is [Latency Pods: node setup](latency-pods.md#node-setup).

| setting | value |
|---|---|
| Kubernetes | >= 1.33: no CPU quota on exclusive containers (the labs ran 1.36) |
| kubelet | `cpuManagerPolicy: static`; x86 `full-pcpus-only`; `reservedSystemCPUs` = the IRQ CPU(s) + the shared CPUs; `cpuCFSQuota` not `false`; feature gate `DisableCPUQuotaWithExclusiveCPUs` not `false` |
| kubelet config file | `/var/lib/kubelet/config.yaml` (kubeadm, kind). The plugin pod does not start without it. Another path: `--set hostContract.kubeletConfig.path=<file>`. A kubelet run with `--config-dir` also needs `hostContract.kubeletConfigDir` (chart 0.1.2); EKS AL2023: [Use the public chart](#use-the-public-chart) |
| kernel | >= 6.2, cgroup v2. Boot line: `isolcpus=managed_irq,domain`, `nohz_full` and `rcu_nocbs` on every CPU outside `reservedSystemCPUs`, never on the shared ones. A `cpu-partition` opt-in also needs `cpuset.cpus.exclusive`: Linux 6.7, or a vendor kernel that backported that file. The plugin reads the file, not the version |
| NIC IRQs | every hardware NIC IRQ, requested and effective affinity, on the IRQ CPU(s): `reservedSystemCPUs` minus `sharedCpus` |
| containerd | >= 2.2, NRI on, default validator on |
| node label | `ergon.io/shared-cpus=true` |

A pod labelled `ergon.io/latency: hot` is checked against the kubelet,
kernel and NIC IRQ rows at every create: its exclusive CPUs must be in
`/sys/devices/system/cpu/isolated`, the shared CPUs online and outside
`isolated` and `nohz_full`. A miss refuses that create
(`CreateContainerError`, naming the rule). Unlabelled pods get the same
grants without the check.

**CPU split.** Pick three CPU sets on the node and write them down; every
later step names one of them.

| set | who | example (8 CPUs) |
|---|---|---|
| exclusive | the pod's busy-spin loops | `2,3` (and `4,5` for a second pod) |
| shared | helpers, kubelet, containerd, system | `0` |
| IRQ | NIC interrupts | `1` |

**Kernel boot line.** Isolate the exclusive CPUs only. Never the shared one.

```
nohz_full=2-5 rcu_nocbs=2-5 isolcpus=managed_irq,domain,2-5
```

**kubelet** — `reservedSystemCPUs: "0,1"` (shared + IRQ) with the policy
above. Restart kubelet and check it took.

**containerd** — NRI on (`disable = false`) and the default validator on.
The validator is what makes an unmet `required-plugins` annotation fail the
create instead of starting the pod half-granted. Leave
`required_plugins` empty: a node-wide list would also block kube-system
pods and the plugin's own pod while it is down.

## Install on Kubernetes

The plugin is public. No image pull secret.

| what | address |
|---|---|
| image | `ghcr.io/mimran1980/ergon-shared-cpus:0.1.1` (and `:latest`) |
| digest of `0.1.1` and `latest` | `sha256:69893e0c23ec6c3656053c0d6c4a06c676e33b726d56f362a30ce760908796c0` |
| chart | `oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version `0.1.1` |

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

`config.sharedCpus` is this node's shared CPU. For the 8-CPU split above
that is `0`: inside `reservedSystemCPUs`, and not the IRQ CPU. The chart
creates `ergon-system` with the privileged Pod Security labels and selects
nodes labelled `ergon.io/shared-cpus=true`. It does not label nodes, and it
does not set the boot line, the kubelet, or containerd.

Do the node setup above first, then the steps below. The public install is
[Use the public chart](#use-the-public-chart). Step 4 below is the local image
`ergon-shared-cpus:dev`. The kind sample that pulls the public chart is
[See it on kind](#see-it-on-kind).

### 1. Label the nodes that take the plugin

```sh
kubectl label node <node> ergon.io/shared-cpus=true
```

Only labelled nodes run the DaemonSet. A pod that asks for the device on an
unlabelled node stays pending.

### 2. Edit `deploy/configmap.yaml`

This is the step people get wrong. The file is
`k8s/nri-shared-cpus/deploy/configmap.yaml`, and it becomes
`/etc/ergon-shared-cpus/config.yaml` in the plugin's container.

```yaml
sharedCpus: "0"          # the node's shared CPUs, kernel cpulist
oversubscription: 1
namespaces: ["trading"]  # who may opt in. Empty refuses every pod
defaultAffinity: shared
annotationOptIn: false
devRoot: /host/dev       # where the DaemonSet mounts the host's /dev; without
                         # it the plugin finds no PHC and refuses ptp grants

# These three are "off" by default. A latency node turns the first two on.
# Each one is a node-level permission: the pod still opts in by annotation,
# and a pod that asks for a grant this field does not allow is refused.
seccomp:
  ioUring: "allow"       # default "off". Without it, a pod asking for
                         # seccomp.ergon.io/<ctr>: "io-uring" is refused.
                         # For apps that use io_uring (ergon-io's uring
                         # backends). Aeron's C media driver does not use it
ptp: "auto"              # default "off". Without it, a pod asking for
                         # ptp.ergon.io/<ctr>: "auto" is refused. "auto" is
                         # the NIC's PHC, or give a /dev/ptp* path the node has
schedFifo: "off"         # off by default; its one A/B was mixed (see
                         # Kubernetes HFT tuning, SCHED_FIFO)
```

Rules:

- `sharedCpus` must sit inside the kubelet's `reservedSystemCPUs`, minus the
  IRQ CPU(s). In the table above that is `0`.
- `namespaces` is an allowlist. Create those namespaces before applying.
- Quote `off`, `on`, `yes`, `no`. YAML 1.1 turns unquoted ones into booleans.
- An unknown field or an out-of-range value stops the plugin at start, and
  the message names the field.
- **`off` is not harmless.** It is a refusal. A pod whose annotation asks for
  a grant the ConfigMap does not allow does not start: the kubelet shows
  `CreateContainerError` and the text names the field to change.

### 3. Install the public chart

Use the `helm install` at the top of this section. One Helm release per
shared-CPU layout: the chart has one ConfigMap.

The plugin is root on the node and installs an executable at
`/opt/ergon/bin/ergon-cpus`. Only this repository's publish workflow pushes.

Kind, or a node with no registry: `docker build -t ergon-shared-cpus:dev .`
from `k8s/nri-shared-cpus`, load that image, and apply the three manifests
in the next step. `deploy/daemonset.yaml` ships
`image: ergon-shared-cpus:dev` with `imagePullPolicy: IfNotPresent`.

### 4. Apply the three manifests

Skip this when the Helm release above is installed. It is the kind path.

```sh
kubectl apply -f deploy/namespace.yaml \
              -f deploy/configmap.yaml \
              -f deploy/daemonset.yaml
```

The `:dev` DaemonSet needs eight host paths: `/var/run/nri`,
`/var/lib/kubelet/device-plugins` and `/opt/ergon/bin` read-write, plus
`/dev` at `/host/dev`, the host cgroup v2 tree at `/sys/fs/cgroup`,
`/var/lib/kubelet/config.yaml` at `/host/kubelet-config`, `/sys` at
`/host/sys` and `/proc` at `/host/proc`, the last five read-only. The
kubelet file is a `File` mount: the pod does not start when that path is
missing. The namespace is labelled Pod Security `privileged` for that
reason. The plugin is root on the node: anything on the NRI socket can
change any container.

### 5. Verify the plugin before any workload

The pod is Ready on every labelled node, `allocatable` shows
`ergon.io/shared-millicpus` greater than zero, `/readyz` is healthy, and the
logs show no config error (strict YAML: a bad field stops it). If
`allocatable` is empty the device plugin did not register: check the logs
and that `/var/lib/kubelet/device-plugins` is mounted.

### 6. Run a pod and check the grants landed

Use the pod under [Pod](#pod). Inside it:

```sh
kubectl exec trader -- sh -c 'echo isolated=$ERGON_ISOLATED_CPUS shared=$ERGON_SHARED_CPUS'
kubectl exec trader -- cat /proc/1/status | grep Cpus_allowed_list
```

`Cpus_allowed_list` must print the shared CPUs only. That is the point of
the plugin.

## Config

`/etc/ergon-shared-cpus/config.yaml`, from the ConfigMap. Strict YAML: an
unknown field is an error at start. No hot reload. A ConfigMap edit does
not roll the DaemonSet. Drain trading workloads, then set `layoutRevision`
(or restart the pods), before a new CPU layout is used. Running grants
stay as they were at create.

| field | default | what it is |
|---|---|---|
| `sharedCpus` | **required** | the housekeeping CPUs, kernel cpulist (`"4"` or `"0,2"`). Inside `reservedSystemCPUs`, never the IRQ CPU(s) |
| `oversubscription` | `1` | 1–100. Capacity is `len(sharedCpus) x 1000 x` this; the max is 100,000 device IDs (the kubelet's 4 MiB gRPC limit) |
| `namespaces` | empty | the allowlist. Empty refuses every opt-in. Each must be a valid namespace name |
| `defaultAffinity` | `shared` | `shared` injects the hook that starts every thread on the shared CPUs. `all` injects nothing; use the `ergon-cpus` wrapper (see Bottlerocket below) |
| `hookPath` | `/opt/ergon/bin/ergon-cpus` | where the daemon writes the hook on the node. Must be a clean absolute path ending in `/ergon-cpus` — the daemon writes it as root, so the name is fixed |
| `annotationOptIn` | `false` | allow the pod annotation `cpu-shared.ergon.io/<container>` as a fallback when the device plugin is not used |
| `metricsAddress` | `:9466` | serves `/metrics`, `/healthz` and `/readyz` |
| `rlimits.memlock`, `.nofile`, `.rtPrio`, `.nice` | unset | the ceiling a pod may ask for by kind. `"inf"` or a positive count. The pod asks for kinds, never values |
| `seccomp.ioUring` (nested: `seccomp:` then `ioUring:`) | `off` | `allow` widens the container's seccomp profile for `io_uring_*`. See [Latency Pods](latency-pods.md) |
| `seccompProfilePath` | `/etc/ergon-shared-cpus/ergon-io-uring.json` | the shipped io_uring profile on the node, the fallback for a pod that cannot take the NRI grant |
| `devRoot` | `/dev` | where this process sees the host's `/dev`. The DaemonSet mounts it read-only at `/host/dev` so the plugin can stat the PHC and PM QoS nodes; granted paths stay `/dev/...` |
| `schedFifo` | `off` | `off`, or `thread` (the hot thread only, never the container). Quote the value: unquoted `off` is a YAML boolean |
| `ptp` | `off` | `off`, `auto`, or a `/dev/ptp*` path the node has. `auto` grants the NIC's PHC |
| `hugepagesAeronDir` | unset | absolute `AERON_DIR` to export when the pod mounts a sized hugetlbfs |

`cgroupRoot` is not a field. The parser rejects it. The binary uses
`/sys/fs/cgroup`, and the DaemonSet mounts the host cgroup tree read-only
at that path. The createRuntime hook matches the same path against
`/proc/<pid>/cgroup`.

The chart can also write `profiles` and `nodeProfileLabel` instead of
the flat fields. At startup the binary reads its node once and uses the
label's value as the profile name. A missing or unknown label stops the
process. It does not watch the node and it does not change the label.
Setting `sharedCpus` and `profiles` in the same file is rejected. Drain
before changing a label or a profile. Containers that are already running
keep the helper CPUs recorded at create. This selection was not run on a
multi-node cluster.

Quote `off`, `on`, `yes` and `no` in this file. YAML 1.1 turns unquoted ones
into booleans.

Unknown fields and out-of-range values are refused at start, each naming its
field: `oversubscription` outside 1–100, a `namespaces` entry that is not a
namespace name, `hookPath` not ending in `/ergon-cpus`, an `rlimits` value
that is neither `inf` nor a positive count, `schedFifo` / `ptp` /
`seccomp.ioUring` outside their sets.

## Pod

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: trader
  namespace: trading                       # must be in the plugin's allowlist
  annotations:
    required-plugins.noderesource.dev: '["ergon-shared-cpus"]'
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
| `cpuset.cpus.effective` | the exclusive CPUs plus the shared ones |
| affinity of PID 1 and of every new thread | the shared CPUs only |
| `nproc`, `GOMAXPROCS`, `availableProcessors()` | usually the shared CPUs: they count the affinity mask |

## Env vars

| name | value | format |
|---|---|---|
| `ERGON_ISOLATED_CPUS` | the exclusive CPUs | sorted comma list, no ranges: `1,5` |
| `ERGON_SHARED_CPUS` | this container's shared CPUs | same |
| `ERGON_ISOLATED_CORES` | the exclusive CPUs by physical core | groups by `;`, CPUs by `,`: `1,5;2,6` |
| `ERGON_SHARED_MILLICPUS` | `M` | decimal |
| `ERGON_HELPER_CPUS` | same as `ERGON_SHARED_CPUS` | comma list |
| `OPENSHIFT_ISOLATED_CPUS` | same as `ERGON_ISOLATED_CPUS` | kernel cpulist with ranges |
| `OPENSHIFT_SHARED_CPUS` | same as `ERGON_SHARED_CPUS` | same |

Read the CPUs from these variables or from `cpuset.cpus.effective`, never
from the process's own affinity: the start mask is the shared CPUs.

## Pinning by index

Pin each hot thread to isolated index `i`. The app never needs the real CPU
numbers.

```c
{{#include ../../../k8s/nri-shared-cpus/examples/pin.c}}
```

Rules:

- leave helpers alone: they start on the shared CPUs;
- create helpers from a thread that is still on the shared set (a new
  thread copies its creator's mask);
- each sleepy helper checks `sched_getcpu()` when it wakes and alarms if it
  is not on `ERGON_SHARED_CPUS`;
- never a spinner or SQPOLL on a shared CPU. ergon-io keeps this: with no
  `sqpoll_cpu`, `uring-sqpoll` takes a CPU of the pod's own outside
  `ERGON_HELPER_CPUS` and refuses (`Error::Cpu`) when there is none
  ([io: Kubernetes](../io/kubernetes.md#shared-cpus-from-the-node));
- two indices are two physical cores only when they are two groups in
  `ERGON_ISOLATED_CORES`. On SMT, `1,5` is one core and its two threads
  share every execution unit: two "hot cores" there is one hot core.

A core is never split between the exclusive and the shared CPUs. On SMT
the hot thread and the helpers would time-share one physical core, which
is the contention this whole setup exists to remove. The plugin refuses
such a container rather than start it claiming an isolation it does not
have. On x86 keep SMT off, or use `full-pcpus-only` and put whole cores
in `reservedSystemCPUs`.

## Worked example: an Aeron media driver

Two pinned cores (sender and receiver) plus the conductor on the shared
CPU, with huge pages. This is the layout a latency node wants.

**Node boot line** (from the table above), for exclusive CPUs 2 and 3,
shared 0, IRQ 1:

```
nohz_full=2-3 rcu_nocbs=2-3 isolcpus=managed_irq,domain,2-3 hugepagesz=2M hugepages=512
```

**ConfigMap** (`sharedCpus: "0"`), **kubelet**
(`reservedSystemCPUs: "0,1"`), and a namespace in the allowlist.

**Pod.** Three grants on top of the shared CPU: a sized hugetlbfs for
`AERON_DIR`, io_uring seccomp, and the NIC PHC. The isolation of CPUs 2-3
comes from the **boot line**, not from a grant: `cpu-partition` needs the
program to move its own threads between cgroups
([Latency Pods](latency-pods.md#2-isolated-partition)), and `aeronmd` does
not.

```yaml
apiVersion: v1
kind: Pod
metadata:
  name: aeron-driver
  namespace: trading
  labels: { ergon.io/latency: "hot" }
  annotations:
    required-plugins.noderesource.dev: '["ergon-shared-cpus"]'
    hugepages.ergon.io/app: "2M"          # sized hugetlbfs for AERON_DIR
    seccomp.ergon.io/app: "io-uring"      # for apps that use io_uring; Aeron does not
    ptp.ergon.io/app: "auto"              # the NIC's PHC
spec:
  nodeSelector: { ergon.io/shared-cpus: "true" }
  priorityClassName: ergon-hot              # k8s/nri-latency-pods/admission/priority-class.yaml
  containers:
  - name: app
    image: ergon-media-driver:dev
    command: ["ergon-cpus",
              "--env", "AERON_SENDER_CPU_AFFINITY=isolated:0",
              "--env", "AERON_RECEIVER_CPU_AFFINITY=isolated:1",
              "--", "aeronmd"]
    env:
    - { name: AERON_THREADING_MODE, value: DEDICATED }
    - { name: AERON_DIR, value: /dev/hugepages/aeron }   # set by the app, see below
    resources:
      requests: { cpu: "2", memory: 2Gi, ergon.io/shared-millicpus: "250", hugepages-2Mi: 256Mi }
      limits:   { cpu: "2", memory: 2Gi, ergon.io/shared-millicpus: "250", hugepages-2Mi: 256Mi }
```

**What each piece does:**

| piece | effect |
|---|---|
| `cpu: "2"` | two exclusive CPUs from the static CPU manager: the sender's and the receiver's |
| `ergon.io/shared-millicpus: "250"` | the plugin adds the shared CPU and starts every thread there |
| `AERON_SENDER_CPU_AFFINITY=isolated:0` | sender on exclusive index 0 |
| `AERON_RECEIVER_CPU_AFFINITY=isolated:1` | receiver on exclusive index 1 |
| `AERON_CONDUCTOR_CPU_AFFINITY` unset | the conductor stays on the shared CPU |
| `AERON_THREADING_MODE=DEDICATED` | sender, receiver and conductor on their own threads |
| `hugepages.ergon.io/app: "2M"` + `hugepages-2Mi` | a **sized** hugetlbfs mounted at `/dev/hugepages` |
| `AERON_DIR` on that hugetlbfs | the CnC file and the terms are 2 MiB pages, not 4 KiB |
| `seccomp.ergon.io/app: "io-uring"` | the profile gains the `io_uring_*` calls an app's uring backend needs; never `Unconfined` |
| `ptp.ergon.io/app: "auto"` | the NIC's PHC, for clock work |
| `ergon.io/latency: "hot"` (pod label) | the admission policy refuses exec probes and `restartPolicy: Always` sidecars |
| `isolcpus=managed_irq,domain,2-3` on the boot line | CPUs 2-3 leave the scheduler's load-balancing domains ([isolation](hft.md#isolation)) |

**The app sets `AERON_DIR` itself.** The plugin exports
`ERGON_HUGEPAGES_DIR` and `ERGON_HUGEPAGE_SIZE` and does not set `AERON_DIR`,
because the app must create the subdirectory:

```sh
mkdir -p "$ERGON_HUGEPAGES_DIR/aeron"
export AERON_DIR="$ERGON_HUGEPAGES_DIR/aeron"
export AERON_FILE_PAGE_SIZE="$ERGON_HUGEPAGE_SIZE"
```

An unsized hugetlbfs — what `emptyDir: {medium: HugePages}` gives you — is
rejected by Aeron. That is why the size comes from the annotation.

**Affinity variables:**

| variable | agent | here |
|---|---|---|
| `AERON_SENDER_CPU_AFFINITY` | sender | `isolated:0` |
| `AERON_RECEIVER_CPU_AFFINITY` | receiver | `isolated:1` |
| `AERON_CONDUCTOR_CPU_AFFINITY` | conductor | leave unset: it starts on the shared CPUs |
| `AERON_DRIVER_NATIVE_RESOURCE_AGENT_CPU_AFFINITY` | native resource agent | leave unset |
| `AERON_DRIVER_CPUSET_AFFINITY` | makes the four above indexes into the cgroup cpuset | keep `false`; the wrapper refuses to translate when it is true |

`ergon-cpus` turns `isolated:<i>` into a real CPU number before exec. It
also sets its own affinity to the shared CPUs, so the driver starts there.

**Check it worked:**

```sh
kubectl exec aeron-driver -- sh -c 'echo isolated=$ERGON_ISOLATED_CPUS shared=$ERGON_SHARED_CPUS'
kubectl exec aeron-driver -- sh -c 'echo hugepages=$ERGON_HUGEPAGES_DIR size=$ERGON_HUGEPAGE_SIZE'
kubectl exec aeron-driver -- sh -c 'ps -L -o pid,tid,psr,comm | head'
```

`psr` must show the sender and receiver on 2 and 3 and every other thread on
0. `mount | grep hugepages` must show a size, not just `hugetlbfs`.

## Fractional shared CPUs

- Extended resources are whole numbers and requests must equal limits, so
  the unit is millicpus: `250` = a quarter.
- The plugin adds `ceil(M/1000)` shared CPUs, at least one. It does not
  grant the whole helper set unless `M = 1000 × |S|`. With `oversubscription: 1`
  that request takes every configured helper CPU, using the same allocator.
- A reconnect keeps the helper CPUs recorded at create. It does not pick
  replacements for a container that is already running.
- The fraction is scheduler accounting only. With 2 shared CPUs and
  oversubscription 2, a node offers 4000; sixteen pods at 250 fit.
- There is no hard cap. Helpers may use the whole shared CPU, shared with
  the kubelet, containerd, system daemons and other pods.
- A helper wake-up on a shared CPU waits on the node's other load; the size
  of that wait is **not measured**. It slows connects and the conductor, not
  the loop.

## Fail closed

| event | effect | guard |
|---|---|---|
| plugin down at create | NRI makes no change | `required-plugins.noderesource.dev` makes the create fail; the wrapper refuses without `ERGON_SHARED_CPUS`; the resource does not allocate |
| kubelet restart, plugin up | the re-write keeps the union | the plugin re-widens in `UpdateContainer` |
| kubelet restart, plugin down | cpuset shrinks to the exclusive CPUs; helpers fall onto them | demonstrated by `test/e2e.sh` step 9. Recovery does not prove the contract held during the outage |
| host drift | a new hot create is refused | the plugin stays connected. `/healthz` stays up. This is not a liveness restart |
| hook binary missing | the create fails | the DaemonSet installs the binary before it connects |

## Limits and what is not verified

- **The plugin is root on the node.** With the install flags the chart
  mounts eight host paths: `/var/run/nri`, `/var/lib/kubelet/device-plugins`
  and `/opt/ergon/bin` read-write; the host's `/dev` at `/host/dev`,
  `/sys/fs/cgroup`, the kubelet config, `/sys` and `/proc` read-only. The
  `:dev` manifest mounts the same. No `hostPID`, no `hostNetwork`,
  not privileged, `capabilities.drop: [ALL]`,
  `readOnlyRootFilesystem: true`, seccomp `RuntimeDefault`. No API access,
  except a get-only ClusterRole on nodes when `config.profiles` is set.
  Only namespaces in the allowlist may opt in.
- **AKS** has no supported way to enable containerd NRI plugins and no
  `reservedSystemCPUs`. Not supported.
- **CRI-O** users use its `shared_cpuset` instead.
- **Bottlerocket**: whether runc may exec a hook binary that a pod wrote to
  a host path is not verified. Use `defaultAffinity: all` and the wrapper
  until checked.
- `kubectl exec` and exec probes start on any CPU of the cpuset, exclusive
  ones included. No exec probes on hot containers; run debug commands as
  `ergon-cpus -- <cmd>`.
- In-place CPU resize leaves the CPU env stale. Use
  `resizePolicy: RestartContainer` for `cpu`.
- **EKS AL2023**: the kubelet settings are in a `config.json.d/` drop-in,
  so hot pods need chart 0.1.2 with `kubeletConfigDir`
  ([Use the public chart](#use-the-public-chart)). With chart 0.1.1 every
  hot pod is refused there; unlabelled pods get the grants.
- The plugin cannot see the kubelet's command line. A `kubeletConfigDir`
  the kubelet does not read (no `--config-dir`) makes the check trust
  drop-ins the kubelet ignores; `kubeletConfigZ` with a fresh `/configz`
  copy cross-checks.
- **Not measured / not verified:** the latency cost of helpers on shared
  CPUs, and a real EKS node (the NRI socket, nodeadm TOML merge, the
  kubelet drop-in merge, `/opt` writable).

**Decision.** Use it on kind or a self-managed node where you can set
`reservedSystemCPUs` and enable NRI. That the exclusive cores stay quiet
with it is a design claim, not a measurement. Not for AKS or CRI-O.
