# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|
| r1-base-0 | 111.82 | 31.5 | 1.0 | 0.5 | 111.82 µs stall: softirq TIMER |
| r1-base-1 | 71.15 | 33.5 | 1.0 | 0.0 | 71.15 µs stall: softirq TIMER |

## HTTP round trip, http-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189941 | 38.21 | 55.04 | 76.13 | 101.60 | 578.68 | 69.6 | 1.0 | 0.0 | 0.0 | 64.90 | 114.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189941 | 38.58 | 52.41 | 63.64 | 92.79 | 551.21 | 30.8 | 0.5 | 0.0 | 0.0 | 64.89 | 131.5 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189941 | 38.37 | 55.21 | 76.78 | 111.04 | 667.03 | 110.9 | 2.0 | 0.0 | 0.0 |
| r1-base-1 | 1189941 | 38.75 | 52.60 | 63.97 | 98.15 | 551.38 | 55.0 | 1.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 2900000 | 3.01 | 6.16 | 13.86 | 19.46 | 32.83 | 0.0 | 0.0 | 0.0 | 0.0 | 511.51 | 1299.0 | 511.51 µs stall: loop work (2650 items) |
| r1-base-1 | 2900000 | 2.98 | 7.36 | 13.77 | 20.20 | 40.59 | 0.0 | 0.0 | 0.0 | 0.0 | 513.07 | 2205.0 | 513.07 µs stall: loop work (2700 items) |

### Market-data burst, kernel arrival → decoded (every message), md-recv-busy, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 2900000 | 58.48 | 77.36 | 92.30 | 130.19 | 164.17 | 1631.4 | 0.0 | 0.0 | 0.0 |
| r1-base-1 | 2900000 | 46.35 | 81.45 | 90.12 | 110.68 | 1005.46 | 487.2 | 3.1 | 3.1 | 0.0 |

## WebSocket round trip, pp-epoll-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1303258 | 43.21 | 68.11 | 101.35 | 235.89 | 1018.10 | 1445.1 | 1.0 | 1.0 | 0.0 | 173.06 | 6.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1280756 | 45.30 | 68.08 | 102.04 | 158.68 | 229.61 | 4138.0 | 0.0 | 0.0 | 0.0 | 152.29 | 11.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-epoll-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 49.90 | 93.97 | 168.85 | 194.09 | 242.88 | 2921.3 | 0.0 | 0.0 | 0.0 | 26.84 | 1.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189999 | 43.38 | 89.02 | 97.91 | 146.21 | 210.48 | 428.1 | 0.0 | 0.0 | 0.0 | 24.26 | 1.0 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-epoll-spin-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 50.13 | 97.49 | 7196.97 | 16580.98 | 18832.10 | 5431.3 | 2382.9 | 1781.9 | 387.7 |
| r1-base-1 | 1189999 | 43.54 | 89.26 | 99.67 | 412.17 | 964.26 | 582.9 | 45.4 | 0.0 | 0.0 |

## WebSocket round trip, pp-mio-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1306166 | 43.66 | 61.11 | 110.19 | 175.23 | 207.32 | 2538.3 | 0.0 | 0.0 | 0.0 | 6182.45 | 84.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1245459 | 48.65 | 92.77 | 101.51 | 145.58 | 303.96 | 6102.7 | 0.0 | 0.0 | 0.0 | 6241.59 | 47.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-mio-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1190017 | 47.80 | 87.55 | 116.77 | 173.62 | 519.38 | 941.9 | 1.0 | 0.0 | 0.0 | 5893.74 | 5.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189998 | 45.64 | 90.12 | 98.73 | 150.40 | 225.49 | 458.8 | 0.0 | 0.0 | 0.0 | 5896.26 | 2.5 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-mio-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1190017 | 47.92 | 88.41 | 248.40 | 4098.03 | 6715.93 | 1576.1 | 283.9 | 160.8 | 0.0 |
| r1-base-1 | 1189998 | 45.75 | 90.34 | 100.25 | 2706.36 | 4543.18 | 624.7 | 116.5 | 91.8 | 0.0 |

## WebSocket round trip, pp-recv-busy-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1570193 | 36.33 | 51.79 | 67.27 | 96.20 | 901.80 | 95.6 | 1.0 | 0.0 | 0.0 | 33.37 | 15.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1534581 | 37.37 | 51.68 | 62.48 | 83.19 | 1016.27 | 53.9 | 1.0 | 1.0 | 0.0 | 29.90 | 18.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 38.79 | 53.96 | 67.86 | 89.18 | 961.34 | 23.7 | 1.0 | 0.0 | 0.0 | 39.32 | 13.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189999 | 38.29 | 50.50 | 65.16 | 94.52 | 639.70 | 39.3 | 0.5 | 0.0 | 0.0 | 34.86 | 32.5 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### WebSocket round trip, pp-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 39.11 | 54.30 | 68.40 | 93.05 | 961.70 | 41.3 | 8.1 | 0.0 | 0.0 |
| r1-base-1 | 1189999 | 38.58 | 50.81 | 65.76 | 98.36 | 640.12 | 53.9 | 1.5 | 0.0 | 0.0 |

## WebSocket round trip, pp-uring-sqpoll-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1474088 | 38.77 | 52.97 | 63.32 | 98.56 | 180.17 | 144.4 | 0.0 | 0.0 | 0.0 | 27.06 | 1.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1467703 | 38.72 | 55.23 | 69.51 | 97.49 | 165.16 | 119.0 | 0.0 | 0.0 | 0.0 | 33.21 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-sqpoll-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189997 | 38.92 | 53.55 | 83.36 | 112.22 | 233.04 | 150.8 | 0.0 | 0.0 | 0.0 | 306.86 | 1.5 | server: softirq TIMER |
| r1-base-1 | 1189998 | 39.56 | 53.53 | 64.47 | 93.46 | 1123.90 | 49.9 | 1.0 | 0.5 | 0.0 | 202.83 | 2.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### WebSocket round trip, pp-uring-sqpoll-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189997 | 38.99 | 53.63 | 85.05 | 128.22 | 320.46 | 244.5 | 0.0 | 0.0 | 0.0 |
| r1-base-1 | 1189998 | 39.66 | 53.67 | 65.17 | 110.47 | 1123.96 | 75.1 | 11.1 | 1.5 | 0.0 |

## WebSocket round trip, pp-uring-sqpoll-sib-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1492556 | 38.34 | 52.59 | 66.14 | 94.06 | 280.82 | 72.2 | 0.0 | 0.0 | 0.0 | 26.81 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1493050 | 38.53 | 51.46 | 63.01 | 95.00 | 431.72 | 99.7 | 0.0 | 0.0 | 0.0 | 32.64 | 3.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

## WebSocket round trip, pp-uring-sqpoll-sib-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 40.67 | 56.67 | 74.04 | 102.57 | 913.07 | 72.6 | 2.0 | 0.0 | 0.0 | 164.61 | 1.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189999 | 40.81 | 55.94 | 68.09 | 95.59 | 699.87 | 47.4 | 0.5 | 0.0 | 0.0 | 258.39 | 2.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### WebSocket round trip, pp-uring-sqpoll-sib-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 40.76 | 56.79 | 75.28 | 145.35 | 1768.84 | 139.7 | 37.3 | 16.6 | 0.0 |
| r1-base-1 | 1189999 | 40.90 | 56.04 | 68.57 | 103.44 | 699.97 | 69.6 | 2.5 | 0.0 | 0.0 |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs, and system-management interrupts (SMI, bare metal only: firmware pauses). The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | SMI | receiving CPU | LOC | IWI | RES | CAL | SMI |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| http-recv-busy-10000 | r1-base-0 | 2 | 23 | 23 | 0 | 8 | 0 |  |  |  |  |  |  |
| http-recv-busy-10000 | r1-base-1 | 2 | 10 | 10 | 0 | 8 | 0 |  |  |  |  |  |  |
| md-recv-busy | r1-base-0 | 2 | 12 | 12 | 0 | 17 | 0 |  |  |  |  |  |  |
| md-recv-busy | r1-base-1 | 2 | 12 | 12 | 0 | 17 | 0 |  |  |  |  |  |  |
| noise | r1-base-0 | 2 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-base-1 | 2 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-epoll-spin-0 | r1-base-0 | 2 | 0 | 0 | 0 | 16 | 0 | 4 | 1008 | 0 | 1 | 16 | 0 |
| pp-epoll-spin-0 | r1-base-1 | 2 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 1 | 13 | 0 |
| pp-epoll-spin-10000 | r1-base-0 | 2 | 0 | 0 | 0 | 8 | 0 | 0 | 1072 | 0 | 1 | 8 | 0 |
| pp-epoll-spin-10000 | r1-base-1 | 2 | 0 | 0 | 0 | 8 | 0 | 0 | 1072 | 0 | 1 | 8 | 0 |
| pp-mio-0 | r1-base-0 | 2 | 0 | 0 | 0 | 16 | 0 |  |  |  |  |  |  |
| pp-mio-0 | r1-base-1 | 2 | 0 | 0 | 0 | 16 | 0 |  |  |  |  |  |  |
| pp-mio-10000 | r1-base-0 | 2 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-mio-10000 | r1-base-1 | 2 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-recv-busy-0 | r1-base-0 | 2 | 41 | 40 | 0 | 16 | 0 |  |  |  |  |  |  |
| pp-recv-busy-0 | r1-base-1 | 2 | 37 | 37 | 0 | 16 | 0 | 4 | 997 | 0 | 0 | 3 | 0 |
| pp-recv-busy-10000 | r1-base-0 | 2 | 22 | 21 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-recv-busy-10000 | r1-base-1 | 2 | 50 | 50 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-uring-sqpoll-0 | r1-base-0 | 2 | 0 | 0 | 0 | 17 | 0 | 1 | 89 | 42 | 0 | 17 | 0 |
| pp-uring-sqpoll-0 | r1-base-1 | 2 | 0 | 0 | 0 | 17 | 0 | 1 | 87 | 41 | 0 | 17 | 0 |
| pp-uring-sqpoll-10000 | r1-base-0 | 2 | 0 | 0 | 0 | 8 | 0 | 1 | 42 | 19 | 1 | 8 | 0 |
| pp-uring-sqpoll-10000 | r1-base-1 | 2 | 0 | 0 | 0 | 8 | 0 | 1 | 53 | 24 | 1 | 8 | 0 |
| pp-uring-sqpoll-sib-0 | r1-base-0 | 2 | 0 | 0 | 0 | 17 | 0 | 6 | 116 | 55 | 0 | 17 | 0 |
| pp-uring-sqpoll-sib-0 | r1-base-1 | 2 | 0 | 0 | 0 | 17 | 0 | 6 | 110 | 52 | 1 | 17 | 0 |
| pp-uring-sqpoll-sib-10000 | r1-base-0 | 2 | 0 | 0 | 0 | 8 | 0 | 6 | 78 | 37 | 1 | 8 | 0 |
| pp-uring-sqpoll-sib-10000 | r1-base-1 | 2 | 0 | 0 | 0 | 8 | 0 | 6 | 70 | 29 | 1 | 8 | 0 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### http-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 578.68 at 117.5 s: neither loop stalled long enough: network, hypervisor or untraced; 503.20 at 117.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 464.85 at 57.5 s: neither loop stalled long enough: network, hypervisor or untraced; 241.47 at 57.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 229.59 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 551.21 at 44.5 s: neither loop stalled long enough: network, hypervisor or untraced; 249.40 at 104.5 s: neither loop stalled long enough: network, hypervisor or untraced; 216.59 at 44.5 s: neither loop stalled long enough: network, hypervisor or untraced; 200.38 at 44.5 s: neither loop stalled long enough: network, hypervisor or untraced; 198.80 at 12.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### md-recv-busy

| condition | worst events |
|---|---|
| r1-base-0 | 32.83 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced; 32.70 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced; 32.56 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced; 32.43 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced; 32.29 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 40.59 at 35.3 s: client: loop work (3 items); 40.45 at 35.3 s: client: loop work (3 items); 40.27 at 35.3 s: client: loop work (3 items); 40.15 at 52.7 s: client: loop work (20 items); 40.00 at 52.7 s: client: loop work (20 items) |

### noise

| condition | worst events |
|---|---|
| r1-base-0 | 111.82 at 89.2 s: softirq TIMER; 21.96 at 53.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.52 at 5.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 15.64 at 114.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 13.22 at 56.1 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-1 | 71.15 at 82.0 s: softirq TIMER; 23.51 at 28.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.32 at 7.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 14.65 at 88.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 13.74 at 0.0 s: local timer tick |

### pp-epoll-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 1018.10 at 11.5 s: neither loop stalled long enough: network, hypervisor or untraced; 398.13 at 26.9 s: neither loop stalled long enough: network, hypervisor or untraced; 383.08 at 44.3 s: neither loop stalled long enough: network, hypervisor or untraced; 372.10 at 26.9 s: neither loop stalled long enough: network, hypervisor or untraced; 357.43 at 32.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 229.61 at 41.6 s: neither loop stalled long enough: network, hypervisor or untraced; 207.75 at 41.6 s: neither loop stalled long enough: network, hypervisor or untraced; 195.52 at 50.2 s: neither loop stalled long enough: network, hypervisor or untraced; 193.99 at 21.8 s: server: softirq TIMER; 193.58 at 5.4 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-epoll-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 242.88 at 46.2 s: neither loop stalled long enough: network, hypervisor or untraced; 229.26 at 22.8 s: neither loop stalled long enough: network, hypervisor or untraced; 225.77 at 12.7 s: neither loop stalled long enough: network, hypervisor or untraced; 224.58 at 53.5 s: neither loop stalled long enough: network, hypervisor or untraced; 222.36 at 99.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 210.48 at 51.5 s: neither loop stalled long enough: network, hypervisor or untraced; 203.55 at 51.5 s: neither loop stalled long enough: network, hypervisor or untraced; 202.11 at 51.5 s: neither loop stalled long enough: network, hypervisor or untraced; 194.49 at 28.1 s: neither loop stalled long enough: network, hypervisor or untraced; 193.40 at 35.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-mio-0

| condition | worst events |
|---|---|
| r1-base-0 | 207.32 at 37.3 s: neither loop stalled long enough: network, hypervisor or untraced; 203.59 at 6.9 s: neither loop stalled long enough: network, hypervisor or untraced; 202.39 at 47.4 s: neither loop stalled long enough: network, hypervisor or untraced; 201.72 at 54.3 s: neither loop stalled long enough: network, hypervisor or untraced; 196.21 at 47.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 303.96 at 14.6 s: neither loop stalled long enough: network, hypervisor or untraced; 300.40 at 14.6 s: neither loop stalled long enough: network, hypervisor or untraced; 275.56 at 14.6 s: neither loop stalled long enough: network, hypervisor or untraced; 246.61 at 14.6 s: neither loop stalled long enough: network, hypervisor or untraced; 229.09 at 14.6 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |

### pp-mio-10000

| condition | worst events |
|---|---|
| r1-base-0 | 519.38 at 117.6 s: neither loop stalled long enough: network, hypervisor or untraced; 505.95 at 57.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 453.09 at 57.6 s: neither loop stalled long enough: network, hypervisor or untraced; 318.50 at 57.6 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 300.00 at 57.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 225.49 at 23.8 s: neither loop stalled long enough: network, hypervisor or untraced; 210.08 at 50.3 s: neither loop stalled long enough: network, hypervisor or untraced; 197.50 at 50.3 s: neither loop stalled long enough: network, hypervisor or untraced; 196.23 at 50.3 s: neither loop stalled long enough: network, hypervisor or untraced; 193.88 at 50.3 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-0

| condition | worst events |
|---|---|
| r1-base-0 | 901.80 at 15.7 s: neither loop stalled long enough: network, hypervisor or untraced; 196.44 at 41.4 s: neither loop stalled long enough: network, hypervisor or untraced; 141.30 at 17.1 s: neither loop stalled long enough: network, hypervisor or untraced; 141.03 at 5.4 s: neither loop stalled long enough: network, hypervisor or untraced; 132.96 at 32.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1016.27 at 35.1 s: neither loop stalled long enough: network, hypervisor or untraced; 468.93 at 52.6 s: neither loop stalled long enough: network, hypervisor or untraced; 464.14 at 52.6 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 380.61 at 52.6 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 319.71 at 52.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 961.34 at 49.9 s: neither loop stalled long enough: network, hypervisor or untraced; 959.44 at 11.3 s: neither loop stalled long enough: network, hypervisor or untraced; 157.39 at 10.9 s: neither loop stalled long enough: network, hypervisor or untraced; 155.40 at 9.9 s: neither loop stalled long enough: network, hypervisor or untraced; 154.13 at 28.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 639.70 at 85.7 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 437.32 at 25.7 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 306.76 at 85.7 s: neither loop stalled long enough: network, hypervisor or untraced; 260.44 at 25.7 s: neither loop stalled long enough: network, hypervisor or untraced; 195.80 at 25.7 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |

### pp-uring-sqpoll-0

| condition | worst events |
|---|---|
| r1-base-0 | 180.17 at 40.3 s: neither loop stalled long enough: network, hypervisor or untraced; 175.03 at 43.3 s: neither loop stalled long enough: network, hypervisor or untraced; 150.00 at 26.2 s: neither loop stalled long enough: network, hypervisor or untraced; 145.61 at 36.3 s: neither loop stalled long enough: network, hypervisor or untraced; 143.95 at 38.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 165.16 at 44.1 s: neither loop stalled long enough: network, hypervisor or untraced; 155.76 at 47.7 s: neither loop stalled long enough: network, hypervisor or untraced; 151.03 at 32.7 s: neither loop stalled long enough: network, hypervisor or untraced; 148.37 at 55.4 s: neither loop stalled long enough: network, hypervisor or untraced; 145.29 at 5.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-10000

| condition | worst events |
|---|---|
| r1-base-0 | 233.04 at 25.8 s: server: softirq TIMER; 188.29 at 43.5 s: neither loop stalled long enough: network, hypervisor or untraced; 161.11 at 53.7 s: neither loop stalled long enough: network, hypervisor or untraced; 154.95 at 63.9 s: neither loop stalled long enough: network, hypervisor or untraced; 152.66 at 2.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1123.90 at 22.0 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 864.18 at 82.0 s: neither loop stalled long enough: network, hypervisor or untraced; 409.15 at 22.0 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 293.54 at 82.0 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 263.41 at 26.4 s: client: softirq TIMER |

### pp-uring-sqpoll-sib-0

| condition | worst events |
|---|---|
| r1-base-0 | 280.82 at 33.8 s: neither loop stalled long enough: network, hypervisor or untraced; 192.51 at 33.8 s: neither loop stalled long enough: network, hypervisor or untraced; 159.93 at 33.8 s: neither loop stalled long enough: network, hypervisor or untraced; 148.48 at 33.8 s: neither loop stalled long enough: network, hypervisor or untraced; 146.25 at 12.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 431.72 at 22.0 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 331.40 at 22.0 s: neither loop stalled long enough: network, hypervisor or untraced; 315.04 at 22.0 s: neither loop stalled long enough: network, hypervisor or untraced; 302.50 at 22.0 s: neither loop stalled long enough: network, hypervisor or untraced; 278.32 at 22.0 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |

### pp-uring-sqpoll-sib-10000

| condition | worst events |
|---|---|
| r1-base-0 | 913.07 at 68.2 s: neither loop stalled long enough: network, hypervisor or untraced; 578.67 at 8.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 565.74 at 68.2 s: neither loop stalled long enough: network, hypervisor or untraced; 527.75 at 8.2 s: neither loop stalled long enough: network, hypervisor or untraced; 388.39 at 68.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 699.87 at 22.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 401.66 at 82.2 s: neither loop stalled long enough: network, hypervisor or untraced; 302.10 at 82.2 s: neither loop stalled long enough: network, hypervisor or untraced; 225.33 at 117.3 s: server: softirq TIMER; 164.95 at 112.5 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| r1-base-0 | loop work 29485.63; mostly unexplained (+irq work (IPI/vector)) 12551.59; unexplained: no kernel event (hypervisor or in-kernel time) 5150.27; mostly unexplained (+preempted by kworker/2:1) 1901.52; mostly unexplained (+softirq NET_RX) 987.12 |
| r1-base-1 | loop work 52839.41; mostly unexplained (+irq work (IPI/vector)) 12584.72; unexplained: no kernel event (hypervisor or in-kernel time) 4305.42; mostly unexplained (+preempted by kworker/2:1) 2122.54; mostly unexplained (+local timer tick) 1257.85 |

## Once-a-minute stalls

Whether each source's events of 300 µs or more (loop stalls, or samples that slow) bunch at one second of the wall-clock minute. The busiest second is the middle of the busiest three; three seconds hold one event in twenty by chance, so a share of 25% or more over at least five events is marked periodic. Each host has its own second (its agents' timers start at boot).

| run | source | events | busiest second | events there (share) | minutes with one there | longest there (µs) | periodic |
|---|---|---|---|---|---|---|---|
| cpumap-receiver/cpu1 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu2 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu3 | counter-only spinner | 1 | :46 | 1 (100%) | 1 of 1 | 857.22 | no |
| cpumap-receiver/cpu5 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu6 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu7 | counter-only spinner | 1 | :46 | 1 (100%) | 1 of 1 | 851.66 | no |
| cpumap-sender/cpu1 | counter-only spinner | 1 | :49 | 1 (100%) | 1 of 1 | 829.19 | no |
| cpumap-sender/cpu3 | counter-only spinner | 1 | :49 | 1 (100%) | 1 of 1 | 834.31 | no |
| http-recv-busy-10000 | idle spare core | 0 | | | | | |
| http-recv-busy-10000 | receiver loop | 0 | | | | | |
| http-recv-busy-10000 | sender loop | 0 | | | | | |
| http-recv-busy-10000 | slow samples | 4 | :22 | 4 (100%) | 3 of 4 | 578.68 | no |
| md-recv-busy | idle spare core | 0 | | | | | |
| md-recv-busy | receiver loop | 0 | | | | | |
| md-recv-busy | slow samples | 0 | | | | | |
| noise | noise probe (loop CPU) | 0 | | | | | |
| pp-epoll-spin-0 | idle spare core | 0 | | | | | |
| pp-epoll-spin-0 | receiver loop | 0 | | | | | |
| pp-epoll-spin-0 | sender loop | 0 | | | | | |
| pp-epoll-spin-0 | slow samples | 21 | :06 | 7 (33%) | 1 of 2 | 1018.10 | **yes** |
| pp-epoll-spin-10000 | idle spare core | 0 | | | | | |
| pp-epoll-spin-10000 | receiver loop | 0 | | | | | |
| pp-epoll-spin-10000 | sender loop | 0 | | | | | |
| pp-epoll-spin-10000 | slow samples | 0 | | | | | |
| pp-mio-0 | idle spare core | 0 | | | | | |
| pp-mio-0 | receiver loop | 0 | | | | | |
| pp-mio-0 | sender loop | 0 | | | | | |
| pp-mio-0 | slow samples | 2 | :22 | 2 (100%) | 1 of 2 | 303.96 | no |
| pp-mio-10000 | idle spare core | 0 | | | | | |
| pp-mio-10000 | receiver loop | 0 | | | | | |
| pp-mio-10000 | sender loop | 0 | | | | | |
| pp-mio-10000 | slow samples | 4 | :22 | 4 (100%) | 2 of 4 | 519.38 | no |
| pp-recv-busy-0 | idle spare core | 0 | | | | | |
| pp-recv-busy-0 | receiver loop | 0 | | | | | |
| pp-recv-busy-0 | sender loop | 0 | | | | | |
| pp-recv-busy-0 | slow samples | 6 | :22 | 4 (67%) | 1 of 2 | 468.93 | no |
| pp-recv-busy-10000 | idle spare core | 0 | | | | | |
| pp-recv-busy-10000 | receiver loop | 0 | | | | | |
| pp-recv-busy-10000 | sender loop | 2 | :22 | 2 (100%) | 2 of 4 | 439.80 | no |
| pp-recv-busy-10000 | slow samples | 5 | :22 | 4 (80%) | 3 of 4 | 961.34 | no |
| pp-uring-sqpoll-0 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-0 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-0 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-0 | slow samples | 0 | | | | | |
| pp-uring-sqpoll-10000 | idle spare core | 1 | :26 | 1 (100%) | 1 of 4 | 376.25 | no |
| pp-uring-sqpoll-10000 | receiver loop | 1 | :08 | 1 (100%) | 1 of 4 | 306.86 | no |
| pp-uring-sqpoll-10000 | sender loop | 2 | :22 | 2 (100%) | 2 of 4 | 794.47 | no |
| pp-uring-sqpoll-10000 | slow samples | 3 | :22 | 3 (100%) | 2 of 4 | 1123.90 | no |
| pp-uring-sqpoll-sib-0 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-sib-0 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-sib-0 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-sib-0 | slow samples | 4 | :22 | 4 (100%) | 1 of 2 | 431.72 | no |
| pp-uring-sqpoll-sib-10000 | idle spare core | 1 | :35 | 1 (100%) | 1 of 4 | 389.77 | no |
| pp-uring-sqpoll-sib-10000 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-sib-10000 | sender loop | 3 | :22 | 3 (100%) | 3 of 4 | 437.06 | no |
| pp-uring-sqpoll-sib-10000 | slow samples | 11 | :22 | 11 (100%) | 4 of 4 | 913.07 | **yes** |

# Every run in detail

#### Stalls: r1-base-0/http-recv-busy-10000, loop 0 on CPU 2

2117013273 iterations, longest 64.9 µs; over 5 / 20 / 100 µs: 88845 / 228 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 90 | 2089.8 | 28.7 |
| mostly unexplained (+preempted by kworker/2:1) | 69 | 1666.4 | 41.9 |
| mostly unexplained (+softirq NET_RX) | 38 | 941.0 | 43.2 |
| mostly unexplained (+irq work (IPI/vector)) | 17 | 379.7 | 26.3 |
| mostly unexplained (+local timer tick) | 7 | 193.3 | 36.6 |
| softirq NET_RX | 3 | 78.3 | 34.0 |
| softirq TIMER | 1 | 64.9 | 64.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 48.6 | 25.8 |
| preempted by kworker/2:1 | 1 | 31.1 | 31.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 64.9 | 1 | softirq TIMER | softirq TIMER 50, local timer tick 6, timer tick_nohz_handler 3 |
| 2 | 43.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by ksoftirqd/2 5, local timer tick 3, irq work (IPI/vector) 1 |
| 3 | 41.9 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 11, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 4 | 38.4 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 8, local timer tick 5, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 5 | 38.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 37.2 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 7, local timer tick 3, irq work (IPI/vector) 1, softirq RCU 1 |
| 7 | 36.6 | 1 | mostly unexplained (+local timer tick) | local timer tick 4, timer tick_nohz_handler 1 |
| 8 | 35.9 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 9 | 34.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 34.0 | 1 | softirq NET_RX | softirq NET_RX 18, preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2774 |
| IWI IRQ work interrupts | 2773 |
| CAL Function call interrupts | 998 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/http-recv-busy-10000 (median 38.21)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 578.68 | 540.47 | 0.00 |  | 219.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 321.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 503.20 | 464.99 | 0.00 |  | 280.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 184.27 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 464.85 | 426.63 | 0.00 |  | 208.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 217.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 241.47 | 203.26 | 0.00 |  | 137.31 | unexplained: no kernel event (hypervisor or in-kernel time) | 65.95 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 229.59 | 191.38 | 0.00 |  | 0.00 |  | 191.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 199.08 | 160.87 | 0.00 |  | 0.00 |  | 160.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 185.89 | 147.68 | 0.00 |  | 0.00 |  | 147.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 161.31 | 123.10 | 0.00 |  | 0.00 |  | 123.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 159.81 | 121.59 | 0.00 |  | 155.98 | softirq TIMER | 0.00 | server: softirq TIMER |
| 10 | 157.52 | 119.31 | 0.00 |  | 117.54 | unexplained: no kernel event (hypervisor or in-kernel time) | 1.77 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

## r1-base-0/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=404764 clocksource=tsc

### Loop counters (r1-base-0/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002650 | 449103 | 6.69 | 113791744 | 37.90 | 1896529 | 1896529 | 99.6 | 65.6 | true | 0 | 691 | 511.5 | 93948 / 1299 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672593600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (52315 of 2900000 in 1-msg reads) | 52315 | 2.53 | 2.48 | 2.93 | 4.32 | 9.28 | 18.33 | 31.29 |

#### Stalls: r1-base-0/md-recv-busy, loop 0 on CPU 2

113791744 iterations, longest 511.5 µs; over 5 / 20 / 100 µs: 93948 / 1299 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1290 | 29485.6 | 511.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 60.5 | 30.5 |
| mostly unexplained (+preempted by kworker/2:1) | 2 | 54.5 | 27.8 |
| softirq NET_RX | 2 | 42.7 | 22.6 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 30.1 | 30.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.7 | 26.7 |
| softirq TIMER | 1 | 23.9 | 23.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 511.5 | 2650 | loop work (2650 items) |  |
| 2 | 43.2 | 16 | loop work (16 items) | preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 38.2 | 18 | loop work (18 items) |  |
| 4 | 38.0 | 10 | loop work (10 items) | softirq NET_RX 16, preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 37.6 | 30 | loop work (30 items) | preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 37.4 | 10 | loop work (10 items) | softirq NET_RX 12, preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 37.2 | 20 | loop work (20 items) | preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 36.7 | 13 | loop work (13 items) | preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 36.6 | 15 | loop work (15 items) | softirq NET_RX 13, preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 36.2 | 10 | loop work (10 items) | preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 702 |
| LOC Local timer interrupts | 702 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-0/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-0/noise, loop 0 on CPU 2

9189380096 iterations, longest 111.8 µs; over 5 / 20 / 100 µs: 63 / 2 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 61 | 501.9 | 22.0 |
| softirq TIMER | 1 | 111.8 | 111.8 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 6.0 | 6.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 111.8 | 0 | softirq TIMER | softirq TIMER 95, local timer tick 8, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 22.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 17.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 15.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 13.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 11.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 9.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 9.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 8.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 8.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 995 |
| IWI IRQ work interrupts | 1 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (r1-base-0/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1325613 | 1325613 | 1.00 | 284510043 | 214.63 | 4741834 | 4697647 | 0.0 | 40.5 | false | 0 | 0 | 173.1 | 33875 / 6 / 1 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-0, loop 0 on CPU 2

281858816 iterations, longest 173.1 µs; over 5 / 20 / 100 µs: 33875 / 6 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 173.1 | 173.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 90.7 | 28.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 27.9 | 27.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 173.1 | 0 | softirq TIMER | softirq TIMER 148, local timer tick 12, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 28.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 27.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 21.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 20.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 20.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 988 |
| LOC Local timer interrupts | 13 |
| IWI IRQ work interrupts | 6 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-0 (median 43.21)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1018.10 | 974.89 | 0.00 |  | 0.00 |  | 974.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 398.13 | 354.92 | 0.00 |  | 0.00 |  | 354.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 383.08 | 339.87 | 0.00 |  | 0.00 |  | 339.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 372.10 | 328.88 | 0.00 |  | 131.41 | mostly unexplained (+local timer tick) | 197.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 357.43 | 314.22 | 0.00 |  | 0.00 |  | 314.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 351.61 | 308.40 | 0.00 |  | 0.00 |  | 308.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 351.41 | 308.20 | 0.00 |  | 0.00 |  | 308.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 340.96 | 297.74 | 0.00 |  | 0.00 |  | 297.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 339.07 | 295.86 | 0.00 |  | 0.00 |  | 295.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 339.00 | 295.78 | 0.00 |  | 0.00 |  | 295.78 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 565642816 | 471.37 | 4713690 | 4693690 | 0.0 | 36.8 | false | 0 | 0 | 26.8 | 8613 / 3 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-10000, loop 0 on CPU 2

563242816 iterations, longest 26.8 µs; over 5 / 20 / 100 µs: 8613 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 47.4 | 26.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 20.4 | 20.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 26.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 20.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 20.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |
| IWI IRQ work interrupts | 5 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-10000 (median 49.90)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 242.88 | 192.98 | 0.00 |  | 0.00 |  | 192.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 229.26 | 179.36 | 0.00 |  | 0.00 |  | 179.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 225.77 | 175.87 | 0.00 |  | 0.00 |  | 175.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 224.58 | 174.68 | 0.00 |  | 0.00 |  | 174.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 222.36 | 172.46 | 0.00 |  | 0.00 |  | 172.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 221.25 | 171.35 | 0.00 |  | 0.00 |  | 171.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 217.78 | 167.88 | 0.00 |  | 0.00 |  | 167.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 215.90 | 166.00 | 0.00 |  | 0.00 |  | 166.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 215.80 | 165.90 | 0.00 |  | 0.00 |  | 165.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 215.21 | 165.31 | 0.00 |  | 0.00 |  | 165.31 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-mio-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 0 | 0 | 0.00 | 255916811 | 0.00 | 4265280 | 4220989 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

#### Stalls: r1-base-0/pp-mio-0, loop 0 on CPU 2

253259335 iterations, longest 6182.4 µs; over 5 / 20 / 100 µs: 531943 / 84 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+irq work (IPI/vector)) | 1 | 6182.4 | 6182.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 82 | 1819.0 | 51.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.3 | 26.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 6182.4 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 51.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 37.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 35.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 29.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 28.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 26.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 26.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 26.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 25.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 988 |
| LOC Local timer interrupts | 12 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-mio-0 (median 43.66)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 207.32 | 163.66 | 0.00 |  | 0.00 |  | 163.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 203.59 | 159.93 | 0.00 |  | 0.00 |  | 159.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 202.39 | 158.73 | 0.00 |  | 0.00 |  | 158.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 201.72 | 158.06 | 0.00 |  | 0.00 |  | 158.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 196.21 | 152.55 | 0.00 |  | 0.00 |  | 152.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 195.34 | 151.68 | 0.00 |  | 0.00 |  | 151.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 194.81 | 151.16 | 0.00 |  | 0.00 |  | 151.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 194.37 | 150.71 | 0.00 |  | 0.00 |  | 150.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 194.09 | 150.43 | 0.00 |  | 0.00 |  | 150.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 192.83 | 149.17 | 0.00 |  | 0.00 |  | 149.17 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-mio-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 0 | 0 | 0.00 | 511563714 | 0.00 | 4263031 | 4243031 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

#### Stalls: r1-base-0/pp-mio-10000, loop 0 on CPU 2

509163714 iterations, longest 5893.7 µs; over 5 / 20 / 100 µs: 29142 / 10 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+irq work (IPI/vector)) | 1 | 5893.7 | 5893.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 226.1 | 45.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 27.3 | 27.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 5893.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 45.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 37.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 28.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 27.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 25.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 24.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 22.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 21.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| IWI IRQ work interrupts | 6 |

#### Slowest round trips: r1-base-0/pp-mio-10000 (median 47.80)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 519.38 | 471.58 | 0.00 |  | 130.03 | unexplained: no kernel event (hypervisor or in-kernel time) | 341.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 505.95 | 458.15 | 0.00 |  | 297.65 | unexplained: no kernel event (hypervisor or in-kernel time) | 160.50 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 453.09 | 405.29 | 0.00 |  | 133.36 | unexplained: no kernel event (hypervisor or in-kernel time) | 271.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 318.50 | 270.70 | 0.00 |  | 234.65 | unexplained: no kernel event (hypervisor or in-kernel time) | 36.05 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 300.00 | 252.20 | 0.00 |  | 0.00 |  | 252.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 286.93 | 239.12 | 0.00 |  | 129.96 | unexplained: no kernel event (hypervisor or in-kernel time) | 109.17 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 275.48 | 227.67 | 0.00 |  | 0.00 |  | 227.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 253.07 | 205.26 | 0.00 |  | 0.00 |  | 205.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 252.45 | 204.65 | 0.00 |  | 159.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 45.45 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 246.42 | 198.62 | 0.00 |  | 52.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 145.80 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1597430 | 1597430 | 1.00 | 107521527 | 67.31 | 1792025 | 1765402 | 98.5 | 71.8 | true | 0 | 2403 | 33.4 | 46170 / 15 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-0, loop 0 on CPU 2

105924096 iterations, longest 33.4 µs; over 5 / 20 / 100 µs: 46170 / 15 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 149.6 | 33.4 |
| mostly unexplained (+preempted by kworker/2:1) | 3 | 60.6 | 20.4 |
| softirq NET_RX | 2 | 48.4 | 27.8 |
| mostly unexplained (+local timer tick) | 2 | 43.4 | 23.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.9 | 28.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 26.3 | 26.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 33.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 28.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 28.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 27.8 | 1 | softirq NET_RX | softirq NET_RX 17, preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 26.3 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 23.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 23.0 | 1 | mostly unexplained (+local timer tick) | local timer tick 3, preempted by kworker/2:1 2, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 8 | 22.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 22.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 20.5 | 1 | softirq NET_RX | softirq NET_RX 11, preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2435 |
| IWI IRQ work interrupts | 2408 |
| CAL Function call interrupts | 987 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-recv-busy-0 (median 36.33)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 901.80 | 865.47 | 0.00 |  | 0.00 |  | 865.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 196.44 | 160.11 | 0.00 |  | 0.00 |  | 160.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 141.30 | 104.97 | 0.00 |  | 0.00 |  | 104.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 141.03 | 104.70 | 0.00 |  | 0.00 |  | 104.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 132.96 | 96.64 | 0.00 |  | 0.00 |  | 96.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 132.10 | 95.77 | 0.00 |  | 0.00 |  | 95.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 127.89 | 91.56 | 0.00 |  | 0.00 |  | 91.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 124.70 | 88.37 | 0.00 |  | 0.00 |  | 88.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 123.69 | 87.36 | 0.00 |  | 0.00 |  | 87.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 123.61 | 87.28 | 0.00 |  | 0.00 |  | 87.28 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 231731392 | 193.11 | 1931095 | 1921095 | 99.5 | 68.4 | true | 0 | 2548 | 39.3 | 47217 / 26 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-10000, loop 0 on CPU 2

230531392 iterations, longest 39.3 µs; over 5 / 20 / 100 µs: 47217 / 26 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 166.6 | 22.2 |
| mostly unexplained (+preempted by kworker/2:1) | 5 | 120.0 | 31.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 70.9 | 26.9 |
| softirq NET_RX | 3 | 67.4 | 23.7 |
| mostly unexplained (+local timer tick) | 3 | 65.5 | 23.8 |
| mostly unexplained (+softirq NET_RX) | 2 | 46.1 | 23.4 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 39.3 | 39.3 |
| mostly unexplained (+softirq TIMER) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 39.3 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, softirq NET_RX 1, ipi_send_cpu 0 |
| 2 | 31.8 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 26.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 23.9 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 23.8 | 1 | mostly unexplained (+local timer tick) | local timer tick 2, softirq RCU 2, timer tick_nohz_handler 1 |
| 6 | 23.7 | 1 | softirq NET_RX | softirq NET_RX 12, local timer tick 4, preempted by kworker/2:1 4, irq work (IPI/vector) 1 |
| 7 | 23.6 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 23.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 9, preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 23.3 | 1 | softirq NET_RX | softirq NET_RX 12, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 22.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 11, preempted by kworker/2:1 5, local timer tick 3, irq work (IPI/vector) 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2597 |
| IWI IRQ work interrupts | 2556 |
| CAL Function call interrupts | 996 |

#### Slowest round trips: r1-base-0/pp-recv-busy-10000 (median 38.79)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 961.34 | 922.55 | 0.00 |  | 0.00 |  | 922.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 959.44 | 920.65 | 0.00 |  | 0.00 |  | 920.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 157.39 | 118.60 | 0.00 |  | 0.00 |  | 118.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 155.40 | 116.61 | 0.00 |  | 0.00 |  | 116.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 154.13 | 115.34 | 0.00 |  | 0.00 |  | 115.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 136.78 | 98.00 | 0.00 |  | 0.00 |  | 98.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 136.07 | 97.28 | 0.00 |  | 0.00 |  | 97.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 135.61 | 96.82 | 0.00 |  | 0.00 |  | 96.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 134.95 | 96.16 | 0.00 |  | 0.00 |  | 96.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 128.81 | 90.02 | 0.00 |  | 0.00 |  | 90.02 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1499042 | 1499042 | 1.00 | 1 | 0.00 | 0 | 31466553 | 0.0 | 0.0 | true | 0 | 0 | 27.1 | 88 / 1 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-0, loop 0 on CPU 2

1887993152 iterations, longest 27.1 µs; over 5 / 20 / 100 µs: 88 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 27.1 | 27.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 27.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-0 (median 38.77)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 180.17 | 141.40 | 0.00 |  | 0.00 |  | 141.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 175.03 | 136.26 | 0.00 |  | 0.00 |  | 136.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 150.00 | 111.22 | 0.00 |  | 0.00 |  | 111.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 145.61 | 106.84 | 0.00 |  | 0.00 |  | 106.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 143.95 | 105.18 | 0.00 |  | 0.00 |  | 105.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 141.97 | 103.19 | 0.00 |  | 0.00 |  | 103.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 141.46 | 102.69 | 0.00 |  | 0.00 |  | 102.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 140.79 | 102.02 | 0.00 |  | 0.00 |  | 102.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 139.47 | 100.70 | 0.00 |  | 0.00 |  | 100.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 137.35 | 98.58 | 0.00 |  | 0.00 |  | 98.58 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 27094805 | 0.0 | 0.0 | true | 0 | 0 | 306.9 | 147 / 3 / 1 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-10000, loop 0 on CPU 2

3251376576 iterations, longest 306.9 µs; over 5 / 20 / 100 µs: 147 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 306.9 | 306.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 47.6 | 24.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 306.9 | 0 | softirq TIMER | softirq TIMER 285, local timer tick 11, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 24.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 23.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-10000 (median 38.92)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 233.04 | 194.11 | 0.00 |  | 206.09 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 188.29 | 149.37 | 0.00 |  | 0.00 |  | 149.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 161.11 | 122.19 | 0.00 |  | 0.00 |  | 122.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 154.95 | 116.02 | 0.00 |  | 0.00 |  | 116.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 152.66 | 113.74 | 0.00 |  | 0.00 |  | 113.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 146.03 | 107.10 | 0.00 |  | 0.00 |  | 107.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 145.53 | 106.61 | 0.00 |  | 0.00 |  | 106.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 145.50 | 106.57 | 0.00 |  | 0.00 |  | 106.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 144.69 | 105.77 | 0.00 |  | 0.00 |  | 105.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 141.04 | 102.11 | 0.00 |  | 0.00 |  | 102.11 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-sib-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1518218 | 1518218 | 1.00 | 1 | 0.00 | 0 | 34896170 | 0.0 | 0.0 | true | 0 | 0 | 26.8 | 89 / 2 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-sib-0, loop 0 on CPU 2

2093770176 iterations, longest 26.8 µs; over 5 / 20 / 100 µs: 89 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.8 | 26.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 25.6 | 25.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 26.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 25.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-sib-0 (median 38.34)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 280.82 | 242.48 | 0.00 |  | 0.00 |  | 242.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 192.51 | 154.17 | 0.00 |  | 55.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 98.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 159.93 | 121.59 | 0.00 |  | 0.00 |  | 121.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 148.48 | 110.14 | 0.00 |  | 0.00 |  | 110.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 146.25 | 107.91 | 0.00 |  | 0.00 |  | 107.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 144.03 | 105.69 | 0.00 |  | 81.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 24.46 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 140.65 | 102.31 | 0.00 |  | 0.00 |  | 102.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 130.25 | 91.91 | 0.00 |  | 0.00 |  | 91.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 127.98 | 89.64 | 0.00 |  | 0.00 |  | 89.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 127.95 | 89.61 | 0.00 |  | 0.00 |  | 89.61 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-sib-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 24677781 | 0.0 | 0.0 | true | 0 | 0 | 164.6 | 123 / 3 / 1 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-sib-10000, loop 0 on CPU 2

2961333760 iterations, longest 164.6 µs; over 5 / 20 / 100 µs: 123 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 164.6 | 164.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 49.8 | 28.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 164.6 | 0 | softirq TIMER | softirq TIMER 147, local timer tick 8, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 28.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 21.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-sib-10000 (median 40.67)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 913.07 | 872.40 | 0.00 |  | 432.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 440.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 578.67 | 538.00 | 0.00 |  | 434.94 | unexplained: no kernel event (hypervisor or in-kernel time) | 103.07 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 565.74 | 525.07 | 0.00 |  | 219.59 | unexplained: no kernel event (hypervisor or in-kernel time) | 305.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 527.75 | 487.08 | 0.00 |  | 153.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 333.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 388.39 | 347.71 | 0.00 |  | 2.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 345.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 385.67 | 345.00 | 0.00 |  | 73.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 271.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 315.75 | 275.08 | 0.00 |  | 71.05 | unexplained: no kernel event (hypervisor or in-kernel time) | 204.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 307.02 | 266.35 | 0.00 |  | 55.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 210.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 298.29 | 257.61 | 0.00 |  | 61.86 | softirq TIMER | 195.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 271.68 | 231.01 | 0.00 |  | 0.00 |  | 231.01 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-1/http-recv-busy-10000, loop 0 on CPU 2

2112657599 iterations, longest 64.9 µs; over 5 / 20 / 100 µs: 67223 / 263 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 79 | 1869.4 | 34.1 |
| mostly unexplained (+preempted by kworker/2:1) | 60 | 1496.9 | 42.4 |
| mostly unexplained (+local timer tick) | 51 | 1172.2 | 27.9 |
| mostly unexplained (+softirq NET_RX) | 33 | 772.2 | 31.5 |
| softirq NET_RX | 27 | 636.4 | 32.7 |
| mostly unexplained (+irq work (IPI/vector)) | 9 | 203.9 | 25.2 |
| softirq TIMER | 1 | 64.9 | 64.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 47.6 | 26.5 |
| loop work | 1 | 29.8 | 29.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 64.9 | 1 | softirq TIMER | softirq TIMER 54, local timer tick 5, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 2 | 42.4 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 34.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 34.0 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 33.2 | 0 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 32.7 | 1 | softirq NET_RX | softirq NET_RX 17, preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 32.0 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 31.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, local timer tick 5, preempted by ksoftirqd/2 5, timer tick_nohz_handler 4 |
| 9 | 31.4 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 31.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 12, preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 1248 |
| IWI IRQ work interrupts | 1142 |
| CAL Function call interrupts | 997 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-1/http-recv-busy-10000 (median 38.58)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 551.21 | 512.63 | 0.00 |  | 231.78 | unexplained: no kernel event (hypervisor or in-kernel time) | 280.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 249.40 | 210.82 | 0.00 |  | 59.78 | unexplained: no kernel event (hypervisor or in-kernel time) | 151.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 216.59 | 178.01 | 0.00 |  | 0.00 |  | 178.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 200.38 | 161.81 | 0.00 |  | 54.69 | unexplained: no kernel event (hypervisor or in-kernel time) | 107.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 198.80 | 160.23 | 0.00 |  | 0.00 |  | 160.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 186.80 | 148.22 | 0.00 |  | 158.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 181.18 | 142.60 | 0.00 |  | 0.00 |  | 142.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 164.11 | 125.53 | 0.00 |  | 4.67 | softirq TIMER | 120.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 160.22 | 121.64 | 0.00 |  | 59.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 62.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 155.60 | 117.02 | 0.00 |  | 0.00 |  | 117.02 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-1/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=565792 clocksource=tsc

### Loop counters (r1-base-1/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002700 | 595699 | 5.04 | 113099008 | 37.67 | 1884983 | 1884983 | 99.5 | 65.8 | true | 0 | 695 | 513.1 | 137945 / 2205 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672604800 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (83453 of 2900000 in 1-msg reads) | 83453 | 2.62 | 2.47 | 2.96 | 7.06 | 13.48 | 18.75 | 30.42 |

#### Stalls: r1-base-1/md-recv-busy, loop 0 on CPU 2

113099008 iterations, longest 513.1 µs; over 5 / 20 / 100 µs: 137945 / 2205 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2181 | 52809.6 | 513.1 |
| softirq NET_RX | 7 | 180.7 | 35.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 152.0 | 36.2 |
| mostly unexplained (+preempted by kworker/2:1) | 3 | 73.5 | 26.9 |
| mostly unexplained (+local timer tick) | 3 | 66.2 | 22.8 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 62.1 | 36.0 |
| mostly unexplained (+softirq NET_RX) | 2 | 51.5 | 26.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 30.5 | 30.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 513.1 | 2700 | loop work (2700 items) |  |
| 2 | 64.1 | 20 | loop work (20 items) | softirq NET_RX 20, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 60.5 | 18 | loop work (18 items) | local timer tick 4, timer tick_nohz_handler 2 |
| 4 | 55.9 | 13 | loop work (13 items) | softirq NET_RX 11, local timer tick 3, preempted by kworker/2:1 2, irq work (IPI/vector) 1 |
| 5 | 49.7 | 20 | loop work (20 items) | softirq NET_RX 3 |
| 6 | 49.4 | 23 | loop work (23 items) | softirq NET_RX 1 |
| 7 | 48.7 | 29 | loop work (29 items) | softirq NET_RX 2 |
| 8 | 48.2 | 20 | loop work (20 items) |  |
| 9 | 45.9 | 20 | loop work (20 items) |  |
| 10 | 45.6 | 12 | loop work (12 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 715 |
| LOC Local timer interrupts | 709 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-1/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-1/noise, loop 0 on CPU 2

9190871040 iterations, longest 71.1 µs; over 5 / 20 / 100 µs: 67 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 60 | 474.2 | 23.5 |
| softirq TIMER | 1 | 71.1 | 71.1 |
| mostly unexplained (+local timer tick) | 3 | 19.5 | 7.8 |
| local timer tick | 2 | 19.3 | 13.7 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 5.7 | 5.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 71.1 | 0 | softirq TIMER | softirq TIMER 57, local timer tick 7, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 23.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 17.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 14.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 13.7 | 0 | local timer tick | local timer tick 7, timer tick_nohz_handler 3 |
| 6 | 10.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 8.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 8.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 8.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 8.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-base-1/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1300807 | 1300807 | 1.00 | 283530063 | 217.96 | 4725501 | 4682141 | 0.0 | 40.6 | false | 0 | 0 | 152.3 | 33187 / 11 / 1 | 0 | 0 |

#### Stalls: r1-base-1/pp-epoll-spin-0, loop 0 on CPU 2

280928448 iterations, longest 152.3 µs; over 5 / 20 / 100 µs: 33187 / 11 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 242.9 | 32.2 |
| softirq TIMER | 1 | 152.3 | 152.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.2 | 26.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 152.3 | 0 | softirq TIMER | softirq TIMER 132, local timer tick 11, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 32.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 31.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 30.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 29.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 26.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 26.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 25.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 23.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 23.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 988 |
| LOC Local timer interrupts | 9 |
| IWI IRQ work interrupts | 6 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-epoll-spin-0 (median 45.30)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 229.61 | 184.30 | 0.00 |  | 0.00 |  | 184.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 207.75 | 162.45 | 0.00 |  | 0.00 |  | 162.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 195.52 | 150.22 | 0.00 |  | 0.00 |  | 150.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 193.99 | 148.69 | 0.00 |  | 192.10 | softirq TIMER | 0.00 | server: softirq TIMER |
| 5 | 193.58 | 148.28 | 0.00 |  | 0.00 |  | 148.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 193.57 | 148.27 | 0.00 |  | 0.00 |  | 148.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 193.40 | 148.10 | 0.00 |  | 0.00 |  | 148.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 191.44 | 146.14 | 0.00 |  | 0.00 |  | 146.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 190.87 | 145.57 | 0.00 |  | 0.00 |  | 145.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 189.49 | 144.19 | 0.00 |  | 0.00 |  | 144.19 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 557228032 | 464.36 | 4643567 | 4623567 | 0.0 | 36.5 | false | 0 | 0 | 24.3 | 10950 / 2 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-epoll-spin-10000, loop 0 on CPU 2

554828032 iterations, longest 24.3 µs; over 5 / 20 / 100 µs: 10950 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 45.0 | 24.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 24.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 20.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-epoll-spin-10000 (median 43.38)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 210.48 | 167.10 | 0.00 |  | 0.00 |  | 167.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 203.55 | 160.16 | 0.00 |  | 0.00 |  | 160.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 202.11 | 158.73 | 0.00 |  | 0.00 |  | 158.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 194.49 | 151.11 | 0.00 |  | 0.00 |  | 151.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 193.40 | 150.02 | 0.00 |  | 0.00 |  | 150.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 193.14 | 149.76 | 0.00 |  | 0.00 |  | 149.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 189.86 | 146.48 | 0.00 |  | 0.00 |  | 146.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 189.24 | 145.86 | 0.00 |  | 0.00 |  | 145.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 185.54 | 142.16 | 0.00 |  | 0.00 |  | 142.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 182.59 | 139.21 | 0.00 |  | 0.00 |  | 139.21 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-mio-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 0 | 0 | 0.00 | 256515244 | 0.00 | 4275254 | 4233011 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

#### Stalls: r1-base-1/pp-mio-0, loop 0 on CPU 2

253980654 iterations, longest 6241.6 µs; over 5 / 20 / 100 µs: 912493 / 47 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+irq work (IPI/vector)) | 1 | 6241.6 | 6241.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 45 | 994.4 | 49.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.8 | 28.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 6241.6 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 49.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 33.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 28.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 27.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 24.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 23.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 23.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 22.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 22.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 988 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-mio-0 (median 48.65)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 303.96 | 255.31 | 0.00 |  | 0.00 |  | 255.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 300.40 | 251.74 | 0.00 |  | 66.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 184.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 275.56 | 226.91 | 0.00 |  | 0.00 |  | 226.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 246.61 | 197.96 | 0.00 |  | 0.00 |  | 197.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 229.09 | 180.44 | 0.00 |  | 155.87 | unexplained: no kernel event (hypervisor or in-kernel time) | 24.57 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 215.19 | 166.53 | 0.00 |  | 64.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 101.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 214.44 | 165.79 | 0.00 |  | 0.00 |  | 165.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 208.16 | 159.51 | 0.00 |  | 0.00 |  | 159.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 206.60 | 157.95 | 0.00 |  | 0.00 |  | 157.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 200.35 | 151.69 | 0.00 |  | 0.00 |  | 151.69 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-mio-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 0 | 0 | 0.00 | 512511726 | 0.00 | 4270931 | 4250931 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

#### Stalls: r1-base-1/pp-mio-10000, loop 0 on CPU 2

510111726 iterations, longest 5896.3 µs; over 5 / 20 / 100 µs: 31199 / 5 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+irq work (IPI/vector)) | 1 | 5896.3 | 5896.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 47.8 | 26.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 46.7 | 23.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 5896.3 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 26.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 23.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 22.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 21.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |
| IWI IRQ work interrupts | 6 |

#### Slowest round trips: r1-base-1/pp-mio-10000 (median 45.64)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 225.49 | 179.85 | 0.00 |  | 0.00 |  | 179.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 210.08 | 164.44 | 0.00 |  | 0.00 |  | 164.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 197.50 | 151.86 | 0.00 |  | 0.00 |  | 151.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 196.23 | 150.59 | 0.00 |  | 0.00 |  | 150.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 193.88 | 148.24 | 0.00 |  | 0.00 |  | 148.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 193.47 | 147.83 | 0.00 |  | 0.00 |  | 147.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 192.92 | 147.28 | 0.00 |  | 0.00 |  | 147.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 192.41 | 146.78 | 0.00 |  | 0.00 |  | 146.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 192.11 | 146.47 | 0.00 |  | 0.00 |  | 146.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 191.47 | 145.84 | 0.00 |  | 0.00 |  | 145.84 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1561131 | 1561131 | 1.00 | 107749996 | 69.02 | 1795833 | 1769814 | 98.5 | 72.0 | true | 0 | 2221 | 29.9 | 44859 / 18 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-recv-busy-0, loop 0 on CPU 2

106188864 iterations, longest 29.9 µs; over 5 / 20 / 100 µs: 44859 / 18 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 4 | 101.5 | 28.3 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 91.1 | 26.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 71.8 | 25.7 |
| mostly unexplained (+softirq NET_RX) | 3 | 67.6 | 25.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.1 | 29.9 |
| softirq NET_RX | 2 | 41.2 | 21.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 29.9 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 28.3 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 28.1 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 26.6 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 25.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 25.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 25.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 24.7 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, local timer tick 3, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 9 | 24.2 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 23.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2249 |
| IWI IRQ work interrupts | 2228 |
| CAL Function call interrupts | 988 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-recv-busy-0 (median 37.37)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1016.27 | 978.89 | 0.00 |  | 0.00 |  | 978.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 468.93 | 431.56 | 0.00 |  | 69.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 361.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 464.14 | 426.77 | 0.00 |  | 288.60 | unexplained: no kernel event (hypervisor or in-kernel time) | 138.17 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 380.61 | 343.23 | 0.00 |  | 217.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 125.85 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 319.71 | 282.33 | 0.00 |  | 76.64 | unexplained: no kernel event (hypervisor or in-kernel time) | 205.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 298.83 | 261.45 | 0.00 |  | 61.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 199.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 257.20 | 219.82 | 0.00 |  | 0.00 |  | 219.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 240.46 | 203.09 | 0.00 |  | 0.00 |  | 203.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 222.73 | 185.35 | 0.00 |  | 88.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 97.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 222.72 | 185.35 | 0.00 |  | 0.00 |  | 185.35 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 233398144 | 194.50 | 1944985 | 1934985 | 99.5 | 68.1 | true | 0 | 5983 | 34.9 | 30286 / 65 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-recv-busy-10000, loop 0 on CPU 2

232198144 iterations, longest 34.9 µs; over 5 / 20 / 100 µs: 30286 / 65 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 19 | 450.7 | 33.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 14 | 377.2 | 34.9 |
| mostly unexplained (+softirq NET_RX) | 13 | 302.9 | 28.3 |
| softirq NET_RX | 13 | 301.1 | 29.5 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 89.8 | 26.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 48.2 | 27.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 34.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 34.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 33.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 33.7 | 0 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 7, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 5 | 29.5 | 1 | softirq NET_RX | softirq NET_RX 16, preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 29.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 29.1 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 29.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 28.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 28.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 6006 |
| IWI IRQ work interrupts | 5992 |
| CAL Function call interrupts | 1009 |

#### Slowest round trips: r1-base-1/pp-recv-busy-10000 (median 38.29)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 639.70 | 601.42 | 0.00 |  | 439.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 161.61 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 437.32 | 399.03 | 0.00 |  | 403.36 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 306.76 | 268.48 | 0.00 |  | 124.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 143.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 260.44 | 222.16 | 0.00 |  | 75.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 147.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 195.80 | 157.52 | 0.00 |  | 163.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 184.50 | 146.22 | 0.00 |  | 0.00 |  | 146.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 155.59 | 117.30 | 0.00 |  | 111.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 6.16 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 130.49 | 92.20 | 0.00 |  | 0.00 |  | 92.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 129.34 | 91.05 | 0.00 |  | 0.00 |  | 91.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 126.54 | 88.25 | 0.00 |  | 0.00 |  | 88.25 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1492620 | 1492620 | 1.00 | 1 | 0.00 | 0 | 35726373 | 0.0 | 0.0 | true | 0 | 0 | 33.2 | 85 / 2 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-0, loop 0 on CPU 2

2143582400 iterations, longest 33.2 µs; over 5 / 20 / 100 µs: 85 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 56.8 | 33.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 33.2 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 23.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 995 |
| LOC Local timer interrupts | 5 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-0 (median 38.72)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 165.16 | 126.44 | 0.00 |  | 0.00 |  | 126.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 155.76 | 117.04 | 0.00 |  | 0.00 |  | 117.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 151.03 | 112.30 | 0.00 |  | 0.00 |  | 112.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 148.37 | 109.64 | 0.00 |  | 0.00 |  | 109.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 145.29 | 106.57 | 0.00 |  | 0.00 |  | 106.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 145.20 | 106.47 | 0.00 |  | 0.00 |  | 106.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 143.01 | 104.29 | 0.00 |  | 0.00 |  | 104.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 139.17 | 100.45 | 0.00 |  | 0.00 |  | 100.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 137.63 | 98.91 | 0.00 |  | 0.00 |  | 98.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 137.61 | 98.89 | 0.00 |  | 0.00 |  | 98.89 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 21963630 | 0.0 | 0.0 | true | 0 | 0 | 202.8 | 150 / 4 / 1 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-10000, loop 0 on CPU 2

2635635648 iterations, longest 202.8 µs; over 5 / 20 / 100 µs: 150 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 202.8 | 202.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 81.6 | 31.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 202.8 | 0 | softirq TIMER | softirq TIMER 186, local timer tick 8, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 31.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 29.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 20.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1008 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-10000 (median 39.56)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1123.90 | 1084.34 | 0.00 |  | 794.47 | unexplained: no kernel event (hypervisor or in-kernel time) | 289.87 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 864.18 | 824.62 | 0.00 |  | 345.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 479.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 409.15 | 369.59 | 0.00 |  | 259.60 | unexplained: no kernel event (hypervisor or in-kernel time) | 109.99 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 293.54 | 253.98 | 0.00 |  | 178.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 75.45 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 263.41 | 223.85 | 202.83 | softirq TIMER | 0.00 |  | 21.01 | client: softirq TIMER |
| 6 | 258.45 | 218.89 | 0.00 |  | 94.29 | softirq TIMER | 124.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 210.41 | 170.85 | 0.00 |  | 0.00 |  | 170.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 195.38 | 155.82 | 0.00 |  | 60.78 | unexplained: no kernel event (hypervisor or in-kernel time) | 95.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 172.04 | 132.48 | 0.00 |  | 0.00 |  | 132.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 161.99 | 122.43 | 0.00 |  | 0.00 |  | 122.43 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-sib-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1518042 | 1518042 | 1.00 | 1 | 0.00 | 0 | 34903691 | 0.0 | 0.0 | true | 0 | 0 | 32.6 | 95 / 3 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-sib-0, loop 0 on CPU 2

2094221440 iterations, longest 32.6 µs; over 5 / 20 / 100 µs: 95 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 53.4 | 32.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.8 | 26.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 32.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 26.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 20.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 995 |
| LOC Local timer interrupts | 20 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-sib-0 (median 38.53)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 431.72 | 393.19 | 0.00 |  | 273.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 119.69 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 331.40 | 292.86 | 0.00 |  | 80.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 212.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 315.04 | 276.50 | 0.00 |  | 0.00 |  | 276.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 302.50 | 263.97 | 0.00 |  | 0.00 |  | 263.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 278.32 | 239.79 | 0.00 |  | 153.61 | unexplained: no kernel event (hypervisor or in-kernel time) | 86.18 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 270.31 | 231.77 | 0.00 |  | 181.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 50.50 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 239.66 | 201.13 | 0.00 |  | 67.70 | unexplained: no kernel event (hypervisor or in-kernel time) | 133.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 161.80 | 123.27 | 0.00 |  | 74.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 49.26 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 155.47 | 116.94 | 0.00 |  | 0.00 |  | 116.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 155.11 | 116.57 | 0.00 |  | 63.83 | unexplained: no kernel event (hypervisor or in-kernel time) | 52.75 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-1/pp-uring-sqpoll-sib-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 24804102 | 0.0 | 0.0 | true | 0 | 0 | 258.4 | 156 / 4 / 1 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-sib-10000, loop 0 on CPU 2

2976492224 iterations, longest 258.4 µs; over 5 / 20 / 100 µs: 156 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 258.4 | 258.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 51.2 | 28.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 23.5 | 23.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 258.4 | 0 | softirq TIMER | softirq TIMER 240, local timer tick 8, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 28.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 23.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 23.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1007 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-sib-10000 (median 40.81)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 699.87 | 659.05 | 0.00 |  | 437.06 | unexplained: no kernel event (hypervisor or in-kernel time) | 222.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 401.66 | 360.85 | 0.00 |  | 72.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 288.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 302.10 | 261.29 | 0.00 |  | 75.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 185.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 225.33 | 184.52 | 0.00 |  | 96.73 | softirq TIMER | 87.78 | server: softirq TIMER |
| 5 | 164.95 | 124.14 | 0.00 |  | 0.00 |  | 124.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 162.34 | 121.53 | 0.00 |  | 0.00 |  | 121.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 152.74 | 111.93 | 0.00 |  | 0.00 |  | 111.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 144.03 | 103.22 | 0.00 |  | 0.00 |  | 103.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 141.47 | 100.66 | 0.00 |  | 0.00 |  | 100.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 140.64 | 99.83 | 0.00 |  | 0.00 |  | 99.83 | neither loop stalled long enough: network, hypervisor or untraced |

## Ping-pong round trip (client send → echo decoded; "from when due": from the slot on the paced schedule, including any wait behind a stall)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189941 | 39.43 | 38.21 | 44.69 | 55.04 | 76.13 | 101.60 | 578.68 |
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189941 | 39.60 | 38.37 | 44.84 | 55.21 | 76.78 | 111.04 | 667.03 |
| r1-base-0/pp-epoll-spin-0: epoll-spin @ 0/s | 1303258 | 45.17 | 43.21 | 51.19 | 68.11 | 101.35 | 235.89 | 1018.10 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 56.19 | 49.90 | 79.98 | 93.97 | 168.85 | 194.09 | 242.88 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 74.89 | 50.13 | 80.58 | 97.49 | 7196.97 | 16580.98 | 18832.10 |
| r1-base-0/pp-mio-0: mio @ 0/s | 1306166 | 45.14 | 43.66 | 49.69 | 61.11 | 110.19 | 175.23 | 207.32 |
| r1-base-0/pp-mio-10000: mio @ 10000/s | 1190017 | 49.18 | 47.80 | 54.17 | 87.55 | 116.77 | 173.62 | 519.38 |
| r1-base-0/pp-mio-10000: mio @ 10000/s, from when due | 1190017 | 50.50 | 47.92 | 54.32 | 88.41 | 248.40 | 4098.03 | 6715.93 |
| r1-base-0/pp-recv-busy-0: recv-busy @ 0/s | 1570193 | 37.47 | 36.33 | 42.84 | 51.79 | 67.27 | 96.20 | 901.80 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 39.89 | 38.79 | 44.80 | 53.96 | 67.86 | 89.18 | 961.34 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 40.21 | 39.11 | 45.12 | 54.30 | 68.40 | 93.05 | 961.70 |
| r1-base-0/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1474088 | 39.73 | 38.77 | 45.04 | 52.97 | 63.32 | 98.56 | 180.17 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189997 | 39.96 | 38.92 | 44.27 | 53.55 | 83.36 | 112.22 | 233.04 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189997 | 40.04 | 38.99 | 44.34 | 53.63 | 85.05 | 128.22 | 320.46 |
| r1-base-0/pp-uring-sqpoll-sib-0: uring-sqpoll @ 0/s | 1492556 | 39.37 | 38.34 | 44.40 | 52.59 | 66.14 | 94.06 | 280.82 |
| r1-base-0/pp-uring-sqpoll-sib-10000: uring-sqpoll @ 10000/s | 1189999 | 41.90 | 40.67 | 47.46 | 56.67 | 74.04 | 102.57 | 913.07 |
| r1-base-0/pp-uring-sqpoll-sib-10000: uring-sqpoll @ 10000/s, from when due | 1189999 | 42.05 | 40.76 | 47.55 | 56.79 | 75.28 | 145.35 | 1768.84 |
| r1-base-1/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189941 | 39.55 | 38.58 | 44.14 | 52.41 | 63.64 | 92.79 | 551.21 |
| r1-base-1/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189941 | 39.73 | 38.75 | 44.32 | 52.60 | 63.97 | 98.15 | 551.38 |
| r1-base-1/pp-epoll-spin-0: epoll-spin @ 0/s | 1280756 | 45.97 | 45.30 | 50.82 | 68.08 | 102.04 | 158.68 | 229.61 |
| r1-base-1/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 45.47 | 43.38 | 50.37 | 89.02 | 97.91 | 146.21 | 210.48 |
| r1-base-1/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 45.73 | 43.54 | 50.55 | 89.26 | 99.67 | 412.17 | 964.26 |
| r1-base-1/pp-mio-0: mio @ 0/s | 1245459 | 47.34 | 48.65 | 51.52 | 92.77 | 101.51 | 145.58 | 303.96 |
| r1-base-1/pp-mio-10000: mio @ 10000/s | 1189998 | 47.93 | 45.64 | 53.67 | 90.12 | 98.73 | 150.40 | 225.49 |
| r1-base-1/pp-mio-10000: mio @ 10000/s, from when due | 1189998 | 48.57 | 45.75 | 54.01 | 90.34 | 100.25 | 2706.36 | 4543.18 |
| r1-base-1/pp-recv-busy-0: recv-busy @ 0/s | 1534581 | 38.34 | 37.37 | 43.09 | 51.68 | 62.48 | 83.19 | 1016.27 |
| r1-base-1/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 39.00 | 38.29 | 42.61 | 50.50 | 65.16 | 94.52 | 639.70 |
| r1-base-1/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 39.30 | 38.58 | 42.91 | 50.81 | 65.76 | 98.36 | 640.12 |
| r1-base-1/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1467703 | 39.94 | 38.72 | 45.80 | 55.23 | 69.51 | 97.49 | 165.16 |
| r1-base-1/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 40.58 | 39.56 | 45.55 | 53.53 | 64.47 | 93.46 | 1123.90 |
| r1-base-1/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 40.70 | 39.66 | 45.66 | 53.67 | 65.17 | 110.47 | 1123.96 |
| r1-base-1/pp-uring-sqpoll-sib-0: uring-sqpoll @ 0/s | 1493050 | 39.36 | 38.53 | 43.49 | 51.46 | 63.01 | 95.00 | 431.72 |
| r1-base-1/pp-uring-sqpoll-sib-10000: uring-sqpoll @ 10000/s | 1189999 | 41.92 | 40.81 | 47.35 | 55.94 | 68.09 | 95.59 | 699.87 |
| r1-base-1/pp-uring-sqpoll-sib-10000: uring-sqpoll @ 10000/s, from when due | 1189999 | 42.02 | 40.90 | 47.44 | 56.04 | 68.57 | 103.44 | 699.97 |

