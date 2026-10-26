# Transport bench CLI (what transport.sh calls)

The runner (`transport.sh`) calls each bench binary by name from the VM's
`PATH` (`~/ergon/target/release`, then `~/extra/bin`). The binaries are
`aeron_lat`, `mpsc_lat`, `udp_lat`, `tcp_lat` and `hostcheck`, plus any binary
an arms file names. One run measures one point: one load at one stream count.
`test/test-transport-runner.sh` runs the runner against stubs that follow this file.

## Common flags (every latency binary)

| flag | meaning |
|---|---|
| `--role <ping\|pong\|local>` | `ping` on host a, `pong` on host b; `local`: both ends in one process (IPC, same-host arms), one way (an `oneway` row) or with `--round-trip` (a `load` row). The binaries refuse `oneway-send` and `oneway-recv` |
| `--streams N` | streams (UDP, Aeron) or connections (TCP) |
| `--rate MSG_PER_S` | total over every stream |
| `--warmup-s S` | unrecorded warm-up at the rate |
| `--duration-s S` | the measured window |
| `--size 64` | message bytes, the same in every arm |
| `--cpus CSV` | one isolated CPU per busy thread, in the binary's documented thread order: `local` two (sender, receiver; with `--round-trip` ping, echo), `ping` and `pong` one. The binary refuses any other count. The binary is started on the housekeeping CPU (`taskset`): its Aeron client conductor keeps that affinity |
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
| `aeron_lat` | `--mode <ipc\|spy\|udp>`, `--reliable <true\|false>` (`false` UDP only), `--endpoint HOST:PORT` (pings; stream i on PORT+i, stream id S+i), `--reply HOST:PORT` (echoes, stream id S+10000+i), `--interface IP` (the three UDP only: refused for ipc and spy), `--aeron-dir PATH`, `--driver external\|embedded`, `--driver-busy-threads N` (the external driver's, as the runner starts it), `--term-length BYTES`, `--mtu BYTES`, `--stream-id S` (1001), `--connect-timeout-s N` (30), `--round-trip` (`local` only) |
| `udp_lat` | `--impl <mio\|ergon>`, `--local IP:PORT`, `--peer IP:PORT` (base ports, stream i on +i; `local`: the receiver binds `--peer`), `--busy-read-us N` (mio only: ergon-io refuses it), `--epoll-events N`, `--datagram-burst N` (ergon only), `--rcvbuf`/`--sndbuf BYTES` |
| `tcp_lat` | `--impl <mio\|ergon>`, `--listen IP:PORT` (pong; `local` takes `--listen` alone) or `--connect IP:PORT` (ping; stream i is connection i), `--nodelay` (always on), `--quickack-per-read`, `--rto-min-us N`, `--delack-max-us N`, `--busy-read-us N` (mio only), `--rcvbuf`/`--sndbuf BYTES` |
| `hostcheck` | `--hot-cpus CSV`, `--out FILE`, `--nic IFACE,...`, `--aeron-dir PATH`; one finding a line (`<id> ok:`, `info:`, `warn:`, `unknown:`); exit 0 clean, 2 not clean, 1 not run |

## What the runner passes

The arm's own flags come from its `arm` row in `transport-matrix.tsv`
(`--mode`, `--reliable`, `--impl`). Each host's flags are the same for every
arm that runs the binary. They are set in `transport-settings.env` (`TR_*`) and
built by `transport.sh tr_bin_args`; a per-arm knob's value (`<knob>` below:
transport-settings.env's list) is `tr_knob`'s: the tuned section's
`TR_ARM_KNOBS` for the arm, else the knob's default (a tuning variant: its
own):

| binary | runner's flags |
|---|---|
| `mpsc_lat` | `--capacity <capacity>` |
| `aeron_lat` | `--aeron-dir <aeron_page's directory: $TR_AERON_DIR (tmpfs), at 2097152 $TR_AERON_HUGE_DIR (2 MiB hugetlbfs), the media driver's AERON_DIR too> --driver external --driver-busy-threads $TR_AERON_DRIVER_BUSY_THREADS (1 under SHARED) --term-length <term_length> --mtu $TR_MTU`; `--mode udp` arms also `--endpoint <pong's data IP>:$TR_PORT_BASE --reply <ping's data IP>:$TR_PORT_BASE --interface <own data IP>`; every run `--stream-id 1001+256k` (the k-th Aeron point of the set; a one-host point's round trip +128), ping and pong the same, so no lingering publication of an earlier run is the image a subscription takes |
| `udp_lat` | `--local <own>:$TR_PORT_BASE --peer <other end>:$TR_PORT_BASE --rcvbuf/--sndbuf <sockbuf> --epoll-events <epoll_events>`, `--busy-read-us <busy_read_us>` for `--impl mio`, `--datagram-burst <datagram_burst>` for `--impl ergon` |
| `tcp_lat` | pong (and `local`, on its receiver NIC) `--listen <own>:port`, ping `--connect <peer>:port`, then `--nodelay --quickack-per-read [false] --rto-min-us <rto_min_us> --delack-max-us --rcvbuf --sndbuf <sockbuf> --epoll-events <epoll_events>`, and `--busy-read-us` for `--impl mio` |
| other (arms file) | `--local <own>:port --peer <other end>:port`; setup `sockets` also `--busy-read-us $TR_BUSY_READ --rcvbuf/--sndbuf $TR_SOCKBUF --nodelay --quickack-per-read --rto-min-us $TR_TCP_RTO_MIN_US --delack-max-us $TR_TCP_DELACK_MAX_US`: every socket and TCP setting the built-in arms get. Such a binary must take them all, apply each it uses, and report them in its manifest |

The media driver's mode (`vm.sh driver`, `tr_driver_mode`): `<aeron_path>`
(kernel-busy or kernel) or dpdk, then `TR_AERON_MODE_SUFFIX`, then
`@AERON_SOCKET_SO_RCVBUF=<aeron_so_rcvbuf>`,
`@AERON_NAK_UNICAST_DELAY=<nak_delay_ns>` and
`@RUSTERON_DPDK_RX_FREE_THRESH=<rx_free_thresh>` where they differ from the
driver's own; aeron-ipc's `aeron_threading=SHARED` takes the conductor's core
and idle settings out and adds `@AERON_THREADING_MODE=SHARED`. A host
setting an arm's knob sets otherwise (`congestion`; the kernel path's busy
poll) goes to `vm.sh transport-net` as `TR_OVERRIDES`, and back after the
arm. Each point's `manifest/<point>.env` records `base_arm`, `bin`, `args`,
every `knob.<name>`, `driver_mode`, and for an Aeron arm `aeron_page_<host>`
(the Aeron directory's page size read back on the host: `vm.sh aeron-page`).

A declared twin (`TR_DECLARED "<arm>:<twin>:<knob>=<value>"`) is its twin's
arm row run with the twin's value of every other knob, the tuned section's
included, and its own value of the one knob: `aeron-ipc-huge` is
`aeron-ipc` with `aeron_page=2097152`. Its driver, `aeron_lat` and its events
use `$TR_AERON_HUGE_DIR`, on the hugetlbfs `vm.sh transport-tune` mounts at
`$TR_AERON_HUGE_MOUNT` on each host (`TR_AERON_HUGE_PAGES` pages of 2 MiB,
reserved with `min_size`; refused when the kernel has fewer free; verified
from `/proc/mounts`: `hugetlbfs`, `pagesize=2M`, its size). When a set of
the session runs such an arm, the session also runs `hostcheck --aeron-dir
$TR_AERON_HUGE_DIR` on each host (`hostcheck-<host>-hugepages.txt`, its
points' report). `transport.sh check` refuses a twin whose arm row differs,
a knob that is not the binary's, and a hugetlbfs smaller than twice what the
arm holds at once (`host.sh transport_huge_pages`).

Addresses: ping binds host a's sender NIC and talks to host b's receiver NIC.
Pong binds b's receiver NIC and talks to a's sender NIC. `local` binds its
host's sender NIC and talks to its receiver NIC.

Ports: `TR_PORT_BASE` is 22000, and stream i uses base + i, within
`TR_PORT_SPAN` (1024). That range stays clear of the media driver's wildcard
ranges (20000-21999), which `aeron_lat`, `udp_lat` and `tcp_lat` refuse.
`vm.sh transport-tune` checks it against each host's ephemeral range
(`ip_local_port_range`), and fails if they meet.

Labels: `<set>:<arm>@STREAMS=<n>[@NETEM=<pct>]`, the A/A arm's arm `aa`
(transport-format.md). `--cpus` comes from `host.sh transport_clients`: the
first CPU (ping, pong) or two (local) of the arm's.

Build (`vm.sh transport-build`): `aeron_lat` and `hostcheck` in the root
workspace with io/benches's release profile (`panic = "abort"`, debug info),
the socket and mpsc binaries by `--manifest-path io/benches/Cargo.toml
--target-dir target` (io/benches is its own workspace), so all land in
`target/release`. Under the transport map the media driver runs with umask
0, so the user's bench binaries can map its files.

A one-host arm's point is two runs, one after the other: `--role local` (one
way, on one TSC) and `--role local --round-trip`. The point's snapshots
cover both runs.

In every run `--warmup-s` is 30 for an arm and stream count's
first load and 5 (the switch) for each later one, `--duration-s` 30.

## Lifecycle

Each binary starts as `sh -c 'echo $$ >run/<x>.pid; exec taskset -c <housekeeping> <binary> ...'`.
The runner's exit stops binaries by the PIDs recorded this way, never by
name.

0. On each host the point uses, `vm.sh events-start` (transport-format.md's
   events file): the trace, osnoise, the gap spinners on the hot CPUs the
   point leaves idle, the 1 s sampler on the housekeeping CPU, before any
   binary of the point starts. `vm.sh events-stop` reads them back after
   the window (before the pong is stopped) into
   `events/<point>-<host>.log.gz`.
1. The runner starts the pong on b (`nohup`).
2. It waits for the pong's one `ready` line on stdout: `udp_lat` once its
   sockets are bound, `tcp_lat` once it listens, `aeron_lat` once its
   subscriptions and publications are added. A pong that exits first, says
   it twice, or says nothing within `TRANSPORT_READY_TIMEOUT_S` (60 s) fails
   the point; the ping is not started.
3. It runs the ping on a in the foreground.
4. It sends SIGTERM to the pong, then SIGKILL after 5 s, and keeps its
   `--out` (its manifest: host b's TCP_INFO and Aeron counters) as
   `points/<point>/pong`.

Rows: the runner puts the rep before the binary's row and writes its
channel column as `<channel>-r<rate>` (transport-format.md's point column).
As the session ends `transport_collect.py` lays the directory out for
fairness.py and analyse.py (transport-format.md).

hostcheck runs once per session per host (`vm.sh hostcheck`, its report saved
as `hostcheck-<a|b>.txt`), and again on `$TR_AERON_HUGE_DIR` when an arm of
the session runs there (`vm.sh hostcheck <dir>`, `hostcheck-<a|b>-hugepages.txt`).
A nonzero exit, or any `warn:` line, stops the session before the first point.

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
| `args` | the arm row's arguments |
| `base_arm`, `knob.<name>`, `driver_mode` | the arm's per-arm knobs as run, its media driver mode (Aeron arms) |
| `aeron_page_a`, `aeron_page_b` | Aeron arms: the page size of the arm's Aeron directory read back on the host (`vm.sh aeron-page`), or `unread` |
| `settings_sha256` | sha256 of transport-settings.env as applied |
| `start_ns`, `end_ns` | the run's wall-clock start and end (CLOCK_REALTIME) |
| `irq_hot_a`, `irq_hot_b` | each isolated CPU's interrupts over the run, `cpu<n>=<all>/<device IRQs>`, or `unused` |
| `placement_a`, `placement_b` | `ok <n> busy threads`, or `FAIL` and what broke: a busy thread off the isolated CPUs, or two on one CPU |
| `netem_a`, `netem_b` | `none`, or `<offered>/<dropped> p=<pct> in [<lo>,<hi>] ok`, or the same ending `FAIL` |
| `hostcheck_a`, `hostcheck_b` | the session's report file that covers the arm on each host (`hostcheck-<host>-hugepages.txt` for an arm on the hugetlbfs) |

The row the binary writes gets the rep in its first column (the binary is
not given it), and goes to `results.csv`.

## Arms files (LAB_ARMS_FILE)

Same format as `transport-matrix.tsv`: `arm` and `set` rows, added after the
built-in ones. A name may not be used twice. `LAB_EXTRA_SRC` is a source tree
copied to `~/extra` on both VMs, which then run `bash lab-build.sh ~/extra/bin`
in it after the main build. Binaries placed there are found by name.
