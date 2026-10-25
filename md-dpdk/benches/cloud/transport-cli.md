# Transport bench CLI (what transport.sh calls)

The runner (`transport.sh`) calls each bench binary by name from the VM's
`PATH` (`~/ergon/target/release`, then `~/extra/bin`). The binaries are
`aeron_lat`, `mpsc_lat`, `udp_lat`, `tcp_lat` and `hostcheck`, plus any binary
an arms file names. One run measures one point: one load at one stream count.
`test/test-transport.sh` runs the runner against stubs that follow this file.

## Common flags (every latency binary)

| flag | meaning |
|---|---|
| `--role <ping\|pong\|local>` | `ping` on host a, `pong` on host b; `local`: both ends in one process (IPC, same-host arms), one way (an `oneway` row) or with `--round-trip` (a `load` row). The binaries refuse `oneway-send` and `oneway-recv` |
| `--streams N` | streams (UDP, Aeron) or connections (TCP) |
| `--rate MSG_PER_S` | total over every stream |
| `--warmup-s S` | unrecorded warm-up at the rate |
| `--duration-s S` | the measured window |
| `--size 64` | message bytes, the same in every arm |
| `--cpus CSV` | one isolated CPU per busy thread, in the binary's documented thread order. The binary is started on the housekeeping CPU (`taskset`): its Aeron client conductor keeps that affinity |
| `--label TEXT` | written into every row |
| `--out DIR` | where its outputs go |
| `--spike-ns N`, `--stall-ns N` | thresholds for the spike and stall files |

Outputs in `--out`: `results.csv` (one core `Row` per point: the 22 `load`
columns, then `min,lost,reordered,dup,retrans,wire_bytes,busy_threads,stream0_p50,stream0_p99,stream0_p99.99,stream0_max`),
`manifest.json` (core `RunManifest` and every setting),
`histogram-<label>.csv`, `spikes-<label>.csv` and `stalls-<label>.csv`. A
nonzero exit on any error. A partial row is never printed.

## Per binary

| binary | flags |
|---|---|
| `mpsc_lat` | `--capacity N` (sync_channel); role `local` only, `--round-trip` for the round trip |
| `aeron_lat` | `--mode <ipc\|spy\|udp>`, `--reliable <true\|false>`, `--endpoint HOST:PORT` (pings; stream i on PORT+i, stream id S+i), `--reply HOST:PORT` (echoes, stream id S+10000+i), `--interface IP`, `--aeron-dir PATH`, `--driver external\|embedded`, `--driver-busy-threads N` (the external driver's, as the runner starts it), `--term-length BYTES`, `--mtu BYTES`, `--stream-id S` (1001), `--connect-timeout-s N` (30), `--round-trip` (`local` only) |
| `udp_lat` | `--impl <mio\|ergon>`, `--local IP:PORT`, `--peer IP:PORT` (base ports, stream i on +i), `--busy-read-us N` (mio), `--epoll-events N`, `--datagram-burst N` (ergon), `--rcvbuf`/`--sndbuf BYTES` |
| `tcp_lat` | `--impl <mio\|ergon>`, `--listen IP:PORT` or `--connect IP:PORT` (stream i is connection i), `--nodelay` (always on), `--quickack-per-read`, `--rto-min-us N`, `--delack-max-us N`, `--busy-read-us N`, `--rcvbuf`/`--sndbuf BYTES` |
| `hostcheck` | `--hot-cpus CSV`, `--out FILE`, `--nic IFACE,...`, `--aeron-dir PATH`; one finding a line (`<id> ok:`, `info:`, `warn:`, `unknown:`); exit 0 clean, 2 not clean, 1 not run |

## What the runner passes

The arm's own flags come from its `arm` row in `transport-matrix.tsv`
(`--mode`, `--reliable`, `--impl`). Each host's flags are the same for every
arm that runs the binary. They are set in `transport-settings.env` (`TR_*`) and
built by `transport.sh tr_bin_args`:

| binary | runner's flags |
|---|---|
| `mpsc_lat` | `--capacity $TR_MPSC_CAPACITY` |
| `aeron_lat` | `--endpoint <pong's data IP>:$TR_PORT_BASE --reply <ping's data IP>:$TR_PORT_BASE --interface <own data IP> --aeron-dir $TR_AERON_DIR --driver external --driver-busy-threads $TR_AERON_DRIVER_BUSY_THREADS --term-length $TR_TERM_LENGTH --mtu $TR_MTU` |
| `udp_lat` | `--local <own>:$TR_PORT_BASE --peer <other end>:$TR_PORT_BASE --busy-read-us $TR_BUSY_READ --rcvbuf/--sndbuf $TR_SOCKBUF` |
| `tcp_lat` | pong `--listen <own>:port`, ping `--connect <peer>:port`, then `--nodelay --quickack-per-read --rto-min-us --delack-max-us --busy-read-us --rcvbuf --sndbuf` |
| other (arms file) | `--local <own>:port --peer <other end>:port` |

Addresses: ping binds host a's sender NIC and talks to host b's receiver NIC.
Pong binds b's receiver NIC and talks to a's sender NIC. `local` binds its
host's sender NIC and talks to its receiver NIC.

Ports: `TR_PORT_BASE` is 22000, and stream i uses base + i, within
`TR_PORT_SPAN` (1024). That range stays clear of the media driver's wildcard
ranges (20000-21999). `vm.sh transport-tune` checks it against each host's
ephemeral range (`ip_local_port_range`), and fails if they meet.

A one-host arm's point is two runs, one after the other: `--role local` (one
way, on one TSC) and `--role local --round-trip`. The point's snapshots
cover both runs.

In every run `--warmup-s` is 30 for an arm and stream count's
first load and 5 (the switch) for each later one, `--duration-s` 30.

## Lifecycle

Each binary starts as `sh -c 'echo $$ >run/<x>.pid; exec taskset -c <housekeeping> <binary> ...'`.
The runner's exit stops binaries by the PIDs recorded this way, never by
name.

1. The runner starts the pong on b (`nohup`).
2. It waits `TRANSPORT_PONG_WAIT_S` (2 s) and checks the pong is still alive.
3. It runs the ping on a in the foreground.
4. It sends SIGTERM to the pong, then SIGKILL after 5 s.

The convention has no ready signal. A pong that binds slower than 2 s makes
the ping fail, and the runner counts that point as failed. A `READY` line on
the pong's stdout would let the runner wait exactly.

hostcheck runs once per session per host (`vm.sh hostcheck`, its report saved
as `hostcheck-<a|b>.txt`). A nonzero exit, or any `warn:` line, stops the
session before the first point.

## Per-point manifest (written by the runner)

`manifest/<point>.env` holds one `key=value` per line, where `<point>` is
`<set>-r<rep>-<mode>-<load>`. The binary's own `manifest.json` is saved next
to it as `manifest/<point>.json` (a one-host point's round-trip run as
`manifest/<point>1.json`).

| key | value |
|---|---|
| `point`, `set`, `rep`, `round`, `arm`, `mode`, `streams`, `load`, `warmup_s`, `netem`, `aa` | what ran |
| `seed`, `order`, `position` | the round's seed, its arm order, this arm's place in it |
| `session`, `pair`, `vm_a`, `vm_b` | session stamp, LAB_PAIR, the VMs (`<group>/a`, `<group>/b`) |
| `bin`, `bin_sha256_<host>`, `commit`, `dirty` | the binary on each host and the build |
| `settings_sha256` | sha256 of transport-settings.env as applied |
| `start_ns`, `end_ns` | the run's wall-clock start and end (CLOCK_REALTIME) |
| `irq_hot_a`, `irq_hot_b` | each isolated CPU's interrupts over the run, `cpu<n>=<all>/<device IRQs>`, or `unused` |
| `placement_a`, `placement_b` | `ok <n> busy threads`, or `FAIL` and what broke: a busy thread off the isolated CPUs, or two on one CPU |
| `netem_a`, `netem_b` | `none`, or `<offered>/<dropped> p=<pct> in [<lo>,<hi>] ok`, or the same ending `FAIL` |
| `hostcheck_a`, `hostcheck_b` | the session's report files |

The row the binary writes gets the rep in its first column (the binary is
not given it), and goes to `results.csv`.

## Arms files (LAB_ARMS_FILE)

Same format as `transport-matrix.tsv`: `arm` and `set` rows, added after the
built-in ones. A name may not be used twice. `LAB_EXTRA_SRC` is a source tree
copied to `~/extra` on both VMs, which then run `bash lab-build.sh ~/extra/bin`
in it after the main build. Binaries placed there are found by name.
