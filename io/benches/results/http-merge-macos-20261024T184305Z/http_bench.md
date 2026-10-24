counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
samples=20000

| row | path | K | ns/response p50 | p99 | max | max sample preempted (Linux) | de-chunk copy B/resp | span copy B/resp | compaction B/resp | head parses/resp |
|---|---|---|---|---|---|---|---|---|---|---|
| chunked 4096 | old | 32 | 173.1 | 201.8 | 1410.3 | n/a | 4096 | 0 | 0 | 1.00 |
| chunked 4096 | new | 32 | 122.3 | 138.0 | 1499.1 | n/a | 0 | 0 | 0 | 1.00 |
| chunked 65536 | old | 8 | 4380.7 | 5375.9 | 22233.5 | n/a | 212992 | 49364 | 0 | 5.00 |
| chunked 65536 | new | 8 | 1615.1 | 1895.9 | 6847.9 | n/a | 65615 | 0 | 0 | 1.00 |
| chunked 1048576 | old | 1 | 596377.5 | 705740.7 | 2695167.9 | n/a | 34865152 | 1034324 | 0 | 65.00 |
| chunked 1048576 | new | 1 | 26009.5 | 30310.3 | 123084.7 | n/a | 1048655 | 0 | 0 | 1.00 |
| split 4096 | old | 32 | 295.5 | 329.5 | 1665.5 | n/a | 0 | 4096 | 0 | 2.00 |
| split 4096 | new | 32 | 173.1 | 201.8 | 783.9 | n/a | 0 | 73 | 0 | 1.00 |

| row | path | K | ns/request p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|
| build 3 | old | 64 | 113.2 | 125.0 | 439.5 | n/a |
| build 3 | new | 64 | 72.2 | 80.7 | 403.7 | n/a |
| build 20 | old | 64 | 526.7 | 694.3 | 2027.1 | n/a |
| build 20 | new | 64 | 326.1 | 425.1 | 1591.9 | n/a |
