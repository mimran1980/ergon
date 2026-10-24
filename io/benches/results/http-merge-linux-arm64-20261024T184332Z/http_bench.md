counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
samples=20000

| row | path | K | ns/response p50 | p99 | max | max sample preempted (Linux) | de-chunk copy B/resp | span copy B/resp | compaction B/resp | head parses/resp |
|---|---|---|---|---|---|---|---|---|---|---|
| chunked 4096 | old | 32 | 1297.5 | 1873.5 | 6051.1 | no | 4096 | 0 | 0 | 1.00 |
| chunked 4096 | new | 32 | 500.3 | 764.3 | 3111.9 | no | 0 | 0 | 0 | 1.00 |
| chunked 65536 | old | 8 | 55807.9 | 70911.9 | 1472921.5 | no | 212992 | 49364 | 0 | 5.00 |
| chunked 65536 | new | 8 | 14591.9 | 20300.7 | 134041.5 | no | 65615 | 0 | 0 | 1.00 |
| chunked 1048576 | old | 1 | 7182745.5 | 8434483.1 | 95315558.3 | yes | 34865152 | 1034324 | 0 | 65.00 |
| chunked 1048576 | new | 1 | 212377.5 | 252518.3 | 2478899.1 | no | 1048655 | 0 | 0 | 1.00 |
| split 4096 | old | 32 | 1796.7 | 2358.3 | 15615.9 | no | 0 | 4096 | 0 | 2.00 |
| split 4096 | new | 32 | 962.3 | 1304.7 | 7123.1 | no | 0 | 73 | 0 | 1.00 |

| row | path | K | ns/request p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|
| build 3 | old | 64 | 98.3 | 119.7 | 1404.7 | no |
| build 3 | new | 64 | 54.6 | 68.3 | 923.1 | no |
| build 20 | old | 64 | 507.9 | 769.5 | 10956.7 | no |
| build 20 | new | 64 | 291.7 | 417.5 | 1854.3 | no |
