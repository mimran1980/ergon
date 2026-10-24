counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=text
samples=20000

| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 5.8 | 7.1 | 313.1 | n/a |
| 100 | ascii | ascii | AssumedAscii | 64 | 5.8 | 7.1 | 249.3 | n/a |
| 100 | ascii | ascii-checked | Ascii | 64 | 9.1 | 10.4 | 337.9 | n/a |
| 100 | ascii | utf8 | Utf8 | 64 | 9.1 | 10.4 | 582.3 | n/a |
| 100 | mixed | none | Unchecked | 64 | 5.8 | 7.1 | 221.3 | n/a |
| 100 | mixed | ascii | AssumedAscii | 64 | 5.8 | 7.1 | 216.1 | n/a |
| 100 | mixed | ascii-checked | NotAscii | 64 | 9.1 | 11.0 | 193.3 | n/a |
| 100 | mixed | utf8 | Utf8 | 64 | 31.9 | 36.4 | 400.3 | n/a |
| 300 | ascii | none | Unchecked | 64 | 6.5 | 7.1 | 432.3 | n/a |
| 300 | ascii | ascii | AssumedAscii | 64 | 6.5 | 7.8 | 373.1 | n/a |
| 300 | ascii | ascii-checked | Ascii | 64 | 10.4 | 12.3 | 343.7 | n/a |
| 300 | ascii | utf8 | Utf8 | 64 | 10.4 | 13.0 | 389.3 | n/a |
| 300 | mixed | none | Unchecked | 64 | 6.5 | 7.8 | 681.9 | n/a |
| 300 | mixed | ascii | AssumedAscii | 64 | 6.5 | 7.8 | 248.1 | n/a |
| 300 | mixed | ascii-checked | NotAscii | 64 | 11.0 | 13.0 | 785.1 | n/a |
| 300 | mixed | utf8 | Utf8 | 64 | 63.8 | 72.9 | 1083.9 | n/a |
| 4096 | ascii | none | Unchecked | 64 | 7.8 | 9.7 | 197.9 | n/a |
| 4096 | ascii | ascii | AssumedAscii | 64 | 7.8 | 9.7 | 145.1 | n/a |
| 4096 | ascii | ascii-checked | Ascii | 64 | 52.7 | 56.6 | 322.9 | n/a |
| 4096 | ascii | utf8 | Utf8 | 64 | 52.7 | 57.2 | 511.9 | n/a |
| 4096 | mixed | none | Unchecked | 64 | 7.8 | 9.7 | 79.4 | n/a |
| 4096 | mixed | ascii | AssumedAscii | 64 | 7.8 | 9.7 | 179.6 | n/a |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 52.0 | 57.2 | 503.5 | n/a |
| 4096 | mixed | utf8 | Utf8 | 64 | 634.3 | 706.3 | 2423.9 | n/a |
| 16384 | ascii | none | Unchecked | 16 | 10.4 | 10.4 | 143.2 | n/a |
| 16384 | ascii | ascii | AssumedAscii | 16 | 10.4 | 13.0 | 151.0 | n/a |
| 16384 | ascii | ascii-checked | Ascii | 16 | 200.5 | 216.1 | 1758.3 | n/a |
| 16384 | ascii | utf8 | Utf8 | 16 | 200.5 | 216.1 | 3271.9 | n/a |
| 16384 | mixed | none | Unchecked | 16 | 10.4 | 13.0 | 656.3 | n/a |
| 16384 | mixed | ascii | AssumedAscii | 16 | 10.4 | 13.0 | 78.1 | n/a |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 197.9 | 210.9 | 1104.7 | n/a |
| 16384 | mixed | utf8 | Utf8 | 16 | 2550.3 | 2756.7 | 7455.9 | n/a |
