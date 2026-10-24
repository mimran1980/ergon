counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
samples=20000
validate_utf8=false

| size B | shape | decision | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg | span copy B/msg |
|---|---|---|---|---|---|---|---|---|---|---|
| 100 | whole | kept | old | 64 | 5.2 | 13.0 | 2684.7 | n/a | 0 | 0 |
| 100 | whole | kept | new | 64 | 5.2 | 13.6 | 6662.3 | n/a | 0 | 0 |
| 100 | whole | skipped | old | 64 | 4.5 | 9.7 | 17049.5 | n/a | 0 | 0 |
| 100 | whole | skipped | new | 64 | 3.9 | 9.1 | 10950.3 | n/a | 0 | 0 |
| 100 | frag4 | kept | old | 64 | 29.2 | 70.3 | 8166.3 | n/a | 100 | 0 |
| 100 | frag4 | kept | new | 64 | 25.3 | 69.0 | 3238.3 | n/a | 0 | 0 |
| 100 | frag4 | skipped | old | 64 | 28.6 | 69.0 | 85094.3 | n/a | 100 | 0 |
| 100 | frag4 | skipped | new | 64 | 15.6 | 39.7 | 49919.9 | n/a | 0 | 0 |
| 100 | split | kept | old | 64 | 14.3 | 46.2 | 32358.3 | n/a | 0 | 101 |
| 100 | split | kept | new | 64 | 14.3 | 44.2 | 105062.3 | n/a | 0 | 101 |
| 100 | split | skipped | old | 64 | 13.0 | 39.7 | 76390.3 | n/a | 0 | 101 |
| 100 | split | skipped | new | 64 | 13.0 | 39.0 | 63948.7 | n/a | 0 | 101 |
| 300 | whole | kept | old | 64 | 5.2 | 13.6 | 196300.7 | n/a | 0 | 0 |
| 300 | whole | kept | new | 64 | 5.2 | 13.6 | 45209.5 | n/a | 0 | 0 |
| 300 | whole | skipped | old | 64 | 4.5 | 10.4 | 29823.9 | n/a | 0 | 0 |
| 300 | whole | skipped | new | 64 | 3.9 | 9.7 | 26700.7 | n/a | 0 | 0 |
| 300 | frag4 | kept | old | 64 | 24.0 | 70.9 | 22758.3 | n/a | 300 | 0 |
| 300 | frag4 | kept | new | 64 | 25.3 | 69.0 | 14246.3 | n/a | 0 | 0 |
| 300 | frag4 | skipped | old | 64 | 23.4 | 69.0 | 9254.3 | n/a | 300 | 0 |
| 300 | frag4 | skipped | new | 64 | 15.6 | 39.0 | 28902.3 | n/a | 0 | 0 |
| 300 | split | kept | old | 64 | 18.2 | 46.8 | 2919.9 | n/a | 0 | 302 |
| 300 | split | kept | new | 64 | 18.2 | 47.5 | 2382.3 | n/a | 0 | 302 |
| 300 | split | skipped | old | 64 | 15.6 | 46.2 | 2235.1 | n/a | 0 | 302 |
| 300 | split | skipped | new | 64 | 14.9 | 44.2 | 3727.9 | n/a | 0 | 302 |
| 4096 | whole | kept | old | 64 | 8.4 | 10.4 | 3660.7 | n/a | 0 | 0 |
| 4096 | whole | kept | new | 64 | 7.1 | 9.7 | 736.3 | n/a | 0 | 0 |
| 4096 | whole | skipped | old | 64 | 5.8 | 8.4 | 1815.9 | n/a | 0 | 0 |
| 4096 | whole | skipped | new | 64 | 5.8 | 8.4 | 1611.1 | n/a | 0 | 0 |
| 4096 | frag4 | kept | old | 64 | 82.0 | 144.5 | 6467.1 | n/a | 4096 | 0 |
| 4096 | frag4 | kept | new | 64 | 27.9 | 45.5 | 3155.1 | n/a | 0 | 0 |
| 4096 | frag4 | skipped | old | 64 | 80.7 | 219.5 | 19353.5 | n/a | 4096 | 0 |
| 4096 | frag4 | skipped | new | 64 | 20.1 | 69.0 | 9753.5 | n/a | 0 | 0 |
| 4096 | split | kept | old | 64 | 91.1 | 220.1 | 7052.7 | n/a | 0 | 4068 |
| 4096 | split | kept | new | 64 | 91.1 | 219.5 | 24383.9 | n/a | 0 | 4068 |
| 4096 | split | skipped | old | 64 | 89.8 | 257.1 | 30259.1 | n/a | 0 | 4068 |
| 4096 | split | skipped | new | 64 | 89.1 | 255.3 | 32127.9 | n/a | 0 | 4068 |
| 65536 | whole | kept | old | 8 | 15.6 | 52.0 | 271155.1 | n/a | 0 | 0 |
| 65536 | whole | kept | new | 8 | 15.6 | 62.5 | 5782.3 | n/a | 0 | 0 |
| 65536 | whole | skipped | old | 8 | 10.4 | 46.8 | 49689.5 | n/a | 0 | 0 |
| 65536 | whole | skipped | new | 8 | 10.4 | 46.8 | 8358.3 | n/a | 0 | 0 |
| 65536 | frag4 | kept | old | 8 | 1223.9 | 3590.3 | 201727.9 | n/a | 65536 | 0 |
| 65536 | frag4 | kept | new | 8 | 46.8 | 291.7 | 59750.3 | n/a | 0 | 0 |
| 65536 | frag4 | skipped | old | 8 | 1182.3 | 3235.1 | 401407.9 | n/a | 65536 | 0 |
| 65536 | frag4 | skipped | new | 8 | 36.4 | 250.1 | 29439.9 | n/a | 0 | 0 |
| 65536 | split | kept | old | 8 | 1307.9 | 6095.9 | 295935.9 | n/a | 0 | 63498 |
| 65536 | split | kept | new | 8 | 1307.9 | 6335.9 | 605798.3 | n/a | 0 | 63498 |
| 65536 | split | skipped | old | 8 | 1276.7 | 1349.5 | 20825.5 | n/a | 0 | 63498 |
| 65536 | split | skipped | new | 8 | 1276.7 | 1349.5 | 29644.7 | n/a | 0 | 63498 |
