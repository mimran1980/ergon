counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=text
samples=20000

| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 5.8 | 7.1 | 84.6 | no |
| 100 | ascii | ascii | AssumedAscii | 64 | 5.8 | 7.8 | 223.3 | no |
| 100 | ascii | ascii-checked | Ascii | 64 | 20.1 | 24.7 | 713.5 | no |
| 100 | ascii | utf8 | Utf8 | 64 | 19.5 | 24.0 | 501.9 | no |
| 100 | mixed | none | Unchecked | 64 | 5.8 | 9.1 | 378.3 | no |
| 100 | mixed | ascii | AssumedAscii | 64 | 6.5 | 9.1 | 270.9 | no |
| 100 | mixed | ascii-checked | NotAscii | 64 | 20.1 | 25.3 | 3491.1 | no |
| 100 | mixed | utf8 | Utf8 | 64 | 42.9 | 85.9 | 780.7 | no |
| 300 | ascii | none | Unchecked | 64 | 5.8 | 7.1 | 358.1 | no |
| 300 | ascii | ascii | AssumedAscii | 64 | 5.8 | 7.1 | 301.5 | no |
| 300 | ascii | ascii-checked | Ascii | 64 | 47.5 | 53.3 | 554.7 | yes |
| 300 | ascii | utf8 | Utf8 | 64 | 47.5 | 54.0 | 1121.5 | yes |
| 300 | mixed | none | Unchecked | 64 | 5.8 | 7.1 | 472.7 | no |
| 300 | mixed | ascii | AssumedAscii | 64 | 5.8 | 7.1 | 302.7 | no |
| 300 | mixed | ascii-checked | NotAscii | 64 | 47.5 | 54.0 | 718.7 | no |
| 300 | mixed | utf8 | Utf8 | 64 | 98.9 | 112.6 | 548.3 | no |
| 4096 | ascii | none | Unchecked | 64 | 9.7 | 36.4 | 1185.5 | no |
| 4096 | ascii | ascii | AssumedAscii | 64 | 7.8 | 37.1 | 4876.7 | no |
| 4096 | ascii | ascii-checked | Ascii | 64 | 571.9 | 720.3 | 2297.5 | no |
| 4096 | ascii | utf8 | Utf8 | 64 | 574.3 | 710.3 | 1644.7 | no |
| 4096 | mixed | none | Unchecked | 64 | 9.7 | 39.0 | 671.5 | no |
| 4096 | mixed | ascii | AssumedAscii | 64 | 7.8 | 39.0 | 723.5 | no |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 573.5 | 727.5 | 2036.7 | no |
| 4096 | mixed | utf8 | Utf8 | 64 | 1167.9 | 1475.9 | 3676.7 | yes |
| 16384 | ascii | none | Unchecked | 16 | 10.4 | 46.8 | 65023.9 | no |
| 16384 | ascii | ascii | AssumedAscii | 16 | 10.4 | 49.4 | 4044.7 | no |
| 16384 | ascii | ascii-checked | Ascii | 16 | 2260.7 | 2982.3 | 43340.7 | no |
| 16384 | ascii | utf8 | Utf8 | 16 | 2279.9 | 2993.5 | 16780.7 | no |
| 16384 | mixed | none | Unchecked | 16 | 10.4 | 46.8 | 1447.9 | no |
| 16384 | mixed | ascii | AssumedAscii | 16 | 10.4 | 49.4 | 1297.5 | no |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 2273.5 | 2774.3 | 149811.1 | no |
| 16384 | mixed | utf8 | Utf8 | 16 | 4620.7 | 5836.7 | 132607.9 | no |
