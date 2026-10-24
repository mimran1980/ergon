counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=tx
samples=20000

| size B | stage | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|
| 100 | build | old | 64 | 14.3 | 16.9 | 324.3 | n/a |
| 100 | build | new | 64 | 13.6 | 15.6 | 199.2 | n/a |
| 100 | sealed | old | 64 | 130.8 | 213.5 | 18009.5 | n/a |
| 100 | sealed | new | 64 | 131.5 | 186.1 | 10988.7 | n/a |
| 300 | build | old | 64 | 23.4 | 26.6 | 1967.9 | n/a |
| 300 | build | new | 64 | 20.1 | 24.0 | 1157.5 | n/a |
| 300 | sealed | old | 64 | 175.7 | 197.9 | 961.5 | n/a |
| 300 | sealed | new | 64 | 173.1 | 190.7 | 630.3 | n/a |
| 4096 | build | old | 64 | 222.1 | 357.5 | 3302.3 | n/a |
| 4096 | build | new | 64 | 180.3 | 250.1 | 4502.3 | n/a |
| 4096 | sealed | old | 64 | 1070.3 | 1300.7 | 5580.7 | n/a |
| 4096 | sealed | new | 64 | 1029.5 | 1246.3 | 13375.9 | n/a |
| 16384 | build | old | 16 | 671.9 | 783.9 | 2503.9 | n/a |
| 16384 | build | new | 16 | 674.7 | 783.9 | 5295.9 | n/a |
| 16384 | sealed | old | 16 | 3916.7 | 5263.9 | 105369.5 | n/a |
| 16384 | sealed | new | 16 | 3919.9 | 5331.1 | 410623.9 | n/a |
