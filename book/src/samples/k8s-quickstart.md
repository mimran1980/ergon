<!-- Generated page: edits here are overwritten by the next export. sha256:ba4efe1e0ad7bbe3af176f66dc4e694dca268baedda371747375499ed16c80a8 -->
# Kubernetes Quickstart

This sample
is the first thing to try on Kubernetes. One command gives a kind cluster
with four tuned pods: a WebSocket feed whose TCP runs on F-Stack over DPDK,
republished over Aeron through the DPDK media driver to a busy-spinning
app. Each pod prints ergon-tune's findings (with their fixes) at start, and
the feed and the app print latency lines with the max every 5 s.

F-Stack is lab-grade and ergon-md-dpdk is a lab proof of concept: this
sample shows the wiring and the tuning, not a production data path. For the
reference layout with the shared-CPU plugin and a verifier, see
[Low-Latency Kubernetes](low-latency-k8s.md).

## Run it

Needs Docker (9 CPUs or more), kind and kubectl.

`run.sh` builds the cluster, the image and the pods and follows the feed and
the app for 60 s; `down.sh` deletes the cluster and the images.

The first build compiles F-Stack, its DPDK and Aeron's C (about 15 minutes);
later builds reuse them. `run.sh` is safe to run again: it reuses the
cluster, rebuilds the image and restarts the pods. `QS_FOLLOW_SECS=0`
follows until Ctrl-C.

| step | what happens |
|---|---|
| cluster | `kind create cluster --config node/kind.yaml` (name `ergon-quickstart`), a 512 MiB tmpfs at `/mnt/huge-aeron` in the node |
| image | the F-Stack base, then `docker/Dockerfile`; `kind load docker-image` |
| pods | `kubectl apply -k k8s/overlays/kind`, then waits until each is ready (the feed and the app: their loop has reported once) |
| output | the echo's and the driver's start-up, then the feed's and the app's logs |

## What you get

```text
qs-ws-echo ──TLS WebSocket──► qs-feed ──aeron:udp──► qs-media-driver ──► qs-app
 kernel TCP    (echoes)      F-Stack TCP   (offer)   md-dpdk sender port    subscriber
                             DPDK port 1             wire, receiver port    busy-spin
                                                     DPDK ports 2 and 3
```

| pod | CPUs | does |
|---|---|---|
| `qs-ws-echo` | 1 | TLS WebSocket echo server on kernel TCP, standing in for a remote feed; serves its self-signed certificate on `:8080/cert.pem` |
| `qs-feed` | 2 | ergon-io's WebSocket client on `fstack-spin`: sends 10 000 messages a second of 64 bytes on a fixed timetable, checks each echo byte for byte (a mismatch stops it), times it from its **intended** send time, stamps it and offers it to an Aeron exclusive publication |
| `qs-media-driver` | 2 | Aeron's C media driver on ergon-md-dpdk, `SHARED_NETWORK`: the network agent spins on one CPU, the conductor and DPDK housekeeping use the other |
| `qs-app` | 2 | Aeron subscriber spinning on one CPU; times each message from the feed's stamps |

Every pod is Guaranteed with whole CPUs, so the static CPU manager gives it
CPUs of its own. Each loop spins on a CPU taken from its pod's cgroup
cpuset; the helpers (reporter, health server, Aeron's client conductor,
ergon-io's warning thread, the EAL's control threads) run on the pod's other
CPU. Nothing on a loop allocates, locks, prints or waits: an offer Aeron
refuses is counted and dropped (a term rotation is retried at once, at most
four times).

F-Stack and md-dpdk never meet:

| | feed (F-Stack) | media driver (md-dpdk) |
|---|---|---|
| process, pod | `feed` in `qs-feed` | `media_driver` in `qs-media-driver` |
| DPDK | F-Stack's own, static | Debian's 24.11, shared |
| EAL file prefix | `qs-feed` | `ll-<pod>` |
| port on kind | `net_af_packet0` on a veth in its pod (10.9.0.2, NAT out of the pod) | `net_tap0` 10.9.0.1 and `net_tap1` 10.9.0.3, bridged in its pod |
| port on EKS | its own ENA (device number 3) | md-dpdk's ENA pair (1 and 2) |

The feed publishes to `aeron:udp?endpoint=10.9.0.3:40001`, the driver's
receiver port: each message leaves by the sender port and comes back in by
the receiver port. Aeron IPC would skip md-dpdk. The feed, the driver and
the app share the node's Aeron directory (`/mnt/huge-aeron`) and its CPU
counter (TSC, `CNTVCT_EL0`), which is why they run on one node: the app's
pod affinity holds it to the feed's node, and a stamp later than the app's
own reading (another node's counter) is counted as `stamped after the app's
clock` and never timed.

## Read the output

Every latency figure comes with its max. One kind run (arm64 macOS, Docker
Desktop, 10 vCPUs), one 5 s line of each:

```text
feed: 5.0 s sent 50000 echoed 50000 published 50000 ws rtt p50 34.0 us p99 93.4 us p99.9 1.45 ms max 2.98 ms
app: 5.0 s recv 50000 aeron hop p50 14.5 us p99 61.1 us p99.9 138.9 us max 875.5 us | ws send to app p50 48.9 us p99 131.2 us p99.9 1.02 ms max 2.51 ms
```

| field | measured from | to |
|---|---|---|
| `ws rtt` | the message's intended send time (so a late send counts) | the echo's arrival in the feed |
| `aeron hop` | the feed's offer | the app's receipt, through md-dpdk |
| `ws send to app` | the message's intended send time (so a late send counts) | the app's receipt |
| `not published` | Aeron refused the offer (back-pressure, no subscriber) | |
| `missing` | sequence numbers the app never saw | |

On SIGTERM each prints a `total:` line over the run. On kind the figures
measure the laptop: the VM's vCPUs are host threads that macOS time-slices,
and the kind node shares Docker's kernel, with no isolated CPUs. In that
run (19 intervals) the maxima of `ws send to app` followed the feed's
`ws rtt` maxima (the WebSocket leg: kernel TCP in the echo pod, the veth
and NAT), while the Aeron hop's stayed under 1 ms in 13 of them. The worst
interval held every max: `ws rtt` 21.95 ms, `aeron hop` 4.61 ms, `ws send to
app` 22.38 ms, in the same 5 s. Latency verdicts
come from a tuned Linux node.

Each pod's start-up, long lines cut:

```text
feed: pod CPUs 4-5 (from its cgroup cpuset, ...); loop on CPU 5, helpers on CPU 4
feed: memory locked (mlockall): no page fault on first touch in the loop
feed: ergon-tune: ergon-io ran 21 checks at start, the feed 3 more; each WARN carries its fix:
feed:  WARN T1 warn: CPU 5 (loop) is not isolated: ... Fix: boot with `isolcpus=nohz,domain,managed_irq,5 nohz_full=5 rcu_nocbs=5 irqaffinity=0-4,6-9` ...
feed:  WARN T13 warn: CPU 5 (loop) is in the unbound workqueue cpumask ...
feed:  ok T9 ok: no CPU quota on this process's cgroup ...
feed:  ok K1 ok: Guaranteed QoS ...
feed:  ok T4 ok: the Aeron directory /mnt/huge-aeron/aeron is on tmpfs
feed: ergon-tune: summary 3 warnings, 3 info, 2 unknown, 16 ok (ERGON_HOST_CHECKS=on)
```

| tag | means | action |
|---|---|---|
| `WARN` | a setting that costs the loop latency; the line ends with the exact fix and its evidence | apply the fix where it says (node or pod) |
| `info` | worth knowing, not measured to matter here | read it |
| `UNKNOWN` | the pod cannot see it (another PID or cgroup namespace) | check on the node: `ergon-tune --hot <cpu>` |
| `ok` | proven fine | nothing |

The app and the echo print the same set (the `ergon-tune` command's, for one
spinning thread); the driver prints ergon-md-dpdk's own. On kind, T1 and
T13 warn in every pod, and T3 everywhere but the feed (ergon-io rates it
info): the node shares Docker's kernel, which has no isolated CPUs, no
`nohz_full` and transparent hugepages `always`. K6 warns when the pod's
CPU saw any pressure. The echo's K4 warns too: it has one CPU and no helper
CPU, which costs nothing here because it never spins. Under
`ERGON_HOST_CHECKS=require` (set it in `k8s/base/quickstart.env`) a
warning, or a pod property ergon-tune cannot prove, stops the start.

## Settings, and where each one goes

| setting | where | file | ergon-tune |
|---|---|---|---|
| requests = limits for CPU and memory, whole CPUs | pod | `k8s/base/*.yaml` | K1, K4 |
| `cpuManagerPolicy: static`, `full-pcpus-only`, `reservedSystemCPUs` | node (kubelet) | `node/kind.yaml`; on EKS md-dpdk's node config | K2, T9 |
| `net.ipv4.tcp_autocorking: "0"` | pod `securityContext.sysctls`; the kubelet's `allowedUnsafeSysctls` | `ws-echo.yaml`, `feed.yaml`, `node/kind.yaml` | T21 |
| `httpGet` probes (`/healthz`, `/readyz`), never `exec` | pod | pod yamls | |
| loop CPU from the pod's cpuset; helpers on its other CPU | app | `src/cpus.rs`, `src/run.rs` | T1, T8, K3, K4 |
| the driver's CPUs from its cpuset | entrypoint | `docker/qs-driver.sh` | T1, T8 |
| `mlockall` before the loop | app (`CAP_IPC_LOCK`, or `RLIMIT_MEMLOCK`) | `src/run.rs`, `app.yaml` | T19 |
| the Aeron directory on tmpfs or hugetlbfs | node | `run.sh`; md-dpdk's bootstrap on EKS | T4 |
| `isolcpus`, `nohz_full`, `rcu_nocbs`, `irqaffinity`, THP `never`, SMT off | node (boot line) | md-dpdk's `node/provision-node.sh` on EKS; not possible on kind | T1, T3, T10, T13 |

The feed and the driver are privileged (DPDK takes their ports; on kind the
feed also makes its veth and NAT) and run as root; the app runs as root
with every capability dropped but `IPC_LOCK`, because the Aeron files are
the driver's; the echo runs as a non-root user with every capability
dropped.

## Move to EKS

The EKS overlay renders and is checked offline (`tests/manifests.rs`); it
has not run on EKS.

| piece | what |
|---|---|
| node | one c8g.2xlarge (Graviton4, 8 CPUs, no SMT, 4 ENIs): md-dpdk's ENA node user data (CPU 0 reserved, 1-7 isolated, hugepages, Aeron's hugetlbfs at `/mnt/huge-aeron`) |
| ENIs | primary (management), device numbers 1 and 2 (the driver's pair, bound by md-dpdk's bootstrap, granted by its device plugin), 3 (F-Stack's); the three data ENIs tagged `node.k8s.amazonaws.com/no_manage=true` so the VPC CNI never claims them |
| F-Stack's ENA | `node/eks-fstack-ena.sh` on the node after each boot: binds it to md-dpdk's DPDK driver and prints the feed's `QS_BACKEND` line |
| CPUs | feed, driver and app take 2 each: 6 of the 7 left after CPU 0; the echo runs on another node |
| hugepages | the driver 2 GiB (md-dpdk's budget: its mempools plus Aeron's 1 GiB hugetlbfs), the feed 1 GiB: 3 of the node's 4 GiB |
| channel | the feed publishes to the receiver ENA (device number 2): its address in place of `RECEIVER_IP` in `QS_AERON_CHANNEL` |
| overlay | `k8s/overlays/eks`: replace `REGISTRY`, `RECEIVER_IP` and the `FSTACK_*` placeholders, then `kubectl apply -k` |

## Tests

| where | what |
|---|---|
| host | fmt, clippy, unit tests (cpuset layout, timetable, message layout and byte-for-byte check, percentiles, settings, health routes, check counts), manifest tests (`kubectl kustomize` of both overlays: Guaranteed, whole CPUs, no `exec` probes, privilege only for the DPDK pods, the app on the feed's node, separate EAL prefixes and ports, one Aeron directory, `aeron:udp` to the receiver port, the driver's values equal to low-latency-k8s's, the ENA node on EKS; the driver's CPU split; the ENA line), shellcheck |
| Docker | in Docker: clippy with and without F-Stack, the unit tests (with the Linux-only ones: the app's cross-node guard), `tests/loopback.rs` (echo, feed and app on 127.0.0.1 with an embedded media driver on kernel UDP: latency lines with their max, SIGTERM `total:` lines and exit 0, a one-byte change in an echo stops the feed, `fstack-spin` refused without the feature), and the feed built with F-Stack |
| kind | the kind run above |
