counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=text
samples=20000

| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 5.8 | 7.1 | 55.9 | no |
| 100 | ascii | ascii | AssumedAscii | 64 | 5.8 | 7.8 | 258.5 | no |
| 100 | ascii | ascii-checked | Ascii | 64 | 20.1 | 24.7 | 624.3 | no |
| 100 | ascii | utf8 | Utf8 | 64 | 19.5 | 24.0 | 787.9 | no |
| 100 | mixed | none | Unchecked | 64 | 5.8 | 6.5 | 78.1 | no |
| 100 | mixed | ascii | AssumedAscii | 64 | 5.8 | 6.5 | 391.3 | no |
| 100 | mixed | ascii-checked | NotAscii | 64 | 19.5 | 21.4 | 123.6 | no |
| 100 | mixed | utf8 | Utf8 | 64 | 42.3 | 46.2 | 922.3 | no |
| 300 | ascii | none | Unchecked | 64 | 5.8 | 7.1 | 844.7 | no |
| 300 | ascii | ascii | AssumedAscii | 64 | 5.8 | 7.8 | 216.1 | no |
| 300 | ascii | ascii-checked | Ascii | 64 | 47.5 | 54.0 | 777.5 | no |
| 300 | ascii | utf8 | Utf8 | 64 | 47.5 | 54.6 | 603.1 | no |
| 300 | mixed | none | Unchecked | 64 | 5.8 | 10.4 | 286.5 | no |
| 300 | mixed | ascii | AssumedAscii | 64 | 5.8 | 10.4 | 218.7 | no |
| 300 | mixed | ascii-checked | NotAscii | 64 | 47.5 | 55.9 | 537.1 | no |
| 300 | mixed | utf8 | Utf8 | 64 | 98.9 | 125.0 | 988.7 | no |
| 4096 | ascii | none | Unchecked | 64 | 9.7 | 33.2 | 944.7 | no |
| 4096 | ascii | ascii | AssumedAscii | 64 | 7.8 | 36.4 | 685.1 | no |
| 4096 | ascii | ascii-checked | Ascii | 64 | 574.3 | 776.3 | 16371.1 | no |
| 4096 | ascii | utf8 | Utf8 | 64 | 574.3 | 800.7 | 2998.3 | no |
| 4096 | mixed | none | Unchecked | 64 | 9.1 | 31.9 | 609.5 | no |
| 4096 | mixed | ascii | AssumedAscii | 64 | 7.8 | 35.8 | 777.5 | no |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 573.5 | 754.7 | 3019.1 | no |
| 4096 | mixed | utf8 | Utf8 | 64 | 1167.9 | 1522.3 | 8883.1 | no |
| 16384 | ascii | none | Unchecked | 16 | 10.4 | 46.8 | 2404.7 | no |
| 16384 | ascii | ascii | AssumedAscii | 16 | 10.4 | 46.8 | 479.1 | no |
| 16384 | ascii | ascii-checked | Ascii | 16 | 2268.7 | 2783.9 | 9145.5 | no |
| 16384 | ascii | utf8 | Utf8 | 16 | 2279.9 | 2790.3 | 9151.9 | no |
| 16384 | mixed | none | Unchecked | 16 | 10.4 | 46.8 | 3401.5 | no |
| 16384 | mixed | ascii | AssumedAscii | 16 | 10.4 | 46.8 | 526.3 | no |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 2279.9 | 2657.5 | 72396.7 | no |
| 16384 | mixed | utf8 | Utf8 | 16 | 4620.7 | 5526.3 | 230809.5 | no |
