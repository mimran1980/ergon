counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=tx
samples=20000

| size B | stage | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|
| 100 | build | old | 64 | 59.8 | 360.7 | 14630.3 | no |
| 100 | build | new | 64 | 58.5 | 326.9 | 60415.9 | no |
| 100 | sealed | old | 64 | 371.1 | 2451.1 | 54271.9 | no |
| 100 | sealed | new | 64 | 369.1 | 2599.9 | 119705.5 | no |
| 300 | build | old | 64 | 159.5 | 1014.3 | 127385.5 | no |
| 300 | build | new | 64 | 160.1 | 1127.9 | 147251.1 | no |
| 300 | sealed | old | 64 | 509.9 | 1678.3 | 250060.7 | no |
| 300 | sealed | new | 64 | 509.9 | 1695.9 | 161996.7 | yes |
| 4096 | build | old | 64 | 2075.1 | 10675.1 | 267673.5 | no |
| 4096 | build | new | 64 | 2031.9 | 10195.1 | 161279.9 | no |
| 4096 | sealed | old | 64 | 3182.3 | 21798.3 | 365158.3 | no |
| 4096 | sealed | new | 64 | 3134.3 | 20223.9 | 519782.3 | no |
| 16384 | build | old | 16 | 8108.7 | 18150.3 | 1114111.9 | no |
| 16384 | build | new | 16 | 8115.1 | 18726.3 | 1415577.5 | yes |
| 16384 | sealed | old | 16 | 11782.3 | 21465.5 | 516505.5 | no |
| 16384 | sealed | new | 16 | 11788.7 | 21964.7 | 394649.5 | no |
