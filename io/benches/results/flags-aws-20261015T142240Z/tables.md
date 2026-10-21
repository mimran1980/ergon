# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|
| r1-base-0 | 26.93 | 0.5 | 0.5 | 0.0 | 26.93 µs stall: mostly unexplained (+softirq TIMER) |
| r1-base-1 | 29.48 | 1.0 | 0.5 | 0.0 | 29.48 µs stall: mostly unexplained (+local timer tick) |

## HTTP round trip, http-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189938 | 39.63 | 54.66 | 67.87 | 80.21 | 1151.51 | 17.6 | 1.0 | 0.5 | 0.0 | 53.19 | 2695.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-1 | 1189938 | 39.58 | 51.17 | 66.53 | 75.23 | 127.66 | 6.6 | 0.0 | 0.0 | 0.0 | 52.94 | 2919.5 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189938 | 39.75 | 54.81 | 68.22 | 86.64 | 1151.62 | 44.9 | 19.7 | 3.0 | 0.0 |
| r1-base-1 | 1189938 | 39.72 | 51.31 | 66.67 | 75.37 | 127.80 | 6.6 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343095 | 4.19 | 11.34 | 13.78 | 17.92 | 28.91 | 0.0 | 0.0 | 0.0 | 0.0 | 154.45 | 2.0 | 154.45 µs stall: loop work (589 items) |
| m2 | 343095 | 4.42 | 11.60 | 22.72 | 34.63 | 37.64 | 0.0 | 0.0 | 0.0 | 0.0 | 183.15 | 50.0 | 183.15 µs stall: loop work (590 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343095 | 25.58 | 47.28 | 55.83 | 63.56 | 113.76 | 1.0 | 0.0 | 0.0 | 0.0 |
| m2 | 343095 | 31.70 | 52.99 | 64.03 | 75.67 | 181.95 | 14.5 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053554 | 4.55 | 12.13 | 16.09 | 29.62 | 42.14 | 0.0 | 0.0 | 0.0 | 0.0 | 1208.52 | 129.0 | 1208.52 µs stall: loop work (5166 items) |
| m2 | 3053557 | 4.38 | 11.32 | 14.31 | 25.91 | 39.19 | 0.0 | 0.0 | 0.0 | 0.0 | 1255.70 | 93.0 | 1255.70 µs stall: loop work (5286 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053554 | 37.15 | 84.06 | 101.27 | 144.21 | 535.14 | 3693.2 | 7.2 | 0.0 | 0.0 |
| m2 | 3053557 | 34.33 | 82.31 | 103.45 | 154.16 | 572.63 | 4259.0 | 7.2 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387080 | 4.03 | 11.49 | 15.43 | 29.43 | 44.41 | 0.0 | 0.0 | 0.0 | 0.0 | 1787.53 | 291.0 | 1787.53 µs stall: loop work (5343 items) |
| m2 | 4387100 | 4.07 | 11.60 | 15.79 | 30.73 | 40.80 | 0.0 | 0.0 | 0.0 | 0.0 | 1808.90 | 365.0 | 1808.90 µs stall: loop work (5472 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387080 | 32.38 | 84.32 | 101.89 | 135.26 | 2640.44 | 5658.6 | 44.5 | 11.4 | 0.0 |
| m2 | 4387100 | 34.76 | 88.75 | 108.59 | 140.74 | 2019.70 | 11502.4 | 27.9 | 20.7 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343095 | 3.12 | 6.80 | 14.90 | 20.91 | 26.53 | 0.0 | 0.0 | 0.0 | 0.0 | 156.57 | 1197.0 | 156.57 µs stall: loop work (589 items) |
| m2 | 343096 | 2.69 | 5.59 | 12.16 | 18.14 | 32.76 | 0.0 | 0.0 | 0.0 | 0.0 | 178.72 | 586.0 | 178.72 µs stall: loop work (601 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343095 | 28.34 | 57.12 | 80.32 | 98.80 | 130.14 | 31.0 | 0.0 | 0.0 | 0.0 |
| m2 | 343096 | 26.94 | 54.76 | 87.71 | 114.32 | 134.95 | 108.6 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053545 | 4.77 | 17.97 | 29.67 | 39.72 | 57.83 | 0.0 | 0.0 | 0.0 | 0.0 | 1069.26 | 5131.0 | 1069.26 µs stall: loop work (4943 items) |
| m2 | 3053553 | 4.67 | 17.71 | 29.20 | 39.54 | 62.92 | 0.0 | 0.0 | 0.0 | 0.0 | 1252.35 | 4543.0 | 1252.35 µs stall: loop work (5313 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053545 | 32.25 | 81.57 | 104.47 | 146.99 | 998.75 | 4415.2 | 29.0 | 0.0 | 0.0 |
| m2 | 3053553 | 35.34 | 83.38 | 100.33 | 131.05 | 1378.58 | 3277.3 | 65.2 | 21.7 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387225 | 28.15 | 73.33 | 86.50 | 100.80 | 134.14 | 553.4 | 0.0 | 0.0 | 0.0 | 2417.04 | 1219519.0 | 2417.04 µs stall: loop work (7712 items) |
| m2 | 4387244 | 28.38 | 73.84 | 88.06 | 103.31 | 127.91 | 737.6 | 0.0 | 0.0 | 0.0 | 2466.91 | 1222911.0 | 2466.91 µs stall: loop work (7730 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387225 | 62.01 | 131.18 | 150.75 | 179.03 | 1472.13 | 508466.1 | 22.8 | 7.2 | 0.0 |
| m2 | 4387244 | 64.25 | 133.27 | 152.10 | 171.88 | 2318.95 | 571207.3 | 55.9 | 52.8 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 2.85 | 6.21 | 11.16 | 23.09 | 35.61 | 0.0 | 0.0 | 0.0 | 0.0 | 154.41 | 459.0 | 154.41 µs stall: loop work (578 items) |
| m2 | 343095 | 2.82 | 5.58 | 10.14 | 14.54 | 33.64 | 0.0 | 0.0 | 0.0 | 0.0 | 180.36 | 585.0 | 180.36 µs stall: loop work (589 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 24.61 | 48.18 | 56.93 | 65.20 | 79.59 | 0.0 | 0.0 | 0.0 | 0.0 |
| m2 | 343095 | 28.16 | 51.00 | 61.87 | 69.41 | 95.65 | 0.0 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053557 | 5.10 | 18.75 | 29.63 | 39.38 | 56.51 | 0.0 | 0.0 | 0.0 | 0.0 | 1172.71 | 5243.0 | 1172.71 µs stall: loop work (5188 items) |
| m2 | 3053552 | 4.92 | 18.58 | 29.57 | 40.28 | 58.10 | 0.0 | 0.0 | 0.0 | 0.0 | 1321.67 | 5179.0 | 1321.67 µs stall: loop work (5356 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053557 | 38.86 | 87.09 | 105.13 | 148.41 | 411.65 | 5959.7 | 0.0 | 0.0 | 0.0 |
| m2 | 3053552 | 35.48 | 85.13 | 112.47 | 188.89 | 955.73 | 6753.1 | 21.7 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387103 | 8.82 | 23.36 | 34.76 | 46.41 | 70.66 | 0.0 | 0.0 | 0.0 | 0.0 | 2210.03 | 33557.0 | 2210.03 µs stall: loop work (6539 items) |
| m2 | 4387112 | 8.94 | 23.33 | 34.23 | 45.49 | 57.16 | 0.0 | 0.0 | 0.0 | 0.0 | 2210.61 | 33273.0 | 2210.61 µs stall: loop work (6478 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387103 | 32.52 | 89.60 | 108.19 | 131.94 | 1763.49 | 12209.0 | 53.8 | 36.2 | 0.0 |
| m2 | 4387112 | 38.46 | 94.37 | 111.80 | 141.49 | 2905.61 | 21169.6 | 89.0 | 30.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 3.59 | 7.12 | 11.17 | 23.04 | 26.37 | 0.0 | 0.0 | 0.0 | 0.0 | 79.36 | 1.0 | 79.36 µs stall: loop work (578 items) |
| m2 | 343095 | 3.45 | 6.08 | 10.81 | 18.65 | 32.91 | 0.0 | 0.0 | 0.0 | 0.0 | 81.82 | 1.0 | 81.82 µs stall: loop work (589 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 14.09 | 29.36 | 38.80 | 49.01 | 58.47 | 0.0 | 0.0 | 0.0 | 0.0 |
| m2 | 343095 | 28.32 | 51.13 | 61.61 | 70.92 | 102.13 | 3.1 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053455 | 5.04 | 15.11 | 27.34 | 34.99 | 48.04 | 0.0 | 0.0 | 0.0 | 0.0 | 678.86 | 1.0 | 678.86 µs stall: loop work (5232 items) |
| m2 | 3053455 | 5.18 | 15.16 | 28.40 | 36.47 | 49.70 | 0.0 | 0.0 | 0.0 | 0.0 | 674.91 | 1.0 | 674.91 µs stall: loop work (5194 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053455 | 38.05 | 88.61 | 110.78 | 143.70 | 695.58 | 8378.5 | 36.2 | 0.0 | 0.0 |
| m2 | 3053455 | 35.06 | 89.20 | 117.23 | 182.60 | 1257.42 | 10391.6 | 36.2 | 7.2 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386962 | 5.84 | 16.77 | 30.49 | 39.52 | 55.77 | 0.0 | 0.0 | 0.0 | 0.0 | 641.39 | 2.0 | 641.39 µs stall: loop work (4753 items) |
| m2 | 4386962 | 5.97 | 16.83 | 30.75 | 39.37 | 63.00 | 0.0 | 0.0 | 0.0 | 0.0 | 646.31 | 2.0 | 646.31 µs stall: loop work (4751 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386962 | 26.69 | 81.23 | 99.27 | 136.44 | 3027.13 | 4182.5 | 71.4 | 31.0 | 0.0 |
| m2 | 4386962 | 35.10 | 89.89 | 106.80 | 139.40 | 1719.55 | 10836.5 | 37.2 | 33.1 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-sib-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 5.14 | 10.60 | 17.76 | 30.98 | 34.18 | 0.0 | 0.0 | 0.0 | 0.0 | 96.67 | 1.0 | 96.67 µs stall: loop work (577 items) |
| m2 | 343095 | 4.95 | 9.62 | 15.40 | 24.25 | 35.00 | 0.0 | 0.0 | 0.0 | 0.0 | 99.19 | 1.0 | 99.19 µs stall: loop work (589 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-sib-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 24.69 | 48.12 | 57.80 | 66.84 | 88.44 | 0.0 | 0.0 | 0.0 | 0.0 |
| m2 | 343095 | 31.75 | 56.17 | 65.65 | 75.75 | 111.59 | 7.2 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-sib-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053556 | 6.74 | 20.95 | 39.63 | 50.84 | 83.77 | 0.0 | 0.0 | 0.0 | 0.0 | 849.29 | 1.0 | 849.29 µs stall: loop work (5271 items) |
| m2 | 3053453 | 6.53 | 21.29 | 40.13 | 51.00 | 68.91 | 0.0 | 0.0 | 0.0 | 0.0 | 866.81 | 1.0 | 866.81 µs stall: loop work (5366 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-sib-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053556 | 41.71 | 95.33 | 131.64 | 190.19 | 950.16 | 20725.2 | 43.4 | 0.0 | 0.0 |
| m2 | 3053453 | 33.63 | 84.73 | 105.68 | 142.97 | 1130.70 | 5734.2 | 36.2 | 14.5 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-sib-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387009 | 7.74 | 23.50 | 41.67 | 54.48 | 92.36 | 0.0 | 0.0 | 0.0 | 0.0 | 782.39 | 2.0 | 782.39 µs stall: loop work (4740 items) |
| m2 | 4386955 | 7.71 | 24.45 | 43.39 | 56.46 | 100.84 | 6.2 | 0.0 | 0.0 | 0.0 | 767.33 | 2.0 | 767.33 µs stall: loop work (4697 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-sib-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387009 | 35.48 | 91.97 | 111.29 | 145.61 | 2593.45 | 16668.1 | 112.8 | 29.0 | 0.0 |
| m2 | 4386955 | 39.07 | 97.11 | 117.45 | 148.19 | 1393.97 | 32333.7 | 29.0 | 11.4 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 2900000 | 2.76 | 6.74 | 9.99 | 13.86 | 34.00 | 0.0 | 0.0 | 0.0 | 0.0 | 467.98 | 5962.0 | 467.98 µs stall: loop work (2650 items) |
| r1-base-1 | 2899983 | 3.00 | 6.88 | 9.61 | 13.01 | 19.03 | 0.0 | 0.0 | 0.0 | 0.0 | 446.42 | 4977.0 | 446.42 µs stall: loop work (2600 items) |

### Market-data burst, kernel arrival → decoded (every message), md-recv-busy, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 2900000 | 33.75 | 54.13 | 80.10 | 165.87 | 252.58 | 1230.0 | 0.0 | 0.0 | 0.0 |
| r1-base-1 | 2899983 | 30.59 | 69.69 | 82.57 | 98.60 | 128.43 | 246.2 | 0.0 | 0.0 | 0.0 |

## WebSocket round trip, pp-epoll-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1400975 | 41.06 | 56.24 | 65.01 | 99.18 | 379.65 | 140.3 | 0.0 | 0.0 | 0.0 | 33.67 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1433115 | 39.44 | 67.27 | 91.77 | 108.19 | 193.31 | 511.5 | 0.0 | 0.0 | 0.0 | 33.62 | 6.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

## WebSocket round trip, pp-epoll-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 39.96 | 50.81 | 60.40 | 98.70 | 927.82 | 56.0 | 1.0 | 0.0 | 0.0 | 33.43 | 0.5 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-1 | 1189999 | 41.18 | 51.13 | 62.53 | 77.04 | 148.68 | 4.5 | 0.0 | 0.0 | 0.0 | 14.20 | 0.0 | server: softirq TIMER |

### WebSocket round trip, pp-epoll-spin-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 40.08 | 50.95 | 60.75 | 106.75 | 927.95 | 79.7 | 8.1 | 0.0 | 0.0 |
| r1-base-1 | 1189999 | 41.29 | 51.25 | 62.66 | 77.22 | 148.75 | 4.5 | 0.0 | 0.0 | 0.0 |

## WebSocket round trip, pp-recv-busy-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1650555 | 35.28 | 43.27 | 55.74 | 64.39 | 206.07 | 23.4 | 0.0 | 0.0 | 0.0 | 22.86 | 1.0 | server: softirq TIMER |
| r1-base-1 | 1634597 | 35.66 | 43.44 | 55.15 | 64.46 | 159.93 | 32.5 | 0.0 | 0.0 | 0.0 | 17.70 | 0.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 37.05 | 43.89 | 56.13 | 64.26 | 112.86 | 3.0 | 0.0 | 0.0 | 0.0 | 39.31 | 0.5 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-1 | 1189999 | 37.82 | 44.84 | 57.35 | 65.06 | 962.59 | 9.6 | 1.0 | 0.0 | 0.0 | 18.67 | 0.0 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 37.30 | 44.15 | 56.41 | 64.45 | 112.92 | 3.0 | 0.0 | 0.0 | 0.0 |
| r1-base-1 | 1189999 | 38.07 | 45.11 | 57.78 | 68.12 | 962.68 | 28.2 | 8.6 | 0.0 | 0.0 |

## WebSocket round trip, pp-uring-sqpoll-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1449905 | 39.68 | 53.82 | 63.28 | 88.76 | 429.02 | 111.9 | 0.0 | 0.0 | 0.0 | 4.87 | 0.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1526877 | 38.05 | 45.46 | 57.06 | 65.92 | 331.87 | 25.4 | 0.0 | 0.0 | 0.0 | 4.46 | 0.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

## WebSocket round trip, pp-uring-sqpoll-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189998 | 38.15 | 50.23 | 59.06 | 72.66 | 169.22 | 9.1 | 0.0 | 0.0 | 0.0 | 16.58 | 0.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-1 | 1189998 | 40.18 | 52.69 | 61.89 | 83.03 | 851.80 | 31.8 | 1.0 | 0.0 | 0.0 | 4.45 | 0.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### WebSocket round trip, pp-uring-sqpoll-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189998 | 38.20 | 50.28 | 59.13 | 72.88 | 169.26 | 11.1 | 0.0 | 0.0 | 0.0 |
| r1-base-1 | 1189998 | 40.23 | 52.77 | 62.21 | 93.69 | 1023.08 | 49.4 | 9.1 | 0.5 | 0.0 |

## WebSocket round trip, pp-uring-sqpoll-sib-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1516530 | 38.38 | 46.40 | 58.61 | 67.40 | 859.17 | 11.2 | 1.0 | 0.0 | 0.0 | 5.26 | 0.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1540377 | 37.81 | 45.32 | 57.27 | 67.18 | 312.20 | 30.5 | 0.0 | 0.0 | 0.0 | 5.61 | 0.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

## WebSocket round trip, pp-uring-sqpoll-sib-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 40.95 | 74.04 | 100.59 | 119.91 | 765.76 | 643.4 | 1.0 | 0.0 | 0.0 | 5.37 | 0.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 1189999 | 41.26 | 48.01 | 61.16 | 70.91 | 881.04 | 8.1 | 1.0 | 0.0 | 0.0 | 20.45 | 0.5 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### WebSocket round trip, pp-uring-sqpoll-sib-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 41.01 | 74.31 | 102.95 | 125.75 | 765.81 | 781.0 | 3.5 | 0.0 | 0.0 |
| r1-base-1 | 1189999 | 41.32 | 48.08 | 61.42 | 72.68 | 881.10 | 21.7 | 5.5 | 0.0 | 0.0 |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs, and system-management interrupts (SMI, bare metal only: firmware pauses). The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | SMI | receiving CPU | LOC | IWI | RES | CAL | SMI |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| http-recv-busy-10000 | r1-base-0 | 1 | 65 | 62 | 0 | 9 | 0 |  |  |  |  |  |  |
| http-recv-busy-10000 | r1-base-1 | 1 | 64 | 64 | 0 | 9 | 0 |  |  |  |  |  |  |
| lab-epoll-spin-c1 | m1 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 0 | 0 | 2 | 0 |
| lab-epoll-spin-c1 | m2 | 1 | 0 | 0 | 0 | 17 | 0 | 48 | 159 | 0 | 0 | 21 | 0 |
| lab-epoll-spin-c10 | m1 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 2 | 0 | 11 | 0 |
| lab-epoll-spin-c10 | m2 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 2 | 0 | 3 | 0 |
| lab-epoll-spin-c100 | m1 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 0 | 1 | 14 | 0 |
| lab-epoll-spin-c100 | m2 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 0 | 1 | 13 | 0 |
| lab-recv-busy-c1 | m1 | 1 | 15 | 95 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c1 | m2 | 1 | 10 | 92 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c10 | m1 | 1 | 30 | 82 | 0 | 17 | 0 | 48 | 150 | 0 | 0 | 21 | 0 |
| lab-recv-busy-c10 | m2 | 1 | 30 | 77 | 0 | 17 | 0 | 0 | 1144 | 0 | 0 | 15 | 0 |
| lab-recv-busy-c100 | m1 | 1 | 71 | 95 | 0 | 18 | 0 | 0 | 1146 | 2 | 1 | 13 | 0 |
| lab-recv-busy-c100 | m2 | 1 | 70 | 96 | 0 | 18 | 0 | 48 | 397 | 2 | 1 | 5 | 0 |
| lab-recv-busy-pq-c1 | m1 | 1 | 8 | 89 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-pq-c1 | m2 | 1 | 12 | 91 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-pq-c10 | m1 | 1 | 30 | 89 | 0 | 17 | 0 | 48 | 195 | 0 | 0 | 20 | 0 |
| lab-recv-busy-pq-c10 | m2 | 1 | 25 | 80 | 0 | 17 | 0 | 48 | 176 | 0 | 0 | 21 | 0 |
| lab-recv-busy-pq-c100 | m1 | 1 | 77 | 105 | 0 | 17 | 0 | 0 | 1145 | 0 | 1 | 13 | 0 |
| lab-recv-busy-pq-c100 | m2 | 1 | 79 | 106 | 0 | 18 | 0 | 48 | 378 | 2 | 0 | 5 | 0 |
| lab-uring-sqpoll-c1 | m1 | 1 | 0 | 0 | 0 | 17 | 0 | 13 | 29 | 91 | 1 | 17 | 0 |
| lab-uring-sqpoll-c1 | m2 | 1 | 0 | 0 | 0 | 17 | 0 | 13 | 25 | 90 | 1 | 17 | 0 |
| lab-uring-sqpoll-c10 | m1 | 1 | 0 | 0 | 0 | 17 | 0 | 13 | 137 | 132 | 0 | 18 | 0 |
| lab-uring-sqpoll-c10 | m2 | 1 | 0 | 0 | 0 | 17 | 0 | 13 | 115 | 121 | 1 | 18 | 0 |
| lab-uring-sqpoll-c100 | m1 | 1 | 0 | 0 | 0 | 17 | 0 | 13 | 262 | 177 | 0 | 21 | 0 |
| lab-uring-sqpoll-c100 | m2 | 1 | 0 | 0 | 0 | 17 | 0 | 48 | 351 | 0 | 0 | 23 | 0 |
| lab-uring-sqpoll-sib-c1 | m1 | 1 | 0 | 0 | 0 | 0 | 0 | 49 | 37 | 95 | 1 | 1 | 0 |
| lab-uring-sqpoll-sib-c1 | m2 | 1 | 0 | 0 | 0 | 0 | 0 | 49 | 36 | 95 | 1 | 1 | 0 |
| lab-uring-sqpoll-sib-c10 | m1 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1141 | 0 | 0 | 8 | 0 |
| lab-uring-sqpoll-sib-c10 | m2 | 1 | 0 | 0 | 0 | 0 | 0 | 49 | 114 | 118 | 0 | 1 | 0 |
| lab-uring-sqpoll-sib-c100 | m1 | 1 | 0 | 0 | 0 | 0 | 0 | 49 | 276 | 177 | 0 | 4 | 0 |
| lab-uring-sqpoll-sib-c100 | m2 | 1 | 0 | 0 | 0 | 0 | 0 | 49 | 286 | 174 | 0 | 7 | 0 |
| md-recv-busy | r1-base-0 | 1 | 114 | 181 | 0 | 17 | 0 |  |  |  |  |  |  |
| md-recv-busy | r1-base-1 | 1 | 102 | 170 | 0 | 17 | 0 |  |  |  |  |  |  |
| noise | r1-base-0 | 1 | 0 | 0 | 0 | 9 | 0 |  |  |  |  |  |  |
| noise | r1-base-1 | 1 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-epoll-spin-0 | r1-base-0 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 2 | 1 | 24 | 0 |
| pp-epoll-spin-0 | r1-base-1 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 2 | 1 | 15 | 0 |
| pp-epoll-spin-10000 | r1-base-0 | 1 | 0 | 0 | 0 | 9 | 0 | 0 | 1071 | 1 | 1 | 5 | 0 |
| pp-epoll-spin-10000 | r1-base-1 | 1 | 0 | 0 | 0 | 9 | 0 | 0 | 1072 | 1 | 0 | 9 | 0 |
| pp-recv-busy-0 | r1-base-0 | 1 | 180 | 179 | 0 | 17 | 0 | 0 | 1174 | 0 | 1 | 21 | 0 |
| pp-recv-busy-0 | r1-base-1 | 1 | 178 | 178 | 0 | 17 | 0 |  |  |  |  |  |  |
| pp-recv-busy-10000 | r1-base-0 | 1 | 94 | 94 | 0 | 9 | 0 |  |  |  |  |  |  |
| pp-recv-busy-10000 | r1-base-1 | 1 | 63 | 63 | 0 | 9 | 0 |  |  |  |  |  |  |
| pp-uring-sqpoll-0 | r1-base-0 | 1 | 0 | 0 | 0 | 17 | 0 | 13 | 310 | 153 | 0 | 17 | 0 |
| pp-uring-sqpoll-0 | r1-base-1 | 1 | 0 | 0 | 0 | 17 | 0 | 13 | 324 | 160 | 0 | 17 | 0 |
| pp-uring-sqpoll-10000 | r1-base-0 | 1 | 0 | 0 | 0 | 9 | 0 | 13 | 170 | 84 | 0 | 9 | 0 |
| pp-uring-sqpoll-10000 | r1-base-1 | 1 | 0 | 0 | 0 | 9 | 0 | 13 | 170 | 85 | 0 | 9 | 0 |
| pp-uring-sqpoll-sib-0 | r1-base-0 | 1 | 0 | 0 | 0 | 0 | 0 | 49 | 344 | 171 | 0 | 1 | 0 |
| pp-uring-sqpoll-sib-0 | r1-base-1 | 1 | 0 | 0 | 0 | 0 | 0 | 49 | 369 | 184 | 0 | 1 | 0 |
| pp-uring-sqpoll-sib-10000 | r1-base-0 | 1 | 0 | 0 | 0 | 0 | 0 | 49 | 163 | 81 | 0 | 0 | 0 |
| pp-uring-sqpoll-sib-10000 | r1-base-1 | 1 | 0 | 0 | 0 | 0 | 0 | 49 | 143 | 70 | 0 | 0 | 0 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### http-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 1151.51 at 65.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 787.75 at 5.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 320.60 at 5.1 s: neither loop stalled long enough: network, hypervisor or untraced; 307.74 at 65.1 s: neither loop stalled long enough: network, hypervisor or untraced; 256.36 at 65.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 127.66 at 115.3 s: neither loop stalled long enough: network, hypervisor or untraced; 126.66 at 96.1 s: neither loop stalled long enough: network, hypervisor or untraced; 119.58 at 17.7 s: neither loop stalled long enough: network, hypervisor or untraced; 117.46 at 102.7 s: neither loop stalled long enough: network, hypervisor or untraced; 113.11 at 91.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-epoll-spin-c1

| condition | worst events |
|---|---|
| m1 | 28.91 at 8.0 s: client: loop work (5 items); 28.74 at 8.0 s: client: loop work (5 items); 27.28 at 8.0 s: client: loop work (5 items); 27.10 at 8.0 s: client: loop work (5 items); 26.93 at 8.0 s: client: loop work (5 items) |
| m2 | 37.64 at 40.2 s: client: loop work (7 items); 37.48 at 40.2 s: client: loop work (7 items); 37.35 at 40.2 s: client: loop work (7 items); 37.23 at 10.8 s: neither loop stalled long enough: network, hypervisor or untraced; 37.21 at 40.2 s: client: loop work (7 items) |

### lab-epoll-spin-c10

| condition | worst events |
|---|---|
| m1 | 42.14 at 37.2 s: neither loop stalled long enough: network, hypervisor or untraced; 42.01 at 37.2 s: neither loop stalled long enough: network, hypervisor or untraced; 41.99 at 9.8 s: neither loop stalled long enough: network, hypervisor or untraced; 41.89 at 37.2 s: neither loop stalled long enough: network, hypervisor or untraced; 41.76 at 37.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 39.19 at 24.6 s: neither loop stalled long enough: network, hypervisor or untraced; 39.06 at 24.6 s: neither loop stalled long enough: network, hypervisor or untraced; 37.70 at 24.6 s: neither loop stalled long enough: network, hypervisor or untraced; 36.75 at 49.3 s: neither loop stalled long enough: network, hypervisor or untraced; 36.06 at 59.1 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-epoll-spin-c100

| condition | worst events |
|---|---|
| m1 | 44.41 at 20.1 s: neither loop stalled long enough: network, hypervisor or untraced; 44.29 at 20.1 s: neither loop stalled long enough: network, hypervisor or untraced; 44.16 at 20.1 s: neither loop stalled long enough: network, hypervisor or untraced; 44.04 at 20.1 s: neither loop stalled long enough: network, hypervisor or untraced; 43.91 at 20.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 40.80 at 9.4 s: neither loop stalled long enough: network, hypervisor or untraced; 40.65 at 9.4 s: neither loop stalled long enough: network, hypervisor or untraced; 39.85 at 51.3 s: neither loop stalled long enough: network, hypervisor or untraced; 39.70 at 51.3 s: neither loop stalled long enough: network, hypervisor or untraced; 39.50 at 29.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |

### lab-recv-busy-c1

| condition | worst events |
|---|---|
| m1 | 26.53 at 30.1 s: neither loop stalled long enough: network, hypervisor or untraced; 26.41 at 30.1 s: neither loop stalled long enough: network, hypervisor or untraced; 26.28 at 30.1 s: neither loop stalled long enough: network, hypervisor or untraced; 25.43 at 30.1 s: neither loop stalled long enough: network, hypervisor or untraced; 25.30 at 30.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 32.76 at 17.9 s: client: loop work (2 items); 32.59 at 17.9 s: client: loop work (2 items); 28.82 at 38.2 s: client: loop work (2 items); 28.65 at 38.2 s: client: loop work (2 items); 28.49 at 38.2 s: client: loop work (2 items) |

### lab-recv-busy-c10

| condition | worst events |
|---|---|
| m1 | 57.83 at 38.4 s: client: loop work (11 items); 57.70 at 38.4 s: client: loop work (11 items); 57.58 at 38.4 s: client: loop work (11 items); 57.45 at 38.4 s: client: loop work (11 items); 57.33 at 38.4 s: client: loop work (11 items) |
| m2 | 62.92 at 2.8 s: client: loop work (19 items); 62.80 at 2.8 s: client: loop work (19 items); 62.67 at 2.8 s: client: loop work (19 items); 62.55 at 2.8 s: client: loop work (19 items); 62.42 at 2.8 s: client: loop work (19 items) |

### lab-recv-busy-c100

| condition | worst events |
|---|---|
| m1 | 134.14 at 54.8 s: neither loop stalled long enough: network, hypervisor or untraced; 119.63 at 18.5 s: client: loop work (49 items); 119.51 at 18.5 s: client: loop work (49 items); 119.38 at 18.5 s: client: loop work (49 items); 119.26 at 18.5 s: client: loop work (49 items) |
| m2 | 127.91 at 12.1 s: client: loop work (97 items); 127.79 at 12.1 s: client: loop work (97 items); 127.67 at 12.1 s: client: loop work (97 items); 127.54 at 12.1 s: client: loop work (97 items); 127.41 at 12.1 s: client: loop work (97 items) |

### lab-recv-busy-pq-c1

| condition | worst events |
|---|---|
| m1 | 35.61 at 56.6 s: neither loop stalled long enough: network, hypervisor or untraced; 35.48 at 56.6 s: neither loop stalled long enough: network, hypervisor or untraced; 35.33 at 56.6 s: neither loop stalled long enough: network, hypervisor or untraced; 34.07 at 12.5 s: neither loop stalled long enough: network, hypervisor or untraced; 33.95 at 12.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 33.64 at 36.4 s: client: loop work (3 items); 33.52 at 36.4 s: client: loop work (3 items); 33.37 at 36.4 s: client: loop work (3 items); 20.20 at 23.9 s: neither loop stalled long enough: network, hypervisor or untraced; 20.08 at 23.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-recv-busy-pq-c10

| condition | worst events |
|---|---|
| m1 | 56.51 at 53.8 s: client: loop work (26 items); 56.38 at 53.8 s: client: loop work (26 items); 56.25 at 53.8 s: client: loop work (26 items); 56.12 at 53.8 s: client: loop work (26 items); 55.99 at 53.8 s: client: loop work (26 items) |
| m2 | 58.10 at 6.9 s: client: loop work (43 items); 57.98 at 6.9 s: client: loop work (43 items); 57.85 at 6.9 s: client: loop work (43 items); 57.72 at 6.9 s: client: loop work (43 items); 52.24 at 51.2 s: client: loop work (24 items) |

### lab-recv-busy-pq-c100

| condition | worst events |
|---|---|
| m1 | 70.66 at 2.5 s: client: loop work (70 items); 70.53 at 2.5 s: client: loop work (70 items); 70.41 at 2.5 s: client: loop work (70 items); 70.28 at 2.5 s: client: loop work (70 items); 70.14 at 2.5 s: client: loop work (70 items) |
| m2 | 57.16 at 20.6 s: client: loop work (47 items); 56.99 at 20.6 s: client: loop work (47 items); 56.84 at 20.6 s: client: loop work (47 items); 56.70 at 20.6 s: client: loop work (47 items); 56.56 at 20.6 s: client: loop work (47 items) |

### lab-uring-sqpoll-c1

| condition | worst events |
|---|---|
| m1 | 26.37 at 21.0 s: neither loop stalled long enough: network, hypervisor or untraced; 26.27 at 34.2 s: neither loop stalled long enough: network, hypervisor or untraced; 26.21 at 21.0 s: neither loop stalled long enough: network, hypervisor or untraced; 26.14 at 34.2 s: neither loop stalled long enough: network, hypervisor or untraced; 25.98 at 21.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 32.91 at 48.5 s: neither loop stalled long enough: network, hypervisor or untraced; 32.79 at 48.5 s: neither loop stalled long enough: network, hypervisor or untraced; 32.66 at 48.5 s: neither loop stalled long enough: network, hypervisor or untraced; 32.54 at 48.5 s: neither loop stalled long enough: network, hypervisor or untraced; 32.41 at 48.5 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-c10

| condition | worst events |
|---|---|
| m1 | 48.04 at 14.8 s: neither loop stalled long enough: network, hypervisor or untraced; 47.91 at 14.8 s: neither loop stalled long enough: network, hypervisor or untraced; 47.79 at 14.8 s: neither loop stalled long enough: network, hypervisor or untraced; 47.66 at 14.8 s: neither loop stalled long enough: network, hypervisor or untraced; 47.54 at 14.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 49.70 at 9.8 s: neither loop stalled long enough: network, hypervisor or untraced; 49.52 at 9.8 s: neither loop stalled long enough: network, hypervisor or untraced; 49.38 at 9.8 s: neither loop stalled long enough: network, hypervisor or untraced; 49.25 at 9.8 s: neither loop stalled long enough: network, hypervisor or untraced; 49.10 at 9.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-c100

| condition | worst events |
|---|---|
| m1 | 55.77 at 25.2 s: neither loop stalled long enough: network, hypervisor or untraced; 55.64 at 25.2 s: neither loop stalled long enough: network, hypervisor or untraced; 55.41 at 25.2 s: neither loop stalled long enough: network, hypervisor or untraced; 55.29 at 25.2 s: neither loop stalled long enough: network, hypervisor or untraced; 55.16 at 25.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 63.00 at 19.6 s: neither loop stalled long enough: network, hypervisor or untraced; 62.88 at 19.6 s: neither loop stalled long enough: network, hypervisor or untraced; 62.75 at 19.6 s: neither loop stalled long enough: network, hypervisor or untraced; 62.63 at 19.6 s: neither loop stalled long enough: network, hypervisor or untraced; 62.50 at 19.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-sib-c1

| condition | worst events |
|---|---|
| m1 | 34.18 at 10.8 s: neither loop stalled long enough: network, hypervisor or untraced; 34.16 at 28.9 s: neither loop stalled long enough: network, hypervisor or untraced; 34.00 at 28.9 s: neither loop stalled long enough: network, hypervisor or untraced; 34.00 at 46.7 s: neither loop stalled long enough: network, hypervisor or untraced; 33.87 at 46.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 35.00 at 59.3 s: neither loop stalled long enough: network, hypervisor or untraced; 33.82 at 41.3 s: neither loop stalled long enough: network, hypervisor or untraced; 33.63 at 41.3 s: neither loop stalled long enough: network, hypervisor or untraced; 33.38 at 41.3 s: neither loop stalled long enough: network, hypervisor or untraced; 33.20 at 41.3 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-sib-c10

| condition | worst events |
|---|---|
| m1 | 83.77 at 7.4 s: neither loop stalled long enough: network, hypervisor or untraced; 83.62 at 7.4 s: neither loop stalled long enough: network, hypervisor or untraced; 83.48 at 7.4 s: neither loop stalled long enough: network, hypervisor or untraced; 83.34 at 7.4 s: neither loop stalled long enough: network, hypervisor or untraced; 83.21 at 7.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 68.91 at 56.9 s: neither loop stalled long enough: network, hypervisor or untraced; 68.75 at 56.9 s: neither loop stalled long enough: network, hypervisor or untraced; 68.60 at 56.9 s: neither loop stalled long enough: network, hypervisor or untraced; 68.43 at 56.9 s: neither loop stalled long enough: network, hypervisor or untraced; 68.35 at 31.7 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-sib-c100

| condition | worst events |
|---|---|
| m1 | 92.36 at 42.2 s: neither loop stalled long enough: network, hypervisor or untraced; 92.22 at 42.2 s: neither loop stalled long enough: network, hypervisor or untraced; 92.07 at 42.2 s: neither loop stalled long enough: network, hypervisor or untraced; 91.92 at 42.2 s: neither loop stalled long enough: network, hypervisor or untraced; 91.78 at 42.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 100.84 at 53.5 s: neither loop stalled long enough: network, hypervisor or untraced; 100.66 at 53.5 s: neither loop stalled long enough: network, hypervisor or untraced; 100.50 at 53.5 s: neither loop stalled long enough: network, hypervisor or untraced; 100.34 at 53.5 s: neither loop stalled long enough: network, hypervisor or untraced; 100.19 at 53.5 s: neither loop stalled long enough: network, hypervisor or untraced |

### md-recv-busy

| condition | worst events |
|---|---|
| r1-base-0 | 34.00 at 29.1 s: client: loop work (4 items); 33.87 at 29.1 s: client: loop work (4 items); 33.74 at 29.1 s: client: loop work (4 items); 33.57 at 29.1 s: client: loop work (4 items); 33.25 at 18.7 s: client: loop work (4 items) |
| r1-base-1 | 19.03 at 55.2 s: neither loop stalled long enough: network, hypervisor or untraced; 18.33 at 55.2 s: neither loop stalled long enough: network, hypervisor or untraced; 18.20 at 55.2 s: neither loop stalled long enough: network, hypervisor or untraced; 18.09 at 30.3 s: neither loop stalled long enough: network, hypervisor or untraced; 18.07 at 55.2 s: neither loop stalled long enough: network, hypervisor or untraced |

### noise

| condition | worst events |
|---|---|
| r1-base-0 | 26.93 at 0.0 s: mostly unexplained (+softirq TIMER) |
| r1-base-1 | 29.48 at 30.1 s: mostly unexplained (+local timer tick); 6.36 at 0.0 s: mostly unexplained (+call function (IPI/vector)) |

### pp-epoll-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 379.65 at 34.7 s: neither loop stalled long enough: network, hypervisor or untraced; 313.39 at 34.7 s: neither loop stalled long enough: network, hypervisor or untraced; 272.03 at 34.7 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 219.79 at 34.7 s: neither loop stalled long enough: network, hypervisor or untraced; 212.72 at 37.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 193.31 at 28.6 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 181.60 at 28.6 s: neither loop stalled long enough: network, hypervisor or untraced; 169.15 at 28.6 s: neither loop stalled long enough: network, hypervisor or untraced; 159.52 at 2.1 s: neither loop stalled long enough: network, hypervisor or untraced; 156.62 at 25.1 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-epoll-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 927.82 at 67.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 686.38 at 7.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 484.86 at 7.1 s: neither loop stalled long enough: network, hypervisor or untraced; 331.57 at 67.1 s: neither loop stalled long enough: network, hypervisor or untraced; 327.38 at 7.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 148.68 at 47.0 s: server: softirq TIMER; 121.64 at 103.3 s: neither loop stalled long enough: network, hypervisor or untraced; 117.42 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced; 114.81 at 6.2 s: neither loop stalled long enough: network, hypervisor or untraced; 111.73 at 7.2 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-0

| condition | worst events |
|---|---|
| r1-base-0 | 206.07 at 53.8 s: server: softirq TIMER; 148.01 at 39.1 s: neither loop stalled long enough: network, hypervisor or untraced; 130.02 at 3.5 s: neither loop stalled long enough: network, hypervisor or untraced; 127.18 at 20.8 s: neither loop stalled long enough: network, hypervisor or untraced; 125.11 at 53.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 159.93 at 6.6 s: neither loop stalled long enough: network, hypervisor or untraced; 149.59 at 46.6 s: neither loop stalled long enough: network, hypervisor or untraced; 140.08 at 42.7 s: neither loop stalled long enough: network, hypervisor or untraced; 134.38 at 26.7 s: neither loop stalled long enough: network, hypervisor or untraced; 133.25 at 29.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 112.86 at 9.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 111.14 at 63.6 s: neither loop stalled long enough: network, hypervisor or untraced; 110.44 at 52.6 s: neither loop stalled long enough: network, hypervisor or untraced; 107.50 at 52.6 s: neither loop stalled long enough: network, hypervisor or untraced; 107.34 at 115.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 962.59 at 52.2 s: neither loop stalled long enough: network, hypervisor or untraced; 746.84 at 23.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 454.27 at 83.2 s: neither loop stalled long enough: network, hypervisor or untraced; 402.79 at 83.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 245.24 at 23.2 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-0

| condition | worst events |
|---|---|
| r1-base-0 | 429.02 at 44.6 s: neither loop stalled long enough: network, hypervisor or untraced; 390.83 at 44.6 s: neither loop stalled long enough: network, hypervisor or untraced; 184.53 at 44.6 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 182.01 at 44.6 s: neither loop stalled long enough: network, hypervisor or untraced; 174.61 at 44.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 331.87 at 39.7 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 267.50 at 39.7 s: neither loop stalled long enough: network, hypervisor or untraced; 196.62 at 39.7 s: neither loop stalled long enough: network, hypervisor or untraced; 174.24 at 39.7 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 171.80 at 58.3 s: server: softirq TIMER |

### pp-uring-sqpoll-10000

| condition | worst events |
|---|---|
| r1-base-0 | 169.22 at 65.4 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 135.26 at 47.2 s: neither loop stalled long enough: network, hypervisor or untraced; 128.83 at 117.9 s: neither loop stalled long enough: network, hypervisor or untraced; 126.41 at 3.9 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 124.21 at 104.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 851.80 at 74.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 598.98 at 14.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 430.62 at 14.3 s: neither loop stalled long enough: network, hypervisor or untraced; 292.90 at 74.3 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 255.53 at 14.2 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-sib-0

| condition | worst events |
|---|---|
| r1-base-0 | 859.17 at 40.7 s: neither loop stalled long enough: network, hypervisor or untraced; 429.26 at 36.6 s: neither loop stalled long enough: network, hypervisor or untraced; 283.87 at 36.6 s: neither loop stalled long enough: network, hypervisor or untraced; 133.05 at 3.7 s: neither loop stalled long enough: network, hypervisor or untraced; 128.68 at 35.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 312.20 at 29.9 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 195.07 at 30.0 s: neither loop stalled long enough: network, hypervisor or untraced; 165.31 at 29.9 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 140.35 at 57.7 s: neither loop stalled long enough: network, hypervisor or untraced; 137.59 at 5.4 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-sib-10000

| condition | worst events |
|---|---|
| r1-base-0 | 765.76 at 9.5 s: neither loop stalled long enough: network, hypervisor or untraced; 547.75 at 69.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 267.42 at 37.4 s: server: softirq TIMER; 241.01 at 9.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 199.69 at 9.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-1 | 881.04 at 63.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 699.90 at 3.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 302.46 at 3.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 208.74 at 63.1 s: neither loop stalled long enough: network, hypervisor or untraced; 147.40 at 63.1 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| r1-base-0 | loop work 147797.94; unexplained: no kernel event (hypervisor or in-kernel time) 116682.19; mostly unexplained (+softirq NET_RX) 31648.00; softirq NET_RX 1172.51; mostly unexplained (+irq work (IPI/vector)) 1161.79 |
| r1-base-1 | unexplained: no kernel event (hypervisor or in-kernel time) 130777.49; loop work 123770.21; mostly unexplained (+softirq NET_RX) 37763.98; softirq NET_RX 1709.35; mostly unexplained (+irq work (IPI/vector)) 807.23 |
| m1 | unexplained: no kernel event (hypervisor or in-kernel time) 38510094.69; loop work 13259007.06; mostly unexplained (+softirq NET_RX) 575302.71; mostly unexplained (+local timer tick) 157374.33; mostly unexplained (+irq work (IPI/vector)) 118946.09 |
| m2 | unexplained: no kernel event (hypervisor or in-kernel time) 38488930.57; loop work 13099239.84; mostly unexplained (+softirq NET_RX) 586543.29; mostly unexplained (+local timer tick) 156663.77; mostly unexplained (+irq work (IPI/vector)) 114731.86 |

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
| cpumap-receiver/cpu16 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu17 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu18 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu19 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu2 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu20 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu21 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu22 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu23 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu24 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu25 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu26 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu27 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu28 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu29 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu3 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu30 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu31 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu32 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu33 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu34 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu35 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu36 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu37 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu38 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu39 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu4 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu40 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu41 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu42 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu43 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu44 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu45 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu46 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu47 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu49 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu5 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu50 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu51 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu52 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu53 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu54 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu55 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu56 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu57 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu58 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu59 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu6 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu60 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu61 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu62 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu63 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu64 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu65 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu66 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu67 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu68 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu69 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu7 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu70 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu71 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu72 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu73 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu74 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu75 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu76 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu77 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu78 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu79 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu8 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu80 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu81 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu82 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu83 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu84 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu85 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu86 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu87 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu88 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu89 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu9 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu90 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu91 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu92 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu93 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu94 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu95 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu1 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu3 | counter-only spinner | 0 | | | | | |
| http-recv-busy-10000 | idle spare core | 0 | | | | | |
| http-recv-busy-10000 | receiver loop | 0 | | | | | |
| http-recv-busy-10000 | sender loop | 2 | :35 | 2 (100%) | 2 of 5 | 655.06 | no |
| http-recv-busy-10000 | slow samples | 4 | :35 | 4 (100%) | 2 of 5 | 1151.51 | no |
| lab-epoll-spin-c1 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c1 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c1 | slow samples | 0 | | | | | |
| lab-epoll-spin-c10 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c10 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c10 | slow samples | 0 | | | | | |
| lab-epoll-spin-c100 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c100 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c100 | slow samples | 0 | | | | | |
| lab-recv-busy-c1 | idle spare core | 0 | | | | | |
| lab-recv-busy-c1 | receiver loop | 0 | | | | | |
| lab-recv-busy-c1 | slow samples | 0 | | | | | |
| lab-recv-busy-c10 | idle spare core | 0 | | | | | |
| lab-recv-busy-c10 | receiver loop | 0 | | | | | |
| lab-recv-busy-c10 | slow samples | 0 | | | | | |
| lab-recv-busy-c100 | idle spare core | 0 | | | | | |
| lab-recv-busy-c100 | receiver loop | 0 | | | | | |
| lab-recv-busy-c100 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c1 | idle spare core | 0 | | | | | |
| lab-recv-busy-pq-c1 | receiver loop | 0 | | | | | |
| lab-recv-busy-pq-c1 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c10 | idle spare core | 0 | | | | | |
| lab-recv-busy-pq-c10 | receiver loop | 0 | | | | | |
| lab-recv-busy-pq-c10 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c100 | idle spare core | 0 | | | | | |
| lab-recv-busy-pq-c100 | receiver loop | 0 | | | | | |
| lab-recv-busy-pq-c100 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c1 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-c1 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-c1 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c10 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-c10 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-c10 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c100 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-c100 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-c100 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-sib-c1 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-sib-c1 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-sib-c1 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-sib-c10 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-sib-c10 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-sib-c10 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-sib-c100 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-sib-c100 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-sib-c100 | slow samples | 0 | | | | | |
| md-recv-busy | idle spare core | 0 | | | | | |
| md-recv-busy | receiver loop | 0 | | | | | |
| md-recv-busy | slow samples | 0 | | | | | |
| noise | noise probe (loop CPU) | 0 | | | | | |
| pp-epoll-spin-0 | idle spare core | 0 | | | | | |
| pp-epoll-spin-0 | receiver loop | 0 | | | | | |
| pp-epoll-spin-0 | sender loop | 0 | | | | | |
| pp-epoll-spin-0 | slow samples | 2 | :35 | 2 (100%) | 1 of 2 | 379.65 | no |
| pp-epoll-spin-10000 | idle spare core | 0 | | | | | |
| pp-epoll-spin-10000 | receiver loop | 0 | | | | | |
| pp-epoll-spin-10000 | sender loop | 2 | :35 | 2 (100%) | 2 of 5 | 664.64 | no |
| pp-epoll-spin-10000 | slow samples | 6 | :35 | 6 (100%) | 2 of 5 | 927.82 | **yes** |
| pp-recv-busy-0 | idle spare core | 0 | | | | | |
| pp-recv-busy-0 | receiver loop | 0 | | | | | |
| pp-recv-busy-0 | sender loop | 0 | | | | | |
| pp-recv-busy-0 | slow samples | 0 | | | | | |
| pp-recv-busy-10000 | idle spare core | 0 | | | | | |
| pp-recv-busy-10000 | receiver loop | 0 | | | | | |
| pp-recv-busy-10000 | sender loop | 2 | :35 | 2 (100%) | 2 of 4 | 425.19 | no |
| pp-recv-busy-10000 | slow samples | 4 | :35 | 3 (75%) | 2 of 4 | 746.84 | no |
| pp-uring-sqpoll-0 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-0 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-0 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-0 | slow samples | 3 | :35 | 3 (100%) | 2 of 2 | 429.02 | no |
| pp-uring-sqpoll-10000 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-10000 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-10000 | sender loop | 2 | :35 | 2 (100%) | 2 of 4 | 656.29 | no |
| pp-uring-sqpoll-10000 | slow samples | 3 | :35 | 3 (100%) | 2 of 4 | 851.80 | no |
| pp-uring-sqpoll-sib-0 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-sib-0 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-sib-0 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-sib-0 | slow samples | 3 | :35 | 2 (67%) | 2 of 2 | 429.26 | no |
| pp-uring-sqpoll-sib-10000 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-sib-10000 | receiver loop | 0 | | | | | |
| pp-uring-sqpoll-sib-10000 | sender loop | 2 | :35 | 2 (100%) | 2 of 4 | 514.78 | no |
| pp-uring-sqpoll-sib-10000 | slow samples | 5 | :35 | 5 (100%) | 4 of 4 | 881.04 | **yes** |

# Every run in detail

#### Stalls: r1-base-0/http-recv-busy-10000, loop 0 on CPU 1

3783652215 iterations, longest 53.2 µs; over 5 / 20 / 100 µs: 265005 / 5390 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4170 | 116067.2 | 53.2 |
| mostly unexplained (+softirq NET_RX) | 1118 | 31606.4 | 41.8 |
| mostly unexplained (+irq work (IPI/vector)) | 38 | 1138.9 | 41.8 |
| mostly unexplained (+local timer tick) | 36 | 1099.5 | 50.7 |
| softirq NET_RX | 14 | 390.4 | 36.4 |
| mostly unexplained (+preempted by kworker/1:1) | 12 | 343.3 | 41.3 |
| softirq TIMER | 2 | 73.1 | 37.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 53.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 52.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 52.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 51.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 50.7 | 1 | mostly unexplained (+local timer tick) | local timer tick 1, softirq NET_RX 1, softirq TIMER 1, timer tick_nohz_handler 1 |
| 6 | 48.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 46.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 46.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 45.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 44.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 7784 |
| IWI IRQ work interrupts | 7399 |
| CAL Function call interrupts | 1022 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/http-recv-busy-10000 (median 39.63)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1151.51 | 1111.87 | 0.00 |  | 796.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 314.97 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 787.75 | 748.12 | 0.00 |  | 616.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 131.89 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 320.60 | 280.97 | 0.00 |  | 65.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 215.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 307.74 | 268.11 | 0.00 |  | 0.00 |  | 268.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 256.36 | 216.73 | 0.00 |  | 85.59 | unexplained: no kernel event (hypervisor or in-kernel time) | 131.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 253.57 | 213.94 | 0.00 |  | 73.45 | page faults | 140.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 247.96 | 208.33 | 0.00 |  | 143.94 | unexplained: no kernel event (hypervisor or in-kernel time) | 64.39 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 231.60 | 191.96 | 0.00 |  | 55.06 | unexplained: no kernel event (hypervisor or in-kernel time) | 136.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 195.48 | 155.85 | 0.00 |  | 51.45 | unexplained: no kernel event (hypervisor or in-kernel time) | 104.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 173.91 | 134.28 | 0.00 |  | 0.00 |  | 134.28 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-0/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=666192 clocksource=tsc

### Loop counters (r1-base-0/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002650 | 779234 | 3.85 | 127272320 | 42.39 | 2121205 | 2121205 | 99.4 | 68.6 | true | 0 | 5820 | 468.0 | 87903 / 5962 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672593600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (144530 of 2900000 in 1-msg reads) | 144530 | 2.52 | 2.38 | 2.78 | 6.34 | 10.03 | 14.00 | 32.84 |

#### Stalls: r1-base-0/md-recv-busy, loop 0 on CPU 1

127272320 iterations, longest 468.0 µs; over 5 / 20 / 100 µs: 87903 / 5962 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5902 | 147797.9 | 468.0 |
| softirq NET_RX | 33 | 782.1 | 34.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 20 | 475.7 | 32.5 |
| softirq TIMER | 5 | 114.4 | 27.5 |
| mostly unexplained (+softirq NET_RX) | 2 | 41.6 | 21.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 468.0 | 2650 | loop work (2650 items) |  |
| 2 | 62.1 | 21 | loop work (21 items) |  |
| 3 | 56.5 | 24 | loop work (24 items) | softirq NET_RX 11, softirq NET_TX 1 |
| 4 | 56.4 | 21 | loop work (21 items) |  |
| 5 | 49.7 | 6 | loop work (6 items) |  |
| 6 | 49.2 | 6 | loop work (6 items) |  |
| 7 | 45.8 | 32 | loop work (32 items) | softirq NET_TX 1 |
| 8 | 45.2 | 18 | loop work (18 items) |  |
| 9 | 45.0 | 17 | loop work (17 items) |  |
| 10 | 44.8 | 18 | loop work (18 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 10831 |
| LOC Local timer interrupts | 6837 |
| CAL Function call interrupts | 1014 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-0/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-0/noise, loop 0 on CPU 1

17515573248 iterations, longest 26.9 µs; over 5 / 20 / 100 µs: 1 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+softirq TIMER) | 1 | 26.9 | 26.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 26.9 | 0 | mostly unexplained (+softirq TIMER) | softirq TIMER 12, local timer tick 8, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1021 |
| LOC Local timer interrupts | 7 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-base-0/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1424604 | 1424604 | 1.00 | 397521209 | 279.04 | 6625353 | 6577867 | 0.0 | 35.0 | false | 0 | 0 | 33.7 | 6048 / 2 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-0, loop 0 on CPU 1

394672000 iterations, longest 33.7 µs; over 5 / 20 / 100 µs: 6048 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 66.5 | 33.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 33.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 32.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1012 |
| LOC Local timer interrupts | 18 |
| IWI IRQ work interrupts | 7 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-0 (median 41.06)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 379.65 | 338.58 | 0.00 |  | 142.03 | unexplained: no kernel event (hypervisor or in-kernel time) | 196.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 313.39 | 272.33 | 0.00 |  | 0.00 |  | 272.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 272.03 | 230.96 | 0.00 |  | 160.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 70.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 219.79 | 178.72 | 0.00 |  | 58.70 | unexplained: no kernel event (hypervisor or in-kernel time) | 120.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 212.72 | 171.66 | 0.00 |  | 0.00 |  | 171.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 191.10 | 150.03 | 0.00 |  | 0.00 |  | 150.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 187.05 | 145.98 | 0.00 |  | 0.00 |  | 145.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 181.94 | 140.87 | 0.00 |  | 76.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 64.61 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 171.73 | 130.66 | 0.00 |  | 0.00 |  | 130.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 171.72 | 130.65 | 0.00 |  | 0.00 |  | 130.65 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 800729664 | 667.27 | 6672747 | 6652747 | 0.0 | 29.7 | false | 0 | 0 | 33.4 | 1841 / 1 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-10000, loop 0 on CPU 1

798329664 iterations, longest 33.4 µs; over 5 / 20 / 100 µs: 1841 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 33.4 | 33.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 33.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1021 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-10000 (median 39.96)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 927.82 | 887.86 | 0.00 |  | 731.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 155.89 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 686.38 | 646.42 | 0.00 |  | 597.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 48.70 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 484.86 | 444.90 | 0.00 |  | 79.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 365.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 331.57 | 291.60 | 0.00 |  | 0.00 |  | 291.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 327.38 | 287.42 | 0.00 |  | 87.96 | unexplained: no kernel event (hypervisor or in-kernel time) | 199.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 311.07 | 271.11 | 0.00 |  | 65.94 | unexplained: no kernel event (hypervisor or in-kernel time) | 205.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 227.93 | 187.97 | 0.00 |  | 66.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 121.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 225.08 | 185.12 | 0.00 |  | 76.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 108.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 158.38 | 118.42 | 0.00 |  | 66.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 52.29 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 154.08 | 114.12 | 0.00 |  | 65.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 48.26 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-0/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1678622 | 1678622 | 1.00 | 120780255 | 71.95 | 2013004 | 1985027 | 98.6 | 74.3 | true | 0 | 10710 | 22.9 | 94990 / 1 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-0, loop 0 on CPU 1

119101632 iterations, longest 22.9 µs; over 5 / 20 / 100 µs: 94990 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+irq work (IPI/vector)) | 1 | 22.9 | 22.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 22.9 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 10811 |
| IWI IRQ work interrupts | 10716 |
| CAL Function call interrupts | 1012 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-0/pp-recv-busy-0 (median 35.28)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 206.07 | 170.79 | 0.00 |  | 184.45 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 148.01 | 112.73 | 0.00 |  | 0.00 |  | 112.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 130.02 | 94.74 | 0.00 |  | 0.00 |  | 94.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 127.18 | 91.90 | 0.00 |  | 0.00 |  | 91.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 125.11 | 89.83 | 0.00 |  | 0.00 |  | 89.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 124.95 | 89.67 | 0.00 |  | 0.00 |  | 89.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 122.00 | 86.72 | 0.00 |  | 93.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 120.19 | 84.91 | 0.00 |  | 0.00 |  | 84.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 119.79 | 84.51 | 0.00 |  | 0.00 |  | 84.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 116.27 | 80.99 | 0.00 |  | 0.00 |  | 80.99 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 265434176 | 221.20 | 2211951 | 2201951 | 99.5 | 70.3 | true | 0 | 11244 | 39.3 | 53530 / 1 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-10000, loop 0 on CPU 1

264234176 iterations, longest 39.3 µs; over 5 / 20 / 100 µs: 53530 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 39.3 | 39.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 39.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 11315 |
| IWI IRQ work interrupts | 11250 |
| CAL Function call interrupts | 1021 |

#### Slowest round trips: r1-base-0/pp-recv-busy-10000 (median 37.05)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 112.86 | 75.80 | 0.00 |  | 80.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 111.14 | 74.09 | 0.00 |  | 0.00 |  | 74.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 110.44 | 73.39 | 0.00 |  | 0.00 |  | 73.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 107.50 | 70.44 | 0.00 |  | 0.00 |  | 70.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 107.34 | 70.29 | 0.00 |  | 0.00 |  | 70.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 101.54 | 64.49 | 0.00 |  | 0.00 |  | 64.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 95.84 | 58.79 | 0.00 |  | 0.00 |  | 58.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 92.76 | 55.71 | 0.00 |  | 0.00 |  | 55.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 90.80 | 53.74 | 0.00 |  | 0.00 |  | 53.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 84.86 | 47.81 | 0.00 |  | 0.00 |  | 47.81 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1474779 | 1474779 | 1.00 | 1 | 0.00 | 0 | 68397372 | 0.0 | 0.0 | true | 0 | 0 | 4.9 | 0 / 0 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-0, loop 0 on CPU 1

4103842304 iterations, longest 4.9 µs; over 5 / 20 / 100 µs: 0 / 0 / 0

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1015 |
| LOC Local timer interrupts | 7 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-0 (median 39.68)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 429.02 | 389.34 | 0.00 |  | 57.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 331.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 390.83 | 351.15 | 0.00 |  | 136.42 | unexplained: no kernel event (hypervisor or in-kernel time) | 214.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 184.53 | 144.86 | 0.00 |  | 76.74 | unexplained: no kernel event (hypervisor or in-kernel time) | 68.12 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 182.01 | 142.34 | 0.00 |  | 0.00 |  | 142.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 174.61 | 134.94 | 0.00 |  | 0.00 |  | 134.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 168.67 | 129.00 | 0.00 |  | 0.00 |  | 129.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 148.86 | 109.19 | 0.00 |  | 0.00 |  | 109.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 148.55 | 108.87 | 0.00 |  | 0.00 |  | 108.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 147.26 | 107.59 | 0.00 |  | 0.00 |  | 107.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 145.72 | 106.05 | 0.00 |  | 0.00 |  | 106.05 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 44646826 | 0.0 | 0.0 | true | 0 | 0 | 16.6 | 2 / 0 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-10000, loop 0 on CPU 1

5357619072 iterations, longest 16.6 µs; over 5 / 20 / 100 µs: 2 / 0 / 0

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1025 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-10000 (median 38.15)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 169.22 | 131.07 | 0.00 |  | 170.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 135.26 | 97.12 | 0.00 |  | 0.00 |  | 97.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 128.83 | 90.68 | 0.00 |  | 0.00 |  | 90.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 126.41 | 88.26 | 0.00 |  | 121.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 124.21 | 86.06 | 0.00 |  | 0.00 |  | 86.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 120.01 | 81.86 | 0.00 |  | 0.00 |  | 81.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 114.75 | 76.61 | 0.00 |  | 0.00 |  | 76.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 113.59 | 75.44 | 0.00 |  | 0.00 |  | 75.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 112.19 | 74.04 | 0.00 |  | 0.00 |  | 74.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 111.69 | 73.55 | 0.00 |  | 0.00 |  | 73.55 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-sib-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1542190 | 1542190 | 1.00 | 1 | 0.00 | 0 | 52445822 | 0.0 | 0.0 | true | 0 | 0 | 5.3 | 1 / 0 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-sib-0, loop 0 on CPU 1

3146749312 iterations, longest 5.3 µs; over 5 / 20 / 100 µs: 1 / 0 / 0

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 13 |
| LOC Local timer interrupts | 7 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-sib-0 (median 38.38)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 859.17 | 820.79 | 0.00 |  | 0.00 |  | 820.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 429.26 | 390.88 | 0.00 |  | 0.00 |  | 390.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 283.87 | 245.49 | 0.00 |  | 0.00 |  | 245.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 133.05 | 94.68 | 0.00 |  | 0.00 |  | 94.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 128.68 | 90.30 | 0.00 |  | 0.00 |  | 90.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 120.00 | 81.62 | 0.00 |  | 0.00 |  | 81.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 118.47 | 80.10 | 0.00 |  | 0.00 |  | 80.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 109.63 | 71.25 | 0.00 |  | 0.00 |  | 71.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 103.38 | 65.00 | 0.00 |  | 0.00 |  | 65.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 102.43 | 64.05 | 0.00 |  | 0.00 |  | 64.05 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-sib-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 39969864 | 0.0 | 0.0 | true | 0 | 0 | 5.4 | 2 / 0 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-sib-10000, loop 0 on CPU 1

4796383680 iterations, longest 5.4 µs; over 5 / 20 / 100 µs: 2 / 0 / 0

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 22 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-sib-10000 (median 40.95)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 765.76 | 724.81 | 0.00 |  | 294.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 430.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 547.75 | 506.80 | 0.00 |  | 373.03 | unexplained: no kernel event (hypervisor or in-kernel time) | 133.78 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 267.42 | 226.47 | 0.00 |  | 231.53 | softirq TIMER | 0.00 | server: softirq TIMER |
| 4 | 241.01 | 200.06 | 0.00 |  | 129.87 | unexplained: no kernel event (hypervisor or in-kernel time) | 70.19 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 199.69 | 158.74 | 0.00 |  | 60.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 98.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 166.12 | 125.17 | 0.00 |  | 0.00 |  | 125.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 154.05 | 113.10 | 0.00 |  | 0.00 |  | 113.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 149.28 | 108.33 | 0.00 |  | 0.00 |  | 108.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 147.40 | 106.45 | 0.00 |  | 0.00 |  | 106.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 147.29 | 106.35 | 0.00 |  | 0.00 |  | 106.35 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-1/http-recv-busy-10000, loop 0 on CPU 1

3804160424 iterations, longest 52.9 µs; over 5 / 20 / 100 µs: 277848 / 5839 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4514 | 129779.0 | 52.6 |
| mostly unexplained (+softirq NET_RX) | 1271 | 37656.2 | 52.9 |
| mostly unexplained (+irq work (IPI/vector)) | 25 | 784.3 | 39.5 |
| softirq NET_RX | 21 | 602.2 | 42.8 |
| mostly unexplained (+preempted by kworker/1:1) | 6 | 199.5 | 41.7 |
| mostly unexplained (+local timer tick) | 1 | 34.6 | 34.6 |
| loop work | 1 | 24.2 | 24.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 52.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 52.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 49.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 49.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 48.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 47.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 45.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 44.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 43.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 3 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 7651 |
| IWI IRQ work interrupts | 7629 |
| CAL Function call interrupts | 1022 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/http-recv-busy-10000 (median 39.58)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 127.66 | 88.07 | 0.00 |  | 0.00 |  | 88.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 126.66 | 87.07 | 0.00 |  | 0.00 |  | 87.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 119.58 | 80.00 | 0.00 |  | 0.00 |  | 80.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 117.46 | 77.88 | 0.00 |  | 0.00 |  | 77.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 113.11 | 73.53 | 0.00 |  | 0.00 |  | 73.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 113.00 | 73.42 | 0.00 |  | 0.00 |  | 73.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 110.22 | 70.64 | 0.00 |  | 0.00 |  | 70.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 110.05 | 70.47 | 0.00 |  | 0.00 |  | 70.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 109.01 | 69.43 | 0.00 |  | 0.00 |  | 69.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 108.42 | 68.84 | 0.00 |  | 0.00 |  | 68.84 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-1/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=556484 clocksource=tsc

### Loop counters (r1-base-1/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002600 | 644966 | 4.66 | 127966848 | 42.62 | 2132781 | 2132781 | 99.5 | 68.3 | true | 0 | 5165 | 446.4 | 114264 / 4977 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672582400 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (102510 of 2899983 in 1-msg reads) | 102510 | 2.63 | 2.54 | 2.97 | 5.92 | 8.99 | 14.21 | 19.03 |

#### Stalls: r1-base-1/md-recv-busy, loop 0 on CPU 1

127966848 iterations, longest 446.4 µs; over 5 / 20 / 100 µs: 114264 / 4977 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4887 | 123746.0 | 446.4 |
| softirq NET_RX | 46 | 1107.1 | 32.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 36 | 798.9 | 26.6 |
| mostly unexplained (+softirq NET_RX) | 5 | 107.8 | 25.9 |
| softirq TIMER | 2 | 41.4 | 21.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 22.9 | 22.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 446.4 | 2600 | loop work (2600 items) |  |
| 2 | 51.3 | 19 | loop work (19 items) |  |
| 3 | 49.0 | 16 | loop work (16 items) | softirq NET_RX 8 |
| 4 | 45.0 | 25 | loop work (25 items) |  |
| 5 | 44.6 | 16 | loop work (16 items) | softirq NET_RX 1 |
| 6 | 44.4 | 32 | loop work (32 items) |  |
| 7 | 41.0 | 37 | loop work (37 items) | softirq NET_RX 2 |
| 8 | 41.0 | 21 | loop work (21 items) |  |
| 9 | 40.9 | 18 | loop work (18 items) |  |
| 10 | 40.7 | 26 | loop work (26 items) | softirq NET_RX 7 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 10178 |
| LOC Local timer interrupts | 6095 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-1/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-1/noise, loop 0 on CPU 1

17521459200 iterations, longest 29.5 µs; over 5 / 20 / 100 µs: 2 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+local timer tick) | 1 | 29.5 | 29.5 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 6.4 | 6.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 29.5 | 0 | mostly unexplained (+local timer tick) | local timer tick 12, timer tick_nohz_handler 11, softirq TIMER 6, irq work (IPI/vector) 1 |
| 2 | 6.4 | 0 | mostly unexplained (+call function (IPI/vector)) | call function (IPI/vector) 2 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1020 |
| LOC Local timer interrupts | 2 |
| IWI IRQ work interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (r1-base-1/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1457441 | 1457441 | 1.00 | 396885635 | 272.32 | 6614761 | 6566179 | 0.0 | 35.0 | false | 0 | 0 | 33.6 | 11964 / 6 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-epoll-spin-0, loop 0 on CPU 1

393970752 iterations, longest 33.6 µs; over 5 / 20 / 100 µs: 11964 / 6 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 199.6 | 33.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 33.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 33.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 33.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 33.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 33.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 32.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1012 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 5 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-1/pp-epoll-spin-0 (median 39.44)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 193.31 | 153.87 | 0.00 |  | 87.35 | unexplained: no kernel event (hypervisor or in-kernel time) | 66.52 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 181.60 | 142.17 | 0.00 |  | 0.00 |  | 142.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 169.15 | 129.72 | 0.00 |  | 0.00 |  | 129.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 159.52 | 120.08 | 0.00 |  | 0.00 |  | 120.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 156.62 | 117.18 | 0.00 |  | 0.00 |  | 117.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 155.74 | 116.30 | 0.00 |  | 0.00 |  | 116.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 151.81 | 112.37 | 0.00 |  | 0.00 |  | 112.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 148.05 | 108.61 | 0.00 |  | 0.00 |  | 108.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 143.85 | 104.41 | 0.00 |  | 0.00 |  | 104.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 142.96 | 103.52 | 0.00 |  | 0.00 |  | 103.52 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 799944064 | 666.62 | 6666201 | 6646201 | 0.0 | 29.9 | false | 0 | 0 | 14.2 | 5166 / 0 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-epoll-spin-10000, loop 0 on CPU 1

797544064 iterations, longest 14.2 µs; over 5 / 20 / 100 µs: 5166 / 0 / 0

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1021 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-epoll-spin-10000 (median 41.18)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 148.68 | 107.50 | 0.00 |  | 174.27 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 121.64 | 80.47 | 0.00 |  | 0.00 |  | 80.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 117.42 | 76.24 | 0.00 |  | 0.00 |  | 76.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 114.81 | 73.63 | 0.00 |  | 0.00 |  | 73.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 111.73 | 70.55 | 0.00 |  | 0.00 |  | 70.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 108.14 | 66.97 | 0.00 |  | 0.00 |  | 66.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 105.80 | 64.63 | 0.00 |  | 0.00 |  | 64.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 104.81 | 63.63 | 0.00 |  | 0.00 |  | 63.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 102.81 | 61.63 | 0.00 |  | 0.00 |  | 61.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 99.50 | 58.32 | 0.00 |  | 0.00 |  | 58.32 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1662410 | 1662410 | 1.00 | 121184139 | 72.90 | 2019736 | 1992029 | 98.6 | 74.2 | true | 0 | 10656 | 17.7 | 76650 / 0 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-recv-busy-0, loop 0 on CPU 1

119521728 iterations, longest 17.7 µs; over 5 / 20 / 100 µs: 76650 / 0 / 0

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 10695 |
| IWI IRQ work interrupts | 10662 |
| CAL Function call interrupts | 1012 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-1/pp-recv-busy-0 (median 35.66)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 159.93 | 124.27 | 0.00 |  | 0.00 |  | 124.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 149.59 | 113.92 | 0.00 |  | 0.00 |  | 113.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 140.08 | 104.42 | 0.00 |  | 0.00 |  | 104.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 134.38 | 98.72 | 0.00 |  | 0.00 |  | 98.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 133.25 | 97.59 | 0.00 |  | 0.00 |  | 97.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 133.11 | 97.45 | 0.00 |  | 0.00 |  | 97.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 128.85 | 93.19 | 0.00 |  | 0.00 |  | 93.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 128.66 | 93.00 | 0.00 |  | 0.00 |  | 93.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 127.67 | 92.01 | 0.00 |  | 0.00 |  | 92.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 124.30 | 88.64 | 0.00 |  | 0.00 |  | 88.64 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 262992384 | 219.16 | 2191603 | 2181603 | 99.5 | 70.4 | true | 0 | 7511 | 18.7 | 127109 / 0 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-recv-busy-10000, loop 0 on CPU 1

261792384 iterations, longest 18.7 µs; over 5 / 20 / 100 µs: 127109 / 0 / 0

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 7533 |
| IWI IRQ work interrupts | 7516 |
| CAL Function call interrupts | 1022 |

#### Slowest round trips: r1-base-1/pp-recv-busy-10000 (median 37.82)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 962.59 | 924.77 | 0.00 |  | 0.00 |  | 924.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 746.84 | 709.02 | 0.00 |  | 497.79 | unexplained: no kernel event (hypervisor or in-kernel time) | 211.24 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 454.27 | 416.45 | 0.00 |  | 129.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 286.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 402.79 | 364.97 | 0.00 |  | 348.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 16.82 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 245.24 | 207.42 | 0.00 |  | 67.65 | unexplained: no kernel event (hypervisor or in-kernel time) | 139.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 227.96 | 190.14 | 0.00 |  | 84.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 105.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 165.85 | 128.03 | 0.00 |  | 146.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 139.16 | 101.34 | 0.00 |  | 0.00 |  | 101.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 135.55 | 97.74 | 0.00 |  | 0.00 |  | 97.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 130.30 | 92.48 | 0.00 |  | 0.00 |  | 92.48 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1552868 | 1552868 | 1.00 | 1 | 0.00 | 0 | 56632679 | 0.0 | 0.0 | true | 0 | 0 | 4.5 | 0 / 0 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-0, loop 0 on CPU 1

3397960768 iterations, longest 4.5 µs; over 5 / 20 / 100 µs: 0 / 0 / 0

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1018 |
| LOC Local timer interrupts | 14 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-0 (median 38.05)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 331.87 | 293.82 | 0.00 |  | 234.42 | unexplained: no kernel event (hypervisor or in-kernel time) | 59.40 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 267.50 | 229.45 | 0.00 |  | 0.00 |  | 229.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 196.62 | 158.57 | 0.00 |  | 63.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 95.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 174.24 | 136.19 | 0.00 |  | 70.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 66.03 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 171.80 | 133.75 | 0.00 |  | 187.23 | softirq TIMER | 0.00 | server: softirq TIMER |
| 6 | 165.91 | 127.86 | 0.00 |  | 0.00 |  | 127.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 141.74 | 103.68 | 0.00 |  | 0.00 |  | 103.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 139.62 | 101.57 | 0.00 |  | 30.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 70.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 128.81 | 90.76 | 0.00 |  | 115.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 127.29 | 89.24 | 0.00 |  | 0.00 |  | 89.24 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 45642327 | 0.0 | 0.0 | true | 0 | 0 | 4.4 | 0 / 0 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-10000, loop 0 on CPU 1

5477079232 iterations, longest 4.4 µs; over 5 / 20 / 100 µs: 0 / 0 / 0

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1030 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-10000 (median 40.18)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 851.80 | 811.62 | 0.00 |  | 656.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 155.33 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 598.98 | 558.80 | 0.00 |  | 370.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 188.56 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 430.62 | 390.44 | 0.00 |  | 109.55 | unexplained: no kernel event (hypervisor or in-kernel time) | 280.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 292.90 | 252.72 | 0.00 |  | 144.76 | unexplained: no kernel event (hypervisor or in-kernel time) | 107.96 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 255.53 | 215.35 | 0.00 |  | 72.63 | unexplained: no kernel event (hypervisor or in-kernel time) | 142.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 250.06 | 209.88 | 0.00 |  | 181.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 28.70 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 184.07 | 143.90 | 0.00 |  | 64.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 79.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 138.40 | 98.22 | 0.00 |  | 0.00 |  | 98.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 135.20 | 95.03 | 0.00 |  | 0.00 |  | 95.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 133.11 | 92.94 | 0.00 |  | 0.00 |  | 92.94 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-sib-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1566636 | 1566636 | 1.00 | 1 | 0.00 | 0 | 52652034 | 0.0 | 0.0 | true | 0 | 0 | 5.6 | 1 / 0 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-sib-0, loop 0 on CPU 1

3159122048 iterations, longest 5.6 µs; over 5 / 20 / 100 µs: 1 / 0 / 0

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 15 |
| LOC Local timer interrupts | 9 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-sib-0 (median 37.81)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 312.20 | 274.39 | 0.00 |  | 143.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 131.34 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 195.07 | 157.26 | 0.00 |  | 0.00 |  | 157.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 165.31 | 127.50 | 0.00 |  | 84.98 | unexplained: no kernel event (hypervisor or in-kernel time) | 42.52 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 140.35 | 102.54 | 0.00 |  | 0.00 |  | 102.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 137.59 | 99.79 | 0.00 |  | 0.00 |  | 99.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 131.05 | 93.24 | 0.00 |  | 26.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 67.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 127.13 | 89.32 | 0.00 |  | 0.00 |  | 89.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 125.40 | 87.59 | 0.00 |  | 0.00 |  | 87.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 124.13 | 86.32 | 0.00 |  | 0.00 |  | 86.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 121.59 | 83.78 | 0.00 |  | 0.00 |  | 83.78 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-1/pp-uring-sqpoll-sib-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 39691381 | 0.0 | 0.0 | true | 0 | 0 | 20.4 | 3 / 1 / 0 | 0 | 0 |

#### Stalls: r1-base-1/pp-uring-sqpoll-sib-10000, loop 0 on CPU 1

4762965760 iterations, longest 20.4 µs; over 5 / 20 / 100 µs: 3 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.4 | 20.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 20.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 25 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-1/pp-uring-sqpoll-sib-10000 (median 41.26)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 881.04 | 839.78 | 0.00 |  | 514.78 | unexplained: no kernel event (hypervisor or in-kernel time) | 325.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 699.90 | 658.65 | 0.00 |  | 364.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 294.56 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 302.46 | 261.21 | 0.00 |  | 137.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 123.80 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 208.74 | 167.48 | 0.00 |  | 65.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 101.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 147.40 | 106.14 | 0.00 |  | 0.00 |  | 106.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 141.52 | 100.26 | 0.00 |  | 0.00 |  | 100.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 138.00 | 96.75 | 0.00 |  | 103.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 134.27 | 93.01 | 0.00 |  | 0.00 |  | 93.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 129.48 | 88.22 | 0.00 |  | 0.00 |  | 88.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 127.42 | 86.16 | 0.00 |  | 69.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 16.33 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

## m1/lab-epoll-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 355509 | 161253 | 2.20 | 358105893 | 1007.30 | 5968432 | 5965744 | 0.0 | 29.1 | false | 0 | 0 | 154.5 | 11435 / 2 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8197 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (86676 of 343095 in 1-msg reads) | 86676 | 3.37 | 2.90 | 4.71 | 8.45 | 14.04 | 16.88 | 18.58 |

#### Stalls: m1/lab-epoll-spin-c1, loop 0 on CPU 1

357944640 iterations, longest 154.5 µs; over 5 / 20 / 100 µs: 11435 / 2 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 180.6 | 154.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 154.5 | 589 | loop work (589 items) |  |
| 2 | 26.2 | 5 | loop work (5 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1014 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-epoll-spin-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3163996 | 1040347 | 3.04 | 344811099 | 108.98 | 5746852 | 5729513 | 0.0 | 31.0 | false | 0 | 0 | 1208.5 | 58762 / 129 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8193 | 0 |
| epoll-spin | 1 | 8196 | 48 |
| epoll-spin | 2 | 8194 | 48 |
| epoll-spin | 3 | 8208 | 48 |
| epoll-spin | 4 | 8194 | 48 |
| epoll-spin | 5 | 8198 | 48 |
| epoll-spin | 6 | 8205 | 0 |
| epoll-spin | 7 | 8199 | 0 |
| epoll-spin | 8 | 8205 | 0 |
| epoll-spin | 9 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 708735104 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (417247 of 3053554 in 1-msg reads) | 417247 | 4.24 | 3.65 | 6.27 | 10.81 | 13.75 | 29.57 | 41.99 |

#### Stalls: m1/lab-epoll-spin-c10, loop 0 on CPU 1

343770752 iterations, longest 1208.5 µs; over 5 / 20 / 100 µs: 58762 / 129 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 78 | 3161.6 | 1208.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 51 | 1288.9 | 33.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1208.5 | 5166 | loop work (5166 items) |  |
| 2 | 34.9 | 2 | loop work (2 items) |  |
| 3 | 33.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 33.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 33.6 | 6 | loop work (6 items) |  |
| 6 | 33.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 33.3 | 7 | loop work (7 items) |  |
| 8 | 32.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.8 | 2 | loop work (2 items) |  |
| 10 | 32.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1014 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-epoll-spin-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 4545633 | 2404360 | 1.89 | 264137544 | 58.11 | 4402292 | 4362220 | 0.0 | 28.5 | false | 0 | 0 | 1787.5 | 85770 / 291 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8203 | 0 |
| epoll-spin | 1 | 8207 | 0 |
| epoll-spin | 2 | 8200 | 48 |
| epoll-spin | 3 | 8206 | 48 |
| epoll-spin | 4 | 8193 | 0 |
| epoll-spin | 5 | 8202 | 48 |
| epoll-spin | 6 | 8207 | 0 |
| epoll-spin | 7 | 8206 | 48 |
| epoll-spin | 8 | 8196 | 48 |
| epoll-spin | 9 | 8194 | 48 |
| epoll-spin | 10 | 8205 | 0 |
| epoll-spin | 11 | 8199 | 0 |
| epoll-spin | 12 | 8195 | 0 |
| epoll-spin | 13 | 8198 | 48 |
| epoll-spin | 14 | 8208 | 48 |
| epoll-spin | 15 | 8204 | 48 |
| epoll-spin | 16 | 8205 | 0 |
| epoll-spin | 17 | 8198 | 48 |
| epoll-spin | 18 | 8199 | 0 |
| epoll-spin | 19 | 8195 | 0 |
| epoll-spin | 20 | 8199 | 0 |
| epoll-spin | 21 | 8199 | 0 |
| epoll-spin | 22 | 8203 | 0 |
| epoll-spin | 23 | 8193 | 0 |
| epoll-spin | 24 | 8206 | 48 |
| epoll-spin | 25 | 8202 | 48 |
| epoll-spin | 26 | 8200 | 48 |
| epoll-spin | 27 | 8202 | 48 |
| epoll-spin | 28 | 8203 | 0 |
| epoll-spin | 29 | 8206 | 48 |
| epoll-spin | 30 | 8200 | 48 |
| epoll-spin | 31 | 8206 | 48 |
| epoll-spin | 32 | 8202 | 48 |
| epoll-spin | 33 | 8207 | 0 |
| epoll-spin | 34 | 8194 | 48 |
| epoll-spin | 35 | 8208 | 48 |
| epoll-spin | 36 | 8194 | 48 |
| epoll-spin | 37 | 8201 | 0 |
| epoll-spin | 38 | 8208 | 48 |
| epoll-spin | 39 | 8206 | 48 |
| epoll-spin | 40 | 8203 | 0 |
| epoll-spin | 41 | 8193 | 0 |
| epoll-spin | 42 | 8207 | 0 |
| epoll-spin | 43 | 8203 | 0 |
| epoll-spin | 44 | 8200 | 48 |
| epoll-spin | 45 | 8202 | 48 |
| epoll-spin | 46 | 8200 | 48 |
| epoll-spin | 47 | 8201 | 0 |
| epoll-spin | 48 | 8194 | 48 |
| epoll-spin | 49 | 8199 | 0 |
| epoll-spin | 50 | 8201 | 0 |
| epoll-spin | 51 | 8205 | 0 |
| epoll-spin | 52 | 8198 | 48 |
| epoll-spin | 53 | 8201 | 0 |
| epoll-spin | 54 | 8199 | 0 |
| epoll-spin | 55 | 8208 | 48 |
| epoll-spin | 56 | 8205 | 0 |
| epoll-spin | 57 | 8195 | 0 |
| epoll-spin | 58 | 8204 | 48 |
| epoll-spin | 59 | 8205 | 0 |
| epoll-spin | 60 | 8196 | 48 |
| epoll-spin | 61 | 8196 | 48 |
| epoll-spin | 62 | 8206 | 48 |
| epoll-spin | 63 | 8197 | 0 |
| epoll-spin | 64 | 8203 | 0 |
| epoll-spin | 65 | 8193 | 0 |
| epoll-spin | 66 | 8206 | 48 |
| epoll-spin | 67 | 8193 | 0 |
| epoll-spin | 68 | 8193 | 0 |
| epoll-spin | 69 | 8197 | 0 |
| epoll-spin | 70 | 8197 | 0 |
| epoll-spin | 71 | 8193 | 0 |
| epoll-spin | 72 | 8200 | 48 |
| epoll-spin | 73 | 8205 | 0 |
| epoll-spin | 74 | 8204 | 48 |
| epoll-spin | 75 | 8205 | 0 |
| epoll-spin | 76 | 8201 | 0 |
| epoll-spin | 77 | 8198 | 48 |
| epoll-spin | 78 | 8201 | 0 |
| epoll-spin | 79 | 8199 | 0 |
| epoll-spin | 80 | 8195 | 0 |
| epoll-spin | 81 | 8201 | 0 |
| epoll-spin | 82 | 8199 | 0 |
| epoll-spin | 83 | 8204 | 48 |
| epoll-spin | 84 | 8198 | 48 |
| epoll-spin | 85 | 8204 | 48 |
| epoll-spin | 86 | 8205 | 0 |
| epoll-spin | 87 | 8198 | 48 |
| epoll-spin | 88 | 8193 | 0 |
| epoll-spin | 89 | 8207 | 0 |
| epoll-spin | 90 | 8197 | 0 |
| epoll-spin | 91 | 8200 | 48 |
| epoll-spin | 92 | 8193 | 0 |
| epoll-spin | 93 | 8203 | 0 |
| epoll-spin | 94 | 8206 | 48 |
| epoll-spin | 95 | 8197 | 0 |
| epoll-spin | 96 | 8196 | 48 |
| epoll-spin | 97 | 8205 | 0 |
| epoll-spin | 98 | 8195 | 0 |
| epoll-spin | 99 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1018221792 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1743650 of 4387080 in 1-msg reads) | 1743650 | 3.45 | 2.92 | 5.11 | 8.74 | 12.01 | 27.19 | 37.63 |

#### Stalls: m1/lab-epoll-spin-c100, loop 0 on CPU 1

261733184 iterations, longest 1787.5 µs; over 5 / 20 / 100 µs: 85770 / 291 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 199 | 5186.7 | 35.0 |
| loop work | 92 | 4716.3 | 1787.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1787.5 | 5343 | loop work (5343 items) |  |
| 2 | 675.2 | 2147 | loop work (2147 items) |  |
| 3 | 53.3 | 101 | loop work (101 items) |  |
| 4 | 35.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 35.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 34.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 34.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 34.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 33.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 33.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1015 |
| IWI IRQ work interrupts | 8 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=122964 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355509 | 143419 | 2.48 | 130624960 | 367.43 | 2177083 | 2177083 | 99.9 | 67.0 | true | 0 | 741 | 156.6 | 26980 / 1197 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8202 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (73913 of 343095 in 1-msg reads) | 73913 | 2.95 | 2.90 | 3.37 | 5.28 | 11.03 | 18.78 | 22.98 |

#### Stalls: m1/lab-recv-busy-c1, loop 0 on CPU 1

130624960 iterations, longest 156.6 µs; over 5 / 20 / 100 µs: 26980 / 1197 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 730 | 21731.1 | 156.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 315 | 7968.2 | 42.3 |
| softirq TIMER | 74 | 1688.3 | 27.2 |
| mostly unexplained (+softirq NET_RX) | 50 | 1369.2 | 39.0 |
| softirq NET_RX | 12 | 359.2 | 35.2 |
| mostly unexplained (+irq work (IPI/vector)) | 12 | 309.7 | 35.9 |
| mostly unexplained (+preempted by kworker/1:1) | 4 | 87.9 | 25.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 156.6 | 589 | loop work (589 items) |  |
| 2 | 48.3 | 10 | loop work (10 items) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 48.1 | 10 | loop work (10 items) |  |
| 4 | 47.8 | 10 | loop work (10 items) |  |
| 5 | 47.5 | 10 | loop work (10 items) |  |
| 6 | 46.6 | 10 | loop work (10 items) |  |
| 7 | 46.4 | 10 | loop work (10 items) |  |
| 8 | 45.6 | 10 | loop work (10 items) |  |
| 9 | 45.4 | 10 | loop work (10 items) |  |
| 10 | 45.0 | 10 | loop work (10 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5695 |
| CAL Function call interrupts | 1015 |
| LOC Local timer interrupts | 907 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=306452 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3163684 | 914556 | 3.46 | 129939840 | 41.07 | 2165664 | 216566 | 99.3 | 69.7 | true | 0 | 1635 | 1069.3 | 689734 / 5131 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 1 |
| recv-busy | 1 | 8198 | 48 |
| recv-busy | 2 | 8199 | 1 |
| recv-busy | 3 | 8194 | 48 |
| recv-busy | 4 | 8208 | 48 |
| recv-busy | 5 | 8205 | 0 |
| recv-busy | 6 | 8203 | 1 |
| recv-busy | 7 | 8197 | 1 |
| recv-busy | 8 | 8203 | 0 |
| recv-busy | 9 | 8196 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708665216 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (313797 of 3053545 in 1-msg reads) | 313797 | 4.29 | 3.58 | 6.67 | 11.27 | 18.11 | 30.40 | 39.88 |

#### Stalls: m1/lab-recv-busy-c10, loop 0 on CPU 1

12993984 iterations, longest 1069.3 µs; over 5 / 20 / 100 µs: 689734 / 5131 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4304 | 120808.3 | 1069.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 617 | 16378.3 | 45.1 |
| softirq TIMER | 141 | 3489.6 | 30.3 |
| mostly unexplained (+irq work (IPI/vector)) | 27 | 788.6 | 35.2 |
| mostly unexplained (+softirq NET_RX) | 27 | 705.0 | 30.8 |
| softirq NET_RX | 14 | 478.0 | 43.8 |
| mostly unexplained (+preempted by kworker/1:1) | 1 | 35.8 | 35.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1069.3 | 4943 | loop work (4943 items) |  |
| 2 | 64.4 | 42 | loop work (42 items) |  |
| 3 | 59.9 | 11 | loop work (11 items) | softirq NET_RX 6 |
| 4 | 58.8 | 22 | loop work (22 items) |  |
| 5 | 56.2 | 18 | loop work (18 items) | softirq NET_RX 5, irq work (IPI/vector) 1, preempted by kworker/1:1 1, workqueue [ena] 1 |
| 6 | 55.7 | 20 | loop work (20 items) |  |
| 7 | 54.0 | 23 | loop work (23 items) |  |
| 8 | 53.9 | 24 | loop work (24 items) |  |
| 9 | 53.0 | 25 | loop work (25 items) |  |
| 10 | 52.5 | 10 | loop work (10 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4925 |
| LOC Local timer interrupts | 1819 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=534755 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4546056 | 1978265 | 2.30 | 121952000 | 26.83 | 2032533 | 20325 | 98.4 | 71.2 | true | 0 | 4158 | 2417.0 | 1219519 / 1219519 / 248 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8207 | 0 |
| recv-busy | 1 | 8203 | 1 |
| recv-busy | 2 | 8196 | 1 |
| recv-busy | 3 | 8193 | 1 |
| recv-busy | 4 | 8200 | 48 |
| recv-busy | 5 | 8206 | 48 |
| recv-busy | 6 | 8202 | 1 |
| recv-busy | 7 | 8193 | 0 |
| recv-busy | 8 | 8206 | 1 |
| recv-busy | 9 | 8195 | 0 |
| recv-busy | 10 | 8194 | 48 |
| recv-busy | 11 | 8199 | 0 |
| recv-busy | 12 | 8199 | 0 |
| recv-busy | 13 | 8198 | 48 |
| recv-busy | 14 | 8198 | 48 |
| recv-busy | 15 | 8195 | 1 |
| recv-busy | 16 | 8202 | 1 |
| recv-busy | 17 | 8206 | 1 |
| recv-busy | 18 | 8193 | 1 |
| recv-busy | 19 | 8196 | 1 |
| recv-busy | 20 | 8207 | 1 |
| recv-busy | 21 | 8206 | 48 |
| recv-busy | 22 | 8203 | 0 |
| recv-busy | 23 | 8206 | 48 |
| recv-busy | 24 | 8208 | 48 |
| recv-busy | 25 | 8201 | 0 |
| recv-busy | 26 | 8198 | 48 |
| recv-busy | 27 | 8194 | 48 |
| recv-busy | 28 | 8204 | 1 |
| recv-busy | 29 | 8205 | 0 |
| recv-busy | 30 | 8198 | 48 |
| recv-busy | 31 | 8199 | 0 |
| recv-busy | 32 | 8194 | 48 |
| recv-busy | 33 | 8208 | 48 |
| recv-busy | 34 | 8195 | 0 |
| recv-busy | 35 | 8205 | 0 |
| recv-busy | 36 | 8194 | 48 |
| recv-busy | 37 | 8208 | 48 |
| recv-busy | 38 | 8194 | 1 |
| recv-busy | 39 | 8201 | 0 |
| recv-busy | 40 | 8196 | 1 |
| recv-busy | 41 | 8193 | 0 |
| recv-busy | 42 | 8207 | 0 |
| recv-busy | 43 | 8206 | 1 |
| recv-busy | 44 | 8200 | 1 |
| recv-busy | 45 | 8197 | 0 |
| recv-busy | 46 | 8196 | 48 |
| recv-busy | 47 | 8197 | 0 |
| recv-busy | 48 | 8196 | 1 |
| recv-busy | 49 | 8200 | 1 |
| recv-busy | 50 | 8193 | 0 |
| recv-busy | 51 | 8206 | 48 |
| recv-busy | 52 | 8203 | 0 |
| recv-busy | 53 | 8207 | 0 |
| recv-busy | 54 | 8203 | 0 |
| recv-busy | 55 | 8196 | 48 |
| recv-busy | 56 | 8201 | 0 |
| recv-busy | 57 | 8201 | 0 |
| recv-busy | 58 | 8205 | 1 |
| recv-busy | 59 | 8204 | 48 |
| recv-busy | 60 | 8194 | 48 |
| recv-busy | 61 | 8201 | 0 |
| recv-busy | 62 | 8204 | 48 |
| recv-busy | 63 | 8205 | 0 |
| recv-busy | 64 | 8205 | 1 |
| recv-busy | 65 | 8208 | 48 |
| recv-busy | 66 | 8205 | 1 |
| recv-busy | 67 | 8195 | 0 |
| recv-busy | 68 | 8201 | 0 |
| recv-busy | 69 | 8204 | 48 |
| recv-busy | 70 | 8205 | 1 |
| recv-busy | 71 | 8205 | 0 |
| recv-busy | 72 | 8202 | 1 |
| recv-busy | 73 | 8193 | 0 |
| recv-busy | 74 | 8202 | 48 |
| recv-busy | 75 | 8206 | 48 |
| recv-busy | 76 | 8206 | 1 |
| recv-busy | 77 | 8204 | 48 |
| recv-busy | 78 | 8198 | 48 |
| recv-busy | 79 | 8195 | 0 |
| recv-busy | 80 | 8208 | 1 |
| recv-busy | 81 | 8199 | 0 |
| recv-busy | 82 | 8198 | 48 |
| recv-busy | 83 | 8203 | 0 |
| recv-busy | 84 | 8202 | 48 |
| recv-busy | 85 | 8193 | 1 |
| recv-busy | 86 | 8202 | 48 |
| recv-busy | 87 | 8206 | 1 |
| recv-busy | 88 | 8203 | 1 |
| recv-busy | 89 | 8207 | 0 |
| recv-busy | 90 | 8193 | 0 |
| recv-busy | 91 | 8207 | 0 |
| recv-busy | 92 | 8196 | 48 |
| recv-busy | 93 | 8200 | 48 |
| recv-busy | 94 | 8207 | 0 |
| recv-busy | 95 | 8197 | 0 |
| recv-busy | 96 | 8200 | 1 |
| recv-busy | 97 | 8206 | 48 |
| recv-busy | 98 | 8202 | 48 |
| recv-busy | 99 | 8205 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1018316544 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1508324 of 4387225 in 1-msg reads) | 1508324 | 25.26 | 25.07 | 45.52 | 58.15 | 69.09 | 81.65 | 134.14 |

#### Stalls: m1/lab-recv-busy-c100, loop 0 on CPU 1

1219520 iterations, longest 2417.0 µs; over 5 / 20 / 100 µs: 1219519 / 1219519 / 248 (170943 stalls not kept)

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 816833 | 38404500.9 | 103.1 |
| loop work | 213979 | 12208173.9 | 2417.0 |
| mostly unexplained (+softirq NET_RX) | 11262 | 570822.3 | 93.6 |
| mostly unexplained (+local timer tick) | 3174 | 157207.1 | 86.1 |
| mostly unexplained (+irq work (IPI/vector)) | 2160 | 116231.3 | 84.3 |
| mostly unexplained (+preempted by kworker/1:1) | 961 | 54732.3 | 87.7 |
| mostly unexplained (+softirq TIMER) | 121 | 8274.7 | 81.2 |
| mostly unexplained (+call function single (IPI/vector)) | 41 | 2028.2 | 56.3 |
| mostly unexplained (+ipi_send_cpu) | 25 | 1182.3 | 55.0 |
| mostly unexplained (+softirq NET_TX) | 8 | 420.0 | 68.2 |
| mostly unexplained (+softirq RCU) | 6 | 318.3 | 75.3 |
| mostly unexplained (+call function (IPI/vector)) | 6 | 310.7 | 53.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2417.0 | 7712 | loop work (7712 items) |  |
| 2 | 177.8 | 154 | loop work (154 items) |  |
| 3 | 130.3 | 72 | loop work (72 items) |  |
| 4 | 128.4 | 85 | loop work (85 items) |  |
| 5 | 127.0 | 39 | loop work (39 items) | softirq NET_RX 1 |
| 6 | 126.2 | 56 | loop work (56 items) |  |
| 7 | 125.0 | 10 | loop work (10 items) |  |
| 8 | 124.3 | 63 | loop work (63 items) | softirq NET_RX 1 |
| 9 | 118.3 | 57 | loop work (57 items) |  |
| 10 | 117.0 | 46 | loop work (46 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5697 |
| LOC Local timer interrupts | 4272 |
| CAL Function call interrupts | 1070 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=126037 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355497 | 147572 | 2.41 | 195467136 | 549.84 | 3257786 | 1628893 | 99.8 | 58.9 | true | 0 | 429 | 154.4 | 18756 / 459 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79631328 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (75676 of 343094 in 1-msg reads) | 75676 | 2.66 | 2.61 | 2.89 | 3.57 | 4.31 | 9.76 | 11.52 |

#### Stalls: m1/lab-recv-busy-pq-c1, loop 0 on CPU 1

97733568 iterations, longest 154.4 µs; over 5 / 20 / 100 µs: 18756 / 459 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 408 | 9751.9 | 154.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 33 | 711.1 | 35.6 |
| mostly unexplained (+irq work (IPI/vector)) | 9 | 195.3 | 25.2 |
| mostly unexplained (+softirq NET_RX) | 5 | 121.9 | 33.0 |
| mostly unexplained (+preempted by kworker/1:1) | 2 | 50.5 | 28.2 |
| softirq TIMER | 2 | 44.8 | 24.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 154.4 | 578 | loop work (578 items) |  |
| 2 | 42.7 | 9 | loop work (9 items) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 42.0 | 10 | loop work (10 items) |  |
| 4 | 40.8 | 10 | loop work (10 items) |  |
| 5 | 40.7 | 8 | loop work (8 items) | softirq NET_RX 4 |
| 6 | 37.3 | 7 | loop work (7 items) |  |
| 7 | 36.8 | 10 | loop work (10 items) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 35.8 | 7 | loop work (7 items) | softirq NET_RX 1 |
| 9 | 35.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 34.0 | 10 | loop work (10 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5348 |
| CAL Function call interrupts | 1014 |
| LOC Local timer interrupts | 487 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=283578 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3163999 | 925235 | 3.42 | 138355130 | 43.73 | 2305919 | 230454 | 99.3 | 68.6 | true | 0 | 1620 | 1172.7 | 647261 / 5243 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8206 | 48 |
| recv-busy | 1 | 8197 | 1 |
| recv-busy | 2 | 8202 | 1 |
| recv-busy | 3 | 8204 | 1 |
| recv-busy | 4 | 8208 | 1 |
| recv-busy | 5 | 8205 | 0 |
| recv-busy | 6 | 8208 | 48 |
| recv-busy | 7 | 8201 | 1 |
| recv-busy | 8 | 8194 | 1 |
| recv-busy | 9 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708735776 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (323146 of 3053557 in 1-msg reads) | 323146 | 4.45 | 3.76 | 6.92 | 11.75 | 20.12 | 31.81 | 47.60 |

#### Stalls: m1/lab-recv-busy-pq-c10, loop 0 on CPU 1

13827264 iterations, longest 1172.7 µs; over 5 / 20 / 100 µs: 647261 / 5243 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4327 | 119931.1 | 1172.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 731 | 19156.6 | 48.0 |
| softirq TIMER | 138 | 3367.4 | 29.4 |
| mostly unexplained (+softirq NET_RX) | 19 | 502.7 | 36.9 |
| mostly unexplained (+irq work (IPI/vector)) | 16 | 480.8 | 41.0 |
| softirq NET_RX | 8 | 278.8 | 44.0 |
| mostly unexplained (+preempted by kworker/1:1) | 4 | 120.3 | 35.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1172.7 | 5188 | loop work (5188 items) |  |
| 2 | 57.1 | 14 | loop work (14 items) |  |
| 3 | 57.0 | 39 | loop work (39 items) |  |
| 4 | 54.6 | 25 | loop work (25 items) |  |
| 5 | 53.6 | 39 | loop work (39 items) |  |
| 6 | 53.1 | 37 | loop work (37 items) |  |
| 7 | 53.0 | 28 | loop work (28 items) |  |
| 8 | 52.5 | 21 | loop work (21 items) | softirq NET_RX 3, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 52.5 | 39 | loop work (39 items) | softirq NET_RX 2 |
| 10 | 52.4 | 24 | loop work (24 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5333 |
| LOC Local timer interrupts | 1806 |
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=547668 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4545927 | 2175990 | 2.09 | 123647552 | 27.20 | 2060793 | 119466 | 98.1 | 70.2 | true | 0 | 4559 | 2210.0 | 7167935 / 33557 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 0 |
| recv-busy | 1 | 8200 | 48 |
| recv-busy | 2 | 8207 | 0 |
| recv-busy | 3 | 8203 | 0 |
| recv-busy | 4 | 8203 | 1 |
| recv-busy | 5 | 8202 | 1 |
| recv-busy | 6 | 8206 | 48 |
| recv-busy | 7 | 8203 | 0 |
| recv-busy | 8 | 8197 | 0 |
| recv-busy | 9 | 8207 | 0 |
| recv-busy | 10 | 8193 | 1 |
| recv-busy | 11 | 8202 | 48 |
| recv-busy | 12 | 8207 | 0 |
| recv-busy | 13 | 8207 | 0 |
| recv-busy | 14 | 8197 | 1 |
| recv-busy | 15 | 8200 | 48 |
| recv-busy | 16 | 8198 | 48 |
| recv-busy | 17 | 8201 | 1 |
| recv-busy | 18 | 8201 | 0 |
| recv-busy | 19 | 8195 | 0 |
| recv-busy | 20 | 8205 | 0 |
| recv-busy | 21 | 8201 | 1 |
| recv-busy | 22 | 8194 | 48 |
| recv-busy | 23 | 8197 | 1 |
| recv-busy | 24 | 8200 | 48 |
| recv-busy | 25 | 8193 | 0 |
| recv-busy | 26 | 8203 | 0 |
| recv-busy | 27 | 8200 | 1 |
| recv-busy | 28 | 8206 | 48 |
| recv-busy | 29 | 8207 | 0 |
| recv-busy | 30 | 8203 | 0 |
| recv-busy | 31 | 8196 | 48 |
| recv-busy | 32 | 8195 | 0 |
| recv-busy | 33 | 8198 | 1 |
| recv-busy | 34 | 8194 | 48 |
| recv-busy | 35 | 8208 | 48 |
| recv-busy | 36 | 8195 | 1 |
| recv-busy | 37 | 8199 | 1 |
| recv-busy | 38 | 8194 | 48 |
| recv-busy | 39 | 8204 | 48 |
| recv-busy | 40 | 8201 | 1 |
| recv-busy | 41 | 8194 | 48 |
| recv-busy | 42 | 8195 | 0 |
| recv-busy | 43 | 8198 | 48 |
| recv-busy | 44 | 8193 | 0 |
| recv-busy | 45 | 8200 | 48 |
| recv-busy | 46 | 8203 | 1 |
| recv-busy | 47 | 8202 | 48 |
| recv-busy | 48 | 8200 | 48 |
| recv-busy | 49 | 8202 | 48 |
| recv-busy | 50 | 8196 | 48 |
| recv-busy | 51 | 8206 | 48 |
| recv-busy | 52 | 8197 | 0 |
| recv-busy | 53 | 8193 | 0 |
| recv-busy | 54 | 8200 | 1 |
| recv-busy | 55 | 8206 | 48 |
| recv-busy | 56 | 8197 | 0 |
| recv-busy | 57 | 8196 | 48 |
| recv-busy | 58 | 8202 | 48 |
| recv-busy | 59 | 8200 | 48 |
| recv-busy | 60 | 8200 | 48 |
| recv-busy | 61 | 8193 | 1 |
| recv-busy | 62 | 8205 | 0 |
| recv-busy | 63 | 8198 | 48 |
| recv-busy | 64 | 8198 | 48 |
| recv-busy | 65 | 8195 | 0 |
| recv-busy | 66 | 8204 | 48 |
| recv-busy | 67 | 8199 | 0 |
| recv-busy | 68 | 8201 | 0 |
| recv-busy | 69 | 8204 | 48 |
| recv-busy | 70 | 8195 | 0 |
| recv-busy | 71 | 8201 | 0 |
| recv-busy | 72 | 8208 | 48 |
| recv-busy | 73 | 8204 | 48 |
| recv-busy | 74 | 8195 | 0 |
| recv-busy | 75 | 8204 | 48 |
| recv-busy | 76 | 8207 | 0 |
| recv-busy | 77 | 8196 | 48 |
| recv-busy | 78 | 8202 | 1 |
| recv-busy | 79 | 8206 | 1 |
| recv-busy | 80 | 8203 | 1 |
| recv-busy | 81 | 8197 | 1 |
| recv-busy | 82 | 8206 | 48 |
| recv-busy | 83 | 8202 | 1 |
| recv-busy | 84 | 8205 | 0 |
| recv-busy | 85 | 8198 | 1 |
| recv-busy | 86 | 8208 | 48 |
| recv-busy | 87 | 8194 | 48 |
| recv-busy | 88 | 8204 | 1 |
| recv-busy | 89 | 8201 | 0 |
| recv-busy | 90 | 8205 | 1 |
| recv-busy | 91 | 8194 | 1 |
| recv-busy | 92 | 8199 | 1 |
| recv-busy | 93 | 8193 | 0 |
| recv-busy | 94 | 8207 | 0 |
| recv-busy | 95 | 8202 | 48 |
| recv-busy | 96 | 8196 | 48 |
| recv-busy | 97 | 8196 | 48 |
| recv-busy | 98 | 8197 | 0 |
| recv-busy | 99 | 8196 | 48 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1018287648 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1558131 of 4387103 in 1-msg reads) | 1558131 | 7.31 | 7.09 | 11.26 | 17.98 | 25.28 | 35.56 | 48.95 |

#### Stalls: m1/lab-recv-busy-pq-c100, loop 0 on CPU 1

7167936 iterations, longest 2210.0 µs; over 5 / 20 / 100 µs: 7167935 / 33557 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 31444 | 766539.8 | 2210.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1802 | 54903.9 | 52.2 |
| softirq TIMER | 150 | 4523.7 | 37.4 |
| mostly unexplained (+softirq NET_RX) | 60 | 1781.6 | 42.7 |
| mostly unexplained (+preempted by kworker/1:1) | 54 | 1314.1 | 42.9 |
| mostly unexplained (+irq work (IPI/vector)) | 34 | 940.4 | 45.4 |
| mostly unexplained (+local timer tick) | 6 | 167.3 | 40.8 |
| softirq NET_RX | 4 | 156.4 | 45.8 |
| mostly unexplained (+softirq TIMER) | 3 | 104.3 | 48.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2210.0 | 6539 | loop work (6539 items) |  |
| 2 | 428.6 | 1225 | loop work (1225 items) |  |
| 3 | 94.9 | 70 | loop work (70 items) |  |
| 4 | 71.9 | 62 | loop work (62 items) |  |
| 5 | 70.9 | 53 | loop work (53 items) |  |
| 6 | 70.2 | 52 | loop work (52 items) |  |
| 7 | 70.1 | 43 | loop work (43 items) | softirq NET_RX 9 |
| 8 | 67.3 | 48 | loop work (48 items) |  |
| 9 | 66.5 | 56 | loop work (56 items) |  |
| 10 | 65.4 | 49 | loop work (49 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6282 |
| LOC Local timer interrupts | 4636 |
| CAL Function call interrupts | 1048 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=122243 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355497 | 148011 | 2.40 | 1 | 0.00 | 0 | 58137830 | 0.0 | 0.0 | true | 0 | 0 | 79.4 | 2 / 1 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8194 | 13 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79631328 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (76547 of 343094 in 1-msg reads) | 76547 | 3.37 | 3.33 | 3.59 | 4.13 | 5.68 | 10.27 | 18.77 |

#### Stalls: m1/lab-uring-sqpoll-c1, loop 0 on CPU 1

3488269824 iterations, longest 79.4 µs; over 5 / 20 / 100 µs: 2 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 79.4 | 79.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 79.4 | 578 | loop work (578 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1018 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=621703 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3164000 | 943019 | 3.36 | 1 | 0.00 | 0 | 43003148 | 0.0 | 0.0 | true | 0 | 0 | 678.9 | 446 / 1 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8199 | 13 |
| uring-sqpoll | 1 | 8204 | 13 |
| uring-sqpoll | 2 | 8208 | 13 |
| uring-sqpoll | 3 | 8195 | 13 |
| uring-sqpoll | 4 | 8208 | 13 |
| uring-sqpoll | 5 | 8195 | 13 |
| uring-sqpoll | 6 | 8198 | 48 |
| uring-sqpoll | 7 | 8208 | 13 |
| uring-sqpoll | 8 | 8205 | 13 |
| uring-sqpoll | 9 | 8195 | 13 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708736000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (329070 of 3053455 in 1-msg reads) | 329070 | 4.54 | 3.93 | 6.56 | 10.91 | 16.74 | 29.76 | 41.99 |

#### Stalls: m1/lab-uring-sqpoll-c10, loop 0 on CPU 1

2580188864 iterations, longest 678.9 µs; over 5 / 20 / 100 µs: 446 / 1 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 678.9 | 678.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 678.9 | 5232 | loop work (5232 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1018 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1305649 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4545928 | 2291387 | 1.98 | 1 | 0.00 | 0 | 8754906 | 0.0 | 0.0 | true | 0 | 0 | 641.4 | 665 / 2 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8204 | 13 |
| uring-sqpoll | 1 | 8195 | 13 |
| uring-sqpoll | 2 | 8206 | 13 |
| uring-sqpoll | 3 | 8197 | 13 |
| uring-sqpoll | 4 | 8202 | 13 |
| uring-sqpoll | 5 | 8196 | 13 |
| uring-sqpoll | 6 | 8193 | 0 |
| uring-sqpoll | 7 | 8200 | 13 |
| uring-sqpoll | 8 | 8203 | 13 |
| uring-sqpoll | 9 | 8206 | 13 |
| uring-sqpoll | 10 | 8196 | 13 |
| uring-sqpoll | 11 | 8197 | 0 |
| uring-sqpoll | 12 | 8194 | 13 |
| uring-sqpoll | 13 | 8199 | 13 |
| uring-sqpoll | 14 | 8208 | 48 |
| uring-sqpoll | 15 | 8194 | 13 |
| uring-sqpoll | 16 | 8195 | 13 |
| uring-sqpoll | 17 | 8198 | 13 |
| uring-sqpoll | 18 | 8199 | 13 |
| uring-sqpoll | 19 | 8205 | 13 |
| uring-sqpoll | 20 | 8208 | 48 |
| uring-sqpoll | 21 | 8195 | 0 |
| uring-sqpoll | 22 | 8208 | 13 |
| uring-sqpoll | 23 | 8194 | 13 |
| uring-sqpoll | 24 | 8204 | 48 |
| uring-sqpoll | 25 | 8199 | 13 |
| uring-sqpoll | 26 | 8208 | 13 |
| uring-sqpoll | 27 | 8194 | 13 |
| uring-sqpoll | 28 | 8199 | 13 |
| uring-sqpoll | 29 | 8202 | 13 |
| uring-sqpoll | 30 | 8196 | 13 |
| uring-sqpoll | 31 | 8193 | 13 |
| uring-sqpoll | 32 | 8200 | 48 |
| uring-sqpoll | 33 | 8196 | 48 |
| uring-sqpoll | 34 | 8207 | 0 |
| uring-sqpoll | 35 | 8203 | 13 |
| uring-sqpoll | 36 | 8197 | 0 |
| uring-sqpoll | 37 | 8193 | 13 |
| uring-sqpoll | 38 | 8200 | 13 |
| uring-sqpoll | 39 | 8202 | 48 |
| uring-sqpoll | 40 | 8202 | 48 |
| uring-sqpoll | 41 | 8196 | 48 |
| uring-sqpoll | 42 | 8200 | 48 |
| uring-sqpoll | 43 | 8197 | 13 |
| uring-sqpoll | 44 | 8196 | 48 |
| uring-sqpoll | 45 | 8204 | 13 |
| uring-sqpoll | 46 | 8205 | 13 |
| uring-sqpoll | 47 | 8194 | 48 |
| uring-sqpoll | 48 | 8201 | 13 |
| uring-sqpoll | 49 | 8204 | 13 |
| uring-sqpoll | 50 | 8199 | 0 |
| uring-sqpoll | 51 | 8204 | 13 |
| uring-sqpoll | 52 | 8201 | 13 |
| uring-sqpoll | 53 | 8208 | 13 |
| uring-sqpoll | 54 | 8194 | 48 |
| uring-sqpoll | 55 | 8204 | 48 |
| uring-sqpoll | 56 | 8195 | 13 |
| uring-sqpoll | 57 | 8204 | 13 |
| uring-sqpoll | 58 | 8199 | 13 |
| uring-sqpoll | 59 | 8208 | 48 |
| uring-sqpoll | 60 | 8204 | 13 |
| uring-sqpoll | 61 | 8197 | 0 |
| uring-sqpoll | 62 | 8196 | 48 |
| uring-sqpoll | 63 | 8197 | 0 |
| uring-sqpoll | 64 | 8202 | 13 |
| uring-sqpoll | 65 | 8196 | 13 |
| uring-sqpoll | 66 | 8206 | 48 |
| uring-sqpoll | 67 | 8206 | 13 |
| uring-sqpoll | 68 | 8196 | 13 |
| uring-sqpoll | 69 | 8207 | 0 |
| uring-sqpoll | 70 | 8196 | 48 |
| uring-sqpoll | 71 | 8207 | 0 |
| uring-sqpoll | 72 | 8197 | 13 |
| uring-sqpoll | 73 | 8193 | 0 |
| uring-sqpoll | 74 | 8198 | 13 |
| uring-sqpoll | 75 | 8205 | 13 |
| uring-sqpoll | 76 | 8199 | 13 |
| uring-sqpoll | 77 | 8208 | 13 |
| uring-sqpoll | 78 | 8199 | 0 |
| uring-sqpoll | 79 | 8201 | 13 |
| uring-sqpoll | 80 | 8195 | 0 |
| uring-sqpoll | 81 | 8194 | 13 |
| uring-sqpoll | 82 | 8208 | 48 |
| uring-sqpoll | 83 | 8195 | 13 |
| uring-sqpoll | 84 | 8194 | 13 |
| uring-sqpoll | 85 | 8198 | 13 |
| uring-sqpoll | 86 | 8201 | 0 |
| uring-sqpoll | 87 | 8197 | 13 |
| uring-sqpoll | 88 | 8202 | 13 |
| uring-sqpoll | 89 | 8206 | 13 |
| uring-sqpoll | 90 | 8202 | 48 |
| uring-sqpoll | 91 | 8197 | 13 |
| uring-sqpoll | 92 | 8205 | 13 |
| uring-sqpoll | 93 | 8204 | 48 |
| uring-sqpoll | 94 | 8201 | 13 |
| uring-sqpoll | 95 | 8198 | 13 |
| uring-sqpoll | 96 | 8199 | 13 |
| uring-sqpoll | 97 | 8205 | 0 |
| uring-sqpoll | 98 | 8201 | 0 |
| uring-sqpoll | 99 | 8201 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1018287872 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1644649 of 4386962 in 1-msg reads) | 1644649 | 4.94 | 4.46 | 7.14 | 11.86 | 21.77 | 31.89 | 51.76 |

#### Stalls: m1/lab-uring-sqpoll-c100, loop 0 on CPU 1

525294336 iterations, longest 641.4 µs; over 5 / 20 / 100 µs: 665 / 2 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1040.7 | 641.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 641.4 | 4753 | loop work (4753 items) |  |
| 2 | 399.3 | 2998 | loop work (2998 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1018 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-sib-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=103607 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-sib-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355496 | 132880 | 2.68 | 1 | 0.00 | 0 | 56105053 | 0.0 | 0.0 | true | 0 | 0 | 96.7 | 3 / 1 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8202 | 49 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79631104 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (62883 of 343094 in 1-msg reads) | 62883 | 5.20 | 5.24 | 5.54 | 6.33 | 9.88 | 18.87 | 34.18 |

#### Stalls: m1/lab-uring-sqpoll-sib-c1, loop 0 on CPU 1

3366303168 iterations, longest 96.7 µs; over 5 / 20 / 100 µs: 3 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 96.7 | 96.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 96.7 | 577 | loop work (577 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 16 |
| IWI IRQ work interrupts | 2 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-sib-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=453664 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-sib-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3164102 | 889546 | 3.56 | 1 | 0.00 | 0 | 27942491 | 0.0 | 0.0 | true | 0 | 0 | 849.3 | 537 / 1 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8205 | 0 |
| uring-sqpoll | 1 | 8198 | 48 |
| uring-sqpoll | 2 | 8205 | 0 |
| uring-sqpoll | 3 | 8204 | 49 |
| uring-sqpoll | 4 | 8208 | 48 |
| uring-sqpoll | 5 | 8201 | 0 |
| uring-sqpoll | 6 | 8197 | 0 |
| uring-sqpoll | 7 | 8196 | 48 |
| uring-sqpoll | 8 | 8197 | 49 |
| uring-sqpoll | 9 | 8206 | 48 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708758848 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (293740 of 3053556 in 1-msg reads) | 293740 | 5.79 | 5.22 | 8.36 | 13.37 | 23.59 | 39.16 | 55.25 |

#### Stalls: m1/lab-uring-sqpoll-sib-c10, loop 0 on CPU 1

1676549440 iterations, longest 849.3 µs; over 5 / 20 / 100 µs: 537 / 1 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 849.3 | 849.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 849.3 | 5271 | loop work (5271 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 16 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-sib-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=968040 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-sib-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4545910 | 2222519 | 2.05 | 1 | 0.00 | 0 | 4739999 | 0.0 | 0.0 | true | 0 | 0 | 782.4 | 977 / 2 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8206 | 49 |
| uring-sqpoll | 1 | 8202 | 48 |
| uring-sqpoll | 2 | 8207 | 49 |
| uring-sqpoll | 3 | 8200 | 49 |
| uring-sqpoll | 4 | 8207 | 0 |
| uring-sqpoll | 5 | 8203 | 49 |
| uring-sqpoll | 6 | 8207 | 0 |
| uring-sqpoll | 7 | 8200 | 48 |
| uring-sqpoll | 8 | 8207 | 0 |
| uring-sqpoll | 9 | 8206 | 48 |
| uring-sqpoll | 10 | 8200 | 49 |
| uring-sqpoll | 11 | 8203 | 49 |
| uring-sqpoll | 12 | 8195 | 0 |
| uring-sqpoll | 13 | 8199 | 0 |
| uring-sqpoll | 14 | 8198 | 49 |
| uring-sqpoll | 15 | 8201 | 0 |
| uring-sqpoll | 16 | 8205 | 0 |
| uring-sqpoll | 17 | 8194 | 49 |
| uring-sqpoll | 18 | 8200 | 49 |
| uring-sqpoll | 19 | 8202 | 49 |
| uring-sqpoll | 20 | 8206 | 49 |
| uring-sqpoll | 21 | 8193 | 49 |
| uring-sqpoll | 22 | 8200 | 49 |
| uring-sqpoll | 23 | 8207 | 49 |
| uring-sqpoll | 24 | 8204 | 49 |
| uring-sqpoll | 25 | 8205 | 0 |
| uring-sqpoll | 26 | 8208 | 48 |
| uring-sqpoll | 27 | 8194 | 48 |
| uring-sqpoll | 28 | 8201 | 49 |
| uring-sqpoll | 29 | 8208 | 48 |
| uring-sqpoll | 30 | 8195 | 49 |
| uring-sqpoll | 31 | 8201 | 0 |
| uring-sqpoll | 32 | 8208 | 48 |
| uring-sqpoll | 33 | 8195 | 49 |
| uring-sqpoll | 34 | 8198 | 48 |
| uring-sqpoll | 35 | 8208 | 49 |
| uring-sqpoll | 36 | 8204 | 48 |
| uring-sqpoll | 37 | 8203 | 49 |
| uring-sqpoll | 38 | 8206 | 48 |
| uring-sqpoll | 39 | 8193 | 0 |
| uring-sqpoll | 40 | 8207 | 0 |
| uring-sqpoll | 41 | 8200 | 49 |
| uring-sqpoll | 42 | 8197 | 0 |
| uring-sqpoll | 43 | 8206 | 48 |
| uring-sqpoll | 44 | 8206 | 49 |
| uring-sqpoll | 45 | 8193 | 0 |
| uring-sqpoll | 46 | 8196 | 49 |
| uring-sqpoll | 47 | 8203 | 0 |
| uring-sqpoll | 48 | 8193 | 49 |
| uring-sqpoll | 49 | 8207 | 49 |
| uring-sqpoll | 50 | 8202 | 48 |
| uring-sqpoll | 51 | 8203 | 0 |
| uring-sqpoll | 52 | 8196 | 48 |
| uring-sqpoll | 53 | 8207 | 0 |
| uring-sqpoll | 54 | 8200 | 49 |
| uring-sqpoll | 55 | 8199 | 0 |
| uring-sqpoll | 56 | 8194 | 49 |
| uring-sqpoll | 57 | 8205 | 49 |
| uring-sqpoll | 58 | 8201 | 0 |
| uring-sqpoll | 59 | 8199 | 0 |
| uring-sqpoll | 60 | 8208 | 48 |
| uring-sqpoll | 61 | 8194 | 48 |
| uring-sqpoll | 62 | 8195 | 0 |
| uring-sqpoll | 63 | 8199 | 0 |
| uring-sqpoll | 64 | 8201 | 49 |
| uring-sqpoll | 65 | 8199 | 0 |
| uring-sqpoll | 66 | 8199 | 49 |
| uring-sqpoll | 67 | 8195 | 0 |
| uring-sqpoll | 68 | 8198 | 48 |
| uring-sqpoll | 69 | 8204 | 49 |
| uring-sqpoll | 70 | 8205 | 49 |
| uring-sqpoll | 71 | 8208 | 48 |
| uring-sqpoll | 72 | 8197 | 0 |
| uring-sqpoll | 73 | 8202 | 49 |
| uring-sqpoll | 74 | 8196 | 49 |
| uring-sqpoll | 75 | 8200 | 49 |
| uring-sqpoll | 76 | 8206 | 49 |
| uring-sqpoll | 77 | 8207 | 0 |
| uring-sqpoll | 78 | 8193 | 49 |
| uring-sqpoll | 79 | 8207 | 0 |
| uring-sqpoll | 80 | 8198 | 48 |
| uring-sqpoll | 81 | 8199 | 0 |
| uring-sqpoll | 82 | 8201 | 49 |
| uring-sqpoll | 83 | 8194 | 48 |
| uring-sqpoll | 84 | 8198 | 49 |
| uring-sqpoll | 85 | 8195 | 0 |
| uring-sqpoll | 86 | 8194 | 48 |
| uring-sqpoll | 87 | 8197 | 0 |
| uring-sqpoll | 88 | 8207 | 0 |
| uring-sqpoll | 89 | 8200 | 48 |
| uring-sqpoll | 90 | 8206 | 48 |
| uring-sqpoll | 91 | 8203 | 0 |
| uring-sqpoll | 92 | 8202 | 48 |
| uring-sqpoll | 93 | 8203 | 49 |
| uring-sqpoll | 94 | 8196 | 49 |
| uring-sqpoll | 95 | 8206 | 48 |
| uring-sqpoll | 96 | 8193 | 0 |
| uring-sqpoll | 97 | 8203 | 49 |
| uring-sqpoll | 98 | 8196 | 49 |
| uring-sqpoll | 99 | 8197 | 49 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1018283840 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1589781 of 4387009 in 1-msg reads) | 1589781 | 6.47 | 5.91 | 9.34 | 16.82 | 29.62 | 42.53 | 72.00 |

#### Stalls: m1/lab-uring-sqpoll-sib-c100, loop 0 on CPU 1

284399936 iterations, longest 782.4 µs; over 5 / 20 / 100 µs: 977 / 2 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1267.5 | 782.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 782.4 | 4740 | loop work (4740 items) |  |
| 2 | 485.1 | 2955 | loop work (2955 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 16 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 355510 | 159920 | 2.22 | 357474096 | 1005.52 | 5957902 | 5955236 | 0.0 | 29.2 | false | 0 | 0 | 183.2 | 6682 / 50 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8200 | 48 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 79634240 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (86238 of 343095 in 1-msg reads) | 86238 | 3.57 | 2.94 | 4.83 | 9.39 | 14.54 | 24.68 | 36.65 |

#### Stalls: m2/lab-epoll-spin-c1, loop 0 on CPU 1

357314176 iterations, longest 183.2 µs; over 5 / 20 / 100 µs: 6682 / 50 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 48 | 1413.9 | 183.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 45.1 | 23.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 183.2 | 590 | loop work (590 items) |  |
| 2 | 36.7 | 7 | loop work (7 items) |  |
| 3 | 34.4 | 7 | loop work (7 items) |  |
| 4 | 33.9 | 3 | loop work (3 items) |  |
| 5 | 33.3 | 2 | loop work (2 items) |  |
| 6 | 32.2 | 7 | loop work (7 items) |  |
| 7 | 31.7 | 7 | loop work (7 items) |  |
| 8 | 31.7 | 7 | loop work (7 items) |  |
| 9 | 31.5 | 2 | loop work (2 items) |  |
| 10 | 31.3 | 7 | loop work (7 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1013 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3164103 | 1046827 | 3.02 | 343849643 | 108.67 | 5730827 | 5713380 | 0.0 | 31.1 | false | 0 | 0 | 1255.7 | 55600 / 93 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8193 | 0 |
| epoll-spin | 1 | 8207 | 0 |
| epoll-spin | 2 | 8193 | 0 |
| epoll-spin | 3 | 8203 | 0 |
| epoll-spin | 4 | 8207 | 0 |
| epoll-spin | 5 | 8197 | 0 |
| epoll-spin | 6 | 8194 | 48 |
| epoll-spin | 7 | 8208 | 48 |
| epoll-spin | 8 | 8201 | 0 |
| epoll-spin | 9 | 8198 | 48 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 708759072 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (422691 of 3053557 in 1-msg reads) | 422691 | 4.15 | 3.51 | 6.16 | 10.39 | 12.84 | 26.76 | 37.70 |

#### Stalls: m2/lab-epoll-spin-c10, loop 0 on CPU 1

342802816 iterations, longest 1255.7 µs; over 5 / 20 / 100 µs: 55600 / 93 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 51 | 2549.0 | 1255.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 42 | 1076.0 | 34.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1255.7 | 5286 | loop work (5286 items) |  |
| 2 | 34.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 33.9 | 7 | loop work (7 items) |  |
| 4 | 33.2 | 2 | loop work (2 items) |  |
| 5 | 33.1 | 2 | loop work (2 items) |  |
| 6 | 32.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 32.4 | 7 | loop work (7 items) |  |
| 8 | 31.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 31.7 | 7 | loop work (7 items) |  |
| 10 | 31.1 | 2 | loop work (2 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1014 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 4545913 | 2401641 | 1.89 | 263882985 | 58.05 | 4398050 | 4358022 | 0.0 | 28.6 | false | 0 | 0 | 1808.9 | 91945 / 365 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8199 | 0 |
| epoll-spin | 1 | 8199 | 0 |
| epoll-spin | 2 | 8198 | 48 |
| epoll-spin | 3 | 8204 | 48 |
| epoll-spin | 4 | 8195 | 0 |
| epoll-spin | 5 | 8193 | 0 |
| epoll-spin | 6 | 8200 | 48 |
| epoll-spin | 7 | 8196 | 48 |
| epoll-spin | 8 | 8203 | 0 |
| epoll-spin | 9 | 8200 | 48 |
| epoll-spin | 10 | 8193 | 0 |
| epoll-spin | 11 | 8196 | 48 |
| epoll-spin | 12 | 8197 | 0 |
| epoll-spin | 13 | 8196 | 48 |
| epoll-spin | 14 | 8207 | 0 |
| epoll-spin | 15 | 8207 | 0 |
| epoll-spin | 16 | 8203 | 0 |
| epoll-spin | 17 | 8207 | 0 |
| epoll-spin | 18 | 8199 | 0 |
| epoll-spin | 19 | 8201 | 0 |
| epoll-spin | 20 | 8194 | 48 |
| epoll-spin | 21 | 8208 | 48 |
| epoll-spin | 22 | 8204 | 48 |
| epoll-spin | 23 | 8198 | 48 |
| epoll-spin | 24 | 8200 | 48 |
| epoll-spin | 25 | 8206 | 48 |
| epoll-spin | 26 | 8197 | 0 |
| epoll-spin | 27 | 8193 | 0 |
| epoll-spin | 28 | 8196 | 48 |
| epoll-spin | 29 | 8204 | 48 |
| epoll-spin | 30 | 8208 | 48 |
| epoll-spin | 31 | 8195 | 0 |
| epoll-spin | 32 | 8195 | 0 |
| epoll-spin | 33 | 8195 | 0 |
| epoll-spin | 34 | 8199 | 0 |
| epoll-spin | 35 | 8194 | 48 |
| epoll-spin | 36 | 8198 | 48 |
| epoll-spin | 37 | 8194 | 48 |
| epoll-spin | 38 | 8201 | 0 |
| epoll-spin | 39 | 8204 | 48 |
| epoll-spin | 40 | 8198 | 48 |
| epoll-spin | 41 | 8194 | 48 |
| epoll-spin | 42 | 8199 | 0 |
| epoll-spin | 43 | 8205 | 0 |
| epoll-spin | 44 | 8208 | 48 |
| epoll-spin | 45 | 8206 | 48 |
| epoll-spin | 46 | 8203 | 0 |
| epoll-spin | 47 | 8200 | 48 |
| epoll-spin | 48 | 8206 | 48 |
| epoll-spin | 49 | 8202 | 48 |
| epoll-spin | 50 | 8200 | 48 |
| epoll-spin | 51 | 8203 | 0 |
| epoll-spin | 52 | 8193 | 0 |
| epoll-spin | 53 | 8207 | 0 |
| epoll-spin | 54 | 8202 | 48 |
| epoll-spin | 55 | 8197 | 0 |
| epoll-spin | 56 | 8206 | 48 |
| epoll-spin | 57 | 8203 | 0 |
| epoll-spin | 58 | 8202 | 48 |
| epoll-spin | 59 | 8197 | 0 |
| epoll-spin | 60 | 8207 | 0 |
| epoll-spin | 61 | 8198 | 48 |
| epoll-spin | 62 | 8195 | 0 |
| epoll-spin | 63 | 8195 | 0 |
| epoll-spin | 64 | 8198 | 48 |
| epoll-spin | 65 | 8199 | 0 |
| epoll-spin | 66 | 8204 | 48 |
| epoll-spin | 67 | 8208 | 48 |
| epoll-spin | 68 | 8201 | 0 |
| epoll-spin | 69 | 8205 | 0 |
| epoll-spin | 70 | 8201 | 0 |
| epoll-spin | 71 | 8201 | 0 |
| epoll-spin | 72 | 8198 | 48 |
| epoll-spin | 73 | 8199 | 0 |
| epoll-spin | 74 | 8200 | 48 |
| epoll-spin | 75 | 8206 | 48 |
| epoll-spin | 76 | 8196 | 48 |
| epoll-spin | 77 | 8193 | 0 |
| epoll-spin | 78 | 8197 | 0 |
| epoll-spin | 79 | 8203 | 0 |
| epoll-spin | 80 | 8193 | 0 |
| epoll-spin | 81 | 8200 | 48 |
| epoll-spin | 82 | 8208 | 48 |
| epoll-spin | 83 | 8208 | 48 |
| epoll-spin | 84 | 8201 | 0 |
| epoll-spin | 85 | 8199 | 0 |
| epoll-spin | 86 | 8198 | 48 |
| epoll-spin | 87 | 8201 | 0 |
| epoll-spin | 88 | 8201 | 0 |
| epoll-spin | 89 | 8207 | 0 |
| epoll-spin | 90 | 8206 | 48 |
| epoll-spin | 91 | 8207 | 0 |
| epoll-spin | 92 | 8202 | 48 |
| epoll-spin | 93 | 8203 | 0 |
| epoll-spin | 94 | 8206 | 48 |
| epoll-spin | 95 | 8200 | 48 |
| epoll-spin | 96 | 8203 | 0 |
| epoll-spin | 97 | 8206 | 48 |
| epoll-spin | 98 | 8202 | 48 |
| epoll-spin | 99 | 8207 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1018284512 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1740987 of 4387100 in 1-msg reads) | 1740987 | 3.49 | 2.95 | 5.17 | 8.88 | 12.28 | 28.13 | 39.38 |

#### Stalls: m2/lab-epoll-spin-c100, loop 0 on CPU 1

261481344 iterations, longest 1808.9 µs; over 5 / 20 / 100 µs: 91945 / 365 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 137 | 5950.4 | 1808.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 228 | 5884.4 | 35.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1808.9 | 5472 | loop work (5472 items) |  |
| 2 | 717.1 | 2266 | loop work (2266 items) |  |
| 3 | 35.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 35.5 | 7 | loop work (7 items) |  |
| 5 | 34.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 34.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 33.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 33.8 | 7 | loop work (7 items) |  |
| 9 | 33.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 33.1 | 7 | loop work (7 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1014 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 5 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=123058 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355521 | 144542 | 2.46 | 132178304 | 371.79 | 2202972 | 2202972 | 99.9 | 67.6 | true | 0 | 533 | 178.7 | 15710 / 586 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8204 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79636704 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (75291 of 343096 in 1-msg reads) | 75291 | 2.55 | 2.51 | 2.82 | 3.37 | 8.22 | 11.29 | 18.73 |

#### Stalls: m2/lab-recv-busy-c1, loop 0 on CPU 1

132178304 iterations, longest 178.7 µs; over 5 / 20 / 100 µs: 15710 / 586 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 475 | 11577.9 | 178.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 56 | 1198.4 | 26.9 |
| softirq TIMER | 21 | 500.0 | 30.3 |
| mostly unexplained (+softirq NET_RX) | 16 | 353.3 | 29.8 |
| mostly unexplained (+irq work (IPI/vector)) | 10 | 223.7 | 29.5 |
| softirq NET_RX | 8 | 181.7 | 25.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 178.7 | 601 | loop work (601 items) |  |
| 2 | 43.8 | 10 | loop work (10 items) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 40.4 | 10 | loop work (10 items) | softirq NET_RX 5 |
| 4 | 40.3 | 7 | loop work (7 items) | softirq NET_RX 1 |
| 5 | 38.8 | 10 | loop work (10 items) |  |
| 6 | 35.9 | 10 | loop work (10 items) |  |
| 7 | 35.7 | 2 | loop work (2 items) |  |
| 8 | 35.2 | 9 | loop work (9 items) |  |
| 9 | 35.0 | 10 | loop work (10 items) | softirq NET_RX 4 |
| 10 | 34.6 | 7 | loop work (7 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5493 |
| CAL Function call interrupts | 1017 |
| LOC Local timer interrupts | 621 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=276601 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3164103 | 909817 | 3.48 | 130333440 | 41.19 | 2172224 | 217222 | 99.3 | 69.7 | true | 0 | 1594 | 1252.3 | 710042 / 4543 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 0 |
| recv-busy | 1 | 8200 | 48 |
| recv-busy | 2 | 8202 | 48 |
| recv-busy | 3 | 8195 | 1 |
| recv-busy | 4 | 8198 | 48 |
| recv-busy | 5 | 8208 | 48 |
| recv-busy | 6 | 8199 | 1 |
| recv-busy | 7 | 8205 | 1 |
| recv-busy | 8 | 8194 | 48 |
| recv-busy | 9 | 8194 | 48 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708759072 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (308357 of 3053553 in 1-msg reads) | 308357 | 4.21 | 3.50 | 6.45 | 11.13 | 18.66 | 31.99 | 46.20 |

#### Stalls: m2/lab-recv-busy-c10, loop 0 on CPU 1

13033344 iterations, longest 1252.3 µs; over 5 / 20 / 100 µs: 710042 / 4543 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 3813 | 108872.2 | 1252.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 553 | 14768.2 | 61.6 |
| softirq TIMER | 141 | 3503.4 | 30.6 |
| mostly unexplained (+irq work (IPI/vector)) | 13 | 381.9 | 36.9 |
| mostly unexplained (+softirq NET_RX) | 13 | 371.1 | 35.9 |
| softirq NET_RX | 7 | 260.3 | 47.7 |
| mostly unexplained (+preempted by kworker/1:1) | 2 | 56.5 | 30.4 |
| mostly unexplained (+local timer tick) | 1 | 31.3 | 31.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1252.3 | 5313 | loop work (5313 items) |  |
| 2 | 66.7 | 30 | loop work (30 items) |  |
| 3 | 61.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 60.4 | 13 | loop work (13 items) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 59.3 | 23 | loop work (23 items) | softirq NET_RX 5 |
| 6 | 58.6 | 26 | loop work (26 items) |  |
| 7 | 57.5 | 38 | loop work (38 items) | preempted by kworker/1:1 1, workqueue [ena] 1 |
| 8 | 56.1 | 37 | loop work (37 items) |  |
| 9 | 54.6 | 19 | loop work (19 items) |  |
| 10 | 54.4 | 23 | loop work (23 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4641 |
| LOC Local timer interrupts | 1796 |
| CAL Function call interrupts | 1017 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=517670 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4546080 | 1975152 | 2.30 | 122291200 | 26.90 | 2038187 | 20382 | 98.4 | 71.1 | true | 0 | 4140 | 2466.9 | 1222911 / 1222911 / 322 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 48 |
| recv-busy | 1 | 8193 | 0 |
| recv-busy | 2 | 8196 | 1 |
| recv-busy | 3 | 8206 | 1 |
| recv-busy | 4 | 8198 | 48 |
| recv-busy | 5 | 8199 | 0 |
| recv-busy | 6 | 8194 | 48 |
| recv-busy | 7 | 8201 | 0 |
| recv-busy | 8 | 8205 | 0 |
| recv-busy | 9 | 8198 | 48 |
| recv-busy | 10 | 8199 | 0 |
| recv-busy | 11 | 8205 | 0 |
| recv-busy | 12 | 8204 | 48 |
| recv-busy | 13 | 8194 | 48 |
| recv-busy | 14 | 8199 | 1 |
| recv-busy | 15 | 8199 | 0 |
| recv-busy | 16 | 8208 | 48 |
| recv-busy | 17 | 8201 | 1 |
| recv-busy | 18 | 8207 | 0 |
| recv-busy | 19 | 8196 | 48 |
| recv-busy | 20 | 8193 | 0 |
| recv-busy | 21 | 8197 | 0 |
| recv-busy | 22 | 8196 | 48 |
| recv-busy | 23 | 8202 | 48 |
| recv-busy | 24 | 8206 | 48 |
| recv-busy | 25 | 8203 | 0 |
| recv-busy | 26 | 8200 | 1 |
| recv-busy | 27 | 8197 | 0 |
| recv-busy | 28 | 8196 | 1 |
| recv-busy | 29 | 8197 | 0 |
| recv-busy | 30 | 8201 | 1 |
| recv-busy | 31 | 8204 | 48 |
| recv-busy | 32 | 8199 | 0 |
| recv-busy | 33 | 8208 | 1 |
| recv-busy | 34 | 8201 | 1 |
| recv-busy | 35 | 8201 | 0 |
| recv-busy | 36 | 8199 | 0 |
| recv-busy | 37 | 8194 | 1 |
| recv-busy | 38 | 8195 | 0 |
| recv-busy | 39 | 8201 | 0 |
| recv-busy | 40 | 8199 | 0 |
| recv-busy | 41 | 8195 | 0 |
| recv-busy | 42 | 8199 | 0 |
| recv-busy | 43 | 8196 | 1 |
| recv-busy | 44 | 8200 | 1 |
| recv-busy | 45 | 8196 | 48 |
| recv-busy | 46 | 8197 | 0 |
| recv-busy | 47 | 8200 | 48 |
| recv-busy | 48 | 8193 | 0 |
| recv-busy | 49 | 8206 | 48 |
| recv-busy | 50 | 8206 | 48 |
| recv-busy | 51 | 8207 | 0 |
| recv-busy | 52 | 8203 | 0 |
| recv-busy | 53 | 8202 | 1 |
| recv-busy | 54 | 8202 | 48 |
| recv-busy | 55 | 8200 | 48 |
| recv-busy | 56 | 8206 | 48 |
| recv-busy | 57 | 8203 | 0 |
| recv-busy | 58 | 8199 | 0 |
| recv-busy | 59 | 8198 | 48 |
| recv-busy | 60 | 8205 | 0 |
| recv-busy | 61 | 8201 | 0 |
| recv-busy | 62 | 8208 | 48 |
| recv-busy | 63 | 8199 | 0 |
| recv-busy | 64 | 8208 | 48 |
| recv-busy | 65 | 8198 | 48 |
| recv-busy | 66 | 8195 | 0 |
| recv-busy | 67 | 8198 | 1 |
| recv-busy | 68 | 8199 | 0 |
| recv-busy | 69 | 8195 | 0 |
| recv-busy | 70 | 8194 | 48 |
| recv-busy | 71 | 8205 | 1 |
| recv-busy | 72 | 8201 | 0 |
| recv-busy | 73 | 8202 | 48 |
| recv-busy | 74 | 8196 | 48 |
| recv-busy | 75 | 8197 | 1 |
| recv-busy | 76 | 8203 | 0 |
| recv-busy | 77 | 8202 | 48 |
| recv-busy | 78 | 8197 | 1 |
| recv-busy | 79 | 8202 | 48 |
| recv-busy | 80 | 8195 | 0 |
| recv-busy | 81 | 8201 | 0 |
| recv-busy | 82 | 8205 | 1 |
| recv-busy | 83 | 8195 | 0 |
| recv-busy | 84 | 8200 | 48 |
| recv-busy | 85 | 8196 | 1 |
| recv-busy | 86 | 8206 | 1 |
| recv-busy | 87 | 8202 | 1 |
| recv-busy | 88 | 8206 | 1 |
| recv-busy | 89 | 8207 | 0 |
| recv-busy | 90 | 8193 | 0 |
| recv-busy | 91 | 8197 | 0 |
| recv-busy | 92 | 8200 | 1 |
| recv-busy | 93 | 8197 | 0 |
| recv-busy | 94 | 8203 | 0 |
| recv-busy | 95 | 8206 | 1 |
| recv-busy | 96 | 8207 | 0 |
| recv-busy | 97 | 8193 | 0 |
| recv-busy | 98 | 8198 | 1 |
| recv-busy | 99 | 8198 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1018321920 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1508350 of 4387244 in 1-msg reads) | 1508350 | 25.30 | 25.18 | 45.30 | 58.02 | 69.14 | 82.28 | 110.45 |

#### Stalls: m2/lab-recv-busy-c100, loop 0 on CPU 1

1222912 iterations, longest 2466.9 µs; over 5 / 20 / 100 µs: 1222911 / 1222911 / 322 (174335 stalls not kept)

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 818618 | 38393233.4 | 94.0 |
| loop work | 212155 | 12071767.3 | 2466.9 |
| mostly unexplained (+softirq NET_RX) | 11540 | 583742.2 | 94.2 |
| mostly unexplained (+local timer tick) | 3143 | 156345.8 | 83.9 |
| mostly unexplained (+irq work (IPI/vector)) | 2056 | 112207.9 | 96.5 |
| mostly unexplained (+preempted by kworker/1:1) | 870 | 49613.6 | 84.4 |
| mostly unexplained (+softirq TIMER) | 129 | 8908.5 | 82.1 |
| mostly unexplained (+call function single (IPI/vector)) | 27 | 1326.2 | 57.9 |
| mostly unexplained (+ipi_send_cpu) | 19 | 885.8 | 64.6 |
| mostly unexplained (+softirq RCU) | 8 | 401.0 | 54.5 |
| mostly unexplained (+call function (IPI/vector)) | 7 | 351.3 | 52.9 |
| mostly unexplained (+softirq NET_TX) | 4 | 247.1 | 91.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2466.9 | 7730 | loop work (7730 items) |  |
| 2 | 151.4 | 160 | loop work (160 items) |  |
| 3 | 137.9 | 66 | loop work (66 items) |  |
| 4 | 125.2 | 42 | loop work (42 items) |  |
| 5 | 122.7 | 51 | loop work (51 items) |  |
| 6 | 122.4 | 13 | loop work (13 items) | irq work (IPI/vector) 1, preempted by kworker/1:1 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 118.1 | 51 | loop work (51 items) |  |
| 8 | 117.7 | 67 | loop work (67 items) | irq work (IPI/vector) 1, preempted by kworker/1:1 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 117.1 | 89 | loop work (89 items) |  |
| 10 | 117.1 | 56 | loop work (56 items) | irq work (IPI/vector) 1, preempted by kworker/1:1 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5730 |
| LOC Local timer interrupts | 4225 |
| CAL Function call interrupts | 1056 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=125592 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355509 | 146616 | 2.42 | 195858432 | 550.92 | 3264307 | 1632154 | 99.9 | 59.0 | true | 0 | 594 | 180.4 | 19144 / 585 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8201 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (75358 of 343095 in 1-msg reads) | 75358 | 2.70 | 2.65 | 2.92 | 3.65 | 8.27 | 10.68 | 19.49 |

#### Stalls: m2/lab-recv-busy-pq-c1, loop 0 on CPU 1

97929216 iterations, longest 180.4 µs; over 5 / 20 / 100 µs: 19144 / 585 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 459 | 11415.4 | 180.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 90 | 1982.6 | 37.8 |
| mostly unexplained (+irq work (IPI/vector)) | 16 | 357.1 | 25.0 |
| mostly unexplained (+softirq NET_RX) | 14 | 305.7 | 30.8 |
| softirq TIMER | 3 | 64.6 | 24.5 |
| mostly unexplained (+preempted by kworker/1:1) | 2 | 50.8 | 27.0 |
| softirq NET_RX | 1 | 22.8 | 22.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 180.4 | 589 | loop work (589 items) |  |
| 2 | 39.6 | 9 | loop work (9 items) | softirq NET_RX 4, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 37.9 | 10 | loop work (10 items) |  |
| 4 | 37.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 35.1 | 2 | loop work (2 items) |  |
| 6 | 34.9 | 9 | loop work (9 items) | softirq NET_RX 2 |
| 7 | 34.6 | 10 | loop work (10 items) | softirq NET_RX 12, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 34.6 | 9 | loop work (9 items) |  |
| 9 | 34.4 | 10 | loop work (10 items) |  |
| 10 | 34.0 | 6 | loop work (6 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5450 |
| CAL Function call interrupts | 1013 |
| LOC Local timer interrupts | 747 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=280324 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3164104 | 913570 | 3.46 | 139345056 | 44.04 | 2322418 | 232112 | 99.3 | 68.4 | true | 0 | 1335 | 1321.7 | 643711 / 5179 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8206 | 1 |
| recv-busy | 1 | 8202 | 48 |
| recv-busy | 2 | 8193 | 1 |
| recv-busy | 3 | 8200 | 1 |
| recv-busy | 4 | 8196 | 48 |
| recv-busy | 5 | 8198 | 48 |
| recv-busy | 6 | 8199 | 0 |
| recv-busy | 7 | 8195 | 1 |
| recv-busy | 8 | 8199 | 0 |
| recv-busy | 9 | 8204 | 48 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708759296 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (314135 of 3053552 in 1-msg reads) | 314135 | 4.35 | 3.65 | 6.70 | 11.56 | 20.17 | 31.03 | 39.67 |

#### Stalls: m2/lab-recv-busy-pq-c10, loop 0 on CPU 1

13926720 iterations, longest 1321.7 µs; over 5 / 20 / 100 µs: 643711 / 5179 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4356 | 121237.7 | 1321.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 661 | 17484.8 | 59.3 |
| softirq TIMER | 112 | 2793.7 | 30.2 |
| mostly unexplained (+irq work (IPI/vector)) | 24 | 681.5 | 36.6 |
| mostly unexplained (+softirq NET_RX) | 17 | 458.2 | 39.9 |
| softirq NET_RX | 7 | 215.5 | 35.7 |
| mostly unexplained (+preempted by kworker/1:1) | 1 | 29.9 | 29.9 |
| mostly unexplained (+local timer tick) | 1 | 28.0 | 28.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1321.7 | 5356 | loop work (5356 items) |  |
| 2 | 64.0 | 43 | loop work (43 items) |  |
| 3 | 63.3 | 14 | loop work (14 items) | softirq NET_RX 2, irq work (IPI/vector) 1, preempted by kworker/1:1 1, workqueue [ena] 1 |
| 4 | 62.1 | 24 | loop work (24 items) |  |
| 5 | 62.1 | 27 | loop work (27 items) |  |
| 6 | 61.9 | 33 | loop work (33 items) |  |
| 7 | 61.3 | 23 | loop work (23 items) | softirq NET_RX 5, irq work (IPI/vector) 1, preempted by kworker/1:1 1, workqueue [ena] 1 |
| 8 | 60.8 | 26 | loop work (26 items) |  |
| 9 | 60.1 | 17 | loop work (17 items) |  |
| 10 | 59.8 | 34 | loop work (34 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4816 |
| LOC Local timer interrupts | 1473 |
| CAL Function call interrupts | 1028 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=552738 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4545936 | 2175615 | 2.09 | 124082485 | 27.30 | 2068041 | 119870 | 98.1 | 70.2 | true | 0 | 4623 | 2210.6 | 7192191 / 33273 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 48 |
| recv-busy | 1 | 8198 | 48 |
| recv-busy | 2 | 8199 | 0 |
| recv-busy | 3 | 8195 | 0 |
| recv-busy | 4 | 8199 | 1 |
| recv-busy | 5 | 8199 | 0 |
| recv-busy | 6 | 8193 | 0 |
| recv-busy | 7 | 8200 | 48 |
| recv-busy | 8 | 8207 | 1 |
| recv-busy | 9 | 8202 | 48 |
| recv-busy | 10 | 8193 | 1 |
| recv-busy | 11 | 8202 | 1 |
| recv-busy | 12 | 8197 | 1 |
| recv-busy | 13 | 8207 | 1 |
| recv-busy | 14 | 8202 | 48 |
| recv-busy | 15 | 8200 | 1 |
| recv-busy | 16 | 8206 | 1 |
| recv-busy | 17 | 8206 | 48 |
| recv-busy | 18 | 8203 | 1 |
| recv-busy | 19 | 8196 | 48 |
| recv-busy | 20 | 8200 | 48 |
| recv-busy | 21 | 8198 | 48 |
| recv-busy | 22 | 8195 | 0 |
| recv-busy | 23 | 8198 | 1 |
| recv-busy | 24 | 8199 | 0 |
| recv-busy | 25 | 8194 | 1 |
| recv-busy | 26 | 8208 | 48 |
| recv-busy | 27 | 8194 | 48 |
| recv-busy | 28 | 8201 | 1 |
| recv-busy | 29 | 8199 | 0 |
| recv-busy | 30 | 8198 | 1 |
| recv-busy | 31 | 8194 | 48 |
| recv-busy | 32 | 8201 | 0 |
| recv-busy | 33 | 8194 | 48 |
| recv-busy | 34 | 8197 | 0 |
| recv-busy | 35 | 8197 | 0 |
| recv-busy | 36 | 8202 | 48 |
| recv-busy | 37 | 8200 | 48 |
| recv-busy | 38 | 8207 | 0 |
| recv-busy | 39 | 8197 | 0 |
| recv-busy | 40 | 8196 | 48 |
| recv-busy | 41 | 8206 | 48 |
| recv-busy | 42 | 8203 | 0 |
| recv-busy | 43 | 8203 | 1 |
| recv-busy | 44 | 8206 | 48 |
| recv-busy | 45 | 8203 | 0 |
| recv-busy | 46 | 8202 | 48 |
| recv-busy | 47 | 8200 | 48 |
| recv-busy | 48 | 8197 | 1 |
| recv-busy | 49 | 8203 | 0 |
| recv-busy | 50 | 8199 | 0 |
| recv-busy | 51 | 8198 | 1 |
| recv-busy | 52 | 8195 | 0 |
| recv-busy | 53 | 8199 | 0 |
| recv-busy | 54 | 8195 | 0 |
| recv-busy | 55 | 8199 | 0 |
| recv-busy | 56 | 8207 | 1 |
| recv-busy | 57 | 8202 | 1 |
| recv-busy | 58 | 8203 | 0 |
| recv-busy | 59 | 8200 | 48 |
| recv-busy | 60 | 8200 | 48 |
| recv-busy | 61 | 8203 | 0 |
| recv-busy | 62 | 8203 | 0 |
| recv-busy | 63 | 8200 | 48 |
| recv-busy | 64 | 8198 | 48 |
| recv-busy | 65 | 8199 | 0 |
| recv-busy | 66 | 8205 | 0 |
| recv-busy | 67 | 8195 | 0 |
| recv-busy | 68 | 8199 | 0 |
| recv-busy | 69 | 8194 | 48 |
| recv-busy | 70 | 8205 | 0 |
| recv-busy | 71 | 8199 | 0 |
| recv-busy | 72 | 8208 | 1 |
| recv-busy | 73 | 8201 | 0 |
| recv-busy | 74 | 8204 | 48 |
| recv-busy | 75 | 8195 | 1 |
| recv-busy | 76 | 8194 | 48 |
| recv-busy | 77 | 8208 | 48 |
| recv-busy | 78 | 8201 | 0 |
| recv-busy | 79 | 8195 | 0 |
| recv-busy | 80 | 8204 | 48 |
| recv-busy | 81 | 8195 | 0 |
| recv-busy | 82 | 8194 | 48 |
| recv-busy | 83 | 8202 | 48 |
| recv-busy | 84 | 8193 | 0 |
| recv-busy | 85 | 8197 | 1 |
| recv-busy | 86 | 8202 | 48 |
| recv-busy | 87 | 8196 | 48 |
| recv-busy | 88 | 8193 | 1 |
| recv-busy | 89 | 8193 | 0 |
| recv-busy | 90 | 8200 | 48 |
| recv-busy | 91 | 8207 | 1 |
| recv-busy | 92 | 8193 | 1 |
| recv-busy | 93 | 8207 | 0 |
| recv-busy | 94 | 8207 | 1 |
| recv-busy | 95 | 8197 | 0 |
| recv-busy | 96 | 8200 | 48 |
| recv-busy | 97 | 8203 | 0 |
| recv-busy | 98 | 8206 | 48 |
| recv-busy | 99 | 8201 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1018289664 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1557496 of 4387112 in 1-msg reads) | 1557496 | 7.48 | 7.33 | 11.28 | 18.05 | 25.59 | 36.57 | 51.17 |

#### Stalls: m2/lab-recv-busy-pq-c100, loop 0 on CPU 1

7192192 iterations, longest 2210.6 µs; over 5 / 20 / 100 µs: 7192191 / 33273 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 31173 | 760440.2 | 2210.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1764 | 53257.5 | 46.5 |
| softirq TIMER | 191 | 5691.6 | 38.3 |
| mostly unexplained (+softirq NET_RX) | 43 | 1312.7 | 44.0 |
| mostly unexplained (+preempted by kworker/1:1) | 51 | 1207.0 | 44.6 |
| mostly unexplained (+irq work (IPI/vector)) | 32 | 879.8 | 42.0 |
| mostly unexplained (+local timer tick) | 10 | 258.7 | 33.4 |
| mostly unexplained (+softirq TIMER) | 7 | 163.5 | 31.0 |
| softirq NET_RX | 2 | 85.0 | 44.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2210.6 | 6478 | loop work (6478 items) |  |
| 2 | 455.4 | 1286 | loop work (1286 items) |  |
| 3 | 69.6 | 42 | loop work (42 items) | softirq NET_RX 4 |
| 4 | 68.6 | 52 | loop work (52 items) | softirq NET_RX 1 |
| 5 | 66.4 | 46 | loop work (46 items) |  |
| 6 | 66.0 | 49 | loop work (49 items) | softirq NET_RX 1 |
| 7 | 65.4 | 47 | loop work (47 items) | softirq NET_RX 1 |
| 8 | 65.0 | 50 | loop work (50 items) |  |
| 9 | 63.2 | 19 | loop work (19 items) |  |
| 10 | 63.1 | 64 | loop work (64 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6361 |
| LOC Local timer interrupts | 4761 |
| CAL Function call interrupts | 1066 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=119092 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355509 | 146256 | 2.43 | 1 | 0.00 | 0 | 71387794 | 0.0 | 0.0 | true | 0 | 0 | 81.8 | 1 / 1 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8198 | 13 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (76088 of 343095 in 1-msg reads) | 76088 | 3.15 | 3.13 | 3.38 | 3.89 | 4.61 | 6.00 | 17.05 |

#### Stalls: m2/lab-uring-sqpoll-c1, loop 0 on CPU 1

4283267648 iterations, longest 81.8 µs; over 5 / 20 / 100 µs: 1 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 81.8 | 81.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 81.8 | 589 | loop work (589 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1018 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=618574 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3164000 | 938820 | 3.37 | 1 | 0.00 | 0 | 43233022 | 0.0 | 0.0 | true | 0 | 0 | 674.9 | 472 / 1 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8204 | 13 |
| uring-sqpoll | 1 | 8198 | 13 |
| uring-sqpoll | 2 | 8199 | 13 |
| uring-sqpoll | 3 | 8201 | 13 |
| uring-sqpoll | 4 | 8194 | 13 |
| uring-sqpoll | 5 | 8205 | 13 |
| uring-sqpoll | 6 | 8194 | 13 |
| uring-sqpoll | 7 | 8201 | 13 |
| uring-sqpoll | 8 | 8208 | 13 |
| uring-sqpoll | 9 | 8204 | 13 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708736000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (327916 of 3053455 in 1-msg reads) | 327916 | 4.64 | 3.97 | 6.73 | 11.22 | 17.81 | 30.50 | 49.70 |

#### Stalls: m2/lab-uring-sqpoll-c10, loop 0 on CPU 1

2593981312 iterations, longest 674.9 µs; over 5 / 20 / 100 µs: 472 / 1 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 674.9 | 674.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 674.9 | 5194 | loop work (5194 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1018 |
| LOC Local timer interrupts | 8 |
| IWI IRQ work interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1311026 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4545924 | 2288634 | 1.99 | 1 | 0.00 | 0 | 8444745 | 0.0 | 0.0 | true | 0 | 0 | 646.3 | 731 / 2 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8206 | 48 |
| uring-sqpoll | 1 | 8193 | 13 |
| uring-sqpoll | 2 | 8200 | 13 |
| uring-sqpoll | 3 | 8196 | 13 |
| uring-sqpoll | 4 | 8196 | 13 |
| uring-sqpoll | 5 | 8197 | 0 |
| uring-sqpoll | 6 | 8199 | 13 |
| uring-sqpoll | 7 | 8204 | 13 |
| uring-sqpoll | 8 | 8198 | 13 |
| uring-sqpoll | 9 | 8199 | 13 |
| uring-sqpoll | 10 | 8201 | 13 |
| uring-sqpoll | 11 | 8195 | 13 |
| uring-sqpoll | 12 | 8204 | 48 |
| uring-sqpoll | 13 | 8201 | 0 |
| uring-sqpoll | 14 | 8195 | 13 |
| uring-sqpoll | 15 | 8198 | 48 |
| uring-sqpoll | 16 | 8204 | 48 |
| uring-sqpoll | 17 | 8199 | 13 |
| uring-sqpoll | 18 | 8198 | 13 |
| uring-sqpoll | 19 | 8194 | 13 |
| uring-sqpoll | 20 | 8205 | 0 |
| uring-sqpoll | 21 | 8204 | 13 |
| uring-sqpoll | 22 | 8194 | 13 |
| uring-sqpoll | 23 | 8204 | 13 |
| uring-sqpoll | 24 | 8208 | 13 |
| uring-sqpoll | 25 | 8207 | 13 |
| uring-sqpoll | 26 | 8203 | 13 |
| uring-sqpoll | 27 | 8206 | 13 |
| uring-sqpoll | 28 | 8197 | 0 |
| uring-sqpoll | 29 | 8206 | 48 |
| uring-sqpoll | 30 | 8202 | 48 |
| uring-sqpoll | 31 | 8206 | 13 |
| uring-sqpoll | 32 | 8197 | 0 |
| uring-sqpoll | 33 | 8196 | 13 |
| uring-sqpoll | 34 | 8197 | 0 |
| uring-sqpoll | 35 | 8193 | 0 |
| uring-sqpoll | 36 | 8203 | 13 |
| uring-sqpoll | 37 | 8203 | 13 |
| uring-sqpoll | 38 | 8207 | 13 |
| uring-sqpoll | 39 | 8202 | 13 |
| uring-sqpoll | 40 | 8196 | 13 |
| uring-sqpoll | 41 | 8206 | 48 |
| uring-sqpoll | 42 | 8193 | 13 |
| uring-sqpoll | 43 | 8195 | 0 |
| uring-sqpoll | 44 | 8204 | 48 |
| uring-sqpoll | 45 | 8204 | 13 |
| uring-sqpoll | 46 | 8199 | 0 |
| uring-sqpoll | 47 | 8205 | 0 |
| uring-sqpoll | 48 | 8204 | 48 |
| uring-sqpoll | 49 | 8198 | 13 |
| uring-sqpoll | 50 | 8195 | 13 |
| uring-sqpoll | 51 | 8201 | 13 |
| uring-sqpoll | 52 | 8198 | 48 |
| uring-sqpoll | 53 | 8198 | 13 |
| uring-sqpoll | 54 | 8204 | 48 |
| uring-sqpoll | 55 | 8205 | 13 |
| uring-sqpoll | 56 | 8206 | 13 |
| uring-sqpoll | 57 | 8197 | 13 |
| uring-sqpoll | 58 | 8200 | 13 |
| uring-sqpoll | 59 | 8203 | 0 |
| uring-sqpoll | 60 | 8193 | 13 |
| uring-sqpoll | 61 | 8197 | 13 |
| uring-sqpoll | 62 | 8196 | 48 |
| uring-sqpoll | 63 | 8206 | 13 |
| uring-sqpoll | 64 | 8204 | 13 |
| uring-sqpoll | 65 | 8205 | 13 |
| uring-sqpoll | 66 | 8195 | 13 |
| uring-sqpoll | 67 | 8198 | 13 |
| uring-sqpoll | 68 | 8204 | 48 |
| uring-sqpoll | 69 | 8205 | 13 |
| uring-sqpoll | 70 | 8204 | 48 |
| uring-sqpoll | 71 | 8205 | 13 |
| uring-sqpoll | 72 | 8201 | 13 |
| uring-sqpoll | 73 | 8203 | 0 |
| uring-sqpoll | 74 | 8200 | 13 |
| uring-sqpoll | 75 | 8197 | 0 |
| uring-sqpoll | 76 | 8206 | 13 |
| uring-sqpoll | 77 | 8196 | 48 |
| uring-sqpoll | 78 | 8207 | 0 |
| uring-sqpoll | 79 | 8197 | 13 |
| uring-sqpoll | 80 | 8207 | 13 |
| uring-sqpoll | 81 | 8202 | 13 |
| uring-sqpoll | 82 | 8196 | 13 |
| uring-sqpoll | 83 | 8193 | 13 |
| uring-sqpoll | 84 | 8196 | 13 |
| uring-sqpoll | 85 | 8206 | 13 |
| uring-sqpoll | 86 | 8207 | 13 |
| uring-sqpoll | 87 | 8203 | 0 |
| uring-sqpoll | 88 | 8200 | 13 |
| uring-sqpoll | 89 | 8206 | 13 |
| uring-sqpoll | 90 | 8206 | 48 |
| uring-sqpoll | 91 | 8199 | 13 |
| uring-sqpoll | 92 | 8205 | 13 |
| uring-sqpoll | 93 | 8194 | 13 |
| uring-sqpoll | 94 | 8201 | 13 |
| uring-sqpoll | 95 | 8205 | 13 |
| uring-sqpoll | 96 | 8201 | 0 |
| uring-sqpoll | 97 | 8194 | 48 |
| uring-sqpoll | 98 | 8204 | 48 |
| uring-sqpoll | 99 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1018286976 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1642284 of 4386962 in 1-msg reads) | 1642284 | 5.06 | 4.57 | 7.23 | 11.85 | 21.70 | 32.21 | 53.07 |

#### Stalls: m2/lab-uring-sqpoll-c100, loop 0 on CPU 1

506684672 iterations, longest 646.3 µs; over 5 / 20 / 100 µs: 731 / 2 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1046.6 | 646.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 646.3 | 4751 | loop work (4751 items) |  |
| 2 | 400.3 | 2995 | loop work (2995 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1018 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-sib-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=105940 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-sib-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355509 | 133430 | 2.66 | 1 | 0.00 | 0 | 56039831 | 0.0 | 0.0 | true | 0 | 0 | 99.2 | 2 / 1 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8206 | 49 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (66751 of 343095 in 1-msg reads) | 66751 | 4.92 | 4.95 | 5.28 | 5.94 | 9.34 | 18.16 | 35.00 |

#### Stalls: m2/lab-uring-sqpoll-sib-c1, loop 0 on CPU 1

3362389888 iterations, longest 99.2 µs; over 5 / 20 / 100 µs: 2 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 99.2 | 99.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 99.2 | 589 | loop work (589 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 16 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-sib-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=463374 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-sib-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3164105 | 899065 | 3.52 | 1 | 0.00 | 0 | 27950556 | 0.0 | 0.0 | true | 0 | 0 | 866.8 | 537 / 1 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8200 | 49 |
| uring-sqpoll | 1 | 8193 | 0 |
| uring-sqpoll | 2 | 8203 | 49 |
| uring-sqpoll | 3 | 8196 | 48 |
| uring-sqpoll | 4 | 8196 | 49 |
| uring-sqpoll | 5 | 8203 | 49 |
| uring-sqpoll | 6 | 8202 | 48 |
| uring-sqpoll | 7 | 8203 | 49 |
| uring-sqpoll | 8 | 8204 | 49 |
| uring-sqpoll | 9 | 8201 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708759520 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (297224 of 3053453 in 1-msg reads) | 297224 | 5.73 | 5.21 | 8.11 | 13.13 | 24.10 | 40.53 | 59.26 |

#### Stalls: m2/lab-uring-sqpoll-sib-c10, loop 0 on CPU 1

1677033344 iterations, longest 866.8 µs; over 5 / 20 / 100 µs: 537 / 1 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 866.8 | 866.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 866.8 | 5366 | loop work (5366 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 16 |
| IWI IRQ work interrupts | 2 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-sib-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1003000 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-sib-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4545775 | 2224881 | 2.04 | 1 | 0.00 | 0 | 4737913 | 0.0 | 0.0 | true | 0 | 0 | 767.3 | 1057 / 2 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8199 | 49 |
| uring-sqpoll | 1 | 8208 | 48 |
| uring-sqpoll | 2 | 8194 | 48 |
| uring-sqpoll | 3 | 8198 | 48 |
| uring-sqpoll | 4 | 8204 | 49 |
| uring-sqpoll | 5 | 8208 | 49 |
| uring-sqpoll | 6 | 8201 | 49 |
| uring-sqpoll | 7 | 8198 | 48 |
| uring-sqpoll | 8 | 8195 | 49 |
| uring-sqpoll | 9 | 8200 | 49 |
| uring-sqpoll | 10 | 8196 | 48 |
| uring-sqpoll | 11 | 8196 | 48 |
| uring-sqpoll | 12 | 8196 | 48 |
| uring-sqpoll | 13 | 8207 | 0 |
| uring-sqpoll | 14 | 8206 | 48 |
| uring-sqpoll | 15 | 8193 | 49 |
| uring-sqpoll | 16 | 8203 | 0 |
| uring-sqpoll | 17 | 8197 | 49 |
| uring-sqpoll | 18 | 8193 | 49 |
| uring-sqpoll | 19 | 8203 | 0 |
| uring-sqpoll | 20 | 8196 | 49 |
| uring-sqpoll | 21 | 8202 | 48 |
| uring-sqpoll | 22 | 8205 | 0 |
| uring-sqpoll | 23 | 8199 | 49 |
| uring-sqpoll | 24 | 8195 | 0 |
| uring-sqpoll | 25 | 8199 | 0 |
| uring-sqpoll | 26 | 8198 | 49 |
| uring-sqpoll | 27 | 8195 | 49 |
| uring-sqpoll | 28 | 8193 | 0 |
| uring-sqpoll | 29 | 8207 | 0 |
| uring-sqpoll | 30 | 8193 | 0 |
| uring-sqpoll | 31 | 8206 | 48 |
| uring-sqpoll | 32 | 8207 | 0 |
| uring-sqpoll | 33 | 8196 | 48 |
| uring-sqpoll | 34 | 8194 | 48 |
| uring-sqpoll | 35 | 8201 | 49 |
| uring-sqpoll | 36 | 8199 | 49 |
| uring-sqpoll | 37 | 8199 | 49 |
| uring-sqpoll | 38 | 8194 | 48 |
| uring-sqpoll | 39 | 8208 | 49 |
| uring-sqpoll | 40 | 8199 | 49 |
| uring-sqpoll | 41 | 8204 | 49 |
| uring-sqpoll | 42 | 8198 | 49 |
| uring-sqpoll | 43 | 8201 | 49 |
| uring-sqpoll | 44 | 8195 | 0 |
| uring-sqpoll | 45 | 8204 | 48 |
| uring-sqpoll | 46 | 8199 | 0 |
| uring-sqpoll | 47 | 8193 | 0 |
| uring-sqpoll | 48 | 8197 | 0 |
| uring-sqpoll | 49 | 8197 | 49 |
| uring-sqpoll | 50 | 8207 | 49 |
| uring-sqpoll | 51 | 8206 | 48 |
| uring-sqpoll | 52 | 8207 | 0 |
| uring-sqpoll | 53 | 8193 | 0 |
| uring-sqpoll | 54 | 8203 | 0 |
| uring-sqpoll | 55 | 8202 | 48 |
| uring-sqpoll | 56 | 8196 | 49 |
| uring-sqpoll | 57 | 8207 | 0 |
| uring-sqpoll | 58 | 8207 | 49 |
| uring-sqpoll | 59 | 8206 | 49 |
| uring-sqpoll | 60 | 8199 | 49 |
| uring-sqpoll | 61 | 8208 | 49 |
| uring-sqpoll | 62 | 8198 | 48 |
| uring-sqpoll | 63 | 8204 | 48 |
| uring-sqpoll | 64 | 8201 | 49 |
| uring-sqpoll | 65 | 8205 | 49 |
| uring-sqpoll | 66 | 8204 | 49 |
| uring-sqpoll | 67 | 8198 | 49 |
| uring-sqpoll | 68 | 8199 | 49 |
| uring-sqpoll | 69 | 8195 | 0 |
| uring-sqpoll | 70 | 8204 | 49 |
| uring-sqpoll | 71 | 8199 | 49 |
| uring-sqpoll | 72 | 8201 | 49 |
| uring-sqpoll | 73 | 8194 | 48 |
| uring-sqpoll | 74 | 8204 | 48 |
| uring-sqpoll | 75 | 8196 | 49 |
| uring-sqpoll | 76 | 8207 | 49 |
| uring-sqpoll | 77 | 8203 | 0 |
| uring-sqpoll | 78 | 8203 | 0 |
| uring-sqpoll | 79 | 8206 | 48 |
| uring-sqpoll | 80 | 8203 | 49 |
| uring-sqpoll | 81 | 8194 | 48 |
| uring-sqpoll | 82 | 8198 | 49 |
| uring-sqpoll | 83 | 8204 | 49 |
| uring-sqpoll | 84 | 8208 | 48 |
| uring-sqpoll | 85 | 8198 | 49 |
| uring-sqpoll | 86 | 8204 | 49 |
| uring-sqpoll | 87 | 8195 | 49 |
| uring-sqpoll | 88 | 8198 | 48 |
| uring-sqpoll | 89 | 8203 | 0 |
| uring-sqpoll | 90 | 8202 | 48 |
| uring-sqpoll | 91 | 8207 | 0 |
| uring-sqpoll | 92 | 8202 | 48 |
| uring-sqpoll | 93 | 8196 | 48 |
| uring-sqpoll | 94 | 8202 | 48 |
| uring-sqpoll | 95 | 8203 | 0 |
| uring-sqpoll | 96 | 8197 | 49 |
| uring-sqpoll | 97 | 8196 | 48 |
| uring-sqpoll | 98 | 8206 | 48 |
| uring-sqpoll | 99 | 8206 | 48 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1018253600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1593369 of 4386955 in 1-msg reads) | 1593369 | 6.55 | 5.95 | 9.41 | 17.96 | 32.97 | 45.02 | 80.83 |

#### Stalls: m2/lab-uring-sqpoll-sib-c100, loop 0 on CPU 1

284274752 iterations, longest 767.3 µs; over 5 / 20 / 100 µs: 1057 / 2 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1246.4 | 767.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 767.3 | 4697 | loop work (4697 items) |  |
| 2 | 479.1 | 2918 | loop work (2918 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 16 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| TLB TLB shootdowns | 1 |

## Ping-pong round trip (client send → echo decoded; "from when due": from the slot on the paced schedule, including any wait behind a stall)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189938 | 40.51 | 39.63 | 44.73 | 54.66 | 67.87 | 80.21 | 1151.51 |
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189938 | 40.66 | 39.75 | 44.85 | 54.81 | 68.22 | 86.64 | 1151.62 |
| r1-base-0/pp-epoll-spin-0: epoll-spin @ 0/s | 1400975 | 42.03 | 41.06 | 47.81 | 56.24 | 65.01 | 99.18 | 379.65 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 40.71 | 39.96 | 43.81 | 50.81 | 60.40 | 98.70 | 927.82 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 40.84 | 40.08 | 43.93 | 50.95 | 60.75 | 106.75 | 927.95 |
| r1-base-0/pp-recv-busy-0: recv-busy @ 0/s | 1650555 | 35.67 | 35.28 | 38.42 | 43.27 | 55.74 | 64.39 | 206.07 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 37.36 | 37.05 | 39.98 | 43.89 | 56.13 | 64.26 | 112.86 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 37.61 | 37.30 | 40.24 | 44.15 | 56.41 | 64.45 | 112.92 |
| r1-base-0/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1449905 | 40.50 | 39.68 | 45.79 | 53.82 | 63.28 | 88.76 | 429.02 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 38.84 | 38.15 | 42.68 | 50.23 | 59.06 | 72.66 | 169.22 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 38.90 | 38.20 | 42.74 | 50.28 | 59.13 | 72.88 | 169.26 |
| r1-base-0/pp-uring-sqpoll-sib-0: uring-sqpoll @ 0/s | 1516530 | 38.78 | 38.38 | 41.56 | 46.40 | 58.61 | 67.40 | 859.17 |
| r1-base-0/pp-uring-sqpoll-sib-10000: uring-sqpoll @ 10000/s | 1189999 | 42.80 | 40.95 | 48.32 | 74.04 | 100.59 | 119.91 | 765.76 |
| r1-base-0/pp-uring-sqpoll-sib-10000: uring-sqpoll @ 10000/s, from when due | 1189999 | 42.87 | 41.01 | 48.40 | 74.31 | 102.95 | 125.75 | 765.81 |
| r1-base-1/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189938 | 40.12 | 39.58 | 42.93 | 51.17 | 66.53 | 75.23 | 127.66 |
| r1-base-1/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189938 | 40.25 | 39.72 | 43.06 | 51.31 | 66.67 | 75.37 | 127.80 |
| r1-base-1/pp-epoll-spin-0: epoll-spin @ 0/s | 1433115 | 41.09 | 39.44 | 46.52 | 67.27 | 91.77 | 108.19 | 193.31 |
| r1-base-1/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 41.88 | 41.18 | 44.99 | 51.13 | 62.53 | 77.04 | 148.68 |
| r1-base-1/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 41.99 | 41.29 | 45.10 | 51.25 | 62.66 | 77.22 | 148.75 |
| r1-base-1/pp-recv-busy-0: recv-busy @ 0/s | 1634597 | 36.02 | 35.66 | 38.71 | 43.44 | 55.15 | 64.46 | 159.93 |
| r1-base-1/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 38.15 | 37.82 | 40.65 | 44.84 | 57.35 | 65.06 | 962.59 |
| r1-base-1/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 38.41 | 38.07 | 40.91 | 45.11 | 57.78 | 68.12 | 962.68 |
| r1-base-1/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1526877 | 38.39 | 38.05 | 40.97 | 45.46 | 57.06 | 65.92 | 331.87 |
| r1-base-1/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 40.81 | 40.18 | 45.19 | 52.69 | 61.89 | 83.03 | 851.80 |
| r1-base-1/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 40.88 | 40.23 | 45.25 | 52.77 | 62.21 | 93.69 | 1023.08 |
| r1-base-1/pp-uring-sqpoll-sib-0: uring-sqpoll @ 0/s | 1540377 | 38.18 | 37.81 | 40.82 | 45.32 | 57.27 | 67.18 | 312.20 |
| r1-base-1/pp-uring-sqpoll-sib-10000: uring-sqpoll @ 10000/s | 1189999 | 41.56 | 41.26 | 44.14 | 48.01 | 61.16 | 70.91 | 881.04 |
| r1-base-1/pp-uring-sqpoll-sib-10000: uring-sqpoll @ 10000/s, from when due | 1189999 | 41.63 | 41.32 | 44.20 | 48.08 | 61.42 | 72.68 | 881.10 |

