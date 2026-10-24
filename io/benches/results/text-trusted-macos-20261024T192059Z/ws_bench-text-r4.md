counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=text
samples=20000

| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 5.8 | 7.1 | 281.3 | n/a |
| 100 | ascii | ascii | AssumedAscii | 64 | 5.8 | 7.1 | 86.5 | n/a |
| 100 | ascii | ascii-checked | Ascii | 64 | 9.1 | 10.4 | 172.5 | n/a |
| 100 | ascii | utf8 | Utf8 | 64 | 9.1 | 10.4 | 328.7 | n/a |
| 100 | mixed | none | Unchecked | 64 | 5.8 | 7.1 | 184.2 | n/a |
| 100 | mixed | ascii | AssumedAscii | 64 | 5.8 | 7.1 | 188.1 | n/a |
| 100 | mixed | ascii-checked | NotAscii | 64 | 9.1 | 10.4 | 286.5 | n/a |
| 100 | mixed | utf8 | Utf8 | 64 | 31.2 | 35.8 | 345.1 | n/a |
| 300 | ascii | none | Unchecked | 64 | 6.5 | 8.4 | 515.1 | n/a |
| 300 | ascii | ascii | AssumedAscii | 64 | 7.1 | 9.1 | 213.5 | n/a |
| 300 | ascii | ascii-checked | Ascii | 64 | 11.0 | 13.6 | 597.9 | n/a |
| 300 | ascii | utf8 | Utf8 | 64 | 10.4 | 19.5 | 735.9 | n/a |
| 300 | mixed | none | Unchecked | 64 | 5.8 | 7.1 | 241.5 | n/a |
| 300 | mixed | ascii | AssumedAscii | 64 | 6.5 | 7.8 | 199.2 | n/a |
| 300 | mixed | ascii-checked | NotAscii | 64 | 11.0 | 12.3 | 404.9 | n/a |
| 300 | mixed | utf8 | Utf8 | 64 | 63.8 | 71.6 | 839.9 | n/a |
| 4096 | ascii | none | Unchecked | 64 | 7.8 | 9.7 | 153.6 | n/a |
| 4096 | ascii | ascii | AssumedAscii | 64 | 7.8 | 9.7 | 298.1 | n/a |
| 4096 | ascii | ascii-checked | Ascii | 64 | 52.7 | 58.5 | 618.7 | n/a |
| 4096 | ascii | utf8 | Utf8 | 64 | 52.7 | 58.5 | 472.3 | n/a |
| 4096 | mixed | none | Unchecked | 64 | 7.8 | 9.1 | 271.5 | n/a |
| 4096 | mixed | ascii | AssumedAscii | 64 | 7.8 | 9.1 | 166.6 | n/a |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 51.4 | 55.3 | 1951.9 | n/a |
| 4096 | mixed | utf8 | Utf8 | 64 | 634.3 | 692.7 | 1328.7 | n/a |
| 16384 | ascii | none | Unchecked | 16 | 10.4 | 13.0 | 554.7 | n/a |
| 16384 | ascii | ascii | AssumedAscii | 16 | 10.4 | 13.0 | 70.3 | n/a |
| 16384 | ascii | ascii-checked | Ascii | 16 | 200.5 | 216.1 | 1122.3 | n/a |
| 16384 | ascii | utf8 | Utf8 | 16 | 200.5 | 216.1 | 1099.1 | n/a |
| 16384 | mixed | none | Unchecked | 16 | 10.4 | 13.0 | 661.5 | n/a |
| 16384 | mixed | ascii | AssumedAscii | 16 | 10.4 | 13.0 | 544.3 | n/a |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 197.9 | 210.9 | 958.3 | n/a |
| 16384 | mixed | utf8 | Utf8 | 16 | 2547.1 | 2889.5 | 4115.1 | n/a |
