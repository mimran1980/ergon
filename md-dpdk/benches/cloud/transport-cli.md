# Transport bench CLI (what transport.sh calls)

The runner (`transport.sh`) calls each bench binary by name from the VM's
`PATH` (`~/ergon/target/release`, then `~/extra/bin`). The binaries are
`aeron_lat`, `mpsc_lat`, `udp_lat`, `tcp_lat`, `fstack_lat` and `hostcheck`, plus
any binary an arms file names. One run measures one point: one load at one stream count.
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
| `aeron_lat` | `--mode <ipc\|spy\|spy-udp\|udp>`, `--reliable <true\|false>` (`false` UDP only), `--endpoint HOST:PORT` (pings; stream i on PORT+i, stream id S+i), `--reply HOST:PORT` (echoes, stream id S+10000+i), `--interface IP` (the three UDP and spy-udp only: refused for ipc and spy). `spy-udp`: `local` measures as `spy` does, on reliable `ssc=false` publications to `--endpoint` and `--reply`, both the sink's addresses; `pong` is that sink (reads every frame, writes `sink.*` counts, echoes nothing); `ping` refused. Every manifest has the driver's `driver.bytes-sent` and `driver.bytes-received`, a local round trip's `echoes_sent`, `--aeron-dir PATH`, `--driver external\|embedded\|invoker` (a driver process at `--aeron-dir`; one this process starts on a thread of its own, configured from the environment; or one in Aeron's INVOKER mode that the measuring thread runs with its client's conductor, every poll and after every offer: `ping` and `pong` over `udp` only), `--driver-busy-threads N` (the driver's, as the runner sets it up; invoker 0, its only value), `--term-length BYTES`, `--mtu BYTES`, `--stream-id S` (1001), `--connect-timeout-s N` (30), `--round-trip` (`local` only) |
| `udp_lat` | `--impl <mio\|ergon>`, `--local IP:PORT`, `--peer IP:PORT` (base ports, stream i on +i; `local`: the receiver binds `--peer`), `--busy-read-us N` (`SO_BUSY_POLL` on every socket, both impls: ergon-io by `busy_poll_us`), `--epoll-events N`, `--datagram-burst N` (ergon only), `--rcvbuf`/`--sndbuf BYTES` |
| `tcp_lat` | `--impl <mio\|ergon>`, `--listen IP:PORT` (pong; `local` takes `--listen`, and `--source`) or `--connect IP:PORT` (ping; stream i is connection i), `--source IP` (the connecting end binds it first), `--source-port N` (an arm flag: the connecting end binds stream i to N + i and closes with a reset, so arms given the same N share 4-tuples; the listening end ignores it; not with `local`), `--nodelay` (always on), `--quickack-per-read`, `--rto-min-us N`, `--delack-max-us N`, `--busy-read-us N` (both impls), `--rcvbuf`/`--sndbuf BYTES`, `--backend epoll-spin\|uring-sqpoll` and `--sqpoll-cpu N` (ergon only: ergon-io's backend, `epoll-spin` by default; `uring-sqpoll` needs `--sqpoll-cpu`, its `SQPOLL` kernel thread's CPU, and `--busy-read-us 0`: that thread polls the NIC) |
| `fstack_lat` | TCP over F-Stack (FreeBSD's stack on DPDK), `ping` or `pong` only (one F-Stack a process, on its one `--cpus` CPU): `--listen IP:PORT` (pong) or `--source IP --connect IP:PORT` (ping), the F-Stack interface taking that address; `--pci BDF` (its port) or `--vdev SPEC` (a virtual one, tests), `--prefix-len N`, `--gateway IP`, `--nodelay` (always on), `--rcvbuf`/`--sndbuf BYTES`, `--delayed-ack true` and `--hz 100` (FreeBSD's), `--no-huge`, `--memory-mb 1024`, `--file-prefix ergon-fstack`. It refuses the kernel's socket flags (busy poll, epoll, quickack, TCP timers). It writes its whole F-Stack config to `<out>/f-stack-<role>.conf`, every key in its manifest (`fstack.conf.*`), with F-Stack's build (`fstack.build.*`). Built with `--features fstack` against `io/benches/fstack/build-fstack.sh`'s F-Stack v2.0 (x86_64 only) |
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
| `aeron_lat` | `--aeron-dir <aeron_page's directory: $TR_AERON_DIR (tmpfs), at 2097152 $TR_AERON_HUGE_DIR (2 MiB hugetlbfs), the media driver's AERON_DIR too> --driver <external; embedded or invoker with aeron_driver embedded> --driver-busy-threads <tr_driver_busy: $TR_AERON_DRIVER_BUSY_THREADS, SHARED_NETWORK one fewer, SHARED 1, INVOKER 0> --term-length <term_length> --mtu $TR_MTU`; `--mode udp` arms also `--endpoint <pong's data IP>:$TR_PORT_BASE --reply <ping's data IP>:$TR_PORT_BASE --interface <own data IP>`; every run `--stream-id 1001+256k` (the k-th Aeron point of the set; a one-host point's round trip +128), ping and pong the same, so no lingering publication of an earlier run is the image a subscription takes |
| `udp_lat` | `--local <own>:$TR_PORT_BASE --peer <other end>:$TR_PORT_BASE --rcvbuf/--sndbuf <sockbuf> --epoll-events <epoll_events> --busy-read-us <busy_read_us>`, `--datagram-burst <datagram_burst>` for `--impl ergon` |
| `tcp_lat` | pong (and `local`, on its receiver NIC) `--listen <own>:port`, ping `--source <own> --connect <peer>:port` (`local` also `--source <own>`), then `--nodelay --quickack-per-read [false] --rto-min-us <rto_min_us> --delack-max-us --rcvbuf --sndbuf <sockbuf> --epoll-events <epoll_events> --busy-read-us <busy_read_us>`; an arm with `--backend uring-sqpoll` `--busy-read-us 0` and no `busy_read_us` knob |
| `fstack_lat` | pong `--listen <own>:port`, ping `--source <own> --connect <peer>:port`, then `--pci <the NIC with that address> --prefix-len <its prefix> --gateway <its subnet's first address>` (vm.sh nics) `--nodelay --delayed-ack false --rcvbuf --sndbuf <sockbuf>` (FreeBSD's `delayed_ack=0`: the kernel arms' quickack) |
| other (arms file) | `--local <own>:port --peer <other end>:port`; setup `sockets` also `--busy-read-us $TR_BUSY_READ --rcvbuf/--sndbuf $TR_SOCKBUF --nodelay --quickack-per-read --rto-min-us $TR_TCP_RTO_MIN_US --delack-max-us $TR_TCP_DELACK_MAX_US`: every socket and TCP setting the built-in arms get. Such a binary must take them all, apply each it uses, and report them in its manifest |

The media driver's mode (`vm.sh driver`, `tr_driver_mode`): `<aeron_path>`
(kernel-busy or kernel) or dpdk, then `TR_AERON_MODE_SUFFIX`, then
`@AERON_SOCKET_SO_RCVBUF=<aeron_so_rcvbuf>`,
`@AERON_NAK_UNICAST_DELAY=<nak_delay_ns>` and
`@RUSTERON_DPDK_RX_FREE_THRESH=<rx_free_thresh>` where they differ from the
driver's own. `aeron_threading` (aeron-ipc's, and every `--mode udp` arm's):
`SHARED_NETWORK` drops the sender's and receiver's idle settings and adds
`@AERON_THREADING_MODE=SHARED_NETWORK` (the conductor keeps its core);
`SHARED` and `INVOKER` also take the conductor's core and idle setting out.
`aeron_driver=embedded` adds `@DRIVER=embedded`: `vm.sh driver` sets
everything up but starts no process, and `aeron_lat` (`--driver embedded`,
or `invoker` under INVOKER) runs with `~/bench/driver.env` sourced, under
`fstack_lat`'s `sudo setpriv` capabilities. The events' gap spinners take
the isolated CPUs the driver's threading leaves idle. A host
setting an arm's knob sets otherwise (`congestion`; the kernel path's busy
poll) goes to `vm.sh transport-net` as `TR_OVERRIDES`, and back after the
arm. Each point's `manifest/<point>.env` records `base_arm`, `bin`, `args`,
every `knob.<name>`, `driver_mode`, and for an Aeron arm `aeron_page_<host>`
(the Aeron directory's page size read back on the host: `vm.sh aeron-page`).

A declared twin (`TR_DECLARED "<arm>:<twin>:<knob>=<value>"`) is its twin's
arm row run with the twin's value of every other knob, the tuned section's
included, and its own value of the one knob: `aeron-ipc-huge` is
`aeron-ipc` with `aeron_page=2097152`. A twin may be another's:
`dpdk-aeron-rel-invoker` is `dpdk-aeron-rel-emb` (`aeron_driver=embedded`)
with `aeron_threading=INVOKER`; `fairness.py` takes each knob along the
chain as the arm's own, and compares driver modes less a driver knob's own
part (`AERON_THREADING_MODE` and the agents it drops; `DRIVER`). Its driver, `aeron_lat` and its events
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

Each data NIC has a routing table its own address selects (`vm.sh
data_kernel_up`), so the address a socket binds is the NIC it leaves by:
`udp_lat` binds `--local`, `tcp_lat`'s connecting end `--source`, its
listening end `--listen`.

Two CPU placeholders in an arm's arguments are replaced by the runner
(`tr_cpu_arg`), from the arm's CPUs (`transport_clients`), the loop's the
first: the map is the same on both hosts, so the CPU is the ping's and the
pong's alike.

- `{next_core}`: the first CPU of the next isolated core (the second of the
  arm's CPUs with SMT off, the third with it on): `eio-tcp-sqpoll-core`'s
  `--sqpoll-cpu`.
- `{loop_sibling}`: the loop CPU's SMT sibling (the second of the arm's CPUs
  with SMT on); only in a session with `smt on` (`transport.sh check`
  refuses it otherwise): `eio-tcp-sqpoll-sib`'s `--sqpoll-cpu`.

An arm's `--sqpoll-cpu` is traced in its events and never a gap spinner's;
under `smt on` neither is the SMT sibling of any CPU the point uses.

Two arm arguments are the runner's own, not passed on:

- `--nics alt`: the arm on each host's other data NIC (ping: a's receiver
  NIC to b's sender NIC; `local`: its receiver NIC to its sender NIC).
- `{own_alt}` and `{peer_alt}`: replaced by the address of the other data
  NIC (the one the arm's addresses do not use) of the end's own host and
  of the peer's, so a binary can run a second path on the other NICs.

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
| `onload_cmd`, `onload_under` | setup `onload`: what the arm ran under (`env <TR_ONLOAD_ENV> onload --profile=<TR_ONLOAD_PROFILE>`), and which process (the binary, or `media_driver` for Aeron) |
| `onload_build_a`, `onload_build_b` | setup `onload`: the Onload build on each host (`vm.sh onload-info`'s first line, read as the session starts) |
| `onload_a`, `onload_b` | setup `onload`: `ok ...` when an Onload stack of the arm's process was there mid-window (`vm.sh onload-stacks`), else `FAIL ...` (fairness.py's onload rule) |

The row the binary writes gets the rep in its first column (the binary is
not given it), and goes to `results.csv`.

## Arms files (LAB_ARMS_FILE)

Same format as `transport-matrix.tsv`: `arm` and `set` rows, added after the
built-in ones. A name may not be used twice. `LAB_EXTRA_SRC` is a source tree
copied to `~/extra` on both VMs, which then run `bash lab-build.sh ~/extra/bin`
in it after the main build. Binaries placed there are found by name.

## SMT on (`smt on`)

A sets file's `smt on` row (`transport_plan.py smt`; off without one)
makes the session keep SMT on: `vm.sh transport-isolate smt-on` builds the
CPU map with each isolated core whole, its first thread then its siblings
(c7i.4xlarge: `0 1 2,10,3,11,4,12,5,13,6,14,7,15`), and `transport-tune`
verifies SMT on (`host.sh transport_smt_on`) where every other session turns
it off. `transport.sh check` takes only two-host socket arms then (a media
driver's threads, or a one-host arm's two ends, would share a core), and
`aws/lab.sh` refuses it with `LAB_THREADS_PER_CORE=1`, on Graviton and on
C7a-family types.

| what | how |
|---|---|
| declared | `devices/system/cpu/smt/active` 1 (0 in every other session) in `declared-sysctls.txt`; `vm.sh settings` reads it back at every point (`transport_smt_read`), fairness.py's settings rule |
| hostcheck | `hostcheck --smt on`: of each pair of hot SMT siblings the first thread a loop and the other its `SQPOLL` thread (T8), T10 reported, not blocking; its first line `S1 info:` records it |
| fairness | T10 (the hostcheck's, and a binary's own) reported only under `smt on`; the `smt` rule: each busy thread's sibling (its neighbour in the hot list) idle, or the arm's own `SQPOLL` thread (`iou-sqp-*`) |

`transport-aws-smt-off.tsv` and `transport-aws-smt.tsv` are the two halves
of the SMT question, each a session of its own on c7i.4xlarge.

## Setup fstack (`dpdk-tcp`)

No media driver. `vm.sh fstack-up` moves both data NICs from `ena` to
`igb_uio` (as `driver dpdk` does) after checking 512 free 2 MiB pages;
`fstack_lat` takes its port by PCI address, so the ping holds host a's
sender NIC and the pong host b's receiver NIC, the kernel TCP arms' path.
`fstack_lat` runs under `sudo setpriv` (`tr_fstack_priv`): the user's own uid
with ambient `CAP_SYS_ADMIN` (igb_uio has no IOMMU, so the EAL needs physical
addresses from `/proc/self/pagemap`; without it the EAL stops with
`Cannot use IOVA as 'PA'`), `CAP_DAC_OVERRIDE` (`/dev/uioN`, the BARs' sysfs
files, `/dev/hugepages`), `CAP_IPC_LOCK` and `CAP_SYS_RAWIO`. `fstack_lat`
refuses to start without the first two. Every binary's exit status lands in
`<pid file>.rc`; a failed point's log line gives it and the binary's error lines.
`tr_arm_down` runs `vm.sh stop`, which returns the NICs to the kernel and
removes F-Stack's leftover hugepage files. AWS x86_64 only (`modes.sh`
refuses it elsewhere before any cloud call). A set with a `dpdk` or
`fstack` arm has netem `-` (`transport_plan.py` refuses another level).
`transport-aws-dpdk.tsv` is the AWS DPDK session's `LAB_SETS_FILE`
(`aws/lab.sh LAB_SUITE=transport LAB_PAIR=a`, two c7a.2xlarge).
`transport-aws-fstack-smoke.tsv` is its short check at `LAB_REPS=1`: both DPDK
arms (`dpdk-tcp`, `dpdk-aeron-rel`) beside `mio-tcp` and the A/A arm, one
stream at 10k and 50k. A failed set-up's run.log line gives the step's error
lines (vm.sh names any step that fails under `set -e`) and, under `kernel` or
`dpdk`, keeps the media driver's log as `logs/driver-<host>-<n>.log`.
`vm.sh transport-build fstack` (when `tr_wants_fstack`) builds F-Stack under
`~/fstack` and `fstack_lat`.

## Setup onload (`onload-*`)

OpenOnload over AF_XDP on the ENA, AWS only (x86_64 and Graviton). The
matrix's `onload-mio-tcp`, `onload-eio-tcp`, `onload-mio-udp`,
`onload-eio-udp`, `onload-aeron-udp` and `onload-aeron-rel` are the kernel
arms' binaries and arguments; only the stack differs.

| what | how |
|---|---|
| build | `vm.sh transport-build onload` (when a set runs an Onload arm): `onload-build.sh`, Xilinx-CNS/onload at `TR_ONLOAD_COMMIT` with `onload-kernel-7.2.patch`, `--no-sfc`, installed without boot-time module loading |
| boot line | `vm.sh transport-isolate onload` adds `ibt=off` on a CPU with IBT (Intel): Onload's module calls `bpf()` through the syscall table to set up AF_XDP |
| NICs | `vm.sh onload-up` (`driver onload` runs it first): the kernel path as `transport-net` sets it, the modules loaded, AF_XDP flow filters off, each data NIC at 1 queue, MTU at most `TR_ONLOAD_MTU`, interrupts not deferred, registered, and its XDP program read back (maxmtu drops to 3498); `stop` unregisters and puts queues and MTU back |
| socket arms | the binary under `env TR_ONLOAD_ENV onload --profile=TR_ONLOAD_PROFILE`; `--busy-read-us 0`; no `--rto-min-us` or `--delack-max-us` (Onload has neither option; `EF_RFC_RTO_MIN` instead) |
| Aeron arms | `aeron_lat` as on the kernel; its media driver under onload (`vm.sh driver onload@...`) |
| knobs | the kernel arm's knobs Onload honours, and `napi_defer` at `TR_ONLOAD_NAPI_DEFER` (read back) |
| one stack | each data NIC has one queue, so one AF_XDP socket and one Onload stack: before each point of a socket arm `vm.sh onload-idle` kills orphan stacks (a closed TCP connection's TIME_WAIT keeps one) and waits up to 30 s for none, else the point fails; `stop` unregisters only with no stack left, else fails |
| check | `vm.sh onload-stacks <process>` mid-window on each host; `EF_NO_FAIL=0` makes a socket Onload cannot accelerate fail instead of falling back |

No netem (`transport_plan.py` refuses it): Onload's AF_XDP sends leave by
`__dev_direct_xmit`, past the qdisc. `transport-aws-onload.tsv` is the
session's `LAB_SETS_FILE` (`aws/lab.sh LAB_SUITE=transport LAB_PAIR=a`, on
c7i.4xlarge, c8g.2xlarge with `LAB_ALLOW_ARM=1`, or c7a.2xlarge);
`transport-aws-onload-smoke.tsv` runs every Onload arm once first
(`LAB_REPS=1`).
