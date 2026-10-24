counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=text
samples=20000

| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 5.8 | 7.1 | 143.8 | no |
| 100 | ascii | ascii | AssumedAscii | 64 | 5.8 | 7.1 | 240.3 | no |
| 100 | ascii | ascii-checked | Ascii | 64 | 20.8 | 24.0 | 431.1 | no |
| 100 | ascii | utf8 | Utf8 | 64 | 20.1 | 23.4 | 445.9 | no |
| 100 | mixed | none | Unchecked | 64 | 5.8 | 6.5 | 185.5 | no |
| 100 | mixed | ascii | AssumedAscii | 64 | 5.8 | 6.5 | 314.5 | no |
| 100 | mixed | ascii-checked | NotAscii | 64 | 19.5 | 22.1 | 382.9 | no |
| 100 | mixed | utf8 | Utf8 | 64 | 42.3 | 47.5 | 1431.1 | no |
| 300 | ascii | none | Unchecked | 64 | 5.8 | 9.1 | 423.1 | no |
| 300 | ascii | ascii | AssumedAscii | 64 | 5.8 | 9.7 | 392.5 | no |
| 300 | ascii | ascii-checked | Ascii | 64 | 47.5 | 54.6 | 839.1 | no |
| 300 | ascii | utf8 | Utf8 | 64 | 47.5 | 54.6 | 1047.9 | no |
| 300 | mixed | none | Unchecked | 64 | 6.5 | 10.4 | 871.1 | no |
| 300 | mixed | ascii | AssumedAscii | 64 | 6.5 | 10.4 | 322.3 | no |
| 300 | mixed | ascii-checked | NotAscii | 64 | 48.1 | 55.9 | 674.7 | no |
| 300 | mixed | utf8 | Utf8 | 64 | 98.9 | 147.1 | 1003.9 | no |
| 4096 | ascii | none | Unchecked | 64 | 7.8 | 37.7 | 1787.1 | no |
| 4096 | ascii | ascii | AssumedAscii | 64 | 7.8 | 29.2 | 1244.7 | no |
| 4096 | ascii | ascii-checked | Ascii | 64 | 575.5 | 791.9 | 5705.5 | no |
| 4096 | ascii | utf8 | Utf8 | 64 | 573.5 | 789.1 | 5388.7 | no |
| 4096 | mixed | none | Unchecked | 64 | 9.7 | 37.7 | 379.5 | no |
| 4096 | mixed | ascii | AssumedAscii | 64 | 7.8 | 29.2 | 679.9 | no |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 573.5 | 713.1 | 5494.3 | no |
| 4096 | mixed | utf8 | Utf8 | 64 | 1167.9 | 1458.3 | 13772.7 | no |
| 16384 | ascii | none | Unchecked | 16 | 10.4 | 46.8 | 385.5 | no |
| 16384 | ascii | ascii | AssumedAscii | 16 | 10.4 | 49.4 | 2881.5 | no |
| 16384 | ascii | ascii-checked | Ascii | 16 | 2267.1 | 2766.3 | 6982.3 | no |
| 16384 | ascii | utf8 | Utf8 | 16 | 2279.9 | 2804.7 | 7020.7 | no |
| 16384 | mixed | none | Unchecked | 16 | 10.4 | 44.2 | 872.7 | no |
| 16384 | mixed | ascii | AssumedAscii | 16 | 10.4 | 46.8 | 416.7 | no |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 2279.9 | 2831.9 | 17702.3 | no |
| 16384 | mixed | utf8 | Utf8 | 16 | 4620.7 | 5708.7 | 20633.5 | no |
