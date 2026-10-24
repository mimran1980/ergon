counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
samples=20000
validate_utf8=true

| size B | shape | decision | path | K | ns/message p50 | p99 | max | max sample preempted (Linux) | reassembly copy B/msg | span copy B/msg |
|---|---|---|---|---|---|---|---|---|---|---|
| 100 | whole | kept | old | 64 | 17.5 | 26.6 | 7276.7 | no | 0 | 0 |
| 100 | whole | kept | new | 64 | 17.5 | 26.0 | 10719.9 | no | 0 | 0 |
| 100 | whole | skipped | old | 64 | 16.2 | 20.8 | 3561.5 | no | 0 | 0 |
| 100 | whole | skipped | new | 64 | 3.9 | 9.7 | 3436.7 | no | 0 | 0 |
| 100 | frag4 | kept | old | 64 | 26.6 | 82.0 | 10918.3 | no | 100 | 0 |
| 100 | frag4 | kept | new | 64 | 35.1 | 118.4 | 6745.5 | no | 0 | 0 |
| 100 | frag4 | skipped | old | 64 | 26.0 | 147.7 | 77772.7 | no | 100 | 0 |
| 100 | frag4 | skipped | new | 64 | 15.6 | 48.8 | 73881.5 | no | 0 | 0 |
| 100 | split | kept | old | 64 | 44.2 | 87.2 | 69427.1 | no | 0 | 101 |
| 100 | split | kept | new | 64 | 44.9 | 92.4 | 99379.1 | no | 0 | 101 |
| 100 | split | skipped | old | 64 | 43.6 | 81.3 | 34585.5 | no | 0 | 101 |
| 100 | split | skipped | new | 64 | 27.3 | 47.5 | 21772.7 | no | 0 | 101 |
| 300 | whole | kept | old | 64 | 61.8 | 338.5 | 66559.9 | no | 0 | 0 |
| 300 | whole | kept | new | 64 | 61.8 | 236.3 | 77875.1 | no | 0 | 0 |
| 300 | whole | skipped | old | 64 | 60.5 | 98.9 | 15756.7 | no | 0 | 0 |
| 300 | whole | skipped | new | 64 | 4.5 | 14.9 | 3564.7 | no | 0 | 0 |
| 300 | frag4 | kept | old | 64 | 131.5 | 306.7 | 11833.5 | no | 300 | 0 |
| 300 | frag4 | kept | new | 64 | 60.5 | 243.5 | 11686.3 | no | 0 | 0 |
| 300 | frag4 | skipped | old | 64 | 130.8 | 549.1 | 11910.3 | no | 300 | 0 |
| 300 | frag4 | skipped | new | 64 | 16.2 | 52.7 | 4543.9 | no | 0 | 0 |
| 300 | split | kept | old | 64 | 130.8 | 634.7 | 23219.1 | no | 0 | 302 |
| 300 | split | kept | new | 64 | 130.8 | 625.1 | 19366.3 | no | 0 | 302 |
| 300 | split | skipped | old | 64 | 129.5 | 151.6 | 10220.7 | no | 0 | 302 |
| 300 | split | skipped | new | 64 | 69.6 | 80.0 | 14579.1 | no | 0 | 302 |
| 4096 | whole | kept | old | 64 | 919.1 | 4774.3 | 62924.7 | no | 0 | 0 |
| 4096 | whole | kept | new | 64 | 919.1 | 4851.1 | 42751.9 | no | 0 | 0 |
| 4096 | whole | skipped | old | 64 | 920.7 | 1817.5 | 31078.3 | no | 0 | 0 |
| 4096 | whole | skipped | new | 64 | 8.4 | 80.7 | 8812.7 | no | 0 | 0 |
| 4096 | frag4 | kept | old | 64 | 1753.5 | 3102.3 | 962559.9 | no | 4096 | 0 |
| 4096 | frag4 | kept | new | 64 | 615.5 | 1171.9 | 27135.9 | no | 0 | 0 |
| 4096 | frag4 | skipped | old | 64 | 1756.7 | 4316.7 | 72243.1 | no | 4096 | 0 |
| 4096 | frag4 | skipped | new | 64 | 27.9 | 76.8 | 5398.3 | no | 0 | 0 |
| 4096 | split | kept | old | 64 | 1787.1 | 5743.9 | 40319.9 | no | 0 | 4068 |
| 4096 | split | kept | new | 64 | 1787.1 | 5708.7 | 77465.5 | no | 0 | 4068 |
| 4096 | split | skipped | old | 64 | 1785.5 | 7135.9 | 57548.7 | no | 0 | 4068 |
| 4096 | split | skipped | new | 64 | 863.9 | 4022.3 | 88268.7 | no | 0 | 4068 |
| 65536 | whole | kept | old | 8 | 14707.1 | 47820.7 | 819609.5 | no | 0 | 0 |
| 65536 | whole | kept | new | 8 | 14707.1 | 46131.1 | 2062745.5 | no | 0 | 0 |
| 65536 | whole | skipped | old | 8 | 14719.9 | 37427.1 | 724172.7 | no | 0 | 0 |
| 65536 | whole | skipped | new | 8 | 15.6 | 218.7 | 31692.7 | no | 0 | 0 |
| 65536 | frag4 | kept | old | 8 | 28467.1 | 118783.9 | 1620377.5 | no | 65536 | 0 |
| 65536 | frag4 | kept | new | 8 | 9721.5 | 49587.1 | 470630.3 | no | 0 | 0 |
| 65536 | frag4 | skipped | old | 8 | 28492.7 | 125439.9 | 2131558.3 | no | 65536 | 0 |
| 65536 | frag4 | skipped | new | 8 | 41.6 | 661.5 | 86015.9 | no | 0 | 0 |
| 65536 | split | kept | old | 8 | 28313.5 | 103987.1 | 4879155.1 | yes | 0 | 63498 |
| 65536 | split | kept | new | 8 | 28313.5 | 104038.3 | 3191603.1 | no | 0 | 63498 |
| 65536 | split | skipped | old | 8 | 28339.1 | 79513.5 | 728678.3 | no | 0 | 63498 |
| 65536 | split | skipped | new | 8 | 13503.9 | 39833.5 | 258252.7 | no | 0 | 63498 |
