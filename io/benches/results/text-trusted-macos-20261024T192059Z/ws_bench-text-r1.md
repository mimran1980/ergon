counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=text
samples=20000

| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 6.5 | 8.4 | 562.7 | n/a |
| 100 | ascii | ascii | AssumedAscii | 64 | 6.5 | 8.4 | 72.9 | n/a |
| 100 | ascii | ascii-checked | Ascii | 64 | 9.1 | 12.3 | 567.1 | n/a |
| 100 | ascii | utf8 | Utf8 | 64 | 9.1 | 12.3 | 404.3 | n/a |
| 100 | mixed | none | Unchecked | 64 | 5.8 | 7.1 | 127.6 | n/a |
| 100 | mixed | ascii | AssumedAscii | 64 | 6.5 | 7.1 | 155.5 | n/a |
| 100 | mixed | ascii-checked | NotAscii | 64 | 9.1 | 10.4 | 246.1 | n/a |
| 100 | mixed | utf8 | Utf8 | 64 | 31.2 | 36.4 | 312.5 | n/a |
| 300 | ascii | none | Unchecked | 64 | 6.5 | 7.8 | 590.7 | n/a |
| 300 | ascii | ascii | AssumedAscii | 64 | 6.5 | 8.4 | 107.4 | n/a |
| 300 | ascii | ascii-checked | Ascii | 64 | 11.0 | 12.3 | 321.7 | n/a |
| 300 | ascii | utf8 | Utf8 | 64 | 10.4 | 13.0 | 236.3 | n/a |
| 300 | mixed | none | Unchecked | 64 | 6.5 | 7.1 | 233.1 | n/a |
| 300 | mixed | ascii | AssumedAscii | 64 | 6.5 | 7.8 | 608.3 | n/a |
| 300 | mixed | ascii-checked | NotAscii | 64 | 11.0 | 13.0 | 527.5 | n/a |
| 300 | mixed | utf8 | Utf8 | 64 | 63.8 | 72.2 | 685.1 | n/a |
| 4096 | ascii | none | Unchecked | 64 | 7.8 | 9.7 | 731.1 | n/a |
| 4096 | ascii | ascii | AssumedAscii | 64 | 7.8 | 9.7 | 179.0 | n/a |
| 4096 | ascii | ascii-checked | Ascii | 64 | 52.7 | 58.5 | 662.7 | n/a |
| 4096 | ascii | utf8 | Utf8 | 64 | 52.7 | 58.5 | 838.3 | n/a |
| 4096 | mixed | none | Unchecked | 64 | 7.8 | 9.7 | 107.4 | n/a |
| 4096 | mixed | ascii | AssumedAscii | 64 | 7.8 | 9.7 | 302.7 | n/a |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 52.7 | 56.6 | 2196.7 | n/a |
| 4096 | mixed | utf8 | Utf8 | 64 | 634.3 | 712.3 | 1854.3 | n/a |
| 16384 | ascii | none | Unchecked | 16 | 10.4 | 13.0 | 101.5 | n/a |
| 16384 | ascii | ascii | AssumedAscii | 16 | 10.4 | 13.0 | 427.1 | n/a |
| 16384 | ascii | ascii-checked | Ascii | 16 | 200.5 | 216.1 | 1208.7 | n/a |
| 16384 | ascii | utf8 | Utf8 | 16 | 200.5 | 218.7 | 1466.3 | n/a |
| 16384 | mixed | none | Unchecked | 16 | 10.4 | 13.0 | 755.5 | n/a |
| 16384 | mixed | ascii | AssumedAscii | 16 | 10.4 | 13.0 | 401.1 | n/a |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 197.9 | 213.5 | 1515.9 | n/a |
| 16384 | mixed | utf8 | Utf8 | 16 | 2547.1 | 2953.5 | 8716.7 | n/a |
