# ergon-runtime

A single-threaded, busy-spin application runtime on Aeron. An application
implements `rt::Agent`; `rt::Runtime` owns the loop, the clock, the timers,
the feeds and the outputs. The same agent runs live, in replay from the Aeron
Archive, and in a backtest on simulated time, because it reads time only from
`Ctx`.

```text
duty cycle: poll every feed (up to `limit` each) -> fire due timers -> idle(work)
```

Housekeeping (`Persist::poll`, the wall-clock offset, the `Source` heartbeat,
`streams.yaml`, SIGTERM) runs on runtime-owned timers in the same wheel. Each
waits for a cycle that found no work, for at most half a millisecond, so it
never delays a message that is already waiting.

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
state allocates nothing. Three of Agrona's behaviours differ:

- a poll after a jump of many ticks visits only occupied spokes, once;
- timers due in one poll fire in `(deadline, seq)` order, the same order the
  simulation merge uses;
- in simulation each timer fires exactly at its deadline.

Timer tokens are caller-assigned (Aeron Cluster's correlation id); tokens with
the top bit set are the runtime's own. `just bench-runtime` compares the wheel
with a lazy-delete binary heap held to the same contract, and fails if the
wheel is slower in any scenario.

## Process set-up for production

All of these are deployment choices, not code:

- `IDLE=spin` on an isolated core (the default for runtime agents); the lab
  uses `yield`.
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
