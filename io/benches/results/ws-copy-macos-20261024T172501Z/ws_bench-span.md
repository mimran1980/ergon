counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=span
samples=20000
text_check=None

| size B | path | K | records | ns/message p50 | p99 | max | max sample preempted (Linux) | span copy B/msg |
|---|---|---|---|---|---|---|---|---|
| 100 | old | 2520 | 16 | 7.4 | 16.1 | 153.4 | n/a | 95 |
| 100 | new | 2520 | 16 | 5.6 | 8.0 | 143.6 | n/a | 0 |
| 300 | old | 862 | 16 | 11.6 | 12.8 | 204.2 | n/a | 285 |
| 300 | new | 862 | 16 | 9.7 | 10.8 | 71.9 | n/a | 3 |
| 4096 | old | 63 | 16 | 82.0 | 91.9 | 2187.1 | n/a | 3840 |
| 4096 | new | 63 | 16 | 31.0 | 35.7 | 463.9 | n/a | 946 |
| 16384 | old | 15 | 16 | 313.9 | 350.1 | 9894.3 | n/a | 15296 |
| 16384 | new | 15 | 16 | 308.3 | 344.5 | 23129.5 | n/a | 15271 |
