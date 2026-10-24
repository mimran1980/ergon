counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=body
samples=20000

| body B | text | check | delivered as | K | ns/response p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 121.7 | 136.7 | 1166.3 | n/a |
| 100 | ascii | ascii | AssumedAscii | 64 | 122.3 | 138.0 | 485.9 | n/a |
| 100 | ascii | ascii-checked | Ascii | 64 | 125.0 | 141.9 | 983.1 | n/a |
| 100 | ascii | utf8 | Utf8 | 64 | 126.9 | 142.5 | 714.3 | n/a |
| 100 | mixed | none | Unchecked | 64 | 121.7 | 136.0 | 769.1 | n/a |
| 100 | mixed | ascii | AssumedAscii | 64 | 122.3 | 136.7 | 875.1 | n/a |
| 100 | mixed | ascii-checked | NotAscii | 64 | 125.0 | 141.2 | 655.1 | n/a |
| 100 | mixed | utf8 | Utf8 | 64 | 143.8 | 161.4 | 1258.3 | n/a |
| 300 | ascii | none | Unchecked | 64 | 121.7 | 136.0 | 603.5 | n/a |
| 300 | ascii | ascii | AssumedAscii | 64 | 122.3 | 137.3 | 1697.5 | n/a |
| 300 | ascii | ascii-checked | Ascii | 64 | 126.3 | 141.2 | 720.7 | n/a |
| 300 | ascii | utf8 | Utf8 | 64 | 133.4 | 148.4 | 2732.7 | n/a |
| 300 | mixed | none | Unchecked | 64 | 121.7 | 136.7 | 690.3 | n/a |
| 300 | mixed | ascii | AssumedAscii | 64 | 122.3 | 136.7 | 1128.7 | n/a |
| 300 | mixed | ascii-checked | NotAscii | 64 | 126.3 | 141.2 | 746.3 | n/a |
| 300 | mixed | utf8 | Utf8 | 64 | 177.7 | 201.1 | 884.7 | n/a |
| 4096 | ascii | none | Unchecked | 64 | 125.6 | 147.1 | 2671.9 | n/a |
| 4096 | ascii | ascii | AssumedAscii | 64 | 125.0 | 143.8 | 1585.5 | n/a |
| 4096 | ascii | ascii-checked | Ascii | 64 | 163.4 | 193.3 | 5647.9 | n/a |
| 4096 | ascii | utf8 | Utf8 | 64 | 257.9 | 395.1 | 2892.7 | n/a |
| 4096 | mixed | none | Unchecked | 64 | 125.6 | 143.2 | 1703.9 | n/a |
| 4096 | mixed | ascii | AssumedAscii | 64 | 124.3 | 141.9 | 16870.3 | n/a |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 163.4 | 197.9 | 4307.1 | n/a |
| 4096 | mixed | utf8 | Utf8 | 64 | 676.7 | 935.1 | 25087.9 | n/a |
| 16384 | ascii | none | Unchecked | 16 | 200.5 | 229.1 | 2393.5 | n/a |
| 16384 | ascii | ascii | AssumedAscii | 16 | 200.5 | 229.1 | 4131.1 | n/a |
| 16384 | ascii | ascii-checked | Ascii | 16 | 356.7 | 398.5 | 4134.3 | n/a |
| 16384 | ascii | utf8 | Utf8 | 16 | 711.1 | 783.9 | 11737.5 | n/a |
| 16384 | mixed | none | Unchecked | 16 | 200.5 | 226.5 | 1979.1 | n/a |
| 16384 | mixed | ascii | AssumedAscii | 16 | 200.5 | 226.5 | 2081.5 | n/a |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 356.7 | 398.5 | 3180.7 | n/a |
| 16384 | mixed | utf8 | Utf8 | 16 | 2198.3 | 2815.9 | 11052.7 | n/a |
