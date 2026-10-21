# Work behind the percentiles — run flags-aws-20261019T175248Z

Extracted from the run's generated `tables.md` (gitignored by policy) so the
completeness counts cited in `BENCHMARK.md` are checkable from the repository.
Counts are the run's own account, reconciled against `records.bin` before any
warm-up is dropped. Every percentile in `BENCHMARK.md` is over `completed`
alone, after the warm-up cut, so `n` is smaller still.

`scheduled` `issued` `completed` `late` `rejected` `censored`

## HTTP round trip, http-epoll-spin-p1-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600001 | 600001 | 93 | 0 | 0 |
| f1-irqlocal | 600001 | 600001 | 600001 | 94 | 0 | 0 |
| f2-irqlocal | 600001 | 600001 | 600001 | 89 | 0 | 0 |
| f2-irqcore | 600001 | 600001 | 600001 | 93 | 0 | 0 |

## HTTP round trip, http-epoll-spin-p20-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600001 | 600001 | 94 | 0 | 0 |
| f1-irqlocal | 600001 | 600001 | 600001 | 96 | 0 | 0 |
| f2-irqlocal | 600001 | 600001 | 600001 | 122 | 0 | 0 |
| f2-irqcore | 600001 | 600001 | 600001 | 102 | 0 | 0 |

## HTTP round trip, http-recv-busy-p1-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600001 | 600001 | 87 | 0 | 0 |
| f2-irqcore | 600001 | 600001 | 600001 | 94 | 0 | 0 |

## HTTP round trip, http-recv-busy-p20-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600001 | 600001 | 142 | 0 | 0 |
| f2-irqcore | 600001 | 600001 | 600001 | 196 | 0 | 0 |

## HTTP round trip, http-recv-busy-pq-p1-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600001 | 600001 | 88 | 0 | 0 |
| f2-irqcore | 600001 | 600001 | 600001 | 93 | 0 | 0 |

## HTTP round trip, http-recv-busy-pq-p20-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600001 | 600001 | 109 | 0 | 0 |
| f2-irqcore | 600001 | 600001 | 600001 | 135 | 0 | 0 |

## HTTP round trip, http-uring-spin-p1-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600001 | 600001 | 89 | 0 | 0 |
| f1-irqlocal | 600001 | 600001 | 600001 | 91 | 0 | 0 |
| f2-irqlocal | 600001 | 600001 | 600001 | 92 | 0 | 0 |
| f2-irqcore | 600001 | 600001 | 600001 | 97 | 0 | 0 |

## HTTP round trip, http-uring-spin-p20-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600001 | 600001 | 97 | 0 | 0 |
| f1-irqlocal | 600001 | 600001 | 600001 | 116 | 0 | 0 |
| f2-irqlocal | 600001 | 600001 | 600001 | 95 | 0 | 0 |
| f2-irqcore | 600001 | 600001 | 600001 | 140 | 0 | 0 |

## HTTP round trip, http-uring-sqpoll-p1-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600001 | 600001 | 88 | 0 | 0 |
| f2-irqcore | 600001 | 600001 | 600001 | 93 | 0 | 0 |

## HTTP round trip, http-uring-sqpoll-p20-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600001 | 600001 | 106 | 0 | 0 |
| f2-irqcore | 600001 | 600001 | 600001 | 99 | 0 | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-epoll-spin-c1

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056298 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3056666 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3056795 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056397 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-epoll-spin-c16

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3056993 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3057207 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056388 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-epoll-spin-c256

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056286 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3057197 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3057694 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056806 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-epoll-spin-c4

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056352 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3056901 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3056902 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056401 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-epoll-spin-c64

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056287 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3057099 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3057388 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056494 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-mio-c1

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056298 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3056373 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3056457 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056477 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-mio-c16

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056266 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3056972 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3057098 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056401 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-mio-c256

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056276 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3057169 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3057281 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056602 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-mio-c4

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056361 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3056884 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3056967 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056479 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-mio-c64

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3057105 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3057206 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056479 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy-c1

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056390 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy-c16

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056361 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056470 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy-c256

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056368 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3057204 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy-c4

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056353 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056403 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy-c64

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056373 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056401 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy-pq-c1

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056396 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy-pq-c16

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056295 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056468 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy-pq-c256

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056298 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056691 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy-pq-c4

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056354 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056457 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy-pq-c64

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056298 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056401 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-uring-spin-c1

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056271 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3056768 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3056764 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056401 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-uring-spin-c16

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056276 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3057007 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3057194 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056399 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-uring-spin-c256

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056283 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3057112 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3057271 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056711 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-uring-spin-c4

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056266 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3056878 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3056969 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056401 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-uring-spin-c64

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |
| f1-irqlocal | not yet measured | not yet measured | 3057010 | not yet measured | not yet measured | 0 |
| f2-irqlocal | not yet measured | not yet measured | 3057611 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056388 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-uring-sqpoll-c1

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056494 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-uring-sqpoll-c16

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056394 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-uring-sqpoll-c256

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056272 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056502 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-uring-sqpoll-c4

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056364 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056474 | not yet measured | not yet measured | 0 |

## Market-data burst, kernel arrival → decoded (every message), md-uring-sqpoll-c64

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | not yet measured | not yet measured | 3056287 | not yet measured | not yet measured | 0 |
| f2-irqcore | not yet measured | not yet measured | 3056399 | not yet measured | not yet measured | 0 |

## WebSocket order round trip with feed connections on the loop, rt-epoll-spin-n1-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 0 | 0 | 0 |
| f1-irqlocal | 600001 | 600000 | 600000 | 0 | 0 | 0 |
| f2-irqlocal | 600001 | 600000 | 600000 | 1 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 0 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-epoll-spin-n16-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 6 | 0 | 0 |
| f1-irqlocal | 600001 | 600000 | 600000 | 13 | 0 | 0 |
| f2-irqlocal | 600001 | 600000 | 600000 | 7 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 2 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-epoll-spin-n4-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 29 | 0 | 0 |
| f1-irqlocal | 600001 | 600000 | 600000 | 5 | 0 | 0 |
| f2-irqlocal | 600001 | 600000 | 600000 | 4 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 2 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-epoll-spin-n64-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 3 | 0 | 0 |
| f1-irqlocal | 600001 | 600000 | 600000 | 10 | 0 | 0 |
| f2-irqlocal | 600001 | 600000 | 600000 | 21 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 4 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-mio-n1-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600157 | 600000 | 600000 | 0 | 0 | 0 |
| f1-irqlocal | 600161 | 600000 | 600000 | 0 | 0 | 0 |
| f2-irqlocal | 600161 | 600000 | 600000 | 0 | 0 | 0 |
| f2-irqcore | 600155 | 600000 | 600000 | 0 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-mio-n16-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600154 | 599997 | 599997 | 0 | 0 | 0 |
| f1-irqlocal | 600247 | 599995 | 599995 | 47 | 0 | 0 |
| f2-irqlocal | 600257 | 599995 | 599995 | 27 | 0 | 0 |
| f2-irqcore | 600158 | 599997 | 599996 | 0 | 0 | 1 |

## WebSocket order round trip with feed connections on the loop, rt-mio-n4-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600156 | 599997 | 599997 | 0 | 0 | 0 |
| f1-irqlocal | 600216 | 599996 | 599996 | 1 | 0 | 0 |
| f2-irqlocal | 600216 | 599996 | 599996 | 4 | 0 | 0 |
| f2-irqcore | 600165 | 599997 | 599997 | 1 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-mio-n64-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600154 | 599996 | 599996 | 0 | 0 | 0 |
| f1-irqlocal | 600261 | 599995 | 599994 | 4 | 0 | 1 |
| f2-irqlocal | 600265 | 599994 | 599994 | 4 | 0 | 0 |
| f2-irqcore | 600156 | 599996 | 599996 | 24 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-recv-busy-n1-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 0 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 27 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-recv-busy-n16-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600005 | 600004 | 600004 | 5 | 0 | 0 |
| f2-irqcore | 600006 | 600005 | 600005 | 6 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-recv-busy-n4-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600002 | 600001 | 600001 | 3 | 0 | 0 |
| f2-irqcore | 600001 | 600001 | 600000 | 2 | 0 | 1 |

## WebSocket order round trip with feed connections on the loop, rt-recv-busy-n64-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600002 | 501710 | 501709 | 501709 | 0 | 1 |
| f2-irqcore | 600001 | 501971 | 501970 | 501970 | 0 | 1 |

## WebSocket order round trip with feed connections on the loop, rt-recv-busy-pq-n1-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600001 | 600000 | 0 | 0 | 1 |
| f2-irqcore | 600001 | 600000 | 600000 | 1 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-recv-busy-pq-n16-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 3 | 0 | 0 |
| f2-irqcore | 600003 | 600002 | 600002 | 3 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-recv-busy-pq-n4-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 3 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 3 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-recv-busy-pq-n64-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600004 | 600002 | 600002 | 7 | 0 | 0 |
| f2-irqcore | 600003 | 600002 | 600001 | 7 | 0 | 1 |

## WebSocket order round trip with feed connections on the loop, rt-uring-spin-n1-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 1 | 0 | 0 |
| f1-irqlocal | 600001 | 600000 | 600000 | 0 | 0 | 0 |
| f2-irqlocal | 600001 | 600000 | 600000 | 1 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 0 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-uring-spin-n16-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 2 | 0 | 0 |
| f1-irqlocal | 600001 | 600000 | 600000 | 6 | 0 | 0 |
| f2-irqlocal | 600001 | 600000 | 600000 | 5 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 2 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-uring-spin-n4-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 1 | 0 | 0 |
| f1-irqlocal | 600001 | 600000 | 600000 | 7 | 0 | 0 |
| f2-irqlocal | 600001 | 600000 | 600000 | 4 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 2 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-uring-spin-n64-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 3 | 0 | 0 |
| f1-irqlocal | 600001 | 600000 | 600000 | 32 | 0 | 0 |
| f2-irqlocal | 600001 | 600000 | 600000 | 11 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 4 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-uring-sqpoll-n1-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 23 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 0 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-uring-sqpoll-n16-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 3 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 2 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-uring-sqpoll-n4-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 1 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 1 | 0 | 0 |

## WebSocket order round trip with feed connections on the loop, rt-uring-sqpoll-n64-r10000

| | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore | 600001 | 600000 | 600000 | 3 | 0 | 0 |
| f2-irqcore | 600001 | 600000 | 600000 | 3 | 0 | 0 |

## f1-irqcore/md-epoll-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-epoll-spin-c1 | not yet measured | not yet measured | 3056298 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-epoll-spin-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-epoll-spin-c16 | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-epoll-spin-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-epoll-spin-c256 | not yet measured | not yet measured | 3056286 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-epoll-spin-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-epoll-spin-c4 | not yet measured | not yet measured | 3056352 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-epoll-spin-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-epoll-spin-c64 | not yet measured | not yet measured | 3056287 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-mio-c1: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-mio-c1 | not yet measured | not yet measured | 3056298 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-mio-c16: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-mio-c16 | not yet measured | not yet measured | 3056266 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-mio-c256: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-mio-c256 | not yet measured | not yet measured | 3056276 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-mio-c4: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-mio-c4 | not yet measured | not yet measured | 3056361 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-mio-c64: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-mio-c64 | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-recv-busy-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-recv-busy-c1 | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-recv-busy-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-recv-busy-c16 | not yet measured | not yet measured | 3056361 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-recv-busy-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-recv-busy-c256 | not yet measured | not yet measured | 3056368 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-recv-busy-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-recv-busy-c4 | not yet measured | not yet measured | 3056353 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-recv-busy-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-recv-busy-c64 | not yet measured | not yet measured | 3056373 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-recv-busy-pq-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-recv-busy-pq-c1 | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-recv-busy-pq-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-recv-busy-pq-c16 | not yet measured | not yet measured | 3056295 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-recv-busy-pq-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-recv-busy-pq-c256 | not yet measured | not yet measured | 3056298 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-recv-busy-pq-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-recv-busy-pq-c4 | not yet measured | not yet measured | 3056354 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-recv-busy-pq-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-recv-busy-pq-c64 | not yet measured | not yet measured | 3056298 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-uring-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-uring-spin-c1 | not yet measured | not yet measured | 3056271 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-uring-spin-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-uring-spin-c16 | not yet measured | not yet measured | 3056276 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-uring-spin-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-uring-spin-c256 | not yet measured | not yet measured | 3056283 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-uring-spin-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-uring-spin-c4 | not yet measured | not yet measured | 3056266 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-uring-spin-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-uring-spin-c64 | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-uring-sqpoll-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-uring-sqpoll-c1 | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-uring-sqpoll-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-uring-sqpoll-c16 | not yet measured | not yet measured | 3056288 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-uring-sqpoll-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-uring-sqpoll-c256 | not yet measured | not yet measured | 3056272 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-uring-sqpoll-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-uring-sqpoll-c4 | not yet measured | not yet measured | 3056364 | not yet measured | not yet measured | 0 |

## f1-irqcore/md-uring-sqpoll-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqcore/md-uring-sqpoll-c64 | not yet measured | not yet measured | 3056287 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-epoll-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-epoll-spin-c1 | not yet measured | not yet measured | 3056666 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-epoll-spin-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-epoll-spin-c16 | not yet measured | not yet measured | 3056993 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-epoll-spin-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-epoll-spin-c256 | not yet measured | not yet measured | 3057197 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-epoll-spin-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-epoll-spin-c4 | not yet measured | not yet measured | 3056901 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-epoll-spin-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-epoll-spin-c64 | not yet measured | not yet measured | 3057099 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-mio-c1: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-mio-c1 | not yet measured | not yet measured | 3056373 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-mio-c16: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-mio-c16 | not yet measured | not yet measured | 3056972 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-mio-c256: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-mio-c256 | not yet measured | not yet measured | 3057169 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-mio-c4: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-mio-c4 | not yet measured | not yet measured | 3056884 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-mio-c64: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-mio-c64 | not yet measured | not yet measured | 3057105 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-uring-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-uring-spin-c1 | not yet measured | not yet measured | 3056768 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-uring-spin-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-uring-spin-c16 | not yet measured | not yet measured | 3057007 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-uring-spin-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-uring-spin-c256 | not yet measured | not yet measured | 3057112 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-uring-spin-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-uring-spin-c4 | not yet measured | not yet measured | 3056878 | not yet measured | not yet measured | 0 |

## f1-irqlocal/md-uring-spin-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f1-irqlocal/md-uring-spin-c64 | not yet measured | not yet measured | 3057010 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-epoll-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-epoll-spin-c1 | not yet measured | not yet measured | 3056795 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-epoll-spin-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-epoll-spin-c16 | not yet measured | not yet measured | 3057207 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-epoll-spin-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-epoll-spin-c256 | not yet measured | not yet measured | 3057694 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-epoll-spin-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-epoll-spin-c4 | not yet measured | not yet measured | 3056902 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-epoll-spin-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-epoll-spin-c64 | not yet measured | not yet measured | 3057388 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-mio-c1: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-mio-c1 | not yet measured | not yet measured | 3056457 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-mio-c16: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-mio-c16 | not yet measured | not yet measured | 3057098 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-mio-c256: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-mio-c256 | not yet measured | not yet measured | 3057281 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-mio-c4: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-mio-c4 | not yet measured | not yet measured | 3056967 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-mio-c64: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-mio-c64 | not yet measured | not yet measured | 3057206 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-uring-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-uring-spin-c1 | not yet measured | not yet measured | 3056764 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-uring-spin-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-uring-spin-c16 | not yet measured | not yet measured | 3057194 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-uring-spin-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-uring-spin-c256 | not yet measured | not yet measured | 3057271 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-uring-spin-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-uring-spin-c4 | not yet measured | not yet measured | 3056969 | not yet measured | not yet measured | 0 |

## f2-irqlocal/md-uring-spin-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqlocal/md-uring-spin-c64 | not yet measured | not yet measured | 3057611 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-epoll-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-epoll-spin-c1 | not yet measured | not yet measured | 3056397 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-epoll-spin-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-epoll-spin-c16 | not yet measured | not yet measured | 3056388 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-epoll-spin-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-epoll-spin-c256 | not yet measured | not yet measured | 3056806 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-epoll-spin-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-epoll-spin-c4 | not yet measured | not yet measured | 3056401 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-epoll-spin-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-epoll-spin-c64 | not yet measured | not yet measured | 3056494 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-mio-c1: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-mio-c1 | not yet measured | not yet measured | 3056477 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-mio-c16: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-mio-c16 | not yet measured | not yet measured | 3056401 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-mio-c256: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-mio-c256 | not yet measured | not yet measured | 3056602 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-mio-c4: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-mio-c4 | not yet measured | not yet measured | 3056479 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-mio-c64: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-mio-c64 | not yet measured | not yet measured | 3056479 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-recv-busy-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-recv-busy-c1 | not yet measured | not yet measured | 3056390 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-recv-busy-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-recv-busy-c16 | not yet measured | not yet measured | 3056470 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-recv-busy-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-recv-busy-c256 | not yet measured | not yet measured | 3057204 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-recv-busy-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-recv-busy-c4 | not yet measured | not yet measured | 3056403 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-recv-busy-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-recv-busy-c64 | not yet measured | not yet measured | 3056401 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-recv-busy-pq-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-recv-busy-pq-c1 | not yet measured | not yet measured | 3056396 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-recv-busy-pq-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-recv-busy-pq-c16 | not yet measured | not yet measured | 3056468 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-recv-busy-pq-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-recv-busy-pq-c256 | not yet measured | not yet measured | 3056691 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-recv-busy-pq-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-recv-busy-pq-c4 | not yet measured | not yet measured | 3056457 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-recv-busy-pq-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-recv-busy-pq-c64 | not yet measured | not yet measured | 3056401 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-uring-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-uring-spin-c1 | not yet measured | not yet measured | 3056401 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-uring-spin-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-uring-spin-c16 | not yet measured | not yet measured | 3056399 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-uring-spin-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-uring-spin-c256 | not yet measured | not yet measured | 3056711 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-uring-spin-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-uring-spin-c4 | not yet measured | not yet measured | 3056401 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-uring-spin-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-uring-spin-c64 | not yet measured | not yet measured | 3056388 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-uring-sqpoll-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-uring-sqpoll-c1 | not yet measured | not yet measured | 3056494 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-uring-sqpoll-c16: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-uring-sqpoll-c16 | not yet measured | not yet measured | 3056394 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-uring-sqpoll-c256: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-uring-sqpoll-c256 | not yet measured | not yet measured | 3056502 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-uring-sqpoll-c4: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-uring-sqpoll-c4 | not yet measured | not yet measured | 3056474 | not yet measured | not yet measured | 0 |

## f2-irqcore/md-uring-sqpoll-c64: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

| run | scheduled | issued | completed | late | rejected | censored |
|---|---|---|---|---|---|---|
| f2-irqcore/md-uring-sqpoll-c64 | not yet measured | not yet measured | 3056399 | not yet measured | not yet measured | 0 |


## Sender schedule behind those percentiles

`batches` are the due times the sender had traffic for — the order connection and the
feeds together, the first message of each — and `late >100 µs` is the part that began
over 100 µs after its due time. This is not `late` above: that counts the paced order
slots filled when the next slot was already due. Traffic sent late is not the traffic
planned (a saturated sender also slows down with a slow receiver), so a row over 1% is
measuring a load the run was not offered on purpose.

| point | sent | batches | late >100 µs | latest µs | verdict |
|---|---|---|---|---|---|
| f1-irqcore/md-epoll-spin-c1 | 3056480 | 1584310 | 0 (0.00%) | 63 | ok |
| f1-irqcore/md-epoll-spin-c16 | 3056491 | 1584321 | 0 (0.00%) | 76 | ok |
| f1-irqcore/md-epoll-spin-c256 | 3056566 | 1584347 | 1866 (0.12%) | 194 | ok |
| f1-irqcore/md-epoll-spin-c4 | 3056486 | 1584316 | 0 (0.00%) | 73 | ok |
| f1-irqcore/md-epoll-spin-c64 | 3056499 | 1584329 | 0 (0.00%) | 94 | ok |
| f1-irqcore/md-mio-c1 | 3062777 | 1587569 | 1071 (0.07%) | 36059 | ok |
| f1-irqcore/md-mio-c16 | 3062690 | 1587531 | 0 (0.00%) | 77 | ok |
| f1-irqcore/md-mio-c256 | 3062791 | 1587583 | 2138 (0.13%) | 185 | ok |
| f1-irqcore/md-mio-c4 | 3062785 | 1587577 | 0 (0.00%) | 69 | ok |
| f1-irqcore/md-mio-c64 | 3062783 | 1587575 | 0 (0.00%) | 94 | ok |
| f1-irqcore/md-recv-busy-c1 | 3056474 | 1584304 | 0 (0.00%) | 71 | ok |
| f1-irqcore/md-recv-busy-c16 | 3056577 | 1584358 | 0 (0.00%) | 76 | ok |
| f1-irqcore/md-recv-busy-c256 | 3058396 | 1585295 | 1983 (0.13%) | 191 | ok |
| f1-irqcore/md-recv-busy-c4 | 3056494 | 1584324 | 0 (0.00%) | 71 | ok |
| f1-irqcore/md-recv-busy-c64 | 3061018 | 1586643 | 0 (0.00%) | 95 | ok |
| f1-irqcore/md-recv-busy-pq-c1 | 3056475 | 1584305 | 0 (0.00%) | 56 | ok |
| f1-irqcore/md-recv-busy-pq-c16 | 3056502 | 1584332 | 0 (0.00%) | 77 | ok |
| f1-irqcore/md-recv-busy-pq-c256 | 3056615 | 1584386 | 2084 (0.13%) | 194 | ok |
| f1-irqcore/md-recv-busy-pq-c4 | 3056494 | 1584324 | 0 (0.00%) | 72 | ok |
| f1-irqcore/md-recv-busy-pq-c64 | 3056566 | 1584347 | 0 (0.00%) | 94 | ok |
| f1-irqcore/md-uring-spin-c1 | 3056397 | 1584276 | 0 (0.00%) | 64 | ok |
| f1-irqcore/md-uring-spin-c16 | 3056426 | 1584285 | 0 (0.00%) | 76 | ok |
| f1-irqcore/md-uring-spin-c256 | 3056566 | 1584347 | 2160 (0.14%) | 186 | ok |
| f1-irqcore/md-uring-spin-c4 | 3056398 | 1584277 | 0 (0.00%) | 78 | ok |
| f1-irqcore/md-uring-spin-c64 | 3056499 | 1584329 | 0 (0.00%) | 93 | ok |
| f1-irqcore/md-uring-sqpoll-c1 | 3056471 | 1584301 | 0 (0.00%) | 81 | ok |
| f1-irqcore/md-uring-sqpoll-c16 | 3056491 | 1584321 | 0 (0.00%) | 78 | ok |
| f1-irqcore/md-uring-sqpoll-c256 | 3056566 | 1584347 | 2096 (0.13%) | 193 | ok |
| f1-irqcore/md-uring-sqpoll-c4 | 3056503 | 1584333 | 0 (0.00%) | 74 | ok |
| f1-irqcore/md-uring-sqpoll-c64 | 3056499 | 1584329 | 0 (0.00%) | 95 | ok |
| f1-irqcore/rt-epoll-spin-n16-r10000 | 3059340 | 1585798 | 4214 (0.27%) | 141 | ok |
| f1-irqcore/rt-epoll-spin-n4-r10000 | 3059335 | 1585793 | 13126 (0.83%) | 170 | ok |
| f1-irqcore/rt-epoll-spin-n64-r10000 | 3059405 | 1585814 | 10037 (0.63%) | 175 | ok |
| f1-irqcore/rt-mio-n16-r10000 | 3053635 | 1582837 | 10784 (0.68%) | 150 | ok |
| f1-irqcore/rt-mio-n4-r10000 | 3053637 | 1582839 | 3324 (0.21%) | 123 | ok |
| f1-irqcore/rt-mio-n64-r10000 | 3053635 | 1582837 | 12262 (0.77%) | 176 | ok |
| f1-irqcore/rt-recv-busy-n16-r10000 | 3059406 | 1585815 | 11300 (0.71%) | 150 | ok |
| f1-irqcore/rt-recv-busy-n4-r10000 | 3059353 | 1585806 | 191 (0.01%) | 114 | ok |
| f1-irqcore/rt-recv-busy-n64-r10000 | 3063660 | 1588011 | 10438 (0.66%) | 172 | ok |
| f1-irqcore/rt-recv-busy-pq-n16-r10000 | 3059405 | 1585814 | 11374 (0.72%) | 148 | ok |
| f1-irqcore/rt-recv-busy-pq-n4-r10000 | 3059302 | 1585760 | 12929 (0.82%) | 165 | ok |
| f1-irqcore/rt-recv-busy-pq-n64-r10000 | 3059429 | 1585838 | 13243 (0.84%) | 172 | ok |
| f1-irqcore/rt-uring-spin-n16-r10000 | 3059328 | 1585786 | 8738 (0.55%) | 142 | ok |
| f1-irqcore/rt-uring-spin-n4-r10000 | 3059324 | 1585782 | 394 (0.02%) | 113 | ok |
| f1-irqcore/rt-uring-spin-n64-r10000 | 3059403 | 1585812 | 13695 (0.86%) | 173 | ok |
| f1-irqcore/rt-uring-sqpoll-n16-r10000 | 3059408 | 1585817 | 12577 (0.79%) | 146 | ok |
| f1-irqcore/rt-uring-sqpoll-n4-r10000 | 3059403 | 1585812 | 8726 (0.55%) | 149 | ok |
| f1-irqcore/rt-uring-sqpoll-n64-r10000 | 3059400 | 1585809 | 11912 (0.75%) | 177 | ok |
| f1-irqlocal/md-epoll-spin-c1 | 3056807 | 1584490 | 0 (0.00%) | 58 | ok |
| f1-irqlocal/md-epoll-spin-c16 | 3057439 | 1584793 | 0 (0.00%) | 79 | ok |
| f1-irqlocal/md-epoll-spin-c256 | 3057911 | 1585055 | 2352 (0.15%) | 193 | ok |
| f1-irqlocal/md-epoll-spin-c4 | 3057200 | 1584687 | 0 (0.00%) | 72 | ok |
| f1-irqlocal/md-epoll-spin-c64 | 3057594 | 1584885 | 0 (0.00%) | 94 | ok |
| f1-irqlocal/md-mio-c1 | 3063395 | 1587893 | 1102 (0.07%) | 38025 | ok |
| f1-irqlocal/md-mio-c16 | 3064831 | 1588643 | 0 (0.00%) | 78 | ok |
| f1-irqlocal/md-mio-c256 | 3065354 | 1588921 | 2218 (0.14%) | 188 | ok |
| f1-irqlocal/md-mio-c4 | 3064388 | 1588396 | 0 (0.00%) | 75 | ok |
| f1-irqlocal/md-mio-c64 | 3065023 | 1588737 | 0 (0.00%) | 96 | ok |
| f1-irqlocal/md-uring-spin-c1 | 3056896 | 1584530 | 0 (0.00%) | 61 | ok |
| f1-irqlocal/md-uring-spin-c16 | 3057439 | 1584793 | 0 (0.00%) | 80 | ok |
| f1-irqlocal/md-uring-spin-c256 | 3057911 | 1585055 | 2101 (0.13%) | 189 | ok |
| f1-irqlocal/md-uring-spin-c4 | 3057108 | 1584644 | 0 (0.00%) | 90 | ok |
| f1-irqlocal/md-uring-spin-c64 | 3057486 | 1584826 | 0 (0.00%) | 94 | ok |
| f1-irqlocal/rt-epoll-spin-n16-r10000 | 3060658 | 1586479 | 12007 (0.76%) | 165 | ok |
| f1-irqlocal/rt-epoll-spin-n4-r10000 | 3060437 | 1586356 | 1103 (0.07%) | 121 | ok |
| f1-irqlocal/rt-epoll-spin-n64-r10000 | 3060753 | 1586525 | 14795 (0.93%) | 179 | ok |
| f1-irqlocal/rt-mio-n16-r10000 | 3054280 | 1583188 | 5168 (0.33%) | 141 | ok |
| f1-irqlocal/rt-mio-n4-r10000 | 3054080 | 1583086 | 3014 (0.19%) | 126 | ok |
| f1-irqlocal/rt-mio-n64-r10000 | 3054355 | 1583214 | 11038 (0.70%) | 170 | ok |
| f1-irqlocal/rt-uring-spin-n16-r10000 | 3060620 | 1586441 | 20077 (1.27%) | 165 | over 1%: not the planned traffic |
| f1-irqlocal/rt-uring-spin-n4-r10000 | 3060455 | 1586374 | 39464 (2.49%) | 178 | over 1%: not the planned traffic |
| f1-irqlocal/rt-uring-spin-n64-r10000 | 3060814 | 1586537 | 12285 (0.77%) | 181 | ok |
| f2-irqcore/md-epoll-spin-c1 | 3056573 | 1584354 | 0 (0.00%) | 62 | ok |
| f2-irqcore/md-epoll-spin-c16 | 3056573 | 1584354 | 0 (0.00%) | 77 | ok |
| f2-irqcore/md-epoll-spin-c256 | 3057213 | 1584700 | 2213 (0.14%) | 194 | ok |
| f2-irqcore/md-epoll-spin-c4 | 3056584 | 1584365 | 0 (0.00%) | 75 | ok |
| f2-irqcore/md-epoll-spin-c64 | 3056685 | 1584417 | 0 (0.00%) | 96 | ok |
| f2-irqcore/md-mio-c1 | 3063076 | 1587721 | 1146 (0.07%) | 38892 | ok |
| f2-irqcore/md-mio-c16 | 3062862 | 1587605 | 0 (0.00%) | 77 | ok |
| f2-irqcore/md-mio-c256 | 3063304 | 1587851 | 2333 (0.15%) | 191 | ok |
| f2-irqcore/md-mio-c4 | 3062994 | 1587688 | 0 (0.00%) | 71 | ok |
| f2-irqcore/md-mio-c64 | 3063002 | 1587696 | 0 (0.00%) | 96 | ok |
| f2-irqcore/md-recv-busy-c1 | 3056565 | 1584346 | 0 (0.00%) | 65 | ok |
| f2-irqcore/md-recv-busy-c16 | 3056697 | 1584429 | 0 (0.00%) | 82 | ok |
| f2-irqcore/md-recv-busy-c256 | 3059226 | 1585733 | 2468 (0.16%) | 194 | ok |
| f2-irqcore/md-recv-busy-c4 | 3056584 | 1584365 | 0 (0.00%) | 73 | ok |
| f2-irqcore/md-recv-busy-c64 | 3061151 | 1586727 | 0 (0.00%) | 98 | ok |
| f2-irqcore/md-recv-busy-pq-c1 | 3056573 | 1584354 | 0 (0.00%) | 62 | ok |
| f2-irqcore/md-recv-busy-pq-c16 | 3056630 | 1584386 | 0 (0.00%) | 79 | ok |
| f2-irqcore/md-recv-busy-pq-c256 | 3057204 | 1584691 | 2490 (0.16%) | 195 | ok |
| f2-irqcore/md-recv-busy-pq-c4 | 3056598 | 1584379 | 0 (0.00%) | 69 | ok |
| f2-irqcore/md-recv-busy-pq-c64 | 3056685 | 1584417 | 0 (0.00%) | 95 | ok |
| f2-irqcore/md-uring-spin-c1 | 3056574 | 1584355 | 0 (0.00%) | 69 | ok |
| f2-irqcore/md-uring-spin-c16 | 3056573 | 1584354 | 0 (0.00%) | 81 | ok |
| f2-irqcore/md-uring-spin-c256 | 3057213 | 1584700 | 2506 (0.16%) | 194 | ok |
| f2-irqcore/md-uring-spin-c4 | 3056574 | 1584355 | 0 (0.00%) | 70 | ok |
| f2-irqcore/md-uring-spin-c64 | 3056600 | 1584381 | 0 (0.00%) | 94 | ok |
| f2-irqcore/md-uring-sqpoll-c1 | 3056672 | 1584404 | 0 (0.00%) | 59 | ok |
| f2-irqcore/md-uring-sqpoll-c16 | 3056573 | 1584354 | 0 (0.00%) | 77 | ok |
| f2-irqcore/md-uring-sqpoll-c256 | 3056989 | 1584574 | 2251 (0.14%) | 195 | ok |
| f2-irqcore/md-uring-sqpoll-c4 | 3056598 | 1584379 | 0 (0.00%) | 77 | ok |
| f2-irqcore/md-uring-sqpoll-c64 | 3056615 | 1584386 | 0 (0.00%) | 96 | ok |
| f2-irqcore/rt-epoll-spin-n16-r10000 | 3059353 | 1585806 | 8282 (0.52%) | 162 | ok |
| f2-irqcore/rt-epoll-spin-n4-r10000 | 3059321 | 1585779 | 15966 (1.01%) | 165 | over 1%: not the planned traffic |
| f2-irqcore/rt-epoll-spin-n64-r10000 | 3059527 | 1585887 | 13858 (0.87%) | 174 | ok |
| f2-irqcore/rt-mio-n16-r10000 | 3053652 | 1582854 | 8237 (0.52%) | 139 | ok |
| f2-irqcore/rt-mio-n4-r10000 | 3053660 | 1582862 | 10913 (0.69%) | 163 | ok |
| f2-irqcore/rt-mio-n64-r10000 | 3053663 | 1582865 | 12490 (0.79%) | 182 | ok |
| f2-irqcore/rt-recv-busy-n16-r10000 | 3059538 | 1585898 | 10977 (0.69%) | 144 | ok |
| f2-irqcore/rt-recv-busy-n4-r10000 | 3059403 | 1585812 | 12005 (0.76%) | 153 | ok |
| f2-irqcore/rt-recv-busy-n64-r10000 | 3063782 | 1588084 | 10675 (0.67%) | 174 | ok |
| f2-irqcore/rt-recv-busy-pq-n16-r10000 | 3059417 | 1585826 | 7344 (0.46%) | 137 | ok |
| f2-irqcore/rt-recv-busy-pq-n4-r10000 | 3059332 | 1585790 | 12670 (0.80%) | 161 | ok |
| f2-irqcore/rt-recv-busy-pq-n64-r10000 | 3059448 | 1585856 | 13283 (0.84%) | 176 | ok |
| f2-irqcore/rt-uring-spin-n16-r10000 | 3059412 | 1585821 | 13861 (0.87%) | 149 | ok |
| f2-irqcore/rt-uring-spin-n4-r10000 | 3059405 | 1585814 | 25728 (1.62%) | 160 | over 1%: not the planned traffic |
| f2-irqcore/rt-uring-spin-n64-r10000 | 3059536 | 1585896 | 19334 (1.22%) | 179 | over 1%: not the planned traffic |
| f2-irqcore/rt-uring-sqpoll-n16-r10000 | 3059409 | 1585818 | 6643 (0.42%) | 136 | ok |
| f2-irqcore/rt-uring-sqpoll-n4-r10000 | 3059339 | 1585797 | 1669 (0.11%) | 122 | ok |
| f2-irqcore/rt-uring-sqpoll-n64-r10000 | 3059425 | 1585834 | 13427 (0.85%) | 181 | ok |
| f2-irqlocal/md-epoll-spin-c1 | 3056967 | 1584552 | 0 (0.00%) | 74 | ok |
| f2-irqlocal/md-epoll-spin-c16 | 3057692 | 1584934 | 2 (0.00%) | 121 | ok |
| f2-irqlocal/md-epoll-spin-c256 | 3058396 | 1585295 | 2411 (0.15%) | 191 | ok |
| f2-irqlocal/md-epoll-spin-c4 | 3057211 | 1584698 | 1 (0.00%) | 111 | ok |
| f2-irqlocal/md-epoll-spin-c64 | 3057916 | 1585060 | 8 (0.00%) | 118 | ok |
| f2-irqlocal/md-mio-c1 | 3063503 | 1587952 | 1182 (0.07%) | 40180 | ok |
| f2-irqlocal/md-mio-c16 | 3064929 | 1588692 | 0 (0.00%) | 77 | ok |
| f2-irqlocal/md-mio-c256 | 3065482 | 1588975 | 2333 (0.15%) | 197 | ok |
| f2-irqlocal/md-mio-c4 | 3064506 | 1588465 | 0 (0.00%) | 72 | ok |
| f2-irqlocal/md-mio-c64 | 3065366 | 1588933 | 0 (0.00%) | 95 | ok |
| f2-irqlocal/md-uring-spin-c1 | 3056884 | 1584518 | 0 (0.00%) | 60 | ok |
| f2-irqlocal/md-uring-spin-c16 | 3057598 | 1584889 | 1 (0.00%) | 118 | ok |
| f2-irqlocal/md-uring-spin-c256 | 3057916 | 1585060 | 2097 (0.13%) | 188 | ok |
| f2-irqlocal/md-uring-spin-c4 | 3057272 | 1584710 | 0 (0.00%) | 77 | ok |
| f2-irqlocal/md-uring-spin-c64 | 3058127 | 1585158 | 4 (0.00%) | 161 | ok |
| f2-irqlocal/rt-epoll-spin-n16-r10000 | 3060639 | 1586460 | 6300 (0.40%) | 143 | ok |
| f2-irqlocal/rt-epoll-spin-n4-r10000 | 3060264 | 1586281 | 6 (0.00%) | 106 | ok |
| f2-irqlocal/rt-epoll-spin-n64-r10000 | 3060814 | 1586537 | 11297 (0.71%) | 166 | ok |
| f2-irqlocal/rt-mio-n16-r10000 | 3054296 | 1583204 | 13814 (0.87%) | 151 | ok |
| f2-irqlocal/rt-mio-n4-r10000 | 3054090 | 1583096 | 21888 (1.38%) | 156 | over 1%: not the planned traffic |
| f2-irqlocal/rt-mio-n64-r10000 | 3054384 | 1583243 | 14098 (0.89%) | 180 | ok |
| f2-irqlocal/rt-uring-spin-n16-r10000 | 3060633 | 1586454 | 4217 (0.27%) | 134 | ok |
| f2-irqlocal/rt-uring-spin-n4-r10000 | 3060350 | 1586318 | 981 (0.06%) | 125 | ok |
| f2-irqlocal/rt-uring-spin-n64-r10000 | 3060748 | 1586520 | 12668 (0.80%) | 185 | ok |

6 of 144 points over 1% of batches late: `f1-irqlocal/rt-uring-spin-n16-r10000`, `f1-irqlocal/rt-uring-spin-n4-r10000`, `f2-irqcore/rt-epoll-spin-n4-r10000`, `f2-irqcore/rt-uring-spin-n4-r10000`, `f2-irqcore/rt-uring-spin-n64-r10000`, `f2-irqlocal/rt-mio-n4-r10000`.

`latest µs` is the worst batch of the whole run. `f1-irqcore/md-mio-c1` at 36,059, `f1-irqlocal/md-mio-c1` at 38,025, `f2-irqcore/md-mio-c1` at 38,892, `f2-irqlocal/md-mio-c1` at 40,180 sit over 100× the median of the column (123). Each is investigated in `io/benches/BENCHMARK.md`; no latency sample saw them, so the published figures stand. Do not rank a row on `latest µs`.

