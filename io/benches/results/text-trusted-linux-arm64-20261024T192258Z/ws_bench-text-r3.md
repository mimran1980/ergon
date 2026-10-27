counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=text
samples=20000

| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 5.8 | 7.1 | 247.3 | no |
| 100 | ascii | ascii | AssumedAscii | 64 | 5.8 | 7.8 | 312.5 | no |
| 100 | ascii | ascii-checked | Ascii | 64 | 20.1 | 24.7 | 777.9 | no |
| 100 | ascii | utf8 | Utf8 | 64 | 19.5 | 24.0 | 626.3 | no |
| 100 | mixed | none | Unchecked | 64 | 5.8 | 7.8 | 401.1 | no |
| 100 | mixed | ascii | AssumedAscii | 64 | 5.8 | 7.8 | 96.3 | no |
| 100 | mixed | ascii-checked | NotAscii | 64 | 20.1 | 24.0 | 435.5 | no |
| 100 | mixed | utf8 | Utf8 | 64 | 42.3 | 52.0 | 489.5 | no |
| 300 | ascii | none | Unchecked | 64 | 6.5 | 7.8 | 3747.1 | no |
| 300 | ascii | ascii | AssumedAscii | 64 | 6.5 | 7.8 | 431.1 | no |
| 300 | ascii | ascii-checked | Ascii | 64 | 48.1 | 54.0 | 6649.5 | no |
| 300 | ascii | utf8 | Utf8 | 64 | 47.5 | 54.0 | 849.5 | no |
| 300 | mixed | none | Unchecked | 64 | 6.5 | 12.3 | 1374.3 | no |
| 300 | mixed | ascii | AssumedAscii | 64 | 6.5 | 13.6 | 905.5 | no |
| 300 | mixed | ascii-checked | NotAscii | 64 | 50.7 | 58.5 | 2881.5 | no |
| 300 | mixed | utf8 | Utf8 | 64 | 104.1 | 144.5 | 1955.1 | no |
| 4096 | ascii | none | Unchecked | 64 | 8.4 | 39.7 | 467.5 | no |
| 4096 | ascii | ascii | AssumedAscii | 64 | 7.8 | 29.2 | 872.7 | no |
| 4096 | ascii | ascii-checked | Ascii | 64 | 576.3 | 738.3 | 6351.9 | no |
| 4096 | ascii | utf8 | Utf8 | 64 | 572.3 | 744.3 | 2199.9 | no |
| 4096 | mixed | none | Unchecked | 64 | 9.7 | 39.0 | 2657.5 | no |
| 4096 | mixed | ascii | AssumedAscii | 64 | 7.8 | 29.9 | 744.7 | no |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 577.5 | 734.3 | 28799.9 | no |
| 4096 | mixed | utf8 | Utf8 | 64 | 1167.9 | 1535.1 | 43110.3 | no |
| 16384 | ascii | none | Unchecked | 16 | 10.4 | 54.6 | 1370.3 | no |
| 16384 | ascii | ascii | AssumedAscii | 16 | 10.4 | 57.2 | 2011.1 | no |
| 16384 | ascii | ascii-checked | Ascii | 16 | 2279.9 | 2894.3 | 47206.3 | no |
| 16384 | ascii | utf8 | Utf8 | 16 | 2247.9 | 2894.3 | 12159.9 | no |
| 16384 | mixed | none | Unchecked | 16 | 10.4 | 44.2 | 1088.7 | no |
| 16384 | mixed | ascii | AssumedAscii | 16 | 10.4 | 46.8 | 2422.3 | no |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 2279.9 | 2721.5 | 5702.3 | no |
| 16384 | mixed | utf8 | Utf8 | 16 | 4601.5 | 5583.9 | 11737.5 | no |
