<!-- Generated page: edits here are overwritten by the next export. sha256:923b79f5b78fb0794a8e84e97d098f94e0291c3e44b66140039e3afda270d674 -->
# Which Transport When

One table for the choice. Numbers are round trips of 64-byte messages, µs,
p50 / p99 / max, as ranges over loads and reps; the hardware key is below
the table. Compare numbers only within one platform. Every table behind
them: the [comparison](comparison.md#conclusion).

| need | use | p50 / p99 / max, µs | measured on | busy threads | how |
|---|---|---|---|---|---|
| between threads of one process | Aeron IPC; `std::sync::mpsc` is as fast below 1,000,000 msg/s | Aeron 0.22 / 0.30–0.39 / 118–146; `mpsc` 0.30–0.33 / 0.45 / 106–143. At 5,000,000 msg/s Aeron 0.25 / 0.38 / 175, `mpsc` falls behind | Azure | Aeron 5 (driver and both ends), `mpsc` 2 | [Aeron client](aeron-client.md) (`aeron:ipc`) |
| between processes on one host | Aeron IPC or a spy, not UDP to the host's own address | spy 0.26 / 0.32–0.42 / 4–108; kernel UDP to itself 10.3–13.9 / 12–20 / 31–181 | Azure; spy with a live UDP destination 0.26 on AWS `c7a` | 5 | [Aeron client](aeron-client.md) |
| between hosts, 1 stream (at 8, not separated from sockets) | Aeron over kernel UDP | Aeron 55.6–59.5 / 60.7–63.7 / 89–2,186; sockets 61.8–62.9 / 66.0–68.9 / 213–2,886. `c7a`: Aeron 40.9 / 48.6 / 420, sockets 47.7–48.4 / 60.8–60.9 / 420–521 | Azure, AWS `c7a` | 4 a host (3 driver, 1 app) | [Aeron client](aeron-client.md) (`aeron:udp`) |
| between hosts, 32 or more streams, up to 50,000 msg/s | plain UDP sockets in one loop: ergon-io (or mio) | sockets 57.8–60.3 / 65.2–66.6 / 200–671; Aeron 62.3–70.0 / 70.8–80.0 / 231–626. `c8g`, 32 streams, 200,000 msg/s: sockets 33.7–34.0 / 43.0–43.6 / 278–299, kernel Aeron 38.7 / 47.4 / 221 | Azure, AWS `c8g` | 1 a host | [UDP](../io/udp.md) |
| between hosts, past what one socket loop carries | Aeron, or ergon-io on a userspace stack (lab) | 500,000 msg/s, 1 stream: Aeron kernel 33.95 / 50.78 / 497, Aeron on DPDK 24.78 / 33.76 / 219, ergon-io on smoltcp 36.03 / 78.21 / 1,908; kernel socket loops fall seconds behind (they carry 200,000 on `c8g`, 151,000–195,000 on Azure) | AWS `c8g` | Aeron 4 a host, smoltcp 1 | [Aeron client](aeron-client.md), [UDP batching](comparison.md#aws-udp-batching-graviton) |
| bursts that each need a reply | reply to each message as it is read, or send the burst as one message | bursts of 8: reply per message 40.90 / 51.77 / 179; reply after the poll pass 45.50 / 54.85 / 184 | AWS `c8g` | 1 a host | [UDP batching](comparison.md#aws-udp-batching-graviton) |
| many logical streams (a symbol, an account) | a few Aeron streams, the key in each message | Aeron's p50 rose 10.5–12.6 from 1 to 128 streams; a socket loop's did not | Azure | as above | [Aeron with many streams](aeron-many-streams.md) |
| reliable delivery between your own hosts | Aeron reliable UDP (the default `aeron:udp`) | no loss: Aeron 54.8–57.6 / 59.9–64.3 / 902–3,725; TCP 64.7–69.8 / 79–225 / 1,366–8,027 | Azure | Aeron 4 a host, TCP 1 | [Aeron client](aeron-client.md) |
| between regions (about 150 ms round trip) over a lossy path | gaps acceptable: UDP, ergon-io or mio. Every message needed, 0.1 % or bursty loss: Aeron reliable, its window inside the socket buffer the kernel keeps and several messages a datagram, while loss × datagrams a second × RTT stays under about 1 (held at 150 ms round trip, not at 250 ms and 50,000 msg/s) (at T, pack several messages into one offer in the application, up to one frame or a 1 ms wait, with the driver busy-spinning; a 1 ms driver idle sleep is not needed. L and bursty loss were measured with the sleep only). At 1 % nothing tested keeps up on one path at 10,000 to 50,000 msg/s: put the reliable copy on a second path with loss × datagrams a second × RTT under about 1 | **ms, one way**, 1 % loss: UDP 75.04 / 75.04 / 75.30–91.50 (the delay is 75), 99.0 % delivered. 0.1 %, Aeron with application batching: 10,000 msg/s 75.63 / 218.37 / 269.35, 50,000 msg/s 75.37 / 408.42 / 565.72, 100 % delivered (the same arm unpacked: p50 1,508 and 1,166); kernel TCP (cubic) fell seconds behind. Aeron on an 88 ms backbone at 0.01 %: 236–531 / 1,063–1,430 / 1,327–1,667, 100 % delivered | desktop netem | UDP 2, Aeron 2 and its driver agent | [long distance](comparison.md#long-distance-with-loss-one-box-netem) |
| a service that speaks TCP, WebSocket or HTTP | ergon-io, backend by socket count | WebSocket order round trip, 10,000/s: 1 socket `recv-busy` 36.0–36.2 / 43.9–46.0 / 148–284; 64 sockets `uring-sqpoll` 40.0–40.1 / 53.7–54.8 / 110–131 | AWS `c8g` fair run | 1, plus its helpers off the loop's CPU | [choosing a backend](../io/overview.md#choosing-a-backend) |
| request/response with timeout, multiplexed, one server and many clients | same host up to about 100 clients: Aeron IPC response channels; otherwise framed TCP (WebSocket if the peer needs it); never HTTP/1.1 (in-order replies) | same host, unary: IPC 0.9 / 16.6 / 18.4 (1 client), 2.1 / 11.7 / 29.5 (100); TCP 23.1 / 235.1 / 1,730 (1), 17.6 / 21.9 / 162 (100), 16.8 / 36.0 / 4,817 (1000); Aeron UDP response channels 18.0 / 51.6 / 58.8 (1), 33.6 / 60.4 / 6,058 (100), did not start 1000 | one x86 desktop host; two-host latency deferred | IPC: driver 1; TCP: 1 a side | [Request/response](../io/request-response.md) |
| one request, many responses (per-client streams) | same host up to about 100 clients: Aeron IPC; otherwise framed TCP; not a shared MDC publication | first item, same host: IPC 1.0 / 8.8 / 11.3 (1), 3.1 / 11.1 / 85.4 (100); TCP 19.5 / 43.6 / 74.4 (1), 22.9 / 64.1 / 349 (1000); shared MDC 2,626 / 4,698 / 88,867 (1000) | one x86 desktop host | as above | [Request/response](../io/request-response.md#b-one-request-many-responses) |
| replicated state (consensus) | Aeron Cluster, through ergo-aeron-cluster's client | not measured here | | the cluster's own | [cluster client](../cluster/overview.md) |
| lower latency than the kernel gives, between hosts, supported | kernel bypass with [Aeron Premium](https://aeron.io/) | not measured here | | its own | vendor |
| learning how a DPDK transport works, reproducing the lab numbers | ergon-md-dpdk, `fstack-spin`, `smoltcp-spin`: lab only | Aeron on DPDK, reliable 27.5–30.7 / 35.5–38.7 / 170–297, unreliable 28.0 / 35.4 / 177; F-Stack TCP 32.1–50.4 / 40.2–83.8 / 165–297, against kernel TCP 52.2–61.1 / 63.6–85.0 / 298–536 | AWS `c7a` | a core per network thread | [ergon-md-dpdk](../md-dpdk/overview.md) |

| label | instances | CPU | NIC | OS |
|---|---|---|---|---|
| Azure | `Standard_F8as_v6` / `F8als_v6` with `F8ams_v6`, one proximity placement group | AMD EPYC 9V74 (Genoa), 8 cores, no SMT | Mellanox mlx5 VF | Debian 13, kernel 7.2 |
| AWS `c7a` | two `c7a.2xlarge`, one cluster placement group | AMD EPYC 9R14 (Genoa), 8 cores, no SMT | ENA | Debian 13, kernel 7.2 |
| AWS `c8g` | two `c8g.2xlarge`, one cluster placement group | Graviton4 (Neoverse V2), 8 vCPUs, each a whole core | ENA | Debian 13, kernel 7.2 |
| AWS `c8g` fair run | `c8g.2xlarge` with `c8g.xlarge`, Tokyo | Graviton4 (Neoverse V2) | ENA, moderation off | kernel `7.0.0-1014-aws` |
| desktop netem | one 6-core desktop x86 box, both ends in one process; distance and loss from `tc netem` on a namespace's `lo`. Not a latency platform: compare its ms figures only with each other | Intel Core i5-8500T, 6 cores, no SMT | none (`lo`) | Linux 7.2.8 |

Every host: housekeeping on CPU 0, the data NIC's interrupts on CPU 1,
each busy thread alone on an isolated CPU
([hosts and CPUs](comparison.md#hosts-and-cpus)).

- **Kernel UDP between hosts.** On Graviton turn interrupt moderation off
  and put the NIC's interrupts on a core of their own; on x86 no kernel
  setting is recommended
  ([ergon-md-dpdk](../md-dpdk/overview.md#measured-between-two-hosts)).
- **Aeron driver threads.** Dedicated, once there are many streams; each
  thread taken from the driver costs latency
  ([driver threading](comparison.md#what-to-use)).
- **Pods.** The node and pod settings are the same for every transport
  ([Kubernetes](../deploy/kubernetes.md)); one thread that polls Aeron and
  ergon-io together is the [busy-spin loop](../deploy/busy-spin-loop.md).
- **Between regions.** In a pod: the TCP sysctls (`tcp_congestion_control`,
  `tcp_rmem`, `tcp_wmem`) and Aeron's driver settings. On the node:
  `net.core.rmem_max` (it caps Aeron's window), loading `tcp_bbr` and
  allowing `tcp_congestion_control` in the kubelet's `allowedUnsafeSysctls`
  ([long distance](comparison.md#kubernetes)).

