counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=body
samples=20000

| body B | text | check | delivered as | K | ns/response p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 490.3 | 613.5 | 46182.3 | no |
| 100 | ascii | ascii | AssumedAscii | 64 | 489.1 | 599.1 | 2055.9 | no |
| 100 | ascii | ascii-checked | Ascii | 64 | 503.5 | 623.9 | 36326.3 | no |
| 100 | ascii | utf8 | Utf8 | 64 | 494.7 | 619.1 | 3871.9 | no |
| 100 | mixed | none | Unchecked | 64 | 490.3 | 594.7 | 2030.3 | no |
| 100 | mixed | ascii | AssumedAscii | 64 | 489.1 | 595.1 | 2043.1 | no |
| 100 | mixed | ascii-checked | NotAscii | 64 | 503.5 | 610.7 | 1987.1 | no |
| 100 | mixed | utf8 | Utf8 | 64 | 510.7 | 615.5 | 2175.9 | no |
| 300 | ascii | none | Unchecked | 64 | 489.5 | 595.9 | 3663.9 | no |
| 300 | ascii | ascii | AssumedAscii | 64 | 489.1 | 597.9 | 5036.7 | no |
| 300 | ascii | ascii-checked | Ascii | 64 | 529.5 | 641.5 | 10086.3 | no |
| 300 | ascii | utf8 | Utf8 | 64 | 500.7 | 604.3 | 1975.9 | no |
| 300 | mixed | none | Unchecked | 64 | 490.3 | 616.7 | 3382.3 | no |
| 300 | mixed | ascii | AssumedAscii | 64 | 489.5 | 629.1 | 2607.9 | no |
| 300 | mixed | ascii-checked | NotAscii | 64 | 529.9 | 669.9 | 2070.3 | no |
| 300 | mixed | utf8 | Utf8 | 64 | 545.1 | 690.7 | 7571.1 | no |
| 4096 | ascii | none | Unchecked | 64 | 493.1 | 612.7 | 1500.7 | no |
| 4096 | ascii | ascii | AssumedAscii | 64 | 493.1 | 606.3 | 1742.3 | no |
| 4096 | ascii | ascii-checked | Ascii | 64 | 1069.5 | 1279.9 | 2614.3 | no |
| 4096 | ascii | utf8 | Utf8 | 64 | 633.5 | 777.9 | 1646.3 | no |
| 4096 | mixed | none | Unchecked | 64 | 493.5 | 638.7 | 1799.9 | no |
| 4096 | mixed | ascii | AssumedAscii | 64 | 493.5 | 645.1 | 9254.3 | no |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 1069.5 | 1311.1 | 5295.9 | no |
| 4096 | mixed | utf8 | Utf8 | 64 | 1044.7 | 1322.3 | 9356.7 | yes |
| 16384 | ascii | none | Unchecked | 16 | 951.1 | 1182.3 | 6211.1 | no |
| 16384 | ascii | ascii | AssumedAscii | 16 | 955.9 | 1219.1 | 10047.9 | yes |
| 16384 | ascii | ascii-checked | Ascii | 16 | 3203.1 | 3958.3 | 15871.9 | no |
| 16384 | ascii | utf8 | Utf8 | 16 | 1445.5 | 1862.3 | 9785.5 | no |
| 16384 | mixed | none | Unchecked | 16 | 951.1 | 1244.7 | 7193.5 | no |
| 16384 | mixed | ascii | AssumedAscii | 16 | 955.9 | 1229.5 | 3708.7 | no |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 3203.1 | 4057.5 | 17343.9 | no |
| 16384 | mixed | utf8 | Utf8 | 16 | 2956.7 | 3903.9 | 9823.9 | no |
