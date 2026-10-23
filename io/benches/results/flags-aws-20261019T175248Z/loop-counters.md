# Loop counters — run flags-aws-20261019T175248Z

Copied verbatim from the run's generated `tables.md` (gitignored by policy),
"Loop counters" blocks, `irqcore` points at 1 and 256 connections and the
WebSocket round trip at N = 1, so the pass costs cited in `BENCHMARK.md` and
the book are checkable from the repository. The `recvmsg_calls` lines are from
each point's `res/<point>/meta.txt` (also not committed). The last section
holds the analyser's attribution for the mio maxima the book cites; the one
before it, the receive-window counter behind the book's market-data caveat.

One pass = 1 ÷ `spins/s` (a mean). mio's harness does not report reads,
kernel CPU, sleeps, preemptions or its longest iteration to these tables:
those cells are 0 or empty. Its `recvmsg` count is in `meta.txt`, below.

## f1-irqcore/md-epoll-spin-c1

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3056298 | 1532679 | 1.99 | 176760327 | 57.83 | 2946005 | 2920461 | 0.0 | 45.2 | false | 0 | 0 | 1119.5 | 23560 / 12 / 1 | 0 | 0 |

## f1-irqcore/md-epoll-spin-c256

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3056286 | 2276961 | 1.34 | 102152609 | 33.42 | 1702543 | 1664594 | 0.0 | 31.2 | false | 0 | 0 | 612.1 | 14253 / 23 / 5 | 0 | 0 |

## f1-irqcore/md-mio-c1

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 3056298 | 0 | 0.00 | 175482675 | 57.42 | 2924711 | 2873057 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

## f1-irqcore/md-mio-c256

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 3056276 | 0 | 0.00 | 171815470 | 56.22 | 2863591 | 2789027 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

## f1-irqcore/md-uring-spin-c1

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 3056271 | 1518525 | 2.01 | 3014578 | 0.99 | 50243 | 34744059 | 0.0 | 4.0 | false | 0 | 0 | 845.1 | 9657 / 18 / 3 | 0 | 0 |

## f1-irqcore/md-uring-spin-c256

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 3056283 | 2305830 | 1.33 | 4029101 | 1.32 | 67152 | 2543086 | 0.0 | 6.3 | false | 0 | 0 | 1015.2 | 14398 / 26 / 6 | 0 | 0 |

## f1-irqcore/md-uring-sqpoll-c1

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3056288 | 1526415 | 2.00 | 1 | 0.00 | 0 | 36414292 | 0.0 | 0.0 | true | 0 | 0 | 845.3 | 2118 / 8 / 2 | 0 | 0 |

## f1-irqcore/md-uring-sqpoll-c256

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3056272 | 2254200 | 1.36 | 1 | 0.00 | 0 | 2714146 | 0.0 | 0.0 | true | 0 | 0 | 230.5 | 2467 / 15 / 5 | 0 | 0 |

## f1-irqcore/md-recv-busy-c1

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3056288 | 1558257 | 1.96 | 104579392 | 34.22 | 1742990 | 1742990 | 98.5 | 65.5 | true | 0 | 0 | 1300.0 | 59358 / 1273 / 2 | 0 | 0 |

## f1-irqcore/md-recv-busy-c256

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3056368 | 1867443 | 1.64 | 102825984 | 33.64 | 1713766 | 6694 | 98.2 | 69.3 | true | 0 | 0 | 1909.5 | 401663 / 401663 / 401663 | 0 | 0 |

## f1-irqcore/md-recv-busy-pq-c1

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3056288 | 1509740 | 2.02 | 128632064 | 42.09 | 2143868 | 1071934 | 97.7 | 61.5 | true | 0 | 0 | 1129.8 | 72270 / 1027 / 2 | 0 | 0 |

## f1-irqcore/md-recv-busy-pq-c256

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3056298 | 2131281 | 1.43 | 98852953 | 32.34 | 1647549 | 179238 | 97.6 | 64.5 | true | 0 | 0 | 804.0 | 5133582 / 16176 / 2424 | 0 | 0 |

## f2-irqcore/md-epoll-spin-c1

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3056397 | 1520004 | 2.01 | 176603972 | 57.78 | 2943400 | 2918066 | 0.0 | 45.2 | false | 0 | 0 | 1451.5 | 25094 / 16 / 3 | 0 | 0 |

## f2-irqcore/md-epoll-spin-c256

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3056806 | 2269706 | 1.35 | 99465674 | 32.54 | 1657761 | 1619933 | 0.0 | 32.8 | false | 0 | 0 | 690.0 | 16227 / 24 / 7 | 0 | 0 |

## f2-irqcore/md-mio-c1

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 3056477 | 0 | 0.00 | 175363451 | 57.37 | 2922724 | 2871388 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

## f2-irqcore/md-mio-c256

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 3056602 | 0 | 0.00 | 171814578 | 56.21 | 2863576 | 2788932 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

## f2-irqcore/md-uring-spin-c1

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 3056401 | 1584223 | 1.93 | 3127657 | 1.02 | 52128 | 34596682 | 0.0 | 4.4 | false | 0 | 0 | 875.9 | 4410 / 15 / 3 | 0 | 0 |

## f2-irqcore/md-uring-spin-c256

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 3056711 | 2297547 | 1.33 | 4015613 | 1.31 | 66927 | 2532577 | 0.0 | 6.6 | false | 0 | 0 | 1199.3 | 15840 / 27 / 7 | 0 | 0 |

## f2-irqcore/md-uring-sqpoll-c1

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3056494 | 1524627 | 2.00 | 1 | 0.00 | 0 | 36412544 | 0.0 | 0.0 | true | 0 | 0 | 884.3 | 1719 / 9 / 2 | 0 | 0 |

## f2-irqcore/md-uring-sqpoll-c256

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3056502 | 2250118 | 1.36 | 1 | 0.00 | 0 | 2719237 | 0.0 | 0.0 | true | 0 | 0 | 245.8 | 1866 / 14 / 5 | 0 | 0 |

## f2-irqcore/md-recv-busy-c1

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3056390 | 1516037 | 2.02 | 102079744 | 33.40 | 1701329 | 1701329 | 98.5 | 66.2 | true | 0 | 0 | 1416.2 | 57262 / 1026 / 2 | 0 | 0 |

## f2-irqcore/md-recv-busy-c256

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3057204 | 1868279 | 1.64 | 102907904 | 33.66 | 1715132 | 6700 | 98.2 | 69.2 | true | 0 | 0 | 2187.1 | 401983 / 401983 / 401983 | 0 | 0 |

## f2-irqcore/md-recv-busy-pq-c1

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3056396 | 1523142 | 2.01 | 128461696 | 42.03 | 2141028 | 1070514 | 97.6 | 61.5 | true | 0 | 0 | 1459.4 | 69170 / 1617 / 3 | 0 | 0 |

## f2-irqcore/md-recv-busy-pq-c256

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3056691 | 2128176 | 1.44 | 99196630 | 32.45 | 1653277 | 179891 | 97.6 | 65.1 | true | 0 | 0 | 896.8 | 5060470 / 17815 / 2642 | 0 | 0 |

## f1-irqcore/rt-epoll-spin-n1-r10000

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 600000 | 600000 | 1.00 | 168152960 | 280.25 | 2802549 | 2782549 | 0.0 | 42.8 | false | 0 | 0 | 133.5 | 4092 / 10 / 1 | 0 | 0 |

```text
loop.0.recvmsg_calls=600000
```

## f1-irqcore/rt-mio-n1-r10000

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 0 | 0 | 0.00 | 168988562 | 0.00 | 2816476 | 2796476 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

```text
loop.0.recvmsg_calls=1200000
```

## f2-irqcore/rt-epoll-spin-n1-r10000

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 600000 | 600000 | 1.00 | 168141312 | 280.24 | 2802355 | 2782355 | 0.0 | 42.8 | false | 0 | 0 | 103.0 | 1633 / 13 / 1 | 0 | 0 |

```text
loop.0.recvmsg_calls=600000
```

## f2-irqcore/rt-mio-n1-r10000

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 0 | 0 | 0.00 | 167800031 | 0.00 | 2796667 | 2776667 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

```text
loop.0.recvmsg_calls=1200000
```

## mio receive-window pressure

The receiver's `TcpExtTCPWantZeroWindowAdv`, after minus before, worked out
from each market-data point's `res/<point>/net-before.txt` and
`net-after.txt`. Every point not listed has 0: the eight `md-mio-c64` and
`md-mio-c256` points, and every ergon-io market-data point in both rounds and
both placements.

| point | c1 | c4 | c16 |
|---|---|---|---|
| f1-irqcore/md-mio | 2825 | 2419 | 948 |
| f2-irqcore/md-mio | 2834 | 2531 | 1051 |
| f1-irqlocal/md-mio | 2402 | 2816 | 1477 |
| f2-irqlocal/md-mio | 2775 | 2862 | 1564 |

## mio maxima: the analyser's attribution

The first entry of each point's worst-case cell in `tables.md`, verbatim, and
the receiver's and sender's TCP counters before and after `f2-irqcore/http-mio-p20`
(`res/<point>/net-*.txt`).

| point | worst case |
|---|---|
| f2-irqcore/http-mio-p20-r10000 | 1478.01 at 51.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| f2-irqlocal/rt-mio-n16-r10000 | 640.75 at 17.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| f2-irqcore/rt-mio-n64-r10000 | 425.45 at 60.0 s: neither loop stalled long enough: network, hypervisor or untraced |

| counter | receiver before / after | sender before / after |
|---|---|---|
| `TcpRetransSegs` | 199 / 199 | 58 / 58 |
| `TcpExtTCPLostRetransmit` | 0 / 0 | 2 / 2 |
| `TcpExtTCPTimeouts` | 1 / 1 | 5 / 5 |
