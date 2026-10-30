# Which Transport When

One table for the choice. The numbers behind each row are in the
[comparison's conclusion](comparison.md#conclusion); each linked page has
its own details.

| need | use | busy threads | how |
|---|---|---|---|
| between threads of one process | Aeron IPC; `std::sync::mpsc` is as fast below 1,000,000 msg/s | Aeron 5 (driver and both ends), `mpsc` 2 | [Aeron client](aeron-client.md) (`aeron:ipc`) |
| between processes on one host | Aeron IPC or a spy, not UDP to the host's own address | 5 | [Aeron client](aeron-client.md) |
| between hosts, 1 stream (at 8, not separated from sockets) | Aeron over kernel UDP | 4 a host (3 driver, 1 app) | [Aeron client](aeron-client.md) (`aeron:udp`) |
| between hosts, 32 or more streams, up to 50,000 msg/s | plain UDP sockets in one loop: ergon-io (or mio) | 1 a host | [UDP](../io/udp.md) |
| between hosts, 200,000 msg/s and more | Aeron: the socket loops fall behind | 4 a host | [Aeron client](aeron-client.md) |
| many logical streams (a symbol, an account) | a few Aeron streams, the key in each message | as above | [Aeron with many streams](aeron-many-streams.md) |
| reliable delivery between your own hosts | Aeron reliable UDP (the default `aeron:udp`) | 4 a host | [Aeron client](aeron-client.md) |
| a service that speaks TCP, WebSocket or HTTP | ergon-io, backend by socket count | 1, plus its helpers off the loop's CPU | [choosing a backend](../io/overview.md#choosing-a-backend) |
| replicated state (consensus) | Aeron Cluster, through ergo-aeron-cluster's client | the cluster's own | [cluster client](../cluster/overview.md) |
| lower latency than the kernel gives, between hosts | kernel bypass with [Aeron Premium](https://aeron.io/) | its own | vendor |
| learning how a DPDK transport works, reproducing the lab numbers | ergon-md-dpdk, `fstack-spin`, `smoltcp-spin`: lab only | a core per network thread | [ergon-md-dpdk](../md-dpdk/overview.md) |

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
