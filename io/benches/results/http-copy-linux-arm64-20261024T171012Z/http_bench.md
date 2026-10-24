counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
samples=20000

| row | path | K | ns/response p50 | p99 | max | max sample preempted (Linux) | de-chunk copy B/resp | span copy B/resp | compaction B/resp | head parses/resp |
|---|---|---|---|---|---|---|---|---|---|---|
| chunked 4096 | old | 32 | 1343.9 | 9343.9 | 117452.7 | no | 4096 | 0 | 0 | 1.00 |
| chunked 4096 | new | 32 | 519.5 | 4553.5 | 217702.3 | no | 0 | 0 | 0 | 1.00 |
| chunked 65536 | old | 8 | 58111.9 | 1170636.7 | 8165785.5 | yes | 212992 | 49364 | 0 | 5.00 |
| chunked 65536 | new | 8 | 15219.1 | 485785.5 | 3496345.5 | no | 65615 | 0 | 0 | 1.00 |
| chunked 1048576 | old | 1 | 8703180.7 | 44826623.9 | 165884723.1 | yes | 34865152 | 1034324 | 0 | 65.00 |
| chunked 1048576 | new | 1 | 231423.9 | 3565158.3 | 47369420.7 | yes | 1048655 | 0 | 0 | 1.00 |
| split 4096 | old | 32 | 1847.9 | 2884.7 | 98457.5 | no | 0 | 4096 | 0 | 2.00 |
| split 4096 | new | 32 | 1833.5 | 2742.3 | 170700.7 | no | 0 | 4096 | 0 | 1.00 |

| row | path | K | ns/request p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|
| build 3 | old | 64 | 102.2 | 125.0 | 862.3 | no |
| build 3 | new | 64 | 57.2 | 64.4 | 921.5 | no |
| build 20 | old | 64 | 527.9 | 704.7 | 2004.7 | no |
| build 20 | new | 64 | 301.5 | 421.5 | 1605.5 | no |
