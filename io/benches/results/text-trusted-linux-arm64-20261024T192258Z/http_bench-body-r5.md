counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=body
samples=20000

| body B | text | check | delivered as | K | ns/response p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 488.3 | 604.3 | 2315.1 | no |
| 100 | ascii | ascii | AssumedAscii | 64 | 489.1 | 604.3 | 3022.3 | no |
| 100 | ascii | ascii-checked | Ascii | 64 | 501.9 | 629.5 | 2947.1 | no |
| 100 | ascii | utf8 | Utf8 | 64 | 494.3 | 621.9 | 1963.1 | no |
| 100 | mixed | none | Unchecked | 64 | 488.3 | 672.7 | 2143.9 | no |
| 100 | mixed | ascii | AssumedAscii | 64 | 489.5 | 674.7 | 5535.9 | no |
| 100 | mixed | ascii-checked | NotAscii | 64 | 502.7 | 680.3 | 2051.1 | no |
| 100 | mixed | utf8 | Utf8 | 64 | 510.7 | 709.1 | 2147.1 | no |
| 300 | ascii | none | Unchecked | 64 | 488.3 | 710.3 | 7615.9 | no |
| 300 | ascii | ascii | AssumedAscii | 64 | 489.1 | 721.5 | 6201.5 | no |
| 300 | ascii | ascii-checked | Ascii | 64 | 527.9 | 759.1 | 4886.3 | no |
| 300 | ascii | utf8 | Utf8 | 64 | 500.7 | 715.1 | 3014.3 | no |
| 300 | mixed | none | Unchecked | 64 | 488.3 | 647.1 | 20646.3 | no |
| 300 | mixed | ascii | AssumedAscii | 64 | 489.1 | 658.3 | 5612.7 | no |
| 300 | mixed | ascii-checked | NotAscii | 64 | 527.9 | 724.7 | 12236.7 | no |
| 300 | mixed | utf8 | Utf8 | 64 | 544.3 | 746.3 | 5276.7 | no |
| 4096 | ascii | none | Unchecked | 64 | 491.5 | 647.1 | 23231.9 | no |
| 4096 | ascii | ascii | AssumedAscii | 64 | 493.5 | 659.1 | 6879.9 | no |
| 4096 | ascii | ascii-checked | Ascii | 64 | 1067.9 | 1383.9 | 24895.9 | no |
| 4096 | ascii | utf8 | Utf8 | 64 | 634.3 | 870.3 | 71270.3 | no |
| 4096 | mixed | none | Unchecked | 64 | 491.5 | 673.9 | 1734.3 | no |
| 4096 | mixed | ascii | AssumedAscii | 64 | 493.5 | 673.9 | 2897.5 | no |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 1067.9 | 1383.1 | 76134.3 | no |
| 4096 | mixed | utf8 | Utf8 | 64 | 1044.7 | 1437.5 | 21030.3 | no |
| 16384 | ascii | none | Unchecked | 16 | 947.9 | 1223.9 | 6406.3 | no |
| 16384 | ascii | ascii | AssumedAscii | 16 | 953.5 | 1242.3 | 15411.1 | no |
| 16384 | ascii | ascii-checked | Ascii | 16 | 3222.3 | 4198.3 | 12787.1 | no |
| 16384 | ascii | utf8 | Utf8 | 16 | 1445.5 | 1982.3 | 54476.7 | no |
| 16384 | mixed | none | Unchecked | 16 | 955.9 | 1343.9 | 109670.3 | no |
| 16384 | mixed | ascii | AssumedAscii | 16 | 961.5 | 1362.3 | 33766.3 | no |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 3238.3 | 4502.3 | 254975.9 | no |
| 16384 | mixed | utf8 | Utf8 | 16 | 2972.7 | 4774.3 | 182579.1 | no |
