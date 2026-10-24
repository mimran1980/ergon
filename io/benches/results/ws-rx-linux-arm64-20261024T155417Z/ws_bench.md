counter=cntvct_el0
counter_hz=24000000
counter_rate=CNTFRQ_EL0
samples=20000
validate_utf8=true

| size B | shape | decision | path | K | ns/message p50 | p99 | max | max preempted | reassembly copy B/msg | span copy B/msg |
|---|---|---|---|---|---|---|---|---|---|---|
| 100 | whole | kept | old | 64 | 16.9 | 19.5 | 47129.5 | yes | 0 | 0 |
| 100 | whole | kept | new | 64 | 16.9 | 18.8 | 35660.7 | yes | 0 | 0 |
| 100 | whole | skipped | old | 64 | 16.2 | 18.2 | 47001.5 | yes | 0 | 0 |
| 100 | whole | skipped | new | 64 | 3.9 | 4.5 | 482.7 | no | 0 | 0 |
| 100 | frag4 | kept | old | 64 | 25.3 | 29.2 | 47359.9 | yes | 100 | 0 |
| 100 | frag4 | kept | new | 64 | 40.3 | 50.7 | 47359.9 | yes | 0 | 0 |
| 100 | frag4 | skipped | old | 64 | 24.7 | 31.9 | 47539.1 | yes | 100 | 0 |
| 100 | frag4 | skipped | new | 64 | 14.9 | 16.9 | 47180.7 | yes | 0 | 0 |
| 100 | split | kept | old | 64 | 43.6 | 48.8 | 47334.3 | yes | 0 | 101 |
| 100 | split | kept | new | 64 | 42.9 | 48.8 | 47180.7 | yes | 0 | 101 |
| 100 | split | skipped | old | 64 | 42.9 | 47.5 | 47590.3 | yes | 0 | 101 |
| 100 | split | skipped | new | 64 | 26.6 | 33.2 | 47206.3 | yes | 0 | 101 |
| 300 | whole | kept | old | 64 | 61.1 | 78.7 | 47513.5 | yes | 0 | 0 |
| 300 | whole | kept | new | 64 | 61.1 | 86.5 | 50892.7 | yes | 0 | 0 |
| 300 | whole | skipped | old | 64 | 60.5 | 66.4 | 47718.3 | yes | 0 | 0 |
| 300 | whole | skipped | new | 64 | 3.9 | 4.5 | 668.7 | no | 0 | 0 |
| 300 | frag4 | kept | old | 64 | 126.9 | 184.2 | 52326.3 | yes | 300 | 0 |
| 300 | frag4 | kept | new | 64 | 79.4 | 132.1 | 47411.1 | yes | 0 | 0 |
| 300 | frag4 | skipped | old | 64 | 129.5 | 171.8 | 48204.7 | yes | 300 | 0 |
| 300 | frag4 | skipped | new | 64 | 15.6 | 39.7 | 47155.1 | yes | 0 | 0 |
| 300 | split | kept | old | 64 | 130.2 | 231.1 | 47692.7 | yes | 0 | 302 |
| 300 | split | kept | new | 64 | 126.3 | 172.5 | 47667.1 | yes | 0 | 302 |
| 300 | split | skipped | old | 64 | 125.6 | 153.6 | 48127.9 | yes | 0 | 302 |
| 300 | split | skipped | new | 64 | 67.0 | 75.5 | 47334.3 | yes | 0 | 302 |
| 4096 | whole | kept | old | 64 | 889.5 | 16652.7 | 48460.7 | yes | 0 | 0 |
| 4096 | whole | kept | new | 64 | 894.3 | 1635.9 | 51276.7 | yes | 0 | 0 |
| 4096 | whole | skipped | old | 64 | 890.3 | 2313.5 | 48511.9 | yes | 0 | 0 |
| 4096 | whole | skipped | new | 64 | 7.8 | 42.3 | 18444.7 | yes | 0 | 0 |
| 4096 | frag4 | kept | old | 64 | 1702.3 | 48691.1 | 237363.1 | yes | 4096 | 0 |
| 4096 | frag4 | kept | new | 64 | 903.9 | 47820.7 | 48844.7 | yes | 0 | 0 |
| 4096 | frag4 | skipped | old | 64 | 1703.9 | 48665.5 | 61747.1 | yes | 4096 | 0 |
| 4096 | frag4 | skipped | new | 64 | 18.8 | 69.0 | 47103.9 | yes | 0 | 0 |
| 4096 | split | kept | old | 64 | 1718.3 | 48691.1 | 61183.9 | yes | 0 | 4068 |
| 4096 | split | kept | new | 64 | 1716.7 | 48665.5 | 62003.1 | yes | 0 | 4068 |
| 4096 | split | skipped | old | 64 | 1777.5 | 48742.3 | 323174.3 | no | 0 | 4068 |
| 4096 | split | skipped | new | 64 | 856.7 | 8985.5 | 121139.1 | yes | 0 | 4068 |
| 65536 | whole | kept | old | 8 | 14489.5 | 390553.5 | 425574.3 | yes | 0 | 0 |
| 65536 | whole | kept | new | 8 | 14131.1 | 389939.1 | 403660.7 | yes | 0 | 0 |
| 65536 | whole | skipped | old | 8 | 14131.1 | 389939.1 | 649215.9 | yes | 0 | 0 |
| 65536 | whole | skipped | new | 8 | 10.4 | 203.1 | 376627.1 | yes | 0 | 0 |
| 65536 | frag4 | kept | old | 8 | 28287.9 | 403660.7 | 7536639.9 | no | 65536 | 0 |
| 65536 | frag4 | kept | new | 8 | 14668.7 | 391782.3 | 13126860.7 | no | 0 | 0 |
| 65536 | frag4 | skipped | old | 8 | 28211.1 | 405708.7 | 992870.3 | yes | 65536 | 0 |
| 65536 | frag4 | skipped | new | 8 | 83.3 | 599.1 | 378060.7 | yes | 0 | 0 |
| 65536 | split | kept | old | 8 | 27878.3 | 405708.7 | 2139750.3 | yes | 0 | 63498 |
| 65536 | split | kept | new | 8 | 27135.9 | 64716.7 | 1159987.1 | yes | 0 | 63498 |
| 65536 | split | skipped | old | 8 | 26956.7 | 36710.3 | 250879.9 | no | 0 | 63498 |
| 65536 | split | skipped | new | 8 | 12876.7 | 18790.3 | 149606.3 | no | 0 | 63498 |
