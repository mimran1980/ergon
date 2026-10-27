counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=body
samples=20000

| body B | text | check | delivered as | K | ns/response p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 492.3 | 593.1 | 3220.7 | no |
| 100 | ascii | ascii | AssumedAscii | 64 | 491.5 | 592.7 | 2132.7 | no |
| 100 | ascii | ascii-checked | Ascii | 64 | 505.5 | 603.1 | 2257.5 | no |
| 100 | ascii | utf8 | Utf8 | 64 | 496.7 | 595.9 | 3047.9 | no |
| 100 | mixed | none | Unchecked | 64 | 489.1 | 610.3 | 8479.9 | no |
| 100 | mixed | ascii | AssumedAscii | 64 | 489.1 | 615.9 | 7635.1 | no |
| 100 | mixed | ascii-checked | NotAscii | 64 | 502.7 | 629.1 | 2481.5 | no |
| 100 | mixed | utf8 | Utf8 | 64 | 509.9 | 641.9 | 4454.3 | no |
| 300 | ascii | none | Unchecked | 64 | 489.1 | 602.3 | 2263.9 | no |
| 300 | ascii | ascii | AssumedAscii | 64 | 489.1 | 590.7 | 2230.3 | no |
| 300 | ascii | ascii-checked | Ascii | 64 | 528.7 | 633.1 | 10335.9 | no |
| 300 | ascii | utf8 | Utf8 | 64 | 500.7 | 617.1 | 23052.7 | no |
| 300 | mixed | none | Unchecked | 64 | 488.3 | 590.7 | 3295.9 | no |
| 300 | mixed | ascii | AssumedAscii | 64 | 488.3 | 583.5 | 2025.5 | no |
| 300 | mixed | ascii-checked | NotAscii | 64 | 527.9 | 625.9 | 1996.7 | no |
| 300 | mixed | utf8 | Utf8 | 64 | 543.9 | 649.1 | 3193.5 | no |
| 4096 | ascii | none | Unchecked | 64 | 491.5 | 591.9 | 2999.9 | no |
| 4096 | ascii | ascii | AssumedAscii | 64 | 491.5 | 591.1 | 1624.7 | no |
| 4096 | ascii | ascii-checked | Ascii | 64 | 1067.1 | 1236.7 | 21721.5 | no |
| 4096 | ascii | utf8 | Utf8 | 64 | 631.5 | 754.7 | 3343.9 | no |
| 4096 | mixed | none | Unchecked | 64 | 492.3 | 647.1 | 2859.1 | no |
| 4096 | mixed | ascii | AssumedAscii | 64 | 492.3 | 657.1 | 1854.3 | no |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 1067.9 | 1322.3 | 7967.9 | no |
| 4096 | mixed | utf8 | Utf8 | 64 | 1043.1 | 1330.3 | 8819.1 | no |
| 16384 | ascii | none | Unchecked | 16 | 953.5 | 1287.1 | 6604.7 | no |
| 16384 | ascii | ascii | AssumedAscii | 16 | 953.5 | 1263.1 | 33023.9 | no |
| 16384 | ascii | ascii-checked | Ascii | 16 | 3223.9 | 4227.1 | 16422.3 | no |
| 16384 | ascii | utf8 | Utf8 | 16 | 1445.5 | 1985.5 | 15731.1 | no |
| 16384 | mixed | none | Unchecked | 16 | 951.1 | 1263.1 | 7929.5 | no |
| 16384 | mixed | ascii | AssumedAscii | 16 | 951.1 | 1219.1 | 4716.7 | no |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 3222.3 | 4111.9 | 228147.1 | yes |
| 16384 | mixed | utf8 | Utf8 | 16 | 2951.9 | 3980.7 | 10937.5 | no |
