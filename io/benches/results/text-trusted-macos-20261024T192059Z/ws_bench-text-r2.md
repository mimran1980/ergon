counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=text
samples=20000

| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 5.8 | 7.1 | 291.7 | n/a |
| 100 | ascii | ascii | AssumedAscii | 64 | 6.5 | 7.1 | 319.1 | n/a |
| 100 | ascii | ascii-checked | Ascii | 64 | 9.1 | 10.4 | 399.7 | n/a |
| 100 | ascii | utf8 | Utf8 | 64 | 9.1 | 10.4 | 309.9 | n/a |
| 100 | mixed | none | Unchecked | 64 | 5.8 | 7.1 | 617.9 | n/a |
| 100 | mixed | ascii | AssumedAscii | 64 | 5.8 | 7.1 | 339.9 | n/a |
| 100 | mixed | ascii-checked | NotAscii | 64 | 9.1 | 10.4 | 421.5 | n/a |
| 100 | mixed | utf8 | Utf8 | 64 | 31.2 | 36.4 | 417.5 | n/a |
| 300 | ascii | none | Unchecked | 64 | 6.5 | 7.1 | 384.7 | n/a |
| 300 | ascii | ascii | AssumedAscii | 64 | 6.5 | 7.8 | 342.5 | n/a |
| 300 | ascii | ascii-checked | Ascii | 64 | 10.4 | 12.3 | 242.9 | n/a |
| 300 | ascii | utf8 | Utf8 | 64 | 10.4 | 11.7 | 240.9 | n/a |
| 300 | mixed | none | Unchecked | 64 | 5.8 | 7.1 | 324.9 | n/a |
| 300 | mixed | ascii | AssumedAscii | 64 | 6.5 | 7.8 | 111.3 | n/a |
| 300 | mixed | ascii-checked | NotAscii | 64 | 11.0 | 12.3 | 408.9 | n/a |
| 300 | mixed | utf8 | Utf8 | 64 | 63.8 | 72.2 | 666.7 | n/a |
| 4096 | ascii | none | Unchecked | 64 | 7.8 | 9.7 | 260.5 | n/a |
| 4096 | ascii | ascii | AssumedAscii | 64 | 7.8 | 9.7 | 250.7 | n/a |
| 4096 | ascii | ascii-checked | Ascii | 64 | 52.7 | 57.9 | 399.1 | n/a |
| 4096 | ascii | utf8 | Utf8 | 64 | 52.7 | 58.5 | 1248.7 | n/a |
| 4096 | mixed | none | Unchecked | 64 | 7.8 | 10.4 | 1740.7 | n/a |
| 4096 | mixed | ascii | AssumedAscii | 64 | 7.8 | 10.4 | 287.7 | n/a |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 52.0 | 59.8 | 13132.7 | n/a |
| 4096 | mixed | utf8 | Utf8 | 64 | 634.3 | 788.7 | 15475.1 | n/a |
| 16384 | ascii | none | Unchecked | 16 | 10.4 | 13.0 | 963.9 | n/a |
| 16384 | ascii | ascii | AssumedAscii | 16 | 10.4 | 13.0 | 4716.7 | n/a |
| 16384 | ascii | ascii-checked | Ascii | 16 | 203.1 | 223.9 | 71423.9 | n/a |
| 16384 | ascii | utf8 | Utf8 | 16 | 203.1 | 223.9 | 2943.9 | n/a |
| 16384 | mixed | none | Unchecked | 16 | 10.4 | 13.0 | 630.3 | n/a |
| 16384 | mixed | ascii | AssumedAscii | 16 | 10.4 | 13.0 | 870.3 | n/a |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 197.9 | 216.1 | 1570.3 | n/a |
| 16384 | mixed | utf8 | Utf8 | 16 | 2547.1 | 2990.3 | 5497.5 | n/a |
