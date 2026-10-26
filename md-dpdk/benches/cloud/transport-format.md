# Transport suite: what the runner writes and what the analysis reads

`analyse.py --transport`, `spikes.py --transport` and `fairness.py` read a
transport run directory in this shape. The runner (`transport.sh`, `vm.sh`,
`host.sh`) and the latency core (`ergon-latency-core`) write what it keeps
per point (`manifest/`, `points/`, `irq/`, `nstat/`, `netem/`, `samples/`,
`settings/`); `transport_collect.py` lays that out as below when the session
ends. A field
this file does not list is ignored; a field it lists as required and that
is missing is a fairness violation, never a default.

## Layout

```
<run>/results.csv                 every row, rep first (as lab.sh writes it)
<run>/transport-settings.env      the settings each arm must run with (transport_collect.py)
<run>/transport-settings.tr.env   the TR_* file the session sourced
<run>/<set>/manifest-<tag>.json   one per point
<run>/<set>/spikes-<tag>.csv      the point's spikes and its max
<run>/<set>/<host>-events-<tag>.csv  timed events on a host during the point
<run>/<set>/<host>-diag-<set>.log    vm.sh diag: gapspin, trace, knoise
<run>/<set>/<host>-rings-<tag>.log   DPDK event log (dpdk arms)
<run>/<set>/hostcheck-<a|b>.txt   each host's session hostcheck report
<run>/<set>/hostcheck-<a|b>-hugepages.txt  the one on the hugetlbfs Aeron directory (aeron-ipc-huge's)
```

`<set>` is the matrix set (`1`, `2a`, `2b`, `3a`, `3b`, `3c`, `fan`).
Manifest host keys: a two-host set's `a` and `b`; a one-host set's one host
is `a` whichever VM it ran on (`hosts` in the manifest maps key to VM).
`<tag>` is `<set>-<arm-mode>-<size>-<rep>-r<rate>`, the load point's name as
lib.sh spells it (`file_name(label)-size-rep-point`); a one-way row's files
add `-oneway`.

## Labels

A row's label is `<set>:<arm>[@STREAMS=<n>][@NETEM=<pct>]`, the A/A arm's
arm `aa`. The runner always writes `@STREAMS`; absent is 1; `@NETEM` absent is no loss row. `@NETEM=0` is netem at 0 %,
`@NETEM=none` the 3c control with no qdisc. The point column is
`<transport>-r<rate>` (for example `udp-r10000`, `tcp-r50000`, `ipc-r1000000`);
the analysis reads the rate after `-r`.

## results.csv rows

The `load` row, unchanged (columns 0-22, see `analyse.py`), then:

| column | name | meaning |
|---|---|---|
| 23 | min | smallest latency, ns |
| 24 | lost | messages never received |
| 25 | reordered | messages received after a later sequence |
| 26 | dup | messages received twice |
| 27 | retrans | retransmits the transport reported (Aeron, TCP) |
| 28 | wire_bytes | bytes on the wire per message, headers included |
| 29 | busy_threads | threads the arm keeps above 90 % CPU |
| 30-33 | stream0_p50, stream0_p99, stream0_p99.99, stream0_max | stream 0 alone, ns |

Kind `load` is a round trip from the intended send time; kind `oneway`
(same host only, same TSC) has the same columns and is one way. A transport
row has all 34 columns. `late` (column 21) counts sends more than 100 µs
after their due time.

## Manifest (`manifest-<tag>.json`)

| field | type | rule it feeds |
|---|---|---|
| `format` | 1 | |
| `set`, `arm`, `label`, `kind` (`load`/`oneway`) | string | |
| `transport` | `mpsc`, `aeron-ipc`, `aeron-spy`, `aeron-udp`, `aeron-dpdk`, `udp`, `tcp`, or several joined by `+` (`udp+tcp`: every part's loss sources) | loss accounting, no DPDK in loss rows |
| `round` | string | fairness groups points by it: one rep of one set, stream count, netem and rate, every arm |
| `rep`, `rate`, `streams`, `size`, `duration_s`, `warmup_s` | int | same schedule |
| `netem` | string or null | as the label's `@NETEM` |
| `aa`, `aa_of` | bool, string | the A/A arm repeats arm `aa_of` |
| `payload_sha256`, `wire_bytes` | string, int | same payload |
| `core_build` | string | same core build (the core binary's hash) |
| `seed`, `arm_order` | int, [arm] | randomised order |
| `scheduled`, `late`, `max_send_lag_ns` | int | lateness |
| `vms` | {host: VM id} | same VM pair |
| `cpus` | {host: {housekeeping: [cpu], irq: [cpu], hot: [cpu]}} | placement, IRQs |
| `threads` | {host: [[{tid, comm, cpu, util}]]} | samples taken during the point; `util` 0 to 1 |
| `irqs` | {host: [{irq, name, data_nic, cpu, delta}]} | growth over the point; `[]` read and none |
| `hostcheck` | {host: path relative to `<run>/<set>`} | hostcheck clean |
| `settings` | {host: {knob: value}} | read back from the host after the point |
| `netem_counters` | {host: {nic: {qdisc, loss_pct, sent_pkts, dropped}}} | netem verified; tc -s on every data NIC |
| `loss_counters` | {host: {source: {counter: growth}}} | sources `tcp_info`, `nstat`, `aeron` |
| `events` | {host: {file, rows, notes}} | the events file, its row count and what could not be recorded |

A TCP arm's own manifest (`tcp_lat`) also carries, per end, the traffic's
and the set-up's counters apart: `tcp.retrans`, `tcp.dsack_dups` (the
spurious ones), `tcp.retrans_not_dsack`, `tcp.lost_end`, `tcp.rtos`, each
with a `_setup` twin, `netstat.window.*` / `netstat.setup.*` (nstat
names), and the busy thread's `sched.run_delay_ns` (table in
`io/benches/src/transport/sock.rs`). At 0 % loss a retransmit is either
spurious (DSACK reported, the thread waited for a CPU) or a bug to trace.
transport_collect.py copies each end's into `loss_counters.<host>.tcp_info`
(`retrans`, `dsack_dups`, `retrans_not_dsack`, `lost_end`, `rtos` and the
`_setup` twins) and `netstat_window`. A TCP point's real loss is
`retrans_not_dsack` plus `lost_end` over both hosts (`transport_points.tcp_loss`);
the DSACK-reported ones are spurious, shown apart and never counted as loss.
fairness.py's `loss` rule flags real loss on a row without loss (no netem,
`none`, 0 %), and `loss-accounting` a `tcp_info` without the split.

Hosts are `a` and `b`; a one-host set (1, 2b) has `a` alone.

## transport-settings.env

```
shared.<knob>=<value>          every arm, both hosts
<arm>.<knob>=<value>           that arm only
sweep.<knob>=<arm>,<arm>,...   the arms swept for a per-arm knob
declared.<arm>=<twin>:<knob>   arm is twin but for this knob (a declared technology difference)
```

An arm reporting a knob the file does not give it, or a different value,
breaks the settings rule. transport_collect.py writes:

- `shared.<sysctl>`: every transport sysctl (`host.sh transport_sysctls`) at
  its TR_* value, each host reading them back after every point (`vm.sh
  settings`); an optional one (kernel 6.15's) that no host had at any point is
  declared `absent`. A sysctl an arm's knob sets otherwise (a tuning variant's
  congestion control; `aeron_path=kernel`'s busy poll, 0) is declared per arm
  instead, for every arm, each its own value.
- `<arm>.knob.<name>`: each per-arm knob of transport-settings.env's list
  that the arm has, as the runner ran it (`manifest/<point>.env` `knob.*`),
  read back from the binary's manifest where it reports it, or from the
  host: `knob.aeron_page`, the page size of every Aeron arm's directory
  (`vm.sh aeron-page`: 4096 on tmpfs, 2097152 on aeron-ipc-huge's hugetlbfs;
  `unread` when the read failed).
- `sweep.<knob>`: the arms a tuning session swept for it: the tuned
  section's `TR_TUNED_SWEPT`, and in a tuning session its variants.
- `declared.<arm>`: each `TR_DECLARED` arm that ran, its twin and the
  `knob.<name>` it differs in (`declared.aeron-ipc-huge=aeron-ipc:knob.aeron_page`).

The shared-knob rule: where the arms of a round run a per-arm knob at
different values, every arm of the round that has it needs a `sweep.` line
naming it (one value everywhere is no arm's own tweak); a knob is never both
`shared.` and per-arm. The A/A arm is checked against `aa_of`'s settings. A
declared arm's own knob is left out of it while its twin is in the round.

The declared rule: a declared arm runs in every round with its twin, and
nothing but its knob differs between them: every other per-arm knob, the
transport, the wire bytes, and the runner's `bin`, `args` and `driver_mode`
(`manifest-<tag>.json`'s `runner`). Its knob's value is read back like any
other (the settings rule).

## The tuning session (LAB_TUNE=1)

`transport.sh run tune-a|tune-b` (`transport_tune.py`): each arm of
`TR_SWEEP_*` (pair a: the arms of no loss set; pair b: the TCP arms and
aeron-rel) is a set of its own, `<set>.<n>`, of variants
`<arm>--<knob>-<value>`, the current value first and its A/A, at the arm's
sets' middle load and 1 and 128 streams where they have them (profile
`loss`: also under netem `TR_TUNE_NETEM`). Then `<set>.c<n>`: each knob
whose sweep found a value better than the current one, the current value,
the top two and the A/A, `TR_TUNE_CONFIRM_REPS` reps. A one-host context
(`1`, `2b`) keeps its set's one host (`<set>.<n>` is one-host too). Files:
`tune-arms.tsv`, `tune-sweep.tsv`, `variants.txt`, `tune-confirm.tsv`,
`tune-sweep.json` (each decision), `tuned.env` (the section
`transport_tune.py apply` writes into transport-settings.env, merged with
the other pair's) and `tune-report.md` (the evidence).

## Spikes file (`spikes-<tag>.csv`)

As `md-dpdk/harness/src/roundtrip.rs` writes it (`# start_realtime_ns=`,
`# spikes_total=`, `stream,due_ns,latency_ns[,ping]`), plus one line
`# max=<due_ns>,<latency_ns>,<stream>`: the point's max, kept whatever the
spike threshold. Without it the largest row is the max when it equals the
row's max column; otherwise the max is not located and is unexplained.

## Events file (`<host>-events-<tag>.csv`)

`kind,start_ns,end_ns,cpu,detail`, times on CLOCK_REALTIME, `cpu` `-` when
none, after `# note: ...` lines. transport_collect.py writes one per host
and tag from what the runner kept (`events/<point>-<host>.log.gz`: vm.sh
events-stop's output) and, on host `a`, the measuring loop's stalls file
(`transport_events.py`). A one-host point's two runs share one log.

How the runner records them, per point and host (`vm.sh events-start`
before any binary of the point starts, so before its warm-up; `vm.sh
events-stop` after its window):

| source | where it runs | what |
|---|---|---|
| tracefs instance `ergon-events` | the hot CPUs, CLOCK_MONOTONIC | sched_switch, irq and softirq entry/exit, IPIs (x86 irq_vectors, arm64 ipi_entry/exit), page faults (x86 exceptions tracepoints, else a kprobe on `handle_mm_fault`) |
| osnoise (`knoise_start`) | the hot CPUs | noise with durations, where the kernel has the tracer and NO_OSNOISE_WORKLOAD |
| gap spinner (`gapspin.c`) | each hot CPU the point leaves idle | gaps of `TR_EVENTS_GAPSPIN_US` or more |
| sampler | the housekeeping CPU, each second | tc -s of every data NIC, the TCP counters (`/proc/net/snmp`, `netstat`), the media driver's loss counters (`counters loss`, Aeron arms) |

Tracepoints and kprobes are enabled (kernel text patched, an IPI to every
CPU) only before the binaries start, never in a window. What a kernel lacks
is a note (`... unavailable: why`), as is a CPU whose trace buffer
overran.

Kinds (cpu-bound kinds only on a CPU whose busy thread is the measured
path's: the task the trace shows on it longest, not a gap spinner):

| kind | source | detail |
|---|---|---|
| `irq` | trace (x86's local timer too), osnoise | IRQ name |
| `softirq` | trace, osnoise | softirq name |
| `ipi` | trace | IPI vector or reason |
| `preempt` | trace | the busy thread, preempted or blocked, and what ran instead |
| `page-fault` | trace, the busy thread's | user/kernel and address, or the kprobe's span |
| `host-pause` | gap spinner, less what the trace shows on its CPU | µs untraced |
| `nak`, `retransmit` | the media driver's counters, each second | count |
| `tcp-rto`, `tcp-tlp`, `tcp-dsack`, `tcp-retrans` | the host's TCP counters, each second | count |
| `tc-drop` | tc -s, each second | packets netem dropped that second |
| `stall` | the measuring loop's stalls file (host `a`) | the loop and its gap |
| `dpdk-refill` | DPDK event log | descriptors |

`spikes.py` also reads `host-pause` from the diag log's `gapspin` section,
IRQs from its `trace` section, softirqs from its `knoise` section and
`refill` rows from the DPDK event log. A `stall` never labels a max: an
unexplained max names the stall it overlaps (the thread did not run, and the
guest's trace shows nothing), or what was not recorded (no events file, the
trace unavailable, an overrun). The two hosts' events are on their own
clocks: a cross-host match is as good as their CLOCK_REALTIME agreement.

## hostcheck report

ergon-tune's lines, one finding a line: `<id> ok: ...`, `<id> info: ...`,
`<id> warn: ...`, `<id> unknown: ...: <reason>`. Clean: at least one line,
no `warn`, and no `unknown` of an id that must prove something
(`tune/src/report.rs` `unknown_stops_require`).

## What the checks use

| check | threshold |
|---|---|
| busy thread | over 90 % of a CPU in a placement sample |
| alone | no other thread over 5 % on its CPU in the same sample |
| sender late | over 1 % of sends more than 100 µs late (shown, never compared) |
| over capacity | achieved under 98 % of the rate (shown, never compared) |
| netem verified | drops within 3.29 σ + 1 of `offered × loss` (99.9 %), pooled over a host's data NICs; 0 % drops nothing |
| a cause with a length | at least half of what the max adds to the p50 |
| a host pause | a spinner gap's untraced part, at least `TR_EVENTS_GAPSPIN_US` and half the gap |
| per-minute stall cluster | a max of 100 µs or more overlapping seconds :49 to :51 of its minute, nothing more specific found |
| TCP RTO/TLP | explained only with a `tc-drop` on either host in the same second |
| not separated | medians within the A/A spread (largest per-rep A/A difference), or rep ranges overlap |
