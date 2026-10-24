counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=text
samples=20000

| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 6.5 | 18.2 | 1831.9 | n/a |
| 100 | ascii | ascii | AssumedAscii | 64 | 6.5 | 18.2 | 3475.1 | n/a |
| 100 | ascii | ascii-checked | Ascii | 64 | 9.1 | 27.9 | 4006.3 | n/a |
| 100 | ascii | utf8 | Utf8 | 64 | 9.1 | 27.3 | 157593.5 | n/a |
| 100 | mixed | none | Unchecked | 64 | 5.8 | 7.1 | 182.2 | n/a |
| 100 | mixed | ascii | AssumedAscii | 64 | 5.8 | 7.1 | 52.7 | n/a |
| 100 | mixed | ascii-checked | NotAscii | 64 | 9.1 | 10.4 | 141.2 | n/a |
| 100 | mixed | utf8 | Utf8 | 64 | 31.2 | 36.4 | 347.7 | n/a |
| 300 | ascii | none | Unchecked | 64 | 6.5 | 7.1 | 87.8 | n/a |
| 300 | ascii | ascii | AssumedAscii | 64 | 6.5 | 7.8 | 283.9 | n/a |
| 300 | ascii | ascii-checked | Ascii | 64 | 10.4 | 12.3 | 235.1 | n/a |
| 300 | ascii | utf8 | Utf8 | 64 | 10.4 | 11.7 | 238.9 | n/a |
| 300 | mixed | none | Unchecked | 64 | 5.8 | 7.1 | 343.1 | n/a |
| 300 | mixed | ascii | AssumedAscii | 64 | 6.5 | 7.8 | 261.7 | n/a |
| 300 | mixed | ascii-checked | NotAscii | 64 | 11.0 | 12.3 | 556.7 | n/a |
| 300 | mixed | utf8 | Utf8 | 64 | 63.8 | 71.6 | 513.9 | n/a |
| 4096 | ascii | none | Unchecked | 64 | 7.8 | 9.7 | 269.5 | n/a |
| 4096 | ascii | ascii | AssumedAscii | 64 | 8.4 | 9.7 | 164.7 | n/a |
| 4096 | ascii | ascii-checked | Ascii | 64 | 53.3 | 57.9 | 13913.5 | n/a |
| 4096 | ascii | utf8 | Utf8 | 64 | 53.3 | 57.9 | 2388.7 | n/a |
| 4096 | mixed | none | Unchecked | 64 | 7.8 | 9.7 | 109.3 | n/a |
| 4096 | mixed | ascii | AssumedAscii | 64 | 7.8 | 9.7 | 91.1 | n/a |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 52.0 | 55.9 | 373.1 | n/a |
| 4096 | mixed | utf8 | Utf8 | 64 | 634.3 | 705.9 | 1542.3 | n/a |
| 16384 | ascii | none | Unchecked | 16 | 10.4 | 13.0 | 1039.1 | n/a |
| 16384 | ascii | ascii | AssumedAscii | 16 | 10.4 | 13.0 | 148.4 | n/a |
| 16384 | ascii | ascii-checked | Ascii | 16 | 200.5 | 223.9 | 3078.3 | n/a |
| 16384 | ascii | utf8 | Utf8 | 16 | 200.5 | 223.9 | 6675.1 | n/a |
| 16384 | mixed | none | Unchecked | 16 | 10.4 | 13.0 | 461.1 | n/a |
| 16384 | mixed | ascii | AssumedAscii | 16 | 10.4 | 13.0 | 1213.5 | n/a |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 197.9 | 216.1 | 2084.7 | n/a |
| 16384 | mixed | utf8 | Utf8 | 16 | 2547.1 | 2873.5 | 46028.7 | n/a |
