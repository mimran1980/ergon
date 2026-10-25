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
```

An arm reporting a knob the file does not give it, or a different value,
breaks the settings rule. transport_collect.py writes `shared.` lines alone:
every transport sysctl (`host.sh transport_sysctls`) at its TR_* value, each
host reading them back after every point (`vm.sh settings`); an optional one
(kernel 6.15's) that no host had at any point is declared `absent`. The shared-knob rule: a per-arm knob needs a
`sweep.` line naming that arm and every other arm of the round that reports
the knob, and a knob is never both `shared.` and per-arm. The A/A arm is
checked against `aa_of`'s settings.

## Spikes file (`spikes-<tag>.csv`)

As `md-dpdk/harness/src/roundtrip.rs` writes it (`# start_realtime_ns=`,
`# spikes_total=`, `stream,due_ns,latency_ns[,ping]`), plus one line
`# max=<due_ns>,<latency_ns>,<stream>`: the point's max, kept whatever the
spike threshold. Without it the largest row is the max when it equals the
row's max column; otherwise the max is not located and is unexplained.

## Events file (`<host>-events-<tag>.csv`)

`kind,start_ns,end_ns,cpu,detail`, times on CLOCK_REALTIME, `cpu` `-` when
none. Kinds:

| kind | source | detail |
|---|---|---|
| `irq` | trace, per CPU | IRQ name |
| `softirq` | trace / osnoise | softirq name |
| `nak`, `retransmit` | Aeron counters, sampled | count |
| `tcp-rto`, `tcp-tlp` | TCP_INFO / nstat, sampled | count |
| `tc-drop` | tc -s, each second | packets netem dropped that second |
| `page-fault` | ergon-tune T19 FaultWatch | faults |
| `dpdk-refill` | DPDK event log | descriptors |
| `host-pause` | gap spinner | µs |

`spikes.py` also reads `host-pause` from the diag log's `gapspin` section,
IRQs from its `trace` section, softirqs from its `knoise` section and
`refill` rows from the DPDK event log.

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
| per-minute stall cluster | a max of 100 µs or more overlapping seconds :49 to :51 of its minute, nothing more specific found |
| TCP RTO/TLP | explained only with a `tc-drop` on either host in the same second |
| not separated | medians within the A/A spread (largest per-rep A/A difference), or rep ranges overlap |
