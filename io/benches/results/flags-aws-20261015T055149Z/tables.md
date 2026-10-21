# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|
| r1-base-0 | 108.36 | 145.5 | 2.0 | 0.5 | 108.36 µs stall: softirq TIMER |
| r1-base-1 | 375.59 | 152.0 | 1.5 | 0.5 | 375.59 µs stall: softirq TIMER |

## HTTP round trip, http-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189947 | 40.56 | 55.11 | 85.85 | 99.96 | 1001.93 | 59.5 | 1.0 | 0.5 | 0.0 | 359.85 | 1638.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189947 | 39.53 | 54.07 | 80.56 | 94.20 | 202010.47 | 38.3 | 1.0 | 0.5 | 0.5 | 194.29 | 1704.5 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189947 | 40.70 | 55.32 | 86.26 | 114.58 | 1002.07 | 90.8 | 8.1 | 0.5 | 0.0 |
| r1-base-1 | 1189947 | 39.68 | 58.42 | 130053.04 | 194869.97 | 202010.61 | 1738.1 | 1676.5 | 1667.5 | 1593.3 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 2899950 | 2.56 | 6.86 | 14.96 | 34.52 | 87.93 | 0.0 | 0.0 | 0.0 | 0.0 | 530.40 | 5674.0 | 530.40 µs stall: loop work (2400 items) |
| r1-base-1 | 2900000 | 2.19 | 6.40 | 11.76 | 31.60 | 45.62 | 0.0 | 0.0 | 0.0 | 0.0 | 470.12 | 5788.0 | 470.12 µs stall: loop work (2400 items) |

## WebSocket round trip, pp-epoll-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1271224 | 45.76 | 58.96 | 73.07 | 103.06 | 1016.26 | 151.5 | 1.0 | 1.0 | 0.0 | 34.53 | 62.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1299250 | 44.54 | 57.24 | 80.47 | 214.38 | 693.50 | 772.9 | 2.0 | 0.0 | 0.0 | 33.28 | 16.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-epoll-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 46.20 | 85.99 | 93.70 | 106.32 | 26739.70 | 116.5 | 0.5 | 0.5 | 0.5 | 43.48 | 164.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189999 | 46.54 | 55.77 | 71.90 | 103.97 | 514.98 | 70.1 | 0.5 | 0.0 | 0.0 | 36.43 | 17.0 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-epoll-spin-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 46.38 | 86.61 | 95.68 | 20757.64 | 26739.91 | 414.0 | 265.2 | 255.1 | 164.4 |
| r1-base-1 | 1189999 | 46.69 | 55.95 | 72.72 | 152.36 | 715.21 | 120.5 | 10.1 | 0.0 | 0.0 |

## WebSocket round trip, pp-recv-busy-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1456185 | 39.37 | 56.49 | 72.09 | 94.00 | 1047.97 | 84.4 | 1.0 | 1.0 | 0.0 | 34.24 | 959.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1522559 | 37.98 | 50.15 | 62.71 | 82.01 | 308.52 | 47.8 | 0.0 | 0.0 | 0.0 | 266.46 | 984.0 | client: mostly unexplained (+softirq NET_RX) |

## WebSocket round trip, pp-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 40.39 | 54.35 | 69.08 | 101.04 | 235.25 | 61.5 | 0.0 | 0.0 | 0.0 | 117.22 | 818.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189999 | 39.57 | 50.00 | 62.93 | 86.64 | 756.98 | 31.3 | 1.5 | 0.0 | 0.0 | 132.38 | 674.0 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 40.68 | 54.67 | 69.74 | 110.56 | 825.28 | 82.7 | 7.1 | 0.0 | 0.0 |
| r1-base-1 | 1189999 | 39.86 | 50.33 | 63.68 | 118.84 | 2312.28 | 73.1 | 29.7 | 17.6 | 0.0 |

## WebSocket round trip, pp-uring-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1265493 | 44.27 | 71.60 | 97.64 | 107.05 | 379.34 | 379.3 | 0.0 | 0.0 | 0.0 | 234.90 | 28.0 | client: softirq TIMER |
| r1-base-1 | 1242864 | 46.47 | 59.21 | 72.65 | 97.59 | 394.26 | 105.8 | 0.0 | 0.0 | 0.0 | 371.81 | 25.0 | client: softirq TIMER |

## WebSocket round trip, pp-uring-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189998 | 45.02 | 91.91 | 98.82 | 116.55 | 277.46 | 475.0 | 0.0 | 0.0 | 0.0 | 42.72 | 168.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189998 | 47.81 | 60.73 | 75.78 | 130.41 | 486.42 | 178.0 | 0.0 | 0.0 | 0.0 | 44.45 | 129.5 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-uring-spin-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189998 | 45.12 | 92.03 | 99.44 | 127.97 | 680.91 | 548.1 | 8.6 | 0.0 | 0.0 |
| r1-base-1 | 1189998 | 47.87 | 60.89 | 77.29 | 134.82 | 871.34 | 207.7 | 6.6 | 0.0 | 0.0 |

## WebSocket round trip, pp-uring-sqpoll-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1419257 | 40.37 | 55.67 | 70.56 | 99.29 | 354.84 | 132.2 | 0.0 | 0.0 | 0.0 | 27.23 | 3.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1472174 | 39.15 | 51.85 | 65.95 | 92.21 | 577.71 | 105.8 | 1.0 | 0.0 | 0.0 | 26.49 | 4.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-sqpoll-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189997 | 40.48 | 55.44 | 70.95 | 98.94 | 949.51 | 56.0 | 0.5 | 0.0 | 0.0 | 342.75 | 2.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189997 | 40.21 | 51.08 | 65.28 | 94.88 | 404.22 | 45.4 | 0.0 | 0.0 | 0.0 | 192.65 | 2.0 | client: softirq TIMER |

### WebSocket round trip, pp-uring-sqpoll-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189997 | 40.55 | 55.54 | 71.59 | 105.34 | 949.58 | 74.6 | 4.0 | 0.0 | 0.0 |
| r1-base-1 | 1189997 | 40.28 | 51.18 | 65.68 | 103.83 | 602.88 | 67.6 | 2.0 | 0.0 | 0.0 |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs. The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | receiving CPU | LOC | IWI | RES | CAL |
|---|---|---|---|---|---|---|---|---|---|---|---|
| http-recv-busy-10000 | r1-base-0 | 3 | 0 | 0 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-base-1 | 3 | 1 | 1 | 0 | 9 | | | | | | |
| md-recv-busy | r1-base-0 | 3 | 12 | 12 | 0 | 16 | | | | | | |
| md-recv-busy | r1-base-1 | 3 | 8 | 11 | 0 | 16 | | | | | | |
| noise | r1-base-0 | 3 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-base-1 | 3 | 0 | 0 | 0 | 8 | | | | | | |
| pp-epoll-spin-0 | r1-base-0 | 3 | 0 | 0 | 0 | 16 | 0 | 1139 | 0 | 1 | 6 |
| pp-epoll-spin-0 | r1-base-1 | 3 | 0 | 0 | 0 | 16 | 4 | 1011 | 0 | 1 | 6 |
| pp-epoll-spin-10000 | r1-base-0 | 3 | 0 | 0 | 0 | 8 | 4 | 262 | 0 | 0 | 14 |
| pp-epoll-spin-10000 | r1-base-1 | 3 | 0 | 0 | 0 | 8 | 4 | 922 | 0 | 0 | 6 |
| pp-recv-busy-0 | r1-base-0 | 3 | 35 | 35 | 0 | 16 | 4 | 1016 | 0 | 1 | 13 |
| pp-recv-busy-0 | r1-base-1 | 3 | 32 | 31 | 0 | 16 | 0 | 1142 | 0 | 0 | 12 |
| pp-recv-busy-10000 | r1-base-0 | 3 | 30 | 29 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-base-1 | 3 | 19 | 15 | 0 | 8 | | | | | | |
| pp-uring-spin-0 | r1-base-0 | 3 | 0 | 0 | 0 | 17 | 4 | 1014 | 0 | 1 | 14 |
| pp-uring-spin-0 | r1-base-1 | 3 | 0 | 0 | 0 | 17 | 4 | 1014 | 0 | 1 | 12 |
| pp-uring-spin-10000 | r1-base-0 | 3 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 1 | 8 |
| pp-uring-spin-10000 | r1-base-1 | 3 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 0 | 10 |
| pp-uring-sqpoll-0 | r1-base-0 | 3 | 0 | 0 | 0 | 17 | 1 | 78 | 37 | 1 | 17 |
| pp-uring-sqpoll-0 | r1-base-1 | 3 | 0 | 0 | 0 | 17 | 1 | 66 | 30 | 1 | 17 |
| pp-uring-sqpoll-10000 | r1-base-0 | 3 | 0 | 0 | 0 | 8 | 1 | 88 | 41 | 1 | 8 |
| pp-uring-sqpoll-10000 | r1-base-1 | 3 | 0 | 0 | 0 | 8 | 1 | 42 | 19 | 1 | 8 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### http-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 1001.93 at 68.5 s: neither loop stalled long enough: network, hypervisor or untraced; 915.40 at 64.9 s: neither loop stalled long enough: network, hypervisor or untraced; 222.89 at 113.2 s: neither loop stalled long enough: network, hypervisor or untraced; 214.49 at 33.9 s: neither loop stalled long enough: network, hypervisor or untraced; 190.32 at 33.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 202010.47 at 77.2 s: neither loop stalled long enough: network, hypervisor or untraced; 688.09 at 83.7 s: neither loop stalled long enough: network, hypervisor or untraced; 402.81 at 43.4 s: neither loop stalled long enough: network, hypervisor or untraced; 333.69 at 88.4 s: neither loop stalled long enough: network, hypervisor or untraced; 317.88 at 43.4 s: neither loop stalled long enough: network, hypervisor or untraced |

### md-recv-busy

| condition | worst events |
|---|---|
| r1-base-0 | 87.93 at 39.2 s: client: softirq NET_RX; 87.79 at 39.2 s: client: softirq NET_RX; 87.54 at 39.2 s: client: softirq NET_RX; 83.42 at 24.3 s: client: loop work (10 items); 83.28 at 24.3 s: client: loop work (10 items) |
| r1-base-1 | 45.62 at 38.6 s: client: loop work (15 items); 45.47 at 38.6 s: client: loop work (15 items); 45.32 at 38.6 s: client: loop work (15 items); 45.17 at 38.6 s: client: loop work (15 items); 45.02 at 38.6 s: client: loop work (15 items) |

### noise

| condition | worst events |
|---|---|
| r1-base-0 | 108.36 at 101.1 s: softirq TIMER; 26.72 at 6.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 21.17 at 109.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 20.50 at 0.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.67 at 50.2 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-1 | 375.59 at 111.7 s: softirq TIMER; 27.52 at 26.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 21.28 at 94.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.52 at 21.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.07 at 0.4 s: unexplained: no kernel event (hypervisor or in-kernel time) |

### pp-epoll-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 1016.26 at 33.9 s: neither loop stalled long enough: network, hypervisor or untraced; 166.45 at 57.5 s: neither loop stalled long enough: network, hypervisor or untraced; 165.35 at 44.1 s: neither loop stalled long enough: network, hypervisor or untraced; 158.51 at 3.5 s: neither loop stalled long enough: network, hypervisor or untraced; 156.93 at 59.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 693.50 at 31.2 s: neither loop stalled long enough: network, hypervisor or untraced; 520.07 at 31.2 s: neither loop stalled long enough: network, hypervisor or untraced; 487.84 at 43.5 s: neither loop stalled long enough: network, hypervisor or untraced; 478.77 at 43.8 s: neither loop stalled long enough: network, hypervisor or untraced; 470.73 at 43.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-epoll-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 26739.70 at 84.5 s: neither loop stalled long enough: network, hypervisor or untraced; 481.02 at 97.3 s: neither loop stalled long enough: network, hypervisor or untraced; 368.26 at 97.3 s: neither loop stalled long enough: network, hypervisor or untraced; 336.92 at 97.3 s: neither loop stalled long enough: network, hypervisor or untraced; 280.96 at 13.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 514.98 at 6.8 s: neither loop stalled long enough: network, hypervisor or untraced; 379.64 at 48.9 s: neither loop stalled long enough: network, hypervisor or untraced; 358.54 at 50.6 s: neither loop stalled long enough: network, hypervisor or untraced; 314.03 at 6.7 s: neither loop stalled long enough: network, hypervisor or untraced; 253.35 at 50.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-0

| condition | worst events |
|---|---|
| r1-base-0 | 1047.97 at 48.3 s: neither loop stalled long enough: network, hypervisor or untraced; 305.26 at 4.1 s: neither loop stalled long enough: network, hypervisor or untraced; 295.91 at 4.1 s: neither loop stalled long enough: network, hypervisor or untraced; 274.62 at 4.1 s: neither loop stalled long enough: network, hypervisor or untraced; 180.23 at 4.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 308.52 at 37.2 s: client: mostly unexplained (+softirq NET_RX); 225.25 at 25.8 s: neither loop stalled long enough: network, hypervisor or untraced; 155.66 at 25.8 s: neither loop stalled long enough: network, hypervisor or untraced; 154.86 at 18.4 s: neither loop stalled long enough: network, hypervisor or untraced; 145.05 at 11.1 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 235.25 at 40.5 s: neither loop stalled long enough: network, hypervisor or untraced; 232.74 at 17.7 s: neither loop stalled long enough: network, hypervisor or untraced; 226.94 at 17.7 s: neither loop stalled long enough: network, hypervisor or untraced; 222.63 at 17.7 s: neither loop stalled long enough: network, hypervisor or untraced; 213.16 at 17.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 756.98 at 114.6 s: neither loop stalled long enough: network, hypervisor or untraced; 707.13 at 114.6 s: neither loop stalled long enough: network, hypervisor or untraced; 524.85 at 114.5 s: neither loop stalled long enough: network, hypervisor or untraced; 478.82 at 114.6 s: neither loop stalled long enough: network, hypervisor or untraced; 351.32 at 114.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 379.34 at 18.7 s: client: softirq TIMER; 245.67 at 18.5 s: neither loop stalled long enough: network, hypervisor or untraced; 235.57 at 22.3 s: server: softirq TIMER; 221.82 at 18.5 s: neither loop stalled long enough: network, hypervisor or untraced; 214.30 at 18.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 394.26 at 21.1 s: client: softirq TIMER; 258.60 at 24.7 s: server: softirq TIMER; 194.74 at 58.5 s: neither loop stalled long enough: network, hypervisor or untraced; 184.57 at 42.3 s: neither loop stalled long enough: network, hypervisor or untraced; 184.52 at 58.5 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 277.46 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced; 242.69 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced; 240.26 at 26.4 s: neither loop stalled long enough: network, hypervisor or untraced; 239.66 at 1.3 s: neither loop stalled long enough: network, hypervisor or untraced; 225.42 at 1.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 486.42 at 81.4 s: neither loop stalled long enough: network, hypervisor or untraced; 311.01 at 81.4 s: neither loop stalled long enough: network, hypervisor or untraced; 301.29 at 59.2 s: neither loop stalled long enough: network, hypervisor or untraced; 249.88 at 81.4 s: neither loop stalled long enough: network, hypervisor or untraced; 216.96 at 15.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-0

| condition | worst events |
|---|---|
| r1-base-0 | 354.84 at 59.8 s: neither loop stalled long enough: network, hypervisor or untraced; 294.29 at 59.9 s: neither loop stalled long enough: network, hypervisor or untraced; 282.91 at 54.8 s: neither loop stalled long enough: network, hypervisor or untraced; 269.04 at 59.9 s: neither loop stalled long enough: network, hypervisor or untraced; 259.26 at 59.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 577.71 at 51.0 s: neither loop stalled long enough: network, hypervisor or untraced; 470.96 at 51.0 s: neither loop stalled long enough: network, hypervisor or untraced; 340.22 at 4.3 s: neither loop stalled long enough: network, hypervisor or untraced; 314.70 at 51.0 s: neither loop stalled long enough: network, hypervisor or untraced; 301.06 at 4.3 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-10000

| condition | worst events |
|---|---|
| r1-base-0 | 949.51 at 72.3 s: neither loop stalled long enough: network, hypervisor or untraced; 370.32 at 63.4 s: client: softirq TIMER; 203.48 at 51.9 s: neither loop stalled long enough: network, hypervisor or untraced; 200.07 at 114.2 s: neither loop stalled long enough: network, hypervisor or untraced; 175.38 at 10.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 404.22 at 71.6 s: client: softirq TIMER; 252.17 at 32.0 s: neither loop stalled long enough: network, hypervisor or untraced; 244.92 at 35.6 s: neither loop stalled long enough: network, hypervisor or untraced; 234.12 at 35.6 s: neither loop stalled long enough: network, hypervisor or untraced; 216.83 at 31.8 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| r1-base-0 | loop work 151938.06; unexplained: no kernel event (hypervisor or in-kernel time) 150177.99; mostly unexplained (+preempted by kworker/3:1) 45448.87; mostly unexplained (+softirq NET_RX) 28183.28; softirq NET_RX 12065.75 |
| r1-base-1 | loop work 161087.56; unexplained: no kernel event (hypervisor or in-kernel time) 134774.38; mostly unexplained (+preempted by kworker/3:1) 41174.40; mostly unexplained (+softirq NET_RX) 30575.60; softirq NET_RX 8858.79 |

## Once-a-minute stalls

Whether each source's events of 300 µs or more (loop stalls, or samples that slow) bunch at one second of the wall-clock minute. The busiest second is the middle of the busiest three; three seconds hold one event in twenty by chance, so a share of 25% or more over at least five events is marked periodic. Each host has its own second (its agents' timers start at boot).

| run | source | events | busiest second | events there (share) | minutes with one there | longest there (µs) | periodic |
|---|---|---|---|---|---|---|---|
| cpumap-receiver/cpu1 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu2 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu3 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu5 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu6 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu7 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu1 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu3 | counter-only spinner | 0 | | | | | |
| http-recv-busy-10000 | idle spare core | 1 | :00 | 1 (100%) | 1 of 4 | 378.04 | no |
| http-recv-busy-10000 | receiver loop | 1 | :09 | 1 (100%) | 1 of 4 | 359.85 | no |
| http-recv-busy-10000 | sender loop | 0 | | | | | |
| http-recv-busy-10000 | slow samples | 7 | :16 | 2 (29%) | 1 of 4 | 402.81 | no |
| md-recv-busy | idle spare core | 0 | | | | | |
| md-recv-busy | receiver loop | 0 | | | | | |
| md-recv-busy | slow samples | 0 | | | | | |
| noise | noise probe (loop CPU) | 1 | :37 | 1 (100%) | 1 of 4 | 375.59 | no |
| pp-epoll-spin-0 | idle spare core | 0 | | | | | |
| pp-epoll-spin-0 | receiver loop | 0 | | | | | |
| pp-epoll-spin-0 | sender loop | 0 | | | | | |
| pp-epoll-spin-0 | slow samples | 28 | :50 | 24 (86%) | 2 of 2 | 1016.26 | **yes** |
| pp-epoll-spin-10000 | idle spare core | 0 | | | | | |
| pp-epoll-spin-10000 | receiver loop | 0 | | | | | |
| pp-epoll-spin-10000 | sender loop | 0 | | | | | |
| pp-epoll-spin-10000 | slow samples | 8 | :28 | 5 (62%) | 2 of 4 | 481.02 | **yes** |
| pp-recv-busy-0 | idle spare core | 0 | | | | | |
| pp-recv-busy-0 | receiver loop | 0 | | | | | |
| pp-recv-busy-0 | sender loop | 0 | | | | | |
| pp-recv-busy-0 | slow samples | 3 | :04 | 1 (33%) | 1 of 2 | 308.52 | no |
| pp-recv-busy-10000 | idle spare core | 0 | | | | | |
| pp-recv-busy-10000 | receiver loop | 0 | | | | | |
| pp-recv-busy-10000 | sender loop | 0 | | | | | |
| pp-recv-busy-10000 | slow samples | 6 | :54 | 6 (100%) | 1 of 4 | 756.98 | **yes** |
| pp-uring-spin-0 | idle spare core | 0 | | | | | |
| pp-uring-spin-0 | receiver loop | 1 | :32 | 1 (100%) | 1 of 2 | 371.81 | no |
| pp-uring-spin-0 | sender loop | 0 | | | | | |
| pp-uring-spin-0 | slow samples | 2 | :32 | 1 (50%) | 1 of 2 | 394.26 | no |
| pp-uring-spin-10000 | idle spare core | 0 | | | | | |
| pp-uring-spin-10000 | receiver loop | 0 | | | | | |
| pp-uring-spin-10000 | sender loop | 0 | | | | | |
| pp-uring-spin-10000 | slow samples | 3 | :05 | 2 (67%) | 1 of 4 | 486.42 | no |
| pp-uring-sqpoll-0 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-0 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-0 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-0 | slow samples | 6 | :07 | 3 (50%) | 1 of 2 | 577.71 | no |
| pp-uring-sqpoll-10000 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-10000 | receiver loop | 1 | :14 | 1 (100%) | 1 of 4 | 342.75 | no |
| pp-uring-sqpoll-10000 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-10000 | slow samples | 3 | :05 | 1 (33%) | 1 of 4 | 404.22 | no |

# Every run in detail

#### Stalls: r1-base-0/http-recv-busy-10000, loop 0 on CPU 3

2140307147 iterations, longest 359.8 µs; over 5 / 20 / 100 µs: 17986 / 3276 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2828 | 124618.9 | 84.3 |
| mostly unexplained (+softirq NET_RX) | 407 | 18612.3 | 103.4 |
| mostly unexplained (+preempted by kworker/3:1) | 31 | 1399.3 | 85.4 |
| softirq TIMER | 1 | 359.8 | 359.8 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 117.2 | 41.9 |
| softirq NET_RX | 3 | 97.8 | 36.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 52.3 | 27.1 |
| loop work | 1 | 22.6 | 22.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 359.8 | 1 | softirq TIMER | softirq TIMER 341, local timer tick 10, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 103.4 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 30, preempted by kworker/3:1 10, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 91.8 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 45, preempted by kworker/3:1 9, irq work (IPI/vector) 2, workqueue [ena] 1 |
| 4 | 85.4 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 84.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 82.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 80.6 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 80.2 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 76.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 76.3 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 45 |
| LOC Local timer interrupts | 37 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/http-recv-busy-10000 (median 40.56)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1001.93 | 961.37 | 0.00 |  | 0.00 |  | 961.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 915.40 | 874.85 | 0.00 |  | 0.00 |  | 874.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 222.89 | 182.33 | 0.00 |  | 0.00 |  | 182.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 214.49 | 173.93 | 0.00 |  | 0.00 |  | 173.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 190.32 | 149.76 | 0.00 |  | 0.00 |  | 149.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 165.51 | 124.95 | 0.00 |  | 0.00 |  | 124.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 158.73 | 118.17 | 0.00 |  | 0.00 |  | 118.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 155.44 | 114.89 | 0.00 |  | 0.00 |  | 114.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 153.32 | 112.76 | 0.00 |  | 0.00 |  | 112.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 152.61 | 112.06 | 0.00 |  | 0.00 |  | 112.06 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-0/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=502603 clocksource=tsc

### Loop counters (r1-base-0/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002400 | 568112 | 5.28 | 119493568 | 39.80 | 1991559 | 1991559 | 99.5 | 67.0 | true | 0 | 738 | 530.4 | 76828 / 5674 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672537600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (85947 of 2899950 in 1-msg reads) | 85947 | 2.43 | 2.08 | 3.46 | 5.57 | 11.73 | 31.05 | 41.61 |

#### Stalls: r1-base-0/md-recv-busy, loop 0 on CPU 3

119493568 iterations, longest 530.4 µs; over 5 / 20 / 100 µs: 76828 / 5674 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5282 | 151915.5 | 530.4 |
| mostly unexplained (+preempted by kworker/3:1) | 159 | 4233.6 | 49.5 |
| softirq NET_RX | 120 | 3418.9 | 94.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 97 | 2310.5 | 46.1 |
| mostly unexplained (+softirq NET_RX) | 11 | 261.0 | 27.6 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 51.0 | 29.8 |
| local timer tick | 1 | 26.1 | 26.1 |
| mostly unexplained (+ipi_send_cpu) | 1 | 25.7 | 25.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 25.5 | 25.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 530.4 | 2400 | loop work (2400 items) |  |
| 2 | 94.5 | 3 | softirq NET_RX | softirq NET_RX 61, preempted by kworker/3:1 15, irq work (IPI/vector) 3, workqueue [ena] 2 |
| 3 | 74.3 | 27 | loop work (27 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 73.4 | 23 | loop work (23 items) |  |
| 5 | 68.9 | 22 | loop work (22 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 65.7 | 22 | loop work (22 items) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 65.4 | 16 | loop work (16 items) | softirq NET_RX 22, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 61.8 | 14 | loop work (14 items) | softirq NET_RX 23, preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 61.5 | 22 | loop work (22 items) | softirq NET_RX 16, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 60.1 | 19 | loop work (19 items) | softirq NET_RX 6 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 750 |
| LOC Local timer interrupts | 741 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-0/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-0/noise, loop 0 on CPU 3

9538752512 iterations, longest 108.4 µs; over 5 / 20 / 100 µs: 291 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 278 | 2794.4 | 26.7 |
| softirq TIMER | 1 | 108.4 | 108.4 |
| mostly unexplained (+call function (IPI/vector)) | 12 | 63.4 | 6.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 108.4 | 0 | softirq TIMER | softirq TIMER 90, local timer tick 11, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 26.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 21.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 20.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 18.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 17.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 16.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 15.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 15.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 15.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| IWI IRQ work interrupts | 1 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (r1-base-0/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1292988 | 1292988 | 1.00 | 305356921 | 236.16 | 5089282 | 5046182 | 0.0 | 39.1 | false | 0 | 0 | 34.5 | 3115 / 62 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-0, loop 0 on CPU 3

302770944 iterations, longest 34.5 µs; over 5 / 20 / 100 µs: 3115 / 62 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 60 | 1439.1 | 34.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.8 | 26.8 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 22.4 | 22.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 34.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 32.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 32.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 31.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 30.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 29.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 29.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 28.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 28.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 27.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 10 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-0 (median 45.76)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1016.26 | 970.50 | 0.00 |  | 0.00 |  | 970.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 166.45 | 120.70 | 0.00 |  | 0.00 |  | 120.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 165.35 | 119.60 | 0.00 |  | 0.00 |  | 119.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 158.51 | 112.75 | 0.00 |  | 0.00 |  | 112.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 156.93 | 111.17 | 0.00 |  | 0.00 |  | 111.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 154.50 | 108.74 | 0.00 |  | 0.00 |  | 108.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 153.96 | 108.20 | 0.00 |  | 0.00 |  | 108.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 151.09 | 105.33 | 0.00 |  | 0.00 |  | 105.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 145.57 | 99.81 | 0.00 |  | 0.00 |  | 99.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 144.67 | 98.91 | 0.00 |  | 0.00 |  | 98.91 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 587936192 | 489.95 | 4899468 | 4879468 | 0.0 | 36.2 | false | 0 | 0 | 43.5 | 24416 / 329 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-10000, loop 0 on CPU 3

585536192 iterations, longest 43.5 µs; over 5 / 20 / 100 µs: 24416 / 329 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 326 | 7862.4 | 43.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 47.0 | 25.8 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 20.7 | 20.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 43.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 39.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 37.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 37.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 35.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 35.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 33.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 33.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 33.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 33.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1012 |
| IWI IRQ work interrupts | 5 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-10000 (median 46.20)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 26739.70 | 26693.50 | 0.00 |  | 24.85 | mostly unexplained (+call function single (IPI/vector)) | 26668.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 481.02 | 434.82 | 0.00 |  | 0.00 |  | 434.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 368.26 | 322.06 | 0.00 |  | 0.00 |  | 322.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 336.92 | 290.72 | 0.00 |  | 0.00 |  | 290.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 280.96 | 234.76 | 0.00 |  | 0.00 |  | 234.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 253.49 | 207.29 | 0.00 |  | 0.00 |  | 207.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 245.54 | 199.34 | 0.00 |  | 0.00 |  | 199.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 240.09 | 193.89 | 0.00 |  | 0.00 |  | 193.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 197.58 | 151.38 | 0.00 |  | 0.00 |  | 151.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 183.24 | 137.03 | 0.00 |  | 21.55 | unexplained: no kernel event (hypervisor or in-kernel time) | 115.48 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1481793 | 1481793 | 1.00 | 113229698 | 76.41 | 1887162 | 1862465 | 98.7 | 71.5 | true | 0 | 2075 | 34.2 | 43498 / 959 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-0, loop 0 on CPU 3

111747904 iterations, longest 34.2 µs; over 5 / 20 / 100 µs: 43498 / 959 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 586 | 13466.8 | 34.2 |
| softirq NET_RX | 177 | 4044.0 | 31.3 |
| mostly unexplained (+softirq NET_RX) | 147 | 3589.9 | 34.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 41 | 906.3 | 29.5 |
| mostly unexplained (+local timer tick) | 7 | 169.9 | 28.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 27.7 | 27.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 34.2 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 2 | 34.1 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 17, preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 34.1 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/3:1 9, local timer tick 4, timer tick_nohz_handler 2 |
| 4 | 33.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 16, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 32.6 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 11, preempted by kworker/3:1 10, workqueue [ena] 2, irq work (IPI/vector) 1 |
| 6 | 31.3 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 31.3 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 31.3 | 0 | softirq NET_RX | softirq NET_RX 18, preempted by kworker/3:1 5, irq work (IPI/vector) 4, workqueue [ena] 1 |
| 9 | 31.2 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 30.9 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 10, local timer tick 3, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2116 |
| IWI IRQ work interrupts | 2078 |
| CAL Function call interrupts | 989 |

#### Slowest round trips: r1-base-0/pp-recv-busy-0 (median 39.37)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1047.97 | 1008.60 | 0.00 |  | 0.00 |  | 1008.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 305.26 | 265.89 | 0.00 |  | 0.00 |  | 265.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 295.91 | 256.54 | 0.00 |  | 0.00 |  | 256.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 274.62 | 235.25 | 0.00 |  | 0.00 |  | 235.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 180.23 | 140.86 | 0.00 |  | 0.00 |  | 140.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 174.40 | 135.03 | 0.00 |  | 0.00 |  | 135.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 163.51 | 124.14 | 0.00 |  | 0.00 |  | 124.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 163.35 | 123.97 | 0.00 |  | 0.00 |  | 123.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 161.59 | 122.22 | 0.00 |  | 0.00 |  | 122.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 148.57 | 109.19 | 0.00 |  | 0.00 |  | 109.19 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 237037312 | 197.53 | 1975311 | 1965311 | 99.5 | 67.9 | true | 0 | 3438 | 117.2 | 66124 / 1637 / 1 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-10000, loop 0 on CPU 3

235837312 iterations, longest 117.2 µs; over 5 / 20 / 100 µs: 66124 / 1637 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 1133 | 26349.3 | 117.2 |
| mostly unexplained (+softirq NET_RX) | 237 | 5720.1 | 43.6 |
| softirq NET_RX | 193 | 4505.0 | 84.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 67 | 1573.3 | 40.2 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 82.0 | 33.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.5 | 31.8 |
| mostly unexplained (+local timer tick) | 2 | 44.6 | 24.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 117.2 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 35, workqueue [ena] 6, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 84.7 | 1 | softirq NET_RX | softirq NET_RX 59, irq work (IPI/vector) 5, preempted by kworker/3:1 5, workqueue [ena] 1 |
| 3 | 73.4 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 12, irq work (IPI/vector) 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 65.4 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 60.8 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 50.9 | 1 | softirq NET_RX | softirq NET_RX 29, preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 50.8 | 1 | softirq NET_RX | softirq NET_RX 26, preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 47.8 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 12, irq work (IPI/vector) 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 47.0 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 45.7 | 1 | softirq NET_RX | softirq NET_RX 23, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3636 |
| IWI IRQ work interrupts | 3446 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-base-0/pp-recv-busy-10000 (median 40.39)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 235.25 | 194.86 | 0.00 |  | 0.00 |  | 194.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 232.74 | 192.35 | 0.00 |  | 0.00 |  | 192.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 226.94 | 186.55 | 0.00 |  | 0.00 |  | 186.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 222.63 | 182.24 | 0.00 |  | 0.00 |  | 182.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 213.16 | 172.77 | 0.00 |  | 0.00 |  | 172.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 195.27 | 154.88 | 0.00 |  | 0.00 |  | 154.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 191.77 | 151.38 | 0.00 |  | 0.00 |  | 151.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 190.36 | 149.96 | 0.00 |  | 0.00 |  | 149.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 182.34 | 141.95 | 0.00 |  | 0.00 |  | 141.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 179.88 | 139.49 | 0.00 |  | 0.00 |  | 139.49 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1287388 | 1287388 | 1.00 | 3862165 | 3.00 | 64369 | 47747509 | 0.0 | 8.8 | false | 0 | 0 | 234.9 | 31476 / 28 / 1 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-spin-0, loop 0 on CPU 3

2864850560 iterations, longest 234.9 µs; over 5 / 20 / 100 µs: 31476 / 28 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 25 | 572.7 | 31.2 |
| softirq TIMER | 1 | 234.9 | 234.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 39.0 | 39.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.0 | 28.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 234.9 | 0 | softirq TIMER | softirq TIMER 217, local timer tick 9, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 39.0 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 31.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 29.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 28.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 27.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 25.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 25.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 25.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 25.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 13 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/pp-uring-spin-0 (median 44.27)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 379.34 | 335.07 | 234.90 | softirq TIMER | 0.00 |  | 100.17 | client: softirq TIMER |
| 2 | 245.67 | 201.40 | 0.00 |  | 0.00 |  | 201.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 235.57 | 191.30 | 0.00 |  | 220.94 | softirq TIMER | 0.00 | server: softirq TIMER |
| 4 | 221.82 | 177.55 | 0.00 |  | 0.00 |  | 177.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 214.30 | 170.04 | 0.00 |  | 0.00 |  | 170.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 211.13 | 166.86 | 0.00 |  | 0.00 |  | 166.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 206.92 | 162.65 | 0.00 |  | 0.00 |  | 162.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 200.77 | 156.50 | 0.00 |  | 0.00 |  | 156.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 199.51 | 155.25 | 0.00 |  | 0.00 |  | 155.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 195.36 | 151.09 | 0.00 |  | 0.00 |  | 151.09 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1200000 | 1200000 | 1.00 | 3600000 | 3.00 | 30000 | 35614938 | 0.0 | 5.2 | false | 0 | 0 | 42.7 | 43675 / 337 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-spin-10000, loop 0 on CPU 3

4273792512 iterations, longest 42.7 µs; over 5 / 20 / 100 µs: 43675 / 337 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 335 | 8011.0 | 36.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 42.7 | 42.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 29.0 | 29.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 42.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 36.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 35.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 35.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 34.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 34.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 34.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 34.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 32.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-uring-spin-10000 (median 45.02)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 277.46 | 232.44 | 0.00 |  | 0.00 |  | 232.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 242.69 | 197.67 | 0.00 |  | 0.00 |  | 197.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 240.26 | 195.24 | 0.00 |  | 0.00 |  | 195.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 239.66 | 194.64 | 0.00 |  | 0.00 |  | 194.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 225.42 | 180.40 | 0.00 |  | 0.00 |  | 180.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 213.48 | 168.46 | 0.00 |  | 0.00 |  | 168.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 210.98 | 165.96 | 0.00 |  | 0.00 |  | 165.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 198.58 | 153.56 | 0.00 |  | 0.00 |  | 153.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 196.00 | 150.98 | 0.00 |  | 0.00 |  | 150.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 192.65 | 147.63 | 0.00 |  | 0.00 |  | 147.63 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1443566 | 1443566 | 1.00 | 1 | 0.00 | 0 | 40076577 | 0.0 | 0.0 | true | 0 | 0 | 27.2 | 284 / 3 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-0, loop 0 on CPU 3

2404594624 iterations, longest 27.2 µs; over 5 / 20 / 100 µs: 284 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 45.7 | 24.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 27.2 | 27.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 27.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 24.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 20.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 11 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-0 (median 40.37)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 354.84 | 314.47 | 0.00 |  | 0.00 |  | 314.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 294.29 | 253.92 | 0.00 |  | 0.00 |  | 253.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 282.91 | 242.54 | 0.00 |  | 0.00 |  | 242.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 269.04 | 228.67 | 0.00 |  | 0.00 |  | 228.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 259.26 | 218.89 | 0.00 |  | 0.00 |  | 218.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 250.66 | 210.28 | 0.00 |  | 0.00 |  | 210.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 244.15 | 203.78 | 0.00 |  | 0.00 |  | 203.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 229.65 | 189.28 | 0.00 |  | 0.00 |  | 189.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 221.07 | 180.70 | 0.00 |  | 0.00 |  | 180.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 216.61 | 176.24 | 0.00 |  | 0.00 |  | 176.24 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 28884490 | 0.0 | 0.0 | true | 0 | 0 | 342.8 | 626 / 5 / 1 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-10000, loop 0 on CPU 3

3466138752 iterations, longest 342.8 µs; over 5 / 20 / 100 µs: 626 / 5 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 342.8 | 342.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.6 | 31.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 43.6 | 23.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 342.8 | 0 | softirq TIMER | softirq TIMER 326, local timer tick 8, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 31.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 23.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 23.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 20.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| LOC Local timer interrupts | 9 |
| IWI IRQ work interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-10000 (median 40.48)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 949.51 | 909.03 | 0.00 |  | 0.00 |  | 909.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 370.32 | 329.83 | 342.75 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 203.48 | 162.99 | 0.00 |  | 0.00 |  | 162.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 200.07 | 159.59 | 0.00 |  | 0.00 |  | 159.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 175.38 | 134.89 | 0.00 |  | 0.00 |  | 134.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 172.40 | 131.91 | 0.00 |  | 0.00 |  | 131.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 170.51 | 130.03 | 0.00 |  | 0.00 |  | 130.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 163.97 | 123.48 | 0.00 |  | 0.00 |  | 123.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 163.79 | 123.31 | 0.00 |  | 0.00 |  | 123.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 151.38 | 110.90 | 0.00 |  | 0.00 |  | 110.90 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-1/http-recv-busy-10000, loop 0 on CPU 3

2177087468 iterations, longest 194.3 µs; over 5 / 20 / 100 µs: 16749 / 3409 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2902 | 118226.2 | 80.9 |
| mostly unexplained (+softirq NET_RX) | 397 | 16722.1 | 71.5 |
| mostly unexplained (+preempted by kworker/3:1) | 78 | 2787.7 | 70.0 |
| softirq NET_RX | 17 | 429.6 | 35.8 |
| loop work | 7 | 221.7 | 39.9 |
| softirq TIMER | 1 | 194.3 | 194.3 |
| mostly unexplained (+local timer tick) | 3 | 149.2 | 72.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 59.2 | 33.9 |
| mostly unexplained (+call function single (IPI/vector)) | 1 | 40.8 | 40.8 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 37.1 | 37.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 194.3 | 1 | softirq TIMER | softirq TIMER 179, local timer tick 8, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 80.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 74.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 72.3 | 2 | mostly unexplained (+local timer tick) | local timer tick 27, timer tick_nohz_handler 18, preempted by kworker/3:1 7, irq work (IPI/vector) 1 |
| 5 | 71.5 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 70.0 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 68.7 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 67.6 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 67.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 66.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1084 |
| LOC Local timer interrupts | 138 |
| IWI IRQ work interrupts | 130 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/http-recv-busy-10000 (median 39.53)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 202010.47 | 201970.94 | 40.82 | mostly unexplained (+call function single (IPI/vector)) | 0.00 |  | 201930.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 688.09 | 648.57 | 0.00 |  | 0.00 |  | 648.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 402.81 | 363.29 | 0.00 |  | 0.00 |  | 363.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 333.69 | 294.17 | 0.00 |  | 0.00 |  | 294.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 317.88 | 278.35 | 0.00 |  | 0.00 |  | 278.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 287.69 | 248.16 | 0.00 |  | 0.00 |  | 248.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 272.50 | 232.97 | 0.00 |  | 0.00 |  | 232.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 258.91 | 219.38 | 0.00 |  | 0.00 |  | 219.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 237.03 | 197.50 | 0.00 |  | 0.00 |  | 197.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 235.40 | 195.87 | 0.00 |  | 0.00 |  | 195.87 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-1/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=563690 clocksource=tsc

### Loop counters (r1-base-1/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002400 | 644425 | 4.66 | 119527552 | 39.81 | 1992126 | 1992126 | 99.5 | 66.8 | true | 0 | 451 | 470.1 | 41672 / 5788 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672537600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (100158 of 2900000 in 1-msg reads) | 100158 | 2.28 | 2.03 | 3.93 | 5.79 | 12.69 | 32.84 | 38.71 |

#### Stalls: r1-base-1/md-recv-busy, loop 0 on CPU 3

119527552 iterations, longest 470.1 µs; over 5 / 20 / 100 µs: 41672 / 5788 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5400 | 160845.9 | 470.1 |
| mostly unexplained (+preempted by kworker/3:1) | 165 | 4460.3 | 61.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 116 | 2767.1 | 29.5 |
| softirq NET_RX | 80 | 2172.0 | 37.2 |
| mostly unexplained (+softirq NET_RX) | 22 | 645.7 | 38.5 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 55.3 | 29.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.4 | 32.1 |
| mostly unexplained (+local timer tick) | 1 | 33.7 | 33.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 470.1 | 2400 | loop work (2400 items) |  |
| 2 | 70.7 | 15 | loop work (15 items) | local timer tick 7, preempted by kworker/3:1 4, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 3 | 68.1 | 11 | loop work (11 items) | local timer tick 5, preempted by kworker/3:1 3, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 4 | 65.1 | 16 | loop work (16 items) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 62.9 | 17 | loop work (17 items) | preempted by kworker/3:1 10, local timer tick 4, irq work (IPI/vector) 1, softirq RCU 1 |
| 6 | 61.7 | 47 | loop work (47 items) | local timer tick 17, timer tick_nohz_handler 12, softirq NET_RX 5 |
| 7 | 61.6 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 10, local timer tick 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 8 | 61.1 | 17 | loop work (17 items) | preempted by kworker/3:1 10, local timer tick 3, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 9 | 60.3 | 10 | loop work (10 items) | preempted by kworker/3:1 8, local timer tick 3, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 59.6 | 12 | loop work (12 items) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 634 |
| LOC Local timer interrupts | 469 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-1/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-1/noise, loop 0 on CPU 3

9538605056 iterations, longest 375.6 µs; over 5 / 20 / 100 µs: 304 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 293 | 2987.2 | 27.5 |
| softirq TIMER | 1 | 375.6 | 375.6 |
| mostly unexplained (+call function (IPI/vector)) | 10 | 53.1 | 6.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 375.6 | 0 | softirq TIMER | softirq TIMER 351, local timer tick 16, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 27.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 21.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 18.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 18.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 17.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 16.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 16.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 16.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 16.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |
| LOC Local timer interrupts | 2 |
| IWI IRQ work interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (r1-base-1/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1320994 | 1320994 | 1.00 | 305331461 | 231.14 | 5088858 | 5044825 | 0.0 | 39.1 | false | 0 | 0 | 33.3 | 3153 / 16 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-epoll-spin-0, loop 0 on CPU 3

302689472 iterations, longest 33.3 µs; over 5 / 20 / 100 µs: 3153 / 16 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 14 | 316.5 | 33.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 27.9 | 27.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 21.7 | 21.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 33.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 27.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 27.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 24.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 22.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 21.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 21.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 21.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 21.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 6 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-1/pp-epoll-spin-0 (median 44.54)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 693.50 | 648.96 | 0.00 |  | 0.00 |  | 648.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 520.07 | 475.53 | 0.00 |  | 0.00 |  | 475.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 487.84 | 443.30 | 0.00 |  | 0.00 |  | 443.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 478.77 | 434.23 | 0.00 |  | 0.00 |  | 434.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 470.73 | 426.19 | 0.00 |  | 0.00 |  | 426.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 457.07 | 412.53 | 0.00 |  | 0.00 |  | 412.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 419.62 | 375.08 | 0.00 |  | 0.00 |  | 375.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 404.47 | 359.93 | 0.00 |  | 0.00 |  | 359.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 391.16 | 346.62 | 0.00 |  | 0.00 |  | 346.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 389.96 | 345.42 | 0.00 |  | 0.00 |  | 345.42 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 589611904 | 491.34 | 4913433 | 4893433 | 0.0 | 36.0 | false | 0 | 0 | 36.4 | 1408 / 34 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-epoll-spin-10000, loop 0 on CPU 3

587211904 iterations, longest 36.4 µs; over 5 / 20 / 100 µs: 1408 / 34 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 31 | 735.4 | 36.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.9 | 32.4 |
| mostly unexplained (+local timer tick) | 1 | 29.1 | 29.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 36.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 33.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 32.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 29.1 | 0 | mostly unexplained (+local timer tick) | local timer tick 14, timer tick_nohz_handler 6, softirq TIMER 3, softirq RCU 1 |
| 5 | 27.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 26.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 26.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 26.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 25.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 25.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1001 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-epoll-spin-10000 (median 46.54)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 514.98 | 468.44 | 0.00 |  | 0.00 |  | 468.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 379.64 | 333.10 | 0.00 |  | 0.00 |  | 333.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 358.54 | 312.00 | 0.00 |  | 0.00 |  | 312.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 314.03 | 267.49 | 0.00 |  | 0.00 |  | 267.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 253.35 | 206.81 | 0.00 |  | 0.00 |  | 206.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 244.87 | 198.34 | 0.00 |  | 0.00 |  | 198.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 244.60 | 198.07 | 0.00 |  | 0.00 |  | 198.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 232.03 | 185.49 | 0.00 |  | 0.00 |  | 185.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 229.43 | 182.89 | 0.00 |  | 0.00 |  | 182.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 226.75 | 180.21 | 0.00 |  | 0.00 |  | 180.21 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1548706 | 1548706 | 1.00 | 115448035 | 74.54 | 1924134 | 1898322 | 98.6 | 71.0 | true | 0 | 1857 | 266.5 | 26737 / 984 / 1 | 0 | 0 |

#### Stalls: r1-base-1/pp-recv-busy-0, loop 0 on CPU 3

113899328 iterations, longest 266.5 µs; over 5 / 20 / 100 µs: 26737 / 984 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 589 | 14000.9 | 73.3 |
| mostly unexplained (+softirq NET_RX) | 193 | 4832.0 | 266.5 |
| softirq NET_RX | 113 | 2541.1 | 37.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 73 | 1776.2 | 41.9 |
| mostly unexplained (+local timer tick) | 10 | 259.8 | 38.5 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 69.1 | 40.6 |
| preempted by kworker/3:1 | 1 | 47.4 | 47.4 |
| local timer tick | 2 | 46.2 | 23.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 32.6 | 32.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 266.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 111, preempted by kworker/3:1 74, workqueue [ena] 6, irq work (IPI/vector) 5 |
| 2 | 73.3 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 17, workqueue [ena] 3, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 73.3 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 48.1 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 47.4 | 0 | preempted by kworker/3:1 | preempted by kworker/3:1 28, workqueue [ena] 6, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 47.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 44.4 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 15, workqueue [ena] 2, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 41.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 41.4 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 3, irq work (IPI/vector) 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 41.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 1911 |
| IWI IRQ work interrupts | 1864 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-1/pp-recv-busy-0 (median 37.98)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 308.52 | 270.55 | 266.35 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 4.20 | client: mostly unexplained (+softirq NET_RX) |
| 2 | 225.25 | 187.28 | 0.00 |  | 0.00 |  | 187.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 155.66 | 117.69 | 0.00 |  | 0.00 |  | 117.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 154.86 | 116.89 | 0.00 |  | 0.00 |  | 116.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 145.05 | 107.08 | 0.00 |  | 0.00 |  | 107.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 144.50 | 106.52 | 0.00 |  | 0.00 |  | 106.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 139.95 | 101.97 | 0.00 |  | 0.00 |  | 101.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 135.63 | 97.66 | 0.00 |  | 0.00 |  | 97.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 133.94 | 95.97 | 0.00 |  | 0.00 |  | 95.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 132.73 | 94.75 | 0.00 |  | 0.00 |  | 94.75 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 241880128 | 201.57 | 2015668 | 2005668 | 99.5 | 68.0 | true | 0 | 1825 | 132.4 | 15922 / 1348 / 3 | 0 | 0 |

#### Stalls: r1-base-1/pp-recv-busy-10000, loop 0 on CPU 3

240680128 iterations, longest 132.4 µs; over 5 / 20 / 100 µs: 15922 / 1348 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 776 | 19925.4 | 132.4 |
| mostly unexplained (+softirq NET_RX) | 322 | 8375.7 | 84.5 |
| softirq NET_RX | 164 | 3716.1 | 41.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 47 | 1064.6 | 32.8 |
| mostly unexplained (+local timer tick) | 30 | 921.1 | 95.0 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 115.3 | 34.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.0 | 31.9 |
| preempted by kworker/3:1 | 1 | 33.5 | 33.5 |
| local timer tick | 1 | 28.7 | 28.7 |
| loop work | 1 | 20.0 | 20.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 132.4 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 2 | 123.4 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, irq work (IPI/vector) 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 104.6 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 95.0 | 1 | mostly unexplained (+local timer tick) | local timer tick 33, timer tick_nohz_handler 16, softirq RCU 9, softirq NET_RX 4 |
| 5 | 84.9 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 13, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 84.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 40, preempted by kworker/3:1 11, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 58.8 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 17, workqueue [ena] 2, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 54.6 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 25, softirq NET_RX 10, workqueue [ena] 2, irq work (IPI/vector) 1 |
| 9 | 53.7 | 0 | mostly unexplained (+local timer tick) | local timer tick 10, preempted by kworker/3:1 10, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 10 | 52.4 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2249 |
| IWI IRQ work interrupts | 1832 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-base-1/pp-recv-busy-10000 (median 39.57)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 756.98 | 717.41 | 0.00 |  | 0.00 |  | 717.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 707.13 | 667.56 | 0.00 |  | 0.00 |  | 667.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 524.85 | 485.28 | 0.00 |  | 0.00 |  | 485.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 478.82 | 439.25 | 0.00 |  | 0.00 |  | 439.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 351.32 | 311.75 | 0.00 |  | 0.00 |  | 311.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 311.62 | 272.05 | 0.00 |  | 0.00 |  | 272.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 284.00 | 244.43 | 0.00 |  | 0.00 |  | 244.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 268.02 | 228.45 | 0.00 |  | 0.00 |  | 228.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 260.52 | 220.95 | 0.00 |  | 0.00 |  | 220.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 236.10 | 196.53 | 0.00 |  | 0.00 |  | 196.53 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1263436 | 1263436 | 1.00 | 3790309 | 3.00 | 63172 | 49049878 | 0.0 | 7.0 | false | 0 | 0 | 371.8 | 2332 / 25 / 1 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-spin-0, loop 0 on CPU 3

2942992704 iterations, longest 371.8 µs; over 5 / 20 / 100 µs: 2332 / 25 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 22 | 545.9 | 63.0 |
| softirq TIMER | 1 | 371.8 | 371.8 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 33.4 | 33.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.8 | 28.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 371.8 | 0 | softirq TIMER | softirq TIMER 344, local timer tick 13, timer tick_nohz_handler 7 |
| 2 | 63.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 33.4 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 30.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 28.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 27.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 27.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 26.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 24.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 24.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 12 |
| IWI IRQ work interrupts | 6 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-uring-spin-0 (median 46.47)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 394.26 | 347.79 | 371.81 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 2 | 258.60 | 212.12 | 0.00 |  | 214.11 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 194.74 | 148.26 | 0.00 |  | 0.00 |  | 148.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 184.57 | 138.10 | 0.00 |  | 0.00 |  | 138.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 184.52 | 138.04 | 0.00 |  | 0.00 |  | 138.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 172.53 | 126.06 | 0.00 |  | 0.00 |  | 126.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 167.93 | 121.46 | 0.00 |  | 0.00 |  | 121.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 152.51 | 106.04 | 0.00 |  | 0.00 |  | 106.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 146.18 | 99.71 | 0.00 |  | 0.00 |  | 99.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 145.50 | 99.03 | 0.00 |  | 0.00 |  | 99.03 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1200000 | 1200000 | 1.00 | 3600000 | 3.00 | 30000 | 36494974 | 0.0 | 4.6 | false | 0 | 0 | 44.5 | 11352 / 259 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-spin-10000, loop 0 on CPU 3

4379396928 iterations, longest 44.5 µs; over 5 / 20 / 100 µs: 11352 / 259 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 257 | 6267.5 | 44.5 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 32.7 | 32.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 29.0 | 29.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 44.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 40.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 37.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 36.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 36.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 34.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 33.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 33.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 32.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 6 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-uring-spin-10000 (median 47.81)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 486.42 | 438.61 | 0.00 |  | 0.00 |  | 438.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 311.01 | 263.20 | 0.00 |  | 0.00 |  | 263.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 301.29 | 253.48 | 0.00 |  | 0.00 |  | 253.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 249.88 | 202.07 | 0.00 |  | 0.00 |  | 202.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 216.96 | 169.16 | 0.00 |  | 0.00 |  | 169.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 201.77 | 153.97 | 0.00 |  | 0.00 |  | 153.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 190.70 | 142.89 | 0.00 |  | 0.00 |  | 142.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 182.42 | 134.61 | 0.00 |  | 0.00 |  | 134.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 177.59 | 129.79 | 0.00 |  | 0.00 |  | 129.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 172.29 | 124.48 | 0.00 |  | 0.00 |  | 124.48 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1496935 | 1496935 | 1.00 | 1 | 0.00 | 0 | 42063405 | 0.0 | 0.0 | true | 0 | 0 | 26.5 | 256 / 4 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-0, loop 0 on CPU 3

2523804288 iterations, longest 26.5 µs; over 5 / 20 / 100 µs: 256 / 4 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 65.1 | 24.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.5 | 26.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 26.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 24.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 20.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 20.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 6 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-0 (median 39.15)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 577.71 | 538.56 | 0.00 |  | 0.00 |  | 538.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 470.96 | 431.80 | 0.00 |  | 0.00 |  | 431.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 340.22 | 301.06 | 0.00 |  | 0.00 |  | 301.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 314.70 | 275.55 | 0.00 |  | 0.00 |  | 275.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 301.06 | 261.90 | 0.00 |  | 0.00 |  | 261.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 275.90 | 236.74 | 0.00 |  | 0.00 |  | 236.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 237.14 | 197.99 | 0.00 |  | 0.00 |  | 197.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 236.84 | 197.69 | 0.00 |  | 0.00 |  | 197.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 207.71 | 168.56 | 0.00 |  | 0.00 |  | 168.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 205.89 | 166.74 | 0.00 |  | 0.00 |  | 166.74 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199999 | 1199999 | 1.00 | 1 | 0.00 | 0 | 26468997 | 0.0 | 0.0 | true | 0 | 0 | 192.7 | 552 / 4 / 1 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-10000, loop 0 on CPU 3

3176279680 iterations, longest 192.7 µs; over 5 / 20 / 100 µs: 552 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 192.7 | 192.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 53.8 | 29.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 22.8 | 22.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 192.7 | 0 | softirq TIMER | softirq TIMER 177, local timer tick 8, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 29.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 24.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 22.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1006 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 6 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-10000 (median 40.21)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 404.22 | 364.01 | 192.65 | softirq TIMER | 0.00 |  | 171.36 | client: softirq TIMER |
| 2 | 252.17 | 211.96 | 0.00 |  | 0.00 |  | 211.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 244.92 | 204.71 | 0.00 |  | 0.00 |  | 204.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 234.12 | 193.91 | 0.00 |  | 0.00 |  | 193.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 216.83 | 176.62 | 0.00 |  | 0.00 |  | 176.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 212.66 | 172.45 | 0.00 |  | 0.00 |  | 172.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 211.54 | 171.33 | 0.00 |  | 0.00 |  | 171.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 203.87 | 163.66 | 0.00 |  | 0.00 |  | 163.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 202.35 | 162.14 | 0.00 |  | 175.50 | softirq TIMER | 0.00 | server: softirq TIMER |
| 10 | 191.66 | 151.45 | 0.00 |  | 0.00 |  | 151.45 | neither loop stalled long enough: network, hypervisor or untraced |

## Ping-pong round trip (client send → echo decoded; "from when due": from the slot on the paced schedule, including any wait behind a stall)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189947 | 41.33 | 40.56 | 44.32 | 55.11 | 85.85 | 99.96 | 1001.93 |
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189947 | 41.49 | 40.70 | 44.46 | 55.32 | 86.26 | 114.58 | 1002.07 |
| r1-base-0/pp-epoll-spin-0: epoll-spin @ 0/s | 1271224 | 46.33 | 45.76 | 50.09 | 58.96 | 73.07 | 103.06 | 1016.26 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 47.24 | 46.20 | 51.07 | 85.99 | 93.70 | 106.32 | 26739.70 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 53.26 | 46.38 | 51.27 | 86.61 | 95.68 | 20757.64 | 26739.91 |
| r1-base-0/pp-recv-busy-0: recv-busy @ 0/s | 1456185 | 40.42 | 39.37 | 45.59 | 56.49 | 72.09 | 94.00 | 1047.97 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 41.23 | 40.39 | 45.21 | 54.35 | 69.08 | 101.04 | 235.25 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 41.52 | 40.68 | 45.50 | 54.67 | 69.74 | 110.56 | 825.28 |
| r1-base-0/pp-uring-spin-0: uring-spin @ 0/s | 1265493 | 45.95 | 44.27 | 51.33 | 71.60 | 97.64 | 107.05 | 379.34 |
| r1-base-0/pp-uring-spin-10000: uring-spin @ 10000/s | 1189998 | 47.26 | 45.02 | 51.47 | 91.91 | 98.82 | 116.55 | 277.46 |
| r1-base-0/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189998 | 47.38 | 45.12 | 51.61 | 92.03 | 99.44 | 127.97 | 680.91 |
| r1-base-0/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1419257 | 41.35 | 40.37 | 45.99 | 55.67 | 70.56 | 99.29 | 354.84 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189997 | 41.37 | 40.48 | 45.68 | 55.44 | 70.95 | 98.94 | 949.51 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189997 | 41.44 | 40.55 | 45.75 | 55.54 | 71.59 | 105.34 | 949.58 |
| r1-base-1/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189947 | 40.32 | 39.53 | 42.81 | 54.07 | 80.56 | 94.20 | 202010.47 |
| r1-base-1/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189947 | 322.85 | 39.68 | 43.02 | 58.42 | 130053.04 | 194869.97 | 202010.61 |
| r1-base-1/pp-epoll-spin-0: epoll-spin @ 0/s | 1299250 | 45.32 | 44.54 | 48.95 | 57.24 | 80.47 | 214.38 | 693.50 |
| r1-base-1/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 47.00 | 46.54 | 49.74 | 55.77 | 71.90 | 103.97 | 514.98 |
| r1-base-1/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 47.18 | 46.69 | 49.89 | 55.95 | 72.72 | 152.36 | 715.21 |
| r1-base-1/pp-recv-busy-0: recv-busy @ 0/s | 1522559 | 38.66 | 37.98 | 42.06 | 50.15 | 62.71 | 82.01 | 308.52 |
| r1-base-1/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 40.12 | 39.57 | 42.79 | 50.00 | 62.93 | 86.64 | 756.98 |
| r1-base-1/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 40.48 | 39.86 | 43.09 | 50.33 | 63.68 | 118.84 | 2312.28 |
| r1-base-1/pp-uring-spin-0: uring-spin @ 0/s | 1242864 | 46.78 | 46.47 | 50.05 | 59.21 | 72.65 | 97.59 | 394.26 |
| r1-base-1/pp-uring-spin-10000: uring-spin @ 10000/s | 1189998 | 48.82 | 47.81 | 53.70 | 60.73 | 75.78 | 130.41 | 486.42 |
| r1-base-1/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189998 | 48.90 | 47.87 | 53.77 | 60.89 | 77.29 | 134.82 | 871.34 |
| r1-base-1/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1472174 | 39.88 | 39.15 | 43.39 | 51.85 | 65.95 | 92.21 | 577.71 |
| r1-base-1/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189997 | 40.77 | 40.21 | 43.56 | 51.08 | 65.28 | 94.88 | 404.22 |
| r1-base-1/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189997 | 40.84 | 40.28 | 43.63 | 51.18 | 65.68 | 103.83 | 602.88 |

