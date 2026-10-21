# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|
| r1-base-0 | 67.40 | 7745.0 | 29.5 | 0.0 | 67.40 µs stall: softirq TIMER |
| r1-base-1 | 214.63 | 7496.5 | 34.5 | 1.0 | 214.63 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |

## HTTP round trip, http-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189906 | 44.16 | 62.47 | 76.58 | 111.86 | 335.75 | 88.2 | 0.0 | 0.0 | 0.0 | 175.46 | 6461.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189904 | 44.18 | 60.73 | 75.05 | 89.42 | 303.51 | 35.3 | 0.0 | 0.0 | 0.0 | 90.52 | 6702.0 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189906 | 44.33 | 62.71 | 76.93 | 115.30 | 335.91 | 101.4 | 0.0 | 0.0 | 0.0 |
| r1-base-1 | 1189904 | 44.37 | 61.23 | 75.47 | 91.27 | 303.66 | 43.9 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 2900000 | 5.30 | 13.27 | 35.96 | 46.05 | 58.86 | 0.0 | 0.0 | 0.0 | 0.0 | 897.07 | 6164.0 | 897.07 µs stall: loop work (3650 items) |
| r1-base-1 | 2900000 | 4.78 | 14.39 | 32.01 | 92.95 | 134.04 | 214.1 | 0.0 | 0.0 | 0.0 | 799.18 | 8132.0 | 799.18 µs stall: loop work (3600 items) |

## WebSocket round trip, pp-epoll-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1191366 | 48.96 | 93.54 | 98.49 | 118.65 | 331.14 | 816.6 | 0.0 | 0.0 | 0.0 | 268.28 | 81.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1193350 | 48.97 | 92.85 | 98.35 | 113.71 | 550.74 | 646.8 | 1.0 | 0.0 | 0.0 | 55.62 | 94.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-epoll-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 46.67 | 92.60 | 97.38 | 173.21 | 580.71 | 352.9 | 0.5 | 0.0 | 0.0 | 225.69 | 507.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189998 | 46.82 | 92.34 | 97.22 | 261.50 | 1016.73 | 434.1 | 0.5 | 0.5 | 0.0 | 289.28 | 85.0 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-epoll-spin-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 47.09 | 93.03 | 98.44 | 176.28 | 581.05 | 467.9 | 1.0 | 0.0 | 0.0 |
| r1-base-1 | 1189998 | 47.27 | 92.93 | 120.63 | 282.60 | 1017.44 | 938.3 | 5.0 | 0.5 | 0.0 |

## WebSocket round trip, pp-recv-busy-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1366642 | 42.74 | 50.73 | 66.39 | 97.69 | 344.09 | 133.2 | 0.0 | 0.0 | 0.0 | 54.35 | 2833.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1355714 | 43.10 | 51.39 | 66.75 | 104.01 | 316.92 | 154.6 | 0.0 | 0.0 | 0.0 | 171.00 | 2791.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

## WebSocket round trip, pp-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 44.84 | 54.44 | 74.89 | 112.95 | 542.01 | 85.2 | 0.5 | 0.0 | 0.0 | 60.54 | 3091.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189998 | 44.38 | 51.51 | 71.20 | 146.97 | 535.92 | 128.1 | 0.5 | 0.0 | 0.0 | 276.13 | 1088.0 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 45.58 | 55.30 | 75.84 | 114.75 | 542.27 | 90.8 | 0.5 | 0.0 | 0.0 |
| r1-base-1 | 1189998 | 45.02 | 52.38 | 75.37 | 153.66 | 536.78 | 190.6 | 0.5 | 0.0 | 0.0 |

## WebSocket round trip, pp-uring-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1196297 | 46.49 | 60.92 | 96.71 | 148.88 | 228.33 | 448.5 | 0.0 | 0.0 | 0.0 | 62.06 | 75.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1216800 | 47.26 | 66.92 | 96.66 | 253.08 | 338.79 | 649.8 | 0.0 | 0.0 | 0.0 | 287.76 | 123.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189998 | 59.25 | 95.29 | 100.98 | 125.52 | 231.56 | 728.6 | 0.0 | 0.0 | 0.0 | 56.67 | 42.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189998 | 46.64 | 92.39 | 96.28 | 165.93 | 466.34 | 311.6 | 0.0 | 0.0 | 0.0 | 84.46 | 109.5 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-uring-spin-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189998 | 59.34 | 95.39 | 101.48 | 128.19 | 231.65 | 795.6 | 0.0 | 0.0 | 0.0 |
| r1-base-1 | 1189998 | 46.75 | 92.59 | 99.74 | 452.51 | 1860.14 | 586.9 | 57.5 | 35.8 | 0.0 |

## WebSocket round trip, pp-uring-sqpoll-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1369055 | 42.57 | 51.77 | 63.16 | 94.99 | 276.68 | 127.1 | 0.0 | 0.0 | 0.0 | 43.48 | 29.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1341497 | 43.46 | 52.34 | 64.03 | 100.89 | 316.03 | 139.3 | 0.0 | 0.0 | 0.0 | 93.60 | 32.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-sqpoll-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189997 | 44.53 | 54.05 | 66.12 | 110.78 | 338.90 | 75.6 | 0.0 | 0.0 | 0.0 | 53.82 | 28.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189998 | 43.95 | 59.67 | 118.00 | 237.90 | 419.13 | 1163.7 | 0.0 | 0.0 | 0.0 | 271.99 | 35.5 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-uring-sqpoll-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189997 | 44.59 | 54.21 | 66.62 | 125.78 | 767.30 | 97.3 | 5.0 | 0.0 | 0.0 |
| r1-base-1 | 1189998 | 44.01 | 60.72 | 819.18 | 1193.68 | 1442.26 | 2773.6 | 1040.2 | 304.0 | 0.0 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### http-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 335.75 at 75.3 s: neither loop stalled long enough: network, hypervisor or untraced; 310.10 at 29.7 s: neither loop stalled long enough: network, hypervisor or untraced; 271.57 at 76.3 s: neither loop stalled long enough: network, hypervisor or untraced; 247.39 at 18.3 s: client: VM paused: the idle spare core stalled too (hypervisor); 220.30 at 19.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 303.51 at 11.2 s: neither loop stalled long enough: network, hypervisor or untraced; 212.63 at 26.4 s: server: softirq TIMER; 184.20 at 117.1 s: neither loop stalled long enough: network, hypervisor or untraced; 170.31 at 3.9 s: neither loop stalled long enough: network, hypervisor or untraced; 165.65 at 96.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### md-recv-busy

| condition | worst events |
|---|---|
| r1-base-0 | 58.86 at 13.0 s: client: loop work (31 items); 58.69 at 13.0 s: client: loop work (31 items); 58.53 at 13.0 s: client: loop work (31 items); 58.36 at 13.0 s: client: loop work (31 items); 58.19 at 13.0 s: client: loop work (31 items) |
| r1-base-1 | 134.04 at 7.4 s: client: loop work (600 items); 133.88 at 7.4 s: client: loop work (600 items); 133.70 at 7.4 s: client: loop work (600 items); 133.53 at 7.4 s: client: loop work (600 items); 133.36 at 7.4 s: client: loop work (600 items) |

### noise

| condition | worst events |
|---|---|
| r1-base-0 | 67.40 at 100.4 s: softirq TIMER; 63.84 at 6.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 45.22 at 0.6 s: mostly unexplained (+local timer tick); 45.20 at 66.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 43.79 at 48.6 s: mostly unexplained (+local timer tick) |
| r1-base-1 | 214.63 at 54.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 124.54 at 109.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 71.11 at 100.9 s: softirq TIMER; 47.17 at 16.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 45.20 at 54.4 s: mostly unexplained (+local timer tick) |

### pp-epoll-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 331.14 at 30.2 s: neither loop stalled long enough: network, hypervisor or untraced; 305.54 at 45.7 s: client: VM paused: the idle spare core stalled too (hypervisor); 296.25 at 24.9 s: neither loop stalled long enough: network, hypervisor or untraced; 292.75 at 41.7 s: client: VM paused: the idle spare core stalled too (hypervisor); 282.44 at 41.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 550.74 at 8.0 s: neither loop stalled long enough: network, hypervisor or untraced; 235.68 at 12.6 s: neither loop stalled long enough: network, hypervisor or untraced; 210.14 at 23.6 s: neither loop stalled long enough: network, hypervisor or untraced; 207.41 at 54.6 s: neither loop stalled long enough: network, hypervisor or untraced; 191.62 at 39.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-epoll-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 580.71 at 106.5 s: neither loop stalled long enough: network, hypervisor or untraced; 364.52 at 15.8 s: neither loop stalled long enough: network, hypervisor or untraced; 362.27 at 97.8 s: neither loop stalled long enough: network, hypervisor or untraced; 337.31 at 69.6 s: neither loop stalled long enough: network, hypervisor or untraced; 329.07 at 87.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1016.73 at 91.7 s: neither loop stalled long enough: network, hypervisor or untraced; 339.15 at 81.8 s: neither loop stalled long enough: network, hypervisor or untraced; 330.91 at 101.5 s: neither loop stalled long enough: network, hypervisor or untraced; 328.70 at 51.0 s: neither loop stalled long enough: network, hypervisor or untraced; 328.66 at 18.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-0

| condition | worst events |
|---|---|
| r1-base-0 | 344.09 at 45.7 s: neither loop stalled long enough: network, hypervisor or untraced; 272.51 at 30.3 s: neither loop stalled long enough: network, hypervisor or untraced; 186.54 at 54.8 s: neither loop stalled long enough: network, hypervisor or untraced; 177.30 at 19.8 s: neither loop stalled long enough: network, hypervisor or untraced; 174.48 at 16.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-1 | 316.92 at 19.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 296.95 at 17.9 s: neither loop stalled long enough: network, hypervisor or untraced; 200.28 at 17.9 s: neither loop stalled long enough: network, hypervisor or untraced; 180.41 at 49.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 171.99 at 43.1 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 542.01 at 10.1 s: neither loop stalled long enough: network, hypervisor or untraced; 180.09 at 74.3 s: neither loop stalled long enough: network, hypervisor or untraced; 164.84 at 25.1 s: neither loop stalled long enough: network, hypervisor or untraced; 141.90 at 105.4 s: neither loop stalled long enough: network, hypervisor or untraced; 141.31 at 46.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 535.92 at 50.8 s: neither loop stalled long enough: network, hypervisor or untraced; 309.70 at 64.0 s: neither loop stalled long enough: network, hypervisor or untraced; 302.29 at 72.1 s: neither loop stalled long enough: network, hypervisor or untraced; 297.10 at 63.2 s: neither loop stalled long enough: network, hypervisor or untraced; 281.20 at 63.2 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 228.33 at 33.2 s: neither loop stalled long enough: network, hypervisor or untraced; 224.57 at 20.1 s: neither loop stalled long enough: network, hypervisor or untraced; 219.04 at 50.4 s: neither loop stalled long enough: network, hypervisor or untraced; 214.94 at 35.3 s: neither loop stalled long enough: network, hypervisor or untraced; 213.96 at 42.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 338.79 at 33.1 s: neither loop stalled long enough: network, hypervisor or untraced; 331.59 at 54.1 s: neither loop stalled long enough: network, hypervisor or untraced; 328.34 at 51.5 s: neither loop stalled long enough: network, hypervisor or untraced; 324.87 at 56.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 324.26 at 32.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 231.56 at 36.2 s: neither loop stalled long enough: network, hypervisor or untraced; 225.85 at 56.2 s: neither loop stalled long enough: network, hypervisor or untraced; 225.55 at 52.2 s: neither loop stalled long enough: network, hypervisor or untraced; 224.11 at 93.2 s: neither loop stalled long enough: network, hypervisor or untraced; 223.75 at 106.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 466.34 at 113.6 s: neither loop stalled long enough: network, hypervisor or untraced; 432.74 at 113.6 s: neither loop stalled long enough: network, hypervisor or untraced; 344.36 at 113.6 s: neither loop stalled long enough: network, hypervisor or untraced; 337.17 at 113.6 s: neither loop stalled long enough: network, hypervisor or untraced; 326.26 at 11.1 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-0

| condition | worst events |
|---|---|
| r1-base-0 | 276.68 at 11.5 s: neither loop stalled long enough: network, hypervisor or untraced; 215.11 at 9.3 s: neither loop stalled long enough: network, hypervisor or untraced; 199.45 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 198.43 at 7.4 s: neither loop stalled long enough: network, hypervisor or untraced; 183.01 at 59.8 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-1 | 316.03 at 57.6 s: neither loop stalled long enough: network, hypervisor or untraced; 224.33 at 35.9 s: neither loop stalled long enough: network, hypervisor or untraced; 218.44 at 24.6 s: neither loop stalled long enough: network, hypervisor or untraced; 204.66 at 57.6 s: neither loop stalled long enough: network, hypervisor or untraced; 172.48 at 26.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-10000

| condition | worst events |
|---|---|
| r1-base-0 | 338.90 at 100.6 s: neither loop stalled long enough: network, hypervisor or untraced; 327.97 at 100.6 s: neither loop stalled long enough: network, hypervisor or untraced; 228.07 at 100.6 s: neither loop stalled long enough: network, hypervisor or untraced; 211.88 at 38.7 s: neither loop stalled long enough: network, hypervisor or untraced; 209.69 at 62.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 419.13 at 40.5 s: neither loop stalled long enough: network, hypervisor or untraced; 402.86 at 41.0 s: neither loop stalled long enough: network, hypervisor or untraced; 400.31 at 44.0 s: neither loop stalled long enough: network, hypervisor or untraced; 390.50 at 42.1 s: neither loop stalled long enough: network, hypervisor or untraced; 388.02 at 41.6 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| r1-base-0 | unexplained: no kernel event (hypervisor or in-kernel time) 278851.87; loop work 228579.87; softirq NET_RX 215702.49; mostly unexplained (+preempted by kworker/1:1) 182226.83; mostly unexplained (+softirq NET_RX) 113982.69 |
| r1-base-1 | loop work 307925.26; unexplained: no kernel event (hypervisor or in-kernel time) 262920.33; softirq NET_RX 152062.55; mostly unexplained (+preempted by kworker/1:1) 145676.80; mostly unexplained (+softirq NET_RX) 90416.08 |

## Once-a-minute stalls

Whether each source's events of 300 µs or more (loop stalls, or samples that slow) bunch at one second of the wall-clock minute. The busiest second is the middle of the busiest three; three seconds hold one event in twenty by chance, so a share of 25% or more over at least five events is marked periodic. Each host has its own second (its agents' timers start at boot).

| run | source | events | busiest second | events there (share) | minutes with one there | longest there (µs) | periodic |
|---|---|---|---|---|---|---|---|
| cpumap-receiver/cpu1 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu2 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu3 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu4 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu5 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu6 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu7 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu1 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu3 | counter-only spinner | 0 | | | | | |
| http-recv-busy-10000 | idle spare core | 0 | | | | | |
| http-recv-busy-10000 | receiver loop | 0 | | | | | |
| http-recv-busy-10000 | sender loop | 0 | | | | | |
| http-recv-busy-10000 | slow samples | 3 | :09 | 1 (33%) | 1 of 4 | 303.51 | no |
| md-recv-busy | idle spare core | 0 | | | | | |
| md-recv-busy | receiver loop | 0 | | | | | |
| md-recv-busy | slow samples | 0 | | | | | |
| noise | noise probe (loop CPU) | 0 | | | | | |
| pp-epoll-spin-0 | idle spare core | 0 | | | | | |
| pp-epoll-spin-0 | receiver loop | 0 | | | | | |
| pp-epoll-spin-0 | sender loop | 0 | | | | | |
| pp-epoll-spin-0 | slow samples | 3 | :14 | 1 (33%) | 1 of 2 | 331.14 | no |
| pp-epoll-spin-10000 | idle spare core | 0 | | | | | |
| pp-epoll-spin-10000 | receiver loop | 0 | | | | | |
| pp-epoll-spin-10000 | sender loop | 0 | | | | | |
| pp-epoll-spin-10000 | slow samples | 49 | :00 | 6 (12%) | 2 of 4 | 362.27 | no |
| pp-recv-busy-0 | idle spare core | 0 | | | | | |
| pp-recv-busy-0 | receiver loop | 0 | | | | | |
| pp-recv-busy-0 | sender loop | 0 | | | | | |
| pp-recv-busy-0 | slow samples | 2 | :17 | 1 (50%) | 1 of 2 | 316.92 | no |
| pp-recv-busy-10000 | idle spare core | 0 | | | | | |
| pp-recv-busy-10000 | receiver loop | 0 | | | | | |
| pp-recv-busy-10000 | sender loop | 0 | | | | | |
| pp-recv-busy-10000 | slow samples | 4 | :20 | 1 (25%) | 1 of 4 | 535.92 | no |
| pp-uring-spin-0 | idle spare core | 0 | | | | | |
| pp-uring-spin-0 | receiver loop | 0 | | | | | |
| pp-uring-spin-0 | sender loop | 0 | | | | | |
| pp-uring-spin-0 | slow samples | 45 | :15 | 6 (13%) | 1 of 2 | 338.79 | no |
| pp-uring-spin-10000 | idle spare core | 0 | | | | | |
| pp-uring-spin-10000 | receiver loop | 0 | | | | | |
| pp-uring-spin-10000 | sender loop | 0 | | | | | |
| pp-uring-spin-10000 | slow samples | 8 | :13 | 5 (62%) | 1 of 4 | 466.34 | **yes** |
| pp-uring-sqpoll-0 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-0 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-0 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-0 | slow samples | 1 | :50 | 1 (100%) | 1 of 2 | 316.03 | no |
| pp-uring-sqpoll-10000 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-10000 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-10000 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-10000 | slow samples | 34 | :07 | 21 (62%) | 1 of 4 | 400.31 | **yes** |

# Every run in detail

#### Stalls: r1-base-0/http-recv-busy-10000, loop 0 on CPU 1

2257288506 iterations, longest 175.5 µs; over 5 / 20 / 100 µs: 948376 / 12922 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4574 | 126546.7 | 175.5 |
| softirq NET_RX | 2907 | 93812.6 | 57.9 |
| mostly unexplained (+softirq NET_RX) | 2921 | 86010.3 | 64.2 |
| mostly unexplained (+preempted by kworker/1:1) | 1952 | 61889.6 | 71.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 528 | 12273.3 | 174.9 |
| mostly unexplained (+local timer tick) | 26 | 765.8 | 51.7 |
| loop work | 10 | 221.4 | 37.8 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 74.3 | 38.1 |
| softirq TIMER | 1 | 54.7 | 54.7 |
| mostly unexplained (+softirq TIMER) | 1 | 20.4 | 20.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 175.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 174.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 71.1 | 2 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 64.2 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 22, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 64.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 63.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 62.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 18, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 62.6 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 62.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 22, preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 61.8 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 5877 |
| IWI IRQ work interrupts | 5757 |
| CAL Function call interrupts | 1033 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/http-recv-busy-10000 (median 44.16)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 335.75 | 291.59 | 0.00 |  | 0.00 |  | 291.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 310.10 | 265.94 | 0.00 |  | 0.00 |  | 265.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 271.57 | 227.41 | 0.00 |  | 0.00 |  | 227.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 247.39 | 203.23 | 174.90 | VM paused: the idle spare core stalled too (hypervisor) | 0.00 |  | 28.33 | client: VM paused: the idle spare core stalled too (hypervisor) |
| 5 | 220.30 | 176.14 | 0.00 |  | 0.00 |  | 176.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 216.97 | 172.81 | 175.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 196.10 | 151.94 | 0.00 |  | 0.00 |  | 151.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 193.09 | 148.93 | 0.00 |  | 0.00 |  | 148.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 188.20 | 144.04 | 0.00 |  | 0.00 |  | 144.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 181.44 | 137.28 | 25.34 | mostly unexplained (+preempted by kworker/1:1) | 0.00 |  | 111.94 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-0/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=388722 clocksource=tsc

### Loop counters (r1-base-0/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3003650 | 418260 | 7.18 | 43651264 | 14.53 | 727521 | 727521 | 99.0 | 59.5 | true | 0 | 1316 | 897.1 | 420727 / 6164 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672817600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (30639 of 2900000 in 1-msg reads) | 30639 | 4.95 | 4.61 | 5.56 | 11.31 | 38.01 | 45.41 | 50.69 |

#### Stalls: r1-base-0/md-recv-busy, loop 0 on CPU 1

43651264 iterations, longest 897.1 µs; over 5 / 20 / 100 µs: 420727 / 6164 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5707 | 228318.4 | 897.1 |
| mostly unexplained (+preempted by kworker/1:1) | 199 | 6790.1 | 66.5 |
| softirq NET_RX | 139 | 5200.5 | 53.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 55 | 1377.8 | 169.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 45 | 1339.4 | 36.1 |
| mostly unexplained (+local timer tick) | 11 | 303.7 | 35.3 |
| mostly unexplained (+softirq NET_RX) | 3 | 106.6 | 39.6 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 80.7 | 30.9 |
| mostly unexplained (+softirq TIMER) | 2 | 53.4 | 27.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 897.1 | 3650 | loop work (3650 items) |  |
| 2 | 169.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 92.8 | 31 | loop work (31 items) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 88.7 | 38 | loop work (38 items) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 82.6 | 30 | loop work (30 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 82.2 | 28 | loop work (28 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 81.7 | 18 | loop work (18 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 81.3 | 31 | loop work (31 items) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 80.6 | 30 | loop work (30 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 79.7 | 20 | loop work (20 items) | softirq NET_RX 29, preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 1339 |
| IWI IRQ work interrupts | 1332 |
| CAL Function call interrupts | 1011 |
| RES Rescheduling interrupts | 2 |
| TLB TLB shootdowns | 2 |

## r1-base-0/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-0/noise, loop 0 on CPU 1

12656644096 iterations, longest 67.4 µs; over 5 / 20 / 100 µs: 15490 / 59 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 15388 | 143862.1 | 63.8 |
| mostly unexplained (+local timer tick) | 35 | 1026.9 | 45.2 |
| mostly unexplained (+call function single (IPI/vector)) | 29 | 483.4 | 19.2 |
| mostly unexplained (+call function (IPI/vector)) | 20 | 212.1 | 11.5 |
| local timer tick | 8 | 181.6 | 24.1 |
| softirq TIMER | 6 | 144.4 | 67.4 |
| mostly unexplained (+softirq TIMER) | 4 | 67.3 | 19.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 67.4 | 0 | softirq TIMER | softirq TIMER 42, local timer tick 13, timer tick_nohz_handler 7 |
| 2 | 63.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 45.2 | 0 | mostly unexplained (+local timer tick) | local timer tick 19, timer tick_nohz_handler 10, softirq TIMER 8, ipi_send_cpu 0 |
| 4 | 45.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 43.8 | 0 | mostly unexplained (+local timer tick) | local timer tick 18, softirq TIMER 9, timer tick_nohz_handler 9, ipi_send_cpu 0 |
| 6 | 43.5 | 0 | mostly unexplained (+local timer tick) | local timer tick 17, timer tick_nohz_handler 9, softirq TIMER 8, ipi_send_cpu 0 |
| 7 | 42.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 18, softirq TIMER 8, timer tick_nohz_handler 8, ipi_send_cpu 0 |
| 8 | 42.6 | 0 | mostly unexplained (+local timer tick) | local timer tick 19, timer tick_nohz_handler 10, softirq TIMER 8, ipi_send_cpu 0 |
| 9 | 42.4 | 0 | mostly unexplained (+local timer tick) | local timer tick 18, softirq TIMER 9, timer tick_nohz_handler 9, ipi_send_cpu 0 |
| 10 | 41.8 | 0 | mostly unexplained (+local timer tick) | local timer tick 19, timer tick_nohz_handler 11, softirq TIMER 8, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1028 |
| LOC Local timer interrupts | 54 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-base-0/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1211724 | 1211724 | 1.00 | 80914457 | 66.78 | 1348574 | 1308183 | 0.0 | 30.9 | false | 0 | 0 | 268.3 | 19021 / 81 / 3 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-0, loop 0 on CPU 1

78491008 iterations, longest 268.3 µs; over 5 / 20 / 100 µs: 19021 / 81 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 53 | 1548.6 | 268.3 |
| mostly unexplained (+local timer tick) | 9 | 325.8 | 50.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 7 | 272.8 | 144.8 |
| local timer tick | 7 | 182.8 | 30.8 |
| mostly unexplained (+softirq TIMER) | 4 | 92.2 | 25.2 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 42.6 | 42.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 268.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 144.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 136.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 50.5 | 0 | mostly unexplained (+local timer tick) | local timer tick 18, softirq RCU 13, timer tick_nohz_handler 8, softirq TIMER 7 |
| 5 | 45.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 42.6 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 42.2 | 1 | mostly unexplained (+local timer tick) | local timer tick 17, timer tick_nohz_handler 10, softirq TIMER 8, softirq RCU 1 |
| 8 | 40.9 | 1 | mostly unexplained (+local timer tick) | local timer tick 17, timer tick_nohz_handler 10, softirq TIMER 6, softirq RCU 1 |
| 9 | 40.4 | 0 | mostly unexplained (+local timer tick) | local timer tick 18, timer tick_nohz_handler 10, softirq TIMER 7, ipi_send_cpu 0 |
| 10 | 35.2 | 1 | mostly unexplained (+local timer tick) | local timer tick 15, timer tick_nohz_handler 9, softirq TIMER 7, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1006 |
| LOC Local timer interrupts | 39 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-0 (median 48.96)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 331.14 | 282.18 | 0.00 |  | 0.00 |  | 282.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 305.54 | 256.58 | 268.28 | VM paused: the idle spare core stalled too (hypervisor) | 0.00 |  | 0.00 | client: VM paused: the idle spare core stalled too (hypervisor) |
| 3 | 296.25 | 247.29 | 0.00 |  | 0.00 |  | 247.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 292.75 | 243.79 | 136.76 | VM paused: the idle spare core stalled too (hypervisor) | 0.00 |  | 107.03 | client: VM paused: the idle spare core stalled too (hypervisor) |
| 5 | 282.44 | 233.48 | 0.00 |  | 0.00 |  | 233.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 215.09 | 166.13 | 0.00 |  | 0.00 |  | 166.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 212.57 | 163.61 | 0.00 |  | 0.00 |  | 163.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 211.46 | 162.50 | 0.00 |  | 0.00 |  | 162.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 196.50 | 147.54 | 0.00 |  | 0.00 |  | 147.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 195.52 | 146.56 | 0.00 |  | 0.00 |  | 146.56 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 167747200 | 139.79 | 1397893 | 1377893 | 0.0 | 27.6 | false | 0 | 0 | 225.7 | 55802 / 1015 / 1 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-10000, loop 0 on CPU 1

165347200 iterations, longest 225.7 µs; over 5 / 20 / 100 µs: 55802 / 1015 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 817 | 17695.1 | 225.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 158 | 3346.0 | 24.8 |
| mostly unexplained (+local timer tick) | 26 | 931.5 | 53.5 |
| local timer tick | 7 | 194.3 | 29.7 |
| mostly unexplained (+softirq TIMER) | 6 | 139.9 | 24.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 31.7 | 31.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 225.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 53.5 | 1 | mostly unexplained (+local timer tick) | local timer tick 22, timer tick_nohz_handler 12, softirq TIMER 7, softirq RCU 1 |
| 3 | 51.2 | 1 | mostly unexplained (+local timer tick) | local timer tick 21, timer tick_nohz_handler 9, softirq TIMER 7, softirq RCU 1 |
| 4 | 50.1 | 1 | mostly unexplained (+local timer tick) | local timer tick 20, timer tick_nohz_handler 9, softirq TIMER 7, softirq RCU 1 |
| 5 | 46.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 46.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 45.5 | 1 | mostly unexplained (+local timer tick) | local timer tick 17, timer tick_nohz_handler 7, softirq TIMER 5, softirq RCU 1 |
| 8 | 42.7 | 1 | mostly unexplained (+local timer tick) | local timer tick 16, timer tick_nohz_handler 10, softirq TIMER 8, softirq RCU 2 |
| 9 | 40.5 | 1 | mostly unexplained (+local timer tick) | local timer tick 15, timer tick_nohz_handler 9, softirq TIMER 7, softirq RCU 1 |
| 10 | 40.3 | 1 | mostly unexplained (+local timer tick) | local timer tick 15, timer tick_nohz_handler 8, softirq TIMER 7, softirq RCU 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1030 |
| LOC Local timer interrupts | 69 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-10000 (median 46.67)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 580.71 | 534.04 | 0.00 |  | 0.00 |  | 534.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 364.52 | 317.85 | 0.00 |  | 0.00 |  | 317.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 362.27 | 315.60 | 0.00 |  | 0.00 |  | 315.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 337.31 | 290.64 | 0.00 |  | 0.00 |  | 290.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 329.07 | 282.40 | 0.00 |  | 0.00 |  | 282.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 299.27 | 252.60 | 0.00 |  | 0.00 |  | 252.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 280.32 | 233.65 | 0.00 |  | 0.00 |  | 233.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 269.99 | 223.32 | 225.69 | VM paused: the idle spare core stalled too (hypervisor) | 0.00 |  | 0.00 | client: VM paused: the idle spare core stalled too (hypervisor) |
| 9 | 243.58 | 196.91 | 0.00 |  | 0.00 |  | 196.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 239.26 | 192.59 | 0.00 |  | 0.00 |  | 192.59 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1389662 | 1389662 | 1.00 | 37375519 | 26.90 | 622925 | 599764 | 96.1 | 65.7 | true | 0 | 2342 | 54.4 | 1560316 / 2833 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-0, loop 0 on CPU 1

35985856 iterations, longest 54.4 µs; over 5 / 20 / 100 µs: 1560316 / 2833 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 1595 | 46265.2 | 54.4 |
| softirq NET_RX | 618 | 18463.8 | 49.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 340 | 7574.8 | 52.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 142 | 3097.7 | 34.7 |
| mostly unexplained (+softirq NET_RX) | 100 | 3012.8 | 50.6 |
| mostly unexplained (+local timer tick) | 32 | 781.0 | 39.2 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 45.5 | 45.5 |
| mostly unexplained (+call function (IPI/vector)) | 2 | 45.4 | 23.2 |
| loop work | 2 | 40.0 | 20.0 |
| mostly unexplained (+softirq TIMER) | 1 | 23.4 | 23.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 54.4 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, local timer tick 4, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 52.2 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 50.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 21, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 49.9 | 1 | softirq NET_RX | softirq NET_RX 29, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 48.9 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 48.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 20, preempted by kworker/1:1 12, local timer tick 4, timer tick_nohz_handler 2 |
| 7 | 48.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 21, preempted by kworker/1:1 11, local timer tick 4, timer tick_nohz_handler 2 |
| 8 | 47.4 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 47.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 46.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 23, preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2423 |
| IWI IRQ work interrupts | 2347 |
| CAL Function call interrupts | 1007 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-0/pp-recv-busy-0 (median 42.74)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 344.09 | 301.35 | 0.00 |  | 0.00 |  | 301.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 272.51 | 229.77 | 0.00 |  | 0.00 |  | 229.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 186.54 | 143.80 | 0.00 |  | 0.00 |  | 143.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 177.30 | 134.56 | 0.00 |  | 0.00 |  | 134.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 174.48 | 131.74 | 0.00 |  | 132.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 174.27 | 131.53 | 0.00 |  | 134.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 173.23 | 130.49 | 0.00 |  | 134.06 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 172.70 | 129.96 | 0.00 |  | 136.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 170.42 | 127.68 | 0.00 |  | 0.00 |  | 127.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 168.54 | 125.80 | 0.00 |  | 0.00 |  | 125.80 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 83617088 | 69.68 | 696809 | 686809 | 98.5 | 61.2 | true | 0 | 6097 | 60.5 | 2197424 / 6182 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-10000, loop 0 on CPU 1

82417088 iterations, longest 60.5 µs; over 5 / 20 / 100 µs: 2197424 / 6182 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq NET_RX | 3107 | 98225.6 | 60.1 |
| mostly unexplained (+preempted by kworker/1:1) | 2195 | 67282.0 | 60.5 |
| mostly unexplained (+softirq NET_RX) | 793 | 24853.0 | 60.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 47 | 1148.4 | 48.1 |
| mostly unexplained (+local timer tick) | 27 | 709.5 | 55.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 7 | 152.6 | 23.5 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 76.6 | 39.0 |
| mostly unexplained (+softirq TIMER) | 2 | 48.6 | 25.6 |
| mostly unexplained (+call function (IPI/vector)) | 2 | 44.1 | 23.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 60.5 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 17, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 2 | 60.1 | 1 | softirq NET_RX | softirq NET_RX 35, preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 60.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 26, preempted by kworker/1:1 10, local timer tick 5, timer tick_nohz_handler 3 |
| 4 | 60.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 26, preempted by kworker/1:1 17, local timer tick 4, timer tick_nohz_handler 2 |
| 5 | 59.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 25, preempted by kworker/1:1 9, local timer tick 5, timer tick_nohz_handler 2 |
| 6 | 59.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 26, preempted by kworker/1:1 10, local timer tick 4, timer tick_nohz_handler 3 |
| 7 | 58.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 22, preempted by kworker/1:1 7, local timer tick 5, timer tick_nohz_handler 2 |
| 8 | 58.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 26, preempted by kworker/1:1 17, local timer tick 4, timer tick_nohz_handler 3 |
| 9 | 58.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 57.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 9, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 6246 |
| IWI IRQ work interrupts | 6102 |
| CAL Function call interrupts | 1032 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-0/pp-recv-busy-10000 (median 44.84)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 542.01 | 497.17 | 0.00 |  | 0.00 |  | 497.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 180.09 | 135.25 | 0.00 |  | 0.00 |  | 135.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 164.84 | 120.00 | 0.00 |  | 0.00 |  | 120.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 141.90 | 97.06 | 0.00 |  | 0.00 |  | 97.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 141.31 | 96.47 | 0.00 |  | 0.00 |  | 96.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 138.84 | 94.00 | 0.00 |  | 0.00 |  | 94.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 137.55 | 92.71 | 0.00 |  | 0.00 |  | 92.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 136.55 | 91.71 | 0.00 |  | 153.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 135.50 | 90.66 | 0.00 |  | 0.00 |  | 90.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 135.48 | 90.64 | 0.00 |  | 0.00 |  | 90.64 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1216659 | 1216659 | 1.00 | 3649978 | 3.00 | 60833 | 75297536 | 0.0 | 13.1 | false | 0 | 0 | 62.1 | 11083 / 75 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-spin-0, loop 0 on CPU 1

4517852160 iterations, longest 62.1 µs; over 5 / 20 / 100 µs: 11083 / 75 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 49 | 1098.9 | 46.8 |
| mostly unexplained (+local timer tick) | 12 | 485.3 | 52.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 130.6 | 23.3 |
| local timer tick | 4 | 101.4 | 29.0 |
| softirq TIMER | 1 | 62.1 | 62.1 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 58.9 | 58.9 |
| mostly unexplained (+softirq TIMER) | 2 | 42.8 | 21.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 62.1 | 0 | softirq TIMER | softirq TIMER 44, local timer tick 13, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 58.9 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 52.4 | 0 | mostly unexplained (+local timer tick) | local timer tick 23, timer tick_nohz_handler 10, softirq TIMER 7, softirq RCU 1 |
| 4 | 49.4 | 0 | mostly unexplained (+local timer tick) | local timer tick 20, timer tick_nohz_handler 8, softirq TIMER 6, softirq RCU 1 |
| 5 | 47.0 | 0 | mostly unexplained (+local timer tick) | local timer tick 18, timer tick_nohz_handler 7, softirq TIMER 5, softirq RCU 1 |
| 6 | 46.8 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 41.7 | 0 | mostly unexplained (+local timer tick) | local timer tick 16, timer tick_nohz_handler 9, softirq TIMER 6, softirq RCU 1 |
| 8 | 41.3 | 0 | mostly unexplained (+local timer tick) | local timer tick 18, timer tick_nohz_handler 10, softirq TIMER 7, softirq RCU 1 |
| 9 | 39.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 16, softirq TIMER 6, timer tick_nohz_handler 6, ipi_send_cpu 0 |
| 10 | 39.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 17, timer tick_nohz_handler 8, softirq TIMER 6, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1009 |
| LOC Local timer interrupts | 64 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/pp-uring-spin-0 (median 46.49)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 228.33 | 181.84 | 0.00 |  | 0.00 |  | 181.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 224.57 | 178.08 | 0.00 |  | 0.00 |  | 178.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 219.04 | 172.55 | 0.00 |  | 0.00 |  | 172.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 214.94 | 168.45 | 0.00 |  | 0.00 |  | 168.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 213.96 | 167.47 | 0.00 |  | 0.00 |  | 167.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 196.99 | 150.50 | 0.00 |  | 196.23 | softirq TIMER | 0.00 | server: softirq TIMER |
| 7 | 195.75 | 149.26 | 0.00 |  | 0.00 |  | 149.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 183.92 | 137.43 | 0.00 |  | 0.00 |  | 137.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 182.53 | 136.04 | 0.00 |  | 0.00 |  | 136.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 181.22 | 134.73 | 0.00 |  | 0.00 |  | 134.73 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199999 | 1199999 | 1.00 | 3599998 | 3.00 | 30000 | 60558559 | 0.0 | 6.7 | false | 0 | 0 | 56.7 | 26173 / 85 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-spin-10000, loop 0 on CPU 1

7267027072 iterations, longest 56.7 µs; over 5 / 20 / 100 µs: 26173 / 85 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 47 | 1159.5 | 50.2 |
| mostly unexplained (+local timer tick) | 18 | 808.2 | 56.7 |
| local timer tick | 12 | 285.4 | 26.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 83.3 | 21.8 |
| mostly unexplained (+softirq TIMER) | 3 | 74.2 | 27.6 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 50.5 | 50.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 56.7 | 0 | mostly unexplained (+local timer tick) | local timer tick 24, timer tick_nohz_handler 10, softirq TIMER 7, softirq RCU 1 |
| 2 | 54.0 | 0 | mostly unexplained (+local timer tick) | local timer tick 21, timer tick_nohz_handler 9, softirq TIMER 7, softirq RCU 1 |
| 3 | 53.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 22, timer tick_nohz_handler 11, softirq TIMER 7, softirq RCU 1 |
| 4 | 53.4 | 0 | mostly unexplained (+local timer tick) | local timer tick 21, timer tick_nohz_handler 11, softirq TIMER 7, softirq RCU 1 |
| 5 | 53.3 | 0 | mostly unexplained (+local timer tick) | local timer tick 21, timer tick_nohz_handler 9, softirq TIMER 7, softirq RCU 1 |
| 6 | 50.5 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 50.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 49.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 19, softirq TIMER 7, timer tick_nohz_handler 7, softirq RCU 1 |
| 9 | 48.1 | 0 | mostly unexplained (+local timer tick) | local timer tick 20, timer tick_nohz_handler 9, softirq TIMER 7, ipi_send_cpu 0 |
| 10 | 47.2 | 0 | mostly unexplained (+local timer tick) | local timer tick 19, timer tick_nohz_handler 8, softirq TIMER 6, softirq RCU 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1034 |
| LOC Local timer interrupts | 78 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 3 |

#### Slowest round trips: r1-base-0/pp-uring-spin-10000 (median 59.25)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 231.56 | 172.31 | 0.00 |  | 0.00 |  | 172.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 225.85 | 166.60 | 0.00 |  | 0.00 |  | 166.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 225.55 | 166.30 | 0.00 |  | 0.00 |  | 166.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 224.11 | 164.86 | 0.00 |  | 0.00 |  | 164.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 223.75 | 164.50 | 0.00 |  | 0.00 |  | 164.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 223.26 | 164.01 | 0.00 |  | 0.00 |  | 164.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 223.23 | 163.98 | 0.00 |  | 0.00 |  | 163.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 222.65 | 163.40 | 0.00 |  | 0.00 |  | 163.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 220.35 | 161.10 | 0.00 |  | 0.00 |  | 161.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 219.82 | 160.57 | 0.00 |  | 0.00 |  | 160.57 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1392161 | 1392161 | 1.00 | 1 | 0.00 | 0 | 87649145 | 0.0 | 0.0 | true | 0 | 0 | 43.5 | 5088 / 29 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-0, loop 0 on CPU 1

5258948672 iterations, longest 43.5 µs; over 5 / 20 / 100 µs: 5088 / 29 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 13 | 325.1 | 43.5 |
| local timer tick | 9 | 222.1 | 28.1 |
| mostly unexplained (+local timer tick) | 6 | 181.3 | 35.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 43.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 35.9 | 1 | mostly unexplained (+local timer tick) | local timer tick 15, timer tick_nohz_handler 7, softirq TIMER 5, ipi_send_cpu 0 |
| 3 | 35.0 | 0 | mostly unexplained (+local timer tick) | local timer tick 13, softirq TIMER 6, timer tick_nohz_handler 5, ipi_send_cpu 0 |
| 4 | 34.4 | 1 | mostly unexplained (+local timer tick) | local timer tick 13, timer tick_nohz_handler 6, softirq TIMER 4, ipi_send_cpu 0 |
| 5 | 32.0 | 1 | mostly unexplained (+local timer tick) | local timer tick 11, softirq TIMER 5, timer tick_nohz_handler 5, ipi_send_cpu 0 |
| 6 | 28.1 | 0 | local timer tick | local timer tick 15, timer tick_nohz_handler 8 |
| 7 | 26.6 | 1 | local timer tick | local timer tick 14, timer tick_nohz_handler 8 |
| 8 | 26.6 | 0 | local timer tick | local timer tick 14, timer tick_nohz_handler 7 |
| 9 | 25.9 | 1 | local timer tick | local timer tick 13, timer tick_nohz_handler 8 |
| 10 | 25.9 | 1 | local timer tick | local timer tick 14, timer tick_nohz_handler 7 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1013 |
| LOC Local timer interrupts | 48 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-0 (median 42.57)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 276.68 | 234.11 | 0.00 |  | 0.00 |  | 234.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 215.11 | 172.54 | 0.00 |  | 0.00 |  | 172.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 199.45 | 156.88 | 0.00 |  | 0.00 |  | 156.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 198.43 | 155.86 | 0.00 |  | 0.00 |  | 155.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 183.01 | 140.44 | 0.00 |  | 138.45 | unexplained: no kernel event (hypervisor or in-kernel time) | 1.99 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 180.83 | 138.26 | 0.00 |  | 130.89 | page faults | 7.37 | server: page faults |
| 7 | 180.16 | 137.59 | 0.00 |  | 160.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 175.37 | 132.80 | 0.00 |  | 132.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.05 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 173.76 | 131.19 | 0.00 |  | 144.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 172.58 | 130.01 | 0.00 |  | 134.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-0/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199999 | 1199999 | 1.00 | 1 | 0.00 | 0 | 54848121 | 0.0 | 0.0 | true | 0 | 0 | 53.8 | 10524 / 56 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-10000, loop 0 on CPU 1

6581774464 iterations, longest 53.8 µs; over 5 / 20 / 100 µs: 10524 / 56 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 26 | 657.9 | 44.2 |
| mostly unexplained (+local timer tick) | 20 | 556.1 | 38.5 |
| local timer tick | 9 | 219.5 | 27.7 |
| softirq TIMER | 1 | 53.8 | 53.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 53.8 | 0 | softirq TIMER | softirq TIMER 41, local timer tick 9, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 44.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 44.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 38.5 | 0 | mostly unexplained (+local timer tick) | local timer tick 15, softirq TIMER 7, timer tick_nohz_handler 7, ipi_send_cpu 0 |
| 5 | 37.5 | 0 | mostly unexplained (+local timer tick) | local timer tick 15, timer tick_nohz_handler 7, softirq TIMER 5, ipi_send_cpu 0 |
| 6 | 35.5 | 0 | mostly unexplained (+local timer tick) | local timer tick 14, timer tick_nohz_handler 8, softirq TIMER 6, ipi_send_cpu 0 |
| 7 | 34.0 | 0 | mostly unexplained (+local timer tick) | local timer tick 11, softirq TIMER 5, timer tick_nohz_handler 5, ipi_send_cpu 0 |
| 8 | 33.3 | 0 | mostly unexplained (+local timer tick) | local timer tick 12, timer tick_nohz_handler 6, softirq TIMER 4, ipi_send_cpu 0 |
| 9 | 32.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 11, softirq TIMER 6, timer tick_nohz_handler 4, ipi_send_cpu 0 |
| 10 | 31.8 | 0 | mostly unexplained (+local timer tick) | local timer tick 13, softirq TIMER 5, timer tick_nohz_handler 5, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1041 |
| LOC Local timer interrupts | 71 |
| IWI IRQ work interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-10000 (median 44.53)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 338.90 | 294.37 | 0.00 |  | 0.00 |  | 294.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 327.97 | 283.44 | 0.00 |  | 0.00 |  | 283.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 228.07 | 183.54 | 0.00 |  | 0.00 |  | 183.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 211.88 | 167.35 | 0.00 |  | 0.00 |  | 167.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 209.69 | 165.16 | 0.00 |  | 0.00 |  | 165.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 199.70 | 155.17 | 0.00 |  | 204.74 | softirq TIMER | 0.00 | server: softirq TIMER |
| 7 | 192.71 | 148.18 | 0.00 |  | 0.00 |  | 148.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 170.93 | 126.40 | 0.00 |  | 151.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 161.81 | 117.28 | 0.00 |  | 0.00 |  | 117.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 158.97 | 114.44 | 0.00 |  | 148.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

#### Stalls: r1-base-1/http-recv-busy-10000, loop 0 on CPU 1

2257987558 iterations, longest 90.5 µs; over 5 / 20 / 100 µs: 970959 / 13404 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4876 | 109514.9 | 37.0 |
| softirq NET_RX | 2829 | 92281.5 | 58.1 |
| mostly unexplained (+softirq NET_RX) | 3340 | 85153.5 | 90.5 |
| mostly unexplained (+preempted by kworker/1:1) | 2210 | 70055.3 | 64.8 |
| mostly unexplained (+local timer tick) | 74 | 1764.0 | 35.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 41 | 1103.9 | 45.3 |
| loop work | 28 | 569.0 | 29.0 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 72.1 | 40.9 |
| softirq TIMER | 1 | 56.5 | 56.5 |
| mostly unexplained (+softirq TIMER) | 2 | 43.8 | 23.1 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 23.5 | 23.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 90.5 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 64.8 | 2 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 63.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 59.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 24, preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 58.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 24, preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 58.1 | 1 | softirq NET_RX | softirq NET_RX 35, preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 57.4 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 56.5 | 1 | softirq TIMER | softirq TIMER 43, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 9 | 56.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 24, preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 54.8 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 6178 |
| IWI IRQ work interrupts | 6036 |
| CAL Function call interrupts | 1032 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-1/http-recv-busy-10000 (median 44.18)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 303.51 | 259.33 | 0.00 |  | 0.00 |  | 259.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 212.63 | 168.45 | 0.00 |  | 206.36 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 184.20 | 140.02 | 0.00 |  | 0.00 |  | 140.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 170.31 | 126.13 | 0.00 |  | 0.00 |  | 126.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 165.65 | 121.47 | 0.00 |  | 0.00 |  | 121.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 164.94 | 120.76 | 0.00 |  | 0.00 |  | 120.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 164.00 | 119.82 | 0.00 |  | 0.00 |  | 119.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 163.07 | 118.89 | 0.00 |  | 0.00 |  | 118.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 160.92 | 116.74 | 0.00 |  | 0.00 |  | 116.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 158.97 | 114.79 | 0.00 |  | 0.00 |  | 114.79 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-1/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=578015 clocksource=tsc

### Loop counters (r1-base-1/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3003600 | 614608 | 4.89 | 42728576 | 14.23 | 712143 | 712143 | 98.6 | 60.2 | true | 0 | 938 | 799.2 | 606853 / 8132 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672806400 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (72945 of 2900000 in 1-msg reads) | 72945 | 4.52 | 4.24 | 4.94 | 9.28 | 31.42 | 43.06 | 50.86 |

#### Stalls: r1-base-1/md-recv-busy, loop 0 on CPU 1

42728576 iterations, longest 799.2 µs; over 5 / 20 / 100 µs: 606853 / 8132 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 7667 | 307316.2 | 799.2 |
| softirq NET_RX | 149 | 5729.7 | 59.3 |
| mostly unexplained (+preempted by kworker/1:1) | 151 | 5486.9 | 70.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 98 | 2877.9 | 35.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 46 | 1073.0 | 45.8 |
| mostly unexplained (+local timer tick) | 14 | 383.9 | 34.7 |
| mostly unexplained (+softirq NET_RX) | 5 | 206.3 | 60.1 |
| mostly unexplained (+softirq TIMER) | 2 | 49.5 | 28.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 799.2 | 3600 | loop work (3600 items) |  |
| 2 | 147.0 | 600 | loop work (600 items) |  |
| 3 | 91.2 | 14 | loop work (14 items) | preempted by kworker/1:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 87.4 | 25 | loop work (25 items) | softirq NET_RX 25, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 85.5 | 26 | loop work (26 items) | softirq NET_RX 25, preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 85.2 | 26 | loop work (26 items) | softirq NET_RX 31, preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 82.7 | 14 | loop work (14 items) | softirq NET_RX 24, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 81.0 | 23 | loop work (23 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 80.2 | 14 | loop work (14 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 79.5 | 13 | loop work (13 items) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1023 |
| LOC Local timer interrupts | 961 |
| IWI IRQ work interrupts | 944 |
| RES Rescheduling interrupts | 2 |
| TLB TLB shootdowns | 2 |

## r1-base-1/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-1/noise, loop 0 on CPU 1

12654247936 iterations, longest 214.6 µs; over 5 / 20 / 100 µs: 14993 / 69 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 14888 | 142868.6 | 214.6 |
| mostly unexplained (+local timer tick) | 33 | 958.9 | 45.2 |
| mostly unexplained (+call function single (IPI/vector)) | 30 | 498.8 | 19.6 |
| local timer tick | 9 | 213.5 | 26.4 |
| mostly unexplained (+call function (IPI/vector)) | 20 | 209.6 | 11.4 |
| softirq TIMER | 8 | 178.8 | 71.1 |
| mostly unexplained (+softirq TIMER) | 5 | 82.4 | 21.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 214.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 124.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 71.1 | 0 | softirq TIMER | softirq TIMER 52, local timer tick 13, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 4 | 47.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 45.2 | 0 | mostly unexplained (+local timer tick) | local timer tick 19, timer tick_nohz_handler 10, softirq TIMER 9, ipi_send_cpu 0 |
| 6 | 44.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 19, timer tick_nohz_handler 10, softirq TIMER 9, ipi_send_cpu 0 |
| 7 | 44.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 44.4 | 0 | mostly unexplained (+local timer tick) | local timer tick 19, timer tick_nohz_handler 10, softirq TIMER 8, ipi_send_cpu 0 |
| 9 | 43.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 18, softirq TIMER 9, timer tick_nohz_handler 9, ipi_send_cpu 0 |
| 10 | 43.3 | 0 | mostly unexplained (+local timer tick) | local timer tick 19, timer tick_nohz_handler 11, softirq TIMER 8, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1029 |
| LOC Local timer interrupts | 55 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-base-1/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1214191 | 1214191 | 1.00 | 80564319 | 66.35 | 1342739 | 1302266 | 0.0 | 30.8 | false | 0 | 0 | 55.6 | 23467 / 94 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-epoll-spin-0, loop 0 on CPU 1

78135936 iterations, longest 55.6 µs; over 5 / 20 / 100 µs: 23467 / 94 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 63 | 1402.8 | 47.7 |
| mostly unexplained (+local timer tick) | 8 | 275.6 | 55.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 11 | 245.2 | 36.1 |
| local timer tick | 7 | 179.2 | 28.6 |
| mostly unexplained (+softirq TIMER) | 4 | 88.3 | 24.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 39.9 | 39.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 55.6 | 1 | mostly unexplained (+local timer tick) | local timer tick 23, timer tick_nohz_handler 11, softirq TIMER 7, softirq RCU 1 |
| 2 | 47.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 43.1 | 1 | mostly unexplained (+local timer tick) | local timer tick 17, timer tick_nohz_handler 9, softirq TIMER 8, softirq RCU 1 |
| 4 | 39.9 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 2, ipi_send_cpu 0 |
| 5 | 36.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 34.1 | 1 | mostly unexplained (+local timer tick) | local timer tick 13, timer tick_nohz_handler 9, softirq TIMER 7, softirq RCU 1 |
| 7 | 32.7 | 1 | mostly unexplained (+local timer tick) | local timer tick 12, softirq TIMER 8, timer tick_nohz_handler 8, softirq RCU 1 |
| 8 | 32.4 | 0 | mostly unexplained (+local timer tick) | local timer tick 13, softirq TIMER 9, timer tick_nohz_handler 8, ipi_send_cpu 0 |
| 9 | 29.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 28.6 | 0 | local timer tick | local timer tick 15, timer tick_nohz_handler 7, softirq RCU 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1006 |
| LOC Local timer interrupts | 63 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-1/pp-epoll-spin-0 (median 48.97)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 550.74 | 501.77 | 0.00 |  | 0.00 |  | 501.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 235.68 | 186.71 | 0.00 |  | 0.00 |  | 186.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 210.14 | 161.17 | 0.00 |  | 0.00 |  | 161.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 207.41 | 158.44 | 0.00 |  | 0.00 |  | 158.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 191.62 | 142.65 | 0.00 |  | 0.00 |  | 142.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 188.34 | 139.37 | 0.00 |  | 0.00 |  | 139.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 186.07 | 137.10 | 0.00 |  | 0.00 |  | 137.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 182.76 | 133.79 | 0.00 |  | 0.00 |  | 133.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 182.64 | 133.67 | 0.00 |  | 0.00 |  | 133.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 178.93 | 129.96 | 0.00 |  | 0.00 |  | 129.96 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1199999 | 1199999 | 1.00 | 167367167 | 139.47 | 1394726 | 1374726 | 0.0 | 27.5 | false | 0 | 0 | 289.3 | 61639 / 170 / 27 | 0 | 0 |

#### Stalls: r1-base-1/pp-epoll-spin-10000, loop 0 on CPU 1

164967168 iterations, longest 289.3 µs; over 5 / 20 / 100 µs: 61639 / 170 / 27

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 115 | 7722.8 | 289.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 18 | 665.8 | 150.6 |
| mostly unexplained (+local timer tick) | 19 | 652.4 | 48.6 |
| local timer tick | 11 | 278.8 | 29.8 |
| mostly unexplained (+softirq TIMER) | 6 | 136.4 | 25.1 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 38.8 | 38.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 289.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 285.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 284.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 282.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 275.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 265.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 259.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 248.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 242.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 233.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1032 |
| LOC Local timer interrupts | 72 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-1/pp-epoll-spin-10000 (median 46.82)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1016.73 | 969.90 | 0.00 |  | 0.00 |  | 969.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 339.15 | 292.33 | 0.00 |  | 0.00 |  | 292.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 330.91 | 284.09 | 0.00 |  | 0.00 |  | 284.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 328.70 | 281.88 | 0.00 |  | 0.00 |  | 281.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 328.66 | 281.84 | 0.00 |  | 0.00 |  | 281.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 328.36 | 281.54 | 0.00 |  | 0.00 |  | 281.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 328.23 | 281.41 | 0.00 |  | 0.00 |  | 281.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 328.18 | 281.36 | 0.00 |  | 0.00 |  | 281.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 324.65 | 277.83 | 0.00 |  | 0.00 |  | 277.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 322.38 | 275.56 | 0.00 |  | 0.00 |  | 275.56 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1378687 | 1378687 | 1.00 | 36803840 | 26.69 | 613397 | 590419 | 96.1 | 66.2 | true | 0 | 2192 | 171.0 | 1493258 / 2791 / 2 | 0 | 0 |

#### Stalls: r1-base-1/pp-recv-busy-0, loop 0 on CPU 1

35425152 iterations, longest 171.0 µs; over 5 / 20 / 100 µs: 1493258 / 2791 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 1362 | 38931.1 | 54.1 |
| softirq NET_RX | 697 | 21445.6 | 49.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 418 | 9391.0 | 48.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 172 | 4120.6 | 171.0 |
| mostly unexplained (+softirq NET_RX) | 98 | 3018.8 | 51.6 |
| mostly unexplained (+local timer tick) | 36 | 906.0 | 45.9 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 127.3 | 46.1 |
| mostly unexplained (+call function (IPI/vector)) | 2 | 45.5 | 23.1 |
| loop work | 2 | 40.0 | 20.0 |
| mostly unexplained (+softirq TIMER) | 1 | 26.2 | 26.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 171.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 164.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 59.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 54.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, local timer tick 3, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 5 | 51.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 22, preempted by kworker/1:1 5, local timer tick 4, timer tick_nohz_handler 2 |
| 6 | 50.1 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 22, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 49.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 11, local timer tick 4, irq work (IPI/vector) 1, softirq RCU 1 |
| 8 | 49.2 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 49.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 49.0 | 1 | softirq NET_RX | softirq NET_RX 25, preempted by kworker/1:1 9, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2265 |
| IWI IRQ work interrupts | 2198 |
| CAL Function call interrupts | 1006 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-1/pp-recv-busy-0 (median 43.10)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 316.92 | 273.82 | 223.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 50.14 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 296.95 | 253.85 | 0.00 |  | 0.00 |  | 253.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 200.28 | 157.18 | 0.00 |  | 0.00 |  | 157.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 180.41 | 137.31 | 170.78 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 171.99 | 128.89 | 0.00 |  | 0.00 |  | 128.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 162.66 | 119.56 | 0.00 |  | 0.00 |  | 119.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 161.36 | 118.26 | 0.00 |  | 0.00 |  | 118.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 159.33 | 116.23 | 105.03 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 11.20 | client: mostly unexplained (+softirq NET_RX) |
| 9 | 158.43 | 115.33 | 0.00 |  | 0.00 |  | 115.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 157.75 | 114.65 | 0.00 |  | 0.00 |  | 114.65 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1199999 | 1199999 | 1.00 | 83147200 | 69.29 | 692893 | 682893 | 98.5 | 61.4 | true | 0 | 2001 | 276.1 | 2367857 / 2176 / 7 | 0 | 0 |

#### Stalls: r1-base-1/pp-recv-busy-10000, loop 0 on CPU 1

81947200 iterations, longest 276.1 µs; over 5 / 20 / 100 µs: 2367857 / 2176 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq NET_RX | 942 | 32605.8 | 276.1 |
| mostly unexplained (+preempted by kworker/1:1) | 998 | 31203.5 | 169.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 122 | 3025.7 | 120.8 |
| mostly unexplained (+softirq NET_RX) | 59 | 2037.5 | 80.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 22 | 893.9 | 227.5 |
| mostly unexplained (+local timer tick) | 26 | 688.0 | 39.9 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 69.8 | 37.5 |
| mostly unexplained (+softirq TIMER) | 3 | 68.1 | 23.9 |
| mostly unexplained (+call function (IPI/vector)) | 2 | 47.0 | 26.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 276.1 | 0 | softirq NET_RX | softirq NET_RX 251, preempted by kworker/1:1 13, local timer tick 4, irq work (IPI/vector) 1 |
| 2 | 227.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 169.8 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 149.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 120.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 110.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 107.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 82.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 80.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 10 | 66.2 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 32, preempted by kworker/1:1 11, local timer tick 6, timer tick_nohz_handler 3 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2044 |
| IWI IRQ work interrupts | 2006 |
| CAL Function call interrupts | 1032 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-recv-busy-10000 (median 44.38)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 535.92 | 491.54 | 0.00 |  | 0.00 |  | 491.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 309.70 | 265.32 | 0.00 |  | 0.00 |  | 265.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 302.29 | 257.91 | 0.00 |  | 0.00 |  | 257.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 297.10 | 252.72 | 0.00 |  | 0.00 |  | 252.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 281.20 | 236.82 | 0.00 |  | 0.00 |  | 236.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 277.34 | 232.96 | 82.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 150.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 266.12 | 221.74 | 227.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 263.65 | 219.27 | 0.00 |  | 0.00 |  | 219.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 261.68 | 217.30 | 0.00 |  | 0.00 |  | 217.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 231.50 | 187.12 | 0.00 |  | 0.00 |  | 187.12 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1238042 | 1238042 | 1.00 | 3714127 | 3.00 | 61902 | 73553346 | 0.0 | 13.3 | false | 0 | 0 | 287.8 | 11162 / 123 / 22 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-spin-0, loop 0 on CPU 1

4413200768 iterations, longest 287.8 µs; over 5 / 20 / 100 µs: 11162 / 123 / 22

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 89 | 4917.8 | 240.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 16 | 1347.5 | 287.8 |
| mostly unexplained (+local timer tick) | 8 | 349.4 | 51.8 |
| local timer tick | 7 | 162.6 | 26.7 |
| softirq TIMER | 1 | 59.6 | 59.6 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 56.0 | 56.0 |
| mostly unexplained (+softirq TIMER) | 1 | 22.6 | 22.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 287.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 240.7 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 238.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 228.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 215.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 204.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 201.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 193.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 172.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 162.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1009 |
| LOC Local timer interrupts | 56 |
| IWI IRQ work interrupts | 7 |
| RES Rescheduling interrupts | 3 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-1/pp-uring-spin-0 (median 47.26)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 338.79 | 291.53 | 0.00 |  | 0.00 |  | 291.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 331.59 | 284.33 | 0.00 |  | 0.00 |  | 284.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 328.34 | 281.08 | 0.00 |  | 0.00 |  | 281.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 324.87 | 277.61 | 287.76 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 324.26 | 277.00 | 0.00 |  | 0.00 |  | 277.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 324.01 | 276.75 | 0.00 |  | 0.00 |  | 276.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 323.86 | 276.60 | 0.00 |  | 0.00 |  | 276.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 323.85 | 276.59 | 0.00 |  | 0.00 |  | 276.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 323.46 | 276.20 | 0.00 |  | 0.00 |  | 276.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 322.22 | 274.96 | 0.00 |  | 0.00 |  | 274.96 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199999 | 1199999 | 1.00 | 3599998 | 3.00 | 30000 | 52848994 | 0.0 | 6.7 | false | 0 | 0 | 84.5 | 45056 / 219 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-spin-10000, loop 0 on CPU 1

6341879232 iterations, longest 84.5 µs; over 5 / 20 / 100 µs: 45056 / 219 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 170 | 3793.2 | 84.5 |
| mostly unexplained (+local timer tick) | 19 | 795.0 | 52.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 16 | 342.1 | 23.9 |
| local timer tick | 12 | 274.9 | 23.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 51.0 | 51.0 |
| mostly unexplained (+softirq TIMER) | 1 | 20.7 | 20.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 84.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 52.4 | 0 | mostly unexplained (+local timer tick) | local timer tick 23, timer tick_nohz_handler 13, softirq TIMER 7, softirq RCU 1 |
| 3 | 52.1 | 0 | mostly unexplained (+local timer tick) | local timer tick 22, timer tick_nohz_handler 10, softirq TIMER 6, softirq RCU 2 |
| 4 | 51.6 | 0 | mostly unexplained (+local timer tick) | local timer tick 22, timer tick_nohz_handler 10, softirq TIMER 7, softirq RCU 2 |
| 5 | 51.0 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 50.8 | 0 | mostly unexplained (+local timer tick) | local timer tick 20, timer tick_nohz_handler 9, softirq TIMER 7, softirq RCU 1 |
| 7 | 50.5 | 0 | mostly unexplained (+local timer tick) | local timer tick 21, timer tick_nohz_handler 9, softirq TIMER 7, softirq RCU 1 |
| 8 | 50.2 | 0 | mostly unexplained (+local timer tick) | local timer tick 20, timer tick_nohz_handler 10, softirq TIMER 7, softirq RCU 1 |
| 9 | 48.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 47.7 | 0 | mostly unexplained (+local timer tick) | local timer tick 19, timer tick_nohz_handler 8, softirq TIMER 6, softirq RCU 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1034 |
| LOC Local timer interrupts | 76 |
| IWI IRQ work interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-uring-spin-10000 (median 46.64)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 466.34 | 419.70 | 0.00 |  | 0.00 |  | 419.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 432.74 | 386.10 | 0.00 |  | 0.00 |  | 386.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 344.36 | 297.72 | 0.00 |  | 0.00 |  | 297.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 337.17 | 290.53 | 0.00 |  | 0.00 |  | 290.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 326.26 | 279.62 | 0.00 |  | 0.00 |  | 279.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 319.81 | 273.17 | 0.00 |  | 0.00 |  | 273.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 313.48 | 266.84 | 0.00 |  | 0.00 |  | 266.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 308.56 | 261.92 | 0.00 |  | 0.00 |  | 261.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 293.61 | 246.97 | 84.46 | VM paused: the idle spare core stalled too (hypervisor) | 0.00 |  | 162.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 290.60 | 243.96 | 0.00 |  | 0.00 |  | 243.96 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1364189 | 1364189 | 1.00 | 1 | 0.00 | 0 | 87383895 | 0.0 | 0.0 | true | 0 | 0 | 93.6 | 5037 / 32 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-0, loop 0 on CPU 1

5243033728 iterations, longest 93.6 µs; over 5 / 20 / 100 µs: 5037 / 32 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 16 | 470.5 | 93.6 |
| mostly unexplained (+local timer tick) | 12 | 317.1 | 36.6 |
| local timer tick | 3 | 77.6 | 27.5 |
| mostly unexplained (+softirq TIMER) | 1 | 20.4 | 20.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 93.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 43.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 36.6 | 0 | mostly unexplained (+local timer tick) | local timer tick 15, timer tick_nohz_handler 7, softirq TIMER 5, ipi_send_cpu 0 |
| 4 | 35.4 | 0 | mostly unexplained (+local timer tick) | local timer tick 14, timer tick_nohz_handler 7, softirq TIMER 5, ipi_send_cpu 0 |
| 5 | 27.5 | 0 | local timer tick | local timer tick 14, timer tick_nohz_handler 8 |
| 6 | 26.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 13, timer tick_nohz_handler 7 |
| 7 | 25.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 25.5 | 1 | mostly unexplained (+local timer tick) | local timer tick 12, timer tick_nohz_handler 7 |
| 9 | 25.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 25.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1011 |
| LOC Local timer interrupts | 37 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-0 (median 43.46)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 316.03 | 272.57 | 0.00 |  | 0.00 |  | 272.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 224.33 | 180.87 | 0.00 |  | 0.00 |  | 180.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 218.44 | 174.98 | 0.00 |  | 0.00 |  | 174.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 204.66 | 161.20 | 0.00 |  | 0.00 |  | 161.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 172.48 | 129.02 | 0.00 |  | 0.00 |  | 129.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 168.13 | 124.67 | 0.00 |  | 0.00 |  | 124.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 166.72 | 123.26 | 0.00 |  | 0.00 |  | 123.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 166.71 | 123.25 | 0.00 |  | 0.00 |  | 123.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 165.43 | 121.97 | 0.00 |  | 0.00 |  | 121.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 161.80 | 118.34 | 0.00 |  | 0.00 |  | 118.34 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199999 | 1199999 | 1.00 | 1 | 0.00 | 0 | 54601460 | 0.0 | 0.0 | true | 0 | 0 | 272.0 | 9363 / 71 / 6 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-10000, loop 0 on CPU 1

6552175168 iterations, longest 272.0 µs; over 5 / 20 / 100 µs: 9363 / 71 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 38 | 1918.1 | 272.0 |
| mostly unexplained (+local timer tick) | 16 | 480.4 | 37.0 |
| local timer tick | 14 | 352.2 | 27.9 |
| softirq TIMER | 1 | 52.2 | 52.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 43.7 | 22.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 272.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 246.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 163.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 148.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 139.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 127.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 52.2 | 0 | softirq TIMER | softirq TIMER 40, local timer tick 8, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 8 | 45.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 44.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 37.0 | 0 | mostly unexplained (+local timer tick) | local timer tick 15, timer tick_nohz_handler 7, softirq TIMER 6, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1038 |
| LOC Local timer interrupts | 70 |
| IWI IRQ work interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-10000 (median 43.95)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 419.13 | 375.18 | 0.00 |  | 0.00 |  | 375.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 402.86 | 358.91 | 0.00 |  | 0.00 |  | 358.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 400.31 | 356.36 | 0.00 |  | 0.00 |  | 356.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 390.50 | 346.55 | 0.00 |  | 0.00 |  | 346.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 388.02 | 344.07 | 0.00 |  | 0.00 |  | 344.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 386.73 | 342.78 | 0.00 |  | 0.00 |  | 342.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 370.65 | 326.70 | 0.00 |  | 0.00 |  | 326.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 364.51 | 320.56 | 0.00 |  | 0.00 |  | 320.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 358.00 | 314.05 | 0.00 |  | 0.00 |  | 314.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 351.66 | 307.71 | 0.00 |  | 0.00 |  | 307.71 | neither loop stalled long enough: network, hypervisor or untraced |

## Ping-pong round trip (client send → echo decoded; "from when due": from the slot on the paced schedule, including any wait behind a stall)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189906 | 44.88 | 44.16 | 47.26 | 62.47 | 76.58 | 111.86 | 335.75 |
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189906 | 45.05 | 44.33 | 47.43 | 62.71 | 76.93 | 115.30 | 335.91 |
| r1-base-0/pp-epoll-spin-0: epoll-spin @ 0/s | 1191366 | 49.38 | 48.96 | 49.93 | 93.54 | 98.49 | 118.65 | 331.14 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 49.83 | 46.67 | 54.41 | 92.60 | 97.38 | 173.21 | 580.71 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 50.25 | 47.09 | 54.79 | 93.03 | 98.44 | 176.28 | 581.05 |
| r1-base-0/pp-recv-busy-0: recv-busy @ 0/s | 1366642 | 43.05 | 42.74 | 45.55 | 50.73 | 66.39 | 97.69 | 344.09 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 45.33 | 44.84 | 47.55 | 54.44 | 74.89 | 112.95 | 542.01 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 46.04 | 45.58 | 48.33 | 55.30 | 75.84 | 114.75 | 542.27 |
| r1-base-0/pp-uring-spin-0: uring-spin @ 0/s | 1196297 | 47.59 | 46.49 | 52.29 | 60.92 | 96.71 | 148.88 | 228.33 |
| r1-base-0/pp-uring-spin-10000: uring-spin @ 10000/s | 1189998 | 62.31 | 59.25 | 86.28 | 95.29 | 100.98 | 125.52 | 231.56 |
| r1-base-0/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189998 | 62.40 | 59.34 | 86.38 | 95.39 | 101.48 | 128.19 | 231.65 |
| r1-base-0/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1369055 | 43.00 | 42.57 | 45.55 | 51.77 | 63.16 | 94.99 | 276.68 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189997 | 44.96 | 44.53 | 47.35 | 54.05 | 66.12 | 110.78 | 338.90 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189997 | 45.04 | 44.59 | 47.41 | 54.21 | 66.62 | 125.78 | 767.30 |
| r1-base-1/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189904 | 44.86 | 44.18 | 47.14 | 60.73 | 75.05 | 89.42 | 303.51 |
| r1-base-1/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189904 | 45.10 | 44.37 | 47.51 | 61.23 | 75.47 | 91.27 | 303.66 |
| r1-base-1/pp-epoll-spin-0: epoll-spin @ 0/s | 1193350 | 49.31 | 48.97 | 49.84 | 92.85 | 98.35 | 113.71 | 550.74 |
| r1-base-1/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189998 | 49.27 | 46.82 | 51.71 | 92.34 | 97.22 | 261.50 | 1016.73 |
| r1-base-1/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189998 | 49.81 | 47.27 | 52.27 | 92.93 | 120.63 | 282.60 | 1017.44 |
| r1-base-1/pp-recv-busy-0: recv-busy @ 0/s | 1355714 | 43.41 | 43.10 | 45.99 | 51.39 | 66.75 | 104.01 | 316.92 |
| r1-base-1/pp-recv-busy-10000: recv-busy @ 10000/s | 1189998 | 44.68 | 44.38 | 47.00 | 51.51 | 71.20 | 146.97 | 535.92 |
| r1-base-1/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189998 | 45.40 | 45.02 | 47.72 | 52.38 | 75.37 | 153.66 | 536.78 |
| r1-base-1/pp-uring-spin-0: uring-spin @ 0/s | 1216800 | 46.74 | 47.26 | 48.23 | 66.92 | 96.66 | 253.08 | 338.79 |
| r1-base-1/pp-uring-spin-10000: uring-spin @ 10000/s | 1189998 | 49.87 | 46.64 | 52.37 | 92.39 | 96.28 | 165.93 | 466.34 |
| r1-base-1/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189998 | 50.14 | 46.75 | 52.76 | 92.59 | 99.74 | 452.51 | 1860.14 |
| r1-base-1/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1341497 | 43.87 | 43.46 | 46.41 | 52.34 | 64.03 | 100.89 | 316.03 |
| r1-base-1/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 44.69 | 43.95 | 46.69 | 59.67 | 118.00 | 237.90 | 419.13 |
| r1-base-1/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 46.45 | 44.01 | 46.76 | 60.72 | 819.18 | 1193.68 | 1442.26 |

