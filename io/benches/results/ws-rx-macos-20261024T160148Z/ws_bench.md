counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
samples=20000
validate_utf8=true

| size B | shape | decision | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg | span copy B/msg |
|---|---|---|---|---|---|---|---|---|---|---|
| 100 | whole | kept | old | 64 | 7.8 | 10.4 | 1289.5 | n/a | 0 | 0 |
| 100 | whole | kept | new | 64 | 7.8 | 9.1 | 871.1 | n/a | 0 | 0 |
| 100 | whole | skipped | old | 64 | 7.1 | 7.8 | 585.5 | n/a | 0 | 0 |
| 100 | whole | skipped | new | 64 | 3.9 | 9.1 | 1056.7 | n/a | 0 | 0 |
| 100 | frag4 | kept | old | 64 | 31.9 | 35.8 | 1819.1 | n/a | 100 | 0 |
| 100 | frag4 | kept | new | 64 | 39.0 | 46.8 | 1273.5 | n/a | 0 | 0 |
| 100 | frag4 | skipped | old | 64 | 31.2 | 34.5 | 1583.1 | n/a | 100 | 0 |
| 100 | frag4 | skipped | new | 64 | 15.6 | 16.9 | 1675.1 | n/a | 0 | 0 |
| 100 | split | kept | old | 64 | 16.2 | 17.5 | 1212.7 | n/a | 0 | 101 |
| 100 | split | kept | new | 64 | 16.9 | 20.1 | 1486.3 | n/a | 0 | 101 |
| 100 | split | skipped | old | 64 | 15.6 | 17.5 | 2537.5 | n/a | 0 | 101 |
| 100 | split | skipped | new | 64 | 13.0 | 14.3 | 1204.7 | n/a | 0 | 101 |
| 300 | whole | kept | old | 64 | 11.7 | 13.6 | 2707.1 | n/a | 0 | 0 |
| 300 | whole | kept | new | 64 | 11.7 | 13.0 | 5462.3 | n/a | 0 | 0 |
| 300 | whole | skipped | old | 64 | 11.0 | 30.5 | 1244.7 | n/a | 0 | 0 |
| 300 | whole | skipped | new | 64 | 3.9 | 9.7 | 2238.3 | n/a | 0 | 0 |
| 300 | frag4 | kept | old | 64 | 30.5 | 44.2 | 3385.5 | n/a | 300 | 0 |
| 300 | frag4 | kept | new | 64 | 52.0 | 63.8 | 2374.3 | n/a | 0 | 0 |
| 300 | frag4 | skipped | old | 64 | 29.9 | 32.5 | 2174.3 | n/a | 300 | 0 |
| 300 | frag4 | skipped | new | 64 | 15.6 | 39.0 | 14540.7 | n/a | 0 | 0 |
| 300 | split | kept | old | 64 | 23.4 | 33.8 | 3201.5 | n/a | 0 | 302 |
| 300 | split | kept | new | 64 | 23.4 | 69.6 | 6511.9 | n/a | 0 | 302 |
| 300 | split | skipped | old | 64 | 22.1 | 30.5 | 3897.5 | n/a | 0 | 302 |
| 300 | split | skipped | new | 64 | 15.6 | 20.8 | 3475.1 | n/a | 0 | 302 |
| 4096 | whole | kept | old | 64 | 55.3 | 151.0 | 5558.3 | n/a | 0 | 0 |
| 4096 | whole | kept | new | 64 | 54.6 | 100.9 | 2956.7 | n/a | 0 | 0 |
| 4096 | whole | skipped | old | 64 | 54.0 | 500.3 | 21567.9 | n/a | 0 | 0 |
| 4096 | whole | skipped | new | 64 | 5.8 | 13.0 | 3571.1 | n/a | 0 | 0 |
| 4096 | frag4 | kept | old | 64 | 112.6 | 958.3 | 13542.3 | n/a | 4096 | 0 |
| 4096 | frag4 | kept | new | 64 | 78.1 | 98.9 | 4662.3 | n/a | 0 | 0 |
| 4096 | frag4 | skipped | old | 64 | 110.0 | 330.7 | 5532.7 | n/a | 4096 | 0 |
| 4096 | frag4 | skipped | new | 64 | 19.5 | 22.7 | 5958.3 | n/a | 0 | 0 |
| 4096 | split | kept | old | 64 | 126.3 | 351.5 | 24473.5 | n/a | 0 | 4068 |
| 4096 | split | kept | new | 64 | 126.3 | 399.1 | 9798.3 | n/a | 0 | 4068 |
| 4096 | split | skipped | old | 64 | 126.9 | 893.5 | 47155.1 | n/a | 0 | 4068 |
| 4096 | split | skipped | new | 64 | 90.4 | 716.3 | 32435.1 | n/a | 0 | 4068 |
| 65536 | whole | kept | old | 8 | 823.1 | 5116.7 | 314777.5 | n/a | 0 | 0 |
| 65536 | whole | kept | new | 8 | 812.7 | 1463.9 | 36403.1 | n/a | 0 | 0 |
| 65536 | whole | skipped | old | 8 | 807.5 | 1161.5 | 8870.3 | n/a | 0 | 0 |
| 65536 | whole | skipped | new | 8 | 10.4 | 15.6 | 286.5 | n/a | 0 | 0 |
| 65536 | frag4 | kept | old | 8 | 1875.1 | 4105.5 | 90060.7 | n/a | 65536 | 0 |
| 65536 | frag4 | kept | new | 8 | 839.1 | 963.9 | 147763.1 | n/a | 0 | 0 |
| 65536 | frag4 | skipped | old | 8 | 1860.7 | 3407.9 | 138239.9 | n/a | 65536 | 0 |
| 65536 | frag4 | skipped | new | 8 | 36.4 | 46.8 | 32844.7 | n/a | 0 | 0 |
| 65536 | split | kept | old | 8 | 1891.1 | 3721.5 | 136499.1 | n/a | 0 | 63498 |
| 65536 | split | kept | new | 8 | 1902.3 | 3507.1 | 127385.5 | n/a | 0 | 63498 |
| 65536 | split | skipped | old | 8 | 1907.1 | 6815.9 | 80947.1 | n/a | 0 | 63498 |
| 65536 | split | skipped | new | 8 | 1307.9 | 3974.3 | 94003.1 | n/a | 0 | 63498 |
