counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=body
samples=20000

| body B | text | check | delivered as | K | ns/response p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 125.6 | 309.9 | 10438.3 | n/a |
| 100 | ascii | ascii | AssumedAscii | 64 | 126.3 | 332.7 | 215039.9 | n/a |
| 100 | ascii | ascii-checked | Ascii | 64 | 128.9 | 320.9 | 18406.3 | n/a |
| 100 | ascii | utf8 | Utf8 | 64 | 130.2 | 322.9 | 22412.7 | n/a |
| 100 | mixed | none | Unchecked | 64 | 122.3 | 140.6 | 13516.7 | n/a |
| 100 | mixed | ascii | AssumedAscii | 64 | 123.0 | 140.6 | 2278.3 | n/a |
| 100 | mixed | ascii-checked | NotAscii | 64 | 125.6 | 145.1 | 747.5 | n/a |
| 100 | mixed | utf8 | Utf8 | 64 | 145.1 | 167.3 | 1833.5 | n/a |
| 300 | ascii | none | Unchecked | 64 | 125.6 | 182.2 | 858.3 | n/a |
| 300 | ascii | ascii | AssumedAscii | 64 | 126.3 | 156.9 | 619.9 | n/a |
| 300 | ascii | ascii-checked | Ascii | 64 | 131.5 | 181.6 | 1000.7 | n/a |
| 300 | ascii | utf8 | Utf8 | 64 | 138.6 | 196.6 | 1336.7 | n/a |
| 300 | mixed | none | Unchecked | 64 | 121.7 | 139.3 | 993.5 | n/a |
| 300 | mixed | ascii | AssumedAscii | 64 | 122.3 | 139.3 | 802.7 | n/a |
| 300 | mixed | ascii-checked | NotAscii | 64 | 126.3 | 146.4 | 1727.9 | n/a |
| 300 | mixed | utf8 | Utf8 | 64 | 177.7 | 219.5 | 1737.5 | n/a |
| 4096 | ascii | none | Unchecked | 64 | 125.6 | 144.5 | 9062.3 | n/a |
| 4096 | ascii | ascii | AssumedAscii | 64 | 124.3 | 144.5 | 1507.1 | n/a |
| 4096 | ascii | ascii-checked | Ascii | 64 | 163.4 | 206.3 | 1756.7 | n/a |
| 4096 | ascii | utf8 | Utf8 | 64 | 256.5 | 389.9 | 1539.9 | n/a |
| 4096 | mixed | none | Unchecked | 64 | 125.6 | 143.8 | 3487.9 | n/a |
| 4096 | mixed | ascii | AssumedAscii | 64 | 124.3 | 144.5 | 1692.7 | n/a |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 163.4 | 195.9 | 2713.5 | n/a |
| 4096 | mixed | utf8 | Utf8 | 64 | 675.9 | 961.5 | 23359.9 | n/a |
| 16384 | ascii | none | Unchecked | 16 | 200.5 | 229.1 | 2276.7 | n/a |
| 16384 | ascii | ascii | AssumedAscii | 16 | 200.5 | 226.5 | 2833.5 | n/a |
| 16384 | ascii | ascii-checked | Ascii | 16 | 359.3 | 398.5 | 2707.1 | n/a |
| 16384 | ascii | utf8 | Utf8 | 16 | 708.3 | 786.7 | 3638.3 | n/a |
| 16384 | mixed | none | Unchecked | 16 | 200.5 | 239.5 | 15756.7 | n/a |
| 16384 | mixed | ascii | AssumedAscii | 16 | 200.5 | 239.5 | 5174.3 | n/a |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 359.3 | 419.5 | 11903.9 | n/a |
| 16384 | mixed | utf8 | Utf8 | 16 | 2235.1 | 3484.7 | 88422.3 | n/a |
