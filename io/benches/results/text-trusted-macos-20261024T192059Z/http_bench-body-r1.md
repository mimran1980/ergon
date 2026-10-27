counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=body
samples=20000

| body B | text | check | delivered as | K | ns/response p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 123.0 | 137.3 | 4790.3 | n/a |
| 100 | ascii | ascii | AssumedAscii | 64 | 123.0 | 138.0 | 1386.3 | n/a |
| 100 | ascii | ascii-checked | Ascii | 64 | 126.3 | 141.9 | 1336.7 | n/a |
| 100 | ascii | utf8 | Utf8 | 64 | 128.2 | 142.5 | 8595.1 | n/a |
| 100 | mixed | none | Unchecked | 64 | 121.7 | 143.8 | 1381.5 | n/a |
| 100 | mixed | ascii | AssumedAscii | 64 | 122.3 | 143.8 | 2212.7 | n/a |
| 100 | mixed | ascii-checked | NotAscii | 64 | 125.6 | 148.4 | 1836.7 | n/a |
| 100 | mixed | utf8 | Utf8 | 64 | 144.5 | 175.7 | 6499.1 | n/a |
| 300 | ascii | none | Unchecked | 64 | 121.7 | 140.6 | 12742.3 | n/a |
| 300 | ascii | ascii | AssumedAscii | 64 | 122.3 | 141.9 | 2665.5 | n/a |
| 300 | ascii | ascii-checked | Ascii | 64 | 126.9 | 145.8 | 1734.3 | n/a |
| 300 | ascii | utf8 | Utf8 | 64 | 133.4 | 154.2 | 3679.9 | n/a |
| 300 | mixed | none | Unchecked | 64 | 123.0 | 141.9 | 1027.9 | n/a |
| 300 | mixed | ascii | AssumedAscii | 64 | 123.6 | 141.9 | 1171.9 | n/a |
| 300 | mixed | ascii-checked | NotAscii | 64 | 127.6 | 147.7 | 882.3 | n/a |
| 300 | mixed | utf8 | Utf8 | 64 | 179.6 | 228.5 | 1342.3 | n/a |
| 4096 | ascii | none | Unchecked | 64 | 125.0 | 141.2 | 956.7 | n/a |
| 4096 | ascii | ascii | AssumedAscii | 64 | 123.6 | 139.3 | 1179.1 | n/a |
| 4096 | ascii | ascii-checked | Ascii | 64 | 162.7 | 183.5 | 1678.3 | n/a |
| 4096 | ascii | utf8 | Utf8 | 64 | 255.9 | 312.5 | 1619.9 | n/a |
| 4096 | mixed | none | Unchecked | 64 | 125.6 | 142.5 | 1999.9 | n/a |
| 4096 | mixed | ascii | AssumedAscii | 64 | 124.3 | 141.2 | 1573.5 | n/a |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 163.4 | 187.5 | 789.1 | n/a |
| 4096 | mixed | utf8 | Utf8 | 64 | 675.1 | 896.7 | 6185.5 | n/a |
| 16384 | ascii | none | Unchecked | 16 | 200.5 | 398.5 | 152166.3 | n/a |
| 16384 | ascii | ascii | AssumedAscii | 16 | 203.1 | 380.3 | 1134591.9 | n/a |
| 16384 | ascii | ascii-checked | Ascii | 16 | 361.9 | 755.5 | 575487.9 | n/a |
| 16384 | ascii | utf8 | Utf8 | 16 | 721.5 | 1633.5 | 303308.7 | n/a |
| 16384 | mixed | none | Unchecked | 16 | 200.5 | 239.5 | 6617.5 | n/a |
| 16384 | mixed | ascii | AssumedAscii | 16 | 203.1 | 242.1 | 115916.7 | n/a |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 361.9 | 421.9 | 7545.5 | n/a |
| 16384 | mixed | utf8 | Utf8 | 16 | 2239.9 | 3078.3 | 593510.3 | n/a |
