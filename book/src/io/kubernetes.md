# Kubernetes

Where ergon-io's helper threads run inside a pod. On a host they run on
the housekeeping CPUs. In a pod there are none. This page says what to
do instead, for every pod size.

Terms: [io glossary](overview.md#glossary),
[deployment glossary](../deploy/overview.md#glossary).

## Why a pod has no housekeeping CPUs

- **On a host**, the housekeeping CPUs are the ones the kernel leaves out
  of `isolcpus`: everything except the isolated cores.
- **Those files describe the node, not the pod.**
  `/sys/devices/system/cpu/{online,isolated}` are host-wide. A pod runs
  only on the CPUs the kubelet gave it: its **cpuset**
  (`cpuset.cpus.effective`). A Guaranteed pod with the static CPU manager
  gets exactly its whole-number `cpu` request. The node's housekeeping
  CPUs are outside that set.
- **A thread outside the cpuset never starts.** The kernel refuses the
  mask (`EINVAL`), and `io_uring_setup` refuses an `SQPOLL` CPU outside
  it the same way.
- **So the helper threads run on the pod's own CPUs.** `Driver::new`
  reads the pod's cpuset once and chooses from it
  ([`placement`](https://docs.rs/ergon-io/latest/ergon_io/placement/)):
  `helper_cpus` / `ERGON_HELPER_CPUS` first, then the loop CPU's SMT
  sibling, then the pod's other CPUs, preferring ones the host does not
  isolate. Never the polling CPU when it is known: set `loop_cpu`, or pin
  the loop thread before `Driver::new`. When it is not known the choice
  cannot keep off it, and the start-up line says the helpers may include
  the CPU the loop ends up on. The line reports the choice.

## Pod layouts

Every layout: Guaranteed QoS (requests equal limits), a whole-number CPU
request, and the kubelet's `cpuManagerPolicy: static` (on x86 with SMT
on, `cpuManagerPolicyOptions: full-pcpus-only`) — the node settings are
under [deployment: Kubernetes](../deploy/kubernetes.md#latency-checklist).

| pod | the loop | the helpers (dial, warnings, SQPOLL) | what to configure |
|---|---|---|---|
| [`cpu: 1`](#cpu-1) | the only CPU | none: they would share the loop's CPU | `Driver::connect`, or `connect_background` with `inline_connect_fallback=true`; `recv-busy` (`per_queue=true` past 4 sockets) |
| [x86 with SMT, `cpu: 2`](#x86-with-smt-cpu-2-one-physical-core) | one hyperthread | the other hyperthread (the loop's SMT sibling) | nothing |
| [Graviton, `cpu: 2`](#graviton-cpu-2-two-cores) | one core | the other core | nothing |
| [shared CPUs](#shared-cpus-from-the-node) | one exclusive CPU | dial and warnings on the shared CPUs the node added; `SQPOLL` on a CPU of the pod's own outside them (never a spinner on shared CPUs) | `helper_cpus` or `ERGON_HELPER_CPUS`; `sqpoll_cpu` only to override that default |

Minimal pod, `cpu: 2` (change to `"1"` for one CPU). Requests equal
limits, and the CPU request is a whole number:

```yaml
apiVersion: v1
kind: Pod
spec:
  containers:
  - name: app
    resources:
      requests: { cpu: "2", memory: 1Gi }
      limits:   { cpu: "2", memory: 1Gi }
```

### `cpu: 1`

`connect_background` refuses at once with `Error::Cpu` (`NoSpareCpu`)
rather than start a dial helper on the polling CPU, and `uring-sqpoll`
refuses at `Driver::new`. The warning thread shares the core, parked
until there is work. One warning line at start says all this.

```rust,ignore
{{#include ../../../io/tests/placement_book.rs:one_cpu}}
```

- With `inline_connect_fallback=true`, `connect_background` connects as
  `Driver::connect` does: the TLS handshake runs on the loop, a long poll
  per connect or reconnect
  ([connection set-up](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#connection-set-up)). A helper
  thread sharing the loop's CPU was not measured.
- **For production, give the pod 2 CPUs** (the warning line ends with
  that). `ERGON_HOST_CHECKS=off` silences the line; the typed refusals
  stay.

### x86 with SMT, `cpu: 2`: one physical core

`cpuManagerPolicyOptions: full-pcpus-only`: the pod holds one core's two
hyperthreads. The loop takes one; `SQPOLL`, the dial helper and the
warning thread take the other. Nothing to configure.

```rust,ignore
{{#include ../../../io/tests/placement_book.rs:x86_two_cpus}}
```

What the SQPOLL thread on the sibling costs against a core of its own:
[SMT sibling cost](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#smt-sibling-cost).

- It needs both hyperthreads and nothing else on the sibling.
- For a core of its own instead: a `cpu: 4` pod (two physical cores) with
  `sqpoll_cpu` on the second core, or SMT off on the node.

### Graviton, `cpu: 2`: two cores

Graviton has no SMT: two vCPUs are two cores. The loop takes one; the
helpers take the other. Nothing to configure.

```rust,ignore
{{#include ../../../io/tests/placement_book.rs:graviton_two_cpus}}
```

### Shared CPUs from the node

A node-level mechanism can add shared CPUs to the pod's cpuset for the
helper threads (dial, warnings), and keep the loop on its exclusive
CPU: [deployment: shared CPUs](../deploy/shared-cpus.md). Name those
CPUs in `Config::helper_cpus`, or let the mechanism set
`ERGON_HELPER_CPUS` (same value: `ERGON_SHARED_CPUS`).

```rust,ignore
{{#include ../../../io/tests/placement_book.rs:helper_cpus}}
```

```rust,ignore
{{#include ../../../io/tests/placement_book.rs:helper_cpus_env}}
```

```yaml
env:
- name: ERGON_HELPER_CPUS
  value: "3-4"          # a kernel CPU list: 3, 3,7, 2-3
```

- A CPU outside the pod's cpuset is refused (`Error::Cpu`, `Outside`),
  never used. `Config::helper_cpus` wins over the variable.
- **Keep `uring-sqpoll` off shared CPUs:** its `SQPOLL` thread spins
  ([io_uring_setup(2)](https://man7.org/linux/man-pages/man2/io_uring_setup.2.html),
  `IORING_SETUP_SQPOLL`). The default never puts it there: with no `sqpoll_cpu` it takes the lowest CPU of the
  pod's own outside that list, and refuses (`Error::Cpu`,
  `NoExclusiveCpu`) when the list holds every CPU but the loop's. Then set
  `sqpoll_cpu` to an exclusive CPU of the pod, or pick a single-CPU backend
  (`recv-busy`, `per_queue`). The dial helper and the warning thread are
  the ones that belong on the shared CPUs.

## Pin the loop thread inside the pod

The kubelet gives the pod its CPUs; it never pins threads to them. Each
busy thread pins itself to one CPU of the pod's cpuset before
`Driver::new`, so the host checks and the helper choice know the polling
CPU. Node settings: [deployment: Kubernetes](../deploy/kubernetes.md).

```rust,ignore
{{#include ../../../io/tests/placement_book.rs:pin_loop}}
```

- The pod's CPUs: `cpuset.cpus.effective` (what
  `placement::process_cpus` reads), or `Cpus_allowed_list` in
  `/proc/self/status`.
- Pinned by other means? Name the CPU as `loop_cpu` in the backend
  configuration ([backends](backends.md#configuration-strings)).

## TCP timers and `TCP_INFO`

Opt-in, off by default. Each is a socket option, so the pod sets it in its
own backend configuration; the node only decides whether the kernel has it.

| key | what | pod or node |
|---|---|---|
| `tcp_rto_min_us` | `TCP_RTO_MIN_US`: the retransmission timeout's floor (kernel default 200 ms). Only this side's retransmissions; a locked route `rto_min` still wins | pod: no capability. Node: kernel 6.15+ and its `CONFIG_HZ` |
| `tcp_delack_max_us` | `TCP_DELACK_MAX_US`: the longest delayed ACK (kernel default 200 ms) | as above |
| `tcp_info_interval_ms` | passive `TCP_INFO` per connection (`Driver::tcp_info`: RTT, min RTT, RTO, retransmits, lost) | pod |

- **Range.** 2,000 to 200,000 µs, in whole kernel ticks of at least two.
  User space cannot read `CONFIG_HZ` (`getconf CLK_TCK` is `USER_HZ`,
  always 100), so `Driver::new` sets each timer on a probe socket and reads
  it back. 5 ms works at `CONFIG_HZ=1000` only; 20 ms works at 100, 250, 300
  and 1000.
- **Refusals are typed, at start-up.** A kernel before 6.15:
  `Error::Unsupported` naming the option. Below two ticks, or a value the
  kernel would round: `Error::Config` naming the key and what the kernel
  keeps. `HttpClient`'s libcurl sockets get the same check and settings.
- **Alternatives.** A route `rto_min` needs `CAP_NET_ADMIN` in the pod's
  network namespace (pod `securityContext`). The `net.ipv4.tcp_rto_min_us`
  sysctl is not on Kubernetes' safe list: the kubelet's
  `--allowed-unsafe-sysctls` (node).
- **`TCP_INFO` cost.** One `getsockopt` per poll at most, from the
  250 µs deadline check, one connection at a time. It takes the socket's
  lock briefly (`lock_sock_fast`), so a softirq delivering to that socket
  waits for it. `DriverStats::tcp_info_max_ns` is the longest read;
  off, nothing runs.
- **Effect not measured.** A lower floor helps only after a lost segment;
  too low a floor sends spurious retransmissions, which a venue may count.

## What the driver reports

- **One start-up line** with the helper CPUs and why
  (`HostReport::helper_line`), or, in a pod with one CPU, the warning
  above.
- **`Driver::host_report()`** carries the same (`HostReport::helpers`).
- `ERGON_HOST_CHECKS=off` silences the warning lines. The typed refusals
  (`Error::Cpu`) are never silenced.
