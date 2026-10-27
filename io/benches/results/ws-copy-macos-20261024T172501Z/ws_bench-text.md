counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=text
samples=20000

| size B | text | check | delivered as | K | ns/message p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 5.8 | 7.8 | 2631.9 | n/a |
| 100 | ascii | utf8 | Utf8 | 64 | 9.1 | 11.7 | 2249.5 | n/a |
| 100 | ascii | ascii | Ascii | 64 | 9.1 | 11.7 | 678.3 | n/a |
| 100 | mixed | none | Unchecked | 64 | 5.8 | 7.1 | 298.9 | n/a |
| 100 | mixed | utf8 | Utf8 | 64 | 33.2 | 36.4 | 1115.9 | n/a |
| 100 | mixed | ascii | NotAscii | 64 | 9.7 | 11.7 | 605.1 | n/a |
| 300 | ascii | none | Unchecked | 64 | 5.8 | 7.1 | 171.8 | n/a |
| 300 | ascii | utf8 | Utf8 | 64 | 10.4 | 17.5 | 443.5 | n/a |
| 300 | ascii | ascii | Ascii | 64 | 10.4 | 20.1 | 220.1 | n/a |
| 300 | mixed | none | Unchecked | 64 | 5.8 | 7.8 | 299.5 | n/a |
| 300 | mixed | utf8 | Utf8 | 64 | 67.7 | 73.5 | 54527.9 | n/a |
| 300 | mixed | ascii | NotAscii | 64 | 11.0 | 13.6 | 2350.3 | n/a |
| 4096 | ascii | none | Unchecked | 64 | 7.8 | 10.4 | 67.0 | n/a |
| 4096 | ascii | utf8 | Utf8 | 64 | 54.6 | 59.8 | 733.9 | n/a |
| 4096 | ascii | ascii | Ascii | 64 | 54.6 | 59.8 | 653.1 | n/a |
| 4096 | mixed | none | Unchecked | 64 | 8.4 | 12.3 | 367.9 | n/a |
| 4096 | mixed | utf8 | Utf8 | 64 | 674.7 | 812.7 | 2366.3 | n/a |
| 4096 | mixed | ascii | NotAscii | 64 | 54.6 | 68.3 | 581.5 | n/a |
| 16384 | ascii | none | Unchecked | 16 | 10.4 | 15.6 | 187.5 | n/a |
| 16384 | ascii | utf8 | Utf8 | 16 | 208.3 | 229.1 | 2388.7 | n/a |
| 16384 | ascii | ascii | Ascii | 16 | 208.3 | 229.1 | 2542.3 | n/a |
| 16384 | mixed | none | Unchecked | 16 | 10.4 | 70.3 | 3756.7 | n/a |
| 16384 | mixed | utf8 | Utf8 | 16 | 2711.9 | 3667.1 | 24550.3 | n/a |
| 16384 | mixed | ascii | NotAscii | 16 | 208.3 | 294.3 | 3107.1 | n/a |
