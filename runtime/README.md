# ergon-runtime

A single-threaded, busy-spin application runtime on Aeron. An application
implements `rt::Agent`; `rt::Runtime` owns the loop, the clock, the timers,
the feeds and the outputs. The same agent runs live, in replay from the Aeron
Archive, and in a backtest on simulated time, because it reads time only from
`Ctx`.

```text
duty cycle: poll every feed (up to `limit` each) -> agent.poll -> fire due timers
            -> Aeron conductor (when the cycle found no work, or 1 ms after its last run)
            -> idle(work)
```

## Client, not server

This crate is the client: feed handlers, engines and simulated venues, each on
one busy-spin thread, built for latency. Its library code has:

- no `Arc`, `Mutex`, `RwLock`, `OnceLock`, atomics, channels, spawned threads
  or name resolution (the media driver resolves names; only the `clickhouse`
  feature's HTTP client resolves its server's, in replays and backtests),
  enforced by `clippy.toml` (`disallowed-types`, `disallowed-methods`): a
  test lints a use of each entry and requires its rejection, and the lab's
  clients (`lab`, `md`, `engine`) carry the same entries;
- no `unsafe impl` of `Send` or `Sync` (a test scans the sources), and
  handles that are not `Send` (`Persist`, `Tracer`, `Metrics` and its
  handles, `Bus`, `Ctx`, `Invoker`, `Runtime`, `Publication`), which the
  library's tests assert at compile time;
- one owner of the Aeron client, `Bus` (the runtime's `Ctx`, or a simulation
  that records or replays through it), and one of each exclusive publication;
  owners close what they own;
- Aeron's conductor driven from the loop (`Bus::poll`), not from a thread of
  its own. Every wait on Aeron drives it too: a client nothing drives for the
  10 s liveness timeout is closed by the driver. A persistent feed's poll runs
  it as well, and polls the feed's archive client (Aeron's persistent
  subscription does both, the first for a client with no conductor thread),
  so each persistent feed adds a conductor run and an archive poll to every
  cycle. Measured on an Apple M4, that is about 35 ns per persistent feed. With
  the engine's seven, an idle cycle rises from about 53 to about 290 ns. A lone
  message's p50 at an idle loop rises from 166 to 250 ns. The cost is accepted:
  it buys the persistent subscription's fall-back to the archive whenever a
  feed falls behind.

One concurrent publication is the exception: the `tracing` bridge
(`Persist::layer`), on its own small IPC channel, which records `tracing`
events that name a `table`, and spans, from any thread. Aeron's multi-writer
publication is thread-safe without anything of ours; what each thread has sent
is its own, in a thread-local. The bridge is cold, and no loop path writes to
it. It switches nothing: the ingester applies `tables.yaml`'s
`enabled`/`apps`/`until` to event rows and `otel_traces` per app, at each
row's time, and a table it does not list is off. Everything else records
through the loop's `Persist`; there is no process-wide handle.

Anything that would block the loop runs in one of four places:

- before the loop starts;
- in the media driver;
- in idle-deferred housekeeping on the timer wheel;
- in the server.

`ergon-runtime-server`, the ingester that moves Archive recordings into
ClickHouse, is built for throughput instead. It may use threads, locks and
channels.

Housekeeping (`Persist::poll`, the wall-clock offset, the `Source` heartbeat,
SIGTERM) runs on runtime-owned timers in the same wheel. Each waits for a cycle
that found no work, for at most half a millisecond, so it never delays a
message that is already waiting.

SIGTERM is a byte on a pipe: `rt::sigterm()` registers a handler that writes
one to a socket pair, and housekeeping reads the other end every 10 ms, with
no flag shared with the handler. The loop stops, and `Invoker::finish`, which
runs once whoever calls it, stops the agent and closes the outputs and
persist's publication at once (an invoker-mode client closes each before the
call returns), so subscribers turn to the next publisher within seconds. A
send or record after it is not published, and not counted as a drop.

## Feeds by name

`ctx.subscribe(service, kind)` and `ctx.publish(service, kind)` ask the
application's `directory::Directory` (`rt::Config::directory`) for the
channel, stream id and, for a persistent subscription, the archive that
records the feed. The runtime keeps no registry of its own; with the default
`NoDirectory` an application subscribes by channel. `ctx.set_directory`
swaps it when the application's registry changes while it runs. A
simulation names each feed `service/kind`; only an archive replay asks the
directory, for each feed's stream id, to find its recordings.

`ctx.region()` is the process's region: `REGION`, or `unknown` when it is
unset. An application uses it to name its own services and to tell near feeds
from far ones.

## Time

| Call | Live | Simulation |
|---|---|---|
| `ctx.now()` | TSC read once per delivered message, cached | the event's publish stamp, or the timer's deadline |
| `ctx.read()` | a TSC read, for latency marks inside one event | `ctx.now()` |
| `ctx.wall_ns()` | `now` + a wall-clock offset re-measured every second, no system call | `now` |
| `ctx.from_remote(epoch_ns)` | another process's wall-clock stamp as an event time | the same, offset 0 |

`Nanos` is UNIX-epoch nanoseconds everywhere. Agent crates carry a
`clippy.toml` that rejects `Instant::now`, `SystemTime::now`,
`minstant::Instant::now`, `jiff::Timestamp::now`, `HashMap` and `HashSet`
(use `DetMap` or `BTreeMap`); see `samples/clickhouse/engine/clippy.toml`.

## Timers

`timer::TimerWheel` is Agrona's `DeadlineTimerWheel` in Rust, on epoch
nanoseconds: O(1) schedule and cancel, an idle poll is one compare, and steady
state allocates nothing. Four of Agrona's behaviours differ:

- a poll after a jump of many ticks visits only occupied spokes, once;
- timers due in one poll fire in `(deadline, seq)` order, the same order the
  simulation merge uses;
- in simulation each timer fires exactly at its deadline;
- a crowded tick never grows the wheel.

A timer is one 32-byte record indexed by its handle, and each spoke is a
doubly linked list threaded through the records, its earliest deadline at the
head. A schedule pops a free handle, writes the record and links it into its
spoke; a cancel unlinks it and frees the handle. A repeating timer's period
sits in a side column, and a re-arm relinks the same record, so its `TimerId`
survives. Spokes keep no occupancy bits: a poll after a jump, and the
next-deadline search the simulation makes after every firing, read the spoke
heads instead, and the search reads one record per occupied spoke. The
exception is a spoke whose earliest timer left since the last search: its list
is walked once to find the new earliest. So a simulation that keeps many
distinct deadlines on one tick and fires or cancels the earliest pays that
walk. In the bench's ungated probes (the simulation's step and the search
after cancelling the earliest), a heap is faster than the wheel.

`Settings::timers_per_spoke` is an average: the slab starts with
`ticks_per_wheel` times that many records, one spoke can hold any number of
them, and the slab doubles, off the hot path, only when every record is in
use. A re-arm reuses its own record, so it never needs room and a repeating
timer is never dropped. A default wheel takes 2.28 MB.

Timer tokens are caller-assigned (Aeron Cluster's correlation id); tokens with
the top bit set are the runtime's own. `just bench-runtime` compares the wheel
with a lazy-delete binary heap held to the same contract, and fails if the
wheel is slower in any scenario. It also reports, without gating, the
simulation's step and the next-deadline search after a cancelled earliest
timer.

## Process set-up for production

All of these are deployment choices, not code:

- `IDLE=spin` on an isolated core (the default for runtime agents). `IDLE`
  also takes `noop`, `yield` and `sleep[:<period>]`. It takes
  `backoff[:<spins>,<yields>,<min park>,<max park>]`, which is Agrona's
  `BackoffIdleStrategy`: spin, yield, then park, doubling up to the cap;
  `backoff` alone is `10,5,1us,1ms`.
  - The lab's manifests set `backoff`; `just spin` puts the engine and the
    dummy exchange on `spin` with a core each.
  - A sleep or park wakes up to the thread's timer slack late, 50 µs by
    default on Linux. On an Azure `D4s_v6`, a requested 1 µs took 61 µs at
    p50.
  - `TIMER_SLACK=1ns` cut that to 7 µs, but a loop of such sleeps then
    burned most of a core. So set it only with short parks you mean to
    take.
- The media driver (`aeron-driver`) runs `DEDICATED` threading with busy-spin
  idle strategies on its own isolated cores, with
  `aeron.term.buffer.sparse.file=false` and `aeron.pre.touch.mapped.memory=true`.
- Linux pins `minstant` to the TSC. If the TSC is unstable, every latency
  number is wrong; check `dmesg | grep -i tsc` on a new instance type.

## Allocator metrics (`mimalloc` feature)

`alloc_stats::AllocStats::sample` reads mimalloc's OS-level counters and the
process's (and on Linux the loop thread's) resource usage, and publishes
gauges and counters on the metrics path: `mem_committed_bytes`,
`mem_reserved_bytes`, `mem_rss_peak_bytes`, `alloc_os_mmap_calls`,
`alloc_os_commit_calls`, `alloc_os_purge_calls`, `alloc_purged_bytes`,
`cpu_user_ms`, `cpu_sys_ms`, `faults_major`, `loop_faults_minor`,
`loop_faults_major`, `loop_ctx_switches_voluntary`,
`loop_ctx_switches_involuntary`, and the sample's own cost, `alloc_sample_ns`.
Any `alloc_os_*` or `loop_*` increment during trading hours is a latency-spike
candidate.

Release builds of mimalloc keep only these OS-level counters. Building it with
`MI_STAT=1` would add an atomic to every allocation in the process, so it is
not done. For a one-off deep dive, set `MIMALLOC_SHOW_STATS=1` (mimalloc's
full report at exit) or `MIMALLOC_VERBOSE=1` (its OS calls as they happen).
