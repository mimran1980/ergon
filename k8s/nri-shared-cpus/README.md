# ergon-shared-cpus

A containerd NRI plugin and a kubelet device plugin in one daemon, plus a
small tool. Together they give a Kubernetes pod its exclusive CPUs **and**
the node's shared housekeeping CPUs, and start every thread on the shared
ones. The busy-spin loop keeps its exclusive core; only sleepy helpers
(dial, TLS, warnings, Aeron conductors) run on the shared CPUs.

**Not released.** Nothing is on a registry. The Dockerfile, the Helm chart in
[`charts/ergon-shared-cpus`](charts/ergon-shared-cpus) and the image workflow
are ready to publish, but the EKS lane and the latency benchmarks are still
open. [`RELEASE.md`](RELEASE.md) is the gate: it names every blocker and the
exact publish step once they are clear. Tested on kind, not yet on EKS.

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
- create helpers from a thread that is still on the shared set (a new
  thread copies its creator's mask);
- each sleepy helper checks `sched_getcpu()` when it wakes and alarms if it
  is not on `ERGON_SHARED_CPUS`.

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

## Deploy

1. Node: Kubernetes >= 1.33, `cpuManagerPolicy: static`,
   `reservedSystemCPUs` = the IRQ CPU(s) + the shared CPUs, kernel >= 6.2,
   cgroup v2, containerd >= 2.2 with the NRI default validator on. Label
   the node `ergon.io/shared-cpus=true`. A `cpu-partition` opt-in also needs
   `cpuset.cpus.exclusive`, which is Linux 6.7 or a vendor kernel that
   backported it: 6.2 is only enough for the shared CPUs. The plugin reads
   the file rather than the version, so a backport is accepted.
2. Image: `docker build -t ergon-shared-cpus:dev .` and load it on the node.
3. `kubectl apply -f deploy/namespace.yaml -f deploy/configmap.yaml -f deploy/daemonset.yaml`
4. Edit `deploy/configmap.yaml` first: `sharedCpus` must be inside
   `reservedSystemCPUs`, and `namespaces` is the allowlist.

containerd:

```toml
[plugins."io.containerd.nri.v1.nri".default_validator]
  enable = true
  # leave required_plugins empty: a node-wide list would also block
  # kube-system pods and this plugin's own pod while it is down
```

The DaemonSet needs five host paths: `/var/run/nri`,
`/var/lib/kubelet/device-plugins` and `/opt/ergon/bin` read-write, plus
`/dev` at `/host/dev` and the host cgroup v2 tree at `/sys/fs/cgroup`, both
read-only. The namespace needs Pod Security `privileged`. It is root on the
node: anything on the NRI socket can change any container.

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
| plugin down at create | `required-plugins` makes the create fail (containerd >= 2.2); the wrapper refuses without `ERGON_SHARED_CPUS`; the device plugin is down too, so the resource does not allocate |
| kubelet restart, plugin up | the re-write goes through `UpdateContainer` and keeps the union |
| kubelet restart, plugin down | cpuset shrinks to the exclusive CPUs; helpers fall onto them; `Synchronize` re-widens on return |
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
  ones included. No exec probes on hot containers; run debug commands as
  `ergon-cpus -- <cmd>`.
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
