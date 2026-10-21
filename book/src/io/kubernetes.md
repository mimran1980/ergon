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
| [`cpu: 1`](#cpu-1) | the only CPU | none: they would time-slice the loop | `Driver::connect`, or `connect_background` with `inline_connect_fallback=true`; `recv-busy` or `epoll-spin` |
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

- **This is not a hidden latency cost.** A helper on the loop's core
  would time-slice the polling thread: worse than the handshake. With
  `inline_connect_fallback=true` `connect_background` connects as
  `Driver::connect` does: the TLS handshake runs on the loop, once per
  connect or reconnect. Figures:
  [`io/benches/BENCHMARK.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md).
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

What a busy sibling costs against a core of its own (arrival → decoded,
run `flags-aws-20261015T122127Z`, three rounds, AWS Tokyo `c7i.2xlarge`):
about **+0.5 µs p50 at 1 connection, +2.3–2.7 µs p50 and +9–12 µs p99 at
10–100 connections**. At 100 connections it still beat every single-vCPU
option by about 8 µs at the median. The maxima are not in that summary —
the full tables with them are in git history at `2517309d`. Deltas only:
no absolute latency figure is claimed here. Tables:
[`io/benches/BENCHMARK.md`](https://github.com/mimran1980/ergon/blob/main/io/benches/BENCHMARK.md#smt-sibling-cost).

- It needs both hyperthreads and nothing else on the sibling.
- Never a second isolated full core just for SQPOLL.

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
- **Never `uring-sqpoll` on shared CPUs**: `SQPOLL` spins. The default
  never puts it there: with no `sqpoll_cpu` it takes the lowest CPU of the
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

## What the driver reports

- **One start-up line** with the helper CPUs and why
  (`HostReport::helper_line`), or, in a pod with one CPU, the warning
  above.
- **`Driver::host_report()`** carries the same (`HostReport::helpers`).
- `ERGON_HOST_CHECKS=off` silences the warning lines. The typed refusals
  (`Error::Cpu`) are never silenced.
