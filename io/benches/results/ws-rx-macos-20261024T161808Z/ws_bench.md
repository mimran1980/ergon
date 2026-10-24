counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
samples=20000
validate_utf8=true

| size B | shape | decision | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg | span copy B/msg |
|---|---|---|---|---|---|---|---|---|---|---|
| 100 | whole | kept | old | 64 | 7.8 | 22.1 | 12870.3 | n/a | 0 | 0 |
| 100 | whole | kept | new | 64 | 7.8 | 22.7 | 6975.9 | n/a | 0 | 0 |
| 100 | whole | skipped | old | 64 | 7.1 | 20.1 | 9715.1 | n/a | 0 | 0 |
| 100 | whole | skipped | new | 64 | 3.9 | 10.4 | 7020.7 | n/a | 0 | 0 |
| 100 | frag4 | kept | old | 64 | 32.5 | 77.4 | 481279.9 | n/a | 100 | 0 |
| 100 | frag4 | kept | new | 64 | 34.5 | 94.4 | 311705.5 | n/a | 0 | 0 |
| 100 | frag4 | skipped | old | 64 | 31.2 | 76.1 | 252108.7 | n/a | 100 | 0 |
| 100 | frag4 | skipped | new | 64 | 15.6 | 39.0 | 50252.7 | n/a | 0 | 0 |
| 100 | split | kept | old | 64 | 18.8 | 52.7 | 87807.9 | n/a | 0 | 101 |
| 100 | split | kept | new | 64 | 18.8 | 52.7 | 285491.1 | n/a | 0 | 101 |
| 100 | split | skipped | old | 64 | 15.6 | 47.5 | 309247.9 | n/a | 0 | 101 |
| 100 | split | skipped | new | 64 | 13.0 | 39.0 | 76799.9 | n/a | 0 | 101 |
| 300 | whole | kept | old | 64 | 11.7 | 35.1 | 184934.3 | n/a | 0 | 0 |
| 300 | whole | kept | new | 64 | 11.7 | 34.5 | 58316.7 | n/a | 0 | 0 |
| 300 | whole | skipped | old | 64 | 11.0 | 30.5 | 47052.7 | n/a | 0 | 0 |
| 300 | whole | skipped | new | 64 | 3.9 | 9.1 | 73932.7 | n/a | 0 | 0 |
| 300 | frag4 | kept | old | 64 | 30.5 | 93.0 | 240025.5 | n/a | 300 | 0 |
| 300 | frag4 | kept | new | 64 | 35.8 | 103.5 | 191897.5 | n/a | 0 | 0 |
| 300 | frag4 | skipped | old | 64 | 30.5 | 91.1 | 248627.1 | n/a | 300 | 0 |
| 300 | frag4 | skipped | new | 64 | 15.6 | 39.7 | 151551.9 | n/a | 0 | 0 |
| 300 | split | kept | old | 64 | 24.7 | 70.3 | 165580.7 | n/a | 0 | 302 |
| 300 | split | kept | new | 64 | 24.7 | 70.9 | 493158.3 | n/a | 0 | 302 |
| 300 | split | skipped | old | 64 | 22.1 | 67.0 | 94361.5 | n/a | 0 | 302 |
| 300 | split | skipped | new | 64 | 14.9 | 44.2 | 105369.5 | n/a | 0 | 302 |
| 4096 | whole | kept | old | 64 | 55.3 | 184.2 | 232243.1 | n/a | 0 | 0 |
| 4096 | whole | kept | new | 64 | 55.3 | 161.4 | 246579.1 | n/a | 0 | 0 |
| 4096 | whole | skipped | old | 64 | 54.0 | 153.6 | 372121.5 | n/a | 0 | 0 |
| 4096 | whole | skipped | new | 64 | 6.5 | 14.3 | 80486.3 | n/a | 0 | 0 |
| 4096 | frag4 | kept | old | 64 | 109.3 | 925.5 | 279756.7 | n/a | 4096 | 0 |
| 4096 | frag4 | kept | new | 64 | 76.1 | 368.5 | 184319.9 | n/a | 0 | 0 |
| 4096 | frag4 | skipped | old | 64 | 109.3 | 1007.9 | 425164.7 | n/a | 4096 | 0 |
| 4096 | frag4 | skipped | new | 64 | 20.1 | 186.1 | 111513.5 | n/a | 0 | 0 |
| 4096 | split | kept | old | 64 | 126.3 | 1249.5 | 387276.7 | n/a | 0 | 4068 |
| 4096 | split | kept | new | 64 | 126.3 | 1739.1 | 630783.9 | n/a | 0 | 4068 |
| 4096 | split | skipped | old | 64 | 124.3 | 335.9 | 38604.7 | n/a | 0 | 4068 |
| 4096 | split | skipped | new | 64 | 89.8 | 220.1 | 46259.1 | n/a | 0 | 4068 |
| 65536 | whole | kept | old | 8 | 817.9 | 3022.3 | 337305.5 | n/a | 0 | 0 |
| 65536 | whole | kept | new | 8 | 817.9 | 3052.7 | 643481.5 | n/a | 0 | 0 |
| 65536 | whole | skipped | old | 8 | 812.7 | 4252.7 | 1762918.3 | n/a | 0 | 0 |
| 65536 | whole | skipped | new | 8 | 10.4 | 93.7 | 266239.9 | n/a | 0 | 0 |
| 65536 | frag4 | kept | old | 8 | 1844.7 | 6204.7 | 881459.1 | n/a | 65536 | 0 |
| 65536 | frag4 | kept | new | 8 | 891.1 | 2667.1 | 422707.1 | n/a | 0 | 0 |
| 65536 | frag4 | skipped | old | 8 | 1927.9 | 32742.3 | 2051276.7 | n/a | 65536 | 0 |
| 65536 | frag4 | skipped | new | 8 | 52.0 | 380.3 | 387481.5 | n/a | 0 | 0 |
| 65536 | split | kept | old | 8 | 1911.9 | 12275.1 | 9286451.1 | n/a | 0 | 63498 |
| 65536 | split | kept | new | 8 | 1911.9 | 13811.1 | 5000396.7 | n/a | 0 | 63498 |
| 65536 | split | skipped | old | 8 | 1907.1 | 20287.9 | 1232076.7 | n/a | 0 | 63498 |
| 65536 | split | skipped | new | 8 | 1307.9 | 11743.9 | 5727846.3 | n/a | 0 | 63498 |
