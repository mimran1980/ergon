counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=body
samples=20000

| body B | text | check | delivered as | K | ns/response p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 517.1 | 779.5 | 20287.9 | no |
| 100 | ascii | ascii | AssumedAscii | 64 | 517.5 | 804.3 | 26547.1 | no |
| 100 | ascii | ascii-checked | Ascii | 64 | 529.5 | 811.9 | 9222.3 | no |
| 100 | ascii | utf8 | Utf8 | 64 | 511.9 | 797.5 | 10380.7 | no |
| 100 | mixed | none | Unchecked | 64 | 505.5 | 692.3 | 72499.1 | no |
| 100 | mixed | ascii | AssumedAscii | 64 | 505.5 | 699.5 | 12153.5 | no |
| 100 | mixed | ascii-checked | NotAscii | 64 | 518.3 | 705.1 | 59545.5 | yes |
| 100 | mixed | utf8 | Utf8 | 64 | 524.3 | 689.5 | 8646.3 | no |
| 300 | ascii | none | Unchecked | 64 | 506.7 | 641.9 | 1979.1 | no |
| 300 | ascii | ascii | AssumedAscii | 64 | 506.7 | 638.7 | 2060.7 | no |
| 300 | ascii | ascii-checked | Ascii | 64 | 543.1 | 697.5 | 2265.5 | no |
| 300 | ascii | utf8 | Utf8 | 64 | 514.3 | 651.9 | 2132.7 | no |
| 300 | mixed | none | Unchecked | 64 | 503.9 | 606.3 | 1937.5 | no |
| 300 | mixed | ascii | AssumedAscii | 64 | 503.9 | 613.5 | 2006.3 | no |
| 300 | mixed | ascii-checked | NotAscii | 64 | 533.5 | 648.7 | 1987.1 | no |
| 300 | mixed | utf8 | Utf8 | 64 | 545.1 | 664.7 | 2063.9 | no |
| 4096 | ascii | none | Unchecked | 64 | 507.1 | 654.3 | 2910.3 | no |
| 4096 | ascii | ascii | AssumedAscii | 64 | 508.7 | 658.3 | 1977.5 | no |
| 4096 | ascii | ascii-checked | Ascii | 64 | 1081.5 | 1319.9 | 3647.9 | no |
| 4096 | ascii | utf8 | Utf8 | 64 | 638.3 | 822.3 | 11046.3 | no |
| 4096 | mixed | none | Unchecked | 64 | 508.7 | 701.9 | 4876.7 | no |
| 4096 | mixed | ascii | AssumedAscii | 64 | 509.9 | 710.3 | 5689.5 | no |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 1082.3 | 1419.1 | 11436.7 | no |
| 4096 | mixed | utf8 | Utf8 | 64 | 1055.1 | 1493.5 | 28031.9 | no |
| 16384 | ascii | none | Unchecked | 16 | 966.3 | 1294.3 | 22527.9 | no |
| 16384 | ascii | ascii | AssumedAscii | 16 | 968.7 | 1279.1 | 13049.5 | no |
| 16384 | ascii | ascii-checked | Ascii | 16 | 3235.1 | 4310.3 | 73420.7 | no |
| 16384 | ascii | utf8 | Utf8 | 16 | 1451.1 | 2060.7 | 43699.1 | no |
| 16384 | mixed | none | Unchecked | 16 | 963.9 | 1211.1 | 109875.1 | no |
| 16384 | mixed | ascii | AssumedAscii | 16 | 966.3 | 1203.1 | 10598.3 | no |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 3227.1 | 4070.3 | 19622.3 | no |
| 16384 | mixed | utf8 | Utf8 | 16 | 2956.7 | 3849.5 | 25049.5 | no |
