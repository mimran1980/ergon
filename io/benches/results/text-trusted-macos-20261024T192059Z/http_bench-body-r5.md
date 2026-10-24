counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=body
samples=20000

| body B | text | check | delivered as | K | ns/response p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 121.7 | 136.0 | 651.9 | n/a |
| 100 | ascii | ascii | AssumedAscii | 64 | 122.3 | 136.7 | 983.9 | n/a |
| 100 | ascii | ascii-checked | Ascii | 64 | 124.3 | 139.3 | 629.1 | n/a |
| 100 | ascii | utf8 | Utf8 | 64 | 126.3 | 141.2 | 734.3 | n/a |
| 100 | mixed | none | Unchecked | 64 | 121.7 | 136.0 | 1255.1 | n/a |
| 100 | mixed | ascii | AssumedAscii | 64 | 122.3 | 136.7 | 1093.5 | n/a |
| 100 | mixed | ascii-checked | NotAscii | 64 | 125.0 | 139.9 | 1555.9 | n/a |
| 100 | mixed | utf8 | Utf8 | 64 | 144.5 | 160.8 | 1731.1 | n/a |
| 300 | ascii | none | Unchecked | 64 | 121.7 | 136.7 | 91955.1 | n/a |
| 300 | ascii | ascii | AssumedAscii | 64 | 122.3 | 137.3 | 2782.3 | n/a |
| 300 | ascii | ascii-checked | Ascii | 64 | 126.3 | 141.9 | 864.7 | n/a |
| 300 | ascii | utf8 | Utf8 | 64 | 133.4 | 149.0 | 4339.1 | n/a |
| 300 | mixed | none | Unchecked | 64 | 121.7 | 136.0 | 2081.5 | n/a |
| 300 | mixed | ascii | AssumedAscii | 64 | 122.3 | 137.3 | 2195.1 | n/a |
| 300 | mixed | ascii-checked | NotAscii | 64 | 126.3 | 141.2 | 823.1 | n/a |
| 300 | mixed | utf8 | Utf8 | 64 | 177.7 | 205.7 | 1328.7 | n/a |
| 4096 | ascii | none | Unchecked | 64 | 128.2 | 260.5 | 3945.5 | n/a |
| 4096 | ascii | ascii | AssumedAscii | 64 | 126.9 | 276.7 | 28825.5 | n/a |
| 4096 | ascii | ascii-checked | Ascii | 64 | 167.3 | 343.1 | 29363.1 | n/a |
| 4096 | ascii | utf8 | Utf8 | 64 | 261.7 | 554.7 | 15385.5 | n/a |
| 4096 | mixed | none | Unchecked | 64 | 128.9 | 163.4 | 2483.1 | n/a |
| 4096 | mixed | ascii | AssumedAscii | 64 | 127.6 | 151.0 | 606.3 | n/a |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 167.9 | 219.5 | 2119.9 | n/a |
| 4096 | mixed | utf8 | Utf8 | 64 | 695.9 | 853.5 | 24614.3 | n/a |
| 16384 | ascii | none | Unchecked | 16 | 197.9 | 231.7 | 3750.3 | n/a |
| 16384 | ascii | ascii | AssumedAscii | 16 | 200.5 | 231.7 | 2630.3 | n/a |
| 16384 | ascii | ascii-checked | Ascii | 16 | 356.7 | 401.1 | 2462.3 | n/a |
| 16384 | ascii | utf8 | Utf8 | 16 | 711.1 | 794.3 | 4495.9 | n/a |
| 16384 | mixed | none | Unchecked | 16 | 197.9 | 229.1 | 8243.1 | n/a |
| 16384 | mixed | ascii | AssumedAscii | 16 | 197.9 | 229.1 | 4140.7 | n/a |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 354.1 | 393.3 | 1873.5 | n/a |
| 16384 | mixed | utf8 | Utf8 | 16 | 2196.7 | 2550.3 | 9235.1 | n/a |
