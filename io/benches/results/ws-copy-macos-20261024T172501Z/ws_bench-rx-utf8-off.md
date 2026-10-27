counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=rx
samples=20000
text_check=None

| size B | shape | decision | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg | span copy B/msg |
|---|---|---|---|---|---|---|---|---|---|---|
| 100 | whole | kept | old | 64 | 5.8 | 15.6 | 1678.3 | n/a | 0 | 0 |
| 100 | whole | kept | new | 64 | 5.8 | 15.6 | 1042.3 | n/a | 0 | 0 |
| 100 | whole | skipped | old | 64 | 5.2 | 5.8 | 1612.7 | n/a | 0 | 0 |
| 100 | whole | skipped | new | 64 | 3.9 | 4.5 | 263.7 | n/a | 0 | 0 |
| 100 | frag4 | kept | old | 64 | 30.5 | 34.5 | 649.9 | n/a | 100 | 0 |
| 100 | frag4 | kept | new | 64 | 24.0 | 27.3 | 435.1 | n/a | 0 | 0 |
| 100 | frag4 | skipped | old | 64 | 29.2 | 33.2 | 617.9 | n/a | 100 | 0 |
| 100 | frag4 | skipped | new | 64 | 15.6 | 18.2 | 498.3 | n/a | 0 | 0 |
| 100 | split | kept | old | 64 | 19.5 | 22.1 | 220.1 | n/a | 0 | 51 |
| 100 | split | kept | new | 64 | 19.5 | 22.1 | 465.5 | n/a | 0 | 51 |
| 100 | split | skipped | old | 64 | 18.8 | 22.1 | 340.5 | n/a | 0 | 51 |
| 100 | split | skipped | new | 64 | 18.2 | 20.8 | 353.5 | n/a | 0 | 51 |
| 300 | whole | kept | old | 64 | 5.8 | 7.1 | 128.9 | n/a | 0 | 0 |
| 300 | whole | kept | new | 64 | 5.8 | 7.1 | 172.5 | n/a | 0 | 0 |
| 300 | whole | skipped | old | 64 | 5.2 | 6.5 | 350.9 | n/a | 0 | 0 |
| 300 | whole | skipped | new | 64 | 4.5 | 5.8 | 266.9 | n/a | 0 | 0 |
| 300 | frag4 | kept | old | 64 | 24.7 | 28.6 | 365.9 | n/a | 300 | 0 |
| 300 | frag4 | kept | new | 64 | 24.0 | 27.3 | 369.1 | n/a | 0 | 0 |
| 300 | frag4 | skipped | old | 64 | 24.0 | 27.9 | 358.1 | n/a | 300 | 0 |
| 300 | frag4 | skipped | new | 64 | 15.6 | 18.2 | 364.5 | n/a | 0 | 0 |
| 300 | split | kept | old | 64 | 18.2 | 20.8 | 264.9 | n/a | 0 | 152 |
| 300 | split | kept | new | 64 | 18.2 | 20.8 | 352.9 | n/a | 0 | 152 |
| 300 | split | skipped | old | 64 | 17.5 | 20.8 | 341.7 | n/a | 0 | 152 |
| 300 | split | skipped | new | 64 | 16.9 | 19.5 | 640.7 | n/a | 0 | 152 |
| 4096 | whole | kept | old | 64 | 7.8 | 10.4 | 42.9 | n/a | 0 | 0 |
| 4096 | whole | kept | new | 64 | 7.8 | 10.4 | 53.3 | n/a | 0 | 0 |
| 4096 | whole | skipped | old | 64 | 7.1 | 8.4 | 97.0 | n/a | 0 | 0 |
| 4096 | whole | skipped | new | 64 | 7.1 | 7.8 | 30.5 | n/a | 0 | 0 |
| 4096 | frag4 | kept | old | 64 | 80.0 | 89.8 | 907.1 | n/a | 4096 | 0 |
| 4096 | frag4 | kept | new | 64 | 27.3 | 32.5 | 449.5 | n/a | 0 | 0 |
| 4096 | frag4 | skipped | old | 64 | 81.3 | 98.3 | 67942.3 | n/a | 4096 | 0 |
| 4096 | frag4 | skipped | new | 64 | 24.0 | 41.6 | 3743.9 | n/a | 0 | 0 |
| 4096 | split | kept | old | 64 | 61.1 | 67.7 | 1860.7 | n/a | 0 | 2050 |
| 4096 | split | kept | new | 64 | 61.1 | 67.7 | 3337.5 | n/a | 0 | 2050 |
| 4096 | split | skipped | old | 64 | 60.5 | 66.4 | 4169.5 | n/a | 0 | 2050 |
| 4096 | split | skipped | new | 64 | 59.2 | 65.1 | 58828.7 | n/a | 0 | 2050 |
| 65536 | whole | kept | old | 8 | 15.6 | 26.0 | 479.1 | n/a | 0 | 0 |
| 65536 | whole | kept | new | 8 | 15.6 | 26.0 | 3315.1 | n/a | 0 | 0 |
| 65536 | whole | skipped | old | 8 | 15.6 | 26.0 | 3897.5 | n/a | 0 | 0 |
| 65536 | whole | skipped | new | 8 | 15.6 | 26.0 | 3135.9 | n/a | 0 | 0 |
| 65536 | frag4 | kept | old | 8 | 1146.3 | 1895.9 | 157798.3 | n/a | 65536 | 0 |
| 65536 | frag4 | kept | new | 8 | 41.6 | 93.7 | 2235.1 | n/a | 0 | 0 |
| 65536 | frag4 | skipped | old | 8 | 1198.3 | 1881.5 | 18457.5 | n/a | 65536 | 0 |
| 65536 | frag4 | skipped | new | 8 | 36.4 | 78.1 | 937.5 | n/a | 0 | 0 |
| 65536 | split | kept | old | 8 | 1223.9 | 1328.7 | 65945.5 | n/a | 0 | 63494 |
| 65536 | split | kept | new | 8 | 1223.9 | 1328.7 | 229785.5 | n/a | 0 | 63494 |
| 65536 | split | skipped | old | 8 | 1223.9 | 1318.3 | 16819.1 | n/a | 0 | 63494 |
| 65536 | split | skipped | new | 8 | 1223.9 | 1318.3 | 18431.9 | n/a | 0 | 63494 |
