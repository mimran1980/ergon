counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=text
samples=20000

| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 5.8 | 10.4 | 361.9 | no |
| 100 | ascii | ascii | AssumedAscii | 64 | 5.8 | 10.4 | 473.9 | no |
| 100 | ascii | ascii-checked | Ascii | 64 | 20.1 | 26.6 | 451.1 | no |
| 100 | ascii | utf8 | Utf8 | 64 | 19.5 | 26.6 | 472.3 | no |
| 100 | mixed | none | Unchecked | 64 | 5.8 | 6.5 | 96.3 | no |
| 100 | mixed | ascii | AssumedAscii | 64 | 5.8 | 7.1 | 118.4 | no |
| 100 | mixed | ascii-checked | NotAscii | 64 | 19.5 | 22.1 | 287.1 | no |
| 100 | mixed | utf8 | Utf8 | 64 | 42.3 | 47.5 | 255.3 | no |
| 300 | ascii | none | Unchecked | 64 | 5.8 | 9.7 | 253.3 | no |
| 300 | ascii | ascii | AssumedAscii | 64 | 5.8 | 9.7 | 119.1 | no |
| 300 | ascii | ascii-checked | Ascii | 64 | 47.5 | 55.3 | 1133.5 | no |
| 300 | ascii | utf8 | Utf8 | 64 | 47.5 | 55.3 | 1255.1 | no |
| 300 | mixed | none | Unchecked | 64 | 5.8 | 11.0 | 301.5 | no |
| 300 | mixed | ascii | AssumedAscii | 64 | 5.8 | 11.0 | 725.9 | no |
| 300 | mixed | ascii-checked | NotAscii | 64 | 48.1 | 55.9 | 1108.7 | no |
| 300 | mixed | utf8 | Utf8 | 64 | 98.9 | 125.6 | 1093.5 | no |
| 4096 | ascii | none | Unchecked | 64 | 7.8 | 35.8 | 1038.3 | no |
| 4096 | ascii | ascii | AssumedAscii | 64 | 7.8 | 34.5 | 433.1 | no |
| 4096 | ascii | ascii-checked | Ascii | 64 | 574.3 | 682.3 | 3779.1 | no |
| 4096 | ascii | utf8 | Utf8 | 64 | 574.3 | 680.3 | 14591.9 | no |
| 4096 | mixed | none | Unchecked | 64 | 9.7 | 39.0 | 3350.3 | no |
| 4096 | mixed | ascii | AssumedAscii | 64 | 7.8 | 37.1 | 964.7 | no |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 576.3 | 795.1 | 9964.7 | no |
| 4096 | mixed | utf8 | Utf8 | 64 | 1169.5 | 1626.3 | 19327.9 | no |
| 16384 | ascii | none | Unchecked | 16 | 10.4 | 57.2 | 2331.1 | no |
| 16384 | ascii | ascii | AssumedAscii | 16 | 10.4 | 59.8 | 1484.7 | no |
| 16384 | ascii | ascii-checked | Ascii | 16 | 2267.1 | 3223.9 | 103270.3 | no |
| 16384 | ascii | utf8 | Utf8 | 16 | 2279.9 | 3247.9 | 17087.9 | no |
| 16384 | mixed | none | Unchecked | 16 | 10.4 | 59.8 | 5839.9 | no |
| 16384 | mixed | ascii | AssumedAscii | 16 | 10.4 | 65.1 | 6854.3 | no |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 2281.5 | 3644.7 | 20787.1 | no |
| 16384 | mixed | utf8 | Utf8 | 16 | 4630.3 | 7270.3 | 119091.1 | no |
