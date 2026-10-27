counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=frag
samples=20000
text_check=None

| size B | interest | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg |
|---|---|---|---|---|---|---|---|
| 100 | keep | 64 | 31.9 | 39.0 | 3702.3 | n/a | 100 |
| 100 | fragments | 64 | 26.6 | 31.2 | 2345.5 | n/a | 0 |
| 100 | stream | 64 | 29.2 | 34.5 | 5430.3 | n/a | 0 |
| 300 | keep | 64 | 26.0 | 29.9 | 519.1 | n/a | 300 |
| 300 | fragments | 64 | 26.6 | 30.5 | 713.5 | n/a | 0 |
| 300 | stream | 64 | 29.2 | 33.2 | 673.9 | n/a | 0 |
| 4096 | keep | 64 | 89.8 | 320.9 | 26444.7 | n/a | 4096 |
| 4096 | fragments | 64 | 28.6 | 87.2 | 8614.3 | n/a | 0 |
| 4096 | stream | 64 | 32.5 | 229.1 | 11827.1 | n/a | 0 |
| 16384 | keep | 16 | 317.7 | 421.9 | 15423.9 | n/a | 16384 |
| 16384 | fragments | 16 | 31.2 | 41.6 | 6150.3 | n/a | 0 |
| 16384 | stream | 16 | 33.8 | 44.2 | 3395.1 | n/a | 0 |
