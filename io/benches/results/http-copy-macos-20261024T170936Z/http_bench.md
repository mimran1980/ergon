counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
samples=20000

| row | path | K | ns/response p50 | p99 | max | max sample preempted (Linux) | de-chunk copy B/resp | span copy B/resp | compaction B/resp | head parses/resp |
|---|---|---|---|---|---|---|---|---|---|---|
| chunked 4096 | old | 32 | 177.0 | 205.7 | 20172.7 | n/a | 4096 | 0 | 0 | 1.00 |
| chunked 4096 | new | 32 | 130.2 | 165.3 | 34252.7 | n/a | 0 | 0 | 0 | 1.00 |
| chunked 65536 | old | 8 | 4700.7 | 31231.9 | 2202009.5 | n/a | 212992 | 49364 | 0 | 5.00 |
| chunked 65536 | new | 8 | 1719.9 | 7948.7 | 9889382.3 | n/a | 65615 | 0 | 0 | 1.00 |
| chunked 1048576 | old | 1 | 670105.5 | 3065446.3 | 217474662.3 | n/a | 34865152 | 1034324 | 0 | 65.00 |
| chunked 1048576 | new | 1 | 28006.3 | 91033.5 | 35835084.7 | n/a | 1048655 | 0 | 0 | 1.00 |
| split 4096 | old | 32 | 287.7 | 328.1 | 45695.9 | n/a | 0 | 4096 | 0 | 2.00 |
| split 4096 | new | 32 | 234.3 | 285.1 | 147967.9 | n/a | 0 | 4096 | 0 | 1.00 |

| row | path | K | ns/request p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|
| build 3 | old | 64 | 114.5 | 123.6 | 727.9 | n/a |
| build 3 | new | 64 | 72.9 | 79.4 | 832.7 | n/a |
| build 20 | old | 64 | 545.5 | 737.1 | 17881.5 | n/a |
| build 20 | new | 64 | 330.7 | 440.3 | 16051.1 | n/a |
