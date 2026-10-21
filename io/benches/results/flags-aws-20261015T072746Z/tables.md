# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|
| r1-base-0 | 73.64 | 26.0 | 1.0 | 0.0 | 73.64 µs stall: softirq TIMER |
| r1-base-1 | 80.98 | 25.5 | 1.0 | 0.0 | 80.98 µs stall: softirq TIMER |

## HTTP round trip, http-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189944 | 40.10 | 51.44 | 65.28 | 75.35 | 717.85 | 5.5 | 0.5 | 0.0 | 0.0 | 707.85 | 3148.5 | client: softirq TIMER |
| r1-base-1 | 1189944 | 39.93 | 50.33 | 64.00 | 80.07 | 549.39 | 46.4 | 1.5 | 0.0 | 0.0 | 445.32 | 2071.0 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189944 | 40.27 | 51.62 | 65.48 | 76.45 | 718.02 | 12.1 | 2.5 | 0.0 | 0.0 |
| r1-base-1 | 1189944 | 40.08 | 50.54 | 64.58 | 134.74 | 1094.60 | 87.7 | 32.8 | 2.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 2900000 | 2.72 | 6.89 | 18.84 | 26.63 | 43.73 | 0.0 | 0.0 | 0.0 | 0.0 | 455.58 | 4524.0 | 455.58 µs stall: loop work (2500 items) |
| r1-base-1 | 2899950 | 2.41 | 12.01 | 20.27 | 32.64 | 49.53 | 0.0 | 0.0 | 0.0 | 0.0 | 638.75 | 7588.0 | 638.75 µs stall: loop work (2500 items) |

## WebSocket round trip, pp-epoll-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1219216 | 48.98 | 59.47 | 97.56 | 188.48 | 372.55 | 1037.3 | 0.0 | 0.0 | 0.0 | 131.02 | 24.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1196712 | 49.05 | 58.91 | 92.98 | 97.97 | 130.79 | 58.0 | 0.0 | 0.0 | 0.0 | 39.31 | 5.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-epoll-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 62.77 | 90.77 | 255.32 | 315.38 | 1049.90 | 3957.5 | 1.5 | 0.5 | 0.0 | 32.12 | 1.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189998 | 45.71 | 59.61 | 91.77 | 94.25 | 267.34 | 9.1 | 0.0 | 0.0 | 0.0 | 30.54 | 5.5 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-epoll-spin-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 63.66 | 1231.58 | 8954.86 | 16091.19 | 18215.30 | 12648.4 | 9422.0 | 6877.8 | 472.4 |
| r1-base-1 | 1189998 | 45.89 | 59.85 | 92.02 | 94.51 | 267.51 | 11.6 | 0.0 | 0.0 | 0.0 |

## WebSocket round trip, pp-recv-busy-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1518389 | 38.49 | 44.66 | 54.30 | 64.44 | 126.31 | 11.2 | 0.0 | 0.0 | 0.0 | 38.31 | 1111.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1547627 | 37.73 | 44.29 | 53.42 | 70.82 | 242.43 | 69.2 | 0.0 | 0.0 | 0.0 | 137.81 | 976.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

## WebSocket round trip, pp-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 39.26 | 49.29 | 56.98 | 66.36 | 120.68 | 2.0 | 0.0 | 0.0 | 0.0 | 53.06 | 441.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189999 | 40.44 | 46.21 | 60.59 | 99.64 | 691.15 | 58.5 | 1.5 | 0.0 | 0.0 | 223.00 | 779.0 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 39.55 | 49.61 | 57.31 | 66.67 | 120.92 | 2.0 | 0.0 | 0.0 | 0.0 |
| r1-base-1 | 1189999 | 40.72 | 46.53 | 61.68 | 422.76 | 1425.84 | 126.1 | 53.9 | 19.7 | 0.0 |

## WebSocket round trip, pp-uring-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1187174 | 48.38 | 92.23 | 97.77 | 106.61 | 521.08 | 346.8 | 1.0 | 0.0 | 0.0 | 477.16 | 20.0 | client: softirq TIMER |
| r1-base-1 | 1197721 | 48.47 | 59.38 | 94.86 | 102.98 | 431.99 | 142.4 | 0.0 | 0.0 | 0.0 | 399.79 | 25.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189998 | 43.73 | 208.39 | 267.85 | 325.68 | 986.79 | 7332.6 | 0.5 | 0.0 | 0.0 | 224.12 | 9.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189999 | 44.89 | 90.90 | 93.29 | 163.29 | 1048.55 | 111.9 | 0.5 | 0.5 | 0.0 | 225.92 | 14.0 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-uring-spin-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189998 | 43.84 | 1265.58 | 3491.18 | 7058.56 | 8397.78 | 23782.2 | 15567.3 | 8438.8 | 0.0 |
| r1-base-1 | 1189999 | 44.95 | 91.00 | 93.65 | 1165.38 | 3735.98 | 254.6 | 81.2 | 65.0 | 0.0 |

## WebSocket round trip, pp-uring-sqpoll-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1473746 | 39.44 | 45.80 | 60.69 | 115.78 | 431.45 | 256.3 | 0.0 | 0.0 | 0.0 | 24.64 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1478388 | 39.37 | 45.44 | 52.13 | 63.30 | 171.56 | 6.1 | 0.0 | 0.0 | 0.0 | 78.80 | 4.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-sqpoll-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189997 | 41.97 | 50.08 | 57.62 | 94.94 | 732.10 | 56.0 | 1.0 | 0.0 | 0.0 | 573.27 | 1.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189998 | 39.43 | 44.33 | 55.36 | 67.00 | 144.69 | 5.0 | 0.0 | 0.0 | 0.0 | 444.14 | 2.5 | server: softirq TIMER |

### WebSocket round trip, pp-uring-sqpoll-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189997 | 42.05 | 50.17 | 58.37 | 214.78 | 1673.82 | 119.0 | 28.2 | 17.1 | 0.0 |
| r1-base-1 | 1189998 | 39.52 | 44.50 | 55.52 | 67.62 | 426.33 | 9.1 | 0.0 | 0.0 | 0.0 |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs, and system-management interrupts (SMI, bare metal only: firmware pauses). The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | SMI | receiving CPU | LOC | IWI | RES | CAL | SMI |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| http-recv-busy-10000 | r1-base-0 | 7 | 27 | 27 | 0 | 8 | 0 | | | | | | | |
| http-recv-busy-10000 | r1-base-1 | 7 | 27 | 27 | 0 | 8 | 0 | | | | | | | |
| md-recv-busy | r1-base-0 | 7 | 2 | 3 | 0 | 17 | 0 | | | | | | | |
| md-recv-busy | r1-base-1 | 7 | 8 | 6 | 0 | 17 | 0 | | | | | | | |
| noise | r1-base-0 | 7 | 0 | 0 | 0 | 8 | 0 | | | | | | | |
| noise | r1-base-1 | 7 | 0 | 0 | 0 | 8 | 0 | | | | | | | |
| pp-epoll-spin-0 | r1-base-0 | 7 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 1 | 14 | 0 |
| pp-epoll-spin-0 | r1-base-1 | 7 | 0 | 0 | 0 | 16 | 0 | 8 | 1014 | 0 | 1 | 22 | 0 |
| pp-epoll-spin-10000 | r1-base-0 | 7 | 0 | 0 | 0 | 8 | 0 | 8 | 991 | 0 | 0 | 12 | 0 |
| pp-epoll-spin-10000 | r1-base-1 | 7 | 0 | 0 | 0 | 8 | 0 | 8 | 336 | 0 | 0 | 9 | 0 |
| pp-recv-busy-0 | r1-base-0 | 7 | 45 | 44 | 0 | 16 | 0 | 8 | 1017 | 0 | 1 | 17 | 0 |
| pp-recv-busy-0 | r1-base-1 | 7 | 42 | 41 | 0 | 16 | 0 | 0 | 1144 | 0 | 1 | 11 | 0 |
| pp-recv-busy-10000 | r1-base-0 | 7 | 35 | 34 | 0 | 8 | 0 | | | | | | | |
| pp-recv-busy-10000 | r1-base-1 | 7 | 29 | 28 | 0 | 8 | 0 | | | | | | | |
| pp-uring-spin-0 | r1-base-0 | 7 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 0 | 1 | 15 | 0 |
| pp-uring-spin-0 | r1-base-1 | 7 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 0 | 1 | 14 | 0 |
| pp-uring-spin-10000 | r1-base-0 | 7 | 0 | 0 | 0 | 8 | 0 | 8 | 972 | 0 | 1 | 13 | 0 |
| pp-uring-spin-10000 | r1-base-1 | 7 | 0 | 0 | 0 | 8 | 0 | 0 | 1071 | 0 | 1 | 6 | 0 |
| pp-uring-sqpoll-0 | r1-base-0 | 7 | 0 | 0 | 0 | 17 | 0 | 1 | 95 | 45 | 1 | 17 | 0 |
| pp-uring-sqpoll-0 | r1-base-1 | 7 | 0 | 0 | 0 | 17 | 0 | 1 | 87 | 41 | 1 | 17 | 0 |
| pp-uring-sqpoll-10000 | r1-base-0 | 7 | 0 | 0 | 0 | 8 | 0 | 1 | 43 | 19 | 0 | 8 | 0 |
| pp-uring-sqpoll-10000 | r1-base-1 | 7 | 0 | 0 | 0 | 8 | 0 | 1 | 38 | 18 | 1 | 8 | 0 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### http-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 717.85 at 17.6 s: client: softirq TIMER; 182.56 at 21.8 s: server: softirq TIMER; 149.78 at 46.9 s: neither loop stalled long enough: network, hypervisor or untraced; 123.11 at 24.8 s: neither loop stalled long enough: network, hypervisor or untraced; 117.00 at 83.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 549.39 at 95.8 s: neither loop stalled long enough: network, hypervisor or untraced; 534.97 at 35.8 s: neither loop stalled long enough: network, hypervisor or untraced; 514.05 at 95.8 s: neither loop stalled long enough: network, hypervisor or untraced; 382.11 at 95.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 336.19 at 35.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### md-recv-busy

| condition | worst events |
|---|---|
| r1-base-0 | 43.73 at 49.0 s: client: loop work (32 items); 43.58 at 49.0 s: client: loop work (32 items); 43.43 at 49.0 s: client: loop work (32 items); 43.25 at 49.0 s: client: loop work (32 items); 43.08 at 49.0 s: client: loop work (32 items) |
| r1-base-1 | 49.53 at 36.6 s: neither loop stalled long enough: network, hypervisor or untraced; 49.38 at 36.6 s: neither loop stalled long enough: network, hypervisor or untraced; 46.57 at 2.8 s: client: loop work (4 items); 45.92 at 45.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 45.77 at 45.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |

### noise

| condition | worst events |
|---|---|
| r1-base-0 | 73.64 at 100.5 s: softirq TIMER; 30.15 at 7.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.41 at 67.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.62 at 3.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 11.81 at 100.0 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-1 | 80.98 at 99.7 s: softirq TIMER; 32.27 at 15.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.38 at 75.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.03 at 4.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 11.68 at 84.0 s: unexplained: no kernel event (hypervisor or in-kernel time) |

### pp-epoll-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 372.55 at 49.2 s: neither loop stalled long enough: network, hypervisor or untraced; 356.39 at 21.7 s: neither loop stalled long enough: network, hypervisor or untraced; 350.90 at 49.2 s: neither loop stalled long enough: network, hypervisor or untraced; 324.20 at 53.4 s: neither loop stalled long enough: network, hypervisor or untraced; 311.22 at 49.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 130.79 at 1.6 s: neither loop stalled long enough: network, hypervisor or untraced; 124.69 at 43.4 s: neither loop stalled long enough: network, hypervisor or untraced; 122.09 at 42.3 s: neither loop stalled long enough: network, hypervisor or untraced; 119.40 at 2.4 s: neither loop stalled long enough: network, hypervisor or untraced; 119.35 at 2.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-epoll-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 1049.90 at 46.3 s: neither loop stalled long enough: network, hypervisor or untraced; 789.42 at 5.0 s: neither loop stalled long enough: network, hypervisor or untraced; 617.99 at 60.0 s: neither loop stalled long enough: network, hypervisor or untraced; 440.64 at 5.7 s: neither loop stalled long enough: network, hypervisor or untraced; 418.77 at 57.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 267.34 at 1.5 s: neither loop stalled long enough: network, hypervisor or untraced; 158.63 at 30.4 s: neither loop stalled long enough: network, hypervisor or untraced; 141.69 at 99.1 s: neither loop stalled long enough: network, hypervisor or untraced; 127.44 at 18.3 s: neither loop stalled long enough: network, hypervisor or untraced; 123.14 at 34.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-0

| condition | worst events |
|---|---|
| r1-base-0 | 126.31 at 23.2 s: neither loop stalled long enough: network, hypervisor or untraced; 121.09 at 30.5 s: neither loop stalled long enough: network, hypervisor or untraced; 118.94 at 20.8 s: neither loop stalled long enough: network, hypervisor or untraced; 118.68 at 33.0 s: neither loop stalled long enough: network, hypervisor or untraced; 111.21 at 5.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 242.43 at 38.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 181.35 at 48.0 s: neither loop stalled long enough: network, hypervisor or untraced; 174.33 at 54.0 s: neither loop stalled long enough: network, hypervisor or untraced; 169.52 at 53.0 s: neither loop stalled long enough: network, hypervisor or untraced; 167.35 at 50.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 120.68 at 22.0 s: neither loop stalled long enough: network, hypervisor or untraced; 111.20 at 56.5 s: neither loop stalled long enough: network, hypervisor or untraced; 106.64 at 94.4 s: neither loop stalled long enough: network, hypervisor or untraced; 101.62 at 100.9 s: neither loop stalled long enough: network, hypervisor or untraced; 98.51 at 1.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 691.15 at 5.0 s: neither loop stalled long enough: network, hypervisor or untraced; 605.74 at 65.0 s: neither loop stalled long enough: network, hypervisor or untraced; 523.63 at 5.0 s: neither loop stalled long enough: network, hypervisor or untraced; 449.88 at 65.0 s: neither loop stalled long enough: network, hypervisor or untraced; 447.31 at 65.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 521.08 at 9.5 s: client: softirq TIMER; 434.94 at 42.3 s: neither loop stalled long enough: network, hypervisor or untraced; 427.62 at 42.3 s: neither loop stalled long enough: network, hypervisor or untraced; 359.38 at 42.3 s: neither loop stalled long enough: network, hypervisor or untraced; 356.13 at 42.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 431.99 at 53.7 s: neither loop stalled long enough: network, hypervisor or untraced; 407.04 at 11.6 s: client: softirq TIMER; 358.12 at 53.7 s: neither loop stalled long enough: network, hypervisor or untraced; 343.57 at 53.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 310.49 at 57.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 986.79 at 38.3 s: neither loop stalled long enough: network, hypervisor or untraced; 439.73 at 61.8 s: neither loop stalled long enough: network, hypervisor or untraced; 422.52 at 94.4 s: neither loop stalled long enough: network, hypervisor or untraced; 422.42 at 76.7 s: neither loop stalled long enough: network, hypervisor or untraced; 395.34 at 48.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1048.55 at 15.9 s: neither loop stalled long enough: network, hypervisor or untraced; 487.79 at 21.3 s: neither loop stalled long enough: network, hypervisor or untraced; 450.17 at 21.3 s: neither loop stalled long enough: network, hypervisor or untraced; 449.33 at 21.3 s: neither loop stalled long enough: network, hypervisor or untraced; 439.43 at 81.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |

### pp-uring-sqpoll-0

| condition | worst events |
|---|---|
| r1-base-0 | 431.45 at 36.2 s: neither loop stalled long enough: network, hypervisor or untraced; 265.94 at 9.0 s: neither loop stalled long enough: network, hypervisor or untraced; 220.26 at 13.7 s: neither loop stalled long enough: network, hypervisor or untraced; 218.66 at 19.0 s: neither loop stalled long enough: network, hypervisor or untraced; 207.03 at 18.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 171.56 at 47.8 s: neither loop stalled long enough: network, hypervisor or untraced; 143.09 at 11.2 s: client: VM paused: the idle spare core stalled too (hypervisor); 115.10 at 53.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 105.32 at 44.2 s: neither loop stalled long enough: network, hypervisor or untraced; 104.36 at 19.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-10000

| condition | worst events |
|---|---|
| r1-base-0 | 732.10 at 63.5 s: neither loop stalled long enough: network, hypervisor or untraced; 556.91 at 3.5 s: neither loop stalled long enough: network, hypervisor or untraced; 434.80 at 63.5 s: neither loop stalled long enough: network, hypervisor or untraced; 385.21 at 63.5 s: neither loop stalled long enough: network, hypervisor or untraced; 381.31 at 63.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 144.69 at 65.2 s: server: softirq TIMER; 129.96 at 55.1 s: neither loop stalled long enough: network, hypervisor or untraced; 128.98 at 102.9 s: neither loop stalled long enough: network, hypervisor or untraced; 126.74 at 115.0 s: neither loop stalled long enough: network, hypervisor or untraced; 126.07 at 102.9 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| r1-base-0 | loop work 122009.61; unexplained: no kernel event (hypervisor or in-kernel time) 108790.05; mostly unexplained (+preempted by kworker/7:1) 68464.26; mostly unexplained (+softirq NET_RX) 32091.43; softirq NET_RX 9063.23 |
| r1-base-1 | loop work 218185.10; mostly unexplained (+preempted by kworker/7:1) 84314.43; unexplained: no kernel event (hypervisor or in-kernel time) 59007.64; mostly unexplained (+softirq NET_RX) 24969.90; softirq NET_RX 11655.93 |

## Once-a-minute stalls

Whether each source's events of 300 µs or more (loop stalls, or samples that slow) bunch at one second of the wall-clock minute. The busiest second is the middle of the busiest three; three seconds hold one event in twenty by chance, so a share of 25% or more over at least five events is marked periodic. Each host has its own second (its agents' timers start at boot).

| run | source | events | busiest second | events there (share) | minutes with one there | longest there (µs) | periodic |
|---|---|---|---|---|---|---|---|
| cpumap-receiver/cpu1 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu10 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu11 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu12 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu13 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu14 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu15 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu2 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu3 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu4 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu5 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu6 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu7 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu9 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu1 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu3 | counter-only spinner | 0 | | | | | |
| http-recv-busy-10000 | idle spare core | 0 | | | | | |
| http-recv-busy-10000 | receiver loop | 2 | :06 | 1 (50%) | 1 of 4 | 445.32 | no |
| http-recv-busy-10000 | sender loop | 0 | | | | | |
| http-recv-busy-10000 | slow samples | 7 | :21 | 6 (86%) | 2 of 4 | 549.39 | **yes** |
| md-recv-busy | idle spare core | 0 | | | | | |
| md-recv-busy | receiver loop | 0 | | | | | |
| md-recv-busy | slow samples | 0 | | | | | |
| noise | noise probe (loop CPU) | 0 | | | | | |
| pp-epoll-spin-0 | idle spare core | 0 | | | | | |
| pp-epoll-spin-0 | receiver loop | 0 | | | | | |
| pp-epoll-spin-0 | sender loop | 0 | | | | | |
| pp-epoll-spin-0 | slow samples | 5 | :21 | 3 (60%) | 1 of 2 | 372.55 | no |
| pp-epoll-spin-10000 | idle spare core | 0 | | | | | |
| pp-epoll-spin-10000 | receiver loop | 0 | | | | | |
| pp-epoll-spin-10000 | sender loop | 0 | | | | | |
| pp-epoll-spin-10000 | slow samples | 238 | :58 | 58 (24%) | 2 of 4 | 390.75 | no |
| pp-recv-busy-0 | idle spare core | 0 | | | | | |
| pp-recv-busy-0 | receiver loop | 0 | | | | | |
| pp-recv-busy-0 | sender loop | 0 | | | | | |
| pp-recv-busy-0 | slow samples | 0 | | | | | |
| pp-recv-busy-10000 | idle spare core | 0 | | | | | |
| pp-recv-busy-10000 | receiver loop | 0 | | | | | |
| pp-recv-busy-10000 | sender loop | 0 | | | | | |
| pp-recv-busy-10000 | slow samples | 13 | :21 | 10 (77%) | 2 of 4 | 691.15 | **yes** |
| pp-uring-spin-0 | idle spare core | 1 | :48 | 1 (100%) | 1 of 2 | 547.76 | no |
| pp-uring-spin-0 | receiver loop | 2 | :38 | 1 (50%) | 1 of 3 | 399.79 | no |
| pp-uring-spin-0 | sender loop | 0 | | | | | |
| pp-uring-spin-0 | slow samples | 12 | :21 | 8 (67%) | 2 of 2 | 434.94 | **yes** |
| pp-uring-spin-10000 | idle spare core | 0 | | | | | |
| pp-uring-spin-10000 | receiver loop | 0 | | | | | |
| pp-uring-spin-10000 | sender loop | 0 | | | | | |
| pp-uring-spin-10000 | slow samples | 460 | :07 | 43 (9%) | 2 of 4 | 381.10 | no |
| pp-uring-sqpoll-0 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-0 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-0 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-0 | slow samples | 1 | :21 | 1 (100%) | 1 of 2 | 431.45 | no |
| pp-uring-sqpoll-10000 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-10000 | receiver loop | 2 | :11 | 1 (50%) | 1 of 5 | 444.14 | no |
| pp-uring-sqpoll-10000 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-10000 | slow samples | 8 | :21 | 8 (100%) | 2 of 4 | 732.10 | **yes** |

# Every run in detail

#### Stalls: r1-base-0/http-recv-busy-10000, loop 0 on CPU 7

2096217891 iterations, longest 707.9 µs; over 5 / 20 / 100 µs: 91762 / 6297 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 3967 | 102272.6 | 108.3 |
| mostly unexplained (+preempted by kworker/7:1) | 1264 | 33129.6 | 54.7 |
| mostly unexplained (+softirq NET_RX) | 938 | 24630.7 | 56.8 |
| softirq NET_RX | 119 | 3031.6 | 32.3 |
| softirq TIMER | 1 | 707.9 | 707.9 |
| mostly unexplained (+irq work (IPI/vector)) | 5 | 135.9 | 39.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 53.6 | 30.2 |
| loop work | 1 | 25.2 | 25.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 707.9 | 2 | softirq TIMER | softirq TIMER 691, local timer tick 7, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 108.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 56.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 17, preempted by kworker/7:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 54.7 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 53.8 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 53.2 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 53.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 17, preempted by kworker/7:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 52.2 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 51.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 18, preempted by kworker/7:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 51.2 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3214 |
| IWI IRQ work interrupts | 3212 |
| CAL Function call interrupts | 999 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/http-recv-busy-10000 (median 40.10)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 717.85 | 677.75 | 707.04 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 2 | 182.56 | 142.46 | 0.00 |  | 74.82 | softirq TIMER | 67.64 | server: softirq TIMER |
| 3 | 149.78 | 109.68 | 0.00 |  | 0.00 |  | 109.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 123.11 | 83.00 | 0.00 |  | 0.00 |  | 83.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 117.00 | 76.90 | 0.00 |  | 0.00 |  | 76.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 115.69 | 75.59 | 0.00 |  | 0.00 |  | 75.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 110.93 | 70.83 | 0.00 |  | 0.00 |  | 70.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 107.64 | 67.54 | 0.00 |  | 0.00 |  | 67.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 103.25 | 63.15 | 0.00 |  | 0.00 |  | 63.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 101.50 | 61.40 | 0.00 |  | 0.00 |  | 61.40 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-0/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=416473 clocksource=tsc

### Loop counters (r1-base-0/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002500 | 469271 | 6.40 | 116817152 | 38.91 | 1946953 | 1946953 | 99.6 | 66.9 | true | 0 | 135 | 455.6 | 65752 / 4524 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 7 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672560000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (62500 of 2900000 in 1-msg reads) | 62500 | 2.32 | 2.16 | 2.75 | 3.84 | 16.56 | 24.91 | 31.50 |

#### Stalls: r1-base-0/md-recv-busy, loop 0 on CPU 7

116817152 iterations, longest 455.6 µs; over 5 / 20 / 100 µs: 65752 / 4524 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4329 | 121984.4 | 455.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 97 | 2302.3 | 47.9 |
| mostly unexplained (+preempted by kworker/7:1) | 56 | 1455.9 | 32.3 |
| softirq NET_RX | 31 | 819.7 | 32.7 |
| mostly unexplained (+softirq NET_RX) | 8 | 200.8 | 32.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 51.8 | 30.4 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 25.5 | 25.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 455.6 | 2500 | loop work (2500 items) |  |
| 2 | 55.5 | 3 | loop work (3 items) | preempted by kworker/7:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 53.8 | 26 | loop work (26 items) | preempted by kworker/7:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 53.7 | 10 | loop work (10 items) | preempted by kworker/7:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 52.9 | 4 | loop work (4 items) | preempted by kworker/7:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 47.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 45.0 | 32 | loop work (32 items) | preempted by kworker/7:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 42.7 | 25 | loop work (25 items) | softirq NET_RX 10 |
| 9 | 40.5 | 27 | loop work (27 items) | softirq NET_TX 2 |
| 10 | 40.2 | 28 | loop work (28 items) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 155 |
| LOC Local timer interrupts | 138 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-0/noise: platform noise on CPU 7 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-0/noise, loop 0 on CPU 7

9239101440 iterations, longest 73.6 µs; over 5 / 20 / 100 µs: 52 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 46 | 439.6 | 30.2 |
| softirq TIMER | 1 | 73.6 | 73.6 |
| mostly unexplained (+local timer tick) | 4 | 27.7 | 7.9 |
| local timer tick | 1 | 10.7 | 10.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 73.6 | 0 | softirq TIMER | softirq TIMER 59, local timer tick 8, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 30.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 18.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 17.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 11.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 11.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 10.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 10.7 | 0 | local timer tick | local timer tick 6, timer tick_nohz_handler 4 |
| 9 | 10.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-base-0/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1241631 | 1241631 | 1.00 | 288013119 | 231.96 | 4800219 | 4758831 | 0.0 | 40.5 | false | 0 | 0 | 131.0 | 21700 / 24 / 3 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-0, loop 0 on CPU 7

285529856 iterations, longest 131.0 µs; over 5 / 20 / 100 µs: 21700 / 24 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 22 | 1273.2 | 131.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 48.5 | 26.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 131.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 114.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 111.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 81.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 71.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 71.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 69.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 65.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 62.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 61.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 11 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-0 (median 48.98)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 372.55 | 323.56 | 0.00 |  | 0.00 |  | 323.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 356.39 | 307.41 | 0.00 |  | 0.00 |  | 307.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 350.90 | 301.92 | 0.00 |  | 0.00 |  | 301.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 324.20 | 275.22 | 0.00 |  | 0.00 |  | 275.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 311.22 | 262.24 | 0.00 |  | 0.00 |  | 262.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 293.86 | 244.88 | 130.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 113.96 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 292.71 | 243.73 | 58.03 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 185.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 291.18 | 242.20 | 81.79 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 160.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 287.85 | 238.87 | 71.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 167.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 281.77 | 232.79 | 0.00 |  | 0.00 |  | 232.79 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 576315520 | 480.26 | 4802629 | 4782629 | 0.0 | 37.2 | false | 0 | 0 | 32.1 | 9856 / 3 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-10000, loop 0 on CPU 7

573915520 iterations, longest 32.1 µs; over 5 / 20 / 100 µs: 9856 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 52.7 | 32.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 26.2 | 26.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 32.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 26.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 20.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-10000 (median 62.77)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1049.90 | 987.13 | 0.00 |  | 0.00 |  | 987.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 789.42 | 726.65 | 0.00 |  | 0.00 |  | 726.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 617.99 | 555.22 | 0.00 |  | 0.00 |  | 555.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 440.64 | 377.88 | 0.00 |  | 0.00 |  | 377.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 418.77 | 356.01 | 0.00 |  | 0.00 |  | 356.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 412.33 | 349.56 | 0.00 |  | 0.00 |  | 349.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 390.75 | 327.98 | 0.00 |  | 0.00 |  | 327.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 388.12 | 325.36 | 0.00 |  | 0.00 |  | 325.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 387.79 | 325.02 | 0.00 |  | 0.00 |  | 325.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 385.45 | 322.69 | 0.00 |  | 0.00 |  | 322.69 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1544320 | 1544320 | 1.00 | 108972673 | 70.56 | 1816211 | 1790473 | 98.6 | 72.5 | true | 0 | 2644 | 38.3 | 66057 / 1111 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-0, loop 0 on CPU 7

107428352 iterations, longest 38.3 µs; over 5 / 20 / 100 µs: 66057 / 1111 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/7:1) | 762 | 18213.4 | 38.3 |
| mostly unexplained (+softirq NET_RX) | 182 | 4529.5 | 30.6 |
| softirq NET_RX | 143 | 3328.2 | 31.1 |
| mostly unexplained (+local timer tick) | 15 | 384.4 | 29.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 100.7 | 30.4 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 89.1 | 34.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 57.5 | 34.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 38.3 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 14, softirq NET_RX 14, local timer tick 4, timer tick_nohz_handler 2 |
| 2 | 37.4 | 0 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 9, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 3 | 34.2 | 0 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 7, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 4 | 34.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 34.0 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 31.8 | 0 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 12, softirq NET_RX 12, local timer tick 4, timer tick_nohz_handler 2 |
| 7 | 31.4 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 31.1 | 0 | softirq NET_RX | softirq NET_RX 23, local timer tick 5, preempted by kworker/7:1 4, timer tick_nohz_handler 2 |
| 9 | 30.8 | 1 | softirq NET_RX | softirq NET_RX 16, local timer tick 6, preempted by kworker/7:1 6, timer tick_nohz_handler 3 |
| 10 | 30.8 | 0 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2699 |
| IWI IRQ work interrupts | 2649 |
| CAL Function call interrupts | 989 |

#### Slowest round trips: r1-base-0/pp-recv-busy-0 (median 38.49)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 126.31 | 87.82 | 0.00 |  | 0.00 |  | 87.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 121.09 | 82.60 | 0.00 |  | 0.00 |  | 82.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 118.94 | 80.45 | 0.00 |  | 0.00 |  | 80.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 118.68 | 80.19 | 0.00 |  | 0.00 |  | 80.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 111.21 | 72.72 | 0.00 |  | 0.00 |  | 72.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 107.33 | 68.84 | 0.00 |  | 0.00 |  | 68.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 104.16 | 65.67 | 0.00 |  | 0.00 |  | 65.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 103.47 | 64.98 | 0.00 |  | 0.00 |  | 64.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 101.47 | 62.98 | 0.00 |  | 0.00 |  | 62.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 100.85 | 62.36 | 0.00 |  | 0.00 |  | 62.36 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 234077312 | 195.06 | 1950644 | 1940644 | 99.5 | 68.4 | true | 0 | 4132 | 53.1 | 55820 / 882 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-10000, loop 0 on CPU 7

232877312 iterations, longest 53.1 µs; over 5 / 20 / 100 µs: 55820 / 882 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/7:1) | 666 | 15665.3 | 34.4 |
| mostly unexplained (+softirq NET_RX) | 111 | 2730.3 | 35.3 |
| softirq NET_RX | 82 | 1883.8 | 29.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 17 | 405.5 | 53.1 |
| mostly unexplained (+local timer tick) | 3 | 75.1 | 29.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 51.7 | 27.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 35.8 | 35.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 53.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 35.8 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 35.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/7:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 34.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/7:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 34.4 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 33.2 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 32.3 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 31.5 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 31.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/7:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 31.1 | 0 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4141 |
| IWI IRQ work interrupts | 4138 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-base-0/pp-recv-busy-10000 (median 39.26)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 120.68 | 81.42 | 0.00 |  | 0.00 |  | 81.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 111.20 | 71.95 | 0.00 |  | 0.00 |  | 71.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 106.64 | 67.38 | 0.00 |  | 0.00 |  | 67.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 101.62 | 62.36 | 0.00 |  | 0.00 |  | 62.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 98.51 | 59.26 | 0.00 |  | 0.00 |  | 59.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 96.64 | 57.38 | 0.00 |  | 0.00 |  | 57.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 96.37 | 57.11 | 0.00 |  | 0.00 |  | 57.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 95.95 | 56.70 | 0.00 |  | 0.00 |  | 56.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 90.79 | 51.53 | 0.00 |  | 0.00 |  | 51.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 90.78 | 51.53 | 0.00 |  | 0.00 |  | 51.53 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1207795 | 1207795 | 1.00 | 3623386 | 3.00 | 60390 | 46041378 | 0.0 | 9.7 | false | 0 | 0 | 477.2 | 28410 / 20 / 5 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-spin-0, loop 0 on CPU 7

2762482688 iterations, longest 477.2 µs; over 5 / 20 / 100 µs: 28410 / 20 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 16 | 1110.5 | 156.5 |
| softirq TIMER | 1 | 477.2 | 477.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 52.6 | 28.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 30.8 | 30.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 477.2 | 0 | softirq TIMER | softirq TIMER 456, local timer tick 13, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 156.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 127.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 126.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 102.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 82.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 71.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 70.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 65.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 62.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 15 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/pp-uring-spin-0 (median 48.38)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 521.08 | 472.70 | 477.16 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 2 | 434.94 | 386.56 | 127.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 259.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 427.62 | 379.24 | 0.00 |  | 0.00 |  | 379.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 359.38 | 311.00 | 0.00 |  | 0.00 |  | 311.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 356.13 | 307.75 | 82.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 225.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 340.73 | 292.35 | 56.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 235.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 329.68 | 281.30 | 0.00 |  | 0.00 |  | 281.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 286.88 | 238.50 | 65.35 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 173.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 282.80 | 234.42 | 0.00 |  | 0.00 |  | 234.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 277.18 | 228.80 | 71.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 157.31 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1200000 | 1200000 | 1.00 | 3600000 | 3.00 | 30000 | 34718749 | 0.0 | 5.7 | false | 0 | 0 | 224.1 | 22311 / 18 / 1 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-spin-10000, loop 0 on CPU 7

4166249920 iterations, longest 224.1 µs; over 5 / 20 / 100 µs: 22311 / 18 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 14 | 669.1 | 224.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 76.1 | 31.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 35.2 | 35.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 224.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 42.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 41.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 41.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 40.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 40.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 40.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 38.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 38.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1003 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-uring-spin-10000 (median 43.73)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 986.79 | 943.07 | 0.00 |  | 0.00 |  | 943.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 439.73 | 396.00 | 0.00 |  | 0.00 |  | 396.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 422.52 | 378.80 | 0.00 |  | 0.00 |  | 378.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 422.42 | 378.69 | 0.00 |  | 0.00 |  | 378.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 395.34 | 351.61 | 0.00 |  | 0.00 |  | 351.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 391.29 | 347.56 | 0.00 |  | 0.00 |  | 347.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 388.87 | 345.15 | 0.00 |  | 0.00 |  | 345.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 381.24 | 337.51 | 0.00 |  | 0.00 |  | 337.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 381.10 | 337.38 | 0.00 |  | 0.00 |  | 337.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 379.29 | 335.56 | 0.00 |  | 0.00 |  | 335.56 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1498786 | 1498786 | 1.00 | 1 | 0.00 | 0 | 32740585 | 0.0 | 0.0 | true | 0 | 0 | 24.6 | 75 / 2 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-0, loop 0 on CPU 7

1964435072 iterations, longest 24.6 µs; over 5 / 20 / 100 µs: 75 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 46.4 | 24.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 24.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 21.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 7 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-0 (median 39.44)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 431.45 | 392.01 | 0.00 |  | 0.00 |  | 392.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 265.94 | 226.50 | 0.00 |  | 0.00 |  | 226.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 220.26 | 180.82 | 0.00 |  | 0.00 |  | 180.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 218.66 | 179.22 | 0.00 |  | 0.00 |  | 179.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 207.03 | 167.59 | 0.00 |  | 0.00 |  | 167.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 206.21 | 166.77 | 0.00 |  | 0.00 |  | 166.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 187.67 | 148.23 | 0.00 |  | 0.00 |  | 148.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 181.84 | 142.40 | 0.00 |  | 0.00 |  | 142.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 176.96 | 137.52 | 0.00 |  | 0.00 |  | 137.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 175.20 | 135.76 | 0.00 |  | 0.00 |  | 135.76 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 25389519 | 0.0 | 0.0 | true | 0 | 0 | 573.3 | 96 / 3 / 2 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-10000, loop 0 on CPU 7

3046742272 iterations, longest 573.3 µs; over 5 / 20 / 100 µs: 96 / 3 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 573.3 | 573.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 190.4 | 190.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 24.9 | 24.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 573.3 | 0 | softirq TIMER | softirq TIMER 552, local timer tick 13, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 190.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 24.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-10000 (median 41.97)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 732.10 | 690.13 | 0.00 |  | 0.00 |  | 690.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 556.91 | 514.94 | 0.00 |  | 0.00 |  | 514.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 434.80 | 392.83 | 0.00 |  | 0.00 |  | 392.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 385.21 | 343.24 | 0.00 |  | 0.00 |  | 343.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 381.31 | 339.34 | 0.00 |  | 0.00 |  | 339.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 367.48 | 325.51 | 0.00 |  | 0.00 |  | 325.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 354.37 | 312.39 | 0.00 |  | 0.00 |  | 312.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 306.50 | 264.53 | 0.00 |  | 0.00 |  | 264.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 293.91 | 251.94 | 0.00 |  | 0.00 |  | 251.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 293.57 | 251.60 | 0.00 |  | 0.00 |  | 251.60 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-1/http-recv-busy-10000, loop 0 on CPU 7

2105495014 iterations, longest 445.3 µs; over 5 / 20 / 100 µs: 104151 / 4142 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1909 | 46923.3 | 222.9 |
| mostly unexplained (+preempted by kworker/7:1) | 1486 | 38521.2 | 60.5 |
| mostly unexplained (+softirq NET_RX) | 606 | 14936.1 | 56.7 |
| softirq NET_RX | 118 | 3026.9 | 36.0 |
| softirq TIMER | 1 | 445.3 | 445.3 |
| mostly unexplained (+irq work (IPI/vector)) | 12 | 349.0 | 40.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 6 | 151.4 | 34.1 |
| loop work | 4 | 96.8 | 36.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 445.3 | 1 | softirq TIMER | softirq TIMER 434, local timer tick 8, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 222.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 208.1 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 114.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 96.1 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 84.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 82.5 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 75.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 74.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 74.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 3279 |
| LOC Local timer interrupts | 3275 |
| CAL Function call interrupts | 999 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/http-recv-busy-10000 (median 39.93)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 549.39 | 509.45 | 146.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 362.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 534.97 | 495.04 | 149.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 345.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 514.05 | 474.11 | 207.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 266.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 382.11 | 342.18 | 294.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 47.50 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 336.19 | 296.26 | 0.00 |  | 0.00 |  | 296.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 322.52 | 282.58 | 84.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 198.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 274.62 | 234.68 | 0.00 |  | 0.00 |  | 234.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 272.99 | 233.06 | 113.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 119.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 249.97 | 210.04 | 153.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 56.07 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 248.15 | 208.22 | 61.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 146.38 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-1/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=763810 clocksource=tsc

### Loop counters (r1-base-1/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002500 | 863900 | 3.48 | 114871040 | 38.26 | 1914517 | 1914517 | 99.2 | 67.5 | true | 0 | 361 | 638.7 | 72569 / 7588 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 7 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672560000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (174309 of 2899950 in 1-msg reads) | 174309 | 2.48 | 2.11 | 2.51 | 15.58 | 21.51 | 34.96 | 46.57 |

#### Stalls: r1-base-1/md-recv-busy, loop 0 on CPU 7

114871040 iterations, longest 638.7 µs; over 5 / 20 / 100 µs: 72569 / 7588 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 7182 | 218088.3 | 638.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 231 | 5617.2 | 54.8 |
| mostly unexplained (+preempted by kworker/7:1) | 84 | 2418.6 | 40.5 |
| mostly unexplained (+softirq NET_RX) | 41 | 1341.8 | 60.6 |
| softirq NET_RX | 37 | 1082.0 | 46.9 |
| mostly unexplained (+local timer tick) | 9 | 316.9 | 40.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 50.8 | 29.4 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 48.4 | 24.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 638.7 | 2500 | loop work (2500 items) |  |
| 2 | 65.6 | 16 | loop work (16 items) | preempted by kworker/7:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 60.6 | 3 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 23, local timer tick 6, preempted by ksoftirqd/7 6, timer tick_nohz_handler 4 |
| 4 | 59.5 | 23 | loop work (23 items) | softirq NET_RX 1 |
| 5 | 59.5 | 14 | loop work (14 items) | preempted by kworker/7:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 59.0 | 11 | loop work (11 items) | softirq NET_RX 23, preempted by kworker/7:1 4, workqueue [ena] 2, irq work (IPI/vector) 1 |
| 7 | 58.9 | 3 | loop work (3 items) | local timer tick 6, preempted by kworker/7:1 6, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 8 | 57.3 | 15 | loop work (15 items) | preempted by kworker/7:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 57.1 | 9 | loop work (9 items) | local timer tick 5, preempted by kworker/7:1 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 56.7 | 9 | loop work (9 items) | local timer tick 5, preempted by kworker/7:1 3, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| LOC Local timer interrupts | 479 |
| IWI IRQ work interrupts | 377 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-1/noise: platform noise on CPU 7 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-1/noise, loop 0 on CPU 7

9241083904 iterations, longest 81.0 µs; over 5 / 20 / 100 µs: 51 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 48 | 457.1 | 32.3 |
| softirq TIMER | 1 | 81.0 | 81.0 |
| mostly unexplained (+call function (IPI/vector)) | 2 | 11.7 | 6.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 81.0 | 0 | softirq TIMER | softirq TIMER 62, local timer tick 12, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 32.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 17.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 17.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 11.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 11.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 11.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 10.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 10.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |
| IWI IRQ work interrupts | 1 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (r1-base-1/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1216996 | 1216996 | 1.00 | 288875145 | 237.37 | 4814586 | 4774019 | 0.0 | 40.3 | false | 0 | 0 | 39.3 | 8044 / 5 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-epoll-spin-0, loop 0 on CPU 7

286441152 iterations, longest 39.3 µs; over 5 / 20 / 100 µs: 8044 / 5 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 81.0 | 39.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.5 | 35.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 39.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 35.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 20.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 20.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 20.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 17 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-1/pp-epoll-spin-0 (median 49.05)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 130.79 | 81.74 | 0.00 |  | 0.00 |  | 81.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 124.69 | 75.64 | 0.00 |  | 0.00 |  | 75.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 122.09 | 73.04 | 0.00 |  | 0.00 |  | 73.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 119.40 | 70.35 | 0.00 |  | 0.00 |  | 70.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 119.35 | 70.30 | 0.00 |  | 0.00 |  | 70.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 119.25 | 70.20 | 0.00 |  | 0.00 |  | 70.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 118.99 | 69.94 | 0.00 |  | 0.00 |  | 69.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 116.02 | 66.97 | 0.00 |  | 0.00 |  | 66.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 114.75 | 65.70 | 0.00 |  | 0.00 |  | 65.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 113.95 | 64.90 | 0.00 |  | 0.00 |  | 64.90 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 562867264 | 469.06 | 4690561 | 4670561 | 0.0 | 36.7 | false | 0 | 0 | 30.5 | 20542 / 11 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-epoll-spin-10000, loop 0 on CPU 7

560467264 iterations, longest 30.5 µs; over 5 / 20 / 100 µs: 20542 / 11 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 166.9 | 22.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 81.6 | 30.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 30.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 26.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 24.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 22.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 21.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 21.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 20.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 20.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 20.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 20.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 4 |

#### Slowest round trips: r1-base-1/pp-epoll-spin-10000 (median 45.71)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 267.34 | 221.63 | 0.00 |  | 0.00 |  | 221.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 158.63 | 112.92 | 0.00 |  | 0.00 |  | 112.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 141.69 | 95.98 | 0.00 |  | 0.00 |  | 95.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 127.44 | 81.73 | 0.00 |  | 0.00 |  | 81.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 123.14 | 77.43 | 0.00 |  | 0.00 |  | 77.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 108.81 | 63.10 | 0.00 |  | 0.00 |  | 63.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 108.78 | 63.07 | 0.00 |  | 0.00 |  | 63.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 107.86 | 62.15 | 0.00 |  | 0.00 |  | 62.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 106.05 | 60.34 | 0.00 |  | 0.00 |  | 60.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 105.36 | 59.65 | 0.00 |  | 0.00 |  | 59.65 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1573873 | 1573873 | 1.00 | 108990642 | 69.25 | 1816511 | 1790279 | 98.5 | 72.4 | true | 0 | 2462 | 137.8 | 53589 / 976 / 1 | 0 | 0 |

#### Stalls: r1-base-1/pp-recv-busy-0, loop 0 on CPU 7

107416768 iterations, longest 137.8 µs; over 5 / 20 / 100 µs: 53589 / 976 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/7:1) | 661 | 16156.1 | 36.4 |
| mostly unexplained (+softirq NET_RX) | 192 | 4838.2 | 47.8 |
| softirq NET_RX | 102 | 2356.9 | 30.4 |
| mostly unexplained (+local timer tick) | 14 | 366.7 | 31.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 188.4 | 137.8 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 55.8 | 33.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.4 | 31.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 137.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 47.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 23, preempted by kworker/7:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 37.0 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/7:1 6, local timer tick 4, timer tick_nohz_handler 2 |
| 4 | 36.4 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 34.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 11, preempted by kworker/7:1 9, local timer tick 5, timer tick_nohz_handler 2 |
| 6 | 33.8 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 12, workqueue [ena] 7, local timer tick 4, irq work (IPI/vector) 1 |
| 7 | 33.6 | 0 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 14, workqueue [ena] 8, local timer tick 5, timer tick_nohz_handler 2 |
| 8 | 33.4 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 33.2 | 1 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 11, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 10 | 32.6 | 0 | mostly unexplained (+preempted by kworker/7:1) | preempted by kworker/7:1 8, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2520 |
| IWI IRQ work interrupts | 2467 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-recv-busy-0 (median 37.73)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 242.43 | 204.69 | 137.43 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 67.26 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 181.35 | 143.62 | 0.00 |  | 0.00 |  | 143.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 174.33 | 136.59 | 0.00 |  | 0.00 |  | 136.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 169.52 | 131.78 | 0.00 |  | 0.00 |  | 131.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 167.35 | 129.62 | 0.00 |  | 0.00 |  | 129.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 164.87 | 127.14 | 0.00 |  | 0.00 |  | 127.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 162.85 | 125.11 | 0.00 |  | 0.00 |  | 125.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 162.37 | 124.63 | 0.00 |  | 0.00 |  | 124.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 160.03 | 122.30 | 0.00 |  | 0.00 |  | 122.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 158.70 | 120.97 | 0.00 |  | 0.00 |  | 120.97 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 233635456 | 194.70 | 1946962 | 1936962 | 99.5 | 68.5 | true | 0 | 3354 | 223.0 | 83931 / 1558 / 3 | 0 | 0 |

#### Stalls: r1-base-1/pp-recv-busy-10000, loop 0 on CPU 7

232435456 iterations, longest 223.0 µs; over 5 / 20 / 100 µs: 83931 / 1558 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/7:1) | 1146 | 27218.4 | 41.8 |
| softirq NET_RX | 225 | 5190.1 | 30.8 |
| mostly unexplained (+softirq NET_RX) | 152 | 3853.8 | 58.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 29 | 2131.2 | 223.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 5 | 125.7 | 39.5 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 31.3 | 31.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 223.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 141.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 108.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 89.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 83.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 82.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 82.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 80.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 79.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 72.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3515 |
| IWI IRQ work interrupts | 3360 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-base-1/pp-recv-busy-10000 (median 40.44)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 691.15 | 650.71 | 230.42 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 420.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 605.74 | 565.31 | 151.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 413.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 523.63 | 483.20 | 142.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 340.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 449.88 | 409.44 | 0.00 |  | 0.00 |  | 409.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 447.31 | 406.87 | 141.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 265.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 433.04 | 392.61 | 0.00 |  | 0.00 |  | 392.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 417.40 | 376.97 | 294.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 82.05 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 380.14 | 339.70 | 129.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 210.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 368.35 | 327.92 | 0.00 |  | 0.00 |  | 327.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 328.94 | 288.50 | 0.00 |  | 0.00 |  | 288.50 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1218358 | 1218358 | 1.00 | 3655075 | 3.00 | 60918 | 45926262 | 0.0 | 9.8 | false | 0 | 0 | 399.8 | 8641 / 25 / 5 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-spin-0, loop 0 on CPU 7

2755575744 iterations, longest 399.8 µs; over 5 / 20 / 100 µs: 8641 / 25 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 21 | 1376.0 | 176.7 |
| softirq TIMER | 1 | 399.8 | 399.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.0 | 30.5 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 34.9 | 34.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 399.8 | 0 | softirq TIMER | softirq TIMER 373, local timer tick 15, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 176.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 131.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 114.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 114.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 81.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 74.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 72.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 70.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 65.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 10 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-1/pp-uring-spin-0 (median 48.47)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 431.99 | 383.53 | 0.00 |  | 0.00 |  | 383.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 407.04 | 358.58 | 399.79 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 358.12 | 309.65 | 72.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 237.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 343.57 | 295.10 | 176.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 118.43 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 310.49 | 262.03 | 0.00 |  | 0.00 |  | 262.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 298.67 | 250.20 | 0.00 |  | 0.00 |  | 250.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 293.09 | 244.63 | 80.98 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 163.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 291.46 | 242.99 | 130.99 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 112.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 280.38 | 231.92 | 0.00 |  | 0.00 |  | 231.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 263.26 | 214.80 | 114.42 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 100.38 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-1/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1200000 | 1200000 | 1.00 | 3600000 | 3.00 | 30000 | 34133963 | 0.0 | 5.9 | false | 0 | 0 | 225.9 | 41941 / 28 / 3 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-spin-10000, loop 0 on CPU 7

4096075584 iterations, longest 225.9 µs; over 5 / 20 / 100 µs: 41941 / 28 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 25 | 1936.1 | 225.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 56.2 | 32.4 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 28.2 | 28.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 225.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 206.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 130.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 82.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 80.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 71.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 71.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 71.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 70.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 70.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-uring-spin-10000 (median 44.89)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1048.55 | 1003.66 | 0.00 |  | 0.00 |  | 1003.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 487.79 | 442.90 | 69.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 373.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 450.17 | 405.29 | 0.00 |  | 0.00 |  | 405.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 449.33 | 404.44 | 82.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 322.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 439.43 | 394.54 | 225.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 168.61 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 418.17 | 373.28 | 206.22 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 167.06 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 412.18 | 367.29 | 63.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 303.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 393.31 | 348.42 | 130.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 218.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 391.26 | 346.37 | 0.00 |  | 0.00 |  | 346.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 391.04 | 346.15 | 53.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 292.99 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1503571 | 1503571 | 1.00 | 1 | 0.00 | 0 | 32618062 | 0.0 | 0.0 | true | 0 | 0 | 78.8 | 68 / 4 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-0, loop 0 on CPU 7

1957083712 iterations, longest 78.8 µs; over 5 / 20 / 100 µs: 68 / 4 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 146.5 | 78.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 78.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 24.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 22.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 21.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 995 |
| LOC Local timer interrupts | 15 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-0 (median 39.37)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 171.56 | 132.19 | 0.00 |  | 0.00 |  | 132.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 143.09 | 103.72 | 78.80 | VM paused: the idle spare core stalled too (hypervisor) | 0.00 |  | 24.93 | client: VM paused: the idle spare core stalled too (hypervisor) |
| 3 | 115.10 | 75.73 | 0.00 |  | 71.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 4.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 105.32 | 65.95 | 0.00 |  | 0.00 |  | 65.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 104.36 | 64.99 | 0.00 |  | 0.00 |  | 64.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 100.99 | 61.62 | 0.00 |  | 0.00 |  | 61.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 99.42 | 60.05 | 0.00 |  | 0.00 |  | 60.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 87.75 | 48.38 | 0.00 |  | 0.00 |  | 48.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 84.69 | 45.32 | 0.00 |  | 0.00 |  | 45.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 83.49 | 44.12 | 0.00 |  | 0.00 |  | 44.12 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 20467820 | 0.0 | 0.0 | true | 0 | 0 | 444.1 | 110 / 5 / 2 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-10000, loop 0 on CPU 7

2456138368 iterations, longest 444.1 µs; over 5 / 20 / 100 µs: 110 / 5 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 444.1 | 444.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 130.4 | 109.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 51.5 | 26.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 444.1 | 0 | softirq TIMER | softirq TIMER 425, local timer tick 10, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 109.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 26.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 25.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 21.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 7 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1008 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-10000 (median 39.43)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 144.69 | 105.27 | 0.00 |  | 121.09 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 129.96 | 90.54 | 0.00 |  | 0.00 |  | 90.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 128.98 | 89.56 | 0.00 |  | 0.00 |  | 89.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 126.74 | 87.32 | 0.00 |  | 0.00 |  | 87.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 126.07 | 86.65 | 0.00 |  | 0.00 |  | 86.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 119.36 | 79.94 | 0.00 |  | 0.00 |  | 79.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 118.44 | 79.01 | 0.00 |  | 0.00 |  | 79.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 116.39 | 76.96 | 0.00 |  | 0.00 |  | 76.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 109.84 | 70.41 | 0.00 |  | 0.00 |  | 70.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 100.51 | 61.09 | 0.00 |  | 0.00 |  | 61.09 | neither loop stalled long enough: network, hypervisor or untraced |

## Ping-pong round trip (client send → echo decoded; "from when due": from the slot on the paced schedule, including any wait behind a stall)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189944 | 40.52 | 40.10 | 42.58 | 51.44 | 65.28 | 75.35 | 717.85 |
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189944 | 40.69 | 40.27 | 42.75 | 51.62 | 65.48 | 76.45 | 718.02 |
| r1-base-0/pp-epoll-spin-0: epoll-spin @ 0/s | 1219216 | 48.29 | 48.98 | 49.50 | 59.47 | 97.56 | 188.48 | 372.55 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 64.33 | 62.77 | 84.22 | 90.77 | 255.32 | 315.38 | 1049.90 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 108.46 | 63.66 | 85.10 | 1231.58 | 8954.86 | 16091.19 | 18215.30 |
| r1-base-0/pp-recv-busy-0: recv-busy @ 0/s | 1518389 | 38.75 | 38.49 | 41.16 | 44.66 | 54.30 | 64.44 | 126.31 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 39.56 | 39.26 | 41.56 | 49.29 | 56.98 | 66.36 | 120.68 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 39.87 | 39.55 | 41.90 | 49.61 | 57.31 | 66.67 | 120.92 |
| r1-base-0/pp-uring-spin-0: uring-spin @ 0/s | 1187174 | 49.03 | 48.38 | 52.31 | 92.23 | 97.77 | 106.61 | 521.08 |
| r1-base-0/pp-uring-spin-10000: uring-spin @ 10000/s | 1189998 | 48.66 | 43.73 | 48.40 | 208.39 | 267.85 | 325.68 | 986.79 |
| r1-base-0/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189998 | 84.05 | 43.84 | 55.84 | 1265.58 | 3491.18 | 7058.56 | 8397.78 |
| r1-base-0/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1473746 | 39.73 | 39.44 | 42.03 | 45.80 | 60.69 | 115.78 | 431.45 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189997 | 42.48 | 41.97 | 45.73 | 50.08 | 57.62 | 94.94 | 732.10 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189997 | 42.62 | 42.05 | 45.81 | 50.17 | 58.37 | 214.78 | 1673.82 |
| r1-base-1/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189944 | 40.29 | 39.93 | 42.38 | 50.33 | 64.00 | 80.07 | 549.39 |
| r1-base-1/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189944 | 40.48 | 40.08 | 42.52 | 50.54 | 64.58 | 134.74 | 1094.60 |
| r1-base-1/pp-epoll-spin-0: epoll-spin @ 0/s | 1196712 | 49.20 | 49.05 | 49.67 | 58.91 | 92.98 | 97.97 | 130.79 |
| r1-base-1/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189998 | 46.52 | 45.71 | 48.67 | 59.61 | 91.77 | 94.25 | 267.34 |
| r1-base-1/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189998 | 46.72 | 45.89 | 48.96 | 59.85 | 92.02 | 94.51 | 267.51 |
| r1-base-1/pp-recv-busy-0: recv-busy @ 0/s | 1547627 | 38.02 | 37.73 | 40.52 | 44.29 | 53.42 | 70.82 | 242.43 |
| r1-base-1/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 40.75 | 40.44 | 42.87 | 46.21 | 60.59 | 99.64 | 691.15 |
| r1-base-1/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 41.12 | 40.72 | 43.17 | 46.53 | 61.68 | 422.76 | 1425.84 |
| r1-base-1/pp-uring-spin-0: uring-spin @ 0/s | 1197721 | 48.59 | 48.47 | 49.08 | 59.38 | 94.86 | 102.98 | 431.99 |
| r1-base-1/pp-uring-spin-10000: uring-spin @ 10000/s | 1189999 | 46.90 | 44.89 | 48.56 | 90.90 | 93.29 | 163.29 | 1048.55 |
| r1-base-1/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189999 | 47.24 | 44.95 | 48.64 | 91.00 | 93.65 | 1165.38 | 3735.98 |
| r1-base-1/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1478388 | 39.61 | 39.37 | 41.94 | 45.44 | 52.13 | 63.30 | 171.56 |
| r1-base-1/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 39.64 | 39.43 | 41.61 | 44.33 | 55.36 | 67.00 | 144.69 |
| r1-base-1/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 39.75 | 39.52 | 41.73 | 44.50 | 55.52 | 67.62 | 426.33 |

