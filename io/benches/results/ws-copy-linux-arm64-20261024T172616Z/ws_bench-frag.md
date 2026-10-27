counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=frag
samples=20000
text_check=None

| size B | interest | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg |
|---|---|---|---|---|---|---|---|
| 100 | keep | 64 | 24.0 | 33.2 | 452.7 | no | 100 |
| 100 | fragments | 64 | 25.3 | 35.8 | 505.9 | no | 0 |
| 100 | stream | 64 | 28.6 | 40.3 | 353.5 | no | 0 |
| 300 | keep | 64 | 76.1 | 115.2 | 939.9 | no | 300 |
| 300 | fragments | 64 | 25.3 | 37.1 | 417.9 | no | 0 |
| 300 | stream | 64 | 28.6 | 42.9 | 751.5 | no | 0 |
| 4096 | keep | 64 | 861.5 | 1769.5 | 15897.5 | no | 4096 |
| 4096 | fragments | 64 | 31.2 | 147.7 | 5151.9 | no | 0 |
| 4096 | stream | 64 | 35.1 | 179.6 | 51199.9 | no | 0 |
| 16384 | keep | 16 | 3430.3 | 9471.9 | 339967.9 | no | 16384 |
| 16384 | fragments | 16 | 96.3 | 403.7 | 139673.5 | yes | 0 |
| 16384 | stream | 16 | 101.5 | 364.5 | 16025.5 | no | 0 |
