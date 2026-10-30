# Changelog

## [Unreleased]

### Changed

- ergon-io's and ergon-md-dpdk's start-up host checks now run T13-T18
  (workqueue cpumask, compaction, other tasks on a hot CPU,
  instrumentation, KSM, steal), and ergon-io also T21 (TCP autocorking).
  They were in ergon-tune with no caller.
- ergon-io `recv-busy` with `per_queue`: no poll looks up more than one NIC
  queue id. The look-again checks one socket every 16 polls in turn, instead
  of every socket in one poll every 4096 polls (256 `getsockopt` calls at 256
  sockets), and adding or removing a socket no longer re-reads every id.

### Docs

- Book: "Aeron with Many Streams". Aeron's latency and memory grow with its
  stream count, so use up to 8 Aeron streams per driver with the key in each
  message, and filter before decoding. Measured layouts, term lengths,
  settings and driver sharding at 1 to 128 streams.

## [0.1.37] — 2026-10-30

### Added

- `fstack-spin` and `smoltcp-spin` work out the NIC in a pod granted the
  ENA device plugin's pair (`rusteron.io/dpdk-ena-pair`): a line that names
  no NIC takes the address, prefix, gateway and PCI port from
  `RUSTERON_DPDK_SENDER_{PCI,IPV4_CIDR,GATEWAY}`, so the bare `fstack-spin`
  (or an empty smoltcp stack line) is a whole config. Copied, never guessed,
  never mixed with typed keys; a broken pair fails the parse naming the
  key. New: `Config::parse_with_env`, `StackConfig::parse_with_env`,
  `FstackConf::take_granted_pair` and `names_a_nic`, and
  `ergon_tune::DpdkNicPair`.

### Fixed

- The `fstack-spin` line T24 prints and the NRI plugin's `ERGON_IO_FSTACK`
  kept a subnet other than /24 as /24 (F-Stack's default); they now add
  `fstack_prefix_len`. A DPDK device name in the pair (test mode) becomes
  `fstack_vdev` and `eal=--vdev <name>` instead of a PCI port.

### Changed

- `ergon-shared-cpus` image and chart 0.1.3: the first published image
  with the `dpdk.ergon.io` NIC pair exports and the Aeron hugepage
  variables (`AERON_FILE_PAGE_SIZE`, `AERON_TERM_BUFFER_SPARSE_FILE`).
  Image 0.1.2 has neither.
- Dependencies: roxmltree 0.21.2, smol_str 0.3.7, webpki-roots 1.0.11,
  rustix 1.1.6, cc 1.10.0.

### Docs

- `io/benches`: `poll_pass` times one poll pass over N sockets for raw
  epoll (level and edge), mio and the `Driver`. With the send set an idle
  pass is one `epoll_wait(0)` at every N (x86: 594 ns at 1 socket, 631 ns
  at 256); a second read to `EAGAIN` costs about 0.7 µs a message.

## [0.1.36] — 2026-10-29

### Added

- `ergon-tune` T24 (`t24_dpdk_nic_pair`): on the DPDK backends and the
  `dpdk-ena` media driver, warns when the ENA device plugin's NIC pair
  (`RUSTERON_DPDK_SENDER_{PCI,IPV4_CIDR,GATEWAY}`) is missing, empty or
  unparsable, naming the exact keys. With all three set the finding prints
  the shortest `fstack-spin` and `smoltcp-spin` lines built from them. No
  pair granted is ok, not a guessed address; never unknown. Read-only: it
  exports nothing, binds no NIC and sets no `AERON_DIR`.
- The shared-CPU NRI plugin's opt-in `dpdk.ergon.io/<container>: "ena-pair"`
  annotation: copies the granted NIC pair into `ERGON_IO_FSTACK`,
  `ERGON_IO_SMOLTCP`, `ERGON_DPDK_PCI`, `ERGON_DPDK_ADDR` and
  `ERGON_DPDK_GATEWAY`. Off by default; a missing, empty or unparsable
  value refuses the pod and names the key. No sysfs scan, no dpdk-devbind,
  no default-route fill.

### Changed

- rusteron 0.2.16 throughout, from 0.2.15: owned archive descriptor
  pollers and checked counter reads. No ergon code changes were needed.
- The book's transport comparison ranks only the cells whose echo was
  pinned: UDP, and F-Stack TCP (`fstack_spin_lat`). Kernel TCP, smoltcp
  TCP and every WebSocket echo stayed on the housekeeping CPU, so those
  rows are not ranked. UDP p99 stays under 52 µs (kernel) and under 46
  (smoltcp) at 0, 0.1% and 1% loss; F-Stack TCP's p99 and max rise with
  loss (at 1%, p99 2,216 to 255,984 µs, max 20,057 to 417,929). The same
  page's `c7a` UDP set is the Aeron comparison: DPDK p99 35.4, kernel
  Aeron p99 48.6, ergon-io kernel UDP p99 58.4 to 61.8. smoltcp was not
  in that session. F-Stack and smoltcp take the address, gateway and port
  only from their config, copied from a granted pair; they never detect
  them. The `c8g` pair's separate cluster at second :05 is on kernel UDP
  (echo pinned). Every pinned max is labelled.
- The pinned remeasure of those kernel TCP, smoltcp TCP and WebSocket
  echoes (`aws-20261029-174643` on `c7i.4xlarge`, us-east-1c;
  `aws-20261029-190425` on `c8g.2xlarge`, us-east-1b; one rep;
  `placement_b=ok` on all 60 cells each). Inside that session,
  smoltcp's no-loss p50 is 3.6 to 6.5 µs below kernel TCP on `c7i` and
  3.0 to 8.9 below on `c8g`; WebSocket is 3.0 to 7.2 and 3.4 to 10.1
  below. F-Stack WebSocket is 2.6 to 8.1 and 1.0 to 9.8 below kernel
  WebSocket. No-loss p99 stays 39.5 to 58.5. At 1% loss every arm's p99
  is milliseconds, and one `c8g` F-Stack WebSocket cell faults to a
  15.3 s max and loses 38.0% of its messages. These sessions stay
  separate from the first sessions' UDP and F-Stack TCP, and from
  Aeron. No Kubernetes overlay is added: one rep, no A/A arm, and the
  tails are loss, faults and unexplained maxima.
- Aeron hugepages, from runs already published. The IPC 2 MiB directory
  overlaps tmpfs at every load (round trip p50 0.20 to 0.25 against 0.22
  to 0.25). A sized 2 MiB hugetlbfs is what lets Aeron start. Transparent
  hugepages off cut an Azure idle tail (p99.99 67 to 53 µs, max 546 to
  106) and a traced 16.25 µs `khugepaged` max; one AWS run overlaps. The
  Kubernetes sample documents that Aeron-on-DPDK layout on the existing
  `eks-ena` overlays (render only). The book and the READMEs link it at
  `samples/README.md#aws-k8s`. WebSocket, ergon-io TCP and ergon-io
  on DPDK stay without an overlay: the remeasure is one rep, and its
  tails are loss, faults and unexplained maxima.
- `hugepages.ergon.io/<container>: "2M"` now also sets
  `AERON_FILE_PAGE_SIZE=2097152` and `AERON_TERM_BUFFER_SPARSE_FILE=false`.
  The process still sets `AERON_DIR`. Address, gateway and PCI stay unset.
  ergon-tune T23 warns when `AERON_DIR` is on hugetlbfs without those two.

### Docs

- `md-dpdk/BENCHMARKS.md` records the Azure 288 B loss study (two
  `Standard_D8ds_v6` pairs): the receive ring overflows while the receiver
  agent stalls, an 80 KiB window stops the loss at a 2 M msgs/s cap, and
  the 20 µs conductor park alone does not. Both runs' output files are
  committed.
- `samples/README.md` (aws-k8s) and the `ena-driver` component name the
  device plugin as what fills PCI, address and gateway for the Aeron driver,
  and give the two minimum lines; the ergon-io backends page now says the
  address, gateway and port are not guessed but copied from a granted pair.

## [0.1.35] — 2026-10-29

### Added

- `ergon-io` `fstack-spin` (feature `fstack`, Linux): TCP on F-Stack, FreeBSD's
  stack on DPDK, on a NIC of its own; framed TCP, WebSocket and HTTP over TLS
  as `FrameConn<FstackTcp>`, `WsConn<FstackTcp>`, `HttpConn<FstackTcp>`
  (dials' `on_stack`), configured by `Config`'s `fstack_*` keys
  (`fstack::FstackConf`). Docker lab test, and measured on AWS against
  kernel `epoll-spin` (the book's ergon-io section).
- `ergon-io` `smoltcp-spin` (feature `smoltcp-dpdk`, Linux x86_64 and
  aarch64): TCP and UDP on smoltcp 0.14.0 over a DPDK port, started by
  `smoltcp_dpdk::Stack::init`; `FrameConn<SmolTcp>`, `WsConn<SmolTcp>`,
  `HttpConn<SmolTcp>`, `UdpConn<SmolUdp>`. A 10 ms TCP RTO floor
  (`StackConfig::rto_min_us`; smoltcp's own is 1 s), immediate ACKs, no
  Nagle, timestamps; ARP entries refreshed or the gateway MAC pinned.
  Docker lab test, and measured on AWS against kernel `epoll-spin`
  (the book's ergon-io section).
- `ergon-io` `FrameDial::connect_blocking` and `WsDial::connect_blocking`
  (`HttpDial`'s exists), `Backend::KERNEL`, `Backend::bypasses_kernel`.
- `ergon-tune` T20 (`t20_irq_idle`): warns when a CPU that takes the data NICs'
  interrupts may sleep deeper than C1 (C1E, C6), with the per-CPU, boot-line
  and `/dev/cpu_dma_latency` fixes; info on `acpi_idle` stopping at C1.
- `ergon-tune` T21 (`t21_tcp_autocorking`): warns when `net.ipv4.tcp_autocorking`
  is on for a caller sending latency-sensitive TCP, with the per-namespace fix.
- `ergon-tune` T22 (`t22_aeron_affinity_names`): warns when an Aeron affinity
  setting uses its pre-1.54 name (`AERON_DRIVER_CPUSET_AFFINITY`,
  `AERON_DRIVER_CPUSET_WARNINGS_AS_ERRORS`), which Aeron 1.54 ignores. The
  `ergon-md-dpdk` media driver runs it at start.
- `ergon-md-dpdk` honours a channel's `socket-tos` (`AERON_SOCKET_TOS`, Aeron
  1.54): the IPv4 ToS byte of DPDK frames, `IP_TOS` on its kernel sockets.
- The transport suite's bypass matrix (`transport-aws-io-bypass.tsv`): kernel
  `epoll-spin`, `smoltcp-spin` and `fstack-spin`, TCP echo and WebSocket-over-TLS
  echo (`wstls_lat`, the same point runner as `tcp_lat`), plus UDP for the
  kernel and smoltcp. 1 and 8 connections, 10k and 50k msg/s, loss 0, 0.1% and
  1% on the kernel peer, minimum RTO 8 ms. `fstack_lat --impl ergon` selects
  `fstack-spin`. `fstack_spin_lat` runs the TCP measuring loop: the ping is
  `Driver<FrameConn<FstackTcp>>` and writes the row; the pong is the kernel
  peer (`epoll-spin`). `wstls_fstack_lat` (the same `--features fstack-spin`
  build) runs WebSocket-over-TLS the same way. `wstls_lat` and `udp_lat`
  built with `--features smoltcp-dpdk` run the smoltcp measuring loops
  (`StackConfig::rto_min_us`). A build without the feature still refuses
  the flag and does not label a kernel socket as that stack. The WebSocket
  client enables rustls secret extraction, stays on TLS 1.3, trusts the
  bench certificate and offers no ALPN. The ergon F-Stack clock on that
  path is 1000 Hz, so the 8 ms floor is 8 ticks (100 Hz rejects it).

### Changed

- Aeron 1.54.0 (rusteron 0.2.15) throughout: the `aeron` submodule, the Java
  jars the cluster harness runs, and every crate and sample. Aeron's Gradle
  9.8.2 builds on JDK 17 to 27.
- `ergon-md-dpdk` pins the driver's agents to CPUs set through the context's
  setters again: `pin_agents_to_configured_cpus` now runs Aeron's affinity
  resolution (cpuset offsets, topology warnings), which Aeron 1.54's pinning
  hook reads. Without it the sender and receiver floated. Housekeeping
  placement and the host checks use the resolved CPUs.
- `ergon-cpus` (nri-shared-cpus) refuses to translate `AERON_*_CPU_AFFINITY`
  when `AERON_THREAD_AFFINITY_USE_CPUSET_OFFSETS` (Aeron 1.54's name) or
  `AERON_DRIVER_CPUSET_AFFINITY` is true.
- Breaking: `ergon-io`'s TCP endpoints and dials take their socket type,
  `FrameConn<T = TcpStream>` (and `FrameDial`, `WsConn`, `WsDial`,
  `HttpConn`, `HttpDial`); code naming them without `T` is unchanged.
  `Backend::ALL` has five backends (`fstack-spin` and `smoltcp-spin` added;
  `Backend::KERNEL` is the old three), and `Config` has an `fstack` field.
  `UdpConn<D = UdpSocket>` takes its socket type too.
- Breaking: `ergon-md-dpdk`'s `clock` counter is type 1080 (was 80) and its
  `nic-arrival-to-dispatch-*` counters type 1081 (was 81), in Aeron's custom
  range: Aeron leaves 80 and 81 unassigned. Its other counters keep Aeron's
  DPDK type IDs.

### Docs

- The book's transport latency comparison has an AWS section (two
  `c7a.2xlarge`, ENA): Aeron on DPDK (md-dpdk), F-Stack TCP on DPDK, kernel
  Aeron, mio and ergon-io, at 1 and 8 streams. Aeron on DPDK is lowest
  (p50 27.5 to 30.7 µs, flat to 200,000 msg/s); F-Stack's p50 is 17 to 28
  µs below kernel TCP and it carries 200,000 msg/s where kernel TCP does
  not. mio against ergon-io over TCP, on one shared connection: no
  difference (the Azure gap was unequal busy polling); overloaded,
  ergon-io drains slower. Every max is attributed or listed as unexplained.
  `md-dpdk/BENCHMARKS.md`, `io/README.md` and `io/benches/BENCHMARK.md`
  carry the matching rows and answers. DPDK on Azure stays excluded.
- The transport comparison measures an Aeron spy whose publications also
  send every frame over UDP (kernel or DPDK) to another host: p50 unchanged
  (0.26 µs round trip, 0.14 one way), every frame verified at the sink.
  This replaces the note that the published spy arm sent nothing.
- Aeron driver threading on AWS, now under mixed load (`c7i` and `c8g`,
  kernel UDP and DPDK, 32 and 128 streams, 50,000 and 200,000 msg/s): once
  there are many streams every thread taken from the driver costs latency.
  Hot p50 above dedicated: `SHARED_NETWORK` +1.9 to +32.6 µs, `SHARED`
  +3.1 to +73.7, `INVOKER` +4.9 to +130.8; the in-process dedicated driver
  is the same as its own process. At 128 streams `SHARED` and `INVOKER`
  saturate their one thread; Graviton pays 1.4 to 2.9 times what Intel
  pays. This replaces the 1-stream result as the guidance (at 1 stream
  DPDK's modes were not separated to 50,000 msg/s, kernel `INVOKER` was
  3.1 to 8.4 µs higher). The `c7i` once-a-minute path outage recurred
  (second :25, 4.2 to 16.2 ms). The embedded kernel driver's 1-stream
  millisecond maxima stay listed as unexplained.
- ergon-io on AWS (`c7i.4xlarge` and `c8g.2xlarge`, one rep each): kernel
  `epoll-spin` against smoltcp and F-Stack, in the book's transport
  comparison. This matrix does not show the mixed-load `c7i` outage at
  second :25. The `c8g` pair has a separate cluster at second :05, on
  kernel `epoll-spin` only. Every max is labelled.
- `uring-sqpoll` with its SQPOLL thread on the loop's SMT sibling, against
  mio with SMT off, both on one physical core (TCP round trip, 1
  connection, AWS `c7i.4xlarge`, the IRQ CPU capped at C1): the sibling
  layout is 2.3 to 7.3 µs lower at p50 and 5.4 to 15.4 µs at p99 than mio
  on the same hosts at 10,000 to 200,000 msg/s, and level with SQPOLL on
  a core of its own to 50,000 msg/s (4 to 6 µs dearer at 200,000).
  `epoll-spin` and mio are level. Tails are not ranked: millisecond round
  trips in every arm, most unexplained. The book's backends page and the
  Kubernetes x86 `cpu: 2` layout recommend it for one busy connection on
  one core; `io/benches/BENCHMARK.md` has every figure.
- Onload over AF_XDP on AWS ENA: not viable (kernel fault at NIC
  registration, instance froze), no latency measured; the Onload runner
  arms are marked experimental.

## [0.1.34] — 2026-10-27

### Added

- `ergon-io`: `Config::busy_poll_us` (key `busy_poll_us`, default 0,
  `epoll-spin` only, 0 to `i32::MAX`): `SO_BUSY_POLL` on every socket the
  driver owns, and a socket read with data is read once more on the
  next poll, so the empty read polls the NIC once, as mio's read to
  `WouldBlock` with the same option does. Other backends, or a value above
  `i32::MAX`, are `Error::Config`.
- `ergon-io`: UDP, `udp::UdpConn` (`UdpOptions`, `UdpHandler::on_datagram`,
  `UdpStats`), on `epoll-spin` only: `Driver::<UdpConn>::new` on
  `recv-busy` or `uring-sqpoll` is `Error::Unsupported`. Connected
  (`UdpConn::connect`) and receive-only (`UdpConn::bind`) sockets; whole
  datagrams sent from preallocated slots (`Error::Busy` when full); 0-byte
  datagrams delivered; a datagram over `max_datagram` (`MSG_TRUNC`) and
  `ECONNREFUSED` counted, not a close. No TCP option or route check on a
  datagram socket.
- `ergon-io`: framed TCP, `frame::FrameConn` (`FrameDial`,
  `FrameOptions`, `FrameHandler::on_frame`, `FrameStats`): `u32`
  little-endian length-prefixed frames over plain TCP, on every backend,
  dialled or from an accepted socket (`FrameConn::from_stream`, then
  `Driver::add`). A received frame over `max_frame`, or a peer close inside
  a frame, closes with the new `Error::Frame`. `TCP_NODELAY` always;
  `FrameOptions::quickack_per_read` sets `TCP_QUICKACK` after each read
  with data.
- `ergon-io`: `Config::epoll_events` (key `epoll_events`, default 64), the
  `epoll_wait` batch, and `Config::datagram_burst` (key `datagram_burst`,
  default 16), the most datagrams one poll reads from one ready UDP socket.
- `ergon-io`: source-IP binding. `Target::source` (`with_source`) binds
  the socket to a local address before it connects, on every dial,
  `Driver::connect`, `connect_background`, `HttpClient` and libcurl
  (`CURLOPT_INTERFACE`); on Linux with `IP_BIND_ADDRESS_NO_PORT`. A
  source of the other address family is `Error::Config` before any
  connect.
- `ergon-io`: several source IPs, each paced by the application.
  `Reconnect::poll_sourced` / `poll_at_sourced` ask a gate for the IP of
  every attempt (`Ok(ip)`, or `Err(wait_ns)` when none has room, not a
  failed attempt) and bind the dial to it (`net::BindSource`, implemented
  by `WsDial`, `HttpDial`, `AnyDial`); `Reconnect::source` names the IP.
  `WsDial::target` and `HttpDial::target` return the dial's target.
- `ergon-io`: `RxMeta::rx_read_ns`, ergon-io's own receive time
  (`CLOCK_REALTIME`, the domain of `rx_sw_ns`), read right after each read
  returns data when `Config::rx_read_timestamp` (key `rx_read_timestamp`)
  is on. Off by default: no clock read, the field is 0. Bytes the
  connection already held when the driver took it (read with the upgrade's
  101, or by a blocking connect before `Driver::add`) are stamped when the
  driver takes them, with a `read_seq` like any read.

- `ergon-io` (repo only): a two-sided mio parity gate,
  `io/benches/parity/mio_parity.rs` (`just io-parity`, CI on Linux x86_64
  and aarch64, lane `io-parity`). Structural tests assert from the
  `Driver`'s counters that `epoll-spin` makes one `epoll_wait` per poll with
  a zero timeout and one `recvmsg` per ready socket, and that a mio loop
  makes two reads per event. A loopback TLS WebSocket round trip through
  the `Driver` and through a minimal mio loop doing the same work, the two
  taking turns round trip by round trip, fails if either side's p50 is
  ahead by more than 5 % in 6 of 7 pairs; 10 % added to either loop fails
  it. The docs now state that `epoll-spin` is mio's (and so
  tokio's) mechanism on Linux.
- `samples/low-latency-k8s`: the Kubernetes reference for the low-latency
  stack. The DPDK media driver runs Aeron `DEDICATED`, with sender and
  receiver on two exclusive CPUs (`ergon-cpus isolated:0/1`) and everything
  else on the shared CPU. An Aeron ping and pong each busy-spin on one
  exclusive CPU, with 100 shared millicpus for the client conductor,
  `ll-log` and `ll-stats`. Logging is `tracing-subscriber` fmt to stdout
  through a lossy `tracing-appender` writer. `ll-probe` and
  `scripts/verify.sh` print PASS/FAIL per thread and per exclusive CPU.
  Also: the kustomize overlays `vdev-local`, `vdev-isolated`,
  `eks-ena-arm64` and `eks-ena-x86`, Helm values for `ergon-shared-cpus`,
  node preparation (`node/eks-userdata.sh` for EKS), a multi-arch
  `baseline` image plus CPU tags, and a vdev e2e in Docker that has run on
  linux/arm64. The `ll-latency` report is `v2` (`nic=`, `write_errors=`,
  an exact `max`).
  `md-dpdk/deploy/low-latency/pod.yaml` is removed (no hugepages, no Aeron
  mount, no `ergon-cpus`); its README points at the sample.
- `ergo-sbe`: `GenerationConfig::with_validate(true)` (off by default) emits
  `validate()` on every message decoder, group entry decoder and
  `AnyMessage`: an opt-in second pass after `try_decode` (unchanged) for
  frames from older or foreign producers. It returns the new
  `sbe_rt::ValidateError`: `ValueOutOfRange` (outside a declared
  `minValue`/`maxValue`), `UnknownEnum`, `UnknownSetBits`, `UnknownTemplate`
  (`AnyMessage::Unknown`) or `Decode` (a malformed tail). Optional nulls
  pass. Not checked: undeclared bounds, floats, fixed char arrays and
  var-data contents. A member named `validate` is
  `GenerateError::InvalidConfiguration` when the option is on. Generated
  `sbe_rt` always contains `ValidateError`.
- `ergo-sbe`: `ConversionSelector::named_type` selects fields of a primitive
  alias (`<type name="Timestamp" primitiveType="uint64"/>`); it no longer
  reaches bare primitives.
- `ergo-sbe` IR: `Encoding::min_value_declared`,
  `Encoding::max_value_declared` (whether a bound came from the schema or is
  the SBE default) and `Encoding::referenced_type` (the alias a primitive
  field refers to).
- `ergon-shared-cpus` 0.1.2 (image and chart): the
  `--kubelet-config-dir` flag and chart value `hostContract.kubeletConfigDir`
  (off by default; a read-only `Directory` mount; needs
  `hostContract.kubeletConfig`). The hot-pod node check merges the
  kubelet's `--config-dir` drop-ins over `--kubelet-config` the way the
  kubelet does: only `*.conf` files, lexical order, subdirectories included,
  each a JSON merge patch. A missing or unreadable directory, or a drop-in
  the kubelet would reject, refuses hot creates. The `configz` and argv
  cross-checks compare against the merged config. On EKS AL2023 (nodeadm's
  `config.json.d/40-nodeadm.conf`) install with
  `--set hostContract.kubeletConfig.path=/etc/kubernetes/kubelet/config.json
  --set hostContract.kubeletConfigDir.enabled=true`. Verified on EKS 1.36
  AL2023 `c8g.xlarge` (`k8s/nri-shared-cpus/RELEASE.md`).
- `ergon-shared-cpus` 0.1.2: the hot-pod NIC check reads
  a Hyper-V synthetic NIC (Azure `hv_netvsc`, which has no PCI interrupt)
  by its VMBus channels: each `device/channels/<relid>/cpu` must be an IRQ
  CPU, and an unreadable one refuses. Before, every hot pod on an Azure VM
  was refused with `no NIC interrupt found`.
- `ergon-io`: opt-in `tcp_rto_min_us` and `tcp_delack_max_us`
  (`TCP_RTO_MIN_US`, `TCP_DELACK_MAX_US`, Linux 6.15+) on every connection
  and libcurl socket, 2,000 to 200,000 µs. `Driver::new` and
  `HttpClient::new` set them on a probe socket and refuse, typed, at start:
  `Error::Unsupported` before 6.15, and `Error::Config` below two kernel
  ticks or when the kernel would round the value (user space cannot read
  `CONFIG_HZ`). Effect not measured.
- `ergon-io`: opt-in passive `TCP_INFO` (`tcp_info_interval_ms`,
  `Driver::tcp_info`, `TcpInfo`: RTT, RTT variation, min RTT, RTO, total
  retransmits, lost). It reads one connection per 250 µs deadline check, so
  at most one `getsockopt` per poll. Counters: `DriverStats::tcp_info_samples`
  and `tcp_info_max_ns`.
- `ergon-io`: the warning thread reads the polling thread's page faults
  (T19, `ergon_tune::FaultWatch`) every 5 s and writes a line when it took
  one. `Driver::new` opens it on the thread creating the driver, once per
  thread.
- `ergon-tune` checks the node's background noise on the busy-spinning CPUs:
  T13 (the unbound workqueue cpumask includes a hot CPU, warn), T14
  (`vm.compaction_proactiveness` or `vm.compact_unevictable_allowed` on,
  info), T15 (another process's task may run on a hot CPU, read from
  `/proc`, warn), T16 (kprobes, ftrace hooks or BPF programs loaded, info;
  unknown without privilege or with `bpf_jit_harden` on), T17 (KSM running,
  warn) and T18 (steal on a hot CPU since boot, info; `cpu_steal` reads it
  per CPU). `FaultWatch` (T19) warns when a hot thread takes a page fault
  while it runs. Each reads `/sys` and `/proc` only, and reports unknown,
  never a warning, when it cannot read. An unknown from any of them does
  not stop an `ERGON_HOST_CHECKS=require` start. T13–T18 have no caller in
  ergon-io or the media driver yet.
- `ergon-md-dpdk` diagnostics.
  - `RUSTERON_DPDK_GAP_ATTRIB=1` now also times each `rx_burst` that
    returns frames and each `tx_burst`: `rx-burst-max-ns`,
    `rx-bursts-over-1us`, `rx-bursts-over-5us` and the `tx-` three (type 87).
  - `RUSTERON_DPDK_EVENT_LOG=<file>` (lab only) writes stamped events. Agents
    log loop gaps of 10 µs or more and ARP traffic. Housekeeping logs the
    timer service, the 1 Hz sample and NIC statistics reads, each with its
    duration. Each writer has a preallocated ring; the agents never write
    the file.
- `ergon-md-dpdk` queue tuning: `RUSTERON_DPDK_RX_FREE_THRESH` and
  `RUSTERON_DPDK_TX_FREE_THRESH` set the queues' free thresholds (each queue
  starts from the PMD's default configuration), and
  `RUSTERON_DPDK_TX_FAST_FREE` turns on `RTE_ETH_TX_OFFLOAD_MBUF_FAST_FREE`
  where the NIC offers it. Defaults are unchanged. No setting is
  recommended yet.
- `ergon-md-dpdk`: `RUSTERON_DPDK_EAL_IN_MEMORY=1` starts the EAL
  `--in-memory` (no hugepage files, none left after an OOM kill; no
  multi-process channel). EAL options in `RUSTERON_DPDK_EAL_ARGS` that
  contradict the choice fail start with `DpdkError::InvalidEnvironment`.
  Default off; latency effect not measured.
- `ergon-md-dpdk`: an ENA running ENA Express with UDP over SRD
  (`ena_srd_mode` bit 1) warns at start when `max_aeron_mtu` is above 8864:
  SRD carries IP packets of at most 8900 bytes, and ENA sends larger ones
  without SRD (measured with kernel UDP on c6gn.4xlarge: delivered, not dropped).
- `ergon-md-dpdk`: `nic-link-changes` (type 79) counts link-status changes
  where the NIC raises them (`RTE_ETH_DEV_INTR_LSC`); `port-info` says
  `lsc=off` otherwise.
- `ergon-md-dpdk`: `RUSTERON_DPDK_GAP_ATTRIB=burst` times the PMD's
  `rx_burst` and `tx_burst` calls without the work/descheduling split. `1`
  also reads the thread CPU clock on every poll, a system call with no vDSO
  path (about 270 ns a poll in the fake-port bench); a latency run should use
  `burst`.
- `ergon-md-dpdk`: `RUSTERON_DPDK_TX_IDLE_CLEAN=<1..1024>` (off by default).
  On a poll that received nothing, the agent reclaims up to that many sent
  descriptors (`rte_eth_tx_done_cleanup`) once the last send is 5 µs old, so
  the PMD rarely reaches its free threshold inside a send. An idle poll with
  nothing outstanding makes no call. New counter `tx-idle-cleaned` (type 87);
  `port-info` shows `tx-idle-clean=<n>`, or `tx-idle-clean=unsupported` where
  the PMD lacks the op (found at start, left off and logged). Latency effect
  not measured.
- `ergon-md-dpdk`: `RUSTERON_DPDK_NIC_SLOW_STATS_MS` (unset by default: no
  change) splits the NIC statistics reads. Every `RUSTERON_DPDK_NIC_STATS_MS`
  only the PMD's software xstats are read, by id (on ENA the queue stats,
  `wd_expired`, `dev_start`, `dev_stop` and `tx_drops`, with no admin
  command); the basic drop counters and every xstat (ENI allowances, SRD) are
  read every `NIC_SLOW_STATS_MS`, `0` meaning never. Latency effect not
  measured.
- Cloud lab: `@HAIRPIN=1` on a raw floor mode (`dpdk-raw@HAIRPIN=1`,
  `kernel-raw@HAIRPIN=1`) runs the round trip on host a alone, from its
  sender ENI to its receiver ENI and back (`floor xhairpin`: ping and pong in
  one process, one port each; on the kernel wire vm.sh routes the host's own
  addresses out of their NICs, checked fail closed). `@RTT_GAP_US=<us>`
  (1 to 1000000) paces `rtt` and `floor` to one ping every that many
  microseconds (`RTT_GAP_US`) on a fixed schedule, a rep of 20 to 60 s.
  A hairpin needs four cores on both hosts. `analyse.py` adds a
  hairpin-against-cross-host table and a round-trips-by-gap table; a paced
  Aeron mode's floor is the floor paced alike.
- Cloud lab: `@DPDK=stock|patched-refill` (AWS, DPDK modes) runs the media
  driver and clients linked against DPDK 25.11 built from source on each VM
  (`vm.sh build-dpdk`, any placement). `patched-refill` applies
  `benches/cloud/dpdk-ena-deferred-refill.patch` (lab only, never shipped):
  ENA's `rx_burst` stops refilling the RX ring in a burst that returns frames
  until two thresholds are owed; the next empty burst refills. `vm.sh driver`
  stops unless the driver log shows the PMD the build should load. New ENA
  counters `rx_q<n>_refill_deferred` and `rx_q<n>_refill_floor`. `vm.sh host`
  (the env file) records each build's DPDK prefix and patch hash.
  `patched-refill` refuses an `RX_FREE_THRESH` of half the receive ring or
  more. Latency effect not measured.
- Cloud lab: raw floor modes `kernel-raw` and `dpdk-raw`. The new `floor`
  client does `rtt`'s round trip with no Aeron, over kernel UDP or the DPDK
  transport's bindings alone, with the same output files; `analyse.py`
  reports each Aeron mode above its floor. Under precision-time, `rtt` splits
  each spike one way by both hosts' NIC receive timestamps (`RTT_ONE_WAY`),
  and `spikes.py` prints the split. `LAB_STATES` accepts a `-nodiag` suffix
  (no tracepoints, kprobes, KNOISE or gapspin). A mode's
  `@RUSTERON_DPDK_ENA_DEVARGS` joins several device arguments with `+`.
  `estimate.py` prices `c8g.2xlarge`. Each run records the DPDK build
  (`dpdk-build`), and the jitter snapshots record `sw-checksum-verified` and
  `tx-idle-cleaned`.
- Cloud lab: pure floor mode `dpdk-pure` (AWS, cross-host). The new
  `purefloor` C client (`benches/cloud/purefloor.c`, built on the host by
  `vm.sh build`) does the floor client's round trip over DPDK alone, without
  ergon-md-dpdk: `rte_eth_rx_burst` and `rte_eth_tx_burst` on the receiver
  port, ARP to the next hop, and a pong that swaps the 5-tuple and sends the
  frame back. The port's rings, pool, LLQ policy, checksum offloads,
  link-change interrupts and RX timestamps are set as the transport sets
  them. It times with the TSC or `CNTVCT_EL0` and writes the same output
  files. `analyse.py` reports `dpdk-raw` above it, and sets a mode with
  overrides against the floor with the same overrides where that ran.
- `ergon-md-dpdk` bench: `clock.rs` overhead adds a `poll_idle` path and
  per-call p99.99 and max.
- EKS lab (`eks-nri.sh`): when no zone of `EKS_REGION` can place two spot
  nodes, it searches every enabled region, probing each candidate zone with
  two instances it then deletes. Next it tries `c9g.xlarge` and
  `c7g.xlarge`, which have the same 4-vCPU, no-SMT CPU pins, and only then
  on-demand, capped by `EKS_ONDEMAND_MAX_HOURLY` (0.55). The market and type
  that ran are logged and written to `instances.txt`. A failed instant fleet
  is still deleted before the next zone.
- Cloud lab: `LAB_GAPSPIN_US` (50 by default) sets the spare CPU's gap
  spinner threshold. The diagnostics trace times each interrupt and IPI
  (their exits too) on `CLOCK_MONOTONIC`, anchored to the wall clock by
  trace markers. `spikes.py` labels every agent loop gap in the DPDK event
  log by what overlapped it on the agent's CPU: a preemption, interrupts (with
  their time inside the gap), a VM pause the spinner also saw, or nothing the
  guest saw. It then counts each host's spinner gaps by size.
- `ergon-md-dpdk`: `ConfiguredDriver::from_env` asks each port's gateway for
  its MAC before the agents start (up to 1 s). A silent gateway is a warning
  line; `ERGON_HOST_CHECKS=require` refuses to start on it.
- `ergon-md-dpdk` on Graviton4 (c8g, CPU part 0xd4f) and Graviton5 (c9g,
  0xd84) on the EKS DPDK path (VM types; Graviton bare metal is not
  supported yet and `eks.sh` refuses it). `provision-node.sh` builds the
  arm64 kernel line (no `nosmt`; `iommu.passthrough=1`) and fails closed on
  any other CPU, on pages other than 4 KiB and without 2 MiB hugepages. The
  media-driver and device-plugin images build for linux/arm64 as well as
  linux/amd64. `eks.sh` defaults to c8g.2xlarge.
- `media_driver --version` prints the architecture, Rust's target CPU and
  the native C's flag.
- `scripts/test-dpdk-vdev-rootless.sh` (`just test-dpdk-vdev-rootless`,
  `verify-dpdk-docker --rootless`) runs every vdev scenario without root,
  through user namespaces, `net_tap` and EAL `--no-huge`. It fails, naming
  `kernel.apparmor_restrict_unprivileged_userns`, when the namespace is
  refused. CI runs it beside the sudo lane on ubuntu-24.04 and
  ubuntu-24.04-arm.
- New vdev scenarios, asserted on counters and logs: `icmp_ping`,
  `peer_mac_move`, `silent_gateway`, `largest_mtu` (8960; 8992 refused).
  Property tests for the receive classifier.
- Lab (`md-dpdk/benches/cloud`), attribution of idle maxima: the cross-host
  round trip stamps every round trip of 30 µs or more and every client loop
  stall of 10 µs or more (`LAB_RTT_SPIKE_NS`, `LAB_RTT_STALL_NS`; `vm.sh
  client` now passes `LOOP_STALL_NS` through sudo). `rtt` spike rows carry
  the ping index. `spikes.py` lists each rep's 10 largest round trips with
  every overlapping event (client stalls, the DPDK event log, spinner gaps,
  guest preemptions, KNOISE) and their phase in 1 s and 30 s, each source
  set against chance (1,000 circular shifts, fixed seed). `diag-start` adds
  KNOISE: osnoise with `NO_OSNOISE_WORKLOAD` on the agent and app CPUs, IPI
  senders by callsite, compaction and page migration. It refuses while a
  rep runs, and records `osnoise unavailable` where the kernel lacks the
  tracer or cannot set `NO_OSNOISE_WORKLOAD` (before Linux 6.3). The jitter
  snapshots keep the PMD burst counters, and `analyse.py --ab` prints them
  per rep.

- `ergon-io`: `WsHandler::on_message_start` (default `Interest::Keep`)
  decides from a data message's first frame whether to keep it.
  `Interest::Skip` drops it without a copy, a UTF-8 check or a delivery;
  `Interest::Fragments` delivers a fragmented message to the new
  `WsHandler::on_fragments` as `Fragments`, one slice of the receive buffer
  per fragment (up to `MAX_FRAGMENTS`, 32), instead of copying it. A message
  the buffer cannot hold that way falls back to one copy, counted in the new
  `WsStats::frag_fallbacks`; `WsStats::skipped` counts dropped messages.
  New types: `Interest`, `Fragments`, `FragmentIter`. `ws_bench` (in
  `io/benches`) times parse-and-deliver per message for each case.
- `ergon-io`: `RequestTemplate` holds a request's line around its path and
  the endpoint's fixed headers (`Host`, `Target::headers`), built and
  checked once. `HttpConn::request_with` and
  `HttpConn::signed_request_with` (also on `ConnMut<HttpConn>`) write only
  a target suffix, per-request headers, `Content-Length` and the body; the
  bytes on the wire equal `HttpConn::request`'s (ciphertext compared at
  every request size up to one record). A template from another target is
  refused.
- `ergon-io`: `HttpOptions::non_ascii_head` (`NonAsciiHead::Flag`, the
  default, or `Reject`): a response head with a byte above 0x7F is delivered
  with the new `Response::head_ascii` false and counted in
  `HttpStats::non_ascii_heads`, or closes the connection.
  `HttpStats::head_parses` counts head parses. `http_bench` (in
  `io/benches`) times the previous and current HTTP paths interleaved.
- `ergon-io`: one text check for WebSocket text and HTTP bodies, in the new
  `text` module (re-exported at the crate root). `TextCheck` is `None` (the
  default on both), `Utf8` (invalid text closes the connection), `Ascii`
  or `AsciiChecked(NonAscii)`. `Ascii` is a trust declaration: the payload
  is never read, costs the same as `None`, and is delivered unchanged as
  `TextStatus::AssumedAscii`, whatever its bytes. `AsciiChecked` is one
  pass checking every byte is below 0x80, a payload with another byte
  closed on (`NonAscii::Close`), dropped (`Skip`, WebSocket only:
  `HttpOptions` refuses it with `Error::Config`) or delivered flagged
  (`DeliverUnchecked`, the default policy). The new `RxMeta::text:
  TextStatus` (`Unchecked`, `Utf8`, `Ascii`, `AssumedAscii`, `NotAscii`)
  says what was found; `Ascii` means every byte index is a character
  boundary. `WsOptions::text_check` replaces `validate_utf8` and checks
  kept messages only (`WsStats::non_ascii` counts non-ASCII text under
  `AsciiChecked`); `HttpOptions::body_check` checks every response body. A
  response head keeps its own rule (`non_ascii_head`).
- `ergon-io`: `AsciiText<'a>`, a `Copy` byte view over `&'a [u8]` built
  from a payload at no cost: `as_bytes`, `len`, `is_empty`, `get` and
  `Index` by byte, `slice` by byte range, `find`, `position`,
  `starts_with`, `split_at`; nothing allocates. The `unsafe`
  `as_str_unchecked` returns a `&str` without a check (the caller vouches
  the bytes are ASCII; debug builds assert it).
- `ergon-io`: `Interest::Stream` hands each fragment of a fragmented
  message to the new `WsHandler::on_fragment(conn, opcode, part, index,
  fin, meta) -> Flow` as soon as it is complete, borrowed from the receive
  buffer: no copy, nothing held, no fragment or size limit but
  `max_message`. `Flow::SkipRest` drops the remaining fragments uncopied
  and unchecked. Text checks run per fragment, a character cut between
  fragments carried across (at most 3 bytes). Pings and Close inside are
  handled as before.
- `ergon-io`: `ws::push_frame`, `ws::push_header`, `ws::push_masked` and
  `ws::header_bytes` build masked client frames onto a `Vec` without a
  zero fill; `WsConn` sends with them, byte-identical to `encode_frame`
  (`io/tests/ws_send.rs` compares every length from 1 to 16 KiB and frames
  over several records).
- `ergon-io` (repo only): `ws_bench --suite tx|text|frag|span`: send cost
  per message old against new, `TextCheck` modes on ASCII and mixed text,
  `Stream` against `Fragments` and `Keep`, and the split-frame join old
  against new, each interleaved on the same input.
- `ergon-io` (repo only): `http_bench --suite body` times one response per
  row through the four `body_check` modes interleaved, on ASCII and mixed
  bodies, each mode's flag checked first.
- `ergon-shared-cpus` lab (repo only): a scale step (`vm.sh scale`,
  `LAB_SCALE=1`) on two shared CPUs records per-CPU busy time, each
  container cgroup's usage, `cpu.weight`, `cpu.max` and throttling, the
  shared CPUs each pod got and where every thread ran, for pods at 100 and
  1,001 millicpus, idle helpers and three pods started in turn, plus the
  scheduler's accounting (`results/scale.md`). A traced plugin-on rep
  (`LAB_TRACE=1`) enables ftrace events filtered to the spin's CPU, with
  the sender's stack on every wakeup, IPI, cross-call and work item sent
  there, and `gapprobe trace-report` names every switch, IPI, timer and
  work item and ties each to the gap log (`results/trace-cpu<N>.md`).
  `LAB_QUIET=1` applies the node settings of `lab_quiet_settings`. The
  scale report also shows the pod cgroup's throttling (a pod-level quota
  throttles there, not in the container's `cpu.stat`) and a placement
  verdict per pod (every sample of the spin thread on its CPU alone, of
  every other thread on that pod's shared CPUs); the step samples from the
  IRQ CPU, and the contention pods spin 30 s longer each so the first one
  outlasts the window. The traced rep waits 20 s more before its spin and
  is a failure if tracing was armed after the spin started. Run on the lab
  VM (runs 20261024230400 to 20261024234841).
- `k8s/nri-shared-cpus/BENCHMARKS.md`: measured on one Azure Cobalt VM
  with THP `never` and the fair server off on the isolated CPUs, helpers
  on the shared CPU: p50 0.02 µs, p99.99 0.03 µs, max 4.81 to 5.52 µs over
  3 reps of 60 s, 0 involuntary switches, no interrupt on the spin's CPU;
  the gaps left (3.5 to 5.5 µs) have no guest kernel event: the
  hypervisor's floor (run 20261024234841). What reached the isolated CPU
  before, from the trace: `khugepaged`'s `lru_add_drain_all`, the fair
  server's 1 s timer, SSH logins' seccomp JIT. A traced rep at a systemd
  daily timer took a 40.60 µs max and one switch (a block-device ioctl's
  `lru_add_drain_all`, and BPF loads). Shared-CPU scaling (run
  20261024230400): `ceil(millicpus/1000)` shared CPUs, all of them when
  busy (0.99 and 1.99 cores), 0.04 idle, two pods on one shared CPU split
  0.33 : 0.66 by `cpu.weight` from the `cpu` request; a pod over the
  allocatable 2000 stays Pending. Raw files under `lab/results/`.
- `samples/low-latency-k8s`: `node/quiet-cpus.sh` and its oneshot unit
  `node/ergon-quiet-cpus.service` set THP `never` and the fair server's
  runtime 0 on every isolated CPU at each boot, after debugfs is mounted
  (`--check` is read-only). `node/eks-userdata.sh` installs and enables
  them before the node's reboot; `node/linux-vm-prep.sh` installs them and
  checks both.

- Transport comparison suite (repo only; `md-dpdk/benches/cloud/transport.sh`
  and `io/benches`): `aeron_lat` (Aeron IPC, spy, kernel or DPDK UDP,
  reliable or not), `udp_lat` and `tcp_lat` (mio or ergon-io), `mpsc_lat`
  and `hostcheck`, every arm measured by one shared core
  (`ergon-latency-core`): 64-byte messages, open-loop pacing, TSC stamps and
  a manifest per point. The Azure runner applies `transport-settings.env`
  and reads it back, sets netem, and runs each rep's arms in a seeded random
  order with an A/A arm. `fairness.py`, `spikes.py --transport` and
  `analyse.py --transport` check every point, label each max and build the
  tables. `tcp_lat --source-port N` binds connection i to port N + i and
  closes with a reset, so two arms can run on the same 4-tuples.

### Changed

- `ergon-io`: closing connections allocates nothing on the polling thread,
  for one connection or many at once, a peer close, `Driver::close` or a
  WebSocket Close handshake alike: the closed list has room for every
  slot, a `WsConn` holds room for the server's Close from the start, and
  `connect_background`'s helper mailbox is bounded and allocated once
  (1 024 messages). `connect_background` fails with `Error::Busy` when that
  mailbox is full; a closed connection that finds it full is freed on the
  polling thread.
- Breaking: `ergon-io` has no `limit` module (`Budget`, `BucketSpec`,
  `Charge`, `Class`, `Ticket`, `Limited`, `Resend`, `Why`, `SourceIps`
  and the rest) and no `outcome::Now`. Pacing to a venue's limits is the
  application's: gate connects through
  `Reconnect::poll_gated` and `ClientHandler::connect_gate` as before;
  several source IPs through `Reconnect::poll_sourced(connector, gate)`,
  whose gate picks the IP (`Ok(ip)`) or waits (`Err(wait_ns)`).
  `Verdict::Throttled` and `Verdict::Banned` now carry
  `retry_after_ms: Option<u64>` instead of a bucket and a hold end;
  `outcome::generic(resp, server_now_ms)` and
  `outcome::refused_upgrade(error)` no longer take a budget, and a 429
  with a malformed `Retry-After` is only an `Error::Http`.
  `io/benches`' `limit_bench` is gone with it.
- Breaking: `ergon-io` keeps the three backends measured in a busy-spin
  loop: `epoll-spin`, `recv-busy` and `uring-sqpoll`. `Backend::UringSpin`,
  `Backend::EpollBusy` and `Backend::UringNapi` (`uring-spin`,
  `epoll-busy`, `uring-napi`) are gone, so is `Backend::may_sleep`
  (nothing sleeps), and `Backend::ALL` has 3 entries. `Config` loses
  `busy_poll_us`, `wait_us`, `prefer_busy_poll` and `busy_poll_budget`,
  and `Config::parse` refuses those keys as unknown: `recv-busy` sets
  `SO_BUSY_POLL` to 1 µs and `uring-sqpoll` a 1 µs NAPI busy-poll timeout,
  as their defaults did. `sys::set_prefer_busy_poll` and
  `sys::set_busy_poll_budget` are gone. For `uring-spin`, use `epoll-spin`
  (level in the fair run) or `uring-sqpoll`.
- Breaking: `ergo-aeron-cluster` has one listener and one poll.
  `AeronCluster::poll(&mut listener, fragment_limit)` replaces
  `poll_egress`, `poll_egress_controlled` and `poll_state_changes`, and takes
  the listener directly: `EgressAdapter` is gone and the client always drops
  other sessions' messages. `EgressListener` needs only `on_message`; the
  lifecycle callbacks default to no-ops. Removed: the `controlled` module
  (`ControlledEgressAdapter`, `ControlledEgressListener`,
  `ControlledPollAction`), the `poller` module with `parse_event` and
  `EgressEvent`, and `offer_range` (slice the payload),
  `send_admin_request` (use `send_admin_request_to_take_snapshot`) and
  `track_ingress_publication_result`.
- Breaking: `ergon-tune` drops `CheckId`. `Finding::id` is a `&'static str`
  (`"T1"`, `"K2"`, `"M1"`), so a caller can add checks of its own.
  `CheckId::unknown_stops_require` is now `unknown_stops_require(id)`, and an
  id it does not know blocks a `require` start when unknown (fails closed).
- Breaking: `ergo-sbe` has one parse and one generate entry point.
  `parse(source)` takes a `SchemaSource`: `SchemaSource::file(path)` (or a
  `&Path` / `PathBuf`) or `SchemaSource::xml(text)`, with optional
  `.with_shared(&ir)` and `.with_xsd_validation()`. `generate(source,
  config)` parses and generates in memory; write the result with
  `GeneratedModuleSet::write_to_out_dir()` or `write_to(dir)`. A bare
  `&str` is no longer accepted, because it could be a path or XML.

  | Removed | Use |
  |---|---|
  | `parse(xml)` | `parse(SchemaSource::xml(xml))` |
  | `parse_file(path)` | `parse(SchemaSource::file(path))` |
  | `parse_with_shared(xml, &ir)` / `parse_file_with_shared(path, &ir)` | `parse(SchemaSource::xml(xml).with_shared(&ir))` / `SchemaSource::file(path).with_shared(&ir)` |
  | `parse_with_xsd_validation(xml)` | `parse(SchemaSource::xml(xml).with_xsd_validation())` |
  | `generate_to_out_dir(path, config)` | `generate(SchemaSource::file(path), config)?.write_to_out_dir()` |
  | `generate_to_dir(path, config, dir)` | `generate(SchemaSource::file(path), config)?.write_to(dir)` |
  | `generate_str_to_out_dir` / `generate_str_to_dir` | `generate(SchemaSource::xml(xml), config)?` then a writer |
  | `generate_multi_to_out_dir` / `generate_multi_to_dir`, `SchemaFile` | `parse` each consumer `with_shared`, `Generator::generate_multi` with `GenerationConfig::with_shared_module(name)` (the helper set it for you; without it each module emits its own copy of the shared types), then a writer |
  | `out_dir()` | `write_to_out_dir()` reads `OUT_DIR` |

- Breaking: `Ir` has a `sources` field: the schema file, every resolved
  include and the shared schema's files. The writers print
  `cargo::rerun-if-changed` for each, so multi-schema builds through
  `Generator::generate_multi` watch every file too. A struct literal needs
  `sources: Vec::new()`. The watched paths are canonical (absolute), not the
  path as passed, and two `Ir`s parsed from the same text via a file and via
  `SchemaSource::xml` are no longer `==`.
- Breaking: a consumer parsed `with_shared` also carries the shared schema's
  composites, enums and sets (not its messages), so it generates on its own.
- `GenerateError` implements `miette::Diagnostic`, so a `build.rs` returning
  `miette::Result` can use `?` on `Generator` results.
- `ergon-shared-cpus`: `schedFifo` takes the same spellings as
  `seccomp.ioUring`: its mode word or a YAML boolean, any case
  (`true`/`yes`/`y`/`on`/`1` = `thread`; `false`/`no`/`n`/`off`/`0` = `off`).
  Unquoted `schedFifo: off` no longer fails at start. The chart schema
  accepts a boolean, `0`/`1` or those words for both fields. Mode words are
  now case-insensitive in both.
- `ergon-io` (Breaking): `Target` has a new public field `source`,
  `RxMeta` a new field `rx_read_ns` and `Config` a new field
  `rx_read_timestamp`, so struct literals that name every field need
  them. `net::connect_nonblocking` takes the source as a second argument.
- `ergon-io` docs: `TextCheck::None`, the default for `WsOptions::text_check`
  and `HttpOptions::body_check`, is documented as what it is: no check, the
  payload delivered as bytes with no ASCII or UTF-8 promise, the default
  because it reads nothing. Under `Interest::Stream` with
  `AsciiChecked(NonAscii::Skip)`, a message whose later fragment is not
  ASCII stops mid-way with no `fin` fragment and no callback, and the next
  message starts at index 0; a test pins it. The first such drop on a
  connection now writes one warning line (queued to the warning thread).

- `ergon-shared-cpus` lab (repo only): one boot line serves one shared
  CPU (e2e, bench) and two (scale): exclusive CPUs 3 to the last,
  `reservedSystemCPUs: 0-2`, so the bench's spin moves from CPU 2 to 3 and
  the lab needs 7 vCPUs or more. The plugin-off arm runs only on request
  (`LAB_ARMS="plugin-on plugin-off"`; default `plugin-on`); the default
  plan is about 75 minutes and $0.44. `gapprobe` writes its gap log and
  histogram before the spin thread pins itself (filling the old log faulted
  one page in on the hot CPU per 256 gaps), counts the loop's page faults,
  and runs its UDP helpers in a child process by default (`--helpers
  process`; `LAB_HELPERS=thread` for the old layout). Its summary row has
  four more fields; `report` still reads the 16-field rows.
- `ergon-shared-cpus` lab (repo only): `LAB_QUIET=1` also sets
  `transparent_hugepage/enabled` `never` and
  `/sys/kernel/debug/sched/fair_server/cpu<N>/runtime` 0 on each exclusive
  CPU; SSH to the VM goes over one multiplexed connection (each login's
  seccomp JIT interrupted every CPU); the traced rep reads tracefs as root
  (it is mode 700); the backstop-timer check waits for cloud-init.
- Breaking: `ergon-md-dpdk` asks every ENA port for large LLQ entries
  (`llq_policy=3`, 256 bytes, from DPDK 24.11; before, `llq_policy=1`, the
  device's recommendation), and `RUSTERON_DPDK_TX_DESCRIPTORS` defaults to
  256 (was 512), the most an ENA TX ring takes under them. On a
  `c9g.2xlarge` pair, idle 32 B round trips were 0.7 µs lower at p50 and
  1.0 µs lower at p99, with rep ranges apart; max unchanged
  (`aws-20261023-231150`). `RUSTERON_DPDK_ENA_DEVARGS=llq_policy=1` restores
  the old policy, and with it a 512-descriptor ring. The EKS ConfigMap sets
  256.
- Breaking: `ergo-sbe` `encode_from`, DTO `encode` and `bulk_add_domain`
  check only the `minValue`/`maxValue` a schema declares, the same bounds
  `validate()` checks. An undeclared side is not checked, so any value of
  the Rust type is written as given (`u64::MAX` in a plain `uint64`,
  `i32::MIN` in an `int32`), as sbe-tool's flyweight encoder does. Before,
  every integer and `char` scalar was checked against the SBE default range.
  A value outside a declared bound is still `EncodeError::ValueOutOfRange`;
  `Some` of an optional field's null value is still `SomeNullValue`.
- Breaking: `ergo-sbe` `Encoding::deprecated` and `FieldInfo::deprecated` are
  `Option<u16>`: the schema version from the XML `deprecated` attribute, or
  `None`. A field that inherits a type's deprecation takes the earlier
  version. `with_deprecated_attrs(true)` emits
  `#[deprecated(note = "deprecated since schema version N")]` on decoder
  getters instead of a bare `#[deprecated]`.
- Tests: the ergo-sbe and cluster scratch-crate tests share one build
  directory (`<target>/ergon-scratch/`, `sbe/tests/common/scratch.rs`)
  instead of a fresh `target_ci` per test, so ergo-sbe and its dependencies
  compile once. ergo-sbe tests then run at default threads (local and CI;
  other crates keep `--test-threads=1`), scratch crates compile with cargo's
  recorded rustc command outside cargo's build lock (at most one compile per
  core; `ERGON_SCRATCH_VIA_CARGO=1` forces cargo), doc fences and the
  codegen-matrix and header-order loops check cases in parallel and still
  report the first failing case, and dependencies build at `opt-level = 2`
  in dev and test. `cargo test -p ergo-sbe --all-features` drops from about
  3.75 h to about 2 min warm on a 10-core Mac; every test and assertion is
  unchanged.
- Release path faster, every gate as strong. `just release` runs `just test`
  once: `release-check` is `test` plus `_release-check-extras` (coverage, book
  checks, publish dry runs, packaged checks, codegen cold path), and the
  release runs only the extras after its own `just test`;
  `check-publish-set.sh` fails if either path stops reaching the tests or the
  dry runs. `just release` starts with `_reset-sources` (submodules at their
  pins, `git clean -fdx`, sample codecs regenerated) instead of `just clean`,
  so warm target directories are kept; run `just clean` first for a cold
  build. The sbe-tool reference check is one Gradle run instead of 47 (33 s
  to 11 s). `just policy` and `just preflight` run independent scripts at the
  same time (`scripts/run-parallel.sh`: every script runs to the end, any
  failure fails; scripts that share a file stay in one command): policy 27 s
  to 7 s, preflight 76 s to 19 s warm. Fuzz replay builds once and runs the
  six targets at once, 30 s each (`scripts/run-fuzz-replay.sh`; about 4.8 min
  to 35 s). The coverage ratchet runs at default threads (618 s to 182 s,
  same TOTAL, same 1,219 tests). Gated benches pass `--noplot` (sampling
  unchanged); the LTO parity benches reuse the binaries `just test` built;
  `bench-historic` deletes its old estimates first and reads Criterion's
  output under `CARGO_TARGET_DIR`, so a stale estimate can no longer pass.
  `just test` no longer reruns ergo-sbe's doctests and `docs_validation_test`.
  `measure-codegen-cold-path.sh` works with `CARGO_TARGET_DIR` set.
- Breaking: `ergo-sbe` conversion selectors fail closed. A `field_path` or
  `semantic_type` that matches no field, a second `with_domain_type` /
  `with_manual_domain_type` type for one selector, and a domain-type
  selector that a higher-precedence selector takes on every field it matches
  are `GenerateError::InvalidConversion`. `with_null_as_option` on anything
  but `named_type(<enum>)` is `GenerateError::InvalidConfiguration`. All of
  these previously generated silently without the mapping.
- Breaking policy: 0.x has no backwards-compatibility gate. Any release may
  break the API, with no deprecation period, and this changelog records each
  break (CONTRIBUTING.md "Versioning", book "Road to 1.0"). Official SBE
  wire compatibility is unaffected.
- Breaking: `ergon-md-dpdk` no longer reads
  `RUSTERON_MEDIA_DRIVER_HOST_CHECK`. `ERGON_HOST_CHECKS` is the only switch
  for the media driver's host checks, as for ergon-io: `on`/`warn`
  (default), `off` (no check runs) or `require`. Rename the old variable;
  it is ignored if set. `checks::HOST_CHECK_ENV` is removed.
- Breaking: `ergon-tune`: `HostCheckPolicy::from_env_or` and
  `HostCheckPolicy::from_vars` are removed. `HostCheckPolicy::parse` takes
  only the value; the new `HostCheckPolicy::from_var(Option<&str>)` maps
  unset to `On`, and `from_env` uses it. `HostCheckError::Invalid` loses its
  `variable` field.
- `ergon-shared-cpus` chart 0.1.2: `hostContract.kubeletConfig`,
  `hostContract.sys` and `hostContract.proc` are on by default, so the
  DaemonSet passes `--kubelet-config`, `--host-sys` and `--host-proc` and
  mounts `/var/lib/kubelet/config.yaml` (File), `/sys` and `/proc`
  read-only. A pod labelled `ergon.io/latency: hot` is checked against the
  node instead of always refused with `kubelet configuration is not
  mounted`. A node without that kubelet file must set
  `hostContract.kubeletConfig.path`, or the plugin pod does not start.
  `appVersion` is 0.1.2, so the default image is 0.1.2. `deploy/daemonset.yaml`
  (`:dev`) also mounts `/sys` and `/proc`.
- `ergon-shared-cpus`: a key in the kubelet `--config` file that matches
  `cpuManagerPolicy`, `cpuCFSQuota`, `reservedSystemCPUs` or `featureGates`
  only when case is ignored is no longer taken; the kubelet decodes that
  file case-sensitively.
- `ergon-io`: `Driver::poll` sends what was queued since the last poll
  (through `get_mut`) before its receive pass. An order queued in idle work
  no longer waits behind `epoll_wait` and every ready feed's decoding. Cost:
  one more scan of the connections in a poll after a `get_mut`, and on
  `uring-spin`/`uring-napi` one more `io_uring_enter` when completions are
  also waiting. Not measured.
- `ergon-io`: the driver keeps a send set, the slots that may have bytes
  queued, in place of one flag and a scan of every slot. `get_mut`, a
  receive, a timer and an endpoint installed with bytes queued list a slot;
  a send pass visits only the listed slots and keeps those with bytes left
  (a full socket, an `io_uring` send in flight). A poll's send side costs
  O(queued), not O(connections), on every backend; the receive side is
  unchanged (`recv-busy` still reads every socket). The set is sized with
  the slots in `add`/`connect`, so `poll` never allocates for it. The send
  pass before the receive pass still runs only after a `get_mut` or an
  install. Not re-measured.
- `ergon-io` API break (struct literals of `Config` and `DriverStats`): new
  pub fields `Config::{tcp_rto_min_us, tcp_delack_max_us,
  tcp_info_interval_ms}`, `DriverStats::{tcp_info_samples,
  tcp_info_max_ns}`; new `TcpInfo`, `Driver::tcp_info`, `sys::{TcpTimer,
  set_tcp_timer, tcp_timer, probe_tcp_timers, apply_tcp_timers, tcp_info}`.
- `ergon-md-dpdk` API break: `DpdkTransportConfig` gains `rx_free_thresh`,
  `tx_free_thresh` and `tx_fast_free`. The native config struct grows from
  4328 to 4336 bytes.
- `ergon-md-dpdk`: the counter set's housekeeping-written sample times sit
  on a cache line of their own, apart from the agents' counter addresses
  (asserted at compile time).
- `ergon-md-dpdk`: an ARP refresh request goes out after the data frames of
  the send that found it due, and ARP/ICMP replies after a receive burst's
  data frames, never before them.
- `ergon-md-dpdk`: counters that only their port's agent writes are bumped
  with a relaxed load and a release store instead of an atomic add (about
  −3.5 ns per send and −8 ns per poll in the arm64 fake-port bench).
  `tests/dpdk_counter_writers.rs` derives each counter's writer threads from
  the native sources and fails unless the shared kinds are listed.
- `ergon-md-dpdk`: the `tx bytes` counter (type 83) counted the caller's
  running total instead of each send's bytes, over-counting on Aeron's
  multi-destination path. Fixed.
- `ergon-md-dpdk`: a SETUP whose MTU exceeds ours and an ICMP
  fragmentation-needed are formatted and recorded by housekeeping (a stderr
  line; one distinct-error-log entry per kind), not on the receiver agent.
  Fragmentation-needed now also reaches the driver's error log.
- `ergon-md-dpdk`: housekeeping no longer holds its lock across NIC
  statistics reads, so closing a transport never waits for an ENA admin
  command.
- `ergon-md-dpdk`: a failed native create keeps its type: a contradictory
  environment is `InvalidEnvironment`, a configuration the NIC cannot carry
  is `InvalidConfiguration` (was `Native`).
- `ergon-md-dpdk`: the native C follows Rust's target CPU: the last
  `-C target-cpu` in `RUSTFLAGS` becomes `-mcpu` (aarch64) or `-march`
  (x86_64), which `cc` does not pass on. A CPU name the C compiler rejects
  stops the build.
- Vdev fixture: a DPDK role sends only once its taps are bridged (an ARP
  race had cost `dpdk_to_udp` about 31 s); the bridge has a fixed MAC; stale
  reports are removed.
- `latency_distribution` times each operation between two ordered reads of
  quanta's counter, instead of 1,000-message batches with `Instant`. It
  prints p50, p99, p99.9 and max, the max's sample index, the samples at or
  above 1 µs and a first guess at the max's cause, against an empty `clock`
  arm. Medians shorter than one counter tick read 0, so take them from
  Criterion.
- `encode_bench` times a `NewOrder` frame through `fixed()` + `asset`
  against `encode_from`. `decode_bench` times mapped `Side` / `Decimal9`
  accessors against the wire enum, `raw_side()` and the mantissas. Both
  check bytes or values equal before timing. Neither is gated.

- Breaking: `ergon-io` `WsStats` has two more fields, `frag_fallbacks`
  and `skipped`: a struct literal of it no longer compiles.
- `ergon-io`: `WsOptions::validate_utf8` checks a text message only once
  the handler keeps it, after `on_message_start` and before delivery. Invalid
  UTF-8 in a skipped message no longer closes the connection (a deliberate
  relaxation of RFC 6455 §8.1). The check scans for ASCII first, then runs
  `str::from_utf8`.
- Breaking: `ergon-io` `WsOptions::validate_utf8` is gone: set
  `text_check: TextCheck::Utf8` for `true`. `RxMeta` has a new field
  `text`, and `WsStats` two more, `non_ascii` and `unhandled`: struct
  literals of them no longer compile. `WsHandler::on_fragments` returns
  `Flow`.
- `ergon-io`: a handler that returns `Interest::Fragments` or
  `Interest::Stream` without implementing `on_fragments` or `on_fragment`
  (whose defaults return `Flow::Unhandled`) gets the message joined by copy
  through `on_message`, counted in `WsStats::unhandled` and
  `frag_copy_bytes`. It used to be dropped.
- `ergon-io`: a frame cut across two TLS records is joined by moving the
  smaller side: the leftover (held fragments included, as one block)
  forward to the new plaintext when it is shorter, else the new plaintext
  back. `span_copy_bytes` counts the bytes moved.
- `ergon-io`: the send queue is drained from a head offset: a partial
  `epoll` send no longer moves the unsent bytes; the queue is compacted
  when it empties or its spare capacity runs low. A `WsConn`'s send queue
  and each `io_uring` send buffer are written once at setup, so the first
  sends take no page faults.
- `ergon-io`: `recv-busy` with `per_queue` looks a socket's NAPI id up after
  its first data and on the look-again every 4096 polls, no longer every
  poll while an id is unknown (loopback, a plain veth: one `getsockopt` per
  socket and an allocation per poll before).
- Breaking: `ergon-io` `Response` has a `head_ascii` field, `HttpOptions`
  has `non_ascii_head` and `body_check`, and `HttpStats` has `head_parses`
  and `non_ascii_heads`: struct literals of them no longer compile (add
  `..HttpOptions::default()`).
- `ergon-io` HTTP receive: a chunked body still arriving is copied out
  chunk by chunk as records come (its head first), once per byte, instead
  of being de-chunked again from the start on every record; the window
  empties, so later records are read in place. A chunked body sent as one
  chunk, whole in the buffer, is borrowed (no copy). A head is parsed once:
  a response whose body arrives in later records keeps its head as offsets.
  `HttpStats::chunk_copy_bytes` now counts every byte the de-chunking moved,
  a streamed response's head included (it counted only the last pass's
  body). The de-chunk buffer is allocated on first use, at `buf_size` minus
  two records, instead of 64 KiB at connect.
- `ergon-io`: `HttpConn`'s 64 KiB send queue is written once at connect
  (as `WsConn`'s is), so the first requests take no page fault. Header
  checks are faster (a token table, CR/LF/NUL scanned eight bytes at a
  time); what they accept is unchanged.
- `ergon-shared-cpus` 0.1.2 is on GHCR. Image
  `ghcr.io/mimran1980/ergon-shared-cpus:0.1.2` and `:latest` (index digest
  `sha256:33ddd06f6b2e58bae6a2bf13092366482b5a292bc50a01223b0b383b66026da6`).
  Chart version `0.1.2` (appVersion `0.1.2`) digest
  `sha256:145054055d604e8a866bf86556cd3a1b46b92b1b6e3347183b8033dd7112b14d`.
  `--version` on linux/amd64 and linux/arm64 printed
  `ergon-shared-cpus 0.1.2 85098f8b16e17d80747c73d6069c1ec07d70496a nri=v0.12.3`.
  Actions run 40173324072. The kind e2e passed at that source.
- Breaking: `ergon-md-dpdk`'s type-81 counters are renamed
  `nic-arrival-to-rx-burst-*` to `nic-arrival-to-dispatch-*` and now measure
  each NIC-stamped frame to its own dispatch. The host clock is read once per
  stamped frame instead of once a burst, so a frame later in a burst includes
  its wait behind the earlier ones. Without `RUSTERON_DPDK_RX_HW_TIMESTAMP=1`
  nothing is read.
- The harness's `load` percentiles include every measured message that was
  owed and due by the run's end. A message sent without an echo, or never
  offered, is recorded at its lower bound (the run's end minus its due time)
  and counted in `censored`. `samples` counts these as well as the echoes.
- The EKS NRI lab (`eks-nri.sh`) moves a partition mode's measured
  processes into the isolated child from the node through the privileged
  probe, because containerd mounts `/sys/fs/cgroup` read-only in an
  unprivileged pod. A pid that is not a task of that container's cgroup is
  refused. `just test-dpdk-lab` runs the EKS runner shell tests.

### Fixed

- `ergon-md-dpdk`: an Azure `net_netvsc` port with no accelerated-networking
  VF attached fails start with the new `DpdkError::NoAcceleratedVf`, naming
  the port, the cause and the fix. It ran on the synthetic VMBus path
  instead, slower than kernel UDP while looking like DPDK (seen with
  Mellanox VFs, which this set-up does not support; MANA VFs only).
  `RUSTERON_DPDK_ALLOW_SYNTHETIC=1` starts it anyway, with a warning, for
  diagnosis only. Breaking: `DpdkError` has a new variant.
- `ergon-io`: the warning thread writes each line in one `write(2)`, its
  newline included (it wrote the text, then the newline). Where stdout and
  stderr share one file, another thread's line could land between the two:
  a `udp_lat` pong's `ready` became `...the polling CPU is 2.ready`, and
  the transport runner, waiting for the line `ready`, timed out on a pong
  that was echoing.

- Transport bench (repo only): `ergon-latency-core` prefaults its spike and
  stall buffers through `black_box`. A release build had folded the zero fill
  into a lazily mapped `calloc`, so under overload every 4 KiB of stall rows
  page-faulted on the measuring thread, in mio's arms and ergon-io's alike.
- Transport bench (repo only): the `eio-tcp` arm's pong stages room for both
  of a poll's reads per connection and reads nothing while echoes wait.
  Overloaded, it had spun in its echo loop and the point failed (9 points at
  200,000 msg/s on Azure).

### Removed

- Breaking: `ergo-sbe` `GenerationConfig::with_error_from_impls`. Implement
  `From<generated::sbe_rt::EncodeError>` and
  `From<generated::sbe_rt::DecodeError>` on your error type instead (keeps
  `needed`/`available`).
- Breaking: `ergo-sbe` public fields `Schema::package`, `Schema::id` and
  `Schema::version`. Read `schema.ir.package`, `schema.ir.id` and
  `schema.ir.version`.
- The cargo-semver-checks gate: `scripts/check-public-api.sh`, its self-test
  `scripts/tests/test-public-api.sh`, CI's `semver` job and the semver step
  in `lint`, and `baseline_tag`/`first_release` in
  `api/public-api-baseline.toml`. `scripts/check-generated-public-api.sh`
  still diffs generated codec surfaces against `api/generated/*.txt` for
  review; CI's lint job now runs it directly.

### Docs

- The book's transport latency comparison has its final Azure results, with
  mio and ergon-io at equal busy polling: unreliable UDP at 1 to 128
  streams, Aeron reliable against TCP without and under 0.01, 0.1 and 1 %
  loss, every max attributed or listed as unexplained, and a conclusion.
  ergon-io's UDP p50 is 0.1 to 0.9 µs behind mio's; over TCP there is no
  verdict (run-to-run noise; a same-connection A/B is due on AWS). The
  first sessions' socket rows and the TCP fan-in set are dropped (unequal
  busy polling). `io/README.md`, `io/benches/BENCHMARK.md` and
  `md-dpdk/BENCHMARKS.md` carry the matching answers and counters. DPDK
  results stay out on Azure (synthetic VMBus path on mlx5 VFs); a DPDK rerun
  is planned on AWS.
- `docs/SBE_COMPATIBILITY.md`: `encode_from` and DTO `encode` check the
  declared bounds of integer and char scalar fields of messages and groups
  only; composite members and array elements are not checked on encode.
- `ergon-shared-cpus`: the root README has a short packages section (chart
  and image 0.1.1), which both GHCR package pages show. The plugin README is
  reordered (status, numbered quick start, chart values, develop last). The
  chart ships its own README (`helm show readme`). The shared-cpus and
  latency-pods docs, chart comments and kind sample describe the latest
  release only (no per-version notes; the CHANGELOG is the history). Every
  shared-cpus install command passes
  `--set hostContract.kubeletConfig.enabled=true --set
  hostContract.sys.enabled=true --set hostContract.proc.enabled=true`
  (valid for 0.1.1 and 0.1.2), and the prerequisites tables name what a hot
  pod needs from the node. The kind sample runs an unlabelled pod for the
  shared-CPU grant and shows a hot pod refused with `exclusive CPU not
  isolated` (kind has no `isolcpus`); not yet run on kind.
- The `ergon-shared-cpus` publish workflow sets the image description,
  source and licence as manifest-index annotations, so the GHCR image page
  shows a description from the next published tag. The chart job fails if
  that description is over 512 characters. The Dockerfile source label is
  now the repository root.
- `ergon-io`: the book records mio as measured and not a backend (the
  fair-run numbers, the reasons, and the maxima still without a cause).
  Corrected: every backend's loop pass grows with sockets, because the
  `Driver` checks each connection for queued sends on every pass. The book
  had said `epoll-spin`, `uring-spin` and `uring-sqpoll` did not scan idle
  sockets, and that `per_queue`'s cost stops at the queue count (its NIC
  polling does). The loop and receive-window counters cited are in
  `io/benches/results/flags-aws-20261019T175248Z/loop-counters.md`.
- `ergon-md-dpdk`: one "where a frame is lost" table in the book.
- Node tuning (`k8s/nri-shared-cpus/README.md`, the book's HFT, hosts,
  shared-CPU and latency-pod pages, `k8s/nri-latency-pods/node-checks.md`):
  THP `never` and the fair server's runtime 0 on isolated CPUs (kernel
  6.12+, debugfs, not persistent, only where nothing real-time runs
  there), with the trace that names each; keep systemd timers, pod churn
  and SSH logins out of trading hours, since `lru_add_drain_all` callers and
  BPF loads still reach every isolated CPU.

## [0.1.33] — 2026-10-23

### Changed

- `tokio` 1.54.1. 1.54.0 is yanked (`RUSTSEC-2026-0384`, a race in
  `Interval` on the multi-thread runtime).
- `ergon-md-dpdk` stops its agents and exits non-zero when a NIC asks for a
  device reset, so a DaemonSet can restart it. The transport does not reset
  the device under polling agents. Sender and receiver ENAs that report two
  different NUMA nodes are refused before the second pool is created, and
  the device plugin does not advertise that pair. An unknown socket does not
  fail the pair. `RUSTERON_MEDIA_DRIVER_MLOCKALL` stays unset on EKS.
- Pin `simple-binary-encoding` to sbe-tool 1.41.0
  (`5f4d6e2152a007918d638d4736106e2978012200`). A set choice with
  `sinceVersion` greater than 0 reads false from a message or group field
  when the acting version is lower, matching the Java and C++ codecs
  (sbe-tool issue 1035). The buffer is left as it arrived. A group
  `raw_<field>()` accessor still returns that wire word. A set member of a
  composite is not gated. The official Rust generator is unchanged in
  1.41.0, so the checked-in reference codecs were left in place. The
  comparator provenance record names 1.41.0.
- `ergon-shared-cpus` checks the mounted kubelet config, NIC IRQ affinity
  and isolated exclusive CPUs before a hot create. A failure refuses that
  create and leaves the plugin connected. Node bootstrap takes a separate
  IRQ destination (`RUSTERON_IRQ_CPUS`), which must be inside the
  housekeeping set. A hot container cannot turn the start-affinity hook
  off. The validating policy denies startup, readiness and liveness exec
  probes, lifecycle exec handlers, a shared-millicpu request missing from
  any application container, and a `required-plugins` value that is not the
  exact plugin token. Request `M = 1000 × |S|` with oversubscription 1 to
  receive the whole helper set. A reconnect keeps the recorded helper CPUs.
  `ergon-cpus show` prints `cpu_max`, per-thread allowed masks and the
  affinity rule. Admission of a Pod does not control a later `kubectl exec`.
  Chart `0.1.0` is not republished. Chart `0.1.1` is on GHCR. It renders
  every config field, `repository@digest` when `image.digest` is set, and
  the two admission files only when `admission.create` is true. It can
  select one named CPU profile from a node label at startup. Its NOTES
  still say the chart is not published. That text was packed with the
  push, and the tag is not republished to change it. EKS is not verified.
  Latency figures are not measured.
- `ergon-shared-cpus` reads the sync resend skip-set only while its lock
  is held, so a delayed re-widen cannot race `StopContainer`. The hot-pod
  admission rule converts resource strings with `quantity(...)` before
  calling quantity methods. Kubernetes 1.33 rejects
  `isInteger(string)` (`no such overload`). The try sample uses pod
  `restartPolicy: OnFailure` with container `resizePolicy: RestartContainer`,
  and deletes its kind cluster only after that run's `kind create` succeeds.
  If containerd on that kind node crash-loops with
  `failed to get metadata for stored sandbox`, the sample wipes that node's
  containerd state once and keeps waiting. It does not wipe a node whose
  containerd is up.
  The `:dev` DaemonSet mounts `/var/lib/kubelet/config.yaml` and passes
  `--kubelet-config`. Image `0.1.0` still rejects `nodeProfileLabel` and
  `--kubelet-config`. Chart `0.1.0` is not republished. EKS is not
  verified. Latency figures are not measured.
- `k8s/nri-shared-cpus/examples/try` installs the public chart on a
  throwaway kind cluster and checks the shared-CPU grant and the hot-pod
  admission denial. A run on 2026-10-22 (chart `0.1.0`,
  `kindest/node:v1.33.0`) printed `ERGON_ISOLATED_CPUS=2,3`,
  `ERGON_SHARED_CPUS=0`, `cpuset.cpus.effective=0,2-3`, `nproc=1`, and the
  admission policy denied the exec probe. A later run the same day, with
  the containerd recovery in the script, printed the same grant and the
  same denial. Containerd stayed up, so the wipe did not run. Chart 0.1.0
  still prints a stale "not released" note and does not emit `seccomp`,
  `ptp` or `schedFifo`.
- `ergon-shared-cpus` 0.1.0 is on GHCR. Image
  `ghcr.io/mimran1980/ergon-shared-cpus:0.1.0` (digest
  `sha256:d28fea2f95ebce3e53d33f8770f3ba69be75c76ec7437ea2b92c1329759e07e9`)
  and chart `oci://ghcr.io/mimran1980/charts/ergon-shared-cpus` version
  `0.1.0`. Anyone can pull them. EKS is not verified. Latency figures are
  not measured.
- `ergon-shared-cpus` 0.1.1 is on GHCR. Image
  `ghcr.io/mimran1980/ergon-shared-cpus:0.1.1` and `:latest` (digest
  `sha256:69893e0c23ec6c3656053c0d6c4a06c676e33b726d56f362a30ce760908796c0`).
  Chart version `0.1.1` digest
  `sha256:77a65305a45ea1b376ce80f9b1d5234b9061f4066a8c5baf5efdb892854fc0d8`.
  `--version` on linux/amd64 and linux/arm64 printed
  `ergon-shared-cpus 0.1.1 515846b0201b5fd70a7a6cf3a88839b6180fa9c1`.
  Actions run 39853440732. A kind install of this tag was not run.
  EKS is not verified. Latency figures are not measured.

## [0.1.32] — 2026-10-21

### Changed

- `ergon-tune` T10 names the hosts where EC2 will not set `ThreadsPerCore`:
  bare metal, and T2, C7a, M7a, R7a, Apple silicon Mac and Graviton
  (AWS *CPU options for Amazon EC2 instances*; bare metal from *Rules for
  specifying CPU options*). The finding stays a warning. `require` still
  refuses every warning, including this one.
- The FIX RLValidator test is compiled only with `--cfg ergon_rlvalidator`.
  `just test` does not pass that cfg, so the leg is absent from the default
  suite. With the cfg, a missing suite, class directory or classpath fails
  the test. `scripts/run-fix-sbe-conformance.sh` builds the suite, sets the
  cfg and stops if the suite is missing.
- Crate READMEs and rustdoc link the book. In the book, Samples follows
  ergo-sbe.
- CI removes the runner's preinstalled Microsoft apt source before
  `apt-get update` (that host returned 403, and nothing here installs from
  it). GitHub Action majors are the Node 24 releases.
- `proptest` 1.12.0. Lockfile: `smol_str` 0.3.6.
- The shared-cpus chart publish reads the job token on stdin. The plugin
  is still not released: no registry artifact, and no EKS measurement.
- The EKS workload image no longer copies the removed `ergon-clock` crate.
- Historical pages removed on 2026-10-18 are back in the tree (the
  AI-assistance account, dated benchmark write-ups, result tables,
  screenshots). A page that describes behaviour describes the current
  verified behaviour. A superseded figure is not the current result. The
  token table sums this MacBook's local `ccusage` with the other machine's
  high-water mark already in that file.

### Removed

- **Breaking (`ergon-md-dpdk`):** `DpdkError::UnsupportedPlatform` and
  `DpdkError::AlreadyInitialized`. Nothing constructed them. The `dpdk`
  feature does not build off Linux x86_64 or aarch64, and selecting
  `dpdk-ena` without it is `FeatureDisabled`. A second install while a
  leaked runtime is still up is `DpdkError::Native`.

## [0.1.31] — 2026-10-21

### Added

- `ergon-tune`: T11, how `CLOCK_REALTIME` is kept (time daemon, PTP-grade or
  NTP verdict, error bound, an unsettled correction, a pending leap second,
  unsynchronised), and T12, the kernel's clock source (the counter, a
  hypervisor's view of it, or `hpet`/`acpi_pm`, read by system call).
  `ClockWatch` checks both again from housekeeping without waiting
  (chronyd's answer is read one check late) and allocates only when a
  finding changes. Ported from `ergon-clock`'s time-sync probe. T11 makes
  the crate's one `unsafe` call, a read-only `adjtimex`, until rustix wraps
  it.
- Book: Deployment → [Clocks](https://mimran1980.github.io/ergon/deploy/clocks.html),
  which clock to read for what, and the chrony set-up for AWS and Azure.

### Removed

- `ergon-clock` and its unpublished `ergon-clock-bench` leave the workspace
  and the release. 0.1.30 stays on crates.io and will be yanked, with
  `ergon-md-dpdk` 0.1.30, which depends on it. Replacements:

  | need | use |
  |---|---|
  | timestamps (wire, logs, storage) | `CLOCK_REALTIME`: `std::time::SystemTime` |
  | durations and timers in one process | `quanta` |
  | is the host's clock right | `ergon-tune` T11, T12 and `ClockWatch` |
  | the DPDK transport's stamps | its own clock (`RUSTERON_DPDK_CLOCK`) |

  The book's `ergon-clock` part is replaced by Deployment → Clocks. The
  md-dpdk AWS lab loses `LAB_CLOCK_ONLY`, `LAB_DISCIPLINE_SECONDS` and
  `vm.sh` `clockbench`, `build-clock` and `discipline`; `aws/lab.sh` refuses
  the two variables. The surviving raw benchmark rows are in
  `md-dpdk/benches/cloud/results/clockbench-aws-20261016.txt`.

### Changed

- `ergon-md-dpdk` check M4 reports global busy polling as information and
  recommends no setting: the committed A/B rows (md-dpdk/BENCHMARKS.md,
  "Measured, but no setting recommended") do not support one. It no longer
  warns, so `require` no longer refuses on it.
- EKS node bootstrap: no C-state cap on any node. The committed runs list
  the C1 cap under "no setting recommended" on VMs and on metal.
- Docs: every figure in the README, the book and the BENCHMARKS files cites a
  committed results file, a code line or a kernel/vendor document, or is
  removed; settings without a measured effect say "not measured here".
  Deployment → Kubernetes HFT tuning is one table of settings with their
  evidence. Raw output recovered for the cited AWS/EKS runs is committed
  under `md-dpdk/benches/cloud/results/`. The book's link check now covers
  absolute links into the repository and anchors cited from code.
- CI: lint, test and dpdk run on x86_64 and arm64; Aeron is built and run on
  Java 25; the test job is split into four suites per architecture; the
  vdev interop places its agents off the EAL main lcore's core and keeps
  send-only publishers up for retransmits; io tests that inspect
  process-wide state hold a lock, so they pass with `--test-threads=2`.
- MSRV raised from 1.88 to 1.99, the pinned toolchain. `serial_test` 4.0.1,
  a dev-dependency, needs 1.93.1.
- **Breaking (ergon-md-dpdk API), released as 0.1.31 all the same:** the
  transport no longer depends on `ergon-clock`. It reads the CPU counter at
  the rate DPDK's EAL gives it (`rte_get_tsc_hz()`), or `CLOCK_MONOTONIC` when
  DPDK gives no usable rate, the kernel's clock source is not the counter (or
  a hypervisor's view of it), the TSC is not invariant (x86_64), or DPDK's
  rate is not `CNTFRQ_EL0` (aarch64). At start the transport measures the
  counter against `CLOCK_MONOTONIC_RAW` for 25 ms and converts at that
  measurement where DPDK's rate is more than 100 ppm off it (DPDK 23.11 rounds
  its own x86 measurement to 10 MHz; see md-dpdk/README.md, Clocks).
  `RUSTERON_DPDK_CLOCK` keeps its values.
  - Removed: `clock::clock_config`, `clock::transport_clock` and
    `DpdkError::Clock`.
  - Changed: `DpdkTransport::clock()` returns `&clock::TransportClock`
    (`Counter { hz, rate, eal_hz, measured_hz }` or `Monotonic { reason }`),
    not `ergon_clock::advanced::Clock`.
  - Added: `clock::TransportClock` (`#[non_exhaustive]`),
    `clock::RateSource`, `clock::MonotonicReason`,
    `clock::kernel_refuses_counter` and `clock::CLOCKSOURCE_PATH`.
  - Kept: `clock::CLOCK_ENV`, `clock::ClockChoice` and
    `ClockChoice::from_env`.
  - Native ABI: the config's 48-byte clock parameters are now a one-byte
    choice, so `rusteron_dpdk_config_t` is 4328 bytes, not 4376; the new
    `rusteron_dpdk_clock_report` (48 bytes) says which clock started and
    why, and which rate it uses: DPDK's or the measured one, with both.

## [0.1.30] — 2026-10-20

### Added

- `ergon-clock` (first release): a hardware-counter clock for low-latency
  threads. `calibrate` once at start-up returns a `Clock` (`rdtsc` or
  `CNTVCT_EL0`, else `CLOCK_MONOTONIC` with the reason), a `Follower` for
  other threads, and a `Report` with time-sync findings. `now_ns` is UTC
  nanoseconds by default (`Domain::Monotonic` is since boot).
- `ergon-tune` (first release): pure-function checks of a host's low-latency
  settings (CPU isolation, interrupts off busy CPUs, hugepages, Aeron
  directory, cgroup CPU quota, NIC settings). Each finding names its
  effect and the exact fix. `ThrottleWatch` warns when a cgroup throttles.
- `ergon-io` (first release) gains a complete WebSocket client
  (close handshake, pings,
  subprotocols, `Reconnect` with backoff), TCP keepalive on every
  connection, a quick-ACK route check per connection
  (`Config::quickack_check`), request outcomes (`ClientHandler::on_error`,
  `outcome::Verdict`), connect gates for venue limits
  (`Reconnect::poll_gated`, `defer`) and a send budget
  (`ergon_io::limit::Budget`).
- `ergon-md-dpdk` (first release): an experimental DPDK ENA kernel-bypass UDP
  transport for the Aeron media driver (Linux, `libdpdk >= 23.11`,
  `dpdk` feature), selected by `RUSTERON_MEDIA_DRIVER_TRANSPORT=dpdk-ena`
  or `DpdkTransport::install`. It interoperates with kernel peers. A
  proof of concept, not for production.
- `k8s/nri-shared-cpus` (experimental, not released): a containerd NRI
  plugin and kubelet device plugin. A Guaranteed pod asks for integer CPUs
  and `ergon.io/shared-millicpus` and gets its exclusive CPUs plus shared
  housekeeping CPUs; every thread starts on the shared CPUs. Tested on
  kind, not yet on EKS. Nothing is on a registry.
- `ergon-md-dpdk`: `RUSTERON_DPDK_GAP_ATTRIB=1` splits each agent loop gap
  into work and descheduling, by the thread's own CPU time across it against
  its wall length (half or more on the CPU is work). Four counters —
  `poll-cycle-gaps-work`, `poll-cycle-gaps-desched` and their two maxima —
  answer whether a receiver's long gaps are the agent working or the host
  descheduling its vCPU. Off by default, so the hot path never reads the CPU
  clock for it.
- ergo-sbe: generated mappings between SBE and application types
  (`with_enum_mapping`, `with_composite_mapping`) with exhaustive
  `TryFromSbe` / `TryToSbe` matches and `sbe_rt::MappingError`.
- ergo-sbe: send helpers that claim, encode, check and commit one frame
  (`Claim`, `OfferClaim`, `send`, `send_frame`, `SendError`).
- ergo-sbe: `with_source_traits(true)` generates a `{Message}Source` trait
  per message and `encode_from(&src)` to encode from an application type.
- The book gains parts for `ergon-io`, `ergon-clock`, `ergon-tune` and
  `ergon-md-dpdk`, and a Deployment part (AWS, Azure, Kubernetes).

- Generate an opt-in cross-schema `AnySchemaMessage` dispatcher
  (`Generator::generate_schema_dispatch`) with typed errors and borrowed
  frames, including unknown schemas and templates.
- `SessionBuilder::aeron`, `client_name` and `error_handler`.
- `AeronCluster::offer_vectors` gathers a session header plus up to seven
  payload slices with no heap allocation.
- `IngressSessionDecorator` prepends a session header on a raw publication.
  `ClusterEvent` is the logged cluster event. Egress listeners gain
  `on_message_with_reserved_value` and `on_extension_message`.
- Report the cluster client's ingress limits, so a client can refuse to start
  when its largest message would not fit:
  `AeronCluster::ingress_max_message_length()`, `ingress_max_payload_length()`
  and `AeronCluster::SESSION_HEADER_LENGTH`.
- Report the egress image a cluster session is bound to:
  `AeronCluster::egress_image_correlation_id()` and `egress_image_session_id()`.
  As in Java, the session binds to the image that delivered its
  `SessionEvent(OK)` or latest `NewLeaderEvent`.
- Read a fixed text array (any `char` array, or a `uint8` array whose
  `characterEncoding` is ASCII or UTF-8) as text: `<field>_as_str()` returns
  `Result<&str, DecodeError>` with trailing NUL padding removed, at message
  and group-entry level. Group-entry encoders gain the `<field>_str(&str)`
  setter message encoders already had. Both use the new public
  `sbe_rt::encode_fixed_text` and `sbe_rt::decode_fixed_text`.

### Changed

- The workspace toolchain is Rust 1.99.0.
- Breaking (generated API): a domain DTO's `encode` reports a refused
  domain conversion as `EncodeError::Conversion` (field path and the
  conversion's error) instead of `DomainConversionFailed`, and refuses
  `Some` of an optional field's null value with `EncodeError::SomeNullValue`.
- Breaking (WebSocket): a complete data frame inside a fragmented message
  closes the connection (RFC 6455); it used to be delivered as a message.
- Breaking (libcurl): methods libcurl may send again (POST, PUT, PATCH)
  are refused unless `HttpClientConfig::curl_allow_non_idempotent_methods`
  is set.
- Aeron Archive and Cluster run on Java 25 (Temurin 25). The cluster
  harness checks the Java version and starts every Java process with
  tested JVM options. `ClusterLauncher` is compiled with
  `javac --release 17`.
- The release publishes `ergon-clock`, `ergon-tune` and `ergon-io` before
  `ergon-md-dpdk`; `just release-check` dry-runs them together.

- Breaking (generated API): drop `try_` from the per-field and closure
  methods.
  - A domain-typed field's getter and setter take the plain field name
    (`price()?`, `price(v)?`); the raw accessor stays `price_wire`. A field
    named after a reserved method takes the same `_field` rename its raw
    accessor would.
  - The decoder's boolean getter is `{field}_bool()` again, reversing
    0.1.24's `try_{field}_bool()`.
  - The `Ordered` lane's `try_fixed`, `try_<group>`, `try_<data>` and
    `try_<data>_as_str` twins are removed; their plain forms return
    `DecodeError`, so map a custom error into it inside the closure.
  - The raw stages' `try_<data>(f)` and `try_<data>_as_message(f)`
    callbacks are removed; use `into_<data>()` / `into_<data>_as_message()`,
    or `.ordered()`.
- Breaking (cluster API): `SessionBuilder` is no longer `Send` or `Sync`,
  because it can hold a shared Aeron client (`SessionBuilder::aeron`), and
  `EgressEvent::SessionEvent` gains `leader_heartbeat_timeout_ns`, so code
  that builds or destructures that variant must name the field.
- Depend on rusteron 0.2.11 from crates.io, which bundles Aeron 1.53.3.
- Pin the `aeron` submodule and the vendored cluster session schema to 1.53.3.
  Session schema id 111 is SBE version 17. `ServiceTerminationPosition` gains
  `ackRequired`.
- Check ergo-sbe's wire bytes against sbe-tool 1.40.2, up from a pre-1.40.0
  snapshot, and add its `issue1116`, `issue1118`,
  `floating-point-infinity-schema` and five more test schemas to the parity
  suite.
- Reject a float or double `nullValue`, `minValue`, `maxValue` or constant
  spelled `inf`, `infinity` or `nan` in any case other than `Infinity` and
  `NaN` (with an optional sign), as sbe-tool does. A malformed float constant
  now fails at parse time, not in the generated module.
- Reject `presence="constant"` on a set or composite field, and a constant
  enum field given by `constantValue` rather than `valueRef`, as sbe-tool
  does for the enum form.
- Stop blocking in the cluster client's polls. `AsyncClusterConnect::poll`
  and `poll_egress` add publications and subscriptions with Aeron's async add
  and pick them up on later polls; each add used to wait up to 5 s. Only the
  first `poll`, which creates the Aeron client, can still wait.
- Follow a new cluster leader as Java's client does: after a `NewLeaderEvent`
  the session stays `AwaitingNewLeader` until the new leader's ingress
  publication has connected, within `message_timeout`. The `Disconnected`
  reason names what the session was waiting for.
- Detect cluster leader loss from the session's own egress image:
  `AeronCluster::is_egress_connected()` is false once that image closes, even
  while a replacement image is open.
- Return `PayloadTooLarge` from `AeronCluster::try_claim` for a claim over the
  ingress publication's max payload length. `connect` and `finish` fail when
  the publication's limits cannot be read.
- Align the cluster client's connect, failover and egress handling with the
  Java client: open an ingress publication to every member when ingress
  endpoints are set, send protocol version 0.3.0, accept a `SessionEvent` or
  `Challenge` only for the request in flight, and wait for a new leader on
  `NOT_CONNECTED` and `CLOSED`. `MAX_POSITION_EXCEEDED` closes the session.
  An unset new-leader timeout is twice the leader heartbeat, or 20 s. UDP
  egress subscriptions set `rejoin=false`, and an egress channel of
  `endpoint=localhost:0` is resolved to the bound port before connecting.
  `poll_egress` no longer sends keep-alives.
  `EgressEvent::SessionEvent` carries `leader_heartbeat_timeout_ns`.

### Fixed

- Supply chain: `rustls-webpki` 0.103.15 → 0.103.16, closing
  [RUSTSEC-2026-0382](https://rustsec.org/advisories/RUSTSEC-2026-0382).
  0.103.15 accepted certificate paths containing distrusted emSign
  intermediates whose private keys are public. `ergon-io` depends on `rustls`
  directly, so this shipped in a published crate. `cargo deny` and
  `cargo audit` block the release on a live advisory; they are what caught it.
- ergo-sbe: a domain DTO's `encode` writes `NullVal` for a `None`
  domain-typed enum field instead of leaving whatever the buffer held.
- Benchmark validity: SBE parity arms pay the same validation class and the
  same observation barriers. Gated body-wrap decode pairs and body-only
  encode pairs time `wrap_unchecked` against sbe-tool's zero-check wrap, with
  the extent proved once in an untimed preflight; `ergo-sbe_try_from` is a
  separate informational arm the gate does not read. Every value one arm
  makes opaque, its partner makes opaque at the same number of barriers and
  the same program points. `fairness_policy_test` enforces both.
- Benchmark accounting: `Work::from_meta` rejects a malformed count instead
  of treating it as missing, and reconciles `work.completed` against the
  records on disk before any warm-up is dropped. `analyse` refuses to
  publish latency rows or exit 0 on a contradictory count. The benchmark
  binaries sample both endpoint clocks before any of the run's own file I/O,
  so the schedule's span no longer absorbs disk time.
- Benchmark gate: a calculation that produced no verdict can no longer
  certify a ratio. A ratio above its ceiling whose ratio CI overlaps a
  measured A/A null band is inconclusive, not a breach; a pair with no
  measured control still fails closed. Ceilings are unchanged. The gate
  reads Criterion's `typical()` estimator — the mean when `slope` is null,
  never the median.
- Benchmark gate: a pair is graded on the **envelope of every independent
  invocation** the runner took, and against a **repeated** A/A control, not
  on one invocation's confidence interval. Identical code walked 3.5 % across
  three invocations (`cluster_decode_session_message_header` measured 1.0236,
  0.9916, 0.9887) while each run's own ratio CI was ±0.5 %, so a single run
  reported a rotating set of "resolved breach" verdicts on unchanged code.
  `just bench-cluster` and `run-sbe-bench.sh` now take BENCH_REPEATS
  invocations (default 5) and snapshot each under `repeats/<k>/`; the gate
  resolves a pair only where the envelope clears the ceiling and the control's
  envelope in every invocation, and otherwise establishes no winner. A
  manifest declaring more invocations than are on disk is refused. Ceilings
  are unchanged and no exception was added.
- Benchmark gate: a repeat snapshot keeps only estimates its own invocation
  produced. The Criterion home is shared with other bench binaries and
  Criterion leaves `new/` in place for groups a run does not touch, so a
  snapshot of the whole home folded foreign keys into the envelope and widened
  it with evidence the run never produced. The runner marks the invocation
  with `.bench-since`; a snapshot with no marker is refused. The graded line
  now shows the envelope the verdict is based on — it used to print the last
  invocation's ratio beside a label decided by the maximum, which read as
  `0.9941 (max 1.00) above ceiling`.
- Benchmark gate: an over-ceiling pair is non-blocking only when a **measured**
  A/A null band reaches it. Without a band, or outside it, it is a resolved
  breach or **unproven**, and unproven blocks. A wide confidence interval is
  not evidence: a 2× slowdown (200 ns against 100 ns, ratio CI 50–400) with
  no control used to exit 0. "The measurement could not resolve it" is not
  "the ceiling was met".
- Benchmark gate: every estimate is parsed before any is used. A malformed,
  non-finite or non-positive repetition refuses the whole key and prints
  nothing; it used to be dropped, so a key with one good repetition and one
  truncated `estimates.json` graded on the good one and passed. `NaN`, `0`
  and negative counts are refused, not read as numbers.
- Benchmark gate: every declared repetition must carry every consumed pair,
  matched by repetition ID. Removing both arms of one comparison from
  repetition two used to still report "2 independent invocations" and grade
  that pair on one. A repetition missing a pair is incomplete evidence.
- Benchmark gate: a consumed A/A control is provenance-checked against the
  run manifest like the pairs it grades. A control stamped by another run
  used to excuse a tightly measured 2× regression as harness noise. A missing
  control is still reported as "A/A null not measured", not as a provenance
  failure.
- Benchmark manifests record the host CPU. `aarch64-apple-darwin` does not
  distinguish an M1 from an M4, and 0.1.30's local figures are M1 where
  0.1.29's were M4, so absolute figures are not comparable across the two
  releases. Ceilings are host-independent ratios and are unchanged.
- Fair comparison reports carry the sender's schedule beside the work counts,
  and name the rows whose sender was not the planned traffic. `Work::late`
  counts paced **order slots**; the sender's `late_100us` counts **batches**
  across all traffic, and the two were published as one number. Six of 144
  points in the Graviton fair run `20261019T175248Z` slipped over 1 % of
  batches; every one is interior on p50 and p99 within its family, and the
  two published endpoints a slipped row sets are now printed with and without
  it. A `latest µs` over 100× the median of its column is named beside the
  table as a stall, not offered as a ranking.
- The work-counts extractor fails closed on the evidence it reads. A present
  but partial sender schedule, a minus count or an unreadable value names the
  key and value and writes nothing; it used to drop the point or copy the
  figure through (`late_100us=-80` reported `-8.00% … ok`, exit 0). Its
  parsing is `u64::from_str`, and its completeness rule is the Rust loader's:
  a sender schedule is all or none. A run that wrote none of it is still an
  honest hole, not a failure.
- Release artifacts: a cluster archive carries only the run's own evidence.
  The cluster Criterion home is shared with other bench binaries and Criterion
  leaves `new/` in place for groups a run does not touch, so one run packaged
  53 "estimates" of which 35 belonged to `ergon-clock-bench` and the historic
  bench. The archive and its estimate count now come from `repeats/<k>/` — the
  tree the runner snapshotted and the gate grades.
- Clock bench: `the_gap_pass_takes_no_page_faults_while_it_times` asserted a
  zero fault count that off Linux is the whole process's
  (`getrusage(RUSAGE_SELF)`; Darwin has no `RUSAGE_THREAD`), so another
  thread's faults failed the test, and its gap assertion could not see a
  fault at all because the read under test is a counter. The pass now proves
  its buffer is resident with `mincore` — scoped to the buffer, and counted
  as pages are touched — and only asserts the fault count where the kernel
  scopes it to the thread.
- Cluster codec benches: `encode/session_keep_alive` and
  `encode/claim_shaped_header_plus_app` measured unequal mechanisms. Both
  arms now carry the controls `encode/session_message_header` already used —
  `.p2align 6` on both loop headers, `wrap_unchecked` on the ergon arm, field
  values opaque and slice length visible on both arms. The pre-fix closures
  are kept as a diagnostic probe pair in `cluster_perf_probe`, which shows
  what the checked `wrap` costs when the slice length is opaque. All three
  maintained encode pairs are now instruction-identical on x86-64 (80/80,
  75/75, 72/72) with byte-identical stores.

- Leave constant fields out of a group entry's `blockLength`. Frames ergo-sbe
  encoded for schemas with a constant ending an entry change width to match
  sbe-tool.
- Return a constant enum in a group entry from its `valueRef`, as message
  level does; its wire-reading entry accessors and setters are removed.
- Count a group entry's DTO `encoded_length()` from the stride the encoder
  writes, including a group's declared `blockLength` padding.
- Take an enum's null from its `encodingType` typedef's `nullValue` when the
  enum declares none, as sbe-tool does. `NullVal`, the `From` conversions and
  an optional enum's `None` now write that value (254 for `nullValue="254"`),
  where they wrote the primitive default.
- Generate float and double constants written `Infinity`, `-Infinity`,
  `+Infinity` or `NaN`; they did not compile. A negative constant such as
  `-1.5` is no longer mistaken for an `Enum.value` reference.
- Decode an optional float or double whose `nullValue` is not NaN as `None`
  when it holds that value, at every level.
- Let fixed-text decoder helpers yield to schema field names at every depth.
- Emit generated doc comments one `#[doc]` line at a time. A multi-paragraph
  doc was rendered as a `/** … */` block whose indented prose rustdoc ran as a
  failing doctest when the generated module was checked into `src/`.
- Stop the cluster crate enabling ergo-sbe's default `fancy` feature, which
  pulled miette's graphical renderer into the cluster build.
- Walk `verify` at the dimension header's block length and the frame's acting
  version, so an extended group block is accepted and a short nested block is
  rejected.
- Reject a nested group block shorter than its fields when reached through a
  warm parent entry; `bulk_decode_into` could panic or read past the buffer.
- Rescale a `rust_decimal::Decimal` to a decimal composite's constant
  exponent instead of passing its mantissa through. A value that cannot be
  represented exactly is an error, never rounded.
- Honour a `UTCTimestamp` field's `timeUnit` in the built-in
  `chrono::DateTime<Utc>` conversion, at every level and in domain DTOs. A
  value more precise than the field holds is an error.
- Reject an invalid `valueRef` at parse time, as sbe-tool does.
- Reject an included file whose root is `<message>`; it was dropped silently.
- Keep the ingress channel's settings when the cluster client follows a
  REDIRECT or a new leader; only a unicast endpoint is replaced.
- Apply a cluster `SessionEvent(CLOSED)` that arrives with or after a
  `NewLeaderEvent`; it was dropped.
- Refuse cluster channel URIs over Aeron's 4096-byte limit; Aeron's printer
  wrote past a fixed 512- or 256-byte buffer.

### Removed

- The recommended JVM options drop `-XX:+TrustFinalNonStaticFields`,
  `-XX:GuaranteedSafepointInterval=300000` and `-XX:+DebugNonSafepoints`,
  and the two `-XX:+Unlock{Experimental,Diagnostic}VMOptions` that existed
  only to admit them. All three came from Aeron's scripts and benchmarks;
  their effect on the recommended JDK was never measured, and two of them
  are experimental or diagnostic, so the JVM is free to change them.
  Every option left in `cluster-test-harness/jvm/aeron.jvm` is either
  required to start or has a measured reason.
- Moved the runtime, its server, the wire and bridge crates, the log printer
  and the ClickHouse lab to a private repository.

## [0.1.29] — 2026-09-19

### Fixed
- **`acting_version()` / `acting_block_length()` still collided wherever a
  type owned tails, so those schemas did not compile.** The 0.1.28 fix covered
  the group-entry decoder only. The ordered lane emitted both names
  unconditionally on every wrapper, and the message decoder and memoized
  wrapper did the same, so a group or var-data named `actingVersion` /
  `actingBlockLength` (at message or entry level), or an entry field of that
  name beside entry tails, failed with E0592 or E0015. Every decoder, stage and
  ordered wrapper now emits them through one helper that yields to a schema
  name on that type. The header values stay on `get_metadata()`.
- **`with_external_sbe_rt` against a runtime owner without groups or var-data
  did not compile.** 0.1.27 and 0.1.28 sized `EntryInfo`, `Ordered` and
  `OrderedFixed` to the emitting schema, but a runtime owner cannot see the
  schemas of the modules that share it. A generated `sbe_rt` is a whole runtime
  again, so one shared runtime means one set of types:
  `consumer::sbe_rt::EntryInfo` *is* the owner's, as it was before 0.1.27.
- **`<field>_as_str` yields to a sibling tail of that name**, not only to a
  fixed field, through the same helper as `<group>_count` / `<field>_len`.
- Group-entry `acting_version()` / `acting_block_length()` yield when a field
  already owns that name, matching the count/len rule. A schema with
  `actingVersion` / `actingBlockLength` on an entry compiles; the field
  accessor keeps its name.
- Ordered `fixed` / `try_fixed` callbacks yield independently. A first tail
  named `tryFixed` keeps `fixed()`. A first tail named `fixed` also owns
  `try_fixed()` (the try-visit), so both callbacks yield and callers read
  fixed fields before `ordered()`.
- Memoized **and group-entry** var-data getters re-check the schema maximum
  even when `encoded_length()` / the iterator has already published a
  bounds-valid end. Over-max payloads are `InvalidVarDataLength` on the base
  lane, a cold getter, and a warmed one.
- Domain `None` for a `with_all_enums_as_option()` (or `with_null_as_option`)
  enum writes `NullVal` into a reused buffer, including nested-entry DTOs.
  Flat-group `bulk_add_domain` and `to_wire_entry` honour `Option<Enum>` the
  same way. A versioned required enum `None` writes `NullVal` rather than
  skipping the write.
- `*_as_str` / `*_as_str_unchecked` yield per name: a sibling `noteAsStr`
  does not drop `note_as_str_unchecked`.
- The gated `ergo-sbe_ordered` Car decode arm walks recursively with ordered
  callbacks at every tail and observes the same fields as the staged
  consuming arm. `fairness_policy_test` rejects staged `.into_*` anywhere in
  that arm.

## [0.1.28] — 2026-09-15

### Added
- **The ordered lane recurses.** `ordered()` is now generated on any decoder
  that owns tails, not only the message decoder: a group entry carrying its own
  groups or var-data has one too, so a walk keeps a single spelling all the way
  down instead of switching to the staged API at the entry boundary. A nested
  `ordered()` walk is the completion the parent visit owes. Entries with no
  tails of their own have nothing to order and get no lane.
- The staged `into_*` iterator is unchanged and still generated beside it. For a
  fixed-stride nested group it remains the better tool — an `ExactSizeIterator`
  you can `break` out of, still reaching the next tail by arithmetic. The lane
  adds a spelling rather than replacing one.

### Changed
- Nested `ordered()` walks return the parent visit's completion directly
  (`sbe_rt::IntoEntryComplete`); `.done()` is only to unwrap the staged
  complete. The message chain ends on `encoded_length_with_header()` /
  `as_bytes_with_header()`, like encode.
- The ordered lane is `sbe_rt::Ordered<S>` wrapping each staged stage, instead
  of a parallel `*DecoderOrdered*` type per tail. Message-level `fixed` takes a
  `{Name}DecoderFixedView` (fixed fields only) and consumes into
  `Ordered<OrderedFixed<Decoder>>`. Entry-level wrappers no longer emit `fixed`:
  the parent callback already holds the entry. Unprefixed group/var-data methods
  return `DecodeError` so closures infer; `try_*` keeps a custom `E`.
- `EntryInfo` for dynamic groups is filled from the attached group decoder the
  staged walk already opened. The discarded `__sbe_*_dim` wrap is gone.

### Fixed
- A group entry with a field literally named `ordered` no longer collides with
  the new entry-level lane. Group entries are not renamed against the static
  reserved list, unlike message and memoized decoders, so such a field already
  owns `ordered()`; the lane yields and is simply not generated for that entry,
  matching how `<group>_count` / `<field>_len` yield to a colliding sibling.
  A converted field named `ordered` (`ordered_wire` / `ordered_as`) no longer
  suppresses the lane. Covered by `ordered_lane_test`.
- A first tail named `fixed` no longer emits duplicate `fixed()` methods: the
  message-level callback yields, and the group/var-data visit keeps the name.
  Last tail named `done` is a different type from `done()` on the complete
  stage. Covered by `ordered_lane_test`.
- The entry-level lane's `group(&mut self, F)` / `try_group` no longer declared
  a default on their `R` type parameter. Function generic parameter defaults
  are not valid Rust (`invalid_type_param_default`, deny-by-default); every
  group whose entries carry their own tails failed to compile. `R` is always
  inferred from the callback's return expression, so the default was dropped
  rather than replaced.

## [0.1.27] — 2026-09-12

### Added
- **Ordered decode lane — `decoder.ordered()`.** One callback per tail, in
  wire order, with the same spelling at every *message-level* tail (nested
  groups inside an entry keep the staged spelling): `fixed(|&Decoder|)`, then
  `group(|entry, EntryInfo|)` for each group, then `field(|&[u8]|)` (or
  `field_as_str(|&str|)` where the schema declares a text encoding). `done()`
  returns the staged complete stage, so extent and full-frame helpers stay
  reachable. It is a façade over the staged stages and owns no cursor, so the
  entry traversal and compile-time tail ordering come from there rather than
  being re-implemented; entries carrying their own tails still return their
  completion from the callback, which is what keeps the walk one-pass. It is
  free over the staged lane for fixed-stride groups, and costs one extra
  dimension-header read per *dynamic* group per message — never per entry, and
  never a scan — because a visit closure exposes no handle to ask for the
  count. The staged lane stays the floor for all-dynamic messages.
- `sbe_rt::EntryInfo { index, count, block_length }`, handed to every ordered
  entry callback, with `is_first()`, `is_last()` and `remaining()`. Both
  `count` and `block_length` come from the group's dimension header, so neither
  costs a scan. A group's total *byte* length is deliberately absent: for
  entries with tails it is only settled by traversing them.
- **Fixed-stride groups decode as real iterators.** A group whose entries carry
  no tails of their own has a known stride, so `into_<group>()` hands back an
  iterator: `for entry in &mut iter`. It implements `Iterator` only for
  `&mut Iter`, never for `Iter`, so `for entry in iter` does not compile and a
  loop cannot silently drop the rest of the message. Also `ExactSizeIterator`
  and `FusedIterator`. The iterator does not have to be drained — `finish()`,
  or the next tail's `into_*` / `skip_*` called straight on it, skips the rest
  by arithmetic.
- Non-advancing `<group>_count()` and `<field>_len()` accessors, generated at
  every decode location: the base decoder, `.memoized()`, group entries, and
  each staged stage that precedes the tail. Both return `Result<usize>` and
  yield `0` when the tail is absent at the acting version, so sizing a `Vec` or
  logging a count no longer costs a stage.
- `iter.remaining_entries()` and `iter.entry_block_length()` on the staged
  fixed-stride iterator. The ordered lane fills `EntryInfo` from these for
  fixed-stride groups, so that shape costs nothing over the staged walk.

### Changed
- **`ordered` is a reserved decoder method name again.** 0.1.26 removed the
  mutable ordered lane and, with it, the rename that a schema field named
  `ordered` used to take; the new callback lane puts `ordered()` back on the
  base decoder, so such a field is once more generated as `ordered_field()` —
  the same rule that has always applied to `memoized`. The *mutable* lane
  (`{Name}OrderedDecoder`, runtime `OutOfOrder` checks) stays deleted; the new
  lane enforces order through types, not at runtime.
- The `ergo-sbe_ordered` benchmark arm now exercises the ordered lane. It and
  `ergo-sbe_consuming` were both measuring the staged API, so the two labels
  reported the same thing; `fairness_policy_test` now asserts each arm uses its
  own lane.
- Groups whose entries carry their own groups or var-data keep the visit
  closure, and now that is a deliberate split rather than an accident of
  history. Such entries have no stride, and `Iterator::next` cannot learn where
  an entry ended from the entry it already handed away, so an iterator would
  have to measure every entry before yielding it and the caller's own walk
  would traverse it again. The closure returns the entry's completion, and that
  *is* the next cursor. One pass, both shapes.
- `absent_tail_length` is now the single predicate for "this tail is not on the
  wire at this version", shared by all four locations that emit a count or
  length accessor; the memoized lane no longer carries its own copy.
- `sbe_rt::EntryInfo` is emitted only for schemas that declare at least one
  group (and unconditionally in shared-module mode, where one module's runtime
  serves its siblings). It is reachable only through the ordered lane's group
  callbacks, so a group-less schema was carrying it as dead code.
- `extended_optional_enum_nullify` now decodes the optional composite's counter
  alongside the two enums, on both arms. The enum-only form was not gateable:
  both codecs compile those two-byte loads to the same shape, so the true ratio
  is ~1.00 and the measured one was decided by code placement rather than codec
  work — a 2.6% change in unrelated generated code in the benchmarks crate moved
  ergon's arm 37% while the decoder's generated source stayed byte-identical.
  Reading the composite adds equal logical work to both arms and leaves the
  ratio near 0.78, with enough margin that placement drift cannot flip it.

### Fixed
- **`<group>_count` / `<field>_len` could collide with a sibling accessor and
  produce a module that did not compile.** A schema declaring a group `orders`
  beside a field `ordersCount` emitted `orders_count()` twice (E0592), at
  message, memoized and group-entry level alike. Sibling *tails* collided the
  same way: a group `fills` beside a group `fillsCount`, or a var-data `blob`
  beside a var-data `blobLen`. These names are derived from the schema, so they
  cannot live in the static reserved list that drives field renaming. The
  existing accessor now wins and keeps its name — renaming it would change an
  accessor that worked before these convenience methods existed — and the
  convenience accessor is not generated for that tail. Consuming stages carry
  no fields or sibling tails, so they still get it. Covered by
  `reserved_name_clash_test::tail_accessors_yield_to_colliding_field_names`,
  which compiles a schema containing all three collision shapes.
- **`finish()` / `skip_*` on a fixed-stride group stepped over each unread
  entry.** Every entry occupies the acting block length, so the group's end is
  `count * block_length`; the loop was doing a bounds check per entry for a
  result that one multiply gives. Groups whose entries carry tails still walk,
  because without a stride there is nothing to compute.
- The fixed-stride group decoder gained the `size_hint` that its
  `ExactSizeIterator` implementation already implied; it was returning the
  default `(0, None)`.
- The codegen matrix's iterator-surface probe bound its message walk to `_`,
  so a decoding failure in the walk could not fail the test. It now propagates
  with `?`; verified by injecting a truncated buffer, which the bound-to-`_`
  form passed and the fixed form fails.
- `EntryInfo::remaining()` underflowed for a zero-`count` value. `EntryInfo` is
  a public struct, so an independently constructed one could panic in debug
  builds; it now saturates. Its three queries also carry `#[must_use]`, since
  discarding a pure position query is always a bug.

## [0.1.26] — 2026-09-07

### Added
- Var-data declared with `characterEncoding="UTF-8"` or `"ASCII"` now gets a
  `*_as_str(&str)` encode-side setter, at both message and group-entry level,
  alongside the existing `*_as_str` decode-side accessors — writes through
  the checked byte setter after validating ASCII where the schema requires it.

### Changed
- Sequential decode is the staged `into_*(|entry|)` / `skip_*` chain — the
  encoder dual, compile-time order, no `&mut`.

### Fixed
- `characterEncoding` spelling variants legal under the SBE spec —
  `US-ASCII` and `UTF8` (no hyphen), used by real schemas including this
  repo's own Aeron cluster codecs — were silently dropped by message-level
  decode, the memoized decode lane, and group-entry decode, though the
  staged-consuming lane already recognised them. Message-level decode,
  group-entry decode, the memoized lane, and the new encoder now classify
  `characterEncoding` through one shared function so they can't drift again.

### Removed
- The mutable ordered decoder lane: `{Name}Decoder::ordered()`,
  `{Name}OrderedDecoder`, `AnyMessage::into_<name>_ordered()`, and
  `DecodeError::OutOfOrder`. Sequential decode is the staged
  `into_*(|entry|)` / `skip_*` chain; random-access and `.memoized()` remain.

## [0.1.25] — 2026-09-06

### Added
- **Four decoder lanes, generated for every message with groups or var-data:**
  random access (`try_decode`, any order, `Sync`), staged (`into_*(|entry|)` /
  `skip_*`, compile-time order), mutable ordered (`.ordered()`, one
  cursor, runtime order checks), and memoized (`.ordered()`'s sibling —
  `decoder.memoized()` consumes the base decoder and returns a progressive
  `Cell`-backed tail-boundary cache, so repeated or out-of-order tail reads
  walk each boundary at most once). A message with no groups or var-data gets
  only the base decoder — there is no tail to memoize or order, so
  `.memoized()` / `.ordered()` are not generated and `AnyMessage` exposes only
  `into_<name>()` for it.
- `AnyMessage::into_<name>_memoized()` / `into_<name>_ordered()` land directly
  in a lane without an intermediate base-decoder conversion.
- Var-data declared with `characterEncoding="UTF-8"` or `"ASCII"` now gets
  `*_as_str` / `*_as_str_unchecked` text accessors on **group entries**, not
  only at message level — the same helper that already existed for message
  var-data, shared rather than duplicated.
- `GenerationConfig::with_encode_version(version)`: the encoder writes
  `version` in the header and omits members introduced after it, while the
  decoder still reads every version in the schema. Useful for producing older
  wire versions for peers that have not upgraded. It is rejected at generate
  time when it would drop members alongside domain objects or conversions.
  Covered by a versioned-L3 fixture (v0–v3) that is byte-compared against
  sbe-tool at every acting version.
- Cluster Callgrind instruction probes (`just bench-cluster-instructions`) —
  mechanism-level evidence for cluster session codec hot paths, alongside the
  existing SBE probe lane.

### Fixed
- **`<data characterEncoding="...">` was silently dropped by the schema
  parser.** SBE allows `characterEncoding` directly on `<data>`, not only on
  the referenced composite's `varData` member; real schemas use the element
  form. The parser only read the composite-member form, so a declared-text
  field silently downgraded to raw `&[u8]` with no `*_as_str` accessor at all
  — a wire-compatible but API-degrading bug with no error to signal it.
- **A truncated or undersized fixed-stride group could be read out of bounds
  through the mutable ordered decoder.** The fixed-stride branch of
  `.ordered()`'s group entry validated `count * blockLength` but never
  `blockLength` against the fixed fields' minimum readable extent — a
  `blockLength` smaller than a required field's offset let a safe getter read
  past its entry. Both the truncation and the undersized-stride case are now
  rejected before any entry reaches a caller callback, matching the
  random-access lane's existing checks.

### Removed
- `GenerationConfig::with_memoized_tail_offsets` and
  `with_compact_tail_offsets` — memoization is now a runtime lane
  (`decoder.memoized()`) rather than a generation-time, module-wide knob, so
  the choice moves to each call site instead of being fixed for the whole
  module. Compact `u32` tail-offset storage was measured and dropped: it saved
  a little decoder memory but cost more instructions on the cache's hot
  primitives and was materially slower under LTO.

## [0.1.24] — 2026-08-31

### Fixed
- **Domain objects + domain types generate compilable code again.** Fifteen
  codegen defects made `with_domain_type` / `with_manual_domain_type` unusable
  alongside `with_domain_objects` for most field shapes. Every one produced a
  generated module that failed to compile, so no wire format changed and no
  working code can have depended on the old behaviour.

  Reported against 0.1.22 and 0.1.23, then found while root-causing it:
  - A `sinceVersion > 0` **composite** wrapped `try_*` in `Some(..)`. `try_*`
    on a versioned field already returns `Result<Option<T>, E>`, so the DTO
    built `Option<Option<T>>` for an `Option<T>` field.
  - The same double-wrap on a `sinceVersion > 0` **primitive scalar**.
  - A **fixed array** matched by a selector generated converters against the
    element type (`u8`) instead of the array type (`[u8; 4]`).
  - An **array or optional scalar** with a conversion read the raw flyweight
    getter under its pre-rename name, though a conversion renames it `*_wire`.
  - An **array or optional scalar** DTO kept the wire type but wrote through
    the domain `try_*` setter, which takes the domain type.

  Found by the combination matrix below:
  - A **versioned array** was treated as `Option`-returning, though an array
    accessor returns a zero-filled array, never `None`.
  - A **constant** field matched by a selector generated conversion accessors
    against a `*_wire` setter that constants do not have.
  - `Display` for a **group-entry composite** with a domain type re-derived the
    conversion and fed `Option<T>` to `TryFromSbe<T>`; it now goes through the
    generated `try_*` accessor.
  - `Display` for a **converted primitive** read the raw getter under its
    pre-rename name, at message and group-entry level.
  - Conversion-only `*_as` accessors did not `Option`-unwrap a versioned or
    optional raw getter, at message and group-entry level.
  - `with_all_enums_as_option` makes enum accessors return `Option<T>`, which
    neither the `*_bool` accessors nor the domain DTO accounted for.
  - An **optional bool enum** produced a group-entry accessor that matched
    `Option` against a plain `BooleanType`.
  - An **optional enum with a domain type** was treated as `Option`-returning;
    an optional enum carries `NullVal` as a variant, so its accessor is plain.
  - `null_as_option` **combined with an enum domain type** left `try_*`
    disagreeing with the accessor it delegates to, in both directions.
  - Group-entry and message-level boolean accessors disagreed on when they
    return `Option`; both now do so only when version-gated.

  Most shared one root cause: the same decision — "is the domain type applied
  to this field?", "is this accessor `Option`-returning?" — was made
  independently in several places and drifted apart. Those now live in single
  predicates (`conversion_helpers::dto_domain_type`,
  `converter_impls::is_optional_domain_field`).
- `ConversionSelector::FieldPath` selects fields again. It is documented as the
  primary selector form and validation accepted it, but codegen matched only
  `NamedType` and `SemanticType`, so every `FieldPath` selector was a silent
  no-op. Group and nested-group fields extend the path with their group names
  (`"Message.group.field"`).
- Documented selector precedence is enforced: `FieldPath` beats `SemanticType`
  beats `NamedType`. Resolution was "first selector in registration order", so
  precedence silently depended on call order.
- Domain types configured for **enum** and **set** fields reach the DTO.
  `try_*` accessors were generated for them all along, but the domain-DTO
  generator ignored them and emitted the raw generated type. Boolean enums are
  unchanged — they still materialise as `bool`.
- Scratch-crate test helpers no longer force `--offline` / `CARGO_NET_OFFLINE`,
  and they strip that flag when inherited from `cargo llvm-cov`. Nested
  compile-and-run crates can download extra transitives (`bit-set`, `ahash`)
  that are not in the workspace lock — the cause of red `test`/`coverage`
  jobs on 0.1.19–0.1.22 and Dependabot PRs.
- The coverage job timeout is 120 minutes (was 30). Instrumented
  compile-and-run tests cannot finish in half an hour; the 0.1.23 coverage
  job was cancelled mid-run.
- Nightly `cargo fuzz` pins `x86_64-unknown-linux-gnu`. cargo-fuzz's Linux
  default is musl+ASAN, which fails on a minimal nightly without that target.
- Group-dimension IR extraction matches the required `blockLength` and
  `numInGroup` members by exact name. Padding fields such as `account`,
  `countPadding`, or `blockLengthPadding` no longer overwrite the layout.
- Generated group-entry `Display` formats a domain-mapped set through
  `try_*` and `{:?}`. The raw accessor is `*_wire`; calling `self.<field>()`
  failed to compile.
- `poll_connect_until_done` idles with work-count 0 while the handshake is
  still in progress. `AsyncClusterConnect::poll`'s `true` means more polling
  is needed, not that Aeron performed work; a positive count reset backoff
  and pinned a core.
- `parse_ingress_endpoints` rejects empty comma-separated entries (leading,
  trailing, or repeated commas) instead of skipping them.
- `cluster_decode_session_event` carries a documented no-LTO-only allowance of
  1.05 (LTO stays at 1.01, where ergon measures 0.99). Its no-LTO arm is a
  memory-bound decode that walks either side of parity — 1.0403 and 1.0444 on
  consecutive quiet-machine runs, matching the 1.0406 already recorded in the
  pair's history. This is an allowance for a placement-sensitive tie, not a
  proof that nothing regressed; above 1.05 the gate fails as before.
- `write_module_set` restore after a mid-commit failure replaces an
  already-promoted destination (Windows-safe) and surfaces restore errors
  rather than discarding them. Successful publication fails if a `.bak`
  backup cannot be removed.

### Added
- **Codegen combination matrix** (`sbe/tests/codegen_matrix_test.rs`). Every
  field shape x `sinceVersion` x mapped/unmapped cell, at all three codegen
  locations (message, group entry, nested group entry), crossed with every
  `GenerationConfig` combination that selects a distinct codegen path — and
  each case **compiles** the generated module rather than inspecting its
  source. Source-string assertions were what let the defects above ship: they
  stay green while the module fails to compile.

  The fixture is generated by `scripts/regenerate-codegen-matrix-fixture.py`,
  whose `SHAPES` list is the single source of truth; the test recomputes the
  same cross product and fails when the fixture falls behind, and `--check`
  (wired into `just preflight`, `just test`, and CI) fails on drift.

### Changed
- **Breaking (generated API):** the decoder's boolean getter is `try_{field}_bool`
  at every shape and location. Message-level code named the `Option`-returning
  one `{field}_bool` while group entries named both shapes `try_*`, so the same
  schema shape had two different accessor names depending on where it sat —
  which is what let the domain DTO call an accessor that did not exist.
  `{field}_bool` is now unambiguously the *encoder setter*; `try_` marks the
  fallible decode. The only remaining shape difference is the return type:
  `Result<bool, _>`, or `Result<Option<bool>, _>` when `sinceVersion > 0` makes
  the field absent on older wire versions.
- **Breaking (generated API):** conversion-only `{field}_as::<T>()` returns
  `Result<Option<T>, _>` for a `sinceVersion > 0` or optional field, matching
  the domain-typed `try_*` accessors. It previously generated a body that fed
  `Option<W>` to `TryFromSbe<W>`, so no compiling code can depend on the old
  signature.
- Hook `FieldInfo::rust_type` for groups and var-data is a real type
  (`LevelsDecoder`, `&[u8]`) instead of the sentinels `"group"` / `"data"`.
  `FieldInfo::kind()` returns `FieldKind` (`Fixed`, `Group`, or `Data`).
- Consumers are advised to build with `lto = true` / `codegen-units = 1`.
  Generated codecs are many small `#[inline]` methods whose inlining only fully
  collapses across codegen units; `sbe/BENCHMARKS.md` and the book's benchmark
  chapter now document the recommendation. Benchmark methodology no longer
  describes the LTO profile as informational — both profiles have been blocking
  gates with the same literal `1.00` ceiling.
- Hook `FieldInfo` carries a `kind: FieldKind` field recorded when the field
  is built. `kind()` returns it instead of inferring from `rust_type` suffix
  and substring checks, so renaming a generated type cannot silently change a
  field's meaning.
- `just check-local`, `just check-products`, and `just test` no longer run the
  `ergo-aeron-cluster` suite twice: the crate is excluded from the workspace
  run that already covers it explicitly, and its doctests run once via
  `cargo test -p ergo-aeron-cluster`. Same for CI's workspace doctest job.
- CI, `just test`, and the local product gates run
  `cargo test -p ergo-aeron-cluster` (integration tests, compile-fail API
  suite) rather than `--lib` only.
- Generated public-API freeze records trait associated-type bounds,
  defaults, generics, where-clauses, and semver attributes.

## [0.1.23] — 2026-08-28

### Fixed
- Generated `Display`/`Debug` bodies no longer assume a `with_domain_type`
  target implements `Display`. Domain values format with `{:?}`, so a domain
  type only has to be `Debug`.
- `with_display_debug(false)` now also omits the group entry decoder's
  `Display` impl, which was emitted unconditionally.

### Changed
- **Breaking:** optional var-data accessors (`into_*_as_compact_str`,
  `into_*_as_smol_str`, `into_*_as_bytes`) are emitted only when the
  *generator* was built with the matching `compact_str` / `smol_str` / `bytes`
  feature — reverting the 0.1.x switch to gating them on the consumer's own
  features. That gate was a `#[cfg(feature = "…")]` in the emitted source,
  which resolved against the consumer crate's feature table; consumer crates do
  not declare features by those names, so the accessors were unreachable dead
  code in every generated module. Enable the feature on `ergo-sbe` in both
  `[build-dependencies]` and `[dependencies]`.
- The generated public-API snapshots no longer record those accessors, since
  their presence now depends on the generator's feature set.

## [0.1.22] — 2026-08-27

### Added
- Generated observers named by T-1 (`after_this_message`, message/group
  `min_readable_fixed_extent`, `MessageHeader` peeks, `schema_id_from_header`,
  domain `to_wire_entry`) carry specific `#[must_use]` diagnostics.
- `StaticCredentials::new` accepts `impl Into<Vec<u8>>` (moved `Vec`, slice,
  array, embedded NUL, invalid UTF-8) with byte-identical connect and challenge
  credentials. `from_utf8` remains the text convenience.
- Poll-driven connect recipe in rustdoc, `cluster/README.md`, and the session
  builder book chapter: `SessionBuilder` → `connect_async` → `default_idle` →
  `poll_connect_until_done` → `finish`. `AsyncClusterConnect` is a poll-driven
  Aeron state machine, not a Rust `Future`.
- Group-dimension composites accept unsigned `u8`/`u16`/`u32`/`u64` members
  (both endians, reordered offsets, padded composites over 32 bytes). Parser
  rejects signed/optional/array/missing/overlapping/out-of-bounds members.
  Over-limit counts and non-representable 32-bit `u64` wire values are typed
  range errors.
- Unpublished workspace crate `ergo-aeron-cluster-test-harness` owns the Java
  ClusterLauncher, harness tests, and examples. Release automation unpacks
  `ergo-aeron-cluster` with every advertised feature and fails closed.

### Changed
- **Breaking:** `ergo-aeron-cluster` no longer advertises a `test-harness`
  feature or `test_support` module. Depend on `ergo-aeron-cluster-test-harness`
  in this repository.
- `AnyMessage::decode_frame` rejects every `frame_len` in `0..HEADER_LENGTH`
  before reading header fields, even when the backing slice contains a later
  valid header.

## [0.1.21] — 2026-08-24

### Added
- `GenerationConfig::with_manual_domain_type` and `with_domain_type` now emit
  correctly `Option`-wrapped `try_*` accessors for optional (or
  `sinceVersion`-gated) domain-typed fields inside repeating groups, matching
  the message-level behaviour. Generated `Display`/`Debug` for group-entry
  domain-typed `Primitive`/`Enum` fields now shows the converted domain value
  instead of failing to compile.

### Changed
- **Breaking:** `BuildError` is `#[non_exhaustive]`. `Io(std::io::Error)` is
  now `Io { action: &'static str, path: PathBuf, source: std::io::Error }`,
  naming the exact attempted step and destination instead of a bare I/O
  message.
- `generate_multi_to_dir` / `write_module_set` publish a complete generated
  module set as one all-or-nothing unit: every module is staged to a unique
  temp file before any destination is touched, and a mid-commit failure
  restores every pre-existing destination and leaves no temp/backup debris.
  A cleanup step that itself fails is reported in preference to the failure
  that triggered it, so a rollback never reports itself clean while leaving
  debris. Deleting the backups of a *successful* commit is past the point of
  no return and does not roll back: every backup that can be removed is
  removed, then the publication fails naming the first that could not.
- `GenerationConfig::with_keyword_append_token` is validated at `generate()`:
  an empty or invalid token now returns
  `GenerateError::InvalidConfiguration` instead of generating uncompilable
  Rust.
- **Breaking:** `SessionBuilder::ingress_endpoints` returns
  `Result<Self, ClusterError>` and validates the endpoint map (grammar,
  duplicate IDs, empty entries) at the call site instead of at `validate()`
  or the first `poll()`.
- **Breaking:** `poller::parse_event` returns `Result<EgressEvent, ClusterError>`
  (was `Result<Option<EgressEvent>, ClusterError>` — every decoded branch
  was already `Some`, so `None` was unreachable).
  `poller::parse_leader_endpoint` returns `Result<Option<String>, ClusterError>`
  (was `Option<String>`) — a malformed endpoint map is now `Err` with the
  parser's specific reason, distinct from `Ok(None)` for a well-formed map
  that simply lacks the requested leader.
- Message-bearing `#[must_use]` on pure cluster decision observers:
  `PublicationFailure::is_retryable`/`raw_code`, `ClusterError::is_retryable`,
  `AeronCluster::{cluster_session_id, leadership_term_id, leader_member_id,
  is_ingress_connected, is_ingress_closed, ingress_position,
  is_egress_connected, state}`, `ClusterClaim::position`, and
  `AsyncClusterConnect::{step, is_complete}`.
- **Breaking:** `PublicationFailure` is `#[non_exhaustive]` and gains
  `TooManyParts`, mapped directly from `AeronOfferError::TooManyParts`
  instead of the fabricated sentinel `Other(-100)`. `raw()` is replaced by
  `raw_code() -> Option<i64>` (`None` for `TooManyParts`, which has no real
  Aeron wire code).

- The generated public-API freeze gate (`api/generated/*.txt`) now snapshots
  the canonical semver-relevant surface — struct fields, enum variant
  payloads/discriminants, full fn/method signatures (receiver, generics,
  args, return type, where-clause), associated types/consts, type aliases,
  and cfg/non_exhaustive/repr/deprecated/must_use attributes — instead of
  item names only. A cfg-gated item is now recorded (with its condition)
  instead of being silently excluded from the freeze.
- The `SbeMessage` sealing-trait path is now explicit per-schema state
  (`GenerationContext`, passed by reference into decoder/encoder generation)
  instead of a `thread_local!`. A hook that invokes a nested `Generator`
  (e.g. to emit a companion crate) can no longer leak its sealed path into
  the outer generation's remaining messages.
- `sbe/BENCHMARKS.md` and `book/src/sbe/benchmarks.md`'s "Group encode: LTO
  on and off" section no longer quotes unprovenanced point estimates
  (`414.1 ns`, etc., with no run-id/commit/host and duplicated verbatim
  across both files) as if they were current — it states the qualitative
  ranking and the reproduction command instead.
- Generated domain-object `encoded_length()` / `encoded_length_with_header()`
  rustdoc no longer contradicts `encode()`: `encoded_length()` is documented
  as body-only, and `encoded_length_with_header()` as the exact buffer size
  and return value of `encode()` for both fixed and dynamic messages.
- Generated `{Group}Encoder::wrap`, `ENTRY_BLOCK_LENGTH`, and
  `GROUP_DIM_TEMPLATE` now carry authored rustdoc explaining the standalone
  group framing contract (offset is the first entry, the dimension header is
  the caller's responsibility) instead of a placeholder.

### Fixed
- Domain-object generation for a required (non-`sinceVersion`) boolean field
  no longer panics on an unknown wire discriminant; `try_from_decoder`
  propagates `DecodeError::InvalidBoolean` instead.
- Single-line schema `description` text containing `&`, `<`, or `>` (e.g.
  `Option<u32>`) is now escaped before emission, so it can no longer break a
  downstream `-D warnings` rustdoc build with `rustdoc::invalid_html_tags`.

## [0.1.20] — 2026-08-20

### Added
- `GeneratedModuleSet::into_parts` consumes the set and returns owned modules
  and warnings without cloning generated source.
- `SchemaFile` plus `generate_multi_to_dir` / `generate_multi_to_out_dir`
  generate a shared schema and its consumers in one transaction, using
  supplied module names and watching every resolved include.
- `EncodeError::InvalidAscii { field }` for ASCII fixed-array `*_str` writes.
- Generated public-API snapshots under `api/generated/` (`car_lean`,
  `car_domain`, `multi_schema_shared`) and
  `scripts/check-generated-public-api.sh`, invoked from `just preflight`.
- `GenerationConfig::with_manual_domain_type` for additive Manual
  `TryFromSbe`/`TryToSbe` mappings. `with_domain_type(selector, path)` is
  the two-argument Generated path.

### Changed
- `GenerationConfig::with_domain_type` stays the two-argument
  `(selector, path)` Generated path it has always been. Use
  [`with_manual_domain_type`] for additive Manual impls.
- **Breaking:** `ParseError` is `#[non_exhaustive]`. Root file reads are
  `Io { path, source }`. Include failures are `Include { href, attempted,
  cause }` with typed `IncludeCause` (`Cycle` / `Io` / `NotFound`).
- **Breaking:** `SessionBuilder::ingress_channel`, `egress_channel`,
  `message_timeout`, and `new_leader_timeout` return `Result` and store only
  validated values. `is_ingress_exclusive` and `owns_aeron` setters are
  removed.
- **Breaking:** generated `*_str` exists only for default-ASCII `char` and
  encodings `ASCII` / `US-ASCII` / `UTF-8` / `UTF8`. Unencoded numeric arrays
  and encodings such as GB18030 keep the raw array setter only — schemas
  relying on the old setter for those types need the raw accessor instead.
- Optional var-data accessors (`into_*_as_compact_str`, `into_*_as_smol_str`,
  `into_*_as_bytes`) are always emitted and gated on the *consumer's*
  `compact_str` / `smol_str` / `bytes` feature, not the generator's. Fixes a
  feature leak where the generator's own feature flags decided what a
  consumer crate could see, independent of that crate's own Cargo features.
- Generated rustdoc snippets referencing a `DomainImpl::Manual` field render
  as plain text instead of an ignored `rust,ignore` fence.
- Side-effect-free generated observers (`as_option` / `as_bool`, completed
  tail views/lengths, encoder metadata, group `written`, exact-length
  terminals) carry message-bearing `#[must_use]`.

### Fixed
- One-byte fixed-array getters (`char` / `uint8`) return the bulk-read
  `[u8; N]` instead of reconstructing each element with `from_le_bytes`.
- `parse_with_xsd_validation` accepts schema-declared enum `nullValue`
  (signed and unsigned) and still rejects unknown non-namespaced enum
  attributes.
- `generate_to_dir` emits `cargo::rerun-if-changed` for the root schema and
  every nested or transitive include, so an include-only edit rebuilds.
- Unresolved-type diagnostics name the containing field as well as the
  invalid type.

### Deprecated
- `GenerationConfig::with_error_from_impls` — it converts generated encode/decode
  errors through `format!` and `From<String>`, dropping fields such as `needed`
  and `available`. Implement `From<generated::sbe_rt::EncodeError>` /
  `From<generated::sbe_rt::DecodeError>` instead. Removal is scheduled for 1.0.

## [0.1.19] — 2026-08-19

### Fixed
- A composite member with `presence="optional"` and a schema `nullValue`
  (e.g. a `PriceNull9`-style Decimal's `mantissa`) now decodes as `Option<T>`,
  checked against the wire null sentinel, instead of silently misreading the
  null image as a real value. `with_domain_type` mappings onto such a
  composite (e.g. `rust_decimal::Decimal`) now fail closed with a typed error
  on the null image rather than decoding the sentinel as a huge/wrong number.

## [0.1.18] — 2026-08-18

### Added
- Reject unknown attributes on `<messageSchema>`, `<message>`, `<field>`,
  `<group>`, and `<data>` at parse time, so an authoring typo (`presense`,
  `semanticTpye`) is an error instead of being silently ignored. Namespaced
  attributes (`xsi:*`, `xi:*`, and vendor namespaces such as Binance's
  `mbx:*`) are outside the SBE grammar and still pass.
- Multiple distinct decimal composites (`Decimal64`, `Decimal128`, …) can each
  map to `rust_decimal::Decimal` via `with_domain_type`; each gets its own
  `TryFromSbe`/`TryToSbe` impl.

### Fixed
- `validate_against_sbe_xsd` no longer rejects valid real-world schemas. It
  dropped every namespaced vendor attribute check (`mbx:exponent`) on the
  floor, and its allow-lists omitted `characterEncoding` on `<data>`, `unit`
  on `<type>`, `jsonValue` on `<validValue>` / `<choice>`, and `package` on
  `<types>` — enough to reject checked-in `l3-book` and `binance-spot`
  schemas. Attribute allow-lists are now shared with the parser so the two
  cannot drift apart.
- Domain-typed optional fields no longer produce a type mismatch. An optional
  `rust_decimal::Decimal` composite decodes to a plain `Decimal` (a composite
  has no null image), an optional `chrono::DateTime<Utc>` decodes to
  `Option<DateTime>`, and the encoder setters take the plain domain value.
- A boolean enum mapped to `bool` via `with_domain_type` now emits its
  `TryFromSbe`/`TryToSbe` impl once per shared module instead of in every
  consumer, which caused a "conflicting implementation" across multi-schema
  modules.

### Changed
- Fixed-only encoders now carry `FieldsState`: `as_bytes_with_header`,
  `as_body_bytes`, `encoded_length*`, and `into_remaining_mut` exist only
  after `fixed(&FixedFields)`, so a reused buffer cannot publish or pack
  unwritten required fields.
- Cluster bench gate uses the same literal `1.00` ceiling as SBE and requires
  `--run-id` provenance (`just bench-cluster` stamps per-estimate run ids).
- `check-public-api.sh` strips a leading `v` from `baseline_tag` before
  passing `--baseline-version` to cargo-semver-checks.

### Fixed
- Schema `nullValue` / `minValue` / `maxValue` that do not fit the declared
  primitive width are rejected at parse time (no more `256_u64 as u8`).
  Signed enum `validValue`s are compared as signed, so `int8` `-1` is
  accepted when `minValue="-5"` and `maxValue="5"`.
- Maintained cluster decode benches wrap both arms at the body offset without
  extra template/schema/version work on the sbe-tool side.
- Docs-validation temp crates `cargo fetch` before `CARGO_NET_OFFLINE=true`,
  so a clean CI runner is not missing optional transitive crates.

## [0.1.17] — 2026-08-14

### Changed
- **Breaking:** writing a message's tail now requires the fixed block first.
  `wrap*` returns `{Msg}UnfixedEncoder`; groups and var-data are reachable only
  after `.fixed(&{Msg}FixedFields { … })`. "Tails before the fixed block" is now
  unrepresentable rather than a runtime hazard.
- Gate the fixed-stride group proof APIs (`add_checked`, `start_entry`,
  `complete`, `EntryComplete`) on groups that actually have a fixed stride. A
  dynamic entry with a var-data tail can no longer claim completion.
- `ClusterError` is `#[non_exhaustive]`, so adding a variant is not a breaking
  change for downstream matches.
- Decode errors keep their UTF-8 cause in the error chain instead of flattening it.
- Malformed XML and schema-invalid SBE now surface as distinct errors.

### Added
- Schema `description` text is emitted as rustdoc on messages, fields,
  composite members, enum variants, group entries, and var-data.
- `#[must_use]` on pure generated observers — getters, set predicates, raw
  enum/set values, and metadata queries — so discarding them is a warning.
- Multi-schema generation validates that same-named shared types have identical
  wire fingerprints, and rejects duplicate or non-identifier module names.
- Fail-closed benchmark evidence packaging (`scripts/package-bench-artifacts.sh`)
  wired into both the release workflow and `just release`.
- sbe-tool parity benchmarks for optional-enum nullification and
  group-with-data, both gated at the strict `1.00` ceiling.

### Fixed
- Optional primitive fields write the schema null image when set to `None`,
  including fixed arrays and nested optional composite members.
- Cluster egress decoding runs through one canonical fail-closed path; short,
  truncated, and invalid-text frames report the real error instead of a
  misleading timeout.
- `parse_event` reports the real template id for decoded-but-unprojected frames
  instead of `0`.
- Session builder timeouts reject zero, sub-millisecond, and overflow values,
  and each timeout field tracks its own error so one cannot mask another.
- `StaticCredentials` and `SessionBuilder` redact credential material from
  `Debug` output.
- Duplicate member IDs in an ingress endpoint map are rejected.
- Generated multi-paragraph rustdoc is emitted one line per `#[doc]`, so it is
  no longer parsed as an indented Markdown code block and compiled as a doctest.

## [0.1.16] — 2026-08-10

### Changed
- Decoder structs use `offset` field (matching sbe-tool naming), replacing internal `base_addr` cache. `read_addr_unchecked` helper removed.
- Generated encoder cursors renamed `offset` for consistency across all decoder/encoder types.

### Added
- Historic ergo regression benchmarks — null-as-option, optional fields, Decimal converters. Gate: `just bench-historic`, `scripts/check-bench-historic.sh`.
- Multi-message/framing documentation: `AnyMessage` dispatch, back-to-back and `remaining()` framing approaches.
- miette error diagnostics page with rendered examples of schema parse errors.

### Fixed
- toml 1.x `FromStr` compat: `toml::Value::from_str` replaced with `toml::from_str` for full-document parsing.
- Matrix test crate missing feature flags (`compact_str`, `smol_str`, `bytes`).
- Book: removed incorrect "Java-style" label from parent-hopping sections.

## [0.1.15] — 2026-08-09

### Added
- **`with_null_as_option(ConversionSelector::named_type("EventCode"))`** — enum fields become `Option<EventCode>`. `NullVal` wire byte → `None`.
- **`with_all_enums_as_option()`** — every enum in the schema → `Option<Enum>`.
- **`as_option()` on every generated enum** — `event_code.as_option()` → `Option<EventCode>`.
- **`nullValue` on `<enum>` elements** now parsed from XML. `<enum encodingType="uint8" nullValue="99">` → `NullVal = 99`.

### Changed
- Crate description: `"Simple Binary Encoding (SBE)..."` for crates.io visibility.
- Group-entry enum fields support `with_null_as_option`.

## [0.1.14] — 2026-08-08

### Breaking
- `SessionState::AwaitingNewLeaderConnection` removed (merged into `AwaitingNewLeader`).
- `ClusterError` now `#[non_exhaustive]`, gains `InvalidTimeout` and `PayloadTooLarge`.
- `AsyncClusterConnect::step()` returns `ConnectStep` (non-exhaustive, wildcard required).
- `GenerationConfig::profile(Lean)` no longer clears explicit conversions/domain-types. Use `.lean()` for a clean baseline.

### Added
- `#[must_use]` on `SessionBuilder`, `AsyncClusterConnect`, `ClusterClaim`.
- `ConnectStep` re-exported at crate root.
- `checked_deadline` helper — zero/overflow timeouts return `InvalidTimeout`.
- `PayloadTooLarge` error with `operation`, `requested`, `maximum` fields.
- Zero-alloc fragmented path via `offer_parts`.
- Feature-gated integrations: `compact_str`, `smol_str`, `bytes`, `chrono`.
- `DomainVarData::CompactStrings`, `SmolStrings`, `BytesCrate` variants.
- `lean()` constructor on `GenerationConfig`.
- Module name validation + path containment.
- `#[must_use]` on codec tail stages.
- FIX SBE conformance suite documented.

### Fixed
- Malformed egress frames (< 8 bytes) → `ProtocolError`, not silently unknown.
- Decode errors propagated on poll path.
- `checked_add` overflow in `offer()`.
- Stale channel parse errors cleared on re-set.
- Benchmark docs match gate: SBE zero tolerance, cluster 0.005.

## [0.1.13] — 2026-08-07

### Breaking
- `SbeMessage` is a sealed trait — only generated types can implement it.
- Staged `EncodedLength` types renamed for stage-just-completed convention.
- Group decoders no longer expose `wrap_with_parent`.
- Safe constructors prove fixed extent and panic if short; `unsafe *_unchecked` skips.
- `From<BooleanType> for bool` removed — use `as_bool()`.
- `EncodeError::BufferTooShort` carries `field`.
- `DomainConversionFailed` carries `reason`.

### Added
- `add_checked` — group entries proven complete at compile time.
- Group decoders poison themselves on malformed entry.
- `#{inline}` on `bulk_decode`, staged finish helpers, domain DTO methods.
- `Error::source` on `EncodeError::Decode` and `VerifyError::DecodeError`.
- `#[must_use]` on consuming stages and `EncodedLength` stages.

### Fixed
- Constant-presence fields no longer count toward readable fixed extent.
- Aeron `try_claim` recipe rewritten against real API.
- Bench batch-decode uses `wrap_unchecked` for equal work.
- Deduped reserved method-name lists.
