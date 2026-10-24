counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=span
samples=20000
text_check=None

| size B | path | K | records | ns/message p50 | p99 | max | max sample preempted (Linux) | span copy B/msg |
|---|---|---|---|---|---|---|---|---|
| 100 | old | 2520 | 16 | 26.2 | 99.1 | 2271.9 | no | 95 |
| 100 | new | 2520 | 16 | 6.1 | 31.8 | 992.7 | no | 0 |
| 300 | old | 862 | 16 | 67.1 | 237.1 | 6252.7 | no | 285 |
| 300 | new | 862 | 16 | 10.9 | 69.8 | 7443.1 | no | 3 |
| 4096 | old | 63 | 16 | 828.7 | 2083.1 | 39372.7 | no | 3840 |
| 4096 | new | 63 | 16 | 203.0 | 537.9 | 21619.1 | no | 946 |
| 16384 | old | 15 | 16 | 3279.9 | 14105.5 | 1636761.5 | no | 15296 |
| 16384 | new | 15 | 16 | 3131.1 | 12959.9 | 853606.3 | no | 15271 |
