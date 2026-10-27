counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=rx
samples=20000
text_check=None

| size B | shape | decision | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg | span copy B/msg |
|---|---|---|---|---|---|---|---|---|---|---|
| 100 | whole | kept | old | 64 | 5.8 | 8.4 | 2684.7 | no | 0 | 0 |
| 100 | whole | kept | new | 64 | 5.8 | 9.1 | 572.3 | no | 0 | 0 |
| 100 | whole | skipped | old | 64 | 5.2 | 9.7 | 545.1 | no | 0 | 0 |
| 100 | whole | skipped | new | 64 | 3.9 | 7.8 | 473.5 | no | 0 | 0 |
| 100 | frag4 | kept | old | 64 | 24.0 | 72.2 | 6441.5 | no | 100 | 0 |
| 100 | frag4 | kept | new | 64 | 24.7 | 80.0 | 7283.1 | no | 0 | 0 |
| 100 | frag4 | skipped | old | 64 | 23.4 | 30.5 | 421.5 | yes | 100 | 0 |
| 100 | frag4 | skipped | new | 64 | 16.2 | 21.4 | 955.9 | no | 0 | 0 |
| 100 | split | kept | old | 64 | 18.2 | 44.9 | 3350.3 | no | 0 | 51 |
| 100 | split | kept | new | 64 | 18.2 | 44.9 | 6047.9 | no | 0 | 51 |
| 100 | split | skipped | old | 64 | 17.5 | 26.0 | 493.1 | no | 0 | 51 |
| 100 | split | skipped | new | 64 | 16.9 | 27.9 | 433.1 | no | 0 | 51 |
| 300 | whole | kept | old | 64 | 5.8 | 14.3 | 1639.9 | no | 0 | 0 |
| 300 | whole | kept | new | 64 | 5.8 | 13.6 | 423.1 | no | 0 | 0 |
| 300 | whole | skipped | old | 64 | 5.2 | 9.1 | 483.9 | no | 0 | 0 |
| 300 | whole | skipped | new | 64 | 4.5 | 8.4 | 125.6 | no | 0 | 0 |
| 300 | frag4 | kept | old | 64 | 72.2 | 84.6 | 480.7 | no | 300 | 0 |
| 300 | frag4 | kept | new | 64 | 24.7 | 34.5 | 378.9 | no | 0 | 0 |
| 300 | frag4 | skipped | old | 64 | 71.6 | 81.3 | 479.9 | no | 300 | 0 |
| 300 | frag4 | skipped | new | 64 | 16.2 | 22.7 | 321.7 | no | 0 | 0 |
| 300 | split | kept | old | 64 | 44.9 | 55.3 | 477.9 | no | 0 | 152 |
| 300 | split | kept | new | 64 | 44.9 | 54.6 | 427.1 | no | 0 | 152 |
| 300 | split | skipped | old | 64 | 45.5 | 67.7 | 14207.9 | no | 0 | 152 |
| 300 | split | skipped | new | 64 | 44.9 | 78.7 | 7020.7 | no | 0 | 152 |
| 4096 | whole | kept | old | 64 | 9.7 | 52.0 | 2252.7 | no | 0 | 0 |
| 4096 | whole | kept | new | 64 | 9.7 | 49.4 | 3235.1 | no | 0 | 0 |
| 4096 | whole | skipped | old | 64 | 10.4 | 57.9 | 4825.5 | no | 0 | 0 |
| 4096 | whole | skipped | new | 64 | 10.4 | 57.9 | 1831.9 | no | 0 | 0 |
| 4096 | frag4 | kept | old | 64 | 853.5 | 1325.5 | 18380.7 | no | 4096 | 0 |
| 4096 | frag4 | kept | new | 64 | 31.9 | 123.0 | 7577.5 | no | 0 | 0 |
| 4096 | frag4 | skipped | old | 64 | 852.7 | 1319.1 | 10175.9 | no | 4096 | 0 |
| 4096 | frag4 | skipped | new | 64 | 29.9 | 104.1 | 22425.5 | no | 0 | 0 |
| 4096 | split | kept | old | 64 | 416.7 | 669.5 | 33177.5 | no | 0 | 2050 |
| 4096 | split | kept | new | 64 | 416.7 | 687.1 | 14015.9 | no | 0 | 2050 |
| 4096 | split | skipped | old | 64 | 417.5 | 728.7 | 57651.1 | no | 0 | 2050 |
| 4096 | split | skipped | new | 64 | 416.3 | 726.7 | 18969.5 | no | 0 | 2050 |
| 65536 | whole | kept | old | 8 | 20.8 | 166.6 | 14220.7 | no | 0 | 0 |
| 65536 | whole | kept | new | 8 | 20.8 | 187.5 | 18201.5 | no | 0 | 0 |
| 65536 | whole | skipped | old | 8 | 15.6 | 161.4 | 15820.7 | no | 0 | 0 |
| 65536 | whole | skipped | new | 8 | 15.6 | 135.4 | 2724.7 | no | 0 | 0 |
| 65536 | frag4 | kept | old | 8 | 13721.5 | 19110.3 | 106803.1 | no | 65536 | 0 |
| 65536 | frag4 | kept | new | 8 | 104.1 | 505.5 | 36659.1 | no | 0 | 0 |
| 65536 | frag4 | skipped | old | 8 | 13811.1 | 121548.7 | 3765043.1 | no | 65536 | 0 |
| 65536 | frag4 | skipped | new | 8 | 156.2 | 1094.3 | 2980249.5 | yes | 0 | 0 |
| 65536 | split | kept | old | 8 | 13401.5 | 140287.9 | 4856217.5 | no | 0 | 63494 |
| 65536 | split | kept | new | 8 | 13401.5 | 134246.3 | 8093695.9 | yes | 0 | 63494 |
| 65536 | split | skipped | old | 8 | 13375.9 | 79308.7 | 2704998.3 | yes | 0 | 63494 |
| 65536 | split | skipped | new | 8 | 13375.9 | 74291.1 | 4561305.5 | yes | 0 | 63494 |
