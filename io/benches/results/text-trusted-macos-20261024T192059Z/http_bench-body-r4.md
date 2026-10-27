counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=body
samples=20000

| body B | text | check | delivered as | K | ns/response p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 122.3 | 139.3 | 3932.7 | n/a |
| 100 | ascii | ascii | AssumedAscii | 64 | 122.3 | 139.9 | 11859.1 | n/a |
| 100 | ascii | ascii-checked | Ascii | 64 | 125.0 | 147.7 | 7500.7 | n/a |
| 100 | ascii | utf8 | Utf8 | 64 | 127.6 | 143.2 | 1676.7 | n/a |
| 100 | mixed | none | Unchecked | 64 | 125.6 | 274.1 | 7782.3 | n/a |
| 100 | mixed | ascii | AssumedAscii | 64 | 126.3 | 287.1 | 40908.7 | n/a |
| 100 | mixed | ascii-checked | NotAscii | 64 | 128.9 | 297.5 | 91955.1 | n/a |
| 100 | mixed | utf8 | Utf8 | 64 | 147.7 | 339.9 | 28211.1 | n/a |
| 300 | ascii | none | Unchecked | 64 | 122.3 | 138.0 | 1014.3 | n/a |
| 300 | ascii | ascii | AssumedAscii | 64 | 123.0 | 140.6 | 2567.9 | n/a |
| 300 | ascii | ascii-checked | Ascii | 64 | 126.9 | 144.5 | 1122.3 | n/a |
| 300 | ascii | utf8 | Utf8 | 64 | 134.1 | 152.3 | 1059.1 | n/a |
| 300 | mixed | none | Unchecked | 64 | 121.7 | 137.3 | 991.1 | n/a |
| 300 | mixed | ascii | AssumedAscii | 64 | 122.3 | 138.0 | 1059.9 | n/a |
| 300 | mixed | ascii-checked | NotAscii | 64 | 126.3 | 141.9 | 727.5 | n/a |
| 300 | mixed | utf8 | Utf8 | 64 | 177.7 | 204.4 | 890.3 | n/a |
| 4096 | ascii | none | Unchecked | 64 | 125.6 | 140.6 | 1627.1 | n/a |
| 4096 | ascii | ascii | AssumedAscii | 64 | 125.0 | 138.6 | 7212.7 | n/a |
| 4096 | ascii | ascii-checked | Ascii | 64 | 162.7 | 181.6 | 1075.1 | n/a |
| 4096 | ascii | utf8 | Utf8 | 64 | 255.9 | 291.7 | 1679.9 | n/a |
| 4096 | mixed | none | Unchecked | 64 | 125.6 | 142.5 | 1110.3 | n/a |
| 4096 | mixed | ascii | AssumedAscii | 64 | 124.3 | 139.9 | 1641.5 | n/a |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 163.4 | 184.2 | 983.1 | n/a |
| 4096 | mixed | utf8 | Utf8 | 64 | 674.7 | 931.1 | 5267.1 | n/a |
| 16384 | ascii | none | Unchecked | 16 | 200.5 | 239.5 | 11955.1 | n/a |
| 16384 | ascii | ascii | AssumedAscii | 16 | 200.5 | 242.1 | 13094.3 | n/a |
| 16384 | ascii | ascii-checked | Ascii | 16 | 364.5 | 435.1 | 7372.7 | n/a |
| 16384 | ascii | utf8 | Utf8 | 16 | 723.9 | 1227.1 | 28211.1 | n/a |
| 16384 | mixed | none | Unchecked | 16 | 200.5 | 236.9 | 7859.1 | n/a |
| 16384 | mixed | ascii | AssumedAscii | 16 | 200.5 | 239.5 | 2665.5 | n/a |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 364.5 | 432.3 | 6092.7 | n/a |
| 16384 | mixed | utf8 | Utf8 | 16 | 2260.7 | 3227.1 | 96767.9 | n/a |
