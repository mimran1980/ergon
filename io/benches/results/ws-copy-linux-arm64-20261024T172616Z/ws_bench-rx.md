counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=rx
samples=20000
text_check=Utf8

| size B | shape | decision | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg | span copy B/msg |
|---|---|---|---|---|---|---|---|---|---|---|
| 100 | whole | kept | old | 64 | 21.4 | 39.7 | 14003.1 | no | 0 | 0 |
| 100 | whole | kept | new | 64 | 21.4 | 38.4 | 28262.3 | no | 0 | 0 |
| 100 | whole | skipped | old | 64 | 20.1 | 33.2 | 4703.9 | no | 0 | 0 |
| 100 | whole | skipped | new | 64 | 3.9 | 12.3 | 2171.1 | no | 0 | 0 |
| 100 | frag4 | kept | old | 64 | 30.5 | 87.2 | 9631.9 | no | 100 | 0 |
| 100 | frag4 | kept | new | 64 | 36.4 | 117.8 | 22783.9 | no | 0 | 0 |
| 100 | frag4 | skipped | old | 64 | 29.9 | 94.4 | 62771.1 | no | 100 | 0 |
| 100 | frag4 | skipped | new | 64 | 16.9 | 50.1 | 18815.9 | no | 0 | 0 |
| 100 | split | kept | old | 64 | 38.4 | 57.2 | 4825.5 | no | 0 | 51 |
| 100 | split | kept | new | 64 | 38.4 | 55.9 | 15859.1 | no | 0 | 51 |
| 100 | split | skipped | old | 64 | 37.7 | 54.0 | 10860.7 | no | 0 | 51 |
| 100 | split | skipped | new | 64 | 17.5 | 37.1 | 10438.3 | no | 0 | 51 |
| 300 | whole | kept | old | 64 | 49.4 | 86.5 | 26623.9 | no | 0 | 0 |
| 300 | whole | kept | new | 64 | 49.4 | 83.3 | 18777.5 | no | 0 | 0 |
| 300 | whole | skipped | old | 64 | 48.8 | 54.0 | 4697.5 | no | 0 | 0 |
| 300 | whole | skipped | new | 64 | 4.5 | 11.7 | 781.5 | no | 0 | 0 |
| 300 | frag4 | kept | old | 64 | 121.7 | 237.7 | 35046.3 | no | 300 | 0 |
| 300 | frag4 | kept | new | 64 | 61.8 | 140.6 | 59289.5 | no | 0 | 0 |
| 300 | frag4 | skipped | old | 64 | 119.7 | 409.5 | 41881.5 | no | 300 | 0 |
| 300 | frag4 | skipped | new | 64 | 16.2 | 50.7 | 9427.1 | no | 0 | 0 |
| 300 | split | kept | old | 64 | 94.4 | 154.9 | 9574.3 | no | 0 | 152 |
| 300 | split | kept | new | 64 | 94.4 | 150.3 | 16025.5 | no | 0 | 152 |
| 300 | split | skipped | old | 64 | 93.7 | 195.3 | 13951.9 | no | 0 | 152 |
| 300 | split | skipped | new | 64 | 44.9 | 85.2 | 7315.1 | no | 0 | 152 |
| 4096 | whole | kept | old | 64 | 617.1 | 1731.1 | 75212.7 | no | 0 | 0 |
| 4096 | whole | kept | new | 64 | 617.1 | 1942.3 | 188927.9 | no | 0 | 0 |
| 4096 | whole | skipped | old | 64 | 617.9 | 1501.5 | 151142.3 | yes | 0 | 0 |
| 4096 | whole | skipped | new | 64 | 27.9 | 93.0 | 14041.5 | no | 0 | 0 |
| 4096 | frag4 | kept | old | 64 | 1459.9 | 3391.9 | 78591.9 | yes | 4096 | 0 |
| 4096 | frag4 | kept | new | 64 | 626.3 | 1505.5 | 85196.7 | no | 0 | 0 |
| 4096 | frag4 | skipped | old | 64 | 1455.9 | 3894.3 | 31411.1 | no | 4096 | 0 |
| 4096 | frag4 | skipped | new | 64 | 24.0 | 207.7 | 31001.5 | no | 0 | 0 |
| 4096 | split | kept | old | 64 | 1033.5 | 1801.5 | 71321.5 | no | 0 | 2050 |
| 4096 | split | kept | new | 64 | 1032.7 | 1745.5 | 16985.5 | no | 0 | 2050 |
| 4096 | split | skipped | old | 64 | 1033.5 | 1687.9 | 30361.5 | no | 0 | 2050 |
| 4096 | split | skipped | new | 64 | 416.3 | 703.9 | 14771.1 | no | 0 | 2050 |
| 65536 | whole | kept | old | 8 | 9695.9 | 17919.9 | 468582.3 | no | 0 | 0 |
| 65536 | whole | kept | new | 8 | 9695.9 | 19071.9 | 546815.9 | no | 0 | 0 |
| 65536 | whole | skipped | old | 8 | 9721.5 | 65587.1 | 2829516.7 | yes | 0 | 0 |
| 65536 | whole | skipped | new | 8 | 41.6 | 213.5 | 343859.1 | no | 0 | 0 |
| 65536 | frag4 | kept | old | 8 | 23308.7 | 119807.9 | 3512729.5 | no | 65536 | 0 |
| 65536 | frag4 | kept | new | 8 | 9715.1 | 54835.1 | 1424588.7 | no | 0 | 0 |
| 65536 | frag4 | skipped | old | 8 | 22873.5 | 27135.9 | 298393.5 | no | 65536 | 0 |
| 65536 | frag4 | skipped | new | 8 | 41.6 | 218.7 | 14259.1 | no | 0 | 0 |
| 65536 | split | kept | old | 8 | 22783.9 | 47462.3 | 627916.7 | no | 0 | 63494 |
| 65536 | split | kept | new | 8 | 22809.5 | 49740.7 | 698777.5 | no | 0 | 63494 |
| 65536 | split | skipped | old | 8 | 22975.9 | 31974.3 | 1027276.7 | no | 0 | 63494 |
| 65536 | split | skipped | new | 8 | 13311.9 | 18495.9 | 2721382.3 | yes | 0 | 63494 |
