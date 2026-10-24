counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
samples=20000
validate_utf8=false

| size B | shape | decision | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg | span copy B/msg |
|---|---|---|---|---|---|---|---|---|---|---|
| 100 | whole | kept | old | 64 | 5.2 | 7.1 | 4796.7 | no | 0 | 0 |
| 100 | whole | kept | new | 64 | 5.2 | 7.1 | 17638.3 | no | 0 | 0 |
| 100 | whole | skipped | old | 64 | 4.5 | 6.5 | 2924.7 | no | 0 | 0 |
| 100 | whole | skipped | new | 64 | 3.9 | 5.8 | 1486.3 | no | 0 | 0 |
| 100 | frag4 | kept | old | 64 | 22.7 | 69.6 | 22758.3 | no | 100 | 0 |
| 100 | frag4 | kept | new | 64 | 31.2 | 133.4 | 69580.7 | no | 0 | 0 |
| 100 | frag4 | skipped | old | 64 | 22.1 | 105.4 | 8313.5 | no | 100 | 0 |
| 100 | frag4 | skipped | new | 64 | 15.6 | 48.1 | 4297.5 | no | 0 | 0 |
| 100 | split | kept | old | 64 | 27.3 | 67.0 | 8070.3 | no | 0 | 101 |
| 100 | split | kept | new | 64 | 27.3 | 58.5 | 4847.9 | no | 0 | 101 |
| 100 | split | skipped | old | 64 | 27.9 | 110.6 | 4131.1 | no | 0 | 101 |
| 100 | split | skipped | new | 64 | 26.6 | 120.4 | 6361.5 | no | 0 | 101 |
| 300 | whole | kept | old | 64 | 5.2 | 14.9 | 6361.5 | yes | 0 | 0 |
| 300 | whole | kept | new | 64 | 5.2 | 14.9 | 15014.3 | no | 0 | 0 |
| 300 | whole | skipped | old | 64 | 4.5 | 11.0 | 3308.7 | no | 0 | 0 |
| 300 | whole | skipped | new | 64 | 3.9 | 10.4 | 1686.3 | no | 0 | 0 |
| 300 | frag4 | kept | old | 64 | 71.6 | 106.1 | 7481.5 | no | 300 | 0 |
| 300 | frag4 | kept | new | 64 | 31.2 | 73.5 | 8089.5 | no | 0 | 0 |
| 300 | frag4 | skipped | old | 64 | 70.9 | 102.8 | 28851.1 | no | 300 | 0 |
| 300 | frag4 | skipped | new | 64 | 15.6 | 42.9 | 12095.9 | no | 0 | 0 |
| 300 | split | kept | old | 64 | 70.3 | 100.9 | 61747.1 | no | 0 | 302 |
| 300 | split | kept | new | 64 | 70.3 | 107.4 | 42700.7 | no | 0 | 302 |
| 300 | split | skipped | old | 64 | 69.0 | 155.5 | 68044.7 | no | 0 | 302 |
| 300 | split | skipped | new | 64 | 69.0 | 98.9 | 20787.1 | no | 0 | 302 |
| 4096 | whole | kept | old | 64 | 10.4 | 117.1 | 9350.3 | no | 0 | 0 |
| 4096 | whole | kept | new | 64 | 10.4 | 110.6 | 8166.3 | no | 0 | 0 |
| 4096 | whole | skipped | old | 64 | 7.8 | 119.1 | 10169.5 | no | 0 | 0 |
| 4096 | whole | skipped | new | 64 | 7.8 | 122.3 | 9919.9 | no | 0 | 0 |
| 4096 | frag4 | kept | old | 64 | 846.3 | 1911.9 | 105983.9 | no | 4096 | 0 |
| 4096 | frag4 | kept | new | 64 | 34.5 | 426.7 | 47615.9 | no | 0 | 0 |
| 4096 | frag4 | skipped | old | 64 | 844.7 | 1964.7 | 70809.5 | no | 4096 | 0 |
| 4096 | frag4 | skipped | new | 64 | 29.2 | 304.1 | 14745.5 | no | 0 | 0 |
| 4096 | split | kept | old | 64 | 860.7 | 2052.7 | 117452.7 | no | 0 | 4068 |
| 4096 | split | kept | new | 64 | 860.7 | 1979.1 | 79257.5 | no | 0 | 4068 |
| 4096 | split | skipped | old | 64 | 860.7 | 1883.1 | 65689.5 | no | 0 | 4068 |
| 4096 | split | skipped | new | 64 | 860.7 | 1929.5 | 65843.1 | no | 0 | 4068 |
| 65536 | whole | kept | old | 8 | 15.6 | 390.7 | 337100.7 | no | 0 | 0 |
| 65536 | whole | kept | new | 8 | 20.8 | 380.3 | 24985.5 | no | 0 | 0 |
| 65536 | whole | skipped | old | 8 | 20.8 | 375.1 | 31615.9 | no | 0 | 0 |
| 65536 | whole | skipped | new | 8 | 20.8 | 359.3 | 16025.5 | no | 0 | 0 |
| 65536 | frag4 | kept | old | 8 | 13670.3 | 30207.9 | 825753.5 | no | 65536 | 0 |
| 65536 | frag4 | kept | new | 8 | 52.0 | 927.1 | 135270.3 | no | 0 | 0 |
| 65536 | frag4 | skipped | old | 8 | 13350.3 | 19110.3 | 643071.9 | no | 65536 | 0 |
| 65536 | frag4 | skipped | new | 8 | 36.4 | 578.3 | 5276.7 | no | 0 | 0 |
| 65536 | split | kept | old | 8 | 13542.3 | 24537.5 | 299827.1 | no | 0 | 63498 |
| 65536 | split | kept | new | 8 | 13542.3 | 24524.7 | 1938227.1 | no | 0 | 63498 |
| 65536 | split | skipped | old | 8 | 13132.7 | 18342.3 | 151142.3 | no | 0 | 63498 |
| 65536 | split | skipped | new | 8 | 13145.5 | 18470.3 | 165375.9 | no | 0 | 63498 |
