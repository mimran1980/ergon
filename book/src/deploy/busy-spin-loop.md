# Busy-Spin Loop

One thread, pinned to one CPU, never sleeps. Each turn it polls every
source once, does the application's work, and fires due timers. Everything
that can block or take long runs on another thread.

## The turn

| step | call | bounded by |
|---|---|---|
| 1. network I/O | ergon-io `driver.poll(&mut app)`: receive, run the handler's callbacks, flush sends | the backend's read burst |
| 2. Aeron in | `subscription.poll_fn(handler, FRAGMENT_LIMIT)` | `FRAGMENT_LIMIT` |
| 3. Aeron out | `publication.offer` of what the callbacks queued | stops at the first retryable error; the rest waits for the next turn |
| 4. timers | one TSC read (`quanta`) and a compare | the timer's own work |

The application is one struct the loop hands every event to:

```rust,ignore
{{#include ../../../samples/low-latency-k8s/tests/busy_loop_book.rs:app}}
```

```rust,ignore
{{#include ../../../samples/low-latency-k8s/tests/busy_loop_book.rs:loop}}
```

## Never inside the loop

| never | why | instead |
|---|---|---|
| a blocking call: file or socket I/O outside the driver, DNS, `sleep`, a lock, a channel `recv` | the thread stops until it returns | before the loop, or on the housekeeping thread |
| a connect or TLS handshake | about 0.7 ms of CPU each time ([kubernetes](../io/kubernetes.md#cpu-1)) | ergon-io `connect_background` (its dial helper) |
| `poll_blocking`, or `add_*` with a timeout | busy-waits for the media driver | before the loop; in it, the async poller's `poll()` once a turn |
| allocation | the allocator can lock and fault | reserve before the loop (`VecDeque::with_capacity`) |
| logging that formats and writes | a write blocks when stdout is full | a bounded non-blocking queue to a logging thread ([sample](../samples/low-latency-k8s.md)) |
| host checks, `ThrottleWatch`, `FaultWatch` | each reads a file | the housekeeping thread |
| `Instant::now()` or `SystemTime::now()` for durations | a vDSO call each | `quanta::Clock` (the TSC), calibrated before the loop ([clocks](clocks.md)) |
| creating a thread | it inherits the loop's CPU mask: it would run on the loop's CPU | before the loop pins itself |
| waiting for a retryable Aeron error to clear | stalls every other source | keep the message, retry next turn |

## Start-up order

A new thread inherits its creator's CPU mask. So the loop thread moves to
the helper CPU first, creates every other thread there (housekeeping, the
Aeron client conductor), and only then pins itself to its own CPU:

```rust,ignore
{{#include ../../../samples/low-latency-k8s/tests/busy_loop_book.rs:start}}
```

| pod | the loop | the helpers (housekeeping, `aeron-client`, ergon-io's) |
|---|---|---|
| `cpu: 1` | the only CPU | the same CPU: give production pods 2 |
| x86 with SMT, `cpu: 2` | one hyperthread | its sibling |
| Graviton, `cpu: 2` | one core | the other core |
| exclusive CPUs plus the node's shared CPUs | the exclusive CPU (`ERGON_ISOLATED_CPUS`) | the shared CPUs, where the process starts ([shared CPUs](shared-cpus.md)) |

Details per pod size: [ergon-io in Kubernetes](../io/kubernetes.md).

## Housekeeping

The checks that must run again while the loop runs, every few seconds, on
a thread off the loop's CPU. Each check reads one file opened before the
loop:

```rust,ignore
{{#include ../../../samples/low-latency-k8s/tests/busy_loop_book.rs:housekeeping}}
```

| watch | finds | fix |
|---|---|---|
| `ThrottleWatch` (T9) | the container's CPU quota stopped it | a Guaranteed pod, whole CPUs, the static CPU manager ([checklist](kubernetes.md#latency-checklist)) |
| `FaultWatch` (T19) | the loop took a page fault | `mlockall` and write every buffer once before the loop |

- ergon-io's `Driver` already runs T19 for the thread that calls
  `Driver::new`, every 5 s from its warning thread. A loop without a
  `Driver` (Aeron only) opens its own `FaultWatch`.
- A loop that has not warmed up faults on its first turns: the test prints
  T19 warnings for them.
- The other checks run once, before the loop
  ([where the checks run](../tune/overview.md#where-the-checks-run)).

## Run it

`cd samples/low-latency-k8s && cargo test --test busy_loop_book`, on Linux
(`Driver` is Linux-only). The test thread plays the other side: UDP quotes
in, Aeron orders out, an Aeron ack for each.

The full application, with logging, a placement self-check and a latency
report, is `ll-app` in the [low-latency-k8s sample](../samples/low-latency-k8s.md).
The Aeron calls one by one: [Aeron client](../transports/aeron-client.md).
