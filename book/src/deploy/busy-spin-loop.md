# Busy-spin loop

One thread, pinned to one CPU, never sleeps. Each turn it polls every
source once, does the application's work, and fires due timers. Everything
that can block or take long runs on another thread.

## The turn

| step | call | bounded by |
|---|---|---|
| 1. network I/O | your socket or kernel-bypass poll: receive, run the handlers, flush sends | the backend's read burst |
| 2. Aeron in | `subscription.poll_fn(handler, FRAGMENT_LIMIT)` | `FRAGMENT_LIMIT` |
| 3. Aeron out | `publication.offer` of what the handlers queued | stops at the first retryable error; the rest waits for the next turn |
| 4. timers | one TSC read (`quanta`) and a compare | the timer's own work |

The application is one struct the loop hands every event to. The loop owns
the sources, the clock and the timers; the struct owns the state.

<div class="ergon-anim" data-anim="busy-spin-turn" data-title="One turn of the busy-spin loop"></div>

*Animation, in words:* two turns of the ring, with illustrative counts. Turn
1: network I/O reads two packets (at most the read burst), the handlers run
and queue `m1`. Aeron in has five fragments waiting but `FRAGMENT_LIMIT` is
3, so it reads three and the other two stay in the image; the handler queues
`m2` and `m3`. Aeron out commits `m1`, then `m2` gets `BackPressured`, a
retryable error, so the loop stops offering and keeps `m2` and `m3`. Timers
read the TSC once and compare: nothing is due (quanta's median batch on
c7i.2xlarge is 15.99 ns a read, slowest batch 16.95 ns; the slowest single
read is not recorded ([clocks](clocks.md))). Turn 2: the socket has
nothing, so the read returns at once; Aeron in reads the two leftover
fragments; Aeron out commits `m2` then `m3`, in queue order; a timer is due
and its work runs. Never in the loop: allocation (reserved before it),
blocking calls, connects and TLS handshakes (a helper thread), and host
checks (the housekeeping thread), those threads on a CPU other than the
loop's. Why it wins: each step is bounded and back-pressure carries over instead of
waiting; on one Azure Cobalt VM with helpers off its CPU the loop measured
p50 0.02 µs, p99.99 0.03 µs, max 4.81 to 5.52 µs, the max being the
hypervisor's 3.5 to 5.5 µs floor ([shared CPUs](shared-cpus.md)). When it loses: it burns a whole
CPU even when idle, a `cpu: 1` pod puts the helpers on the loop's CPU, and
polling many sources every turn adds up: plain `recv-busy` at 64 sockets
costs about 0.6 µs a socket a poll on Graviton ([findings](../findings.md)). The
animation shows order, not timings.

## Never inside the loop

| never | why | instead |
|---|---|---|
| a blocking call: file or socket I/O outside the poll, DNS, `sleep`, a lock, a channel `recv` | the thread stops until it returns | before the loop, or on the housekeeping thread |
| a connect or TLS handshake | about 0.7 ms of CPU each time | a helper thread that dials and hands the connected socket over |
| `poll_blocking`, or `add_*` with a timeout | busy-waits for the media driver | before the loop; in it, the async poller's `poll()` once a turn |
| allocation | the allocator can lock and fault | reserve before the loop (`VecDeque::with_capacity`) |
| logging that formats and writes | a write blocks when stdout is full | a bounded non-blocking queue to a logging thread |
| host checks, `ThrottleWatch`, `FaultWatch` | each reads a file | the housekeeping thread |
| `Instant::now()` or `SystemTime::now()` for durations | a vDSO call each | `quanta::Clock` (the TSC), calibrated before the loop ([clocks](clocks.md)) |
| creating a thread | it inherits the loop's CPU mask: it would run on the loop's CPU | before the loop pins itself |
| waiting for a retryable Aeron error to clear | stalls every other source | keep the message, retry next turn |

## Start-up order

A new thread inherits its creator's CPU mask. So the loop thread moves to
the helper CPU first, creates every other thread there (housekeeping, the
Aeron client conductor, logging), and only then pins itself to its own CPU.

| pod | the loop | the helpers (housekeeping, `aeron-client`, logging) |
|---|---|---|
| `cpu: 1` | the only CPU | the same CPU: give production pods 2 |
| x86 with SMT, `cpu: 2` | one hyperthread | its sibling |
| Graviton, `cpu: 2` | one core | the other core |
| exclusive CPUs plus the node's shared CPUs | the exclusive CPU (`ERGON_ISOLATED_CPUS`) | the shared CPUs, where the process starts ([shared CPUs](shared-cpus.md)) |

## Housekeeping

The checks that must run again while the loop runs, every few seconds, on
a thread off the loop's CPU. Each check reads one file opened before the
loop:

| watch | finds | fix |
|---|---|---|
| `ThrottleWatch` (T9) | the container's CPU quota stopped it | a Guaranteed pod, whole CPUs, the static CPU manager ([checklist](kubernetes.md#latency-checklist)) |
| `FaultWatch` (T19) | the loop took a page fault | `mlockall` and write every buffer once before the loop |

- Open the `FaultWatch` for the loop's thread before the loop starts.
- A loop that has not warmed up faults on its first turns: expect T19
  warnings for them.
- The other checks run once, before the loop
  ([where the checks run](../tune/overview.md#where-the-checks-run)).

The Aeron calls one by one: [rusteron-client](https://docs.rs/rusteron-client).
