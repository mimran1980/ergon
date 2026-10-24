counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
suite=body
samples=20000

| body B | text | check | delivered as | K | ns/response p50 | p99 | max | max sample preempted (Linux) |
|---|---|---|---|---|---|---|---|---|
| 100 | ascii | none | Unchecked | 64 | 488.3 | 627.1 | 4473.5 | no |
| 100 | ascii | ascii | AssumedAscii | 64 | 489.1 | 613.9 | 3350.3 | no |
| 100 | ascii | ascii-checked | Ascii | 64 | 501.5 | 628.3 | 2719.9 | no |
| 100 | ascii | utf8 | Utf8 | 64 | 494.3 | 629.5 | 9523.1 | no |
| 100 | mixed | none | Unchecked | 64 | 488.3 | 589.1 | 12172.7 | no |
| 100 | mixed | ascii | AssumedAscii | 64 | 489.1 | 593.1 | 26623.9 | no |
| 100 | mixed | ascii-checked | NotAscii | 64 | 501.5 | 615.5 | 18803.1 | no |
| 100 | mixed | utf8 | Utf8 | 64 | 509.9 | 620.7 | 3878.3 | no |
| 300 | ascii | none | Unchecked | 64 | 488.3 | 628.3 | 4915.1 | no |
| 300 | ascii | ascii | AssumedAscii | 64 | 489.1 | 634.7 | 16102.3 | no |
| 300 | ascii | ascii-checked | Ascii | 64 | 527.9 | 659.5 | 5343.9 | no |
| 300 | ascii | utf8 | Utf8 | 64 | 500.7 | 624.3 | 11500.7 | no |
| 300 | mixed | none | Unchecked | 64 | 489.5 | 891.9 | 29670.3 | no |
| 300 | mixed | ascii | AssumedAscii | 64 | 490.3 | 895.1 | 70143.9 | no |
| 300 | mixed | ascii-checked | NotAscii | 64 | 529.5 | 979.1 | 78847.9 | yes |
| 300 | mixed | utf8 | Utf8 | 64 | 545.5 | 960.7 | 27750.3 | no |
| 4096 | ascii | none | Unchecked | 64 | 491.5 | 667.5 | 8415.9 | no |
| 4096 | ascii | ascii | AssumedAscii | 64 | 493.5 | 672.7 | 237158.3 | no |
| 4096 | ascii | ascii-checked | Ascii | 64 | 1068.7 | 1397.5 | 12614.3 | no |
| 4096 | ascii | utf8 | Utf8 | 64 | 634.7 | 875.1 | 4995.1 | no |
| 4096 | mixed | none | Unchecked | 64 | 491.5 | 692.3 | 5782.3 | no |
| 4096 | mixed | ascii | AssumedAscii | 64 | 493.5 | 699.5 | 5849.5 | no |
| 4096 | mixed | ascii-checked | NotAscii | 64 | 1067.9 | 1420.7 | 14924.7 | no |
| 4096 | mixed | utf8 | Utf8 | 64 | 1045.5 | 1426.3 | 8428.7 | no |
| 16384 | ascii | none | Unchecked | 16 | 951.1 | 1219.1 | 44364.7 | no |
| 16384 | ascii | ascii | AssumedAscii | 16 | 953.5 | 1255.9 | 5065.5 | no |
| 16384 | ascii | ascii-checked | Ascii | 16 | 3222.3 | 4300.7 | 11155.1 | yes |
| 16384 | ascii | utf8 | Utf8 | 16 | 1445.5 | 2028.7 | 9727.9 | no |
| 16384 | mixed | none | Unchecked | 16 | 947.9 | 1216.7 | 27878.3 | no |
| 16384 | mixed | ascii | AssumedAscii | 16 | 951.1 | 1231.9 | 11206.3 | no |
| 16384 | mixed | ascii-checked | NotAscii | 16 | 3219.1 | 3932.7 | 12729.5 | no |
| 16384 | mixed | utf8 | Utf8 | 16 | 2953.5 | 3833.5 | 37452.7 | no |
