# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

**Runs not included** (they could not be loaded):

- `m1/lab-recv-busy-c1`: meta.txt: No such file or directory (os error 2)

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343095 | 4.27 | 18.81 | 22.30 | 28.93 | 31.03 | 0.0 | 0.0 | 0.0 | 0.0 | 189.91 | 5.0 | 189.91 µs stall: loop work (589 items) |
| m2 | 343094 | 4.55 | 19.49 | 27.04 | 29.09 | 31.76 | 0.0 | 0.0 | 0.0 | 0.0 | 172.91 | 11.0 | 172.91 µs stall: loop work (589 items) |
| m3 | 343085 | 10.37 | 20.78 | 21.86 | 24.50 | 40.51 | 0.0 | 0.0 | 0.0 | 0.0 | 209.12 | 6.0 | 209.12 µs stall: loop work (593 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343095 | 36.99 | 87.62 | 160.54 | 201.65 | 254.66 | 2410.4 | 0.0 | 0.0 | 0.0 |
| m2 | 343094 | 60.66 | 143.68 | 181.28 | 210.82 | 266.60 | 43259.1 | 0.0 | 0.0 | 0.0 |
| m3 | 343085 | 63.42 | 194.88 | 249.04 | 289.23 | 329.11 | 138146.9 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053557 | 7.05 | 21.83 | 28.48 | 39.01 | 54.62 | 0.0 | 0.0 | 0.0 | 0.0 | 1388.73 | 125.0 | 1388.73 µs stall: loop work (5272 items) |
| m2 | 3053557 | 6.85 | 19.58 | 25.27 | 30.55 | 39.59 | 0.0 | 0.0 | 0.0 | 0.0 | 1464.35 | 97.0 | 1464.35 µs stall: loop work (5272 items) |
| m3 | 3053556 | 6.67 | 17.53 | 22.56 | 31.37 | 43.73 | 0.0 | 0.0 | 0.0 | 0.0 | 1414.87 | 79.0 | 1414.87 µs stall: loop work (5283 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053557 | 54.69 | 110.68 | 125.78 | 141.30 | 174.02 | 118600.2 | 0.0 | 0.0 | 0.0 |
| m2 | 3053557 | 52.60 | 108.00 | 122.36 | 139.93 | 160.37 | 82385.8 | 0.0 | 0.0 | 0.0 |
| m3 | 3053556 | 66.57 | 119.77 | 133.97 | 151.88 | 190.55 | 192519.6 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387088 | 6.00 | 19.92 | 26.42 | 33.63 | 174.85 | 8.3 | 0.0 | 0.0 | 0.0 | 1980.86 | 261.0 | 1980.86 µs stall: loop work (5435 items) |
| m2 | 4387089 | 6.08 | 22.50 | 30.51 | 38.14 | 60.64 | 0.0 | 0.0 | 0.0 | 0.0 | 1916.65 | 758.0 | 1916.65 µs stall: loop work (5419 items) |
| m3 | 4387098 | 5.84 | 19.29 | 25.78 | 34.40 | 318.94 | 6.2 | 0.0 | 0.0 | 0.0 | 1984.51 | 243.0 | 1984.51 µs stall: loop work (5459 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387088 | 37.26 | 109.64 | 127.50 | 147.71 | 239.82 | 102332.5 | 0.0 | 0.0 | 0.0 |
| m2 | 4387089 | 39.71 | 111.02 | 132.99 | 181.51 | 274.98 | 112663.8 | 0.0 | 0.0 | 0.0 |
| m3 | 4387098 | 38.41 | 110.67 | 128.34 | 147.48 | 360.72 | 107236.8 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m2 | 343095 | 2.59 | 7.40 | 19.01 | 25.71 | 31.14 | 0.0 | 0.0 | 0.0 | 0.0 | 178.87 | 1185.0 | 178.87 µs stall: loop work (589 items) |
| m3 | 343095 | 2.88 | 5.98 | 21.80 | 35.41 | 41.08 | 0.0 | 0.0 | 0.0 | 0.0 | 205.24 | 1396.0 | 205.24 µs stall: loop work (589 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m2 | 343095 | 17.44 | 40.19 | 49.58 | 62.14 | 77.02 | 0.0 | 0.0 | 0.0 | 0.0 |
| m3 | 343095 | 25.52 | 51.59 | 65.73 | 74.22 | 87.14 | 0.0 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053557 | 5.13 | 25.90 | 45.36 | 61.73 | 99.20 | 0.0 | 0.0 | 0.0 | 0.0 | 1204.65 | 26657.0 | 1204.65 µs stall: loop work (5271 items) |
| m2 | 3053557 | 5.13 | 25.62 | 44.90 | 57.67 | 84.86 | 0.0 | 0.0 | 0.0 | 0.0 | 1421.48 | 25417.0 | 1421.48 µs stall: loop work (5292 items) |
| m3 | 3053553 | 5.00 | 29.81 | 51.02 | 69.48 | 145.56 | 36.2 | 0.0 | 0.0 | 0.0 | 1548.73 | 39567.0 | 1548.73 µs stall: loop work (5315 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053557 | 35.77 | 87.50 | 106.07 | 120.71 | 142.82 | 6881.4 | 0.0 | 0.0 | 0.0 |
| m2 | 3053557 | 34.66 | 85.19 | 104.24 | 121.33 | 140.63 | 5319.3 | 0.0 | 0.0 | 0.0 |
| m3 | 3053553 | 33.90 | 87.90 | 111.92 | 175.95 | 247.66 | 9713.9 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387235 | 36.60 | 107.59 | 129.25 | 148.16 | 186.05 | 93036.0 | 0.0 | 0.0 | 0.0 | 2538.43 | 1083135.0 | 2538.43 µs stall: loop work (7601 items) |
| m2 | 4387357 | 35.63 | 105.76 | 127.84 | 147.29 | 179.04 | 77980.2 | 0.0 | 0.0 | 0.0 | 2632.09 | 1082751.0 | 2632.09 µs stall: loop work (7604 items) |
| m3 | 4387311 | 34.83 | 105.27 | 128.19 | 145.35 | 171.07 | 75927.9 | 0.0 | 0.0 | 0.0 | 2648.08 | 1085951.0 | 2648.08 µs stall: loop work (7627 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387235 | 67.94 | 154.87 | 176.87 | 196.31 | 237.60 | 1100120.9 | 0.0 | 0.0 | 0.0 |
| m2 | 4387357 | 72.29 | 159.17 | 181.11 | 197.72 | 233.32 | 1223425.7 | 0.0 | 0.0 | 0.0 |
| m3 | 4387311 | 79.32 | 166.31 | 188.34 | 206.90 | 231.57 | 1465009.7 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343095 | 2.81 | 6.00 | 18.97 | 30.19 | 33.41 | 0.0 | 0.0 | 0.0 | 0.0 | 154.96 | 1231.0 | 154.96 µs stall: loop work (589 items) |
| m2 | 343095 | 2.87 | 6.07 | 20.35 | 35.26 | 41.48 | 0.0 | 0.0 | 0.0 | 0.0 | 302.43 | 1292.0 | 302.43 µs stall: softirq TIMER |
| m3 | 343094 | 2.86 | 5.90 | 20.03 | 32.24 | 36.61 | 0.0 | 0.0 | 0.0 | 0.0 | 182.21 | 1366.0 | 182.21 µs stall: loop work (589 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343095 | 33.17 | 53.01 | 62.23 | 76.16 | 88.21 | 0.0 | 0.0 | 0.0 | 0.0 |
| m2 | 343095 | 16.50 | 39.07 | 48.23 | 61.82 | 74.10 | 0.0 | 0.0 | 0.0 | 0.0 |
| m3 | 343094 | 20.91 | 65.49 | 75.47 | 87.97 | 103.52 | 2.1 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053557 | 5.26 | 24.30 | 43.03 | 55.57 | 77.67 | 0.0 | 0.0 | 0.0 | 0.0 | 1349.56 | 15713.0 | 1349.56 µs stall: loop work (5271 items) |
| m2 | 3053556 | 4.44 | 25.16 | 43.14 | 54.51 | 77.75 | 0.0 | 0.0 | 0.0 | 0.0 | 1458.77 | 19386.0 | 1458.77 µs stall: loop work (5272 items) |
| m3 | 3053552 | 5.28 | 27.82 | 47.33 | 62.57 | 95.15 | 0.0 | 0.0 | 0.0 | 0.0 | 1561.04 | 24527.0 | 1561.04 µs stall: loop work (5355 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053557 | 36.82 | 89.83 | 109.45 | 126.26 | 151.30 | 9637.3 | 0.0 | 0.0 | 0.0 |
| m2 | 3053556 | 34.69 | 86.16 | 104.53 | 120.24 | 156.11 | 5691.8 | 0.0 | 0.0 | 0.0 |
| m3 | 3053552 | 35.68 | 88.92 | 110.48 | 128.39 | 159.56 | 9788.3 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387015 | 7.52 | 28.90 | 46.72 | 60.94 | 93.08 | 0.0 | 0.0 | 0.0 | 0.0 | 2222.21 | 51261.0 | 2222.21 µs stall: loop work (5968 items) |
| m2 | 4387095 | 7.31 | 28.48 | 46.56 | 59.02 | 86.88 | 0.0 | 0.0 | 0.0 | 0.0 | 2331.90 | 50529.0 | 2331.90 µs stall: loop work (5950 items) |
| m3 | 4387098 | 7.24 | 26.66 | 44.39 | 56.05 | 189.24 | 5.2 | 0.0 | 0.0 | 0.0 | 2232.32 | 43520.0 | 2232.32 µs stall: loop work (5994 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387015 | 39.09 | 100.75 | 121.39 | 140.26 | 178.73 | 49201.2 | 0.0 | 0.0 | 0.0 |
| m2 | 4387095 | 36.77 | 99.92 | 121.40 | 141.03 | 196.06 | 45037.4 | 0.0 | 0.0 | 0.0 |
| m3 | 4387098 | 42.16 | 103.39 | 123.35 | 140.09 | 413.90 | 65323.6 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343095 | 3.59 | 8.42 | 16.66 | 22.35 | 118.34 | 10.3 | 0.0 | 0.0 | 0.0 | 391.43 | 4.0 | 391.43 µs stall: softirq TIMER |
| m2 | 343095 | 3.46 | 12.92 | 23.67 | 27.95 | 31.57 | 0.0 | 0.0 | 0.0 | 0.0 | 89.66 | 2.0 | 89.66 µs stall: loop work (589 items) |
| m3 | 343094 | 3.79 | 12.10 | 20.44 | 29.75 | 41.50 | 0.0 | 0.0 | 0.0 | 0.0 | 91.59 | 2.0 | 91.59 µs stall: loop work (589 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343095 | 21.55 | 43.93 | 51.44 | 65.10 | 177.81 | 11.4 | 0.0 | 0.0 | 0.0 |
| m2 | 343095 | 23.53 | 44.54 | 52.22 | 64.12 | 69.58 | 0.0 | 0.0 | 0.0 | 0.0 |
| m3 | 343094 | 17.56 | 60.29 | 63.94 | 81.97 | 88.62 | 0.0 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053556 | 5.77 | 22.05 | 40.66 | 50.88 | 109.42 | 1.0 | 0.0 | 0.0 | 0.0 | 743.32 | 3.0 | 743.32 µs stall: loop work (5271 items) |
| m2 | 3053552 | 5.18 | 20.41 | 38.62 | 53.49 | 80.71 | 0.0 | 0.0 | 0.0 | 0.0 | 743.26 | 2.0 | 743.26 µs stall: loop work (5271 items) |
| m3 | 3053552 | 6.04 | 23.81 | 42.04 | 54.52 | 77.03 | 0.0 | 0.0 | 0.0 | 0.0 | 741.37 | 2.0 | 741.37 µs stall: loop work (5272 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053556 | 33.64 | 84.05 | 102.96 | 119.10 | 223.25 | 4647.0 | 0.0 | 0.0 | 0.0 |
| m2 | 3053552 | 26.38 | 76.53 | 95.68 | 112.32 | 133.45 | 1757.6 | 0.0 | 0.0 | 0.0 |
| m3 | 3053552 | 31.46 | 84.40 | 104.22 | 121.83 | 144.62 | 5155.0 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386956 | 5.28 | 21.98 | 40.03 | 51.12 | 76.01 | 0.0 | 0.0 | 0.0 | 0.0 | 692.30 | 4.0 | 692.30 µs stall: loop work (4732 items) |
| m2 | 4386957 | 5.07 | 20.79 | 40.03 | 54.48 | 301.08 | 10.3 | 0.0 | 0.0 | 0.0 | 694.86 | 4.0 | 694.86 µs stall: loop work (4729 items) |
| m3 | 4386956 | 5.00 | 20.56 | 38.87 | 51.41 | 77.86 | 0.0 | 0.0 | 0.0 | 0.0 | 692.30 | 3.0 | 692.30 µs stall: loop work (4741 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386956 | 21.57 | 79.29 | 98.57 | 115.74 | 164.98 | 3799.8 | 0.0 | 0.0 | 0.0 |
| m2 | 4386957 | 29.13 | 88.16 | 107.32 | 124.85 | 320.30 | 11047.6 | 0.0 | 0.0 | 0.0 |
| m3 | 4386956 | 31.81 | 88.47 | 107.31 | 124.53 | 162.18 | 11142.8 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-sib-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 4.01 | 6.99 | 20.98 | 26.77 | 41.33 | 0.0 | 0.0 | 0.0 | 0.0 | 106.51 | 3.0 | 106.51 µs stall: loop work (589 items) |
| m2 | 343095 | 4.17 | 7.29 | 20.43 | 26.56 | 34.72 | 0.0 | 0.0 | 0.0 | 0.0 | 107.40 | 4.0 | 107.40 µs stall: loop work (589 items) |
| m3 | 343095 | 4.30 | 11.69 | 25.46 | 30.45 | 45.83 | 0.0 | 0.0 | 0.0 | 0.0 | 108.25 | 3.0 | 108.25 µs stall: loop work (590 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-sib-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 22.51 | 45.81 | 54.20 | 66.75 | 74.02 | 0.0 | 0.0 | 0.0 | 0.0 |
| m2 | 343095 | 25.44 | 48.90 | 57.13 | 70.52 | 77.28 | 0.0 | 0.0 | 0.0 | 0.0 |
| m3 | 343095 | 20.60 | 63.65 | 76.25 | 93.07 | 129.81 | 22.8 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-sib-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053552 | 8.29 | 31.60 | 51.31 | 67.65 | 100.87 | 1.0 | 0.0 | 0.0 | 0.0 | 914.41 | 3.0 | 914.41 µs stall: loop work (5272 items) |
| m2 | 3053552 | 8.37 | 32.77 | 53.85 | 69.20 | 95.85 | 0.0 | 0.0 | 0.0 | 0.0 | 907.83 | 4.0 | 907.83 µs stall: loop work (5271 items) |
| m3 | 3053551 | 7.53 | 29.46 | 52.22 | 68.79 | 349.01 | 9.3 | 0.0 | 0.0 | 0.0 | 920.17 | 4.0 | 920.17 µs stall: loop work (5272 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-sib-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053552 | 32.30 | 85.62 | 107.06 | 124.30 | 151.64 | 7263.3 | 0.0 | 0.0 | 0.0 |
| m2 | 3053552 | 41.76 | 95.65 | 116.42 | 134.32 | 263.00 | 20159.5 | 0.0 | 0.0 | 0.0 |
| m3 | 3053551 | 31.97 | 87.59 | 110.36 | 130.36 | 367.14 | 8842.9 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-sib-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386953 | 7.81 | 33.57 | 55.47 | 72.11 | 102.48 | 15.5 | 0.0 | 0.0 | 0.0 | 855.69 | 5.0 | 855.69 µs stall: loop work (4731 items) |
| m2 | 4386954 | 7.46 | 34.17 | 55.15 | 76.22 | 111.56 | 8.3 | 0.0 | 0.0 | 0.0 | 852.85 | 5.0 | 852.85 µs stall: loop work (4729 items) |
| m3 | 4386965 | 7.68 | 32.46 | 54.52 | 70.32 | 94.51 | 0.0 | 0.0 | 0.0 | 0.0 | 840.03 | 4.0 | 840.03 µs stall: loop work (4742 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-sib-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386953 | 27.37 | 90.61 | 113.24 | 133.84 | 168.18 | 17969.5 | 0.0 | 0.0 | 0.0 |
| m2 | 4386954 | 30.64 | 95.43 | 118.30 | 139.16 | 189.73 | 29075.0 | 0.0 | 0.0 | 0.0 |
| m3 | 4386965 | 36.95 | 99.28 | 121.81 | 139.74 | 178.64 | 42264.0 | 0.0 | 0.0 | 0.0 |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs, and system-management interrupts (SMI, bare metal only: firmware pauses). The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | SMI | receiving CPU | LOC | IWI | RES | CAL | SMI |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lab-epoll-spin-c1 | m1 | 3 | 0 | 0 | 0 | 16 | 0 | 4 | 75 | 0 | 0 | 19 | 0 |
| lab-epoll-spin-c1 | m2 | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 0 | 10 | 0 |
| lab-epoll-spin-c1 | m3 | 3 | 0 | 0 | 0 | 16 | 0 | 4 | 99 | 0 | 0 | 11 | 0 |
| lab-epoll-spin-c10 | m1 | 3 | 0 | 0 | 0 | 16 | 0 | 4 | 138 | 0 | 0 | 11 | 0 |
| lab-epoll-spin-c10 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 4 | 142 | 0 | 0 | 12 | 0 |
| lab-epoll-spin-c10 | m3 | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 0 | 18 | 0 |
| lab-epoll-spin-c100 | m1 | 3 | 0 | 0 | 0 | 16 | 0 | 4 | 359 | 0 | 0 | 12 | 0 |
| lab-epoll-spin-c100 | m2 | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 0 | 12 | 0 |
| lab-epoll-spin-c100 | m3 | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 1 | 12 | 0 |
| lab-recv-busy-c1 | m2 | 3 | 4 | 4 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c1 | m3 | 3 | 4 | 4 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c10 | m1 | 3 | 15 | 16 | 0 | 17 | 0 | 0 | 1141 | 0 | 0 | 10 | 0 |
| lab-recv-busy-c10 | m2 | 3 | 17 | 17 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c10 | m3 | 3 | 19 | 18 | 0 | 17 | 0 | 0 | 1142 | 0 | 0 | 11 | 0 |
| lab-recv-busy-c100 | m1 | 3 | 72 | 75 | 0 | 17 | 0 | 4 | 393 | 0 | 1 | 4 | 0 |
| lab-recv-busy-c100 | m2 | 3 | 70 | 72 | 0 | 17 | 0 | 0 | 1142 | 0 | 0 | 20 | 0 |
| lab-recv-busy-c100 | m3 | 3 | 67 | 70 | 0 | 16 | 0 | 0 | 1143 | 0 | 0 | 20 | 0 |
| lab-recv-busy-pq-c1 | m1 | 3 | 3 | 3 | 0 | 16 | 0 |  |  |  |  |  |  |
| lab-recv-busy-pq-c1 | m2 | 3 | 3 | 3 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-pq-c1 | m3 | 3 | 4 | 4 | 0 | 17 | 0 | 4 | 82 | 0 | 0 | 11 | 0 |
| lab-recv-busy-pq-c10 | m1 | 3 | 17 | 18 | 0 | 17 | 0 | 0 | 1141 | 0 | 0 | 10 | 0 |
| lab-recv-busy-pq-c10 | m2 | 3 | 19 | 19 | 0 | 17 | 0 | 4 | 153 | 0 | 0 | 20 | 0 |
| lab-recv-busy-pq-c10 | m3 | 3 | 23 | 24 | 0 | 16 | 0 | 4 | 158 | 0 | 0 | 11 | 0 |
| lab-recv-busy-pq-c100 | m1 | 3 | 75 | 78 | 0 | 16 | 0 | 0 | 1143 | 0 | 0 | 12 | 0 |
| lab-recv-busy-pq-c100 | m2 | 3 | 82 | 85 | 0 | 17 | 0 | 0 | 1143 | 0 | 1 | 19 | 0 |
| lab-recv-busy-pq-c100 | m3 | 3 | 80 | 83 | 0 | 16 | 0 | 4 | 348 | 0 | 0 | 12 | 0 |
| lab-uring-sqpoll-c1 | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 17 | 6 | 1 | 17 | 0 |
| lab-uring-sqpoll-c1 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 19 | 6 | 1 | 17 | 0 |
| lab-uring-sqpoll-c1 | m3 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 21 | 7 | 1 | 17 | 0 |
| lab-uring-sqpoll-c10 | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 40 | 16 | 1 | 17 | 0 |
| lab-uring-sqpoll-c10 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 43 | 17 | 1 | 17 | 0 |
| lab-uring-sqpoll-c10 | m3 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 50 | 23 | 1 | 17 | 0 |
| lab-uring-sqpoll-c100 | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 125 | 63 | 0 | 20 | 0 |
| lab-uring-sqpoll-c100 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 137 | 70 | 0 | 20 | 0 |
| lab-uring-sqpoll-c100 | m3 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 127 | 65 | 0 | 20 | 0 |
| lab-uring-sqpoll-sib-c1 | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 7 | 20 | 7 | 1 | 17 | 0 |
| lab-uring-sqpoll-sib-c1 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 7 | 18 | 7 | 1 | 17 | 0 |
| lab-uring-sqpoll-sib-c1 | m3 | 3 | 0 | 0 | 0 | 17 | 0 | 7 | 24 | 10 | 1 | 17 | 0 |
| lab-uring-sqpoll-sib-c10 | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 7 | 56 | 26 | 0 | 17 | 0 |
| lab-uring-sqpoll-sib-c10 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 7 | 55 | 25 | 1 | 17 | 0 |
| lab-uring-sqpoll-sib-c10 | m3 | 3 | 0 | 0 | 0 | 18 | 0 | 7 | 52 | 23 | 1 | 19 | 0 |
| lab-uring-sqpoll-sib-c100 | m1 | 3 | 0 | 0 | 0 | 16 | 0 | 7 | 217 | 114 | 0 | 20 | 0 |
| lab-uring-sqpoll-sib-c100 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 7 | 210 | 110 | 0 | 20 | 0 |
| lab-uring-sqpoll-sib-c100 | m3 | 3 | 0 | 0 | 0 | 17 | 0 | 7 | 202 | 106 | 0 | 20 | 0 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### lab-epoll-spin-c1

| condition | worst events |
|---|---|
| m1 | 31.03 at 21.6 s: neither loop stalled long enough: network, hypervisor or untraced; 30.89 at 21.6 s: neither loop stalled long enough: network, hypervisor or untraced; 30.76 at 21.6 s: neither loop stalled long enough: network, hypervisor or untraced; 30.75 at 23.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 30.62 at 21.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 31.76 at 6.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 31.62 at 6.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 31.49 at 6.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 31.36 at 6.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 31.22 at 6.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| m3 | 40.51 at 52.1 s: neither loop stalled long enough: network, hypervisor or untraced; 31.75 at 24.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 31.64 at 57.7 s: neither loop stalled long enough: network, hypervisor or untraced; 31.39 at 42.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 27.14 at 6.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |

### lab-epoll-spin-c10

| condition | worst events |
|---|---|
| m1 | 54.62 at 52.0 s: neither loop stalled long enough: network, hypervisor or untraced; 54.48 at 52.0 s: neither loop stalled long enough: network, hypervisor or untraced; 54.31 at 52.0 s: neither loop stalled long enough: network, hypervisor or untraced; 54.13 at 52.0 s: neither loop stalled long enough: network, hypervisor or untraced; 53.97 at 52.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 39.59 at 51.8 s: client: loop work (2 items); 39.44 at 51.8 s: client: loop work (2 items); 39.30 at 51.8 s: client: loop work (2 items); 39.16 at 51.8 s: client: loop work (2 items); 39.01 at 51.8 s: client: loop work (2 items) |
| m3 | 43.73 at 17.8 s: neither loop stalled long enough: network, hypervisor or untraced; 43.59 at 17.8 s: neither loop stalled long enough: network, hypervisor or untraced; 42.37 at 5.7 s: client: loop work (3 items); 42.21 at 5.7 s: client: loop work (3 items); 42.09 at 24.3 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-epoll-spin-c100

| condition | worst events |
|---|---|
| m1 | 174.85 at 18.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 174.71 at 18.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 174.57 at 18.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 174.44 at 18.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 174.31 at 18.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| m2 | 60.64 at 4.8 s: client: loop work (16 items); 59.16 at 2.2 s: neither loop stalled long enough: network, hypervisor or untraced; 59.02 at 2.2 s: neither loop stalled long enough: network, hypervisor or untraced; 56.07 at 2.2 s: neither loop stalled long enough: network, hypervisor or untraced; 55.90 at 2.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 318.94 at 3.8 s: client: softirq TIMER; 139.46 at 3.8 s: client: softirq TIMER; 138.47 at 3.8 s: client: softirq TIMER; 136.26 at 3.8 s: client: softirq TIMER; 121.35 at 3.8 s: client: softirq TIMER |

### lab-recv-busy-c1

| condition | worst events |
|---|---|
| m2 | 31.14 at 19.8 s: client: loop work (3 items); 30.99 at 19.8 s: client: loop work (3 items); 30.84 at 19.8 s: client: loop work (3 items); 29.94 at 26.7 s: client: loop work (2 items); 29.79 at 26.7 s: client: loop work (2 items) |
| m3 | 41.08 at 27.5 s: client: loop work (3 items); 40.93 at 53.2 s: client: mostly unexplained (+preempted by kworker/3:1); 40.93 at 27.5 s: client: loop work (3 items); 40.78 at 27.5 s: client: loop work (3 items); 38.98 at 45.3 s: client: loop work (3 items) |

### lab-recv-busy-c10

| condition | worst events |
|---|---|
| m1 | 99.20 at 18.7 s: client: loop work (29 items); 99.06 at 18.7 s: client: loop work (29 items); 98.92 at 18.7 s: client: loop work (29 items); 98.78 at 18.7 s: client: loop work (29 items); 98.65 at 18.7 s: client: loop work (29 items) |
| m2 | 84.86 at 23.5 s: client: loop work (47 items); 84.71 at 23.5 s: client: loop work (47 items); 84.56 at 23.5 s: client: loop work (47 items); 84.42 at 23.5 s: client: loop work (47 items); 84.28 at 23.5 s: client: loop work (47 items) |
| m3 | 145.56 at 59.2 s: client: softirq TIMER; 145.36 at 59.2 s: client: softirq TIMER; 145.18 at 59.2 s: client: softirq TIMER; 145.00 at 59.2 s: client: softirq TIMER; 144.84 at 59.2 s: client: softirq TIMER |

### lab-recv-busy-c100

| condition | worst events |
|---|---|
| m1 | 186.05 at 51.6 s: client: loop work (39 items); 185.91 at 51.6 s: client: loop work (39 items); 185.78 at 51.6 s: client: loop work (39 items); 185.64 at 51.6 s: client: loop work (39 items); 185.51 at 51.6 s: client: loop work (39 items) |
| m2 | 179.04 at 43.2 s: client: loop work (90 items); 178.90 at 43.2 s: client: loop work (90 items); 178.77 at 43.2 s: client: loop work (90 items); 178.63 at 43.2 s: client: loop work (90 items); 178.49 at 43.2 s: client: loop work (90 items) |
| m3 | 171.07 at 27.1 s: client: loop work (104 items); 169.54 at 14.1 s: client: loop work (40 items); 169.40 at 14.1 s: client: loop work (40 items); 169.26 at 14.1 s: client: loop work (40 items); 169.13 at 14.1 s: client: loop work (40 items) |

### lab-recv-busy-pq-c1

| condition | worst events |
|---|---|
| m1 | 33.41 at 56.2 s: client: loop work (3 items); 33.27 at 56.2 s: client: loop work (3 items); 33.11 at 56.2 s: client: loop work (3 items); 33.05 at 32.6 s: client: loop work (10 items); 33.04 at 7.7 s: client: loop work (3 items) |
| m2 | 41.48 at 49.5 s: client: mostly unexplained (+softirq NET_RX); 40.77 at 6.3 s: client: mostly unexplained (+preempted by kworker/3:1); 39.97 at 55.6 s: client: mostly unexplained (+softirq NET_RX); 39.50 at 45.9 s: client: loop work (10 items); 39.34 at 45.9 s: client: loop work (10 items) |
| m3 | 36.61 at 10.4 s: client: mostly unexplained (+preempted by kworker/3:1); 36.39 at 22.1 s: client: loop work (3 items); 36.25 at 22.1 s: client: loop work (3 items); 36.23 at 22.9 s: client: mostly unexplained (+preempted by kworker/3:1); 36.08 at 22.1 s: client: loop work (3 items) |

### lab-recv-busy-pq-c10

| condition | worst events |
|---|---|
| m1 | 77.67 at 25.8 s: client: loop work (40 items); 77.45 at 25.8 s: client: loop work (40 items); 77.27 at 25.8 s: client: loop work (40 items); 75.53 at 25.8 s: client: loop work (40 items); 75.40 at 25.8 s: client: loop work (40 items) |
| m2 | 77.75 at 47.9 s: client: loop work (36 items); 77.61 at 47.9 s: client: loop work (36 items); 77.47 at 47.9 s: client: loop work (36 items); 71.01 at 20.1 s: client: loop work (29 items); 70.88 at 20.1 s: client: loop work (29 items) |
| m3 | 95.15 at 28.7 s: client: loop work (64 items); 95.01 at 28.7 s: client: loop work (64 items); 94.85 at 28.7 s: client: loop work (64 items); 94.72 at 28.7 s: client: loop work (64 items); 94.58 at 28.7 s: client: loop work (64 items) |

### lab-recv-busy-pq-c100

| condition | worst events |
|---|---|
| m1 | 93.08 at 23.9 s: client: loop work (61 items); 92.94 at 23.9 s: client: loop work (61 items); 92.81 at 23.9 s: client: loop work (61 items); 92.67 at 23.9 s: client: loop work (61 items); 92.54 at 23.9 s: client: loop work (61 items) |
| m2 | 86.88 at 39.2 s: client: loop work (66 items); 86.75 at 39.2 s: client: loop work (66 items); 86.61 at 39.2 s: client: loop work (66 items); 86.44 at 39.2 s: client: loop work (66 items); 86.30 at 39.2 s: client: loop work (66 items) |
| m3 | 189.24 at 11.9 s: client: softirq TIMER; 187.71 at 11.9 s: client: softirq TIMER; 186.47 at 11.9 s: client: softirq TIMER; 172.46 at 11.9 s: client: softirq TIMER; 115.25 at 11.9 s: client: softirq TIMER |

### lab-uring-sqpoll-c1

| condition | worst events |
|---|---|
| m1 | 118.34 at 14.1 s: client: softirq TIMER; 118.18 at 14.1 s: client: softirq TIMER; 118.03 at 14.1 s: client: softirq TIMER; 117.87 at 14.1 s: client: softirq TIMER; 117.70 at 14.1 s: client: softirq TIMER |
| m2 | 31.57 at 59.6 s: neither loop stalled long enough: network, hypervisor or untraced; 31.57 at 32.6 s: neither loop stalled long enough: network, hypervisor or untraced; 31.54 at 45.7 s: neither loop stalled long enough: network, hypervisor or untraced; 31.43 at 32.6 s: neither loop stalled long enough: network, hypervisor or untraced; 31.41 at 45.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 41.50 at 57.7 s: neither loop stalled long enough: network, hypervisor or untraced; 34.55 at 38.8 s: neither loop stalled long enough: network, hypervisor or untraced; 34.41 at 38.8 s: neither loop stalled long enough: network, hypervisor or untraced; 34.40 at 51.0 s: neither loop stalled long enough: network, hypervisor or untraced; 34.27 at 38.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-c10

| condition | worst events |
|---|---|
| m1 | 109.42 at 56.0 s: client: softirq TIMER; 81.10 at 59.6 s: neither loop stalled long enough: network, hypervisor or untraced; 80.96 at 59.6 s: neither loop stalled long enough: network, hypervisor or untraced; 80.83 at 59.6 s: neither loop stalled long enough: network, hypervisor or untraced; 80.69 at 59.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 80.71 at 54.2 s: neither loop stalled long enough: network, hypervisor or untraced; 80.57 at 54.2 s: neither loop stalled long enough: network, hypervisor or untraced; 80.44 at 54.2 s: neither loop stalled long enough: network, hypervisor or untraced; 80.30 at 54.2 s: neither loop stalled long enough: network, hypervisor or untraced; 80.16 at 54.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 77.03 at 40.1 s: neither loop stalled long enough: network, hypervisor or untraced; 76.90 at 40.1 s: neither loop stalled long enough: network, hypervisor or untraced; 76.67 at 40.1 s: neither loop stalled long enough: network, hypervisor or untraced; 76.53 at 40.1 s: neither loop stalled long enough: network, hypervisor or untraced; 76.40 at 40.1 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-c100

| condition | worst events |
|---|---|
| m1 | 76.01 at 54.6 s: neither loop stalled long enough: network, hypervisor or untraced; 75.87 at 54.6 s: neither loop stalled long enough: network, hypervisor or untraced; 75.73 at 54.6 s: neither loop stalled long enough: network, hypervisor or untraced; 75.59 at 54.6 s: neither loop stalled long enough: network, hypervisor or untraced; 75.46 at 54.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 301.08 at 4.3 s: client: softirq TIMER; 175.27 at 4.3 s: client: softirq TIMER; 131.28 at 4.3 s: client: softirq TIMER; 111.68 at 25.5 s: neither loop stalled long enough: network, hypervisor or untraced; 111.55 at 25.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 77.86 at 46.1 s: neither loop stalled long enough: network, hypervisor or untraced; 77.72 at 46.1 s: neither loop stalled long enough: network, hypervisor or untraced; 77.59 at 46.1 s: neither loop stalled long enough: network, hypervisor or untraced; 77.45 at 46.1 s: neither loop stalled long enough: network, hypervisor or untraced; 77.31 at 46.1 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-sib-c1

| condition | worst events |
|---|---|
| m1 | 41.33 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced; 41.16 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced; 40.97 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced; 40.80 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced; 40.65 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 34.72 at 22.4 s: client: VM paused: the idle spare core stalled too (hypervisor); 32.05 at 49.5 s: neither loop stalled long enough: network, hypervisor or untraced; 31.85 at 49.5 s: neither loop stalled long enough: network, hypervisor or untraced; 31.66 at 49.5 s: neither loop stalled long enough: network, hypervisor or untraced; 31.48 at 49.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 45.83 at 32.1 s: neither loop stalled long enough: network, hypervisor or untraced; 34.31 at 52.5 s: neither loop stalled long enough: network, hypervisor or untraced; 34.12 at 52.5 s: neither loop stalled long enough: network, hypervisor or untraced; 33.92 at 52.5 s: neither loop stalled long enough: network, hypervisor or untraced; 33.84 at 13.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-sib-c10

| condition | worst events |
|---|---|
| m1 | 100.87 at 20.1 s: neither loop stalled long enough: network, hypervisor or untraced; 86.87 at 20.9 s: neither loop stalled long enough: network, hypervisor or untraced; 86.64 at 20.9 s: neither loop stalled long enough: network, hypervisor or untraced; 86.47 at 20.9 s: neither loop stalled long enough: network, hypervisor or untraced; 86.32 at 20.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 95.85 at 59.0 s: neither loop stalled long enough: network, hypervisor or untraced; 95.67 at 59.0 s: neither loop stalled long enough: network, hypervisor or untraced; 95.45 at 59.0 s: neither loop stalled long enough: network, hypervisor or untraced; 91.94 at 13.5 s: neither loop stalled long enough: network, hypervisor or untraced; 91.78 at 13.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 349.01 at 33.5 s: neither loop stalled long enough: network, hypervisor or untraced; 103.06 at 11.3 s: neither loop stalled long enough: network, hypervisor or untraced; 102.86 at 11.3 s: neither loop stalled long enough: network, hypervisor or untraced; 102.69 at 11.3 s: neither loop stalled long enough: network, hypervisor or untraced; 102.51 at 11.3 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-sib-c100

| condition | worst events |
|---|---|
| m1 | 102.48 at 44.1 s: neither loop stalled long enough: network, hypervisor or untraced; 102.32 at 44.1 s: neither loop stalled long enough: network, hypervisor or untraced; 102.16 at 44.1 s: neither loop stalled long enough: network, hypervisor or untraced; 102.01 at 44.1 s: neither loop stalled long enough: network, hypervisor or untraced; 102.00 at 48.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 111.56 at 29.1 s: neither loop stalled long enough: network, hypervisor or untraced; 107.10 at 13.8 s: neither loop stalled long enough: network, hypervisor or untraced; 106.92 at 13.8 s: neither loop stalled long enough: network, hypervisor or untraced; 106.75 at 13.8 s: neither loop stalled long enough: network, hypervisor or untraced; 106.59 at 13.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 94.51 at 48.4 s: neither loop stalled long enough: network, hypervisor or untraced; 94.34 at 48.4 s: neither loop stalled long enough: network, hypervisor or untraced; 94.19 at 48.4 s: neither loop stalled long enough: network, hypervisor or untraced; 93.99 at 48.4 s: neither loop stalled long enough: network, hypervisor or untraced; 93.80 at 48.4 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| m1 | unexplained: no kernel event (hypervisor or in-kernel time) 42005480.42; loop work 17130502.35; mostly unexplained (+softirq NET_RX) 1174454.90; mostly unexplained (+preempted by kworker/3:1) 181583.77; mostly unexplained (+local timer tick) 156146.40 |
| m2 | unexplained: no kernel event (hypervisor or in-kernel time) 41828553.30; loop work 17309774.97; mostly unexplained (+softirq NET_RX) 1204147.98; mostly unexplained (+local timer tick) 230681.92; mostly unexplained (+preempted by kworker/3:1) 184428.75 |
| m3 | unexplained: no kernel event (hypervisor or in-kernel time) 41811956.45; loop work 17597162.93; mostly unexplained (+softirq NET_RX) 1130666.99; mostly unexplained (+local timer tick) 199331.43; mostly unexplained (+preempted by kworker/3:1) 177273.53 |

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
| cpumap-sender/cpu1 | counter-only spinner | 1 | :30 | 1 (100%) | 1 of 2 | 752.13 | no |
| cpumap-sender/cpu3 | counter-only spinner | 1 | :30 | 1 (100%) | 1 of 2 | 741.53 | no |
| lab-epoll-spin-c1 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c1 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c1 | slow samples | 0 | | | | | |
| lab-epoll-spin-c10 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c10 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c10 | slow samples | 0 | | | | | |
| lab-epoll-spin-c100 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c100 | receiver loop | 1 | :10 | 1 (100%) | 1 of 3 | 386.11 | no |
| lab-epoll-spin-c100 | slow samples | 1 | :10 | 1 (100%) | 1 of 3 | 318.94 | no |
| lab-recv-busy-c1 | idle spare core | 0 | | | | | |
| lab-recv-busy-c1 | receiver loop | 0 | | | | | |
| lab-recv-busy-c1 | slow samples | 0 | | | | | |
| lab-recv-busy-c10 | idle spare core | 0 | | | | | |
| lab-recv-busy-c10 | receiver loop | 0 | | | | | |
| lab-recv-busy-c10 | slow samples | 0 | | | | | |
| lab-recv-busy-c100 | idle spare core | 1 | :38 | 1 (100%) | 1 of 3 | 319.84 | no |
| lab-recv-busy-c100 | receiver loop | 0 | | | | | |
| lab-recv-busy-c100 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c1 | idle spare core | 0 | | | | | |
| lab-recv-busy-pq-c1 | receiver loop | 1 | :01 | 1 (100%) | 1 of 3 | 302.43 | no |
| lab-recv-busy-pq-c1 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c10 | idle spare core | 0 | | | | | |
| lab-recv-busy-pq-c10 | receiver loop | 0 | | | | | |
| lab-recv-busy-pq-c10 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c100 | idle spare core | 0 | | | | | |
| lab-recv-busy-pq-c100 | receiver loop | 1 | :43 | 1 (100%) | 1 of 3 | 310.39 | no |
| lab-recv-busy-pq-c100 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c1 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-c1 | receiver loop | 1 | :15 | 1 (100%) | 1 of 3 | 391.43 | no |
| lab-uring-sqpoll-c1 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c10 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-c10 | receiver loop | 1 | :11 | 1 (100%) | 1 of 3 | 379.54 | no |
| lab-uring-sqpoll-c10 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c100 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-c100 | receiver loop | 1 | :24 | 1 (100%) | 1 of 3 | 303.15 | no |
| lab-uring-sqpoll-c100 | slow samples | 1 | :24 | 1 (100%) | 1 of 3 | 301.08 | no |
| lab-uring-sqpoll-sib-c1 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-sib-c1 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-sib-c1 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-sib-c10 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-sib-c10 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-sib-c10 | slow samples | 1 | :15 | 1 (100%) | 1 of 3 | 349.01 | no |
| lab-uring-sqpoll-sib-c100 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-sib-c100 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-sib-c100 | slow samples | 0 | | | | | |

# Every run in detail

## m1/lab-epoll-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 355509 | 146682 | 2.42 | 267202298 | 751.60 | 4453372 | 4450927 | 0.0 | 35.0 | false | 0 | 0 | 189.9 | 6649 / 5 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (84529 of 343095 in 1-msg reads) | 84529 | 4.18 | 3.29 | 5.74 | 12.60 | 18.88 | 25.47 | 30.75 |

#### Stalls: m1/lab-epoll-spin-c1, loop 0 on CPU 3

267055616 iterations, longest 189.9 µs; over 5 / 20 / 100 µs: 6649 / 5 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 218.2 | 189.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 45.6 | 24.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 29.5 | 29.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 189.9 | 589 | loop work (589 items) |  |
| 2 | 29.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 28.3 | 3 | loop work (3 items) |  |
| 4 | 24.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 21.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 988 |
| LOC Local timer interrupts | 15 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-epoll-spin-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3164102 | 908663 | 3.48 | 257270455 | 81.31 | 4287841 | 4272697 | 0.0 | 36.6 | false | 0 | 0 | 1388.7 | 121664 / 125 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8196 | 4 |
| epoll-spin | 1 | 8198 | 4 |
| epoll-spin | 2 | 8195 | 0 |
| epoll-spin | 3 | 8199 | 0 |
| epoll-spin | 4 | 8196 | 4 |
| epoll-spin | 5 | 8196 | 4 |
| epoll-spin | 6 | 8197 | 0 |
| epoll-spin | 7 | 8198 | 4 |
| epoll-spin | 8 | 8193 | 0 |
| epoll-spin | 9 | 8196 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 708758848 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (337598 of 3053557 in 1-msg reads) | 337598 | 5.30 | 4.96 | 8.72 | 16.38 | 24.11 | 32.94 | 51.20 |

#### Stalls: m1/lab-epoll-spin-c10, loop 0 on CPU 3

256361792 iterations, longest 1388.7 µs; over 5 / 20 / 100 µs: 121664 / 125 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 79 | 3324.9 | 1388.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 45 | 1142.8 | 35.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 31.1 | 31.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1388.7 | 5272 | loop work (5272 items) |  |
| 2 | 35.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 34.2 | 7 | loop work (7 items) |  |
| 4 | 33.5 | 3 | loop work (3 items) |  |
| 5 | 32.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 32.3 | 3 | loop work (3 items) |  |
| 7 | 32.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 31.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 31.7 | 7 | loop work (7 items) |  |
| 10 | 31.1 | 10 | loop work (10 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-epoll-spin-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 4545784 | 2288911 | 1.99 | 205938127 | 45.30 | 3432302 | 3394154 | 0.0 | 34.5 | false | 0 | 0 | 1980.9 | 202561 / 261 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8198 | 4 |
| epoll-spin | 1 | 8198 | 4 |
| epoll-spin | 2 | 8200 | 4 |
| epoll-spin | 3 | 8193 | 0 |
| epoll-spin | 4 | 8196 | 4 |
| epoll-spin | 5 | 8193 | 0 |
| epoll-spin | 6 | 8199 | 0 |
| epoll-spin | 7 | 8197 | 0 |
| epoll-spin | 8 | 8196 | 4 |
| epoll-spin | 9 | 8198 | 4 |
| epoll-spin | 10 | 8197 | 0 |
| epoll-spin | 11 | 8198 | 4 |
| epoll-spin | 12 | 8194 | 4 |
| epoll-spin | 13 | 8193 | 0 |
| epoll-spin | 14 | 8194 | 4 |
| epoll-spin | 15 | 8196 | 4 |
| epoll-spin | 16 | 8200 | 4 |
| epoll-spin | 17 | 8195 | 0 |
| epoll-spin | 18 | 8194 | 4 |
| epoll-spin | 19 | 8197 | 0 |
| epoll-spin | 20 | 8199 | 0 |
| epoll-spin | 21 | 8198 | 4 |
| epoll-spin | 22 | 8196 | 4 |
| epoll-spin | 23 | 8198 | 4 |
| epoll-spin | 24 | 8193 | 0 |
| epoll-spin | 25 | 8197 | 0 |
| epoll-spin | 26 | 8198 | 4 |
| epoll-spin | 27 | 8194 | 4 |
| epoll-spin | 28 | 8193 | 0 |
| epoll-spin | 29 | 8197 | 0 |
| epoll-spin | 30 | 8199 | 0 |
| epoll-spin | 31 | 8193 | 0 |
| epoll-spin | 32 | 8193 | 0 |
| epoll-spin | 33 | 8197 | 0 |
| epoll-spin | 34 | 8194 | 4 |
| epoll-spin | 35 | 8197 | 0 |
| epoll-spin | 36 | 8200 | 4 |
| epoll-spin | 37 | 8194 | 4 |
| epoll-spin | 38 | 8193 | 0 |
| epoll-spin | 39 | 8199 | 0 |
| epoll-spin | 40 | 8195 | 0 |
| epoll-spin | 41 | 8200 | 4 |
| epoll-spin | 42 | 8196 | 4 |
| epoll-spin | 43 | 8199 | 0 |
| epoll-spin | 44 | 8196 | 4 |
| epoll-spin | 45 | 8195 | 0 |
| epoll-spin | 46 | 8200 | 4 |
| epoll-spin | 47 | 8196 | 4 |
| epoll-spin | 48 | 8198 | 4 |
| epoll-spin | 49 | 8199 | 0 |
| epoll-spin | 50 | 8193 | 0 |
| epoll-spin | 51 | 8199 | 0 |
| epoll-spin | 52 | 8200 | 4 |
| epoll-spin | 53 | 8200 | 4 |
| epoll-spin | 54 | 8198 | 4 |
| epoll-spin | 55 | 8195 | 0 |
| epoll-spin | 56 | 8193 | 0 |
| epoll-spin | 57 | 8197 | 0 |
| epoll-spin | 58 | 8196 | 4 |
| epoll-spin | 59 | 8199 | 0 |
| epoll-spin | 60 | 8193 | 0 |
| epoll-spin | 61 | 8198 | 4 |
| epoll-spin | 62 | 8199 | 0 |
| epoll-spin | 63 | 8197 | 0 |
| epoll-spin | 64 | 8196 | 4 |
| epoll-spin | 65 | 8199 | 0 |
| epoll-spin | 66 | 8195 | 0 |
| epoll-spin | 67 | 8200 | 4 |
| epoll-spin | 68 | 8195 | 0 |
| epoll-spin | 69 | 8199 | 0 |
| epoll-spin | 70 | 8199 | 0 |
| epoll-spin | 71 | 8195 | 0 |
| epoll-spin | 72 | 8195 | 0 |
| epoll-spin | 73 | 8198 | 4 |
| epoll-spin | 74 | 8200 | 4 |
| epoll-spin | 75 | 8199 | 0 |
| epoll-spin | 76 | 8199 | 0 |
| epoll-spin | 77 | 8200 | 4 |
| epoll-spin | 78 | 8195 | 0 |
| epoll-spin | 79 | 8193 | 0 |
| epoll-spin | 80 | 8195 | 0 |
| epoll-spin | 81 | 8194 | 4 |
| epoll-spin | 82 | 8200 | 4 |
| epoll-spin | 83 | 8199 | 0 |
| epoll-spin | 84 | 8198 | 4 |
| epoll-spin | 85 | 8195 | 0 |
| epoll-spin | 86 | 8193 | 0 |
| epoll-spin | 87 | 8195 | 0 |
| epoll-spin | 88 | 8196 | 4 |
| epoll-spin | 89 | 8193 | 0 |
| epoll-spin | 90 | 8194 | 4 |
| epoll-spin | 91 | 8200 | 4 |
| epoll-spin | 92 | 8193 | 0 |
| epoll-spin | 93 | 8197 | 0 |
| epoll-spin | 94 | 8198 | 4 |
| epoll-spin | 95 | 8198 | 4 |
| epoll-spin | 96 | 8193 | 0 |
| epoll-spin | 97 | 8197 | 0 |
| epoll-spin | 98 | 8195 | 0 |
| epoll-spin | 99 | 8197 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1018255616 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1677920 of 4387088 in 1-msg reads) | 1677920 | 4.20 | 3.22 | 7.00 | 13.18 | 20.31 | 28.71 | 167.82 |

#### Stalls: m1/lab-epoll-spin-c100, loop 0 on CPU 3

203649216 iterations, longest 1980.9 µs; over 5 / 20 / 100 µs: 202561 / 261 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 190 | 4885.5 | 170.9 |
| loop work | 70 | 4448.6 | 1980.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.3 | 33.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1980.9 | 5435 | loop work (5435 items) |  |
| 2 | 763.0 | 2234 | loop work (2234 items) |  |
| 3 | 170.9 | 7 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 38.3 | 67 | loop work (67 items) |  |
| 5 | 35.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 35.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 34.3 | 6 | loop work (6 items) |  |
| 8 | 34.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 33.5 | 15 | loop work (15 items) |  |
| 10 | 33.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 988 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=750644 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3164102 | 797642 | 3.97 | 118279680 | 37.38 | 1971328 | 197133 | 99.3 | 69.1 | true | 0 | 929 | 1204.7 | 800064 / 26657 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 0 |
| recv-busy | 1 | 8196 | 3 |
| recv-busy | 2 | 8194 | 3 |
| recv-busy | 3 | 8198 | 4 |
| recv-busy | 4 | 8196 | 4 |
| recv-busy | 5 | 8194 | 3 |
| recv-busy | 6 | 8194 | 3 |
| recv-busy | 7 | 8199 | 3 |
| recv-busy | 8 | 8196 | 3 |
| recv-busy | 9 | 8199 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708758848 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (223784 of 3053557 in 1-msg reads) | 223784 | 4.09 | 3.22 | 6.70 | 13.55 | 27.36 | 43.96 | 62.86 |

#### Stalls: m1/lab-recv-busy-c10, loop 0 on CPU 3

11827968 iterations, longest 1204.7 µs; over 5 / 20 / 100 µs: 800064 / 26657 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 25310 | 707569.3 | 1204.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1078 | 31527.3 | 45.8 |
| mostly unexplained (+preempted by kworker/3:1) | 185 | 5268.4 | 54.0 |
| mostly unexplained (+softirq NET_RX) | 61 | 1953.9 | 61.7 |
| softirq NET_RX | 8 | 307.6 | 54.4 |
| mostly unexplained (+irq work (IPI/vector)) | 8 | 212.7 | 36.1 |
| mostly unexplained (+local timer tick) | 5 | 136.8 | 38.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 38.8 | 38.8 |
| mostly unexplained (+softirq NET_TX) | 1 | 38.5 | 38.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1204.7 | 5271 | loop work (5271 items) |  |
| 2 | 111.2 | 60 | loop work (60 items) | softirq NET_RX 1 |
| 3 | 91.9 | 42 | loop work (42 items) | softirq NET_RX 6 |
| 4 | 91.3 | 40 | loop work (40 items) |  |
| 5 | 91.0 | 46 | loop work (46 items) | softirq NET_RX 2 |
| 6 | 89.3 | 42 | loop work (42 items) |  |
| 7 | 88.6 | 41 | loop work (41 items) |  |
| 8 | 88.4 | 46 | loop work (46 items) |  |
| 9 | 88.0 | 40 | loop work (40 items) |  |
| 10 | 86.4 | 36 | loop work (36 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 950 |
| LOC Local timer interrupts | 929 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1632242 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4545926 | 1930036 | 2.36 | 108313600 | 23.83 | 1805227 | 18052 | 98.2 | 70.9 | true | 0 | 4445 | 2538.4 | 1083135 / 1083135 / 17470 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 3 |
| recv-busy | 1 | 8197 | 3 |
| recv-busy | 2 | 8195 | 0 |
| recv-busy | 3 | 8200 | 3 |
| recv-busy | 4 | 8199 | 0 |
| recv-busy | 5 | 8194 | 4 |
| recv-busy | 6 | 8197 | 0 |
| recv-busy | 7 | 8200 | 3 |
| recv-busy | 8 | 8197 | 3 |
| recv-busy | 9 | 8199 | 3 |
| recv-busy | 10 | 8194 | 4 |
| recv-busy | 11 | 8193 | 3 |
| recv-busy | 12 | 8198 | 3 |
| recv-busy | 13 | 8195 | 3 |
| recv-busy | 14 | 8198 | 3 |
| recv-busy | 15 | 8197 | 3 |
| recv-busy | 16 | 8199 | 0 |
| recv-busy | 17 | 8197 | 3 |
| recv-busy | 18 | 8200 | 3 |
| recv-busy | 19 | 8196 | 3 |
| recv-busy | 20 | 8200 | 3 |
| recv-busy | 21 | 8196 | 4 |
| recv-busy | 22 | 8198 | 4 |
| recv-busy | 23 | 8193 | 3 |
| recv-busy | 24 | 8200 | 3 |
| recv-busy | 25 | 8193 | 0 |
| recv-busy | 26 | 8199 | 3 |
| recv-busy | 27 | 8198 | 4 |
| recv-busy | 28 | 8200 | 3 |
| recv-busy | 29 | 8200 | 3 |
| recv-busy | 30 | 8198 | 3 |
| recv-busy | 31 | 8199 | 3 |
| recv-busy | 32 | 8193 | 0 |
| recv-busy | 33 | 8197 | 3 |
| recv-busy | 34 | 8199 | 3 |
| recv-busy | 35 | 8194 | 3 |
| recv-busy | 36 | 8196 | 3 |
| recv-busy | 37 | 8195 | 0 |
| recv-busy | 38 | 8200 | 3 |
| recv-busy | 39 | 8199 | 0 |
| recv-busy | 40 | 8194 | 3 |
| recv-busy | 41 | 8197 | 0 |
| recv-busy | 42 | 8194 | 3 |
| recv-busy | 43 | 8198 | 3 |
| recv-busy | 44 | 8196 | 4 |
| recv-busy | 45 | 8199 | 3 |
| recv-busy | 46 | 8197 | 0 |
| recv-busy | 47 | 8200 | 3 |
| recv-busy | 48 | 8200 | 3 |
| recv-busy | 49 | 8195 | 3 |
| recv-busy | 50 | 8193 | 3 |
| recv-busy | 51 | 8196 | 3 |
| recv-busy | 52 | 8193 | 0 |
| recv-busy | 53 | 8198 | 3 |
| recv-busy | 54 | 8197 | 3 |
| recv-busy | 55 | 8200 | 4 |
| recv-busy | 56 | 8197 | 3 |
| recv-busy | 57 | 8200 | 3 |
| recv-busy | 58 | 8194 | 3 |
| recv-busy | 59 | 8199 | 3 |
| recv-busy | 60 | 8200 | 3 |
| recv-busy | 61 | 8197 | 0 |
| recv-busy | 62 | 8197 | 3 |
| recv-busy | 63 | 8195 | 0 |
| recv-busy | 64 | 8198 | 3 |
| recv-busy | 65 | 8199 | 3 |
| recv-busy | 66 | 8196 | 3 |
| recv-busy | 67 | 8193 | 0 |
| recv-busy | 68 | 8197 | 3 |
| recv-busy | 69 | 8195 | 3 |
| recv-busy | 70 | 8196 | 4 |
| recv-busy | 71 | 8199 | 3 |
| recv-busy | 72 | 8198 | 4 |
| recv-busy | 73 | 8197 | 0 |
| recv-busy | 74 | 8200 | 3 |
| recv-busy | 75 | 8195 | 3 |
| recv-busy | 76 | 8200 | 3 |
| recv-busy | 77 | 8197 | 0 |
| recv-busy | 78 | 8194 | 3 |
| recv-busy | 79 | 8200 | 3 |
| recv-busy | 80 | 8196 | 3 |
| recv-busy | 81 | 8200 | 3 |
| recv-busy | 82 | 8199 | 0 |
| recv-busy | 83 | 8198 | 3 |
| recv-busy | 84 | 8200 | 4 |
| recv-busy | 85 | 8195 | 0 |
| recv-busy | 86 | 8198 | 3 |
| recv-busy | 87 | 8194 | 3 |
| recv-busy | 88 | 8196 | 3 |
| recv-busy | 89 | 8196 | 3 |
| recv-busy | 90 | 8198 | 3 |
| recv-busy | 91 | 8200 | 3 |
| recv-busy | 92 | 8199 | 3 |
| recv-busy | 93 | 8193 | 0 |
| recv-busy | 94 | 8194 | 3 |
| recv-busy | 95 | 8194 | 4 |
| recv-busy | 96 | 8198 | 3 |
| recv-busy | 97 | 8197 | 3 |
| recv-busy | 98 | 8196 | 3 |
| recv-busy | 99 | 8195 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1018287424 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1491448 of 4387235 in 1-msg reads) | 1491448 | 28.38 | 27.96 | 50.40 | 71.92 | 92.33 | 118.21 | 176.35 |

#### Stalls: m1/lab-recv-busy-c100, loop 0 on CPU 3

1083136 iterations, longest 2538.4 µs; over 5 / 20 / 100 µs: 1083135 / 1083135 / 17470 (34559 stalls not kept)

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 800006 | 41788579.3 | 129.6 |
| loop work | 222856 | 14702201.5 | 2538.4 |
| mostly unexplained (+softirq NET_RX) | 20615 | 1167406.7 | 122.6 |
| mostly unexplained (+local timer tick) | 2500 | 155151.6 | 109.2 |
| mostly unexplained (+preempted by kworker/3:1) | 2021 | 142019.5 | 150.9 |
| mostly unexplained (+irq work (IPI/vector)) | 539 | 35945.4 | 101.3 |
| mostly unexplained (+softirq NET_TX) | 17 | 1214.9 | 106.2 |
| mostly unexplained (+ipi_send_cpu) | 7 | 430.6 | 67.5 |
| mostly unexplained (+call function (IPI/vector)) | 6 | 337.5 | 58.8 |
| mostly unexplained (+call function single (IPI/vector)) | 4 | 220.2 | 60.1 |
| mostly unexplained (+softirq TIMER) | 2 | 199.3 | 131.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 108.7 | 54.8 |
| mostly unexplained (+softirq TASKLET) | 1 | 64.2 | 64.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2538.4 | 7601 | loop work (7601 items) |  |
| 2 | 214.1 | 39 | loop work (39 items) |  |
| 3 | 185.6 | 62 | loop work (62 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, softirq NET_RX 1, workqueue [ena] 1 |
| 4 | 183.4 | 96 | loop work (96 items) | softirq NET_RX 26 |
| 5 | 179.0 | 70 | loop work (70 items) |  |
| 6 | 178.3 | 79 | loop work (79 items) | local timer tick 3, softirq NET_RX 1, softirq NET_TX 1, timer tick_nohz_handler 1 |
| 7 | 175.0 | 76 | loop work (76 items) |  |
| 8 | 173.0 | 64 | loop work (64 items) |  |
| 9 | 172.7 | 35 | loop work (35 items) |  |
| 10 | 171.5 | 71 | loop work (71 items) | softirq NET_RX 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4486 |
| LOC Local timer interrupts | 4326 |
| CAL Function call interrupts | 992 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=117551 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355509 | 130415 | 2.73 | 168132480 | 472.93 | 2802208 | 1401104 | 99.8 | 58.7 | true | 0 | 189 | 155.0 | 25646 / 1231 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (66448 of 343095 in 1-msg reads) | 66448 | 2.76 | 2.72 | 3.18 | 4.06 | 6.63 | 18.40 | 28.25 |

#### Stalls: m1/lab-recv-busy-pq-c1, loop 0 on CPU 3

84066240 iterations, longest 155.0 µs; over 5 / 20 / 100 µs: 25646 / 1231 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 761 | 22093.9 | 155.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 377 | 9167.2 | 37.3 |
| mostly unexplained (+preempted by kworker/3:1) | 35 | 919.1 | 45.2 |
| mostly unexplained (+softirq NET_RX) | 32 | 825.5 | 48.7 |
| softirq NET_RX | 22 | 545.8 | 32.5 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 95.7 | 35.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 32.6 | 32.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 155.0 | 589 | loop work (589 items) |  |
| 2 | 56.8 | 10 | loop work (10 items) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 55.0 | 10 | loop work (10 items) |  |
| 4 | 52.9 | 10 | loop work (10 items) | softirq NET_RX 15, preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 50.3 | 10 | loop work (10 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 48.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 46.8 | 10 | loop work (10 items) | softirq NET_RX 13, preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 46.1 | 10 | loop work (10 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 45.2 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 43.7 | 10 | loop work (10 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| IWI IRQ work interrupts | 203 |
| LOC Local timer interrupts | 197 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=799606 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3164102 | 805749 | 3.93 | 130954168 | 41.39 | 2182569 | 435014 | 99.2 | 66.1 | true | 0 | 1035 | 1349.6 | 512533 / 15713 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 3 |
| recv-busy | 1 | 8200 | 3 |
| recv-busy | 2 | 8193 | 0 |
| recv-busy | 3 | 8200 | 3 |
| recv-busy | 4 | 8199 | 3 |
| recv-busy | 5 | 8197 | 3 |
| recv-busy | 6 | 8193 | 3 |
| recv-busy | 7 | 8193 | 3 |
| recv-busy | 8 | 8199 | 0 |
| recv-busy | 9 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708758848 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (219088 of 3053557 in 1-msg reads) | 219088 | 4.36 | 3.83 | 6.70 | 13.93 | 27.60 | 41.26 | 57.35 |

#### Stalls: m1/lab-recv-busy-pq-c10, loop 0 on CPU 3

26100864 iterations, longest 1349.6 µs; over 5 / 20 / 100 µs: 512533 / 15713 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 14408 | 448552.5 | 1349.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1068 | 28735.0 | 43.1 |
| mostly unexplained (+preempted by kworker/3:1) | 184 | 4779.9 | 42.9 |
| mostly unexplained (+softirq NET_RX) | 34 | 895.8 | 32.4 |
| softirq NET_RX | 8 | 262.8 | 41.8 |
| mostly unexplained (+irq work (IPI/vector)) | 6 | 175.7 | 49.9 |
| mostly unexplained (+local timer tick) | 4 | 116.0 | 35.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 31.4 | 31.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1349.6 | 5271 | loop work (5271 items) |  |
| 2 | 106.8 | 60 | loop work (60 items) | preempted by kworker/3:1 3, softirq NET_RX 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 87.8 | 50 | loop work (50 items) |  |
| 4 | 87.7 | 45 | loop work (45 items) | softirq NET_RX 3 |
| 5 | 85.2 | 41 | loop work (41 items) |  |
| 6 | 84.9 | 40 | loop work (40 items) |  |
| 7 | 83.7 | 50 | loop work (50 items) |  |
| 8 | 83.1 | 28 | loop work (28 items) | softirq NET_RX 6 |
| 9 | 82.7 | 33 | loop work (33 items) | softirq NET_RX 4 |
| 10 | 81.9 | 40 | loop work (40 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1066 |
| LOC Local timer interrupts | 1031 |
| CAL Function call interrupts | 991 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1736270 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4545813 | 2144642 | 2.12 | 112272918 | 24.70 | 1871215 | 204342 | 97.9 | 69.1 | true | 0 | 4625 | 2222.2 | 1884097 / 51261 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 3 |
| recv-busy | 1 | 8194 | 3 |
| recv-busy | 2 | 8195 | 3 |
| recv-busy | 3 | 8199 | 3 |
| recv-busy | 4 | 8194 | 3 |
| recv-busy | 5 | 8193 | 3 |
| recv-busy | 6 | 8195 | 3 |
| recv-busy | 7 | 8194 | 3 |
| recv-busy | 8 | 8198 | 3 |
| recv-busy | 9 | 8199 | 0 |
| recv-busy | 10 | 8196 | 3 |
| recv-busy | 11 | 8193 | 3 |
| recv-busy | 12 | 8195 | 0 |
| recv-busy | 13 | 8198 | 4 |
| recv-busy | 14 | 8200 | 3 |
| recv-busy | 15 | 8198 | 3 |
| recv-busy | 16 | 8199 | 3 |
| recv-busy | 17 | 8196 | 3 |
| recv-busy | 18 | 8195 | 3 |
| recv-busy | 19 | 8194 | 3 |
| recv-busy | 20 | 8195 | 3 |
| recv-busy | 21 | 8198 | 4 |
| recv-busy | 22 | 8197 | 3 |
| recv-busy | 23 | 8195 | 3 |
| recv-busy | 24 | 8193 | 3 |
| recv-busy | 25 | 8197 | 3 |
| recv-busy | 26 | 8200 | 3 |
| recv-busy | 27 | 8199 | 3 |
| recv-busy | 28 | 8196 | 3 |
| recv-busy | 29 | 8197 | 0 |
| recv-busy | 30 | 8193 | 3 |
| recv-busy | 31 | 8200 | 3 |
| recv-busy | 32 | 8200 | 4 |
| recv-busy | 33 | 8198 | 3 |
| recv-busy | 34 | 8200 | 4 |
| recv-busy | 35 | 8195 | 3 |
| recv-busy | 36 | 8195 | 3 |
| recv-busy | 37 | 8196 | 3 |
| recv-busy | 38 | 8194 | 4 |
| recv-busy | 39 | 8198 | 3 |
| recv-busy | 40 | 8196 | 3 |
| recv-busy | 41 | 8193 | 0 |
| recv-busy | 42 | 8199 | 3 |
| recv-busy | 43 | 8199 | 3 |
| recv-busy | 44 | 8200 | 3 |
| recv-busy | 45 | 8193 | 3 |
| recv-busy | 46 | 8194 | 3 |
| recv-busy | 47 | 8199 | 0 |
| recv-busy | 48 | 8194 | 3 |
| recv-busy | 49 | 8199 | 3 |
| recv-busy | 50 | 8196 | 4 |
| recv-busy | 51 | 8195 | 3 |
| recv-busy | 52 | 8199 | 0 |
| recv-busy | 53 | 8194 | 3 |
| recv-busy | 54 | 8193 | 3 |
| recv-busy | 55 | 8193 | 3 |
| recv-busy | 56 | 8198 | 3 |
| recv-busy | 57 | 8194 | 3 |
| recv-busy | 58 | 8199 | 3 |
| recv-busy | 59 | 8196 | 4 |
| recv-busy | 60 | 8194 | 3 |
| recv-busy | 61 | 8200 | 3 |
| recv-busy | 62 | 8196 | 4 |
| recv-busy | 63 | 8198 | 3 |
| recv-busy | 64 | 8199 | 3 |
| recv-busy | 65 | 8197 | 3 |
| recv-busy | 66 | 8197 | 3 |
| recv-busy | 67 | 8196 | 3 |
| recv-busy | 68 | 8198 | 4 |
| recv-busy | 69 | 8196 | 4 |
| recv-busy | 70 | 8200 | 3 |
| recv-busy | 71 | 8196 | 4 |
| recv-busy | 72 | 8199 | 3 |
| recv-busy | 73 | 8198 | 3 |
| recv-busy | 74 | 8195 | 3 |
| recv-busy | 75 | 8195 | 0 |
| recv-busy | 76 | 8200 | 3 |
| recv-busy | 77 | 8199 | 0 |
| recv-busy | 78 | 8195 | 3 |
| recv-busy | 79 | 8196 | 4 |
| recv-busy | 80 | 8194 | 3 |
| recv-busy | 81 | 8200 | 3 |
| recv-busy | 82 | 8198 | 4 |
| recv-busy | 83 | 8193 | 3 |
| recv-busy | 84 | 8197 | 3 |
| recv-busy | 85 | 8195 | 3 |
| recv-busy | 86 | 8195 | 3 |
| recv-busy | 87 | 8195 | 3 |
| recv-busy | 88 | 8200 | 3 |
| recv-busy | 89 | 8197 | 0 |
| recv-busy | 90 | 8199 | 0 |
| recv-busy | 91 | 8198 | 3 |
| recv-busy | 92 | 8196 | 3 |
| recv-busy | 93 | 8199 | 3 |
| recv-busy | 94 | 8198 | 3 |
| recv-busy | 95 | 8197 | 0 |
| recv-busy | 96 | 8200 | 3 |
| recv-busy | 97 | 8197 | 0 |
| recv-busy | 98 | 8195 | 3 |
| recv-busy | 99 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1018262112 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1530755 of 4387015 in 1-msg reads) | 1530755 | 5.95 | 5.46 | 8.70 | 20.17 | 33.40 | 46.91 | 85.68 |

#### Stalls: m1/lab-recv-busy-pq-c100, loop 0 on CPU 3

12260544 iterations, longest 2222.2 µs; over 5 / 20 / 100 µs: 1884097 / 51261 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 44903 | 1237744.0 | 2222.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4914 | 141228.2 | 51.4 |
| mostly unexplained (+preempted by kworker/3:1) | 1222 | 28596.9 | 82.3 |
| mostly unexplained (+softirq NET_RX) | 126 | 3373.0 | 43.8 |
| mostly unexplained (+irq work (IPI/vector)) | 60 | 1580.9 | 47.6 |
| mostly unexplained (+local timer tick) | 26 | 742.0 | 45.3 |
| softirq NET_RX | 5 | 160.7 | 41.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 64.5 | 42.9 |
| mostly unexplained (+ipi_send_cpu) | 2 | 56.9 | 29.4 |
| local timer tick | 1 | 21.2 | 21.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2222.2 | 5968 | loop work (5968 items) |  |
| 2 | 614.6 | 1768 | loop work (1768 items) |  |
| 3 | 107.0 | 61 | loop work (61 items) | softirq NET_RX 6 |
| 4 | 103.4 | 62 | loop work (62 items) | softirq NET_RX 1 |
| 5 | 99.9 | 62 | loop work (62 items) | softirq NET_RX 1 |
| 6 | 98.5 | 46 | loop work (46 items) | softirq NET_RX 3 |
| 7 | 92.7 | 70 | loop work (70 items) |  |
| 8 | 90.5 | 50 | loop work (50 items) |  |
| 9 | 89.8 | 46 | loop work (46 items) | preempted by kworker/3:1 10, workqueue [ena] 7, local timer tick 3, irq work (IPI/vector) 2 |
| 10 | 88.5 | 47 | loop work (47 items) | local timer tick 3, preempted by kworker/3:1 2, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4670 |
| LOC Local timer interrupts | 4518 |
| CAL Function call interrupts | 990 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=109876 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355509 | 127671 | 2.78 | 1 | 0.00 | 0 | 37425820 | 0.0 | 0.0 | true | 0 | 0 | 391.4 | 58 / 4 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8199 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (64926 of 343095 in 1-msg reads) | 64926 | 3.61 | 3.48 | 4.19 | 5.19 | 12.94 | 19.78 | 94.50 |

#### Stalls: m1/lab-uring-sqpoll-c1, loop 0 on CPU 3

2245549184 iterations, longest 391.4 µs; over 5 / 20 / 100 µs: 58 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 391.4 | 391.4 |
| loop work | 1 | 89.9 | 89.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 65.8 | 33.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 391.4 | 10 | softirq TIMER | softirq TIMER 373, local timer tick 9, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 89.9 | 589 | loop work (589 items) |  |
| 3 | 33.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 31.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| LOC Local timer interrupts | 5 |
| IWI IRQ work interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=771444 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3164102 | 850608 | 3.72 | 1 | 0.00 | 0 | 32874235 | 0.0 | 0.0 | true | 0 | 0 | 743.3 | 2040 / 3 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8196 | 2 |
| uring-sqpoll | 1 | 8198 | 2 |
| uring-sqpoll | 2 | 8199 | 2 |
| uring-sqpoll | 3 | 8194 | 2 |
| uring-sqpoll | 4 | 8196 | 2 |
| uring-sqpoll | 5 | 8199 | 2 |
| uring-sqpoll | 6 | 8194 | 2 |
| uring-sqpoll | 7 | 8193 | 2 |
| uring-sqpoll | 8 | 8200 | 2 |
| uring-sqpoll | 9 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708758848 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (251343 of 3053556 in 1-msg reads) | 251343 | 4.54 | 3.79 | 6.97 | 12.78 | 23.44 | 39.35 | 109.42 |

#### Stalls: m1/lab-uring-sqpoll-c10, loop 0 on CPU 3

1972454080 iterations, longest 743.3 µs; over 5 / 20 / 100 µs: 2040 / 3 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 743.3 | 743.3 |
| softirq TIMER | 1 | 379.5 | 379.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.8 | 35.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 743.3 | 5271 | loop work (5271 items) |  |
| 2 | 379.5 | 1 | softirq TIMER | softirq TIMER 360, local timer tick 11, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 3 | 35.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 5 |
| IWI IRQ work interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2024378 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4545786 | 2224364 | 2.04 | 1 | 0.00 | 0 | 8000930 | 0.0 | 0.0 | true | 0 | 0 | 692.3 | 3436 / 4 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8196 | 2 |
| uring-sqpoll | 1 | 8198 | 2 |
| uring-sqpoll | 2 | 8195 | 2 |
| uring-sqpoll | 3 | 8194 | 2 |
| uring-sqpoll | 4 | 8194 | 2 |
| uring-sqpoll | 5 | 8196 | 2 |
| uring-sqpoll | 6 | 8197 | 2 |
| uring-sqpoll | 7 | 8195 | 2 |
| uring-sqpoll | 8 | 8200 | 2 |
| uring-sqpoll | 9 | 8199 | 2 |
| uring-sqpoll | 10 | 8193 | 2 |
| uring-sqpoll | 11 | 8198 | 2 |
| uring-sqpoll | 12 | 8197 | 2 |
| uring-sqpoll | 13 | 8200 | 2 |
| uring-sqpoll | 14 | 8197 | 2 |
| uring-sqpoll | 15 | 8195 | 2 |
| uring-sqpoll | 16 | 8197 | 2 |
| uring-sqpoll | 17 | 8196 | 2 |
| uring-sqpoll | 18 | 8198 | 2 |
| uring-sqpoll | 19 | 8195 | 2 |
| uring-sqpoll | 20 | 8200 | 2 |
| uring-sqpoll | 21 | 8195 | 2 |
| uring-sqpoll | 22 | 8200 | 2 |
| uring-sqpoll | 23 | 8197 | 2 |
| uring-sqpoll | 24 | 8194 | 2 |
| uring-sqpoll | 25 | 8196 | 2 |
| uring-sqpoll | 26 | 8197 | 2 |
| uring-sqpoll | 27 | 8196 | 2 |
| uring-sqpoll | 28 | 8197 | 2 |
| uring-sqpoll | 29 | 8198 | 4 |
| uring-sqpoll | 30 | 8193 | 2 |
| uring-sqpoll | 31 | 8200 | 2 |
| uring-sqpoll | 32 | 8199 | 2 |
| uring-sqpoll | 33 | 8193 | 2 |
| uring-sqpoll | 34 | 8195 | 2 |
| uring-sqpoll | 35 | 8193 | 2 |
| uring-sqpoll | 36 | 8193 | 2 |
| uring-sqpoll | 37 | 8195 | 2 |
| uring-sqpoll | 38 | 8199 | 2 |
| uring-sqpoll | 39 | 8194 | 2 |
| uring-sqpoll | 40 | 8199 | 2 |
| uring-sqpoll | 41 | 8194 | 2 |
| uring-sqpoll | 42 | 8196 | 2 |
| uring-sqpoll | 43 | 8193 | 2 |
| uring-sqpoll | 44 | 8200 | 2 |
| uring-sqpoll | 45 | 8200 | 2 |
| uring-sqpoll | 46 | 8198 | 2 |
| uring-sqpoll | 47 | 8195 | 2 |
| uring-sqpoll | 48 | 8193 | 2 |
| uring-sqpoll | 49 | 8193 | 2 |
| uring-sqpoll | 50 | 8195 | 2 |
| uring-sqpoll | 51 | 8194 | 2 |
| uring-sqpoll | 52 | 8200 | 2 |
| uring-sqpoll | 53 | 8197 | 2 |
| uring-sqpoll | 54 | 8196 | 2 |
| uring-sqpoll | 55 | 8198 | 2 |
| uring-sqpoll | 56 | 8199 | 2 |
| uring-sqpoll | 57 | 8199 | 2 |
| uring-sqpoll | 58 | 8197 | 2 |
| uring-sqpoll | 59 | 8196 | 2 |
| uring-sqpoll | 60 | 8193 | 2 |
| uring-sqpoll | 61 | 8193 | 2 |
| uring-sqpoll | 62 | 8194 | 4 |
| uring-sqpoll | 63 | 8196 | 2 |
| uring-sqpoll | 64 | 8198 | 2 |
| uring-sqpoll | 65 | 8194 | 2 |
| uring-sqpoll | 66 | 8198 | 2 |
| uring-sqpoll | 67 | 8198 | 2 |
| uring-sqpoll | 68 | 8199 | 2 |
| uring-sqpoll | 69 | 8200 | 2 |
| uring-sqpoll | 70 | 8196 | 2 |
| uring-sqpoll | 71 | 8195 | 2 |
| uring-sqpoll | 72 | 8200 | 2 |
| uring-sqpoll | 73 | 8194 | 2 |
| uring-sqpoll | 74 | 8197 | 2 |
| uring-sqpoll | 75 | 8199 | 2 |
| uring-sqpoll | 76 | 8196 | 2 |
| uring-sqpoll | 77 | 8196 | 2 |
| uring-sqpoll | 78 | 8195 | 2 |
| uring-sqpoll | 79 | 8200 | 2 |
| uring-sqpoll | 80 | 8197 | 2 |
| uring-sqpoll | 81 | 8194 | 2 |
| uring-sqpoll | 82 | 8197 | 2 |
| uring-sqpoll | 83 | 8198 | 2 |
| uring-sqpoll | 84 | 8194 | 2 |
| uring-sqpoll | 85 | 8194 | 2 |
| uring-sqpoll | 86 | 8196 | 2 |
| uring-sqpoll | 87 | 8193 | 2 |
| uring-sqpoll | 88 | 8199 | 2 |
| uring-sqpoll | 89 | 8198 | 2 |
| uring-sqpoll | 90 | 8196 | 2 |
| uring-sqpoll | 91 | 8196 | 2 |
| uring-sqpoll | 92 | 8198 | 2 |
| uring-sqpoll | 93 | 8195 | 2 |
| uring-sqpoll | 94 | 8199 | 2 |
| uring-sqpoll | 95 | 8197 | 0 |
| uring-sqpoll | 96 | 8197 | 2 |
| uring-sqpoll | 97 | 8194 | 2 |
| uring-sqpoll | 98 | 8197 | 2 |
| uring-sqpoll | 99 | 8196 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1018256064 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1582774 of 4386956 in 1-msg reads) | 1582774 | 4.17 | 3.57 | 6.05 | 13.93 | 26.72 | 38.90 | 57.69 |

#### Stalls: m1/lab-uring-sqpoll-c100, loop 0 on CPU 3

480055808 iterations, longest 692.3 µs; over 5 / 20 / 100 µs: 3436 / 4 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1115.5 | 692.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.0 | 34.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 34.0 | 34.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 692.3 | 4732 | loop work (4732 items) |  |
| 2 | 423.2 | 2898 | loop work (2898 items) |  |
| 3 | 34.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 34.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 3 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-sib-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=104292 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-sib-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355508 | 126075 | 2.82 | 1 | 0.00 | 0 | 39180457 | 0.0 | 0.0 | true | 0 | 0 | 106.5 | 56 / 3 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8199 | 7 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79633792 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (63560 of 343094 in 1-msg reads) | 63560 | 4.03 | 3.92 | 4.64 | 5.52 | 14.06 | 22.84 | 32.18 |

#### Stalls: m1/lab-uring-sqpoll-sib-c1, loop 0 on CPU 3

2350827392 iterations, longest 106.5 µs; over 5 / 20 / 100 µs: 56 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 106.5 | 106.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.5 | 33.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 21.1 | 21.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 106.5 | 589 | loop work (589 items) |  |
| 2 | 33.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 21.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-sib-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=743996 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-sib-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3164102 | 794265 | 3.98 | 1 | 0.00 | 0 | 23206628 | 0.0 | 0.0 | true | 0 | 0 | 914.4 | 3419 / 3 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8193 | 7 |
| uring-sqpoll | 1 | 8194 | 7 |
| uring-sqpoll | 2 | 8193 | 7 |
| uring-sqpoll | 3 | 8196 | 7 |
| uring-sqpoll | 4 | 8195 | 7 |
| uring-sqpoll | 5 | 8198 | 7 |
| uring-sqpoll | 6 | 8193 | 7 |
| uring-sqpoll | 7 | 8197 | 7 |
| uring-sqpoll | 8 | 8193 | 7 |
| uring-sqpoll | 9 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708758848 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (210667 of 3053552 in 1-msg reads) | 210667 | 5.86 | 4.88 | 9.09 | 16.91 | 32.19 | 47.52 | 100.87 |

#### Stalls: m1/lab-uring-sqpoll-sib-c10, loop 0 on CPU 3

1392397696 iterations, longest 914.4 µs; over 5 / 20 / 100 µs: 3419 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 914.4 | 914.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 31.0 | 31.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 21.1 | 21.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 914.4 | 5272 | loop work (5272 items) |  |
| 2 | 31.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 21.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-sib-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1899542 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-sib-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4545787 | 2161106 | 2.10 | 1 | 0.00 | 0 | 4539573 | 0.0 | 0.0 | true | 0 | 0 | 855.7 | 6676 / 5 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8197 | 7 |
| uring-sqpoll | 1 | 8195 | 7 |
| uring-sqpoll | 2 | 8197 | 7 |
| uring-sqpoll | 3 | 8193 | 7 |
| uring-sqpoll | 4 | 8197 | 7 |
| uring-sqpoll | 5 | 8194 | 7 |
| uring-sqpoll | 6 | 8199 | 7 |
| uring-sqpoll | 7 | 8194 | 7 |
| uring-sqpoll | 8 | 8196 | 7 |
| uring-sqpoll | 9 | 8197 | 7 |
| uring-sqpoll | 10 | 8199 | 7 |
| uring-sqpoll | 11 | 8196 | 7 |
| uring-sqpoll | 12 | 8196 | 7 |
| uring-sqpoll | 13 | 8194 | 7 |
| uring-sqpoll | 14 | 8199 | 7 |
| uring-sqpoll | 15 | 8194 | 7 |
| uring-sqpoll | 16 | 8194 | 4 |
| uring-sqpoll | 17 | 8196 | 7 |
| uring-sqpoll | 18 | 8197 | 7 |
| uring-sqpoll | 19 | 8199 | 7 |
| uring-sqpoll | 20 | 8194 | 7 |
| uring-sqpoll | 21 | 8195 | 7 |
| uring-sqpoll | 22 | 8193 | 7 |
| uring-sqpoll | 23 | 8198 | 7 |
| uring-sqpoll | 24 | 8196 | 4 |
| uring-sqpoll | 25 | 8199 | 7 |
| uring-sqpoll | 26 | 8194 | 7 |
| uring-sqpoll | 27 | 8193 | 7 |
| uring-sqpoll | 28 | 8200 | 7 |
| uring-sqpoll | 29 | 8199 | 0 |
| uring-sqpoll | 30 | 8199 | 0 |
| uring-sqpoll | 31 | 8195 | 7 |
| uring-sqpoll | 32 | 8200 | 7 |
| uring-sqpoll | 33 | 8194 | 7 |
| uring-sqpoll | 34 | 8194 | 4 |
| uring-sqpoll | 35 | 8200 | 7 |
| uring-sqpoll | 36 | 8195 | 7 |
| uring-sqpoll | 37 | 8198 | 7 |
| uring-sqpoll | 38 | 8194 | 7 |
| uring-sqpoll | 39 | 8193 | 7 |
| uring-sqpoll | 40 | 8199 | 7 |
| uring-sqpoll | 41 | 8194 | 7 |
| uring-sqpoll | 42 | 8197 | 7 |
| uring-sqpoll | 43 | 8199 | 7 |
| uring-sqpoll | 44 | 8193 | 7 |
| uring-sqpoll | 45 | 8198 | 7 |
| uring-sqpoll | 46 | 8194 | 7 |
| uring-sqpoll | 47 | 8200 | 7 |
| uring-sqpoll | 48 | 8196 | 7 |
| uring-sqpoll | 49 | 8195 | 7 |
| uring-sqpoll | 50 | 8196 | 7 |
| uring-sqpoll | 51 | 8198 | 7 |
| uring-sqpoll | 52 | 8195 | 7 |
| uring-sqpoll | 53 | 8200 | 4 |
| uring-sqpoll | 54 | 8196 | 7 |
| uring-sqpoll | 55 | 8194 | 4 |
| uring-sqpoll | 56 | 8199 | 7 |
| uring-sqpoll | 57 | 8195 | 7 |
| uring-sqpoll | 58 | 8196 | 7 |
| uring-sqpoll | 59 | 8195 | 0 |
| uring-sqpoll | 60 | 8194 | 7 |
| uring-sqpoll | 61 | 8196 | 7 |
| uring-sqpoll | 62 | 8198 | 4 |
| uring-sqpoll | 63 | 8197 | 0 |
| uring-sqpoll | 64 | 8200 | 7 |
| uring-sqpoll | 65 | 8195 | 7 |
| uring-sqpoll | 66 | 8200 | 7 |
| uring-sqpoll | 67 | 8200 | 7 |
| uring-sqpoll | 68 | 8200 | 7 |
| uring-sqpoll | 69 | 8194 | 7 |
| uring-sqpoll | 70 | 8197 | 7 |
| uring-sqpoll | 71 | 8194 | 7 |
| uring-sqpoll | 72 | 8200 | 7 |
| uring-sqpoll | 73 | 8200 | 7 |
| uring-sqpoll | 74 | 8196 | 7 |
| uring-sqpoll | 75 | 8199 | 7 |
| uring-sqpoll | 76 | 8198 | 7 |
| uring-sqpoll | 77 | 8196 | 7 |
| uring-sqpoll | 78 | 8198 | 7 |
| uring-sqpoll | 79 | 8199 | 7 |
| uring-sqpoll | 80 | 8196 | 7 |
| uring-sqpoll | 81 | 8199 | 7 |
| uring-sqpoll | 82 | 8197 | 7 |
| uring-sqpoll | 83 | 8193 | 7 |
| uring-sqpoll | 84 | 8199 | 7 |
| uring-sqpoll | 85 | 8198 | 7 |
| uring-sqpoll | 86 | 8198 | 7 |
| uring-sqpoll | 87 | 8200 | 7 |
| uring-sqpoll | 88 | 8193 | 7 |
| uring-sqpoll | 89 | 8196 | 4 |
| uring-sqpoll | 90 | 8195 | 7 |
| uring-sqpoll | 91 | 8199 | 7 |
| uring-sqpoll | 92 | 8200 | 7 |
| uring-sqpoll | 93 | 8198 | 7 |
| uring-sqpoll | 94 | 8195 | 7 |
| uring-sqpoll | 95 | 8193 | 7 |
| uring-sqpoll | 96 | 8194 | 7 |
| uring-sqpoll | 97 | 8193 | 7 |
| uring-sqpoll | 98 | 8199 | 7 |
| uring-sqpoll | 99 | 8196 | 7 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1018256288 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1543331 of 4386953 in 1-msg reads) | 1543331 | 5.73 | 4.74 | 8.67 | 21.15 | 37.15 | 52.36 | 84.90 |

#### Stalls: m1/lab-uring-sqpoll-sib-c100, loop 0 on CPU 3

272374400 iterations, longest 855.7 µs; over 5 / 20 / 100 µs: 6676 / 5 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1380.0 | 855.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 93.4 | 73.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.0 | 35.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 855.7 | 4731 | loop work (4731 items) |  |
| 2 | 524.3 | 2917 | loop work (2917 items) |  |
| 3 | 73.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 35.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 20.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 355508 | 140035 | 2.54 | 266484739 | 749.59 | 4441412 | 4439078 | 0.0 | 35.2 | false | 0 | 0 | 172.9 | 11518 / 11 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 79633792 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (81903 of 343094 in 1-msg reads) | 81903 | 5.04 | 3.48 | 11.63 | 16.90 | 20.81 | 27.16 | 29.62 |

#### Stalls: m2/lab-epoll-spin-c1, loop 0 on CPU 3

266344704 iterations, longest 172.9 µs; over 5 / 20 / 100 µs: 11518 / 11 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 204.1 | 27.7 |
| loop work | 1 | 172.9 | 172.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 29.4 | 29.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 172.9 | 589 | loop work (589 items) |  |
| 2 | 29.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 27.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 24.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 22.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 22.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 22.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 22.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 21.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 20.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3164102 | 930610 | 3.40 | 256701234 | 81.13 | 4278354 | 4262844 | 0.0 | 36.7 | false | 0 | 0 | 1464.3 | 128802 / 97 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8200 | 4 |
| epoll-spin | 1 | 8199 | 0 |
| epoll-spin | 2 | 8195 | 0 |
| epoll-spin | 3 | 8193 | 0 |
| epoll-spin | 4 | 8198 | 4 |
| epoll-spin | 5 | 8193 | 0 |
| epoll-spin | 6 | 8199 | 0 |
| epoll-spin | 7 | 8196 | 4 |
| epoll-spin | 8 | 8193 | 0 |
| epoll-spin | 9 | 8194 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 708758848 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (355717 of 3053557 in 1-msg reads) | 355717 | 5.31 | 5.03 | 8.85 | 14.36 | 20.79 | 28.82 | 36.38 |

#### Stalls: m2/lab-epoll-spin-c10, loop 0 on CPU 3

255770624 iterations, longest 1464.3 µs; over 5 / 20 / 100 µs: 128802 / 97 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 52 | 2747.3 | 1464.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 44 | 1091.0 | 34.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 32.7 | 32.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1464.3 | 5272 | loop work (5272 items) |  |
| 2 | 35.0 | 2 | loop work (2 items) |  |
| 3 | 34.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 34.3 | 2 | loop work (2 items) |  |
| 5 | 34.2 | 7 | loop work (7 items) |  |
| 6 | 32.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 32.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 32.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.0 | 3 | loop work (3 items) |  |
| 10 | 31.2 | 7 | loop work (7 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 4545780 | 2281626 | 1.99 | 204743194 | 45.04 | 3412387 | 3374359 | 0.0 | 34.8 | false | 0 | 0 | 1916.6 | 196693 / 758 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8197 | 0 |
| epoll-spin | 1 | 8194 | 4 |
| epoll-spin | 2 | 8200 | 4 |
| epoll-spin | 3 | 8196 | 4 |
| epoll-spin | 4 | 8197 | 0 |
| epoll-spin | 5 | 8199 | 0 |
| epoll-spin | 6 | 8200 | 4 |
| epoll-spin | 7 | 8199 | 0 |
| epoll-spin | 8 | 8198 | 4 |
| epoll-spin | 9 | 8193 | 0 |
| epoll-spin | 10 | 8197 | 0 |
| epoll-spin | 11 | 8194 | 4 |
| epoll-spin | 12 | 8195 | 0 |
| epoll-spin | 13 | 8196 | 4 |
| epoll-spin | 14 | 8198 | 4 |
| epoll-spin | 15 | 8200 | 4 |
| epoll-spin | 16 | 8198 | 4 |
| epoll-spin | 17 | 8199 | 0 |
| epoll-spin | 18 | 8197 | 0 |
| epoll-spin | 19 | 8199 | 0 |
| epoll-spin | 20 | 8195 | 0 |
| epoll-spin | 21 | 8198 | 4 |
| epoll-spin | 22 | 8196 | 4 |
| epoll-spin | 23 | 8194 | 4 |
| epoll-spin | 24 | 8195 | 0 |
| epoll-spin | 25 | 8198 | 4 |
| epoll-spin | 26 | 8196 | 4 |
| epoll-spin | 27 | 8194 | 4 |
| epoll-spin | 28 | 8198 | 4 |
| epoll-spin | 29 | 8197 | 0 |
| epoll-spin | 30 | 8197 | 0 |
| epoll-spin | 31 | 8196 | 4 |
| epoll-spin | 32 | 8200 | 4 |
| epoll-spin | 33 | 8195 | 0 |
| epoll-spin | 34 | 8197 | 0 |
| epoll-spin | 35 | 8200 | 4 |
| epoll-spin | 36 | 8197 | 0 |
| epoll-spin | 37 | 8198 | 4 |
| epoll-spin | 38 | 8193 | 0 |
| epoll-spin | 39 | 8200 | 4 |
| epoll-spin | 40 | 8196 | 4 |
| epoll-spin | 41 | 8199 | 0 |
| epoll-spin | 42 | 8196 | 4 |
| epoll-spin | 43 | 8193 | 0 |
| epoll-spin | 44 | 8198 | 4 |
| epoll-spin | 45 | 8200 | 4 |
| epoll-spin | 46 | 8194 | 4 |
| epoll-spin | 47 | 8196 | 4 |
| epoll-spin | 48 | 8193 | 0 |
| epoll-spin | 49 | 8200 | 4 |
| epoll-spin | 50 | 8198 | 4 |
| epoll-spin | 51 | 8194 | 4 |
| epoll-spin | 52 | 8198 | 4 |
| epoll-spin | 53 | 8193 | 0 |
| epoll-spin | 54 | 8193 | 0 |
| epoll-spin | 55 | 8195 | 0 |
| epoll-spin | 56 | 8199 | 0 |
| epoll-spin | 57 | 8193 | 0 |
| epoll-spin | 58 | 8197 | 0 |
| epoll-spin | 59 | 8195 | 0 |
| epoll-spin | 60 | 8200 | 4 |
| epoll-spin | 61 | 8194 | 4 |
| epoll-spin | 62 | 8197 | 0 |
| epoll-spin | 63 | 8198 | 4 |
| epoll-spin | 64 | 8197 | 0 |
| epoll-spin | 65 | 8194 | 4 |
| epoll-spin | 66 | 8199 | 0 |
| epoll-spin | 67 | 8200 | 4 |
| epoll-spin | 68 | 8196 | 4 |
| epoll-spin | 69 | 8198 | 4 |
| epoll-spin | 70 | 8198 | 4 |
| epoll-spin | 71 | 8193 | 0 |
| epoll-spin | 72 | 8193 | 0 |
| epoll-spin | 73 | 8195 | 0 |
| epoll-spin | 74 | 8198 | 4 |
| epoll-spin | 75 | 8196 | 4 |
| epoll-spin | 76 | 8199 | 0 |
| epoll-spin | 77 | 8195 | 0 |
| epoll-spin | 78 | 8196 | 4 |
| epoll-spin | 79 | 8194 | 4 |
| epoll-spin | 80 | 8197 | 0 |
| epoll-spin | 81 | 8194 | 4 |
| epoll-spin | 82 | 8199 | 0 |
| epoll-spin | 83 | 8194 | 4 |
| epoll-spin | 84 | 8193 | 0 |
| epoll-spin | 85 | 8197 | 0 |
| epoll-spin | 86 | 8196 | 4 |
| epoll-spin | 87 | 8198 | 4 |
| epoll-spin | 88 | 8196 | 4 |
| epoll-spin | 89 | 8197 | 0 |
| epoll-spin | 90 | 8199 | 0 |
| epoll-spin | 91 | 8196 | 4 |
| epoll-spin | 92 | 8198 | 4 |
| epoll-spin | 93 | 8198 | 4 |
| epoll-spin | 94 | 8196 | 4 |
| epoll-spin | 95 | 8198 | 4 |
| epoll-spin | 96 | 8199 | 0 |
| epoll-spin | 97 | 8199 | 0 |
| epoll-spin | 98 | 8196 | 4 |
| epoll-spin | 99 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1018254720 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1674937 of 4387089 in 1-msg reads) | 1674937 | 4.21 | 3.20 | 7.00 | 14.02 | 22.50 | 31.06 | 60.64 |

#### Stalls: m2/lab-epoll-spin-c100, loop 0 on CPU 3

202461568 iterations, longest 1916.6 µs; over 5 / 20 / 100 µs: 196693 / 758 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 432 | 12354.8 | 1916.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 325 | 7690.9 | 40.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.5 | 35.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1916.6 | 5419 | loop work (5419 items) |  |
| 2 | 738.4 | 2233 | loop work (2233 items) |  |
| 3 | 47.7 | 82 | loop work (82 items) |  |
| 4 | 47.6 | 16 | loop work (16 items) |  |
| 5 | 40.7 | 3 | loop work (3 items) |  |
| 6 | 40.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 39.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 38.0 | 5 | loop work (5 items) |  |
| 9 | 37.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 8 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=117543 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355509 | 133228 | 2.67 | 119754112 | 336.85 | 1995902 | 1995902 | 99.9 | 66.7 | true | 0 | 217 | 178.9 | 17561 / 1185 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (68862 of 343095 in 1-msg reads) | 68862 | 2.52 | 2.50 | 2.79 | 3.35 | 6.53 | 18.71 | 26.40 |

#### Stalls: m2/lab-recv-busy-c1, loop 0 on CPU 3

119754112 iterations, longest 178.9 µs; over 5 / 20 / 100 µs: 17561 / 1185 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 698 | 19352.7 | 178.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 385 | 9268.7 | 38.0 |
| mostly unexplained (+softirq NET_RX) | 40 | 1002.1 | 46.2 |
| mostly unexplained (+preempted by kworker/3:1) | 36 | 874.5 | 48.3 |
| softirq NET_RX | 23 | 539.1 | 28.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 60.6 | 34.3 |
| mostly unexplained (+local timer tick) | 1 | 27.6 | 27.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 178.9 | 589 | loop work (589 items) |  |
| 2 | 51.3 | 10 | loop work (10 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 48.3 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 46.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 45.8 | 10 | loop work (10 items) |  |
| 6 | 41.8 | 10 | loop work (10 items) |  |
| 7 | 38.7 | 10 | loop work (10 items) |  |
| 8 | 38.2 | 10 | loop work (10 items) |  |
| 9 | 38.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.7 | 10 | loop work (10 items) | softirq NET_RX 2 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 227 |
| LOC Local timer interrupts | 220 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=765623 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3164103 | 817663 | 3.87 | 117612160 | 37.17 | 1960203 | 196020 | 99.3 | 68.9 | true | 0 | 1020 | 1421.5 | 835782 / 25417 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 3 |
| recv-busy | 1 | 8193 | 3 |
| recv-busy | 2 | 8194 | 3 |
| recv-busy | 3 | 8200 | 3 |
| recv-busy | 4 | 8196 | 3 |
| recv-busy | 5 | 8194 | 3 |
| recv-busy | 6 | 8193 | 3 |
| recv-busy | 7 | 8196 | 3 |
| recv-busy | 8 | 8198 | 3 |
| recv-busy | 9 | 8198 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708759072 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (232378 of 3053557 in 1-msg reads) | 232378 | 4.44 | 3.79 | 6.95 | 13.88 | 28.30 | 41.62 | 57.40 |

#### Stalls: m2/lab-recv-busy-c10, loop 0 on CPU 3

11761216 iterations, longest 1421.5 µs; over 5 / 20 / 100 µs: 835782 / 25417 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 23984 | 672090.1 | 1421.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1175 | 34680.0 | 46.9 |
| mostly unexplained (+preempted by kworker/3:1) | 169 | 4628.1 | 55.4 |
| mostly unexplained (+softirq NET_RX) | 72 | 2158.3 | 40.2 |
| softirq TIMER | 1 | 298.1 | 298.1 |
| mostly unexplained (+irq work (IPI/vector)) | 10 | 282.2 | 45.1 |
| softirq NET_RX | 2 | 92.0 | 50.4 |
| mostly unexplained (+local timer tick) | 3 | 69.4 | 27.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 39.0 | 39.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1421.5 | 5292 | loop work (5292 items) |  |
| 2 | 298.1 | 0 | softirq TIMER | softirq TIMER 274, local timer tick 11, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 3 | 91.0 | 50 | loop work (50 items) |  |
| 4 | 90.8 | 47 | loop work (47 items) |  |
| 5 | 90.0 | 60 | loop work (60 items) |  |
| 6 | 88.3 | 20 | loop work (20 items) |  |
| 7 | 86.9 | 43 | loop work (43 items) |  |
| 8 | 86.6 | 51 | loop work (51 items) |  |
| 9 | 85.7 | 34 | loop work (34 items) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 84.6 | 38 | loop work (38 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1048 |
| LOC Local timer interrupts | 1026 |
| CAL Function call interrupts | 991 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1594672 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4546052 | 1935004 | 2.35 | 108275200 | 23.82 | 1804587 | 18046 | 98.2 | 70.9 | true | 0 | 4297 | 2632.1 | 1082751 / 1082751 / 15546 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 0 |
| recv-busy | 1 | 8193 | 3 |
| recv-busy | 2 | 8195 | 3 |
| recv-busy | 3 | 8199 | 3 |
| recv-busy | 4 | 8200 | 3 |
| recv-busy | 5 | 8200 | 4 |
| recv-busy | 6 | 8195 | 3 |
| recv-busy | 7 | 8197 | 3 |
| recv-busy | 8 | 8193 | 3 |
| recv-busy | 9 | 8198 | 4 |
| recv-busy | 10 | 8199 | 3 |
| recv-busy | 11 | 8197 | 3 |
| recv-busy | 12 | 8194 | 3 |
| recv-busy | 13 | 8198 | 3 |
| recv-busy | 14 | 8200 | 3 |
| recv-busy | 15 | 8194 | 3 |
| recv-busy | 16 | 8200 | 3 |
| recv-busy | 17 | 8197 | 3 |
| recv-busy | 18 | 8199 | 3 |
| recv-busy | 19 | 8199 | 3 |
| recv-busy | 20 | 8197 | 0 |
| recv-busy | 21 | 8196 | 3 |
| recv-busy | 22 | 8193 | 3 |
| recv-busy | 23 | 8195 | 0 |
| recv-busy | 24 | 8194 | 3 |
| recv-busy | 25 | 8195 | 3 |
| recv-busy | 26 | 8193 | 3 |
| recv-busy | 27 | 8200 | 3 |
| recv-busy | 28 | 8193 | 3 |
| recv-busy | 29 | 8194 | 4 |
| recv-busy | 30 | 8195 | 3 |
| recv-busy | 31 | 8200 | 3 |
| recv-busy | 32 | 8196 | 3 |
| recv-busy | 33 | 8198 | 3 |
| recv-busy | 34 | 8196 | 4 |
| recv-busy | 35 | 8193 | 0 |
| recv-busy | 36 | 8198 | 3 |
| recv-busy | 37 | 8196 | 4 |
| recv-busy | 38 | 8195 | 3 |
| recv-busy | 39 | 8198 | 3 |
| recv-busy | 40 | 8199 | 0 |
| recv-busy | 41 | 8197 | 3 |
| recv-busy | 42 | 8199 | 0 |
| recv-busy | 43 | 8194 | 3 |
| recv-busy | 44 | 8194 | 3 |
| recv-busy | 45 | 8200 | 3 |
| recv-busy | 46 | 8197 | 0 |
| recv-busy | 47 | 8195 | 0 |
| recv-busy | 48 | 8194 | 4 |
| recv-busy | 49 | 8200 | 3 |
| recv-busy | 50 | 8200 | 3 |
| recv-busy | 51 | 8195 | 0 |
| recv-busy | 52 | 8200 | 3 |
| recv-busy | 53 | 8197 | 0 |
| recv-busy | 54 | 8197 | 0 |
| recv-busy | 55 | 8198 | 3 |
| recv-busy | 56 | 8193 | 3 |
| recv-busy | 57 | 8193 | 0 |
| recv-busy | 58 | 8198 | 3 |
| recv-busy | 59 | 8193 | 3 |
| recv-busy | 60 | 8197 | 3 |
| recv-busy | 61 | 8194 | 4 |
| recv-busy | 62 | 8195 | 3 |
| recv-busy | 63 | 8195 | 3 |
| recv-busy | 64 | 8200 | 3 |
| recv-busy | 65 | 8198 | 3 |
| recv-busy | 66 | 8195 | 3 |
| recv-busy | 67 | 8197 | 3 |
| recv-busy | 68 | 8197 | 0 |
| recv-busy | 69 | 8196 | 4 |
| recv-busy | 70 | 8197 | 3 |
| recv-busy | 71 | 8193 | 3 |
| recv-busy | 72 | 8199 | 3 |
| recv-busy | 73 | 8199 | 0 |
| recv-busy | 74 | 8200 | 4 |
| recv-busy | 75 | 8197 | 0 |
| recv-busy | 76 | 8198 | 3 |
| recv-busy | 77 | 8194 | 4 |
| recv-busy | 78 | 8196 | 4 |
| recv-busy | 79 | 8196 | 3 |
| recv-busy | 80 | 8200 | 3 |
| recv-busy | 81 | 8196 | 4 |
| recv-busy | 82 | 8195 | 0 |
| recv-busy | 83 | 8200 | 3 |
| recv-busy | 84 | 8199 | 3 |
| recv-busy | 85 | 8198 | 3 |
| recv-busy | 86 | 8197 | 3 |
| recv-busy | 87 | 8194 | 3 |
| recv-busy | 88 | 8193 | 0 |
| recv-busy | 89 | 8196 | 4 |
| recv-busy | 90 | 8200 | 3 |
| recv-busy | 91 | 8198 | 3 |
| recv-busy | 92 | 8193 | 3 |
| recv-busy | 93 | 8198 | 3 |
| recv-busy | 94 | 8193 | 3 |
| recv-busy | 95 | 8197 | 3 |
| recv-busy | 96 | 8196 | 3 |
| recv-busy | 97 | 8195 | 3 |
| recv-busy | 98 | 8195 | 0 |
| recv-busy | 99 | 8193 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1018315648 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1492992 of 4387357 in 1-msg reads) | 1492992 | 28.20 | 27.85 | 50.24 | 71.23 | 90.79 | 113.81 | 164.03 |

#### Stalls: m2/lab-recv-busy-c100, loop 0 on CPU 3

1082752 iterations, longest 2632.1 µs; over 5 / 20 / 100 µs: 1082751 / 1082751 / 15546 (34175 stalls not kept)

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 796073 | 41600421.2 | 126.4 |
| loop work | 225201 | 14825987.4 | 2632.1 |
| mostly unexplained (+softirq NET_RX) | 21173 | 1195156.6 | 137.3 |
| mostly unexplained (+local timer tick) | 3686 | 228415.0 | 109.3 |
| mostly unexplained (+preempted by kworker/3:1) | 1894 | 132665.1 | 108.1 |
| mostly unexplained (+irq work (IPI/vector)) | 504 | 33635.0 | 100.8 |
| mostly unexplained (+softirq NET_TX) | 16 | 1083.7 | 109.5 |
| mostly unexplained (+ipi_send_cpu) | 10 | 592.2 | 64.8 |
| mostly unexplained (+call function (IPI/vector)) | 7 | 393.0 | 62.1 |
| mostly unexplained (+call function single (IPI/vector)) | 4 | 250.2 | 68.5 |
| mostly unexplained (+softirq RCU) | 3 | 172.3 | 67.2 |
| mostly unexplained (+softirq TASKLET) | 2 | 110.9 | 56.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 94.7 | 48.2 |
| mostly unexplained (+softirq TIMER) | 1 | 69.7 | 69.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2632.1 | 7604 | loop work (7604 items) |  |
| 2 | 176.1 | 75 | loop work (75 items) |  |
| 3 | 175.7 | 81 | loop work (81 items) | softirq NET_RX 1 |
| 4 | 173.9 | 87 | loop work (87 items) | softirq NET_RX 1 |
| 5 | 172.8 | 72 | loop work (72 items) |  |
| 6 | 171.8 | 103 | loop work (103 items) | softirq NET_RX 1 |
| 7 | 171.4 | 36 | loop work (36 items) | softirq NET_RX 1 |
| 8 | 171.3 | 71 | loop work (71 items) |  |
| 9 | 169.2 | 33 | loop work (33 items) | softirq NET_RX 1 |
| 10 | 168.9 | 74 | loop work (74 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4333 |
| LOC Local timer interrupts | 4192 |
| CAL Function call interrupts | 991 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=116735 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355509 | 127443 | 2.79 | 166930048 | 469.55 | 2782167 | 1391084 | 99.8 | 58.4 | true | 0 | 180 | 302.4 | 29135 / 1292 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (64047 of 343095 in 1-msg reads) | 64047 | 2.90 | 2.75 | 3.52 | 4.62 | 6.62 | 33.60 | 41.48 |

#### Stalls: m2/lab-recv-busy-pq-c1, loop 0 on CPU 3

83465024 iterations, longest 302.4 µs; over 5 / 20 / 100 µs: 29135 / 1292 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 765 | 22746.2 | 210.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 405 | 10174.6 | 36.7 |
| mostly unexplained (+preempted by kworker/3:1) | 65 | 1997.5 | 63.1 |
| mostly unexplained (+softirq NET_RX) | 36 | 1014.2 | 61.0 |
| softirq NET_RX | 14 | 437.7 | 40.8 |
| softirq TIMER | 1 | 302.4 | 302.4 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 105.4 | 40.1 |
| mostly unexplained (+local timer tick) | 2 | 78.8 | 40.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.8 | 34.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 302.4 | 0 | softirq TIMER | softirq TIMER 282, local timer tick 11, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 210.7 | 589 | loop work (589 items) |  |
| 3 | 63.1 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 62.9 | 10 | loop work (10 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 61.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 21, preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 56.9 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 53.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 17, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 53.4 | 9 | loop work (9 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 50.9 | 10 | loop work (10 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 48.8 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| IWI IRQ work interrupts | 197 |
| LOC Local timer interrupts | 184 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=732182 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3164102 | 815044 | 3.88 | 125475377 | 39.66 | 2091256 | 260944 | 99.3 | 67.4 | true | 0 | 1124 | 1458.8 | 528232 / 19386 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 3 |
| recv-busy | 1 | 8194 | 3 |
| recv-busy | 2 | 8198 | 3 |
| recv-busy | 3 | 8200 | 3 |
| recv-busy | 4 | 8195 | 3 |
| recv-busy | 5 | 8200 | 4 |
| recv-busy | 6 | 8193 | 0 |
| recv-busy | 7 | 8197 | 3 |
| recv-busy | 8 | 8196 | 3 |
| recv-busy | 9 | 8194 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708758848 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (226049 of 3053556 in 1-msg reads) | 226049 | 4.29 | 2.91 | 7.53 | 14.72 | 28.85 | 41.02 | 56.41 |

#### Stalls: m2/lab-recv-busy-pq-c10, loop 0 on CPU 3

15656640 iterations, longest 1458.8 µs; over 5 / 20 / 100 µs: 528232 / 19386 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 18095 | 519681.5 | 1458.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1011 | 28913.0 | 44.1 |
| mostly unexplained (+preempted by kworker/3:1) | 219 | 5518.1 | 54.5 |
| mostly unexplained (+softirq NET_RX) | 36 | 1012.2 | 35.7 |
| softirq NET_RX | 7 | 260.3 | 56.4 |
| mostly unexplained (+irq work (IPI/vector)) | 10 | 243.1 | 30.1 |
| mostly unexplained (+local timer tick) | 7 | 195.8 | 40.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 39.1 | 39.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1458.8 | 5272 | loop work (5272 items) |  |
| 2 | 93.5 | 41 | loop work (41 items) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 91.3 | 51 | loop work (51 items) |  |
| 4 | 88.7 | 46 | loop work (46 items) | softirq NET_RX 6 |
| 5 | 84.6 | 40 | loop work (40 items) |  |
| 6 | 83.8 | 46 | loop work (46 items) |  |
| 7 | 83.4 | 33 | loop work (33 items) |  |
| 8 | 82.2 | 43 | loop work (43 items) |  |
| 9 | 81.5 | 36 | loop work (36 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 81.4 | 37 | loop work (37 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1160 |
| LOC Local timer interrupts | 1122 |
| CAL Function call interrupts | 992 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1728382 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4545905 | 2156679 | 2.11 | 111330017 | 24.49 | 1855500 | 202512 | 97.8 | 69.2 | true | 0 | 5072 | 2331.9 | 1936520 / 50529 / 6 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 3 |
| recv-busy | 1 | 8195 | 3 |
| recv-busy | 2 | 8200 | 3 |
| recv-busy | 3 | 8197 | 3 |
| recv-busy | 4 | 8196 | 3 |
| recv-busy | 5 | 8193 | 0 |
| recv-busy | 6 | 8199 | 3 |
| recv-busy | 7 | 8193 | 0 |
| recv-busy | 8 | 8200 | 3 |
| recv-busy | 9 | 8195 | 3 |
| recv-busy | 10 | 8193 | 3 |
| recv-busy | 11 | 8196 | 3 |
| recv-busy | 12 | 8198 | 3 |
| recv-busy | 13 | 8195 | 0 |
| recv-busy | 14 | 8197 | 3 |
| recv-busy | 15 | 8197 | 3 |
| recv-busy | 16 | 8200 | 4 |
| recv-busy | 17 | 8199 | 0 |
| recv-busy | 18 | 8200 | 3 |
| recv-busy | 19 | 8199 | 3 |
| recv-busy | 20 | 8193 | 3 |
| recv-busy | 21 | 8199 | 0 |
| recv-busy | 22 | 8193 | 3 |
| recv-busy | 23 | 8195 | 3 |
| recv-busy | 24 | 8195 | 3 |
| recv-busy | 25 | 8199 | 3 |
| recv-busy | 26 | 8197 | 3 |
| recv-busy | 27 | 8200 | 3 |
| recv-busy | 28 | 8195 | 3 |
| recv-busy | 29 | 8200 | 3 |
| recv-busy | 30 | 8197 | 3 |
| recv-busy | 31 | 8193 | 3 |
| recv-busy | 32 | 8196 | 3 |
| recv-busy | 33 | 8197 | 3 |
| recv-busy | 34 | 8199 | 3 |
| recv-busy | 35 | 8198 | 3 |
| recv-busy | 36 | 8193 | 3 |
| recv-busy | 37 | 8200 | 4 |
| recv-busy | 38 | 8197 | 3 |
| recv-busy | 39 | 8199 | 0 |
| recv-busy | 40 | 8193 | 3 |
| recv-busy | 41 | 8198 | 3 |
| recv-busy | 42 | 8196 | 3 |
| recv-busy | 43 | 8193 | 3 |
| recv-busy | 44 | 8195 | 3 |
| recv-busy | 45 | 8199 | 3 |
| recv-busy | 46 | 8199 | 3 |
| recv-busy | 47 | 8193 | 0 |
| recv-busy | 48 | 8200 | 3 |
| recv-busy | 49 | 8194 | 3 |
| recv-busy | 50 | 8199 | 3 |
| recv-busy | 51 | 8196 | 3 |
| recv-busy | 52 | 8197 | 0 |
| recv-busy | 53 | 8195 | 3 |
| recv-busy | 54 | 8197 | 0 |
| recv-busy | 55 | 8194 | 4 |
| recv-busy | 56 | 8196 | 3 |
| recv-busy | 57 | 8194 | 4 |
| recv-busy | 58 | 8195 | 3 |
| recv-busy | 59 | 8193 | 3 |
| recv-busy | 60 | 8197 | 3 |
| recv-busy | 61 | 8194 | 3 |
| recv-busy | 62 | 8194 | 4 |
| recv-busy | 63 | 8193 | 0 |
| recv-busy | 64 | 8197 | 3 |
| recv-busy | 65 | 8195 | 3 |
| recv-busy | 66 | 8194 | 3 |
| recv-busy | 67 | 8197 | 3 |
| recv-busy | 68 | 8194 | 3 |
| recv-busy | 69 | 8193 | 3 |
| recv-busy | 70 | 8199 | 3 |
| recv-busy | 71 | 8200 | 3 |
| recv-busy | 72 | 8195 | 3 |
| recv-busy | 73 | 8194 | 3 |
| recv-busy | 74 | 8200 | 3 |
| recv-busy | 75 | 8194 | 3 |
| recv-busy | 76 | 8200 | 3 |
| recv-busy | 77 | 8199 | 0 |
| recv-busy | 78 | 8194 | 3 |
| recv-busy | 79 | 8197 | 0 |
| recv-busy | 80 | 8200 | 4 |
| recv-busy | 81 | 8200 | 3 |
| recv-busy | 82 | 8198 | 3 |
| recv-busy | 83 | 8200 | 3 |
| recv-busy | 84 | 8195 | 0 |
| recv-busy | 85 | 8194 | 3 |
| recv-busy | 86 | 8196 | 3 |
| recv-busy | 87 | 8200 | 3 |
| recv-busy | 88 | 8193 | 3 |
| recv-busy | 89 | 8195 | 3 |
| recv-busy | 90 | 8199 | 3 |
| recv-busy | 91 | 8196 | 3 |
| recv-busy | 92 | 8194 | 4 |
| recv-busy | 93 | 8196 | 3 |
| recv-busy | 94 | 8193 | 3 |
| recv-busy | 95 | 8199 | 0 |
| recv-busy | 96 | 8193 | 3 |
| recv-busy | 97 | 8199 | 3 |
| recv-busy | 98 | 8195 | 3 |
| recv-busy | 99 | 8197 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1018282720 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1536409 of 4387095 in 1-msg reads) | 1536409 | 5.91 | 5.31 | 8.79 | 20.35 | 33.82 | 46.33 | 71.77 |

#### Stalls: m2/lab-recv-busy-pq-c100, loop 0 on CPU 3

12150720 iterations, longest 2331.9 µs; over 5 / 20 / 100 µs: 1936520 / 50529 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 44113 | 1230278.2 | 2331.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4574 | 135954.0 | 66.1 |
| mostly unexplained (+preempted by kworker/3:1) | 1553 | 38745.5 | 67.4 |
| mostly unexplained (+softirq NET_RX) | 138 | 3804.5 | 41.9 |
| mostly unexplained (+irq work (IPI/vector)) | 82 | 1959.3 | 44.7 |
| mostly unexplained (+local timer tick) | 62 | 1895.4 | 47.1 |
| softirq NET_RX | 6 | 222.8 | 56.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 43.1 | 43.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2331.9 | 5950 | loop work (5950 items) |  |
| 2 | 649.1 | 1796 | loop work (1796 items) |  |
| 3 | 109.8 | 53 | loop work (53 items) |  |
| 4 | 102.7 | 66 | loop work (66 items) | softirq NET_RX 3 |
| 5 | 101.8 | 45 | loop work (45 items) |  |
| 6 | 101.3 | 66 | loop work (66 items) |  |
| 7 | 99.7 | 68 | loop work (68 items) |  |
| 8 | 97.4 | 57 | loop work (57 items) |  |
| 9 | 97.3 | 52 | loop work (52 items) |  |
| 10 | 95.3 | 62 | loop work (62 items) | softirq NET_RX 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5117 |
| LOC Local timer interrupts | 4941 |
| CAL Function call interrupts | 991 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=114211 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355509 | 131234 | 2.71 | 1 | 0.00 | 0 | 36455440 | 0.0 | 0.0 | true | 0 | 0 | 89.7 | 72 / 2 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8196 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (67583 of 343095 in 1-msg reads) | 67583 | 3.38 | 3.21 | 4.13 | 5.48 | 16.42 | 25.61 | 28.16 |

#### Stalls: m2/lab-uring-sqpoll-c1, loop 0 on CPU 3

2187326400 iterations, longest 89.7 µs; over 5 / 20 / 100 µs: 72 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 89.7 | 89.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.3 | 33.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 89.7 | 589 | loop work (589 items) |  |
| 2 | 33.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=796650 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3164102 | 861291 | 3.67 | 1 | 0.00 | 0 | 29727131 | 0.0 | 0.0 | true | 0 | 0 | 743.3 | 2028 / 2 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8196 | 2 |
| uring-sqpoll | 1 | 8194 | 2 |
| uring-sqpoll | 2 | 8196 | 4 |
| uring-sqpoll | 3 | 8200 | 2 |
| uring-sqpoll | 4 | 8194 | 2 |
| uring-sqpoll | 5 | 8199 | 2 |
| uring-sqpoll | 6 | 8196 | 2 |
| uring-sqpoll | 7 | 8199 | 2 |
| uring-sqpoll | 8 | 8196 | 2 |
| uring-sqpoll | 9 | 8199 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708758848 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (259043 of 3053552 in 1-msg reads) | 259043 | 4.28 | 3.76 | 6.11 | 11.52 | 22.03 | 38.37 | 68.80 |

#### Stalls: m2/lab-uring-sqpoll-c10, loop 0 on CPU 3

1783627840 iterations, longest 743.3 µs; over 5 / 20 / 100 µs: 2028 / 2 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 743.3 | 743.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.7 | 33.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 743.3 | 5271 | loop work (5271 items) |  |
| 2 | 33.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2042226 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4545781 | 2236235 | 2.03 | 1 | 0.00 | 0 | 7797934 | 0.0 | 0.0 | true | 0 | 0 | 694.9 | 2735 / 4 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8198 | 2 |
| uring-sqpoll | 1 | 8195 | 2 |
| uring-sqpoll | 2 | 8199 | 0 |
| uring-sqpoll | 3 | 8195 | 2 |
| uring-sqpoll | 4 | 8200 | 4 |
| uring-sqpoll | 5 | 8195 | 2 |
| uring-sqpoll | 6 | 8200 | 2 |
| uring-sqpoll | 7 | 8196 | 2 |
| uring-sqpoll | 8 | 8193 | 2 |
| uring-sqpoll | 9 | 8194 | 2 |
| uring-sqpoll | 10 | 8197 | 2 |
| uring-sqpoll | 11 | 8197 | 2 |
| uring-sqpoll | 12 | 8194 | 2 |
| uring-sqpoll | 13 | 8196 | 4 |
| uring-sqpoll | 14 | 8197 | 2 |
| uring-sqpoll | 15 | 8196 | 2 |
| uring-sqpoll | 16 | 8198 | 2 |
| uring-sqpoll | 17 | 8197 | 2 |
| uring-sqpoll | 18 | 8200 | 2 |
| uring-sqpoll | 19 | 8195 | 2 |
| uring-sqpoll | 20 | 8193 | 2 |
| uring-sqpoll | 21 | 8195 | 2 |
| uring-sqpoll | 22 | 8197 | 2 |
| uring-sqpoll | 23 | 8200 | 2 |
| uring-sqpoll | 24 | 8195 | 2 |
| uring-sqpoll | 25 | 8199 | 2 |
| uring-sqpoll | 26 | 8193 | 2 |
| uring-sqpoll | 27 | 8193 | 2 |
| uring-sqpoll | 28 | 8198 | 2 |
| uring-sqpoll | 29 | 8197 | 2 |
| uring-sqpoll | 30 | 8200 | 2 |
| uring-sqpoll | 31 | 8198 | 2 |
| uring-sqpoll | 32 | 8197 | 2 |
| uring-sqpoll | 33 | 8195 | 2 |
| uring-sqpoll | 34 | 8194 | 2 |
| uring-sqpoll | 35 | 8196 | 2 |
| uring-sqpoll | 36 | 8200 | 2 |
| uring-sqpoll | 37 | 8195 | 2 |
| uring-sqpoll | 38 | 8194 | 4 |
| uring-sqpoll | 39 | 8196 | 2 |
| uring-sqpoll | 40 | 8198 | 2 |
| uring-sqpoll | 41 | 8199 | 2 |
| uring-sqpoll | 42 | 8197 | 2 |
| uring-sqpoll | 43 | 8197 | 2 |
| uring-sqpoll | 44 | 8195 | 2 |
| uring-sqpoll | 45 | 8195 | 2 |
| uring-sqpoll | 46 | 8196 | 2 |
| uring-sqpoll | 47 | 8197 | 2 |
| uring-sqpoll | 48 | 8196 | 2 |
| uring-sqpoll | 49 | 8200 | 2 |
| uring-sqpoll | 50 | 8194 | 2 |
| uring-sqpoll | 51 | 8199 | 2 |
| uring-sqpoll | 52 | 8193 | 2 |
| uring-sqpoll | 53 | 8196 | 4 |
| uring-sqpoll | 54 | 8194 | 2 |
| uring-sqpoll | 55 | 8197 | 2 |
| uring-sqpoll | 56 | 8197 | 2 |
| uring-sqpoll | 57 | 8194 | 2 |
| uring-sqpoll | 58 | 8199 | 2 |
| uring-sqpoll | 59 | 8195 | 2 |
| uring-sqpoll | 60 | 8197 | 2 |
| uring-sqpoll | 61 | 8195 | 2 |
| uring-sqpoll | 62 | 8196 | 2 |
| uring-sqpoll | 63 | 8200 | 2 |
| uring-sqpoll | 64 | 8199 | 2 |
| uring-sqpoll | 65 | 8193 | 2 |
| uring-sqpoll | 66 | 8193 | 2 |
| uring-sqpoll | 67 | 8198 | 2 |
| uring-sqpoll | 68 | 8196 | 2 |
| uring-sqpoll | 69 | 8200 | 2 |
| uring-sqpoll | 70 | 8198 | 2 |
| uring-sqpoll | 71 | 8194 | 2 |
| uring-sqpoll | 72 | 8197 | 2 |
| uring-sqpoll | 73 | 8194 | 2 |
| uring-sqpoll | 74 | 8196 | 2 |
| uring-sqpoll | 75 | 8198 | 2 |
| uring-sqpoll | 76 | 8200 | 2 |
| uring-sqpoll | 77 | 8194 | 2 |
| uring-sqpoll | 78 | 8194 | 2 |
| uring-sqpoll | 79 | 8197 | 2 |
| uring-sqpoll | 80 | 8196 | 2 |
| uring-sqpoll | 81 | 8194 | 2 |
| uring-sqpoll | 82 | 8198 | 2 |
| uring-sqpoll | 83 | 8198 | 2 |
| uring-sqpoll | 84 | 8194 | 2 |
| uring-sqpoll | 85 | 8195 | 2 |
| uring-sqpoll | 86 | 8197 | 2 |
| uring-sqpoll | 87 | 8199 | 2 |
| uring-sqpoll | 88 | 8194 | 2 |
| uring-sqpoll | 89 | 8193 | 2 |
| uring-sqpoll | 90 | 8194 | 2 |
| uring-sqpoll | 91 | 8197 | 2 |
| uring-sqpoll | 92 | 8194 | 2 |
| uring-sqpoll | 93 | 8198 | 2 |
| uring-sqpoll | 94 | 8195 | 2 |
| uring-sqpoll | 95 | 8194 | 2 |
| uring-sqpoll | 96 | 8200 | 2 |
| uring-sqpoll | 97 | 8196 | 2 |
| uring-sqpoll | 98 | 8197 | 2 |
| uring-sqpoll | 99 | 8194 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1018254944 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1590095 of 4386957 in 1-msg reads) | 1590095 | 4.23 | 3.66 | 6.12 | 13.59 | 26.59 | 40.17 | 301.08 |

#### Stalls: m2/lab-uring-sqpoll-c100, loop 0 on CPU 3

467876032 iterations, longest 694.9 µs; over 5 / 20 / 100 µs: 2735 / 4 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1118.3 | 694.9 |
| softirq TIMER | 1 | 303.1 | 303.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 36.4 | 36.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 694.9 | 4729 | loop work (4729 items) |  |
| 2 | 423.5 | 2893 | loop work (2893 items) |  |
| 3 | 303.1 | 3 | softirq TIMER | softirq TIMER 281, local timer tick 11, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 4 | 36.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 8 |
| LOC Local timer interrupts | 5 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-sib-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=104917 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-sib-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355509 | 127192 | 2.80 | 1 | 0.00 | 0 | 39144833 | 0.0 | 0.0 | true | 0 | 0 | 107.4 | 80 / 4 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8200 | 7 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (64051 of 343095 in 1-msg reads) | 64051 | 4.19 | 4.07 | 4.84 | 5.77 | 7.83 | 18.95 | 34.72 |

#### Stalls: m2/lab-uring-sqpoll-sib-c1, loop 0 on CPU 3

2348689984 iterations, longest 107.4 µs; over 5 / 20 / 100 µs: 80 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 107.4 | 107.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 44.5 | 24.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.4 | 34.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 107.4 | 589 | loop work (589 items) |  |
| 2 | 34.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 24.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 20.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-sib-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=741747 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-sib-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3164102 | 783103 | 4.04 | 1 | 0.00 | 0 | 23189246 | 0.0 | 0.0 | true | 0 | 0 | 907.8 | 3832 / 4 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8195 | 7 |
| uring-sqpoll | 1 | 8198 | 4 |
| uring-sqpoll | 2 | 8196 | 7 |
| uring-sqpoll | 3 | 8196 | 7 |
| uring-sqpoll | 4 | 8194 | 7 |
| uring-sqpoll | 5 | 8198 | 7 |
| uring-sqpoll | 6 | 8200 | 7 |
| uring-sqpoll | 7 | 8195 | 7 |
| uring-sqpoll | 8 | 8193 | 7 |
| uring-sqpoll | 9 | 8194 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708758848 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (199475 of 3053552 in 1-msg reads) | 199475 | 5.59 | 4.68 | 8.58 | 16.52 | 32.74 | 48.14 | 66.75 |

#### Stalls: m2/lab-uring-sqpoll-sib-c10, loop 0 on CPU 3

1391354752 iterations, longest 907.8 µs; over 5 / 20 / 100 µs: 3832 / 4 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 907.8 | 907.8 |
| softirq TIMER | 1 | 101.2 | 101.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 90.9 | 90.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.0 | 35.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 907.8 | 5271 | loop work (5271 items) |  |
| 2 | 101.2 | 0 | softirq TIMER | softirq TIMER 83, local timer tick 9, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 3 | 90.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 35.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-sib-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1928284 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-sib-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4545783 | 2184176 | 2.08 | 1 | 0.00 | 0 | 4439424 | 0.0 | 0.0 | true | 0 | 0 | 852.9 | 5538 / 5 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8195 | 7 |
| uring-sqpoll | 1 | 8195 | 7 |
| uring-sqpoll | 2 | 8196 | 7 |
| uring-sqpoll | 3 | 8195 | 7 |
| uring-sqpoll | 4 | 8193 | 0 |
| uring-sqpoll | 5 | 8197 | 7 |
| uring-sqpoll | 6 | 8199 | 7 |
| uring-sqpoll | 7 | 8198 | 7 |
| uring-sqpoll | 8 | 8193 | 7 |
| uring-sqpoll | 9 | 8198 | 7 |
| uring-sqpoll | 10 | 8196 | 7 |
| uring-sqpoll | 11 | 8198 | 7 |
| uring-sqpoll | 12 | 8197 | 7 |
| uring-sqpoll | 13 | 8196 | 4 |
| uring-sqpoll | 14 | 8198 | 7 |
| uring-sqpoll | 15 | 8195 | 7 |
| uring-sqpoll | 16 | 8196 | 4 |
| uring-sqpoll | 17 | 8198 | 7 |
| uring-sqpoll | 18 | 8195 | 7 |
| uring-sqpoll | 19 | 8196 | 7 |
| uring-sqpoll | 20 | 8195 | 7 |
| uring-sqpoll | 21 | 8199 | 7 |
| uring-sqpoll | 22 | 8200 | 7 |
| uring-sqpoll | 23 | 8196 | 7 |
| uring-sqpoll | 24 | 8199 | 0 |
| uring-sqpoll | 25 | 8196 | 7 |
| uring-sqpoll | 26 | 8194 | 7 |
| uring-sqpoll | 27 | 8200 | 7 |
| uring-sqpoll | 28 | 8193 | 7 |
| uring-sqpoll | 29 | 8194 | 4 |
| uring-sqpoll | 30 | 8199 | 7 |
| uring-sqpoll | 31 | 8197 | 7 |
| uring-sqpoll | 32 | 8193 | 7 |
| uring-sqpoll | 33 | 8195 | 7 |
| uring-sqpoll | 34 | 8199 | 0 |
| uring-sqpoll | 35 | 8193 | 7 |
| uring-sqpoll | 36 | 8199 | 7 |
| uring-sqpoll | 37 | 8198 | 7 |
| uring-sqpoll | 38 | 8199 | 7 |
| uring-sqpoll | 39 | 8194 | 7 |
| uring-sqpoll | 40 | 8200 | 7 |
| uring-sqpoll | 41 | 8196 | 7 |
| uring-sqpoll | 42 | 8199 | 7 |
| uring-sqpoll | 43 | 8200 | 7 |
| uring-sqpoll | 44 | 8198 | 7 |
| uring-sqpoll | 45 | 8194 | 4 |
| uring-sqpoll | 46 | 8200 | 7 |
| uring-sqpoll | 47 | 8196 | 7 |
| uring-sqpoll | 48 | 8196 | 7 |
| uring-sqpoll | 49 | 8198 | 7 |
| uring-sqpoll | 50 | 8199 | 7 |
| uring-sqpoll | 51 | 8198 | 7 |
| uring-sqpoll | 52 | 8199 | 7 |
| uring-sqpoll | 53 | 8195 | 7 |
| uring-sqpoll | 54 | 8197 | 7 |
| uring-sqpoll | 55 | 8196 | 4 |
| uring-sqpoll | 56 | 8197 | 7 |
| uring-sqpoll | 57 | 8195 | 7 |
| uring-sqpoll | 58 | 8196 | 7 |
| uring-sqpoll | 59 | 8198 | 4 |
| uring-sqpoll | 60 | 8195 | 7 |
| uring-sqpoll | 61 | 8200 | 7 |
| uring-sqpoll | 62 | 8198 | 4 |
| uring-sqpoll | 63 | 8199 | 7 |
| uring-sqpoll | 64 | 8200 | 7 |
| uring-sqpoll | 65 | 8194 | 7 |
| uring-sqpoll | 66 | 8193 | 7 |
| uring-sqpoll | 67 | 8199 | 7 |
| uring-sqpoll | 68 | 8199 | 7 |
| uring-sqpoll | 69 | 8197 | 7 |
| uring-sqpoll | 70 | 8200 | 7 |
| uring-sqpoll | 71 | 8199 | 7 |
| uring-sqpoll | 72 | 8194 | 7 |
| uring-sqpoll | 73 | 8200 | 7 |
| uring-sqpoll | 74 | 8197 | 7 |
| uring-sqpoll | 75 | 8193 | 7 |
| uring-sqpoll | 76 | 8198 | 7 |
| uring-sqpoll | 77 | 8194 | 7 |
| uring-sqpoll | 78 | 8197 | 7 |
| uring-sqpoll | 79 | 8199 | 7 |
| uring-sqpoll | 80 | 8199 | 7 |
| uring-sqpoll | 81 | 8199 | 7 |
| uring-sqpoll | 82 | 8196 | 7 |
| uring-sqpoll | 83 | 8195 | 7 |
| uring-sqpoll | 84 | 8199 | 7 |
| uring-sqpoll | 85 | 8200 | 7 |
| uring-sqpoll | 86 | 8197 | 7 |
| uring-sqpoll | 87 | 8193 | 7 |
| uring-sqpoll | 88 | 8200 | 7 |
| uring-sqpoll | 89 | 8198 | 7 |
| uring-sqpoll | 90 | 8193 | 7 |
| uring-sqpoll | 91 | 8196 | 4 |
| uring-sqpoll | 92 | 8195 | 7 |
| uring-sqpoll | 93 | 8199 | 0 |
| uring-sqpoll | 94 | 8194 | 7 |
| uring-sqpoll | 95 | 8194 | 4 |
| uring-sqpoll | 96 | 8196 | 7 |
| uring-sqpoll | 97 | 8198 | 4 |
| uring-sqpoll | 98 | 8194 | 7 |
| uring-sqpoll | 99 | 8196 | 7 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1018255392 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1555523 of 4386954 in 1-msg reads) | 1555523 | 5.79 | 4.72 | 8.81 | 22.37 | 38.24 | 55.27 | 111.56 |

#### Stalls: m2/lab-uring-sqpoll-sib-c100, loop 0 on CPU 3

266365440 iterations, longest 852.9 µs; over 5 / 20 / 100 µs: 5538 / 5 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1397.3 | 852.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 72.9 | 42.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 20.3 | 20.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 852.9 | 4729 | loop work (4729 items) |  |
| 2 | 544.4 | 2894 | loop work (2894 items) |  |
| 3 | 42.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 30.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 20.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m3/lab-epoll-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m3/lab-epoll-spin-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 355510 | 136634 | 2.60 | 266210874 | 748.81 | 4436848 | 4434571 | 0.0 | 35.3 | false | 0 | 0 | 209.1 | 50649 / 6 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 79634240 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (82164 of 343085 in 1-msg reads) | 82164 | 9.72 | 11.89 | 16.91 | 18.21 | 20.13 | 23.07 | 40.51 |

#### Stalls: m3/lab-epoll-spin-c1, loop 0 on CPU 3

266074240 iterations, longest 209.1 µs; over 5 / 20 / 100 µs: 50649 / 6 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 209.1 | 209.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 90.0 | 25.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.0 | 35.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 209.1 | 593 | loop work (593 items) |  |
| 2 | 35.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 25.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 22.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 21.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 20.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-epoll-spin-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m3/lab-epoll-spin-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3164102 | 933320 | 3.39 | 256718088 | 81.13 | 4278635 | 4263079 | 0.0 | 36.8 | false | 0 | 0 | 1414.9 | 122056 / 79 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8193 | 0 |
| epoll-spin | 1 | 8198 | 4 |
| epoll-spin | 2 | 8200 | 4 |
| epoll-spin | 3 | 8194 | 4 |
| epoll-spin | 4 | 8197 | 0 |
| epoll-spin | 5 | 8199 | 0 |
| epoll-spin | 6 | 8198 | 4 |
| epoll-spin | 7 | 8193 | 0 |
| epoll-spin | 8 | 8195 | 0 |
| epoll-spin | 9 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 708758848 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (362336 of 3053556 in 1-msg reads) | 362336 | 5.11 | 4.95 | 8.35 | 13.28 | 18.45 | 27.11 | 37.86 |

#### Stalls: m3/lab-epoll-spin-c10, loop 0 on CPU 3

255784768 iterations, longest 1414.9 µs; over 5 / 20 / 100 µs: 122056 / 79 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 46 | 2564.4 | 1414.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 32 | 776.9 | 34.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.7 | 33.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1414.9 | 5283 | loop work (5283 items) |  |
| 2 | 40.2 | 93 | loop work (93 items) |  |
| 3 | 34.6 | 3 | loop work (3 items) |  |
| 4 | 34.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 33.8 | 3 | loop work (3 items) |  |
| 6 | 33.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 32.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 31.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 31.3 | 7 | loop work (7 items) |  |
| 10 | 31.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-epoll-spin-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m3/lab-epoll-spin-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 4545902 | 2301873 | 1.97 | 203789809 | 44.83 | 3396497 | 3358132 | 0.0 | 35.1 | false | 0 | 0 | 1984.5 | 197370 / 243 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8193 | 0 |
| epoll-spin | 1 | 8194 | 4 |
| epoll-spin | 2 | 8197 | 0 |
| epoll-spin | 3 | 8200 | 4 |
| epoll-spin | 4 | 8195 | 0 |
| epoll-spin | 5 | 8198 | 4 |
| epoll-spin | 6 | 8194 | 4 |
| epoll-spin | 7 | 8194 | 4 |
| epoll-spin | 8 | 8196 | 4 |
| epoll-spin | 9 | 8198 | 4 |
| epoll-spin | 10 | 8196 | 4 |
| epoll-spin | 11 | 8193 | 0 |
| epoll-spin | 12 | 8198 | 4 |
| epoll-spin | 13 | 8194 | 4 |
| epoll-spin | 14 | 8197 | 0 |
| epoll-spin | 15 | 8195 | 0 |
| epoll-spin | 16 | 8195 | 0 |
| epoll-spin | 17 | 8197 | 0 |
| epoll-spin | 18 | 8200 | 4 |
| epoll-spin | 19 | 8196 | 4 |
| epoll-spin | 20 | 8199 | 0 |
| epoll-spin | 21 | 8198 | 4 |
| epoll-spin | 22 | 8198 | 4 |
| epoll-spin | 23 | 8200 | 4 |
| epoll-spin | 24 | 8198 | 4 |
| epoll-spin | 25 | 8195 | 0 |
| epoll-spin | 26 | 8197 | 0 |
| epoll-spin | 27 | 8194 | 4 |
| epoll-spin | 28 | 8200 | 4 |
| epoll-spin | 29 | 8196 | 4 |
| epoll-spin | 30 | 8194 | 4 |
| epoll-spin | 31 | 8197 | 0 |
| epoll-spin | 32 | 8196 | 4 |
| epoll-spin | 33 | 8198 | 4 |
| epoll-spin | 34 | 8195 | 0 |
| epoll-spin | 35 | 8196 | 4 |
| epoll-spin | 36 | 8195 | 0 |
| epoll-spin | 37 | 8195 | 0 |
| epoll-spin | 38 | 8197 | 0 |
| epoll-spin | 39 | 8198 | 4 |
| epoll-spin | 40 | 8199 | 0 |
| epoll-spin | 41 | 8198 | 4 |
| epoll-spin | 42 | 8200 | 4 |
| epoll-spin | 43 | 8199 | 0 |
| epoll-spin | 44 | 8193 | 0 |
| epoll-spin | 45 | 8197 | 0 |
| epoll-spin | 46 | 8197 | 0 |
| epoll-spin | 47 | 8194 | 4 |
| epoll-spin | 48 | 8195 | 0 |
| epoll-spin | 49 | 8197 | 0 |
| epoll-spin | 50 | 8196 | 4 |
| epoll-spin | 51 | 8198 | 4 |
| epoll-spin | 52 | 8193 | 0 |
| epoll-spin | 53 | 8199 | 0 |
| epoll-spin | 54 | 8197 | 0 |
| epoll-spin | 55 | 8200 | 4 |
| epoll-spin | 56 | 8194 | 4 |
| epoll-spin | 57 | 8195 | 0 |
| epoll-spin | 58 | 8197 | 0 |
| epoll-spin | 59 | 8195 | 0 |
| epoll-spin | 60 | 8193 | 0 |
| epoll-spin | 61 | 8200 | 4 |
| epoll-spin | 62 | 8199 | 0 |
| epoll-spin | 63 | 8195 | 0 |
| epoll-spin | 64 | 8193 | 0 |
| epoll-spin | 65 | 8196 | 4 |
| epoll-spin | 66 | 8197 | 0 |
| epoll-spin | 67 | 8196 | 4 |
| epoll-spin | 68 | 8199 | 0 |
| epoll-spin | 69 | 8196 | 4 |
| epoll-spin | 70 | 8197 | 0 |
| epoll-spin | 71 | 8195 | 0 |
| epoll-spin | 72 | 8199 | 0 |
| epoll-spin | 73 | 8200 | 4 |
| epoll-spin | 74 | 8196 | 4 |
| epoll-spin | 75 | 8198 | 4 |
| epoll-spin | 76 | 8198 | 4 |
| epoll-spin | 77 | 8197 | 0 |
| epoll-spin | 78 | 8195 | 0 |
| epoll-spin | 79 | 8199 | 0 |
| epoll-spin | 80 | 8199 | 0 |
| epoll-spin | 81 | 8197 | 0 |
| epoll-spin | 82 | 8196 | 4 |
| epoll-spin | 83 | 8194 | 4 |
| epoll-spin | 84 | 8197 | 0 |
| epoll-spin | 85 | 8199 | 0 |
| epoll-spin | 86 | 8197 | 0 |
| epoll-spin | 87 | 8194 | 4 |
| epoll-spin | 88 | 8199 | 0 |
| epoll-spin | 89 | 8195 | 0 |
| epoll-spin | 90 | 8196 | 4 |
| epoll-spin | 91 | 8197 | 0 |
| epoll-spin | 92 | 8197 | 0 |
| epoll-spin | 93 | 8194 | 4 |
| epoll-spin | 94 | 8197 | 0 |
| epoll-spin | 95 | 8196 | 4 |
| epoll-spin | 96 | 8200 | 4 |
| epoll-spin | 97 | 8193 | 0 |
| epoll-spin | 98 | 8194 | 4 |
| epoll-spin | 99 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1018282048 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1686828 of 4387098 in 1-msg reads) | 1686828 | 4.19 | 3.21 | 7.01 | 13.11 | 19.86 | 28.22 | 318.94 |

#### Stalls: m3/lab-epoll-spin-c100, loop 0 on CPU 3

201487936 iterations, longest 1984.5 µs; over 5 / 20 / 100 µs: 197370 / 243 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 81 | 4702.8 | 1984.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 159 | 3886.3 | 36.4 |
| softirq TIMER | 1 | 386.1 | 386.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.5 | 32.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1984.5 | 5459 | loop work (5459 items) |  |
| 2 | 774.1 | 2279 | loop work (2279 items) |  |
| 3 | 386.1 | 0 | softirq TIMER | softirq TIMER 359, local timer tick 13, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 4 | 39.2 | 6 | loop work (6 items) |  |
| 5 | 36.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 35.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 35.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 34.7 | 5 | loop work (5 items) |  |
| 9 | 34.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 34.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 8 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-recv-busy-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=121877 clocksource=tsc

### Loop counters (m3/lab-recv-busy-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355509 | 136594 | 2.60 | 119256512 | 335.45 | 1987609 | 1987609 | 99.9 | 66.4 | true | 0 | 223 | 205.2 | 27349 / 1396 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (68268 of 343095 in 1-msg reads) | 68268 | 3.04 | 2.83 | 3.83 | 5.07 | 7.11 | 33.89 | 40.93 |

#### Stalls: m3/lab-recv-busy-c1, loop 0 on CPU 3

119256512 iterations, longest 205.2 µs; over 5 / 20 / 100 µs: 27349 / 1396 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 806 | 24104.0 | 205.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 396 | 10121.0 | 35.1 |
| mostly unexplained (+preempted by kworker/3:1) | 89 | 2592.2 | 53.8 |
| mostly unexplained (+softirq NET_RX) | 74 | 2039.6 | 38.2 |
| softirq NET_RX | 29 | 855.1 | 37.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.9 | 35.9 |
| mostly unexplained (+local timer tick) | 1 | 35.8 | 35.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 205.2 | 589 | loop work (589 items) |  |
| 2 | 57.4 | 10 | loop work (10 items) | preempted by kworker/3:1 10, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 57.2 | 9 | loop work (9 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 53.8 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 51.7 | 6 | loop work (6 items) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 47.5 | 3 | loop work (3 items) | softirq NET_RX 20, preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 44.5 | 3 | loop work (3 items) | softirq NET_RX 18, preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 44.4 | 8 | loop work (8 items) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 44.2 | 3 | loop work (3 items) | softirq NET_RX 19, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 43.8 | 10 | loop work (10 items) | softirq NET_RX 3 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 228 |
| LOC Local timer interrupts | 227 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-recv-busy-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=753001 clocksource=tsc

### Loop counters (m3/lab-recv-busy-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3164103 | 793899 | 3.99 | 116612480 | 36.85 | 1943541 | 194354 | 99.3 | 69.3 | true | 0 | 1046 | 1548.7 | 954370 / 39567 / 14 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 3 |
| recv-busy | 1 | 8198 | 3 |
| recv-busy | 2 | 8199 | 0 |
| recv-busy | 3 | 8197 | 0 |
| recv-busy | 4 | 8200 | 3 |
| recv-busy | 5 | 8193 | 0 |
| recv-busy | 6 | 8199 | 0 |
| recv-busy | 7 | 8200 | 3 |
| recv-busy | 8 | 8195 | 0 |
| recv-busy | 9 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708759072 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (223339 of 3053553 in 1-msg reads) | 223339 | 5.07 | 4.30 | 8.16 | 18.04 | 35.96 | 54.46 | 82.30 |

#### Stalls: m3/lab-recv-busy-c10, loop 0 on CPU 3

11661248 iterations, longest 1548.7 µs; over 5 / 20 / 100 µs: 954370 / 39567 / 14

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 37932 | 1053214.2 | 1548.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1146 | 36227.3 | 64.5 |
| mostly unexplained (+preempted by kworker/3:1) | 369 | 13433.0 | 69.8 |
| mostly unexplained (+softirq NET_RX) | 77 | 2577.7 | 59.6 |
| mostly unexplained (+irq work (IPI/vector)) | 24 | 616.5 | 41.8 |
| mostly unexplained (+local timer tick) | 12 | 307.6 | 48.0 |
| softirq NET_RX | 4 | 155.0 | 56.4 |
| softirq TIMER | 1 | 148.5 | 148.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 40.0 | 40.0 |
| mostly unexplained (+ipi_send_cpu) | 1 | 27.7 | 27.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1548.7 | 5315 | loop work (5315 items) |  |
| 2 | 148.5 | 25 | softirq TIMER | softirq TIMER 107, local timer tick 12, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 3 | 118.6 | 55 | loop work (55 items) | local timer tick 5, preempted by kworker/3:1 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 4 | 115.4 | 40 | loop work (40 items) | softirq NET_RX 4 |
| 5 | 110.0 | 73 | loop work (73 items) | preempted by kworker/3:1 6, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 6 | 108.2 | 20 | loop work (20 items) | preempted by kworker/3:1 6, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 7 | 107.3 | 50 | loop work (50 items) |  |
| 8 | 103.5 | 22 | loop work (22 items) | softirq NET_RX 6, local timer tick 5, preempted by kworker/3:1 4, timer tick_nohz_handler 2 |
| 9 | 103.4 | 64 | loop work (64 items) | preempted by kworker/3:1 5, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 102.4 | 34 | loop work (34 items) | preempted by kworker/3:1 6, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 1114 |
| IWI IRQ work interrupts | 1094 |
| CAL Function call interrupts | 991 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-recv-busy-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1554262 clocksource=tsc

### Loop counters (m3/lab-recv-busy-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4546081 | 1934171 | 2.35 | 108595200 | 23.89 | 1809920 | 18099 | 98.2 | 70.8 | true | 0 | 4134 | 2648.1 | 1085951 / 1085951 / 15130 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 3 |
| recv-busy | 1 | 8197 | 0 |
| recv-busy | 2 | 8195 | 3 |
| recv-busy | 3 | 8199 | 0 |
| recv-busy | 4 | 8196 | 4 |
| recv-busy | 5 | 8200 | 3 |
| recv-busy | 6 | 8199 | 0 |
| recv-busy | 7 | 8195 | 3 |
| recv-busy | 8 | 8199 | 0 |
| recv-busy | 9 | 8197 | 3 |
| recv-busy | 10 | 8195 | 3 |
| recv-busy | 11 | 8199 | 0 |
| recv-busy | 12 | 8196 | 3 |
| recv-busy | 13 | 8195 | 3 |
| recv-busy | 14 | 8196 | 3 |
| recv-busy | 15 | 8199 | 0 |
| recv-busy | 16 | 8198 | 3 |
| recv-busy | 17 | 8197 | 3 |
| recv-busy | 18 | 8198 | 3 |
| recv-busy | 19 | 8200 | 4 |
| recv-busy | 20 | 8197 | 0 |
| recv-busy | 21 | 8198 | 4 |
| recv-busy | 22 | 8197 | 3 |
| recv-busy | 23 | 8195 | 3 |
| recv-busy | 24 | 8193 | 0 |
| recv-busy | 25 | 8195 | 0 |
| recv-busy | 26 | 8196 | 3 |
| recv-busy | 27 | 8197 | 3 |
| recv-busy | 28 | 8200 | 3 |
| recv-busy | 29 | 8194 | 4 |
| recv-busy | 30 | 8195 | 3 |
| recv-busy | 31 | 8194 | 3 |
| recv-busy | 32 | 8193 | 0 |
| recv-busy | 33 | 8200 | 3 |
| recv-busy | 34 | 8196 | 3 |
| recv-busy | 35 | 8195 | 3 |
| recv-busy | 36 | 8200 | 3 |
| recv-busy | 37 | 8195 | 3 |
| recv-busy | 38 | 8194 | 4 |
| recv-busy | 39 | 8200 | 4 |
| recv-busy | 40 | 8193 | 3 |
| recv-busy | 41 | 8197 | 3 |
| recv-busy | 42 | 8200 | 4 |
| recv-busy | 43 | 8196 | 3 |
| recv-busy | 44 | 8199 | 0 |
| recv-busy | 45 | 8197 | 3 |
| recv-busy | 46 | 8197 | 3 |
| recv-busy | 47 | 8196 | 4 |
| recv-busy | 48 | 8198 | 3 |
| recv-busy | 49 | 8194 | 3 |
| recv-busy | 50 | 8194 | 4 |
| recv-busy | 51 | 8193 | 3 |
| recv-busy | 52 | 8194 | 4 |
| recv-busy | 53 | 8200 | 3 |
| recv-busy | 54 | 8198 | 4 |
| recv-busy | 55 | 8200 | 4 |
| recv-busy | 56 | 8198 | 3 |
| recv-busy | 57 | 8195 | 3 |
| recv-busy | 58 | 8200 | 3 |
| recv-busy | 59 | 8196 | 4 |
| recv-busy | 60 | 8194 | 3 |
| recv-busy | 61 | 8199 | 0 |
| recv-busy | 62 | 8194 | 3 |
| recv-busy | 63 | 8193 | 3 |
| recv-busy | 64 | 8195 | 3 |
| recv-busy | 65 | 8200 | 4 |
| recv-busy | 66 | 8199 | 0 |
| recv-busy | 67 | 8198 | 3 |
| recv-busy | 68 | 8200 | 4 |
| recv-busy | 69 | 8194 | 3 |
| recv-busy | 70 | 8198 | 3 |
| recv-busy | 71 | 8193 | 3 |
| recv-busy | 72 | 8198 | 4 |
| recv-busy | 73 | 8194 | 4 |
| recv-busy | 74 | 8196 | 3 |
| recv-busy | 75 | 8193 | 3 |
| recv-busy | 76 | 8195 | 3 |
| recv-busy | 77 | 8193 | 0 |
| recv-busy | 78 | 8198 | 3 |
| recv-busy | 79 | 8198 | 3 |
| recv-busy | 80 | 8197 | 3 |
| recv-busy | 81 | 8193 | 3 |
| recv-busy | 82 | 8199 | 0 |
| recv-busy | 83 | 8195 | 3 |
| recv-busy | 84 | 8198 | 4 |
| recv-busy | 85 | 8197 | 0 |
| recv-busy | 86 | 8196 | 3 |
| recv-busy | 87 | 8194 | 3 |
| recv-busy | 88 | 8199 | 0 |
| recv-busy | 89 | 8193 | 0 |
| recv-busy | 90 | 8194 | 3 |
| recv-busy | 91 | 8196 | 3 |
| recv-busy | 92 | 8193 | 3 |
| recv-busy | 93 | 8195 | 3 |
| recv-busy | 94 | 8194 | 3 |
| recv-busy | 95 | 8196 | 4 |
| recv-busy | 96 | 8198 | 3 |
| recv-busy | 97 | 8197 | 3 |
| recv-busy | 98 | 8198 | 3 |
| recv-busy | 99 | 8193 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1018322144 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1492568 of 4387311 in 1-msg reads) | 1492568 | 27.91 | 27.01 | 50.19 | 71.16 | 90.71 | 115.84 | 171.07 |

#### Stalls: m3/lab-recv-busy-c100, loop 0 on CPU 3

1085952 iterations, longest 2648.1 µs; over 5 / 20 / 100 µs: 1085951 / 1085951 / 15130 (37375 stalls not kept)

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 798099 | 41582495.0 | 136.3 |
| loop work | 225141 | 14783402.8 | 2648.1 |
| mostly unexplained (+softirq NET_RX) | 19876 | 1119900.8 | 139.4 |
| mostly unexplained (+local timer tick) | 3191 | 197153.5 | 100.2 |
| mostly unexplained (+preempted by kworker/3:1) | 1684 | 118080.4 | 131.6 |
| mostly unexplained (+irq work (IPI/vector)) | 555 | 37242.9 | 99.4 |
| mostly unexplained (+softirq NET_TX) | 11 | 722.1 | 92.9 |
| mostly unexplained (+ipi_send_cpu) | 7 | 429.0 | 69.3 |
| mostly unexplained (+call function (IPI/vector)) | 6 | 376.1 | 68.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 148.2 | 53.9 |
| mostly unexplained (+softirq RCU) | 2 | 103.5 | 51.8 |
| mostly unexplained (+softirq TIMER) | 1 | 75.0 | 75.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2648.1 | 7627 | loop work (7627 items) |  |
| 2 | 185.0 | 34 | loop work (34 items) |  |
| 3 | 179.6 | 50 | loop work (50 items) | softirq NET_RX 1 |
| 4 | 179.0 | 83 | loop work (83 items) |  |
| 5 | 177.2 | 40 | loop work (40 items) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, softirq NET_RX 1, workqueue [ena] 1 |
| 6 | 170.8 | 72 | loop work (72 items) |  |
| 7 | 170.8 | 86 | loop work (86 items) |  |
| 8 | 170.6 | 51 | loop work (51 items) |  |
| 9 | 170.2 | 74 | loop work (74 items) | softirq NET_RX 1 |
| 10 | 169.3 | 23 | loop work (23 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4179 |
| LOC Local timer interrupts | 4042 |
| CAL Function call interrupts | 990 |
| TLB TLB shootdowns | 1 |

## m3/lab-recv-busy-pq-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=126557 clocksource=tsc

### Loop counters (m3/lab-recv-busy-pq-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355508 | 138789 | 2.56 | 166938240 | 469.58 | 2782304 | 1391152 | 99.8 | 58.5 | true | 0 | 208 | 182.2 | 27537 / 1366 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79633792 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (68080 of 343094 in 1-msg reads) | 68080 | 3.06 | 2.92 | 3.68 | 4.74 | 6.70 | 23.52 | 36.61 |

#### Stalls: m3/lab-recv-busy-pq-c1, loop 0 on CPU 3

83469120 iterations, longest 182.2 µs; over 5 / 20 / 100 µs: 27537 / 1366 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 806 | 23629.7 | 182.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 407 | 10111.6 | 31.4 |
| mostly unexplained (+preempted by kworker/3:1) | 64 | 1829.8 | 44.3 |
| mostly unexplained (+softirq NET_RX) | 59 | 1563.3 | 33.2 |
| softirq NET_RX | 27 | 753.0 | 43.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.0 | 35.0 |
| mostly unexplained (+local timer tick) | 1 | 34.9 | 34.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 34.3 | 34.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 182.2 | 589 | loop work (589 items) |  |
| 2 | 64.9 | 8 | loop work (8 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 58.4 | 9 | loop work (9 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 53.7 | 7 | loop work (7 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 53.5 | 10 | loop work (10 items) | softirq NET_RX 13, preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 50.4 | 10 | loop work (10 items) | softirq NET_RX 16, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 45.7 | 5 | loop work (5 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 44.3 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 44.0 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 43.2 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 213 |
| LOC Local timer interrupts | 211 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-recv-busy-pq-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=703234 clocksource=tsc

### Loop counters (m3/lab-recv-busy-pq-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3164104 | 819539 | 3.86 | 125446129 | 39.65 | 2090769 | 297971 | 99.2 | 67.5 | true | 0 | 1388 | 1561.0 | 520312 / 24527 / 6 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 3 |
| recv-busy | 1 | 8200 | 3 |
| recv-busy | 2 | 8194 | 4 |
| recv-busy | 3 | 8197 | 0 |
| recv-busy | 4 | 8193 | 0 |
| recv-busy | 5 | 8200 | 3 |
| recv-busy | 6 | 8195 | 3 |
| recv-busy | 7 | 8195 | 3 |
| recv-busy | 8 | 8200 | 4 |
| recv-busy | 9 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708759296 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (240601 of 3053552 in 1-msg reads) | 240601 | 5.18 | 4.34 | 8.71 | 16.60 | 33.85 | 44.78 | 85.74 |

#### Stalls: m3/lab-recv-busy-pq-c10, loop 0 on CPU 3

17878272 iterations, longest 1561.0 µs; over 5 / 20 / 100 µs: 520312 / 24527 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 22980 | 664530.8 | 1561.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1087 | 33479.9 | 50.6 |
| mostly unexplained (+preempted by kworker/3:1) | 392 | 13318.6 | 72.6 |
| mostly unexplained (+softirq NET_RX) | 31 | 1003.2 | 40.7 |
| mostly unexplained (+local timer tick) | 13 | 414.4 | 50.9 |
| mostly unexplained (+irq work (IPI/vector)) | 14 | 353.1 | 33.6 |
| softirq NET_RX | 8 | 315.2 | 48.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 37.2 | 37.2 |
| softirq TIMER | 1 | 27.4 | 27.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1561.0 | 5355 | loop work (5355 items) |  |
| 2 | 126.5 | 64 | loop work (64 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 113.3 | 49 | loop work (49 items) | softirq NET_RX 9 |
| 4 | 113.0 | 40 | loop work (40 items) | softirq NET_RX 7 |
| 5 | 103.6 | 49 | loop work (49 items) |  |
| 6 | 101.4 | 44 | loop work (44 items) |  |
| 7 | 98.3 | 61 | loop work (61 items) |  |
| 8 | 94.5 | 55 | loop work (55 items) |  |
| 9 | 93.9 | 44 | loop work (44 items) |  |
| 10 | 93.6 | 36 | loop work (36 items) | softirq NET_RX 9 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1419 |
| LOC Local timer interrupts | 1383 |
| CAL Function call interrupts | 990 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-recv-busy-pq-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1743001 clocksource=tsc

### Loop counters (m3/lab-recv-busy-pq-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4545915 | 2165651 | 2.10 | 112190045 | 24.68 | 1869834 | 204141 | 97.8 | 69.0 | true | 0 | 4929 | 2232.3 | 1935269 / 43520 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 3 |
| recv-busy | 1 | 8194 | 3 |
| recv-busy | 2 | 8197 | 3 |
| recv-busy | 3 | 8194 | 4 |
| recv-busy | 4 | 8196 | 3 |
| recv-busy | 5 | 8196 | 3 |
| recv-busy | 6 | 8199 | 3 |
| recv-busy | 7 | 8196 | 3 |
| recv-busy | 8 | 8194 | 3 |
| recv-busy | 9 | 8200 | 3 |
| recv-busy | 10 | 8193 | 3 |
| recv-busy | 11 | 8199 | 3 |
| recv-busy | 12 | 8198 | 4 |
| recv-busy | 13 | 8193 | 0 |
| recv-busy | 14 | 8195 | 3 |
| recv-busy | 15 | 8197 | 3 |
| recv-busy | 16 | 8198 | 3 |
| recv-busy | 17 | 8196 | 3 |
| recv-busy | 18 | 8199 | 3 |
| recv-busy | 19 | 8198 | 3 |
| recv-busy | 20 | 8197 | 3 |
| recv-busy | 21 | 8197 | 3 |
| recv-busy | 22 | 8195 | 3 |
| recv-busy | 23 | 8198 | 3 |
| recv-busy | 24 | 8199 | 3 |
| recv-busy | 25 | 8197 | 3 |
| recv-busy | 26 | 8196 | 3 |
| recv-busy | 27 | 8200 | 3 |
| recv-busy | 28 | 8194 | 3 |
| recv-busy | 29 | 8194 | 4 |
| recv-busy | 30 | 8193 | 3 |
| recv-busy | 31 | 8199 | 3 |
| recv-busy | 32 | 8196 | 3 |
| recv-busy | 33 | 8195 | 3 |
| recv-busy | 34 | 8196 | 4 |
| recv-busy | 35 | 8196 | 4 |
| recv-busy | 36 | 8197 | 3 |
| recv-busy | 37 | 8199 | 3 |
| recv-busy | 38 | 8195 | 0 |
| recv-busy | 39 | 8194 | 3 |
| recv-busy | 40 | 8195 | 0 |
| recv-busy | 41 | 8197 | 0 |
| recv-busy | 42 | 8200 | 3 |
| recv-busy | 43 | 8200 | 3 |
| recv-busy | 44 | 8197 | 0 |
| recv-busy | 45 | 8194 | 4 |
| recv-busy | 46 | 8196 | 3 |
| recv-busy | 47 | 8193 | 3 |
| recv-busy | 48 | 8194 | 3 |
| recv-busy | 49 | 8199 | 3 |
| recv-busy | 50 | 8198 | 4 |
| recv-busy | 51 | 8196 | 3 |
| recv-busy | 52 | 8196 | 3 |
| recv-busy | 53 | 8194 | 3 |
| recv-busy | 54 | 8196 | 3 |
| recv-busy | 55 | 8200 | 3 |
| recv-busy | 56 | 8199 | 3 |
| recv-busy | 57 | 8200 | 3 |
| recv-busy | 58 | 8196 | 3 |
| recv-busy | 59 | 8198 | 3 |
| recv-busy | 60 | 8199 | 3 |
| recv-busy | 61 | 8198 | 3 |
| recv-busy | 62 | 8196 | 3 |
| recv-busy | 63 | 8194 | 4 |
| recv-busy | 64 | 8199 | 3 |
| recv-busy | 65 | 8198 | 3 |
| recv-busy | 66 | 8200 | 4 |
| recv-busy | 67 | 8194 | 3 |
| recv-busy | 68 | 8199 | 3 |
| recv-busy | 69 | 8193 | 3 |
| recv-busy | 70 | 8200 | 3 |
| recv-busy | 71 | 8197 | 0 |
| recv-busy | 72 | 8199 | 3 |
| recv-busy | 73 | 8195 | 0 |
| recv-busy | 74 | 8200 | 3 |
| recv-busy | 75 | 8196 | 3 |
| recv-busy | 76 | 8198 | 4 |
| recv-busy | 77 | 8193 | 3 |
| recv-busy | 78 | 8197 | 3 |
| recv-busy | 79 | 8198 | 4 |
| recv-busy | 80 | 8194 | 3 |
| recv-busy | 81 | 8193 | 3 |
| recv-busy | 82 | 8194 | 4 |
| recv-busy | 83 | 8199 | 3 |
| recv-busy | 84 | 8200 | 3 |
| recv-busy | 85 | 8194 | 4 |
| recv-busy | 86 | 8196 | 3 |
| recv-busy | 87 | 8197 | 3 |
| recv-busy | 88 | 8199 | 3 |
| recv-busy | 89 | 8195 | 3 |
| recv-busy | 90 | 8197 | 3 |
| recv-busy | 91 | 8197 | 0 |
| recv-busy | 92 | 8197 | 0 |
| recv-busy | 93 | 8198 | 3 |
| recv-busy | 94 | 8195 | 3 |
| recv-busy | 95 | 8197 | 0 |
| recv-busy | 96 | 8200 | 4 |
| recv-busy | 97 | 8197 | 0 |
| recv-busy | 98 | 8193 | 0 |
| recv-busy | 99 | 8198 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1018284960 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1544292 of 4387098 in 1-msg reads) | 1544292 | 5.91 | 5.42 | 8.72 | 18.98 | 31.73 | 45.34 | 189.24 |

#### Stalls: m3/lab-recv-busy-pq-c100, loop 0 on CPU 3

12248448 iterations, longest 2232.3 µs; over 5 / 20 / 100 µs: 1935269 / 43520 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 37353 | 1036452.3 | 2232.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4698 | 134704.6 | 60.9 |
| mostly unexplained (+preempted by kworker/3:1) | 1218 | 28019.5 | 53.3 |
| mostly unexplained (+softirq NET_RX) | 131 | 3582.3 | 47.0 |
| mostly unexplained (+irq work (IPI/vector)) | 65 | 1683.7 | 51.5 |
| mostly unexplained (+local timer tick) | 44 | 1385.4 | 45.0 |
| softirq TIMER | 1 | 310.4 | 310.4 |
| softirq NET_RX | 7 | 276.3 | 50.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 40.2 | 40.2 |
| mostly unexplained (+ipi_send_cpu) | 1 | 28.8 | 28.8 |
| mostly unexplained (+softirq TIMER) | 1 | 20.4 | 20.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2232.3 | 5994 | loop work (5994 items) |  |
| 2 | 615.8 | 1756 | loop work (1756 items) |  |
| 3 | 310.4 | 5 | softirq TIMER | softirq TIMER 281, local timer tick 6, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 4 | 95.8 | 49 | loop work (49 items) |  |
| 5 | 93.7 | 64 | loop work (64 items) |  |
| 6 | 91.4 | 64 | loop work (64 items) |  |
| 7 | 91.4 | 51 | loop work (51 items) |  |
| 8 | 86.3 | 63 | loop work (63 items) |  |
| 9 | 84.7 | 47 | loop work (47 items) |  |
| 10 | 84.5 | 33 | loop work (33 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4978 |
| LOC Local timer interrupts | 4796 |
| CAL Function call interrupts | 990 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-sqpoll-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=113442 clocksource=tsc

### Loop counters (m3/lab-uring-sqpoll-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355509 | 131098 | 2.71 | 1 | 0.00 | 0 | 29756226 | 0.0 | 0.0 | true | 0 | 0 | 91.6 | 92 / 2 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8195 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79634016 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (65569 of 343094 in 1-msg reads) | 65569 | 4.03 | 3.65 | 5.44 | 7.33 | 17.76 | 24.21 | 41.50 |

#### Stalls: m3/lab-uring-sqpoll-c1, loop 0 on CPU 3

1785373568 iterations, longest 91.6 µs; over 5 / 20 / 100 µs: 92 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 91.6 | 91.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.5 | 34.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 91.6 | 589 | loop work (589 items) |  |
| 2 | 34.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 2 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-sqpoll-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=771474 clocksource=tsc

### Loop counters (m3/lab-uring-sqpoll-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3164102 | 849848 | 3.72 | 1 | 0.00 | 0 | 32174525 | 0.0 | 0.0 | true | 0 | 0 | 741.4 | 2228 / 2 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8193 | 2 |
| uring-sqpoll | 1 | 8200 | 2 |
| uring-sqpoll | 2 | 8199 | 2 |
| uring-sqpoll | 3 | 8200 | 2 |
| uring-sqpoll | 4 | 8196 | 2 |
| uring-sqpoll | 5 | 8194 | 2 |
| uring-sqpoll | 6 | 8197 | 2 |
| uring-sqpoll | 7 | 8193 | 2 |
| uring-sqpoll | 8 | 8195 | 2 |
| uring-sqpoll | 9 | 8198 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708758848 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (253142 of 3053552 in 1-msg reads) | 253142 | 4.77 | 3.93 | 7.44 | 14.02 | 25.60 | 38.93 | 57.84 |

#### Stalls: m3/lab-uring-sqpoll-c10, loop 0 on CPU 3

1930471488 iterations, longest 741.4 µs; over 5 / 20 / 100 µs: 2228 / 2 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 741.4 | 741.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 36.8 | 36.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 741.4 | 5272 | loop work (5272 items) |  |
| 2 | 36.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-sqpoll-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2045823 clocksource=tsc

### Loop counters (m3/lab-uring-sqpoll-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4545902 | 2246388 | 2.02 | 1 | 0.00 | 0 | 7862609 | 0.0 | 0.0 | true | 0 | 0 | 692.3 | 2587 / 3 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8199 | 2 |
| uring-sqpoll | 1 | 8195 | 2 |
| uring-sqpoll | 2 | 8195 | 2 |
| uring-sqpoll | 3 | 8193 | 2 |
| uring-sqpoll | 4 | 8195 | 2 |
| uring-sqpoll | 5 | 8195 | 2 |
| uring-sqpoll | 6 | 8193 | 2 |
| uring-sqpoll | 7 | 8197 | 0 |
| uring-sqpoll | 8 | 8195 | 2 |
| uring-sqpoll | 9 | 8198 | 2 |
| uring-sqpoll | 10 | 8199 | 2 |
| uring-sqpoll | 11 | 8193 | 2 |
| uring-sqpoll | 12 | 8194 | 2 |
| uring-sqpoll | 13 | 8195 | 2 |
| uring-sqpoll | 14 | 8197 | 2 |
| uring-sqpoll | 15 | 8200 | 2 |
| uring-sqpoll | 16 | 8194 | 2 |
| uring-sqpoll | 17 | 8195 | 2 |
| uring-sqpoll | 18 | 8197 | 2 |
| uring-sqpoll | 19 | 8200 | 2 |
| uring-sqpoll | 20 | 8194 | 2 |
| uring-sqpoll | 21 | 8196 | 2 |
| uring-sqpoll | 22 | 8200 | 2 |
| uring-sqpoll | 23 | 8194 | 2 |
| uring-sqpoll | 24 | 8198 | 4 |
| uring-sqpoll | 25 | 8197 | 2 |
| uring-sqpoll | 26 | 8194 | 2 |
| uring-sqpoll | 27 | 8193 | 2 |
| uring-sqpoll | 28 | 8197 | 2 |
| uring-sqpoll | 29 | 8200 | 4 |
| uring-sqpoll | 30 | 8193 | 2 |
| uring-sqpoll | 31 | 8194 | 2 |
| uring-sqpoll | 32 | 8193 | 2 |
| uring-sqpoll | 33 | 8195 | 2 |
| uring-sqpoll | 34 | 8193 | 0 |
| uring-sqpoll | 35 | 8200 | 2 |
| uring-sqpoll | 36 | 8196 | 2 |
| uring-sqpoll | 37 | 8193 | 2 |
| uring-sqpoll | 38 | 8195 | 2 |
| uring-sqpoll | 39 | 8197 | 2 |
| uring-sqpoll | 40 | 8198 | 2 |
| uring-sqpoll | 41 | 8197 | 2 |
| uring-sqpoll | 42 | 8195 | 2 |
| uring-sqpoll | 43 | 8194 | 2 |
| uring-sqpoll | 44 | 8196 | 4 |
| uring-sqpoll | 45 | 8193 | 0 |
| uring-sqpoll | 46 | 8195 | 2 |
| uring-sqpoll | 47 | 8194 | 2 |
| uring-sqpoll | 48 | 8194 | 2 |
| uring-sqpoll | 49 | 8198 | 2 |
| uring-sqpoll | 50 | 8193 | 2 |
| uring-sqpoll | 51 | 8194 | 2 |
| uring-sqpoll | 52 | 8198 | 2 |
| uring-sqpoll | 53 | 8196 | 2 |
| uring-sqpoll | 54 | 8200 | 2 |
| uring-sqpoll | 55 | 8198 | 2 |
| uring-sqpoll | 56 | 8197 | 2 |
| uring-sqpoll | 57 | 8199 | 0 |
| uring-sqpoll | 58 | 8195 | 2 |
| uring-sqpoll | 59 | 8199 | 2 |
| uring-sqpoll | 60 | 8193 | 2 |
| uring-sqpoll | 61 | 8198 | 2 |
| uring-sqpoll | 62 | 8195 | 2 |
| uring-sqpoll | 63 | 8196 | 2 |
| uring-sqpoll | 64 | 8200 | 2 |
| uring-sqpoll | 65 | 8194 | 2 |
| uring-sqpoll | 66 | 8195 | 2 |
| uring-sqpoll | 67 | 8196 | 2 |
| uring-sqpoll | 68 | 8199 | 0 |
| uring-sqpoll | 69 | 8199 | 2 |
| uring-sqpoll | 70 | 8195 | 2 |
| uring-sqpoll | 71 | 8200 | 4 |
| uring-sqpoll | 72 | 8195 | 2 |
| uring-sqpoll | 73 | 8200 | 2 |
| uring-sqpoll | 74 | 8197 | 2 |
| uring-sqpoll | 75 | 8195 | 2 |
| uring-sqpoll | 76 | 8198 | 2 |
| uring-sqpoll | 77 | 8200 | 4 |
| uring-sqpoll | 78 | 8197 | 2 |
| uring-sqpoll | 79 | 8196 | 4 |
| uring-sqpoll | 80 | 8194 | 2 |
| uring-sqpoll | 81 | 8196 | 2 |
| uring-sqpoll | 82 | 8198 | 4 |
| uring-sqpoll | 83 | 8195 | 2 |
| uring-sqpoll | 84 | 8193 | 2 |
| uring-sqpoll | 85 | 8194 | 2 |
| uring-sqpoll | 86 | 8195 | 2 |
| uring-sqpoll | 87 | 8195 | 2 |
| uring-sqpoll | 88 | 8196 | 2 |
| uring-sqpoll | 89 | 8197 | 2 |
| uring-sqpoll | 90 | 8195 | 2 |
| uring-sqpoll | 91 | 8199 | 0 |
| uring-sqpoll | 92 | 8193 | 2 |
| uring-sqpoll | 93 | 8200 | 2 |
| uring-sqpoll | 94 | 8200 | 2 |
| uring-sqpoll | 95 | 8199 | 2 |
| uring-sqpoll | 96 | 8198 | 2 |
| uring-sqpoll | 97 | 8195 | 2 |
| uring-sqpoll | 98 | 8200 | 2 |
| uring-sqpoll | 99 | 8195 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1018282048 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1597865 of 4386956 in 1-msg reads) | 1597865 | 4.15 | 3.56 | 6.04 | 13.62 | 27.07 | 39.62 | 59.00 |

#### Stalls: m3/lab-uring-sqpoll-c100, loop 0 on CPU 3

471756544 iterations, longest 692.3 µs; over 5 / 20 / 100 µs: 2587 / 3 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1120.7 | 692.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.4 | 34.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 692.3 | 4741 | loop work (4741 items) |  |
| 2 | 428.4 | 2955 | loop work (2955 items) |  |
| 3 | 34.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-sqpoll-sib-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=106294 clocksource=tsc

### Loop counters (m3/lab-uring-sqpoll-sib-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355510 | 129722 | 2.74 | 1 | 0.00 | 0 | 39172769 | 0.0 | 0.0 | true | 0 | 0 | 108.3 | 107 / 3 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8199 | 7 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79634240 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (63699 of 343095 in 1-msg reads) | 63699 | 4.76 | 4.28 | 6.28 | 9.42 | 23.45 | 26.62 | 45.83 |

#### Stalls: m3/lab-uring-sqpoll-sib-c1, loop 0 on CPU 3

2350366144 iterations, longest 108.3 µs; over 5 / 20 / 100 µs: 107 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 108.3 | 108.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.2 | 33.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 20.9 | 20.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 108.3 | 590 | loop work (590 items) |  |
| 2 | 33.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 20.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-sqpoll-sib-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=792081 clocksource=tsc

### Loop counters (m3/lab-uring-sqpoll-sib-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3164103 | 840149 | 3.77 | 1 | 0.00 | 0 | 23230029 | 0.0 | 0.0 | true | 0 | 0 | 920.2 | 2594 / 4 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8198 | 7 |
| uring-sqpoll | 1 | 8197 | 7 |
| uring-sqpoll | 2 | 8194 | 7 |
| uring-sqpoll | 3 | 8194 | 7 |
| uring-sqpoll | 4 | 8199 | 7 |
| uring-sqpoll | 5 | 8197 | 7 |
| uring-sqpoll | 6 | 8193 | 7 |
| uring-sqpoll | 7 | 8196 | 7 |
| uring-sqpoll | 8 | 8194 | 7 |
| uring-sqpoll | 9 | 8197 | 7 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708759072 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (244993 of 3053551 in 1-msg reads) | 244993 | 5.92 | 4.88 | 9.21 | 17.34 | 36.96 | 54.63 | 349.01 |

#### Stalls: m3/lab-uring-sqpoll-sib-c10, loop 0 on CPU 3

1393801728 iterations, longest 920.2 µs; over 5 / 20 / 100 µs: 2594 / 4 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 920.2 | 920.2 |
| softirq TIMER | 1 | 159.9 | 159.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.1 | 34.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 20.8 | 20.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 920.2 | 5272 | loop work (5272 items) |  |
| 2 | 159.9 | 0 | softirq TIMER | softirq TIMER 142, local timer tick 10, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 3 | 34.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 20.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1082 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-sqpoll-sib-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1918316 clocksource=tsc

### Loop counters (m3/lab-uring-sqpoll-sib-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4545913 | 2171198 | 2.09 | 1 | 0.00 | 0 | 4535513 | 0.0 | 0.0 | true | 0 | 0 | 840.0 | 5744 / 4 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8198 | 7 |
| uring-sqpoll | 1 | 8198 | 7 |
| uring-sqpoll | 2 | 8196 | 7 |
| uring-sqpoll | 3 | 8198 | 7 |
| uring-sqpoll | 4 | 8196 | 7 |
| uring-sqpoll | 5 | 8200 | 7 |
| uring-sqpoll | 6 | 8194 | 7 |
| uring-sqpoll | 7 | 8198 | 4 |
| uring-sqpoll | 8 | 8200 | 7 |
| uring-sqpoll | 9 | 8194 | 7 |
| uring-sqpoll | 10 | 8198 | 7 |
| uring-sqpoll | 11 | 8195 | 7 |
| uring-sqpoll | 12 | 8197 | 7 |
| uring-sqpoll | 13 | 8200 | 7 |
| uring-sqpoll | 14 | 8194 | 7 |
| uring-sqpoll | 15 | 8193 | 7 |
| uring-sqpoll | 16 | 8195 | 0 |
| uring-sqpoll | 17 | 8196 | 7 |
| uring-sqpoll | 18 | 8196 | 7 |
| uring-sqpoll | 19 | 8195 | 7 |
| uring-sqpoll | 20 | 8200 | 7 |
| uring-sqpoll | 21 | 8194 | 7 |
| uring-sqpoll | 22 | 8193 | 7 |
| uring-sqpoll | 23 | 8194 | 7 |
| uring-sqpoll | 24 | 8196 | 4 |
| uring-sqpoll | 25 | 8200 | 7 |
| uring-sqpoll | 26 | 8199 | 7 |
| uring-sqpoll | 27 | 8200 | 7 |
| uring-sqpoll | 28 | 8196 | 7 |
| uring-sqpoll | 29 | 8197 | 7 |
| uring-sqpoll | 30 | 8195 | 7 |
| uring-sqpoll | 31 | 8196 | 7 |
| uring-sqpoll | 32 | 8199 | 0 |
| uring-sqpoll | 33 | 8199 | 7 |
| uring-sqpoll | 34 | 8194 | 4 |
| uring-sqpoll | 35 | 8197 | 7 |
| uring-sqpoll | 36 | 8200 | 7 |
| uring-sqpoll | 37 | 8197 | 7 |
| uring-sqpoll | 38 | 8195 | 0 |
| uring-sqpoll | 39 | 8199 | 7 |
| uring-sqpoll | 40 | 8200 | 7 |
| uring-sqpoll | 41 | 8200 | 4 |
| uring-sqpoll | 42 | 8195 | 7 |
| uring-sqpoll | 43 | 8193 | 7 |
| uring-sqpoll | 44 | 8199 | 7 |
| uring-sqpoll | 45 | 8199 | 7 |
| uring-sqpoll | 46 | 8194 | 7 |
| uring-sqpoll | 47 | 8196 | 4 |
| uring-sqpoll | 48 | 8197 | 7 |
| uring-sqpoll | 49 | 8198 | 7 |
| uring-sqpoll | 50 | 8196 | 4 |
| uring-sqpoll | 51 | 8194 | 7 |
| uring-sqpoll | 52 | 8196 | 4 |
| uring-sqpoll | 53 | 8197 | 7 |
| uring-sqpoll | 54 | 8200 | 7 |
| uring-sqpoll | 55 | 8195 | 0 |
| uring-sqpoll | 56 | 8200 | 7 |
| uring-sqpoll | 57 | 8195 | 0 |
| uring-sqpoll | 58 | 8199 | 7 |
| uring-sqpoll | 59 | 8200 | 4 |
| uring-sqpoll | 60 | 8195 | 7 |
| uring-sqpoll | 61 | 8197 | 7 |
| uring-sqpoll | 62 | 8200 | 4 |
| uring-sqpoll | 63 | 8200 | 4 |
| uring-sqpoll | 64 | 8198 | 7 |
| uring-sqpoll | 65 | 8197 | 7 |
| uring-sqpoll | 66 | 8194 | 7 |
| uring-sqpoll | 67 | 8199 | 7 |
| uring-sqpoll | 68 | 8198 | 7 |
| uring-sqpoll | 69 | 8193 | 0 |
| uring-sqpoll | 70 | 8195 | 7 |
| uring-sqpoll | 71 | 8198 | 4 |
| uring-sqpoll | 72 | 8195 | 7 |
| uring-sqpoll | 73 | 8196 | 7 |
| uring-sqpoll | 74 | 8195 | 7 |
| uring-sqpoll | 75 | 8194 | 7 |
| uring-sqpoll | 76 | 8197 | 7 |
| uring-sqpoll | 77 | 8199 | 7 |
| uring-sqpoll | 78 | 8193 | 7 |
| uring-sqpoll | 79 | 8197 | 7 |
| uring-sqpoll | 80 | 8200 | 4 |
| uring-sqpoll | 81 | 8197 | 7 |
| uring-sqpoll | 82 | 8194 | 4 |
| uring-sqpoll | 83 | 8193 | 7 |
| uring-sqpoll | 84 | 8196 | 7 |
| uring-sqpoll | 85 | 8198 | 7 |
| uring-sqpoll | 86 | 8195 | 7 |
| uring-sqpoll | 87 | 8196 | 7 |
| uring-sqpoll | 88 | 8197 | 7 |
| uring-sqpoll | 89 | 8194 | 7 |
| uring-sqpoll | 90 | 8200 | 7 |
| uring-sqpoll | 91 | 8194 | 4 |
| uring-sqpoll | 92 | 8195 | 7 |
| uring-sqpoll | 93 | 8196 | 7 |
| uring-sqpoll | 94 | 8199 | 7 |
| uring-sqpoll | 95 | 8193 | 7 |
| uring-sqpoll | 96 | 8197 | 7 |
| uring-sqpoll | 97 | 8197 | 7 |
| uring-sqpoll | 98 | 8195 | 0 |
| uring-sqpoll | 99 | 8194 | 7 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1018284512 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1546705 of 4386965 in 1-msg reads) | 1546705 | 5.78 | 4.76 | 8.74 | 21.14 | 37.44 | 53.06 | 85.21 |

#### Stalls: m3/lab-uring-sqpoll-sib-c100, loop 0 on CPU 3

272130752 iterations, longest 840.0 µs; over 5 / 20 / 100 µs: 5744 / 4 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1370.7 | 840.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 32.3 | 32.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 22.3 | 22.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 840.0 | 4742 | loop work (4742 items) |  |
| 2 | 530.7 | 2958 | loop work (2958 items) |  |
| 3 | 32.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 22.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

