# ergon-md-dpdk

[![Book](https://img.shields.io/badge/book-mimran1980.github.io%2Fergon-blue)](https://mimran1980.github.io/ergon/md-dpdk/overview.html)

> **Not recommended for production use.**
>
> This crate is a proof of concept: a kernel-bypass UDP transport for Aeron,
> written to learn how one is put together. It is verified by the `dpdk` test
> lane — 15 test files, each under no features, `dpdk` and `static,dpdk` —
> including `dpdk_interop`'s interoperability tests (byte-identical payloads,
> the connected/unconnected addressing rules, port-0 handling, reconnect,
> interceptors and role-frame dispatch), plus the ENA acceptance script
> (the repository's `scripts/aws-dpdk-acceptance.sh`) and the Azure benchmark lab
> (`benches/cloud/azure/lab.sh`) on real instances.
> [BENCHMARKS.md](BENCHMARKS.md) reports the measurements. What it lacks is
> the edge-case handling a production transport accumulates over years.
>
> **If you need kernel bypass for Aeron in production, use
> [Aeron Premium](https://aeron.io/),** which is supported. No kernel-bypass
> product was compared on this hardware, so no speed claim is made either way.
>
> **On one host it gains nothing:** host-local channels run on ordinary kernel
> sockets in this driver too ([Host-local channels](#host-local-channels)).

An experimental DPDK kernel-bypass UDP transport for the
[rusteron](https://crates.io/crates/rusteron-media-driver) Aeron media driver,
on AWS (ENA) and Azure (MANA), Linux x86_64 and aarch64 (Graviton).
Measurements against kernel UDP: [BENCHMARKS.md](BENCHMARKS.md). A plain-English
guide, with which transport to use when:
[the book](https://mimran1980.github.io/ergon/md-dpdk/overview.html).
The Kubernetes reference (driver on two exclusive CPUs plus a shared one,
Aeron apps, node settings, a layout verifier) is the
[low-latency-k8s sample](../samples/README.md#low-latency-k8s).

> **AI assistance.** Like the rest of ergon, this crate was written **with heavy
> AI assistance**. See
> [AI assistance disclosure](https://mimran1980.github.io/ergon/project/ai-assistance.html).
> It is not production-ready.

- Aeron's C media driver sends and receives through a table of function
  pointers, `aeron_udp_channel_transport_bindings_t`. This crate's C fills it
  in on DPDK and installs it into an `AeronDriverContext`.
- One NIC for the sender agent, one for the receiver. The conductor still
  resolves on the kernel socket; the VM's primary NIC stays with the kernel.
- Needs rusteron-media-driver 0.2.13 or newer (`Cargo.toml`), which exports the headers of the
  Aeron it builds (`DEP_AERON_DRIVER_*`), so this C compiles against exactly
  that driver.

| Cloud | DPDK driver | Port selected by |
|---|---|---|
| AWS | `net_ena`, the ENA bound to `igb_uio` (`wc_activate=1`) or a write-combining `vfio-pci` | its PCI address |
| Azure | `net_netvsc` over the MANA VF. `net_mana` alone is accepted, but no committed run shows it receiving | its MAC (`RUSTERON_DPDK_*_MAC`): every accelerated NIC of a VM is a vport on one PCI function |

## Requirements

- **`dpdk` feature:** Linux x86_64 or aarch64, `libdpdk >= 23.11` via
  pkg-config (`libdpdk-dev` on Ubuntu 24.04 or Debian 13; from source on
  Amazon Linux 2023, as `deploy/aws-eks/Dockerfile.media-driver` does). Both
  architectures are tested against the fakes in Docker
  (`just verify-dpdk-docker`, `--arm64`).
- **Without the feature** the crate builds everywhere, and selecting
  `dpdk-ena` returns `DpdkError::FeatureDisabled`.
- **`static`** links Aeron's C driver statically (rusteron's `static`).
- **Graviton: build for the CPU,** or Ubuntu's and Debian's gcc turn every
  atomic add on the send and receive paths into a call to libgcc's
  outline-atomics helper instead of one `ldadd`. C: `CFLAGS=-mcpu=native`
  (as the Graviton runs did) or the core: `neoverse-v1` (Graviton3), `-v2`
  (Graviton4), `-v3` (Graviton5). Rust: `RUSTFLAGS=-Ctarget-cpu=...`.
  `CFLAGS` does not reach Aeron's own C, which
  rusteron builds for the build host's CPU (`-march=native`) unless
  `RUSTERON_C_MARCH` names another.

## Selecting the transport

```sh
RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena cargo run -p ergon-md-dpdk --features dpdk --bin media_driver
```

| `RUSTERON_MEDIA_DRIVER_TRANSPORT` | result |
|---|---|
| unset or `default` | the socket transport, unchanged |
| `dpdk-ena` | the DPDK transport; any failure exits non-zero, never falling back to the socket driver |
| anything else | invalid, exits non-zero |

- **Required:** `RUSTERON_DPDK_FILE_PREFIX`, `RUSTERON_DPDK_SENDER_PCI`,
  `RUSTERON_DPDK_SENDER_IPV4_CIDR`, `RUSTERON_DPDK_SENDER_GATEWAY`, and their
  `RUSTERON_DPDK_RECEIVER_*` twins (`src/env.rs`). The names are rusteron's,
  so deployments for rusteron's DPDK branch work unchanged.
- **`RUSTERON_DPDK_EAL_ARGS`** appends EAL options (log levels, `-l <cpu>`),
  at most 1 KiB with the device arguments.
- **`RUSTERON_DPDK_EAL_IN_MEMORY=1`** (pod env; default `0`) starts the EAL
  `--in-memory` instead of with `--huge-dir`: no shared configuration, no
  multi-process channel thread, and no hugepage files left in hugetlbfs after
  an OOM kill. The pod still needs its `hugepages-2Mi` limit; the hugetlbfs
  mount is then only Aeron's. EAL options that contradict the choice
  (`--huge-dir`, `--no-huge`, a `--proc-type` other than `primary` or
  `--huge-unlink=never` with it; `--in-memory` or `--no-shconf` without it)
  fail start with `DpdkError::InvalidEnvironment` naming both variables. Its
  latency effect is not measured here.
- **`RUSTERON_DPDK_HUGE_DIR`**: the hugetlbfs mount DPDK uses, an absolute
  path; `/dev/hugepages` by default.
- **`RUSTERON_MEDIA_DRIVER_DIR`**: the Aeron directory `ConfiguredDriver`
  sets.
- **The Aeron context** needs its network agents busy-spinning on CPUs of
  their own (Threading modes), disjoint wildcard port ranges, and an MTU no
  larger than `RUSTERON_DPDK_MAX_AERON_MTU`, set with the usual `AERON_*`
  variables.
- **`RUSTERON_MEDIA_DRIVER_MLOCKALL=1`** (or `lock_memory_from_env()`) calls
  `mlockall(MCL_CURRENT | MCL_FUTURE)` before the driver starts. Needs
  `CAP_IPC_LOCK` or a large enough `RLIMIT_MEMLOCK`. Not recommended; its
  latency effect is not measured here.
- **Embedded:** a typed `DpdkTransportConfig`
  ([`examples/embedded_dpdk.rs`](examples/embedded_dpdk.rs)), or the binary's
  whole configuration with `driver::ConfiguredDriver`, then
  `EnvMediaDriver::do_work` in your loop.

## Threading modes

The agent that carries the network work must busy-spin (`spin` or `noop`;
sleeping or yielding stalls a DPDK queue) on a CPU the transport knows.

| `AERON_THREADING_MODE` | The network agent | Its CPU | Its idle strategy |
|---|---|---|---|
| `DEDICATED` | the sender and the receiver, a NIC each | `AERON_SENDER_CPU_AFFINITY`, `AERON_RECEIVER_CPU_AFFINITY`, distinct | `AERON_SENDER_IDLE_STRATEGY`, `AERON_RECEIVER_IDLE_STRATEGY` |
| `SHARED_NETWORK` | one agent, both NICs | `AERON_SENDER_CPU_AFFINITY` | `AERON_SHAREDNETWORK_IDLE_STRATEGY` |
| `SHARED` | one agent for everything | `AERON_CONDUCTOR_CPU_AFFINITY` | `AERON_SHARED_IDLE_STRATEGY` |
| `INVOKER` | the application's thread that runs the driver | `AERON_CONDUCTOR_CPU_AFFINITY` | `AERON_SHARED_IDLE_STRATEGY` |

- **Each NIC is touched only by its own agent's thread.**
- **With one thread for both directions** (`SHARED_NETWORK`, `SHARED`,
  `INVOKER`) a send never waits out a pacing debt:
  `RUSTERON_DPDK_TX_PACE_SPIN_US` is 0, and a non-zero value fails the
  install.
- **Under `SHARED` and `INVOKER`** the conductor's work (mapping log buffers,
  opening host-local sockets and channel transports) runs on the network
  thread, as over kernel UDP. The bundled binary idles there with the shared
  idle strategy; an `INVOKER` application's own loop must not sleep.
- **The install refuses** the EAL's main lcore on a network agent's core; a
  creating thread confined to that core (an `INVOKER` application that pinned
  its thread first); and, under `DEDICATED` and `SHARED_NETWORK`, a network
  agent on the conductor's CPU.
- **So create the transport on a housekeeping CPU** from the pod's cpuset
  (the binary under `taskset -c <cpu>`, an `INVOKER` application before it
  pins), or name it:
  `RUSTERON_DPDK_EAL_ARGS='-l <cpu>'`, `RUSTERON_DPDK_HOUSEKEEPING_CPU`.

## CPU pinning

Aeron's C driver parses `AERON_{CONDUCTOR,SENDER,RECEIVER}_CPU_AFFINITY`, but
only `aeronmd` applies them. `pin_agents_to_configured_cpus` installs the same
start hook; `DpdkTransport::install` and the bundled binary call it in every
mode. Running unpinned is not measured here.

## Receive ring

- **`RUSTERON_DPDK_RX_DESCRIPTORS`**, unset: min(4096, the NIC's maximum),
  rounded down to a power of two: 4096 on ENA, 256 on MANA. Set: 64 to 8192,
  used as given; start fails on a NIC that cannot take it. The `port-info`
  label reports it (`rx-desc=`).
- **4096 against ENA's 1024** did not separate at idle: `c7i.4xlarge`
  round 6 max 107 (88–517) against 125 (94–142) µs, `c9g.2xlarge` G1 73
  (60–88) against 75 (68–155) µs. Round 6's loss rows for 1024 were not
  recovered
  ([BENCHMARKS.md](BENCHMARKS.md#measured-but-no-setting-recommended)).
- **`RUSTERON_DPDK_MBUFS_PER_PORT`** (default 16384) must hold both rings,
  four bursts and two lcores' mempool caches (1.5 ×
  `RUSTERON_DPDK_MEMPOOL_CACHE` each): 5,248 with the defaults. Start fails,
  naming both variables, when it falls short. On MANA with the ring unset, a
  pool sized for 256 fails validation (which assumes 4096): set
  `RUSTERON_DPDK_RX_DESCRIPTORS=256` or enlarge the pool.
- **Hugepages:** the pool's size does not depend on the ring: about 37 MiB a
  port at the default MTU (2,048-byte mbufs), 145 MiB at 8960.

## Queue tuning

Pod env, read at start. Unset, each queue keeps the PMD's defaults.

| Variable | Default | Effect |
|---|---|---|
| `RUSTERON_DPDK_RX_FREE_THRESH` | PMD's (ENA: 256 at 4096 descriptors) | Descriptors used before the PMD refills them, inside the `rx_burst` that crosses it. 1 to the ring less 1. |
| `RUSTERON_DPDK_TX_FREE_THRESH` | PMD's (ENA: 224 at 256 descriptors) | ENA reclaims every completed descriptor inside the send that finds fewer than this free. 1 to `RUSTERON_DPDK_TX_DESCRIPTORS` less 1. |
| `RUSTERON_DPDK_TX_FAST_FREE` | off | `RTE_ETH_TX_OFFLOAD_MBUF_FAST_FREE`: sent mbufs freed in bulk. Configured only where the NIC offers it (ENA does). |
| `RUSTERON_DPDK_TX_IDLE_CLEAN` | off (`0`) | Sent descriptors the agent reclaims on a poll that received nothing, at most this many a poll (1 to 1024), once the last send is 5 µs old (`rte_eth_tx_done_cleanup`), so the PMD rarely reaches its free threshold inside a send. Counted as `tx-idle-cleaned` (type 87). |

- **Validation:** a threshold of 0 or at least its ring is refused, naming
  the variable: in Rust against the configured ring (4096 unset), and at start
  against the ring the port took (MANA's 256).
- **`port-info` label:** `tx-fast-free=on`, `tx-fast-free=unsupported`
  (asked for, NIC lacks it, left off and logged), and
  `rx-free-thresh=… tx-free-thresh=…` when set.
- **FAST_FREE soundness:** every frame sent (data, ARP, ICMP) is one segment
  from the port's own pool, held by nothing else. `tests/dpdk_queue_conf.rs`
  sends through each path with the fake checking all three.
- **`port-info` label:** `tx-idle-clean=<n>`, or `tx-idle-clean=unsupported`
  where the PMD lacks the op (found at start, left off and logged).
- No setting here is recommended: none separated from the A/A control on
  `c9g.2xlarge` (`BENCHMARKS.md`).

## ENA low-latency queues

ENA's transmit path needs its low-latency queue (LLQ), which writes into the
device's BAR 2; the transport needs that BAR mapped write-combined, and
start fails otherwise (`tests/dpdk_ena_llq.rs`).

- **Every ENA port asks for large LLQ entries:** `llq_policy=3` with DPDK
  24.11+ (256-byte entries: a small frame's headers go with the descriptor),
  `enable_llq=1` before. On `c9g.2xlarge` they cut idle p50 by 0.7 µs and
  p99 by 1.0 µs against `llq_policy=1`, the device's recommendation
  (`BENCHMARKS.md`).
- **They halve an ENA's TX ring to 256**, the default
  `RUSTERON_DPDK_TX_DESCRIPTORS`. A larger ring needs another policy
  (`RUSTERON_DPDK_ENA_DEVARGS=llq_policy=1` takes 512); start fails otherwise,
  naming the variable.
- **`RUSTERON_DPDK_ENA_DEVARGS="<key>=<value>,..."`** adds device arguments
  to every ENA port (`miss_txc_to=5`, `control_path_poll_interval=500`); an
  LLQ key in it (`llq_policy=3`) replaces the transport's. Keys and values
  `[A-Za-z0-9_.:-]`, at most 200 bytes, the LLQ value decimal and in range;
  else initialisation fails. Put ENA arguments here, not in
  `RUSTERON_DPDK_EAL_ARGS`: DPDK takes the first `-a` for a device.
- **Start checks every ENA port:** LLQ not turned off, and BAR 2 mapped
  write-combined in `/proc/self/maps` (`resource2_wc`, as `igb_uio` with
  `wc_activate=1` or `uio_pci_generic` map it). Otherwise it fails, naming the
  port: BAR uncached (`resource2`), or unseen, as under `vfio-pci`.
- **`RUSTERON_DPDK_VFIO_WC=1`** confirms a write-combining `vfio-pci` (Amazon's
  patched module), so an unseen mapping starts. `0` (default) or `1`. The EKS
  ConfigMap sets it.
- **`RUSTERON_DPDK_ENA_LLQ_REQUIRED=0`** starts a failing port anyway, with a
  warning, for diagnosis.
- **`port-info` label:** `llq=wc`, `uncached`, `unseen`, `unknown` (map
  unreadable) or `off`. It cannot see a kernel that demoted the mapping to
  uncached (`benches/cloud/vm.sh wc-check` reads PAT's table), nor the PMD's
  final policy (`Placement policy: Low latency` at
  `--log-level=pmd.net.ena.driver:info`).

## Azure

- **Move each data NIC's VMBus device to `uio_hv_generic`** (its kernel
  interface and VF down), so the netvsc PMD takes it and keeps the VF data
  path. `benches/cloud/vm.sh driver dpdk` shows the steps.
- **TX descriptors:** MANA queues hold at most 256, the default. A count
  outside a NIC's limits fails start.
- **The transport stops DPDK's automatic vdev_netvsc fail-safe device,**
  which would otherwise take the ports.

## Host checks

`driver::ConfiguredDriver::from_env` (and so the bundled binary) checks the
host after pinning and before the driver starts (`src/checks.rs`, on
`ergon-tune`). It prints one stderr line per check that is not fine:
`ergon-md-dpdk warning: ...` with the measured effect and the fix, or
`ergon-md-dpdk info: ...` (including `unknown`, never a warning, for data it
cannot read, such as from a pod's network namespace).

| Check | What | Severity |
|---|---|---|
| M1 | The network agents busy-spin on pinned CPUs of their own, off the conductor's | socket transport: warn; DPDK: the install refuses otherwise |
| M2 | Non-sparse term buffers (`AERON_TERM_BUFFER_SPARSE_FILE=false`) | warn |
| M3 | `AERON_SOCKET_SO_RCVBUF`/`SNDBUF` granted as asked (a probe socket) | socket transport: warn; DPDK: info |
| M4 | Kernel busy polling (`net.core.busy_read`, `busy_poll`): its state and what was measured; no setting recommended ([BENCHMARKS.md](BENCHMARKS.md#measured-but-no-setting-recommended)) | socket transport: info |
| M-host | T1 and T8 on the agents' CPUs, T7, T3, T4 (`AERON_DIR`), T5, T9 (no CPU quota on the driver's cgroup), T10 (SMT on) | as `ergon-tune` gives them |
| M-nic | T2 and T6 on the data NICs; N1, N2, and N4 with busy polling, for the socket transport | as `ergon-tune` gives them |

- **Data NICs:** under `dpdk-ena`, the ports. For the socket transport, the
  interfaces in `RUSTERON_MEDIA_DRIVER_DATA_NICS` (`ens6,ens7`); without it,
  one information line says what is skipped.
- **Re-checks:** a thread (`md-host-checks`, Linux, off the busy cores) reads
  each named NIC's `carrier_changes` once a second and re-runs N1 and T2
  after a reset, which on ENA re-places interrupts and turns adaptive
  moderation back on. It only prints; it never stops the driver. Nothing is
  re-checked under `dpdk-ena`.
- **Gateways.** Under `dpdk-ena` the creating thread asks each port's
  gateway for its MAC before the agents start, waiting up to 1 s. An answer
  is learned, so the first send through it does not wait; a silent gateway
  is one warning line naming the port and its `*_GATEWAY` variable.
- **`ERGON_HOST_CHECKS`**: `on`/`warn` (default), `off` (no check runs), or
  `require`, which refuses to start while any check warns, a silent gateway
  included (`DpdkError::HostCheck`). Any other value refuses to start.

## No pauses on the hot path

The sender and receiver agents only send, receive and bump counters. Nothing
periodic runs on them.

- **ARP.**
  - Only the first send to a next hop waits for resolution (Aeron retries).
    A next hop silent for 3 s fails one send with `EHOSTUNREACH`; later sends
    drop silently while it is re-probed once a second.
  - Up to 1024 next hops a NIC, at most 16 slots examined a lookup; a new one
    replaces the least recently used next hop idle for 60 s, else its
    datagrams drop (`ENOBUFS`, reported at most once a minute).
  - A refresh request goes out at 30 s, after the data frames of the send
    that found it due, never before them; an unanswered refresh keeps the
    cached MAC. A frame from a peer's IP with a new MAC triggers a refresh at
    once; only a solicited answer changes the MAC.
  - ARP requests for the NIC's addresses are answered at most 1000 times a
    second. ARP and ICMP replies are built after the receive burst's data
    frames are dispatched, so a datagram never waits behind one.
- **Error reports.** A SETUP from a peer whose MTU is above ours and an ICMP
  fragmentation-needed are counted on the agent (`peer-mtu-too-large`,
  `icmp-frag-needed`), which queues a fixed-size record at most once a second
  per NIC. Housekeeping formats it: one line on stderr with the address, and
  one entry per kind in the driver's distinct error log. The agent never
  formats an address or takes that log's mutex, which the conductor shares.
- **Statistics** run once a second on a housekeeping thread: NIC drop counters
  (`nic-rx-errors`, `nic-rx-missed`, `nic-rx-nombuf`), PMD extended
  statistics (on ENA an admin command each) and the mempool.
  `RUSTERON_DPDK_NIC_STATS_MS` sets the interval (1000 to 3600000; `0` stops
  the reads). `RUSTERON_DPDK_NIC_SLOW_STATS_MS` (same range, unset by
  default) splits them: every `NIC_STATS_MS` only the PMD's software xstats
  (on ENA its queue statistics, `wd_expired`, `dev_start`, `dev_stop`,
  `tx_drops`) by id, no admin command; the basic drop counters and every
  xstat (ENI allowances, SRD) every `NIC_SLOW_STATS_MS`, checked on the
  1 Hz tick (`0`: never). Once split, `NIC_STATS_MS=0` stops only the
  software reads. The latency effect of the reads is not measured
  here.
  Housekeeping holds its lock only to copy a port's state and to store what
  it read, never across the read, so a transport's close never waits for an
  ENA admin command.
- **Housekeeping placement:** `RUSTERON_DPDK_HOUSEKEEPING_CPU`, else the
  conductor's CPU unless the conductor spins, else the EAL main lcore's CPU;
  not a busy agent's core or its SMT sibling. The EAL's own threads are
  confined to the main lcore's CPUs. Start the driver confined to a
  housekeeping CPU from the pod's cpuset (`taskset -c <cpu> media_driver`) so
  threads it does not own stay there too.
- **NIC health.** Housekeeping runs DPDK's timer service once a second, which
  drives the ENA PMD's keep-alive and transmit-completion watchdog. A failed
  check is counted (`nic-reset-requested`, type 79) and logged at error
  level. The transport does not reset the device under the polling agents.
  The `media_driver` process stops them and exits non-zero, so a DaemonSet
  restarts it. Where the NIC can raise link-status-change interrupts
  (`RTE_ETH_DEV_INTR_LSC`; ENA does), each change is counted
  (`nic-link-changes`, type 79) and logged as a warning; the transport does
  nothing else about it. Otherwise none is asked for and `port-info` says
  `lsc=off`.
  Under `uio_pci_generic` the keep-alive needs ENA's
  `control_path_poll_interval` devarg (DPDK 24.11+), or the watchdog fires.
- **Receive drain.** A poll takes up to `RUSTERON_DPDK_BURST_SIZE` frames a
  burst, zero copy, and keeps draining while bursts come back full, up to
  `RUSTERON_DPDK_RX_MAX_BURSTS` (default: enough to fill the smaller receive
  ring, at most 8; 1 to 64). Counters: `rx-backlog-max`, and on Azure
  `rx-ring-full-suspected` (MANA overflows without counting a drop; ENA counts
  its own as `nic-rx-missed`).
- **Loop health.** Per port: the longest gap between polls that found frames
  (`rx-poll-gap-max-ns`, and how many reached 100 µs), full-burst polls
  (`rx-burst-full`), and the agent's own loop gap (`poll-cycle-gap-max-ns`,
  counted in 20–50, 50–100 and over 100 µs), to match a latency spike to the
  agent not running. `RUSTERON_DPDK_GAP_ATTRIB=1` additionally splits each
  loop gap by the thread's own CPU time across it — half or more of the wall
  length is work (`poll-cycle-gaps-work`, `poll-cycle-gap-work-max-ns`), less
  is the host descheduling the vCPU (`poll-cycle-gaps-desched`,
  `poll-cycle-gap-desched-max-ns`). Off by default, so the hot path never
  reads the CPU clock for it. Both agents are covered: the sender agent's
  control poll times the sender port (`role=sender ... dir=rx`), so a stall
  in its sends lands there.
- **PMD call timing.** With `RUSTERON_DPDK_GAP_ATTRIB=1` or `burst`, each `rx_burst`
  that returns frames and each `tx_burst` is timed: `rx-burst-max-ns`,
  `rx-bursts-over-1us`, `rx-bursts-over-5us` and the `tx-` three (type 87).
  ENA refills its receive ring and reclaims sent descriptors inside these
  calls. Two clock reads a burst while on; one predictable branch while off.
  `burst` times the bursts without the loop-gap split: `1` also reads the
  thread CPU clock every poll, a system call (no vDSO path; about 280 ns a
  poll in the fake-port bench), so a latency run should use `burst`.
- **Event log.** `RUSTERON_DPDK_EVENT_LOG=<file>` (pod env, lab only) writes
  stamped events so a latency maximum can be matched to what the driver did:

  | Kind | Written by | `value_ns` |
  |---|---|---|
  | `gap` | the port's agent: a loop gap of 10 µs or more | its length |
  | `arp-request`, `arp-reply`, `arp-learned` | the port's agent | 0 |
  | `timers` | housekeeping: DPDK's timer service (ENA watchdog) | its duration |
  | `sample`, `nic-stats` | housekeeping: a port's 1 Hz sample, its NIC statistics read | its duration |
  | `drops` | housekeeping: events a full ring dropped | how many |

  Rows are `kind,tsc,realtime_ns,value_ns,port,agent`: the CPU counter and
  `CLOCK_REALTIME` when the event ended, the DPDK port (`-` for `timers`) and
  the thread's kernel id. Each writer has its own preallocated ring of 4096
  events; it never locks, allocates or writes the file, and a full ring drops
  the newest event. Housekeeping writes the file every 100 ms; close writes
  the rest. Start fails, naming the variable, if the file cannot be opened.

## Kernel socket semantics

DPDK peers and kernel peers interoperate: the transport behaves as Aeron's
kernel UDP sockets.

- **Ports.** An explicit port already bound fails with `EADDRINUSE`. Port 0
  gets an ephemeral port, unique on the NIC, from
  `RUSTERON_DPDK_EPHEMERAL_PORT_RANGE="<low> <high>"`, else
  `net.ipv4.ip_local_port_range` minus reserved ports and Aeron's wildcard
  ranges. For a fixed source port use `interface=<nic-ip>:<port>`. If a kernel
  interface holds a DPDK NIC's IP, start fails unless the transport's range
  avoids the kernel's or is reserved there.
- **Addresses.** Bind `0.0.0.0`, the NIC's address or one of
  `RUSTERON_DPDK_{SENDER,RECEIVER}_EXTRA_IPV4`, else `EADDRNOTAVAIL`; each
  address holds its own ports, and a `0.0.0.0` channel replies from the
  address the peer last reached. Multicast fails at creation. Sends to
  `0.0.0.0`, broadcast, port 0 or the NIC's own address are errors.
- **Connected transports** receive only from their peer; only they can
  `reconnect`.
- **Receive.** UDP checksums verified (RFC 768); IP fragments and frames
  larger than one mbuf dropped and counted. Port unreachable and echo
  replies, at most 100 a second each.
- **Checksums out:** offloaded to the NIC where it can, else computed.
- **Full transmit ring.** A send is all or nothing, as a kernel socket: what
  the ring refuses waits, built, in the NIC's queue (`tx-queued`) and goes
  first at the next send or poll. While frames wait, a new vector is refused
  whole and Aeron retries. `RUSTERON_DPDK_TX_RETRY_US` (default 0) re-offers
  refused frames for that long first.
- **MTU.** `max_aeron_mtu` up to 8960 (8960 on an AWS 9001-byte VPC, 8448
  across inter-region peering). Every frame is one mbuf; start fails, naming
  the largest workable value, when the NIC cannot carry it. A peer announcing
  a larger MTU is dropped and reported (`peer-mtu-too-large`), so its
  publication shows as not connected rather than stalling.
- **ENA Express.** UDP over SRD carries IP packets of at most 8900 bytes. An
  ENA whose `ena_srd_mode` xstat has UDP on (bit 1) fails start with
  `DpdkError::InvalidConfiguration` when `max_aeron_mtu` + 28 is above that,
  naming the largest MTU that fits (8864). ENA Express is set on the ENI
  (infrastructure), not in the pod.

## Bandwidth allowance

Clouds drop traffic over a VM's allowance; Aeron then retransmits and
throughput collapses. On ENA the drops are counted by the PMD's ENI xstats,
which the transport mirrors as type-92 counters named by the PMD
(`bw_in_allowance_exceeded`, `bw_out_allowance_exceeded`,
`pps_allowance_exceeded`, `conntrack_allowance_exceeded`,
`linklocal_allowance_exceeded`), read every `RUSTERON_DPDK_NIC_STATS_MS`.
MANA counts none, nor its own ring overflow; `rx-ring-full-suspected` is a
heuristic for the latter (`native/dpdk/rusteron_dpdk_runtime.c`). No Azure
run output is committed.

| variable | meaning |
|---|---|
| `RUSTERON_DPDK_TX_RATE_MBPS` | each NIC's transmit cap, Mbit/s of frames; up to 400000. Unset or `0`: no cap, the fastest path |
| `RUSTERON_DPDK_TX_BUCKET_BYTES` | the cap's token bucket, 64 KiB by default |
| `RUSTERON_DPDK_TX_PACE_SPIN_US` | how long a send may spin to pay a debt: 20 under `DEDICATED`, 0 (and only 0) otherwise; longer sends nothing, and Aeron retries (`tx-paced`). `0` always refuses instead of waiting |
| `RUSTERON_DPDK_TX_ROUTE_MBPS` | `<a.b.c.d>/<len>=<Mbit/s>,...`, up to 16 prefixes: an extra cap for a slower link, shared by every channel that crosses it |
| `RUSTERON_DPDK_TX_LOW_STREAMS` | `<stream>\|<first>-<last>,...`, up to 16: streams that send only from what leaves `RUSTERON_DPDK_TX_LOW_RESERVE_BYTES` (half the bucket) for the rest, so latency-critical streams never queue behind them. Needs a cap set |

- **A cap is per NIC and opt-in.** Set one only to keep a sender below the
  link's capacity, or to shape a slower route with `RUSTERON_DPDK_TX_ROUTE_MBPS`.
  It costs a refill, a sum and a charge on every send.
- **One AWS flow (one channel) topped out at 7.75–7.77 Gbit/s of Aeron
  frames** on `c7i.4xlarge`, kernel UDP and DPDK alike; on `c9g.2xlarge`
  1,344 B messages stopped at 685,574 msgs/s
  ([BENCHMARKS.md](BENCHMARKS.md#one-flows-ceiling-on-aws)). AWS documents a
  10 Gbit/s single-flow limit in a cluster placement group. Spread a heavier
  stream over several channels on different ports.
- **A small receiver window (`rcv-wnd`) caps the rate.** Only a ring that
  cannot hold more needs one (Azure MANA: 256 × the smallest datagram, 80 KiB
  for 288 B), paired with the conductor's park capped at 20 µs
  (`AERON_CONDUCTOR_IDLE_STRATEGY_INIT_ARGS=100-10-1us-20us`). On AWS
  (`c7i.4xlarge`, round 7) the 20 µs park with a 256 KiB window did not
  separate at idle, and at 288 B 500 k msgs/s its max was 5,140 against
  135 µs
  ([BENCHMARKS.md](BENCHMARKS.md#measured-but-no-setting-recommended)). The
  Azure pairing is not measured here.

## Host-local channels

A DPDK NIC cannot reach its own host. Channels on loopback or the host's
kernel interfaces (Archive and Cluster control channels, co-located
processes) run on kernel UDP sockets in the same driver, as Aeron's kernel
transport would carry them. That is why localhost gains nothing.

- **No idle system calls:** a local-I/O thread on housekeeping waits on those
  sockets with epoll and flags the ones with data.
- **First send** to a host-local destination: that thread opens the socket;
  the agent reports zero-length sends (Aeron retries) until it is ready,
  within about 10 ms. A failed open is reported once and retried each second.
- **A publication with mixed destinations** (MDC) sends host-local ones
  through a kernel socket and the rest over DPDK.
- **A `0.0.0.0` subscription** also listens on the kernel's interfaces
  (`RUSTERON_DPDK_WILDCARD_LOCAL=0` opts out). These sockets hear only this
  host.
- **`RUSTERON_DPDK_HOST_LOCAL=loopback`** limits host-local to `127/8`, for a
  test rig whose DPDK peer is a kernel interface on the same host.
- **`RUSTERON_DPDK_LOCAL_IO`:**
  - `adaptive` (default): read only when flagged while idle, but every duty
    cycle for `RUSTERON_DPDK_LOCAL_HOT_US` (1000) after data. A hot socket
    leaves the epoll, since a registered one wakes that thread on every
    datagram; `RUSTERON_DPDK_LOCAL_UNWATCH=0` keeps it on. The cost of that
    wake-up is **not measured**.
  - `epoll`: only when flagged; a thread wake-up per message. Cost **not
    measured**.
  - `spin`: every duty cycle, a `recvmmsg` per socket, no epoll.

## Clocks

The transport times intervals (ARP freshness, the pacer, poll gaps) in
nanoseconds since boot; that clock is not wall-clock time. Receive
timestamps ([below](#receive-timestamps)) use `CLOCK_REALTIME`. In your own
code: `quanta` for durations, `CLOCK_REALTIME` for stamps.

| `RUSTERON_DPDK_CLOCK` | The transport reads |
|---|---|
| `counter` (default; `tsc` too) | The CPU counter inline (TSC on x86_64, `CNTVCT_EL0` on aarch64), scaled by its rate (below), when all of the checks below pass; else `CLOCK_MONOTONIC` |
| `monotonic` | `CLOCK_MONOTONIC` |

The counter is used only when:

- DPDK's EAL gives it a rate;
- the kernel's clock source is the counter (`tsc`, `arch_sys_counter`) or a
  hypervisor's view of it (`kvm-clock`, `hyperv_clocksource_tsc_page`,
  `xen`);
- x86_64: CPUID calls the TSC invariant;
- aarch64: DPDK's rate is `CNTFRQ_EL0` (a DPDK built to time with the PMU
  reports another).

**The rate.** When the transport starts, before its threads, it measures
the counter against `CLOCK_MONOTONIC_RAW` for 25 ms (each end the tightest of
16 bracketed reads), then converts at:

| DPDK's rate (`rte_get_tsc_hz()`) against the measurement | Rate used |
|---|---|
| within 100 ppm | DPDK's |
| more than 100 ppm off | the measurement |

DPDK's rate is not always exact. On x86_64 it is the CPU's stated rate or
one 100 ms measurement DPDK makes at start, depending on the DPDK version and
the CPU. DPDK rounds that measurement to the nearest 10 MHz on 23.11 (up to
0.2% off at 2.5 GHz) and to the nearest 100 kHz from 24.11. On aarch64 it
must be `CNTFRQ_EL0` (above), the generic timer's stated rate.

**Which one ran:** each port's `clock` counter (type 80) reads 1 TSC,
2 `CNTVCT_EL0`, 0 `CLOCK_MONOTONIC`. `DpdkTransport::clock()` returns a
`TransportClock`:

| Variant | Fields |
|---|---|
| `Counter` | `hz` (the rate used), `rate` (`Eal` or `Measured`), `eal_hz`, `measured_hz` |
| `Monotonic` | `reason` (`MonotonicReason`) |

## Receive timestamps

- **NIC timestamps:** `RUSTERON_DPDK_RX_HW_TIMESTAMP=1`
  (`DpdkTransportConfig::rx_hw_timestamp`). Init fails on a port that cannot:
  today only AWS ENA on Gen7+ instances in a `precision-time` placement group,
  DPDK 25.11+. Azure MANA has none.
- **What it measures:** per port, NIC arrival to the receive burst that
  carried the frame, against `CLOCK_REALTIME` read once a burst (type 81:
  `nic-arrival-to-rx-burst-max-ns`, `-min-ns`, `-sum-ns`, `-count`). The read
  is after the NIC handed the burst over and before any frame of it is
  classified or dispatched, so a frame after the burst's first is measured
  before its own dispatch: a lower bound on arrival to dispatch, exact only
  for the first frame of the burst. A stamp after the read is clock skew,
  counted in `nic-arrival-to-rx-burst-negative`. It means something only while
  chrony keeps the host on the NIC's PTP clock (`benches/cloud/vm.sh
  build-ptp` sets it up).
- **`media-rcv-ts-offset` channels** get each datagram's receive time, as on
  Aeron's kernel transport: the NIC's stamp with hardware timestamps on, else
  `CLOCK_REALTIME` once a burst; on host-local sockets, the kernel's
  `SO_TIMESTAMPNS`. A NIC with no such channel open reads no clock for them.
  `channel-rcv-ts-offset` needs nothing from the transport.
- **Cost, one run** (Graviton G3, DPDK): p50 within 0.07 µs of without at
  idle and every steady rate; idle max 153 against 79 µs; at 1,344 B bursts
  1 of 5 reps lossy (short sends) against 0. Not separated; repeat before
  relying on it
  ([BENCHMARKS.md](BENCHMARKS.md#measured-but-no-setting-recommended)).

## Lifetime

- **Dropping a `DpdkTransport` only leaks it** (`src/ffi.rs`, no `Drop`).
  Freeing it is `DpdkTransport::close`, which is `unsafe` and sound only once
  every driver started from the context has stopped. `EnvMediaDriver` frees
  it in that order.
- **Counters after exit:** closing a driver frees its counters. To keep them
  readable in the `CnC` file after the process exits (a lab client),
  `EnvMediaDriver::abandon` closes the transport but not the driver; only
  `SHARED` and `INVOKER` allow it.

## Layout

| Path | What |
|---|---|
| `src/` | Rust API: config, environment, the transport's clock, errors, FFI, host checks, and the `media_driver` binary |
| `native/dpdk/` | The transport in C, split so tests link DPDK-free fakes of the EAL and port seams |
| `tests/` | Config/environment, transport-clock, binary-selection and host-check tests (every platform); ABI, runtime, clock, TX, RX, counters, health, ENA LLQ, media timestamp, threading and interop tests against the fakes (`dpdk`) |
| `harness/` | `dpdk-harness`, the interop scenario runner behind `scripts/test-dpdk-vdev.sh` and `scripts/aws-dpdk-acceptance.sh`, and the `rtt`, `load`, `tput` and `counters` benchmark clients (unpublished). With `HARNESS_DRIVER=invoker` each embeds its own driver, configured as `media_driver` is, in `INVOKER` mode (`harness/src/lab.rs`) |
| `benches/cloud/` | The cloud labs behind BENCHMARKS.md: `azure/lab.sh` and `aws/lab.sh` (two VMs, created and deleted per run), `vm.sh` (each VM's side), `remote.sh`, `analyse.py` |
| `device-plugin/` | Kubelet device plugin advertising VFIO ENA pairs (unpublished) |
| `deploy/aws-eks/` | Runtime images, DaemonSets and the node bootstrap and preflight scripts; the node and pod settings for latency: the book's [Kubernetes checklist](https://mimran1980.github.io/ergon/deploy/kubernetes.html#latency-checklist) |
| `docs/` | The AWS EKS rollout and rollback runbook |

## Checks

From the ergon root:

```sh
cargo test -p ergon-md-dpdk -- --test-threads=1   # every platform
just check-dpdk                                   # Linux x86_64 or aarch64 + libdpdk
just verify-dpdk-docker --build --all             # the same in Docker amd64 emulation
just verify-dpdk-docker --arm64 --build --all     # and in Docker arm64 (native on Apple silicon)
just test-dpdk-bootstrap                          # node scripts against fixtures
just test-dpdk-vdev                               # tap-device interop (root)
scripts/test-dpdk-vdev-rootless.sh                # the same scenarios, unprivileged (below)
just verify-dpdk-docker --arm64 --build --rootless  # and those in Docker, as a non-root user
```

The vdev scenarios run the real tap PMD, EAL and mempools, and assert on the
reports' counters and logs: interop both ways with kernel UDP, reconnect,
three endpoints, loss recovery, restart, ICMP echo, a peer MAC that moves
mid-stream, a gateway that never answers ARP, and full frames at the largest
Aeron MTU (8960; 8992 refused at start).

| Lane | Needs | Memory |
|---|---|---|
| `test-dpdk-vdev.sh` (sudo) | root, `/dev/net/tun`, hugetlbfs | 2 MiB hugepages |
| `test-dpdk-vdev-rootless.sh` | unprivileged user namespaces (`kernel.apparmor_restrict_unprivileged_userns=0` on Ubuntu 23.10+, set once by root), `/dev/net/tun`, `sch_netem` loaded; Docker: `--security-opt seccomp=unconfined --security-opt apparmor=unconfined --device /dev/net/tun` | EAL `--no-huge -m 512` |

The rootless runner fails, naming the sysctl, when the namespace is refused;
it never skips. It is a CI and developer-box tool: nothing here goes into a pod
or onto a node.

## License

Apache-2.0.
