counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=text
samples=20000

| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 5.8 | 10.4 | 91.7 | no |
| 100 | ascii | utf8 | Utf8 | 64 | 20.8 | 26.6 | 982.3 | no |
| 100 | ascii | ascii | Ascii | 64 | 20.8 | 26.0 | 886.3 | no |
| 100 | mixed | none | Unchecked | 64 | 5.8 | 9.1 | 435.1 | no |
| 100 | mixed | utf8 | Utf8 | 64 | 44.9 | 56.6 | 657.5 | no |
| 100 | mixed | ascii | NotAscii | 64 | 21.4 | 29.2 | 2462.3 | yes |
| 300 | ascii | none | Unchecked | 64 | 5.8 | 24.0 | 3756.7 | no |
| 300 | ascii | utf8 | Utf8 | 64 | 50.1 | 60.5 | 9158.3 | no |
| 300 | ascii | ascii | Ascii | 64 | 50.1 | 60.5 | 127795.1 | no |
| 300 | mixed | none | Unchecked | 64 | 5.8 | 13.0 | 263.1 | no |
| 300 | mixed | utf8 | Utf8 | 64 | 105.4 | 141.2 | 1282.3 | no |
| 300 | mixed | ascii | NotAscii | 64 | 50.7 | 57.9 | 2347.1 | yes |
| 4096 | ascii | none | Unchecked | 64 | 26.6 | 74.2 | 19135.9 | no |
| 4096 | ascii | utf8 | Utf8 | 64 | 619.1 | 1187.1 | 155238.3 | no |
| 4096 | ascii | ascii | Ascii | 64 | 618.7 | 1079.1 | 59801.5 | no |
| 4096 | mixed | none | Unchecked | 64 | 26.6 | 84.6 | 177561.5 | no |
| 4096 | mixed | utf8 | Utf8 | 64 | 1276.7 | 2863.9 | 178278.3 | no |
| 4096 | mixed | ascii | NotAscii | 64 | 619.9 | 1188.7 | 69836.7 | no |
| 16384 | ascii | none | Unchecked | 16 | 39.0 | 174.4 | 12063.9 | no |
| 16384 | ascii | utf8 | Utf8 | 16 | 2455.9 | 5753.5 | 136396.7 | no |
| 16384 | ascii | ascii | Ascii | 16 | 2455.9 | 6499.1 | 132095.9 | no |
| 16384 | mixed | none | Unchecked | 16 | 31.2 | 78.1 | 111923.1 | no |
| 16384 | mixed | utf8 | Utf8 | 16 | 4982.3 | 9939.1 | 247398.3 | no |
| 16384 | mixed | ascii | NotAscii | 16 | 2435.1 | 4406.3 | 158207.9 | no |
