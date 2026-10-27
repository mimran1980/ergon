counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=rx
samples=20000
text_check=Utf8

| size B | shape | decision | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg | span copy B/msg |
|---|---|---|---|---|---|---|---|---|---|---|
| 100 | whole | kept | old | 64 | 9.1 | 11.0 | 333.3 | n/a | 0 | 0 |
| 100 | whole | kept | new | 64 | 9.1 | 11.0 | 381.5 | n/a | 0 | 0 |
| 100 | whole | skipped | old | 64 | 8.4 | 9.1 | 201.1 | n/a | 0 | 0 |
| 100 | whole | skipped | new | 64 | 3.9 | 4.5 | 103.5 | n/a | 0 | 0 |
| 100 | frag4 | kept | old | 64 | 34.5 | 37.1 | 1021.5 | n/a | 100 | 0 |
| 100 | frag4 | kept | new | 64 | 35.8 | 38.4 | 559.9 | n/a | 0 | 0 |
| 100 | frag4 | skipped | old | 64 | 32.5 | 36.4 | 292.9 | n/a | 100 | 0 |
| 100 | frag4 | skipped | new | 64 | 15.6 | 17.5 | 399.1 | n/a | 0 | 0 |
| 100 | split | kept | old | 64 | 23.4 | 26.0 | 785.9 | n/a | 0 | 51 |
| 100 | split | kept | new | 64 | 23.4 | 26.0 | 288.5 | n/a | 0 | 51 |
| 100 | split | skipped | old | 64 | 22.7 | 24.7 | 655.1 | n/a | 0 | 51 |
| 100 | split | skipped | new | 64 | 18.2 | 20.1 | 406.9 | n/a | 0 | 51 |
| 300 | whole | kept | old | 64 | 10.4 | 11.7 | 170.5 | n/a | 0 | 0 |
| 300 | whole | kept | new | 64 | 10.4 | 11.7 | 269.5 | n/a | 0 | 0 |
| 300 | whole | skipped | old | 64 | 9.7 | 11.7 | 311.1 | n/a | 0 | 0 |
| 300 | whole | skipped | new | 64 | 4.5 | 5.2 | 194.0 | n/a | 0 | 0 |
| 300 | frag4 | kept | old | 64 | 29.9 | 33.8 | 987.1 | n/a | 300 | 0 |
| 300 | frag4 | kept | new | 64 | 36.4 | 40.3 | 585.5 | n/a | 0 | 0 |
| 300 | frag4 | skipped | old | 64 | 29.2 | 32.5 | 445.9 | n/a | 300 | 0 |
| 300 | frag4 | skipped | new | 64 | 16.2 | 18.2 | 506.7 | n/a | 0 | 0 |
| 300 | split | kept | old | 64 | 26.0 | 29.2 | 350.3 | n/a | 0 | 152 |
| 300 | split | kept | new | 64 | 26.0 | 29.2 | 337.9 | n/a | 0 | 152 |
| 300 | split | skipped | old | 64 | 23.4 | 26.6 | 425.9 | n/a | 0 | 152 |
| 300 | split | skipped | new | 64 | 16.9 | 19.5 | 211.5 | n/a | 0 | 152 |
| 4096 | whole | kept | old | 64 | 55.9 | 60.5 | 489.5 | n/a | 0 | 0 |
| 4096 | whole | kept | new | 64 | 55.9 | 60.5 | 494.3 | n/a | 0 | 0 |
| 4096 | whole | skipped | old | 64 | 54.6 | 60.5 | 549.5 | n/a | 0 | 0 |
| 4096 | whole | skipped | new | 64 | 6.5 | 8.4 | 202.4 | n/a | 0 | 0 |
| 4096 | frag4 | kept | old | 64 | 117.8 | 131.5 | 3641.5 | n/a | 4096 | 0 |
| 4096 | frag4 | kept | new | 64 | 87.8 | 100.2 | 2071.9 | n/a | 0 | 0 |
| 4096 | frag4 | skipped | old | 64 | 115.2 | 134.7 | 2659.1 | n/a | 4096 | 0 |
| 4096 | frag4 | skipped | new | 64 | 23.4 | 37.7 | 1016.7 | n/a | 0 | 0 |
| 4096 | split | kept | old | 64 | 102.2 | 115.2 | 13036.7 | n/a | 0 | 2050 |
| 4096 | split | kept | new | 64 | 102.2 | 117.8 | 1910.3 | n/a | 0 | 2050 |
| 4096 | split | skipped | old | 64 | 101.5 | 109.3 | 723.9 | n/a | 0 | 2050 |
| 4096 | split | skipped | new | 64 | 59.8 | 65.7 | 477.5 | n/a | 0 | 2050 |
| 65536 | whole | kept | old | 8 | 812.7 | 916.7 | 7679.9 | n/a | 0 | 0 |
| 65536 | whole | kept | new | 8 | 812.7 | 932.7 | 5964.7 | n/a | 0 | 0 |
| 65536 | whole | skipped | old | 8 | 812.7 | 1073.5 | 123187.1 | n/a | 0 | 0 |
| 65536 | whole | skipped | new | 8 | 15.6 | 26.0 | 203.1 | n/a | 0 | 0 |
| 65536 | frag4 | kept | old | 8 | 1937.5 | 2255.9 | 10502.3 | n/a | 65536 | 0 |
| 65536 | frag4 | kept | new | 8 | 864.7 | 1213.5 | 7129.5 | n/a | 0 | 0 |
| 65536 | frag4 | skipped | old | 8 | 1964.7 | 2308.7 | 23027.1 | n/a | 65536 | 0 |
| 65536 | frag4 | skipped | new | 8 | 36.4 | 88.5 | 1171.9 | n/a | 0 | 0 |
| 65536 | split | kept | old | 8 | 1985.5 | 2219.1 | 27622.3 | n/a | 0 | 63494 |
| 65536 | split | kept | new | 8 | 1990.3 | 2219.1 | 18329.5 | n/a | 0 | 63494 |
| 65536 | split | skipped | old | 8 | 1990.3 | 2167.9 | 30284.7 | n/a | 0 | 63494 |
| 65536 | split | skipped | new | 8 | 1203.1 | 1312.7 | 28211.1 | n/a | 0 | 63494 |
