# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 5.22 | 18.28 | 29.77 | 37.52 | 78.64 | 0.0 | 0.0 | 0.0 | 0.0 | 247.50 | 38.0 | 247.50 µs stall: softirq TIMER |
| m2 | 343095 | 6.11 | 23.76 | 26.25 | 30.19 | 42.07 | 0.0 | 0.0 | 0.0 | 0.0 | 177.70 | 6.0 | 177.70 µs stall: loop work (461 items) |
| m3 | 343094 | 4.63 | 25.26 | 29.17 | 34.03 | 37.33 | 0.0 | 0.0 | 0.0 | 0.0 | 160.02 | 9.0 | 160.02 µs stall: loop work (473 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 48.48 | 110.44 | 218.61 | 273.15 | 301.67 | 4731.8 | 0.0 | 0.0 | 0.0 |
| m2 | 343095 | 41.58 | 214.84 | 282.75 | 358.58 | 448.79 | 43086.5 | 0.0 | 0.0 | 0.0 |
| m3 | 343094 | 34.00 | 155.01 | 205.59 | 241.70 | 298.65 | 9608.5 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053557 | 7.84 | 23.67 | 32.22 | 44.47 | 64.02 | 0.0 | 0.0 | 0.0 | 0.0 | 1313.10 | 326.0 | 1313.10 µs stall: loop work (4219 items) |
| m2 | 3053556 | 7.97 | 21.22 | 27.58 | 38.64 | 67.12 | 0.0 | 0.0 | 0.0 | 0.0 | 1361.34 | 172.0 | 1361.34 µs stall: loop work (4301 items) |
| m3 | 3053560 | 7.80 | 20.96 | 33.30 | 45.06 | 67.52 | 0.0 | 0.0 | 0.0 | 0.0 | 1305.44 | 515.0 | 1305.44 µs stall: loop work (4219 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053557 | 68.73 | 125.99 | 143.92 | 163.68 | 197.37 | 290415.1 | 0.0 | 0.0 | 0.0 |
| m2 | 3053556 | 61.17 | 114.58 | 131.19 | 150.98 | 213.85 | 140810.7 | 0.0 | 0.0 | 0.0 |
| m3 | 3053560 | 53.54 | 114.30 | 133.86 | 155.44 | 224.95 | 116605.9 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387094 | 6.63 | 21.71 | 30.70 | 43.84 | 75.30 | 0.0 | 0.0 | 0.0 | 0.0 | 1772.29 | 768.0 | 1772.29 µs stall: loop work (4361 items) |
| m2 | 4387092 | 6.75 | 21.67 | 31.90 | 44.02 | 72.94 | 0.0 | 0.0 | 0.0 | 0.0 | 1824.74 | 968.0 | 1824.74 µs stall: loop work (4334 items) |
| m3 | 4387091 | 6.63 | 21.98 | 32.72 | 44.92 | 96.49 | 0.0 | 0.0 | 0.0 | 0.0 | 1848.77 | 949.0 | 1848.77 µs stall: loop work (4403 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387094 | 51.61 | 125.51 | 148.82 | 199.68 | 309.49 | 260655.3 | 0.0 | 0.0 | 0.0 |
| m2 | 4387092 | 36.30 | 110.57 | 138.18 | 199.22 | 321.57 | 110345.5 | 0.0 | 0.0 | 0.0 |
| m3 | 4387091 | 33.69 | 107.72 | 130.45 | 151.56 | 227.89 | 91125.9 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-mio-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|

### Market-data burst, kernel arrival → decoded (every message), lab-mio-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 47.50 | 93.84 | 187.67 | 284.83 | 320.52 | 2738 | 0 | 0 | 0 |
| m2 | 343094 | 64.66 | 169.93 | 250.17 | 304.28 | 482.39 | 8537 | 0 | 0 | 0 |
| m3 | 343094 | 58.01 | 238.25 | 305.45 | 358.43 | 453.12 | 130191 | 0 | 0 | 0 |

## Market-data burst, kernel arrival → decoded (every message), lab-mio-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|

### Market-data burst, kernel arrival → decoded (every message), lab-mio-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053522 | 76.70 | 135.16 | 154.11 | 173.99 | 228.54 | 452596 | 0 | 0 | 0 |
| m2 | 3053546 | 61.86 | 118.69 | 145.28 | 216.60 | 278.54 | 167820 | 0 | 0 | 0 |
| m3 | 3053555 | 63.25 | 119.05 | 140.95 | 179.99 | 308.79 | 217987 | 0 | 0 | 0 |

## Market-data burst, kernel arrival → decoded (every message), lab-mio-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|

### Market-data burst, kernel arrival → decoded (every message), lab-mio-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387086 | 52.09 | 125.52 | 148.93 | 169.76 | 227.21 | 258670 | 0 | 0 | 0 |
| m2 | 4387086 | 37.44 | 110.94 | 132.32 | 154.74 | 287.97 | 109212 | 0 | 0 | 0 |
| m3 | 4387092 | 37.87 | 112.64 | 140.19 | 200.14 | 343.14 | 124990 | 0 | 0 | 0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343093 | 3.16 | 11.64 | 28.32 | 35.78 | 41.50 | 0.0 | 0.0 | 0.0 | 0.0 | 152.58 | 1401.0 | 152.58 µs stall: loop work (472 items) |
| m2 | 343094 | 2.93 | 10.38 | 28.23 | 34.52 | 37.63 | 0.0 | 0.0 | 0.0 | 0.0 | 180.12 | 1382.0 | 180.12 µs stall: loop work (472 items) |
| m3 | 343095 | 3.07 | 21.27 | 28.76 | 35.60 | 40.42 | 0.0 | 0.0 | 0.0 | 0.0 | 170.39 | 1445.0 | 170.39 µs stall: loop work (472 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343093 | 32.86 | 57.33 | 71.44 | 90.70 | 136.78 | 13.4 | 0.0 | 0.0 | 0.0 |
| m2 | 343094 | 21.69 | 64.53 | 69.99 | 89.67 | 109.39 | 9.3 | 0.0 | 0.0 | 0.0 |
| m3 | 343095 | 17.44 | 42.00 | 51.31 | 79.49 | 85.46 | 0.0 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053559 | 5.72 | 33.61 | 54.22 | 71.05 | 104.45 | 9.3 | 0.0 | 0.0 | 0.0 | 1139.71 | 59700.0 | 1139.71 µs stall: loop work (4167 items) |
| m2 | 3053557 | 5.86 | 33.10 | 52.88 | 69.52 | 106.96 | 16.6 | 0.0 | 0.0 | 0.0 | 1316.20 | 61228.0 | 1316.20 µs stall: loop work (4249 items) |
| m3 | 3053561 | 7.03 | 38.58 | 60.31 | 77.95 | 112.68 | 29.0 | 0.0 | 0.0 | 0.0 | 1265.53 | 73535.0 | 1265.53 µs stall: loop work (4218 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053559 | 54.94 | 106.97 | 128.98 | 147.37 | 197.31 | 66493.5 | 0.0 | 0.0 | 0.0 |
| m2 | 3053557 | 34.62 | 87.62 | 109.51 | 126.87 | 156.01 | 8898.7 | 0.0 | 0.0 | 0.0 |
| m3 | 3053561 | 34.04 | 88.35 | 111.28 | 130.26 | 170.72 | 10255.9 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387093 | 40.64 | 120.21 | 143.37 | 161.70 | 196.30 | 210665.3 | 0.0 | 0.0 | 0.0 | 2419.96 | 951807.0 | 2419.96 µs stall: loop work (6075 items) |
| m2 | 4387259 | 41.49 | 123.58 | 148.22 | 166.26 | 216.05 | 248871.9 | 0.0 | 0.0 | 0.0 | 2357.09 | 953343.0 | 2357.09 µs stall: loop work (6014 items) |
| m3 | 4387126 | 40.90 | 121.03 | 145.61 | 166.53 | 404.42 | 219614.5 | 0.0 | 0.0 | 0.0 | 2326.77 | 955647.0 | 2326.77 µs stall: loop work (6079 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387093 | 85.03 | 177.63 | 200.71 | 219.39 | 339.98 | 1710017.5 | 0.0 | 0.0 | 0.0 |
| m2 | 4387259 | 70.25 | 165.63 | 189.57 | 209.80 | 266.35 | 1295286.2 | 0.0 | 0.0 | 0.0 |
| m3 | 4387126 | 74.72 | 169.31 | 193.47 | 212.25 | 420.44 | 1406347.6 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 3.18 | 12.44 | 30.04 | 35.89 | 47.09 | 0.0 | 0.0 | 0.0 | 0.0 | 179.58 | 1487.0 | 179.58 µs stall: loop work (473 items) |
| m2 | 343084 | 2.96 | 23.64 | 29.38 | 36.26 | 39.52 | 0.0 | 0.0 | 0.0 | 0.0 | 184.82 | 1350.0 | 184.82 µs stall: loop work (462 items) |
| m3 | 343094 | 3.10 | 24.67 | 42.38 | 46.91 | 52.81 | 0.0 | 0.0 | 0.0 | 0.0 | 173.74 | 1910.0 | 173.74 µs stall: loop work (473 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 43.94 | 86.08 | 96.97 | 112.14 | 122.83 | 194.5 | 0.0 | 0.0 | 0.0 |
| m2 | 343084 | 19.98 | 48.18 | 60.98 | 78.30 | 98.37 | 0.0 | 0.0 | 0.0 | 0.0 |
| m3 | 343094 | 15.62 | 48.90 | 66.90 | 86.72 | 152.71 | 4.1 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053560 | 6.95 | 32.91 | 53.28 | 69.26 | 96.23 | 0.0 | 0.0 | 0.0 | 0.0 | 1221.91 | 41167.0 | 1221.91 µs stall: loop work (4218 items) |
| m2 | 3053555 | 5.78 | 31.89 | 51.45 | 67.70 | 98.57 | 0.0 | 0.0 | 0.0 | 0.0 | 1496.18 | 43108.0 | 1496.18 µs stall: loop work (4610 items) |
| m3 | 3053501 | 6.86 | 34.88 | 53.79 | 70.86 | 110.92 | 12.4 | 0.0 | 0.0 | 0.0 | 1362.34 | 55375.0 | 1362.34 µs stall: loop work (4313 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053560 | 52.95 | 106.42 | 128.34 | 145.94 | 179.02 | 61342.3 | 0.0 | 0.0 | 0.0 |
| m2 | 3053555 | 34.48 | 88.22 | 110.38 | 129.39 | 155.93 | 9796.6 | 0.0 | 0.0 | 0.0 |
| m3 | 3053501 | 36.84 | 90.99 | 113.11 | 132.84 | 199.91 | 12722.1 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387135 | 8.64 | 34.56 | 54.02 | 70.06 | 93.43 | 0.0 | 0.0 | 0.0 | 0.0 | 2074.00 | 77125.0 | 2074.00 µs stall: loop work (4804 items) |
| m2 | 4387092 | 9.05 | 36.42 | 55.81 | 71.48 | 89.65 | 0.0 | 0.0 | 0.0 | 0.0 | 1986.58 | 81571.0 | 1986.58 µs stall: loop work (4770 items) |
| m3 | 4387088 | 9.24 | 36.29 | 55.58 | 71.46 | 101.18 | 7.2 | 0.0 | 0.0 | 0.0 | 2076.74 | 83093.0 | 2076.74 µs stall: loop work (4836 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4387135 | 45.02 | 109.97 | 133.02 | 152.70 | 199.63 | 120556.3 | 0.0 | 0.0 | 0.0 |
| m2 | 4387092 | 41.27 | 106.80 | 133.48 | 192.28 | 263.79 | 86435.7 | 0.0 | 0.0 | 0.0 |
| m3 | 4387088 | 36.29 | 102.43 | 126.00 | 146.63 | 202.95 | 56930.8 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-spin-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343095 | 8.16 | 23.52 | 28.37 | 35.40 | 41.94 | 0.0 | 0.0 | 0.0 | 0.0 | 93.69 | 41.0 | 93.69 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| m2 | 343094 | 12.97 | 21.75 | 23.53 | 28.38 | 46.65 | 0.0 | 0.0 | 0.0 | 0.0 | 107.67 | 11.0 | 107.67 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| m3 | 343094 | 6.98 | 22.31 | 26.92 | 32.76 | 52.52 | 0.0 | 0.0 | 0.0 | 0.0 | 277.62 | 14.0 | 277.62 µs stall: softirq TIMER |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-spin-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343095 | 44.62 | 211.48 | 283.03 | 356.31 | 450.25 | 46580.1 | 0.0 | 0.0 | 0.0 |
| m2 | 343094 | 71.22 | 269.96 | 336.99 | 401.89 | 450.43 | 149460.4 | 0.0 | 0.0 | 0.0 |
| m3 | 343094 | 30.91 | 190.63 | 250.98 | 318.34 | 399.95 | 39355.8 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-spin-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053559 | 8.37 | 24.60 | 33.43 | 46.02 | 77.46 | 0.0 | 0.0 | 0.0 | 0.0 | 680.29 | 443.0 | 680.29 µs stall: loop work (4218 items) |
| m2 | 3053555 | 9.15 | 29.16 | 39.80 | 52.34 | 81.90 | 0.0 | 0.0 | 0.0 | 0.0 | 671.12 | 406.0 | 671.12 µs stall: loop work (4219 items) |
| m3 | 3053555 | 8.77 | 24.87 | 32.90 | 42.98 | 71.39 | 0.0 | 0.0 | 0.0 | 0.0 | 671.76 | 231.0 | 671.76 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-spin-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053559 | 71.65 | 127.81 | 147.92 | 165.33 | 195.97 | 355083.9 | 0.0 | 0.0 | 0.0 |
| m2 | 3053555 | 60.77 | 115.86 | 134.08 | 152.37 | 189.34 | 178627.7 | 0.0 | 0.0 | 0.0 |
| m3 | 3053555 | 56.60 | 117.06 | 142.17 | 163.71 | 253.10 | 154436.5 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-spin-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386956 | 7.37 | 26.74 | 37.43 | 50.77 | 88.12 | 0.0 | 0.0 | 0.0 | 0.0 | 1346.58 | 1128.0 | 1346.58 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| m2 | 4386984 | 7.56 | 27.91 | 37.76 | 48.90 | 208.87 | 19.7 | 0.0 | 0.0 | 0.0 | 1316.43 | 1058.0 | 1316.43 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| m3 | 4387058 | 7.32 | 25.72 | 34.68 | 45.76 | 343.30 | 7.2 | 0.0 | 0.0 | 0.0 | 1346.77 | 840.0 | 1346.77 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-spin-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386956 | 54.34 | 128.15 | 151.38 | 175.13 | 228.33 | 315857.6 | 0.0 | 0.0 | 0.0 |
| m2 | 4386984 | 41.38 | 115.79 | 140.19 | 165.71 | 310.52 | 152118.2 | 0.0 | 0.0 | 0.0 |
| m3 | 4387058 | 38.10 | 111.66 | 133.11 | 154.73 | 356.80 | 118391.2 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c1

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 3.96 | 9.23 | 27.31 | 35.34 | 49.72 | 0.0 | 0.0 | 0.0 | 0.0 | 95.67 | 2.0 | 95.67 µs stall: loop work (473 items) |
| m2 | 343094 | 3.95 | 10.57 | 24.54 | 33.83 | 36.50 | 0.0 | 0.0 | 0.0 | 0.0 | 101.87 | 3.0 | 101.87 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| m3 | 343094 | 4.04 | 10.53 | 22.96 | 31.96 | 35.52 | 0.0 | 0.0 | 0.0 | 0.0 | 80.57 | 5.0 | 80.57 µs stall: loop work (473 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c1, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 343094 | 35.14 | 61.76 | 69.47 | 85.19 | 109.93 | 14.5 | 0.0 | 0.0 | 0.0 |
| m2 | 343094 | 21.38 | 44.76 | 55.01 | 75.04 | 83.08 | 0.0 | 0.0 | 0.0 | 0.0 |
| m3 | 343094 | 14.04 | 55.45 | 60.44 | 78.86 | 90.34 | 0.0 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c10

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053457 | 7.20 | 30.27 | 49.02 | 64.53 | 96.23 | 0.0 | 0.0 | 0.0 | 0.0 | 644.26 | 3.0 | 644.26 µs stall: loop work (4196 items) |
| m2 | 3053556 | 6.97 | 27.98 | 48.16 | 62.57 | 92.30 | 0.0 | 0.0 | 0.0 | 0.0 | 710.34 | 6.0 | 710.34 µs stall: loop work (4322 items) |
| m3 | 3053455 | 6.95 | 28.15 | 48.72 | 63.46 | 90.61 | 0.0 | 0.0 | 0.0 | 0.0 | 662.38 | 3.0 | 662.38 µs stall: loop work (4229 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c10, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 3053457 | 46.35 | 99.96 | 121.80 | 142.40 | 231.14 | 31457.5 | 0.0 | 0.0 | 0.0 |
| m2 | 3053556 | 30.34 | 83.40 | 105.72 | 123.30 | 157.46 | 6077.7 | 0.0 | 0.0 | 0.0 |
| m3 | 3053455 | 30.98 | 85.28 | 107.27 | 126.31 | 206.95 | 6955.0 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c100

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386952 | 6.28 | 27.24 | 46.88 | 62.67 | 98.52 | 0.0 | 0.0 | 0.0 | 0.0 | 595.10 | 5.0 | 595.10 µs stall: loop work (3744 items) |
| m2 | 4386938 | 6.34 | 27.59 | 48.07 | 64.88 | 200.60 | 4.1 | 0.0 | 0.0 | 0.0 | 646.85 | 8.0 | 646.85 µs stall: loop work (3814 items) |
| m3 | 4386948 | 6.32 | 28.38 | 48.76 | 63.07 | 102.77 | 7.2 | 0.0 | 0.0 | 0.0 | 603.04 | 3.0 | 603.04 µs stall: loop work (3784 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c100, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 4386952 | 39.63 | 100.67 | 122.82 | 141.50 | 218.90 | 48745.3 | 0.0 | 0.0 | 0.0 |
| m2 | 4386938 | 29.31 | 90.29 | 112.43 | 132.63 | 281.15 | 16801.5 | 0.0 | 0.0 | 0.0 |
| m3 | 4386948 | 29.14 | 90.25 | 113.16 | 132.17 | 238.22 | 17364.3 | 0.0 | 0.0 | 0.0 |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs, and system-management interrupts (SMI, bare metal only: firmware pauses). The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | SMI | receiving CPU | LOC | IWI | RES | CAL | SMI |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lab-epoll-spin-c1 | m1 | 1 | 0 | 0 | 0 | 16 | 0 | 4 | 84 | 0 | 0 | 20 | 0 |
| lab-epoll-spin-c1 | m2 | 1 | 0 | 0 | 0 | 16 | 0 | 4 | 81 | 0 | 0 | 11 | 0 |
| lab-epoll-spin-c1 | m3 | 1 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 0 | 10 | 0 |
| lab-epoll-spin-c10 | m1 | 1 | 0 | 0 | 0 | 16 | 0 | 0 | 1142 | 0 | 0 | 10 | 0 |
| lab-epoll-spin-c10 | m2 | 1 | 0 | 0 | 0 | 16 | 0 | 4 | 152 | 0 | 0 | 4 | 0 |
| lab-epoll-spin-c10 | m3 | 1 | 0 | 0 | 0 | 16 | 0 | 4 | 193 | 0 | 0 | 2 | 0 |
| lab-epoll-spin-c100 | m1 | 1 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 0 | 4 | 0 |
| lab-epoll-spin-c100 | m2 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1142 | 0 | 1 | 12 | 0 |
| lab-epoll-spin-c100 | m3 | 1 | 0 | 0 | 0 | 16 | 0 | 4 | 349 | 0 | 1 | 22 | 0 |
| lab-mio-c1 | m1 | 1 | 0 | 0 | 0 | 16 | 0 |  |  |  |  |  |  |
| lab-mio-c1 | m2 | 1 | 0 | 0 | 0 | 16 | 0 |  |  |  |  |  |  |
| lab-mio-c1 | m3 | 1 | 0 | 0 | 0 | 16 | 0 |  |  |  |  |  |  |
| lab-mio-c10 | m1 | 1 | 0 | 1 | 0 | 16 | 0 |  |  |  |  |  |  |
| lab-mio-c10 | m2 | 1 | 0 | 1 | 0 | 16 | 0 |  |  |  |  |  |  |
| lab-mio-c10 | m3 | 1 | 0 | 1 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-mio-c100 | m1 | 1 | 0 | 5 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-mio-c100 | m2 | 1 | 0 | 5 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-mio-c100 | m3 | 1 | 0 | 5 | 0 | 16 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c1 | m1 | 1 | 5 | 5 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c1 | m2 | 1 | 4 | 4 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c1 | m3 | 1 | 5 | 5 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c10 | m1 | 1 | 19 | 19 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c10 | m2 | 1 | 21 | 21 | 0 | 17 | 0 | 0 | 1142 | 0 | 0 | 2 | 0 |
| lab-recv-busy-c10 | m3 | 1 | 27 | 24 | 0 | 17 | 0 | 4 | 136 | 0 | 0 | 4 | 0 |
| lab-recv-busy-c100 | m1 | 1 | 75 | 78 | 0 | 16 | 0 | 0 | 1143 | 0 | 1 | 11 | 0 |
| lab-recv-busy-c100 | m2 | 1 | 79 | 81 | 0 | 17 | 0 | 4 | 294 | 0 | 1 | 13 | 0 |
| lab-recv-busy-c100 | m3 | 1 | 84 | 87 | 0 | 16 | 0 | 0 | 1144 | 0 | 0 | 20 | 0 |
| lab-recv-busy-pq-c1 | m1 | 1 | 5 | 5 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-pq-c1 | m2 | 1 | 3 | 3 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-pq-c1 | m3 | 1 | 6 | 6 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-pq-c10 | m1 | 1 | 23 | 23 | 0 | 17 | 0 | 4 | 138 | 0 | 0 | 11 | 0 |
| lab-recv-busy-pq-c10 | m2 | 1 | 23 | 23 | 0 | 16 | 0 | 0 | 1142 | 0 | 0 | 18 | 0 |
| lab-recv-busy-pq-c10 | m3 | 1 | 24 | 24 | 0 | 17 | 0 | 4 | 152 | 0 | 0 | 3 | 0 |
| lab-recv-busy-pq-c100 | m1 | 1 | 95 | 99 | 0 | 16 | 0 | 0 | 1143 | 0 | 1 | 19 | 0 |
| lab-recv-busy-pq-c100 | m2 | 1 | 97 | 99 | 0 | 16 | 0 | 0 | 1143 | 0 | 0 | 12 | 0 |
| lab-recv-busy-pq-c100 | m3 | 1 | 97 | 101 | 0 | 17 | 0 | 0 | 1143 | 0 | 1 | 3 | 0 |
| lab-uring-spin-c1 | m1 | 1 | 0 | 0 | 0 | 17 | 0 | 4 | 58 | 0 | 0 | 3 | 0 |
| lab-uring-spin-c1 | m2 | 1 | 0 | 0 | 0 | 16 | 0 | 4 | 66 | 0 | 0 | 2 | 0 |
| lab-uring-spin-c1 | m3 | 1 | 0 | 0 | 0 | 17 | 0 | 4 | 77 | 0 | 0 | 11 | 0 |
| lab-uring-spin-c10 | m1 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 0 | 0 | 2 | 0 |
| lab-uring-spin-c10 | m2 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1142 | 0 | 0 | 10 | 0 |
| lab-uring-spin-c10 | m3 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1142 | 0 | 0 | 2 | 0 |
| lab-uring-spin-c100 | m1 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 0 | 1 | 12 | 0 |
| lab-uring-spin-c100 | m2 | 1 | 0 | 0 | 0 | 17 | 0 | 4 | 301 | 0 | 1 | 21 | 0 |
| lab-uring-spin-c100 | m3 | 1 | 0 | 0 | 0 | 17 | 0 | 4 | 375 | 0 | 1 | 13 | 0 |
| lab-uring-sqpoll-c1 | m1 | 1 | 0 | 0 | 0 | 17 | 0 | 2 | 20 | 6 | 2 | 17 | 0 |
| lab-uring-sqpoll-c1 | m2 | 1 | 0 | 0 | 0 | 17 | 0 | 2 | 17 | 6 | 1 | 17 | 0 |
| lab-uring-sqpoll-c1 | m3 | 1 | 0 | 0 | 0 | 17 | 0 | 2 | 21 | 7 | 1 | 17 | 0 |
| lab-uring-sqpoll-c10 | m1 | 1 | 0 | 0 | 0 | 17 | 0 | 2 | 47 | 22 | 1 | 17 | 0 |
| lab-uring-sqpoll-c10 | m2 | 1 | 0 | 0 | 0 | 17 | 0 | 0 | 1144 | 0 | 0 | 9 | 0 |
| lab-uring-sqpoll-c10 | m3 | 1 | 0 | 0 | 0 | 17 | 0 | 2 | 57 | 26 | 1 | 17 | 0 |
| lab-uring-sqpoll-c100 | m1 | 1 | 0 | 0 | 0 | 17 | 0 | 2 | 148 | 76 | 0 | 20 | 0 |
| lab-uring-sqpoll-c100 | m2 | 1 | 0 | 0 | 0 | 17 | 0 | 2 | 160 | 82 | 0 | 20 | 0 |
| lab-uring-sqpoll-c100 | m3 | 1 | 0 | 0 | 0 | 17 | 0 | 2 | 169 | 88 | 0 | 20 | 0 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### lab-epoll-spin-c1

| condition | worst events |
|---|---|
| m1 | 78.64 at 5.2 s: client: softirq TIMER; 50.85 at 26.8 s: neither loop stalled long enough: network, hypervisor or untraced; 50.70 at 26.8 s: neither loop stalled long enough: network, hypervisor or untraced; 50.55 at 26.8 s: neither loop stalled long enough: network, hypervisor or untraced; 50.40 at 26.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 42.07 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced; 41.92 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced; 41.78 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced; 41.64 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced; 41.50 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 37.33 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced; 37.16 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced; 37.00 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced; 36.84 at 31.0 s: neither loop stalled long enough: network, hypervisor or untraced; 36.82 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-epoll-spin-c10

| condition | worst events |
|---|---|
| m1 | 64.02 at 52.3 s: neither loop stalled long enough: network, hypervisor or untraced; 63.87 at 52.3 s: neither loop stalled long enough: network, hypervisor or untraced; 63.70 at 52.3 s: neither loop stalled long enough: network, hypervisor or untraced; 63.53 at 52.3 s: neither loop stalled long enough: network, hypervisor or untraced; 63.35 at 52.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 67.12 at 38.5 s: client: loop work (14 items); 66.92 at 38.5 s: client: loop work (14 items); 66.74 at 38.5 s: client: loop work (14 items); 63.60 at 38.5 s: client: loop work (14 items); 63.49 at 32.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| m3 | 67.52 at 16.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 67.35 at 16.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 67.06 at 50.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 66.91 at 50.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 66.69 at 50.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |

### lab-epoll-spin-c100

| condition | worst events |
|---|---|
| m1 | 75.30 at 18.3 s: client: loop work (20 items); 75.16 at 18.3 s: client: loop work (20 items); 75.01 at 18.3 s: client: loop work (20 items); 74.87 at 18.3 s: client: loop work (20 items); 74.72 at 18.3 s: client: loop work (20 items) |
| m2 | 72.94 at 13.2 s: neither loop stalled long enough: network, hypervisor or untraced; 71.81 at 13.2 s: neither loop stalled long enough: network, hypervisor or untraced; 71.67 at 13.2 s: neither loop stalled long enough: network, hypervisor or untraced; 71.52 at 13.2 s: neither loop stalled long enough: network, hypervisor or untraced; 71.36 at 13.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 96.49 at 13.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 83.38 at 25.9 s: neither loop stalled long enough: network, hypervisor or untraced; 71.55 at 59.1 s: neither loop stalled long enough: network, hypervisor or untraced; 71.40 at 59.1 s: neither loop stalled long enough: network, hypervisor or untraced; 71.26 at 59.1 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-mio-c1

| condition | worst events |
|---|---|
| m1 |  |
| m2 |  |
| m3 |  |

### lab-mio-c10

| condition | worst events |
|---|---|
| m1 |  |
| m2 |  |
| m3 |  |

### lab-mio-c100

| condition | worst events |
|---|---|
| m1 |  |
| m2 |  |
| m3 |  |

### lab-recv-busy-c1

| condition | worst events |
|---|---|
| m1 | 41.50 at 3.6 s: client: mostly unexplained (+preempted by kworker/1:1); 40.66 at 41.5 s: client: mostly unexplained (+softirq NET_RX); 38.69 at 55.2 s: client: mostly unexplained (+preempted by kworker/1:1); 38.43 at 56.5 s: client: mostly unexplained (+preempted by kworker/1:1); 37.77 at 52.9 s: client: mostly unexplained (+preempted by kworker/1:1) |
| m2 | 37.63 at 33.8 s: client: loop work (3 items); 37.54 at 27.7 s: client: mostly unexplained (+local timer tick); 37.48 at 33.8 s: client: loop work (3 items); 37.45 at 13.3 s: client: loop work (7 items); 37.31 at 13.3 s: client: loop work (7 items) |
| m3 | 40.42 at 50.8 s: client: mostly unexplained (+preempted by kworker/1:1); 40.28 at 44.4 s: client: mostly unexplained (+preempted by kworker/1:1); 39.91 at 54.7 s: client: loop work (10 items); 39.73 at 54.7 s: client: loop work (10 items); 39.56 at 54.7 s: client: loop work (10 items) |

### lab-recv-busy-c10

| condition | worst events |
|---|---|
| m1 | 104.45 at 30.1 s: client: loop work (69 items); 104.29 at 30.1 s: client: loop work (69 items); 104.13 at 30.1 s: client: loop work (69 items); 103.97 at 30.1 s: client: loop work (69 items); 103.81 at 30.1 s: client: loop work (69 items) |
| m2 | 106.96 at 21.3 s: client: loop work (26 items); 106.82 at 21.3 s: client: loop work (26 items); 106.68 at 21.3 s: client: loop work (26 items); 106.53 at 21.3 s: client: loop work (26 items); 106.39 at 21.3 s: client: loop work (26 items) |
| m3 | 112.68 at 51.3 s: client: loop work (64 items); 112.53 at 51.3 s: client: loop work (64 items); 112.38 at 51.3 s: client: loop work (64 items); 112.24 at 51.3 s: client: loop work (64 items); 112.09 at 51.3 s: client: loop work (64 items) |

### lab-recv-busy-c100

| condition | worst events |
|---|---|
| m1 | 196.30 at 40.6 s: client: loop work (62 items); 196.15 at 40.6 s: client: loop work (62 items); 196.00 at 40.6 s: client: loop work (62 items); 195.86 at 40.6 s: client: loop work (62 items); 195.71 at 40.6 s: client: loop work (62 items) |
| m2 | 216.05 at 5.4 s: client: loop work (27 items); 215.88 at 5.4 s: client: loop work (27 items); 215.71 at 5.4 s: client: loop work (27 items); 215.54 at 5.4 s: client: loop work (27 items); 215.37 at 5.4 s: client: loop work (27 items) |
| m3 | 404.42 at 54.9 s: client: softirq TIMER; 399.37 at 54.9 s: client: softirq TIMER; 337.66 at 54.9 s: client: softirq TIMER; 257.03 at 54.9 s: client: softirq TIMER; 203.86 at 54.9 s: client: softirq TIMER |

### lab-recv-busy-pq-c1

| condition | worst events |
|---|---|
| m1 | 47.09 at 42.6 s: client: mostly unexplained (+preempted by kworker/1:1); 45.47 at 55.6 s: client: mostly unexplained (+local timer tick); 41.54 at 4.1 s: client: mostly unexplained (+preempted by kworker/1:1); 41.51 at 4.0 s: client: loop work (7 items); 41.36 at 4.0 s: client: loop work (7 items) |
| m2 | 39.52 at 34.4 s: client: loop work (3 items); 39.36 at 34.4 s: client: loop work (3 items); 39.20 at 34.4 s: client: loop work (3 items); 37.90 at 2.4 s: client: loop work (7 items); 37.82 at 36.3 s: client: loop work (3 items) |
| m3 | 52.81 at 34.3 s: client: loop work (3 items); 52.61 at 34.3 s: client: loop work (3 items); 52.25 at 34.3 s: client: loop work (3 items); 50.40 at 58.3 s: client: loop work (7 items); 50.22 at 58.3 s: client: loop work (7 items) |

### lab-recv-busy-pq-c10

| condition | worst events |
|---|---|
| m1 | 96.23 at 50.6 s: client: loop work (57 items); 96.08 at 50.6 s: client: loop work (57 items); 95.93 at 50.6 s: client: loop work (57 items); 95.79 at 50.6 s: client: loop work (57 items); 95.63 at 50.6 s: client: loop work (57 items) |
| m2 | 98.57 at 12.2 s: client: loop work (66 items); 98.42 at 12.2 s: client: loop work (66 items); 98.28 at 12.2 s: client: loop work (66 items); 98.13 at 12.2 s: client: loop work (66 items); 97.98 at 12.2 s: client: loop work (66 items) |
| m3 | 110.92 at 10.4 s: client: loop work (20 items); 110.77 at 10.4 s: client: loop work (20 items); 110.62 at 10.4 s: client: loop work (20 items); 110.47 at 10.4 s: client: loop work (20 items); 110.33 at 10.4 s: client: loop work (20 items) |

### lab-recv-busy-pq-c100

| condition | worst events |
|---|---|
| m1 | 93.43 at 54.7 s: client: loop work (56 items); 93.28 at 54.7 s: client: loop work (56 items); 93.10 at 54.7 s: client: loop work (56 items); 92.95 at 54.7 s: client: loop work (56 items); 92.80 at 54.7 s: client: loop work (56 items) |
| m2 | 89.65 at 18.2 s: client: loop work (80 items); 89.50 at 18.2 s: client: loop work (80 items); 89.36 at 6.4 s: client: loop work (50 items); 89.33 at 18.2 s: client: loop work (80 items); 89.21 at 6.4 s: client: loop work (50 items) |
| m3 | 101.18 at 46.2 s: client: loop work (65 items); 101.02 at 46.2 s: client: loop work (65 items); 100.85 at 46.2 s: client: loop work (65 items); 100.67 at 46.2 s: client: loop work (65 items); 100.53 at 46.2 s: client: loop work (65 items) |

### lab-uring-spin-c1

| condition | worst events |
|---|---|
| m1 | 41.94 at 10.4 s: neither loop stalled long enough: network, hypervisor or untraced; 40.89 at 10.4 s: neither loop stalled long enough: network, hypervisor or untraced; 40.74 at 10.4 s: neither loop stalled long enough: network, hypervisor or untraced; 40.59 at 10.4 s: neither loop stalled long enough: network, hypervisor or untraced; 40.44 at 10.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 46.65 at 6.9 s: neither loop stalled long enough: network, hypervisor or untraced; 37.33 at 17.1 s: neither loop stalled long enough: network, hypervisor or untraced; 37.18 at 17.1 s: neither loop stalled long enough: network, hypervisor or untraced; 37.04 at 17.1 s: neither loop stalled long enough: network, hypervisor or untraced; 36.90 at 17.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 52.52 at 43.3 s: client: softirq TIMER; 46.65 at 30.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 46.46 at 30.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 46.28 at 30.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 46.09 at 30.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |

### lab-uring-spin-c10

| condition | worst events |
|---|---|
| m1 | 77.46 at 9.1 s: client: loop work (7 items); 77.29 at 9.1 s: client: loop work (7 items); 77.11 at 9.1 s: client: loop work (7 items); 76.94 at 9.1 s: client: loop work (7 items); 76.77 at 9.1 s: client: loop work (7 items) |
| m2 | 81.90 at 3.9 s: client: loop work (14 items); 81.76 at 3.9 s: client: loop work (14 items); 81.62 at 3.9 s: client: loop work (14 items); 81.45 at 3.9 s: client: loop work (14 items); 81.31 at 3.9 s: client: loop work (14 items) |
| m3 | 71.39 at 22.4 s: neither loop stalled long enough: network, hypervisor or untraced; 71.25 at 22.4 s: neither loop stalled long enough: network, hypervisor or untraced; 70.14 at 22.4 s: neither loop stalled long enough: network, hypervisor or untraced; 62.84 at 34.9 s: neither loop stalled long enough: network, hypervisor or untraced; 62.68 at 34.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-spin-c100

| condition | worst events |
|---|---|
| m1 | 88.12 at 14.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 87.95 at 14.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 73.61 at 50.9 s: client: loop work (26 items); 73.44 at 50.9 s: client: loop work (26 items); 71.45 at 14.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| m2 | 208.87 at 21.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 208.69 at 21.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 204.39 at 21.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 204.18 at 21.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 204.00 at 21.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| m3 | 343.30 at 58.9 s: client: softirq TIMER; 321.50 at 58.9 s: client: softirq TIMER; 294.06 at 58.9 s: client: softirq TIMER; 274.02 at 58.9 s: client: softirq TIMER; 261.02 at 58.9 s: client: softirq TIMER |

### lab-uring-sqpoll-c1

| condition | worst events |
|---|---|
| m1 | 49.72 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced; 49.57 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced; 49.42 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced; 49.27 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced; 49.09 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 36.50 at 22.7 s: neither loop stalled long enough: network, hypervisor or untraced; 36.30 at 6.3 s: neither loop stalled long enough: network, hypervisor or untraced; 36.13 at 6.3 s: neither loop stalled long enough: network, hypervisor or untraced; 35.88 at 6.3 s: neither loop stalled long enough: network, hypervisor or untraced; 35.71 at 6.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 35.52 at 36.3 s: neither loop stalled long enough: network, hypervisor or untraced; 35.36 at 36.3 s: neither loop stalled long enough: network, hypervisor or untraced; 35.20 at 36.3 s: neither loop stalled long enough: network, hypervisor or untraced; 35.04 at 36.3 s: neither loop stalled long enough: network, hypervisor or untraced; 34.88 at 36.3 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-c10

| condition | worst events |
|---|---|
| m1 | 96.23 at 30.4 s: neither loop stalled long enough: network, hypervisor or untraced; 96.08 at 30.4 s: neither loop stalled long enough: network, hypervisor or untraced; 95.93 at 30.4 s: neither loop stalled long enough: network, hypervisor or untraced; 95.78 at 30.4 s: neither loop stalled long enough: network, hypervisor or untraced; 95.63 at 30.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 92.30 at 17.7 s: neither loop stalled long enough: network, hypervisor or untraced; 92.15 at 17.7 s: neither loop stalled long enough: network, hypervisor or untraced; 92.00 at 17.7 s: neither loop stalled long enough: network, hypervisor or untraced; 91.85 at 17.7 s: neither loop stalled long enough: network, hypervisor or untraced; 91.70 at 17.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 90.61 at 38.6 s: neither loop stalled long enough: network, hypervisor or untraced; 90.43 at 38.6 s: neither loop stalled long enough: network, hypervisor or untraced; 90.18 at 38.6 s: neither loop stalled long enough: network, hypervisor or untraced; 90.00 at 38.6 s: neither loop stalled long enough: network, hypervisor or untraced; 89.83 at 38.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-c100

| condition | worst events |
|---|---|
| m1 | 98.52 at 25.2 s: neither loop stalled long enough: network, hypervisor or untraced; 98.35 at 25.2 s: neither loop stalled long enough: network, hypervisor or untraced; 98.14 at 25.2 s: neither loop stalled long enough: network, hypervisor or untraced; 97.99 at 25.2 s: neither loop stalled long enough: network, hypervisor or untraced; 97.84 at 25.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 200.60 at 33.3 s: client: softirq TIMER; 195.46 at 33.3 s: client: softirq TIMER; 108.35 at 33.3 s: client: softirq TIMER; 107.86 at 33.3 s: client: softirq TIMER; 99.31 at 27.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| m3 | 102.77 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced; 102.62 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced; 102.47 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced; 102.32 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced; 102.17 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| m1 | unexplained: no kernel event (hypervisor or in-kernel time) 45021072.79; loop work 17926704.47; mostly unexplained (+softirq NET_RX) 1534491.13; mostly unexplained (+preempted by kworker/1:1) 331277.64; mostly unexplained (+local timer tick) 296924.95 |
| m2 | unexplained: no kernel event (hypervisor or in-kernel time) 44924664.81; loop work 18270503.36; mostly unexplained (+softirq NET_RX) 1508314.55; mostly unexplained (+preempted by kworker/1:1) 350665.42; mostly unexplained (+local timer tick) 312630.93 |
| m3 | unexplained: no kernel event (hypervisor or in-kernel time) 44816544.01; loop work 19040941.95; mostly unexplained (+softirq NET_RX) 1544025.11; mostly unexplained (+preempted by kworker/1:1) 387234.96; mostly unexplained (+local timer tick) 341095.76 |

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
| lab-epoll-spin-c1 | idle spare core | 1 | :43 | 1 (100%) | 1 of 3 | 358.17 | no |
| lab-epoll-spin-c1 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c1 | slow samples | 0 | | | | | |
| lab-epoll-spin-c10 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c10 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c10 | slow samples | 0 | | | | | |
| lab-epoll-spin-c100 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c100 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c100 | slow samples | 0 | | | | | |
| lab-mio-c1 | idle spare core | 0 | | | | | |
| lab-mio-c1 | receiver loop | 0 | | | | | |
| lab-mio-c1 | slow samples | 0 | | | | | |
| lab-mio-c10 | idle spare core | 0 | | | | | |
| lab-mio-c10 | receiver loop | 0 | | | | | |
| lab-mio-c10 | slow samples | 0 | | | | | |
| lab-mio-c100 | idle spare core | 0 | | | | | |
| lab-mio-c100 | receiver loop | 0 | | | | | |
| lab-mio-c100 | slow samples | 0 | | | | | |
| lab-recv-busy-c1 | idle spare core | 0 | | | | | |
| lab-recv-busy-c1 | receiver loop | 0 | | | | | |
| lab-recv-busy-c1 | slow samples | 0 | | | | | |
| lab-recv-busy-c10 | idle spare core | 1 | :43 | 1 (100%) | 1 of 3 | 310.99 | no |
| lab-recv-busy-c10 | receiver loop | 0 | | | | | |
| lab-recv-busy-c10 | slow samples | 0 | | | | | |
| lab-recv-busy-c100 | idle spare core | 0 | | | | | |
| lab-recv-busy-c100 | receiver loop | 1 | :38 | 1 (100%) | 1 of 3 | 388.11 | no |
| lab-recv-busy-c100 | slow samples | 3 | :38 | 3 (100%) | 1 of 3 | 404.42 | no |
| lab-recv-busy-pq-c1 | idle spare core | 0 | | | | | |
| lab-recv-busy-pq-c1 | receiver loop | 0 | | | | | |
| lab-recv-busy-pq-c1 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c10 | idle spare core | 1 | :11 | 1 (100%) | 1 of 3 | 379.81 | no |
| lab-recv-busy-pq-c10 | receiver loop | 0 | | | | | |
| lab-recv-busy-pq-c10 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c100 | idle spare core | 0 | | | | | |
| lab-recv-busy-pq-c100 | receiver loop | 0 | | | | | |
| lab-recv-busy-pq-c100 | slow samples | 0 | | | | | |
| lab-uring-spin-c1 | idle spare core | 1 | :15 | 1 (100%) | 1 of 3 | 364.07 | no |
| lab-uring-spin-c1 | receiver loop | 0 | | | | | |
| lab-uring-spin-c1 | slow samples | 0 | | | | | |
| lab-uring-spin-c10 | idle spare core | 0 | | | | | |
| lab-uring-spin-c10 | receiver loop | 0 | | | | | |
| lab-uring-spin-c10 | slow samples | 0 | | | | | |
| lab-uring-spin-c100 | idle spare core | 0 | | | | | |
| lab-uring-spin-c100 | receiver loop | 1 | :06 | 1 (100%) | 1 of 3 | 366.86 | no |
| lab-uring-spin-c100 | slow samples | 2 | :06 | 2 (100%) | 1 of 3 | 343.30 | no |
| lab-uring-sqpoll-c1 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-c1 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-c1 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c10 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-c10 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-c10 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c100 | idle spare core | 1 | :20 | 1 (100%) | 1 of 3 | 300.90 | no |
| lab-uring-sqpoll-c100 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-c100 | slow samples | 0 | | | | | |

# Every run in detail

## m1/lab-epoll-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 355402 | 141763 | 2.51 | 233581059 | 657.23 | 3893018 | 3890655 | 0.0 | 33.6 | false | 0 | 0 | 247.5 | 30856 / 38 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 79610048 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (84052 of 343094 in 1-msg reads) | 84052 | 5.89 | 5.46 | 8.50 | 13.00 | 24.83 | 29.17 | 78.64 |

#### Stalls: m1/lab-epoll-spin-c1, loop 0 on CPU 1

233439296 iterations, longest 247.5 µs; over 5 / 20 / 100 µs: 30856 / 38 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 31 | 951.8 | 169.0 |
| softirq TIMER | 1 | 247.5 | 247.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 5 | 120.3 | 32.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.4 | 35.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 247.5 | 1 | softirq TIMER | softirq TIMER 219, local timer tick 19, timer tick_nohz_handler 9, irq work (IPI/vector) 2 |
| 2 | 169.0 | 483 | loop work (483 items) |  |
| 3 | 35.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 34.8 | 7 | loop work (7 items) |  |
| 5 | 33.9 | 7 | loop work (7 items) |  |
| 6 | 32.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 31.9 | 7 | loop work (7 items) |  |
| 8 | 31.7 | 7 | loop work (7 items) |  |
| 9 | 30.8 | 7 | loop work (7 items) |  |
| 10 | 30.7 | 7 | loop work (7 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 9 |
| IWI IRQ work interrupts | 6 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-epoll-spin-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3163049 | 908878 | 3.48 | 226542094 | 71.62 | 3775702 | 3760554 | 0.0 | 35.2 | false | 0 | 0 | 1313.1 | 185522 / 326 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8195 | 0 |
| epoll-spin | 1 | 8193 | 0 |
| epoll-spin | 2 | 8196 | 4 |
| epoll-spin | 3 | 8198 | 4 |
| epoll-spin | 4 | 8200 | 4 |
| epoll-spin | 5 | 8199 | 0 |
| epoll-spin | 6 | 8195 | 0 |
| epoll-spin | 7 | 8200 | 4 |
| epoll-spin | 8 | 8195 | 0 |
| epoll-spin | 9 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 708522976 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (340080 of 3053557 in 1-msg reads) | 340080 | 6.14 | 5.51 | 9.87 | 17.83 | 27.29 | 38.41 | 59.29 |

#### Stalls: m1/lab-epoll-spin-c10, loop 0 on CPU 1

225633216 iterations, longest 1313.1 µs; over 5 / 20 / 100 µs: 185522 / 326 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 189 | 6317.2 | 1313.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 135 | 3593.3 | 49.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.2 | 31.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1313.1 | 4219 | loop work (4219 items) |  |
| 2 | 49.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 47.7 | 10 | loop work (10 items) |  |
| 4 | 47.2 | 4 | loop work (4 items) |  |
| 5 | 43.8 | 102 | loop work (102 items) |  |
| 6 | 42.6 | 7 | loop work (7 items) |  |
| 7 | 40.7 | 5 | loop work (5 items) |  |
| 8 | 39.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 37.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-epoll-spin-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 4544262 | 2275595 | 2.00 | 183457419 | 40.37 | 3057624 | 3019697 | 0.0 | 33.7 | false | 0 | 0 | 1772.3 | 271410 / 768 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8197 | 0 |
| epoll-spin | 1 | 8195 | 0 |
| epoll-spin | 2 | 8199 | 0 |
| epoll-spin | 3 | 8200 | 4 |
| epoll-spin | 4 | 8197 | 0 |
| epoll-spin | 5 | 8199 | 0 |
| epoll-spin | 6 | 8193 | 0 |
| epoll-spin | 7 | 8197 | 0 |
| epoll-spin | 8 | 8193 | 0 |
| epoll-spin | 9 | 8199 | 0 |
| epoll-spin | 10 | 8196 | 4 |
| epoll-spin | 11 | 8199 | 0 |
| epoll-spin | 12 | 8195 | 0 |
| epoll-spin | 13 | 8194 | 4 |
| epoll-spin | 14 | 8195 | 0 |
| epoll-spin | 15 | 8197 | 0 |
| epoll-spin | 16 | 8195 | 0 |
| epoll-spin | 17 | 8198 | 4 |
| epoll-spin | 18 | 8200 | 4 |
| epoll-spin | 19 | 8196 | 4 |
| epoll-spin | 20 | 8198 | 4 |
| epoll-spin | 21 | 8198 | 4 |
| epoll-spin | 22 | 8200 | 4 |
| epoll-spin | 23 | 8195 | 0 |
| epoll-spin | 24 | 8196 | 4 |
| epoll-spin | 25 | 8196 | 4 |
| epoll-spin | 26 | 8193 | 0 |
| epoll-spin | 27 | 8200 | 4 |
| epoll-spin | 28 | 8193 | 0 |
| epoll-spin | 29 | 8200 | 4 |
| epoll-spin | 30 | 8198 | 4 |
| epoll-spin | 31 | 8195 | 0 |
| epoll-spin | 32 | 8198 | 4 |
| epoll-spin | 33 | 8194 | 4 |
| epoll-spin | 34 | 8195 | 0 |
| epoll-spin | 35 | 8193 | 0 |
| epoll-spin | 36 | 8200 | 4 |
| epoll-spin | 37 | 8196 | 4 |
| epoll-spin | 38 | 8198 | 4 |
| epoll-spin | 39 | 8200 | 4 |
| epoll-spin | 40 | 8196 | 4 |
| epoll-spin | 41 | 8196 | 4 |
| epoll-spin | 42 | 8198 | 4 |
| epoll-spin | 43 | 8200 | 4 |
| epoll-spin | 44 | 8195 | 0 |
| epoll-spin | 45 | 8196 | 4 |
| epoll-spin | 46 | 8193 | 0 |
| epoll-spin | 47 | 8198 | 4 |
| epoll-spin | 48 | 8193 | 0 |
| epoll-spin | 49 | 8194 | 4 |
| epoll-spin | 50 | 8198 | 4 |
| epoll-spin | 51 | 8196 | 4 |
| epoll-spin | 52 | 8198 | 4 |
| epoll-spin | 53 | 8193 | 0 |
| epoll-spin | 54 | 8198 | 4 |
| epoll-spin | 55 | 8199 | 0 |
| epoll-spin | 56 | 8193 | 0 |
| epoll-spin | 57 | 8200 | 4 |
| epoll-spin | 58 | 8195 | 0 |
| epoll-spin | 59 | 8196 | 4 |
| epoll-spin | 60 | 8194 | 4 |
| epoll-spin | 61 | 8197 | 0 |
| epoll-spin | 62 | 8193 | 0 |
| epoll-spin | 63 | 8200 | 4 |
| epoll-spin | 64 | 8195 | 0 |
| epoll-spin | 65 | 8194 | 4 |
| epoll-spin | 66 | 8196 | 4 |
| epoll-spin | 67 | 8193 | 0 |
| epoll-spin | 68 | 8194 | 4 |
| epoll-spin | 69 | 8196 | 4 |
| epoll-spin | 70 | 8195 | 0 |
| epoll-spin | 71 | 8200 | 4 |
| epoll-spin | 72 | 8197 | 0 |
| epoll-spin | 73 | 8194 | 4 |
| epoll-spin | 74 | 8197 | 0 |
| epoll-spin | 75 | 8200 | 4 |
| epoll-spin | 76 | 8198 | 4 |
| epoll-spin | 77 | 8194 | 4 |
| epoll-spin | 78 | 8193 | 0 |
| epoll-spin | 79 | 8197 | 0 |
| epoll-spin | 80 | 8199 | 0 |
| epoll-spin | 81 | 8194 | 4 |
| epoll-spin | 82 | 8199 | 0 |
| epoll-spin | 83 | 8196 | 4 |
| epoll-spin | 84 | 8194 | 4 |
| epoll-spin | 85 | 8195 | 0 |
| epoll-spin | 86 | 8193 | 0 |
| epoll-spin | 87 | 8198 | 4 |
| epoll-spin | 88 | 8199 | 0 |
| epoll-spin | 89 | 8194 | 4 |
| epoll-spin | 90 | 8193 | 0 |
| epoll-spin | 91 | 8200 | 4 |
| epoll-spin | 92 | 8195 | 0 |
| epoll-spin | 93 | 8197 | 0 |
| epoll-spin | 94 | 8196 | 4 |
| epoll-spin | 95 | 8197 | 0 |
| epoll-spin | 96 | 8199 | 0 |
| epoll-spin | 97 | 8196 | 4 |
| epoll-spin | 98 | 8193 | 0 |
| epoll-spin | 99 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1017914688 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1672822 of 4387094 in 1-msg reads) | 1672822 | 4.68 | 3.70 | 7.63 | 14.46 | 23.82 | 36.27 | 65.47 |

#### Stalls: m1/lab-epoll-spin-c100, loop 0 on CPU 1

181181824 iterations, longest 1772.3 µs; over 5 / 20 / 100 µs: 271410 / 768 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 537 | 14895.2 | 185.2 |
| loop work | 229 | 8598.6 | 1772.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.9 | 34.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1772.3 | 4361 | loop work (4361 items) |  |
| 2 | 672.8 | 1773 | loop work (1773 items) |  |
| 3 | 185.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 64.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 62.0 | 8 | loop work (8 items) |  |
| 6 | 61.6 | 20 | loop work (20 items) |  |
| 7 | 60.6 | 16 | loop work (16 items) |  |
| 8 | 57.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 54.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 54.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-mio-c1: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m1/lab-mio-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 355380 | 0 | 0.00 | 262820854 | 739.55 | 4380348 | 4376179 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| mio | 0 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|

#### Stalls: m1/lab-mio-c1, loop 0 on CPU 1

262570762 iterations, longest 6771.5 µs; over 5 / 20 / 100 µs: 44441 / 10 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5 | 6859.6 | 6771.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 119.4 | 50.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.9 | 33.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 6771.5 | 462 | loop work (462 items) | local timer tick 11, timer tick_nohz_handler 7, irq work (IPI/vector) 1, reschedule (IPI/vector) 1 |
| 2 | 50.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 33.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 24.8 | 3 | loop work (3 items) |  |
| 5 | 23.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 23.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 22.4 | 10 | loop work (10 items) |  |
| 8 | 21.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 20.7 | 7 | loop work (7 items) |  |
| 10 | 20.1 | 11 | loop work (11 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| LOC Local timer interrupts | 13 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |

## m1/lab-mio-c10: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m1/lab-mio-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 3162945 | 0 | 0.00 | 253473590 | 80.14 | 4224560 | 4197389 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| mio | 0 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|

#### Stalls: m1/lab-mio-c10, loop 0 on CPU 1

251843356 iterations, longest 8570.6 µs; over 5 / 20 / 100 µs: 279258 / 351 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 334 | 17203.9 | 8570.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 16 | 416.2 | 37.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 32.9 | 32.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 8570.6 | 4207 | loop work (4207 items) |  |
| 2 | 59.7 | 3 | loop work (3 items) |  |
| 3 | 50.2 | 3 | loop work (3 items) |  |
| 4 | 50.0 | 3 | loop work (3 items) |  |
| 5 | 47.7 | 3 | loop work (3 items) |  |
| 6 | 47.1 | 17 | loop work (17 items) |  |
| 7 | 44.7 | 3 | loop work (3 items) |  |
| 8 | 40.2 | 3 | loop work (3 items) |  |
| 9 | 39.9 | 3 | loop work (3 items) |  |
| 10 | 39.8 | 3 | loop work (3 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| IWI IRQ work interrupts | 32 |
| LOC Local timer interrupts | 10 |
| RES Rescheduling interrupts | 1 |

## m1/lab-mio-c100: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m1/lab-mio-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 4544379 | 0 | 0.00 | 240204830 | 52.86 | 4003414 | 3931372 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| mio | 0 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|

#### Stalls: m1/lab-mio-c100, loop 0 on CPU 1

235882296 iterations, longest 10069.9 µs; over 5 / 20 / 100 µs: 354676 / 2208 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1728 | 52606.1 | 10069.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 479 | 12864.3 | 114.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.5 | 21.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 10069.9 | 6164 | loop work (6164 items) |  |
| 2 | 114.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 108.3 | 85 | loop work (85 items) |  |
| 4 | 69.6 | 9 | loop work (9 items) |  |
| 5 | 61.7 | 7 | loop work (7 items) |  |
| 6 | 61.1 | 8 | loop work (8 items) |  |
| 7 | 60.9 | 7 | loop work (7 items) |  |
| 8 | 59.7 | 7 | loop work (7 items) |  |
| 9 | 59.0 | 6 | loop work (6 items) |  |
| 10 | 58.8 | 3 | loop work (3 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 303 |
| LOC Local timer interrupts | 17 |
| RES Rescheduling interrupts | 2 |

## m1/lab-recv-busy-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=110589 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355389 | 122315 | 2.91 | 106702016 | 300.24 | 1778367 | 1778367 | 99.9 | 65.5 | true | 0 | 285 | 152.6 | 49868 / 1401 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79607136 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (61726 of 343093 in 1-msg reads) | 61726 | 3.71 | 3.39 | 5.02 | 7.34 | 25.78 | 35.63 | 41.50 |

#### Stalls: m1/lab-recv-busy-c1, loop 0 on CPU 1

106702016 iterations, longest 152.6 µs; over 5 / 20 / 100 µs: 49868 / 1401 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 815 | 25511.1 | 152.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 375 | 10171.0 | 41.4 |
| mostly unexplained (+preempted by kworker/1:1) | 104 | 3149.0 | 60.3 |
| softirq NET_RX | 53 | 1552.9 | 37.8 |
| mostly unexplained (+softirq NET_RX) | 50 | 1465.2 | 42.1 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 65.4 | 43.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 57.7 | 31.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 152.6 | 472 | loop work (472 items) |  |
| 2 | 60.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 57.0 | 10 | loop work (10 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 56.4 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 52.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 47.7 | 10 | loop work (10 items) |  |
| 7 | 47.7 | 10 | loop work (10 items) | softirq NET_RX 3 |
| 8 | 47.5 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 10, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 47.0 | 10 | loop work (10 items) | softirq NET_RX 2 |
| 10 | 46.2 | 10 | loop work (10 items) | softirq NET_RX 13, preempted by kworker/1:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 294 |
| LOC Local timer interrupts | 287 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=668284 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3162945 | 789486 | 4.01 | 105502080 | 33.36 | 1758368 | 175837 | 99.3 | 67.8 | true | 0 | 1127 | 1139.7 | 10393072 / 59700 / 9 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 1 |
| recv-busy | 1 | 8199 | 1 |
| recv-busy | 2 | 8200 | 1 |
| recv-busy | 3 | 8196 | 1 |
| recv-busy | 4 | 8198 | 1 |
| recv-busy | 5 | 8197 | 1 |
| recv-busy | 6 | 8195 | 1 |
| recv-busy | 7 | 8197 | 1 |
| recv-busy | 8 | 8195 | 1 |
| recv-busy | 9 | 8199 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708499680 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (225702 of 3053559 in 1-msg reads) | 225702 | 5.68 | 4.74 | 9.30 | 19.71 | 38.19 | 53.04 | 83.55 |

#### Stalls: m1/lab-recv-busy-c10, loop 0 on CPU 1

10550208 iterations, longest 1139.7 µs; over 5 / 20 / 100 µs: 10393072 / 59700 / 9

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 57966 | 1559745.3 | 1139.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1212 | 40460.2 | 61.9 |
| mostly unexplained (+preempted by kworker/1:1) | 395 | 14737.6 | 64.6 |
| mostly unexplained (+softirq NET_RX) | 62 | 2074.1 | 49.8 |
| mostly unexplained (+local timer tick) | 37 | 947.4 | 79.0 |
| mostly unexplained (+irq work (IPI/vector)) | 19 | 500.8 | 37.7 |
| softirq NET_RX | 7 | 342.3 | 59.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 72.4 | 42.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1139.7 | 4167 | loop work (4167 items) |  |
| 2 | 119.9 | 56 | loop work (56 items) |  |
| 3 | 107.4 | 59 | loop work (59 items) |  |
| 4 | 104.6 | 67 | loop work (67 items) |  |
| 5 | 103.6 | 47 | loop work (47 items) |  |
| 6 | 102.6 | 62 | loop work (62 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 102.6 | 61 | loop work (61 items) | softirq NET_RX 1 |
| 8 | 101.3 | 63 | loop work (63 items) |  |
| 9 | 100.9 | 46 | loop work (46 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 99.3 | 45 | loop work (45 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1161 |
| LOC Local timer interrupts | 1131 |
| CAL Function call interrupts | 994 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1523578 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4544268 | 1917431 | 2.37 | 95180800 | 20.95 | 1586347 | 15863 | 98.0 | 70.1 | true | 0 | 4621 | 2420.0 | 951807 / 951807 / 28601 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 0 |
| recv-busy | 1 | 8195 | 0 |
| recv-busy | 2 | 8197 | 1 |
| recv-busy | 3 | 8193 | 1 |
| recv-busy | 4 | 8199 | 0 |
| recv-busy | 5 | 8200 | 1 |
| recv-busy | 6 | 8197 | 0 |
| recv-busy | 7 | 8199 | 0 |
| recv-busy | 8 | 8196 | 1 |
| recv-busy | 9 | 8195 | 1 |
| recv-busy | 10 | 8196 | 1 |
| recv-busy | 11 | 8200 | 4 |
| recv-busy | 12 | 8196 | 1 |
| recv-busy | 13 | 8194 | 1 |
| recv-busy | 14 | 8199 | 1 |
| recv-busy | 15 | 8199 | 0 |
| recv-busy | 16 | 8196 | 4 |
| recv-busy | 17 | 8198 | 4 |
| recv-busy | 18 | 8194 | 1 |
| recv-busy | 19 | 8196 | 1 |
| recv-busy | 20 | 8198 | 1 |
| recv-busy | 21 | 8196 | 4 |
| recv-busy | 22 | 8198 | 4 |
| recv-busy | 23 | 8196 | 1 |
| recv-busy | 24 | 8193 | 0 |
| recv-busy | 25 | 8198 | 1 |
| recv-busy | 26 | 8195 | 1 |
| recv-busy | 27 | 8198 | 1 |
| recv-busy | 28 | 8196 | 4 |
| recv-busy | 29 | 8195 | 1 |
| recv-busy | 30 | 8195 | 1 |
| recv-busy | 31 | 8194 | 1 |
| recv-busy | 32 | 8197 | 0 |
| recv-busy | 33 | 8198 | 1 |
| recv-busy | 34 | 8194 | 4 |
| recv-busy | 35 | 8198 | 1 |
| recv-busy | 36 | 8200 | 1 |
| recv-busy | 37 | 8199 | 1 |
| recv-busy | 38 | 8194 | 1 |
| recv-busy | 39 | 8196 | 1 |
| recv-busy | 40 | 8196 | 1 |
| recv-busy | 41 | 8200 | 1 |
| recv-busy | 42 | 8195 | 1 |
| recv-busy | 43 | 8194 | 4 |
| recv-busy | 44 | 8200 | 4 |
| recv-busy | 45 | 8196 | 1 |
| recv-busy | 46 | 8198 | 1 |
| recv-busy | 47 | 8199 | 1 |
| recv-busy | 48 | 8196 | 1 |
| recv-busy | 49 | 8200 | 1 |
| recv-busy | 50 | 8200 | 4 |
| recv-busy | 51 | 8198 | 4 |
| recv-busy | 52 | 8199 | 0 |
| recv-busy | 53 | 8194 | 1 |
| recv-busy | 54 | 8197 | 0 |
| recv-busy | 55 | 8198 | 4 |
| recv-busy | 56 | 8196 | 4 |
| recv-busy | 57 | 8193 | 0 |
| recv-busy | 58 | 8198 | 1 |
| recv-busy | 59 | 8193 | 1 |
| recv-busy | 60 | 8194 | 4 |
| recv-busy | 61 | 8200 | 1 |
| recv-busy | 62 | 8195 | 1 |
| recv-busy | 63 | 8196 | 1 |
| recv-busy | 64 | 8200 | 4 |
| recv-busy | 65 | 8193 | 0 |
| recv-busy | 66 | 8197 | 1 |
| recv-busy | 67 | 8199 | 1 |
| recv-busy | 68 | 8195 | 0 |
| recv-busy | 69 | 8193 | 1 |
| recv-busy | 70 | 8199 | 1 |
| recv-busy | 71 | 8198 | 1 |
| recv-busy | 72 | 8196 | 1 |
| recv-busy | 73 | 8195 | 1 |
| recv-busy | 74 | 8195 | 0 |
| recv-busy | 75 | 8193 | 0 |
| recv-busy | 76 | 8199 | 1 |
| recv-busy | 77 | 8197 | 0 |
| recv-busy | 78 | 8200 | 1 |
| recv-busy | 79 | 8194 | 4 |
| recv-busy | 80 | 8195 | 1 |
| recv-busy | 81 | 8193 | 1 |
| recv-busy | 82 | 8198 | 4 |
| recv-busy | 83 | 8195 | 0 |
| recv-busy | 84 | 8193 | 0 |
| recv-busy | 85 | 8199 | 1 |
| recv-busy | 86 | 8200 | 1 |
| recv-busy | 87 | 8196 | 1 |
| recv-busy | 88 | 8193 | 0 |
| recv-busy | 89 | 8198 | 1 |
| recv-busy | 90 | 8196 | 4 |
| recv-busy | 91 | 8193 | 1 |
| recv-busy | 92 | 8199 | 0 |
| recv-busy | 93 | 8197 | 0 |
| recv-busy | 94 | 8199 | 1 |
| recv-busy | 95 | 8193 | 1 |
| recv-busy | 96 | 8196 | 1 |
| recv-busy | 97 | 8194 | 1 |
| recv-busy | 98 | 8195 | 1 |
| recv-busy | 99 | 8193 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1017916032 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1490116 of 4387093 in 1-msg reads) | 1490116 | 32.14 | 31.03 | 57.60 | 81.99 | 107.45 | 138.36 | 187.95 |

#### Stalls: m1/lab-recv-busy-c100, loop 0 on CPU 1

951808 iterations, longest 2420.0 µs; over 5 / 20 / 100 µs: 951807 / 951807 / 28601

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 750372 | 44714293.1 | 155.5 |
| loop work | 171061 | 13145388.5 | 2420.0 |
| mostly unexplained (+softirq NET_RX) | 23493 | 1524084.3 | 157.8 |
| mostly unexplained (+local timer tick) | 3970 | 283572.5 | 129.4 |
| mostly unexplained (+preempted by kworker/1:1) | 2668 | 225335.7 | 137.8 |
| mostly unexplained (+irq work (IPI/vector)) | 215 | 15835.9 | 108.5 |
| mostly unexplained (+ipi_send_cpu) | 9 | 614.4 | 79.2 |
| mostly unexplained (+softirq NET_TX) | 8 | 608.0 | 142.4 |
| mostly unexplained (+call function (IPI/vector)) | 7 | 470.7 | 74.0 |
| mostly unexplained (+softirq RCU) | 2 | 133.0 | 75.9 |
| mostly unexplained (+softirq TIMER) | 1 | 95.9 | 95.9 |
| mostly unexplained (+call function single (IPI/vector)) | 1 | 62.8 | 62.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2420.0 | 6075 | loop work (6075 items) |  |
| 2 | 202.1 | 67 | loop work (67 items) | softirq NET_RX 16, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 196.2 | 75 | loop work (75 items) | preempted by kworker/1:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 193.9 | 55 | loop work (55 items) |  |
| 5 | 192.5 | 62 | loop work (62 items) | softirq NET_RX 6 |
| 6 | 190.7 | 77 | loop work (77 items) | softirq NET_RX 5 |
| 7 | 189.1 | 55 | loop work (55 items) | softirq NET_RX 2 |
| 8 | 188.9 | 104 | loop work (104 items) |  |
| 9 | 188.1 | 69 | loop work (69 items) |  |
| 10 | 187.5 | 50 | loop work (50 items) | softirq NET_RX 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4662 |
| LOC Local timer interrupts | 4491 |
| CAL Function call interrupts | 989 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=112238 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355391 | 123249 | 2.88 | 148499968 | 417.85 | 2474999 | 1237500 | 99.8 | 56.9 | true | 0 | 290 | 179.6 | 62082 / 1487 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79607584 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (61003 of 343094 in 1-msg reads) | 61003 | 3.83 | 3.51 | 5.28 | 7.55 | 27.62 | 38.18 | 47.09 |

#### Stalls: m1/lab-recv-busy-pq-c1, loop 0 on CPU 1

74249984 iterations, longest 179.6 µs; over 5 / 20 / 100 µs: 62082 / 1487 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 923 | 28746.1 | 179.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 382 | 10583.1 | 37.1 |
| mostly unexplained (+preempted by kworker/1:1) | 96 | 3060.5 | 62.8 |
| softirq NET_RX | 39 | 1187.8 | 39.9 |
| mostly unexplained (+softirq NET_RX) | 37 | 1079.3 | 36.2 |
| mostly unexplained (+local timer tick) | 4 | 131.2 | 46.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 90.3 | 38.7 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 83.0 | 36.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 179.6 | 473 | loop work (473 items) |  |
| 2 | 62.9 | 9 | loop work (9 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 62.8 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 59.9 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 59.0 | 10 | loop work (10 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 58.3 | 3 | loop work (3 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 56.5 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 56.2 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 52.9 | 3 | loop work (3 items) | preempted by kworker/1:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 48.9 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 14, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 300 |
| LOC Local timer interrupts | 296 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=668510 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3163048 | 784355 | 4.03 | 112782066 | 35.66 | 1879701 | 267840 | 99.2 | 66.0 | true | 0 | 1355 | 1221.9 | 475956 / 41167 / 6 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 1 |
| recv-busy | 1 | 8199 | 1 |
| recv-busy | 2 | 8200 | 4 |
| recv-busy | 3 | 8199 | 0 |
| recv-busy | 4 | 8195 | 1 |
| recv-busy | 5 | 8194 | 1 |
| recv-busy | 6 | 8197 | 1 |
| recv-busy | 7 | 8197 | 1 |
| recv-busy | 8 | 8198 | 4 |
| recv-busy | 9 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708522752 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (220066 of 3053560 in 1-msg reads) | 220066 | 6.14 | 5.43 | 9.84 | 19.79 | 37.14 | 50.49 | 69.31 |

#### Stalls: m1/lab-recv-busy-pq-c10, loop 0 on CPU 1

16070400 iterations, longest 1221.9 µs; over 5 / 20 / 100 µs: 475956 / 41167 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 39595 | 1092080.1 | 1221.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1029 | 33548.5 | 67.7 |
| mostly unexplained (+preempted by kworker/1:1) | 448 | 16276.6 | 72.8 |
| mostly unexplained (+softirq NET_RX) | 44 | 1508.1 | 46.1 |
| mostly unexplained (+irq work (IPI/vector)) | 22 | 560.3 | 38.4 |
| mostly unexplained (+local timer tick) | 20 | 548.5 | 52.6 |
| softirq NET_RX | 6 | 231.7 | 52.3 |
| softirq TIMER | 1 | 119.5 | 119.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 40.4 | 40.4 |
| mostly unexplained (+ipi_send_cpu) | 1 | 38.1 | 38.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1221.9 | 4218 | loop work (4218 items) |  |
| 2 | 119.5 | 0 | softirq TIMER | softirq TIMER 92, local timer tick 12, timer tick_nohz_handler 6 |
| 3 | 108.2 | 54 | loop work (54 items) | softirq NET_RX 3 |
| 4 | 103.5 | 61 | loop work (61 items) | softirq NET_RX 5 |
| 5 | 102.4 | 57 | loop work (57 items) |  |
| 6 | 100.6 | 57 | loop work (57 items) |  |
| 7 | 97.5 | 66 | loop work (66 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 96.4 | 51 | loop work (51 items) |  |
| 9 | 96.0 | 50 | loop work (50 items) |  |
| 10 | 95.8 | 66 | loop work (66 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1386 |
| LOC Local timer interrupts | 1358 |
| CAL Function call interrupts | 992 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1636638 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4544378 | 2117850 | 2.15 | 98349416 | 21.64 | 1639157 | 178557 | 97.6 | 68.3 | true | 0 | 5886 | 2074.0 | 3628045 / 77125 / 11 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 1 |
| recv-busy | 1 | 8198 | 1 |
| recv-busy | 2 | 8197 | 0 |
| recv-busy | 3 | 8200 | 4 |
| recv-busy | 4 | 8199 | 0 |
| recv-busy | 5 | 8193 | 1 |
| recv-busy | 6 | 8195 | 0 |
| recv-busy | 7 | 8200 | 4 |
| recv-busy | 8 | 8194 | 4 |
| recv-busy | 9 | 8198 | 1 |
| recv-busy | 10 | 8199 | 0 |
| recv-busy | 11 | 8195 | 1 |
| recv-busy | 12 | 8200 | 1 |
| recv-busy | 13 | 8197 | 0 |
| recv-busy | 14 | 8193 | 0 |
| recv-busy | 15 | 8195 | 1 |
| recv-busy | 16 | 8197 | 0 |
| recv-busy | 17 | 8199 | 0 |
| recv-busy | 18 | 8195 | 1 |
| recv-busy | 19 | 8195 | 0 |
| recv-busy | 20 | 8197 | 1 |
| recv-busy | 21 | 8200 | 1 |
| recv-busy | 22 | 8197 | 0 |
| recv-busy | 23 | 8195 | 1 |
| recv-busy | 24 | 8200 | 1 |
| recv-busy | 25 | 8194 | 1 |
| recv-busy | 26 | 8200 | 1 |
| recv-busy | 27 | 8194 | 1 |
| recv-busy | 28 | 8195 | 1 |
| recv-busy | 29 | 8197 | 0 |
| recv-busy | 30 | 8197 | 0 |
| recv-busy | 31 | 8196 | 1 |
| recv-busy | 32 | 8195 | 0 |
| recv-busy | 33 | 8200 | 1 |
| recv-busy | 34 | 8194 | 1 |
| recv-busy | 35 | 8200 | 1 |
| recv-busy | 36 | 8195 | 1 |
| recv-busy | 37 | 8193 | 1 |
| recv-busy | 38 | 8194 | 1 |
| recv-busy | 39 | 8195 | 1 |
| recv-busy | 40 | 8194 | 1 |
| recv-busy | 41 | 8193 | 0 |
| recv-busy | 42 | 8196 | 1 |
| recv-busy | 43 | 8200 | 1 |
| recv-busy | 44 | 8196 | 1 |
| recv-busy | 45 | 8193 | 1 |
| recv-busy | 46 | 8194 | 1 |
| recv-busy | 47 | 8200 | 1 |
| recv-busy | 48 | 8195 | 1 |
| recv-busy | 49 | 8196 | 1 |
| recv-busy | 50 | 8198 | 1 |
| recv-busy | 51 | 8200 | 1 |
| recv-busy | 52 | 8193 | 1 |
| recv-busy | 53 | 8197 | 1 |
| recv-busy | 54 | 8199 | 0 |
| recv-busy | 55 | 8194 | 1 |
| recv-busy | 56 | 8196 | 1 |
| recv-busy | 57 | 8196 | 4 |
| recv-busy | 58 | 8199 | 1 |
| recv-busy | 59 | 8198 | 1 |
| recv-busy | 60 | 8200 | 1 |
| recv-busy | 61 | 8193 | 1 |
| recv-busy | 62 | 8194 | 1 |
| recv-busy | 63 | 8197 | 0 |
| recv-busy | 64 | 8199 | 1 |
| recv-busy | 65 | 8197 | 1 |
| recv-busy | 66 | 8196 | 1 |
| recv-busy | 67 | 8195 | 1 |
| recv-busy | 68 | 8196 | 4 |
| recv-busy | 69 | 8200 | 1 |
| recv-busy | 70 | 8197 | 0 |
| recv-busy | 71 | 8195 | 1 |
| recv-busy | 72 | 8196 | 1 |
| recv-busy | 73 | 8193 | 1 |
| recv-busy | 74 | 8198 | 1 |
| recv-busy | 75 | 8194 | 1 |
| recv-busy | 76 | 8197 | 1 |
| recv-busy | 77 | 8200 | 1 |
| recv-busy | 78 | 8195 | 1 |
| recv-busy | 79 | 8193 | 1 |
| recv-busy | 80 | 8195 | 1 |
| recv-busy | 81 | 8199 | 0 |
| recv-busy | 82 | 8194 | 4 |
| recv-busy | 83 | 8197 | 0 |
| recv-busy | 84 | 8196 | 4 |
| recv-busy | 85 | 8194 | 1 |
| recv-busy | 86 | 8197 | 0 |
| recv-busy | 87 | 8193 | 0 |
| recv-busy | 88 | 8197 | 0 |
| recv-busy | 89 | 8198 | 1 |
| recv-busy | 90 | 8196 | 1 |
| recv-busy | 91 | 8200 | 1 |
| recv-busy | 92 | 8196 | 1 |
| recv-busy | 93 | 8200 | 1 |
| recv-busy | 94 | 8194 | 4 |
| recv-busy | 95 | 8196 | 1 |
| recv-busy | 96 | 8197 | 1 |
| recv-busy | 97 | 8199 | 0 |
| recv-busy | 98 | 8194 | 1 |
| recv-busy | 99 | 8196 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1017940672 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1524622 of 4387135 in 1-msg reads) | 1524622 | 6.80 | 5.95 | 10.00 | 24.23 | 38.88 | 53.83 | 82.56 |

#### Stalls: m1/lab-recv-busy-pq-c100, loop 0 on CPU 1

10713408 iterations, longest 2074.0 µs; over 5 / 20 / 100 µs: 3628045 / 77125 / 11

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 69504 | 1965423.0 | 2074.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4560 | 149163.2 | 87.3 |
| mostly unexplained (+preempted by kworker/1:1) | 2351 | 68718.3 | 70.5 |
| mostly unexplained (+local timer tick) | 459 | 11068.0 | 57.3 |
| mostly unexplained (+softirq NET_RX) | 138 | 4280.1 | 47.1 |
| mostly unexplained (+irq work (IPI/vector)) | 97 | 2277.5 | 55.7 |
| softirq NET_RX | 13 | 504.7 | 64.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 42.9 | 42.9 |
| mostly unexplained (+ipi_send_cpu) | 1 | 42.5 | 42.5 |
| mostly unexplained (+softirq TIMER) | 1 | 30.5 | 30.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2074.0 | 4804 | loop work (4804 items) |  |
| 2 | 523.9 | 1360 | loop work (1360 items) |  |
| 3 | 121.5 | 58 | loop work (58 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 119.6 | 71 | loop work (71 items) | softirq NET_RX 5, preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 109.8 | 63 | loop work (63 items) | softirq NET_RX 14, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 109.4 | 77 | loop work (77 items) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 105.5 | 59 | loop work (59 items) |  |
| 8 | 103.9 | 63 | loop work (63 items) | softirq NET_RX 4 |
| 9 | 102.6 | 52 | loop work (52 items) |  |
| 10 | 102.6 | 61 | loop work (61 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5922 |
| LOC Local timer interrupts | 5726 |
| CAL Function call interrupts | 990 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-uring-spin-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 355391 | 131661 | 2.70 | 231793 | 0.65 | 3863 | 50030674 | 0.0 | 0.8 | false | 0 | 0 | 93.7 | 34333 / 41 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 79607584 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-spin (82507 of 343095 in 1-msg reads) | 82507 | 8.15 | 6.83 | 13.54 | 20.49 | 23.85 | 34.34 | 41.94 |

#### Stalls: m1/lab-uring-spin-c1, loop 0 on CPU 1

3001840448 iterations, longest 93.7 µs; over 5 / 20 / 100 µs: 34333 / 41 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 39 | 1017.7 | 93.7 |
| loop work | 1 | 81.1 | 81.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.3 | 35.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 93.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 81.1 | 472 | loop work (472 items) |  |
| 3 | 35.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 33.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 33.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 32.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 30.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 30.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 29.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 28.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-spin-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-uring-spin-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 3163048 | 916807 | 3.45 | 1166662 | 0.37 | 19444 | 30881374 | 0.0 | 4.4 | false | 0 | 0 | 680.3 | 179898 / 443 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8199 | 0 |
| uring-spin | 1 | 8200 | 4 |
| uring-spin | 2 | 8194 | 4 |
| uring-spin | 3 | 8195 | 0 |
| uring-spin | 4 | 8193 | 0 |
| uring-spin | 5 | 8196 | 4 |
| uring-spin | 6 | 8193 | 0 |
| uring-spin | 7 | 8195 | 0 |
| uring-spin | 8 | 8200 | 4 |
| uring-spin | 9 | 8197 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 708522752 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-spin (358186 of 3053559 in 1-msg reads) | 358186 | 6.70 | 6.05 | 10.73 | 17.54 | 27.34 | 41.73 | 58.95 |

#### Stalls: m1/lab-uring-spin-c10, loop 0 on CPU 1

1852882432 iterations, longest 680.3 µs; over 5 / 20 / 100 µs: 179898 / 443 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 248 | 7010.5 | 61.4 |
| loop work | 190 | 5536.9 | 680.3 |
| mostly unexplained (+local timer tick) | 2 | 657.2 | 623.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 52.6 | 32.1 |
| mostly unexplained (+softirq NET_TX) | 1 | 22.3 | 22.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 680.3 | 4218 | loop work (4218 items) | local timer tick 7, timer tick_nohz_handler 3 |
| 2 | 623.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 11, timer tick_nohz_handler 7, softirq RCU 2 |
| 3 | 68.2 | 7 | loop work (7 items) |  |
| 4 | 61.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 56.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 53.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 53.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 50.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 50.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 48.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-spin-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-uring-spin-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 4544257 | 2289431 | 1.98 | 3574106 | 0.79 | 59568 | 6947730 | 0.0 | 10.2 | false | 0 | 0 | 1346.6 | 267601 / 1128 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8199 | 0 |
| uring-spin | 1 | 8195 | 0 |
| uring-spin | 2 | 8197 | 0 |
| uring-spin | 3 | 8197 | 0 |
| uring-spin | 4 | 8196 | 4 |
| uring-spin | 5 | 8194 | 4 |
| uring-spin | 6 | 8197 | 0 |
| uring-spin | 7 | 8195 | 0 |
| uring-spin | 8 | 8195 | 0 |
| uring-spin | 9 | 8199 | 0 |
| uring-spin | 10 | 8200 | 4 |
| uring-spin | 11 | 8197 | 0 |
| uring-spin | 12 | 8194 | 4 |
| uring-spin | 13 | 8194 | 4 |
| uring-spin | 14 | 8198 | 4 |
| uring-spin | 15 | 8200 | 4 |
| uring-spin | 16 | 8195 | 0 |
| uring-spin | 17 | 8199 | 0 |
| uring-spin | 18 | 8198 | 4 |
| uring-spin | 19 | 8196 | 4 |
| uring-spin | 20 | 8199 | 0 |
| uring-spin | 21 | 8193 | 0 |
| uring-spin | 22 | 8194 | 4 |
| uring-spin | 23 | 8197 | 0 |
| uring-spin | 24 | 8195 | 0 |
| uring-spin | 25 | 8199 | 0 |
| uring-spin | 26 | 8194 | 4 |
| uring-spin | 27 | 8197 | 0 |
| uring-spin | 28 | 8195 | 0 |
| uring-spin | 29 | 8197 | 0 |
| uring-spin | 30 | 8199 | 0 |
| uring-spin | 31 | 8198 | 4 |
| uring-spin | 32 | 8200 | 4 |
| uring-spin | 33 | 8196 | 4 |
| uring-spin | 34 | 8199 | 0 |
| uring-spin | 35 | 8200 | 4 |
| uring-spin | 36 | 8196 | 4 |
| uring-spin | 37 | 8198 | 4 |
| uring-spin | 38 | 8198 | 4 |
| uring-spin | 39 | 8196 | 4 |
| uring-spin | 40 | 8198 | 4 |
| uring-spin | 41 | 8196 | 4 |
| uring-spin | 42 | 8194 | 4 |
| uring-spin | 43 | 8195 | 0 |
| uring-spin | 44 | 8193 | 0 |
| uring-spin | 45 | 8197 | 0 |
| uring-spin | 46 | 8197 | 0 |
| uring-spin | 47 | 8193 | 0 |
| uring-spin | 48 | 8194 | 4 |
| uring-spin | 49 | 8197 | 0 |
| uring-spin | 50 | 8194 | 4 |
| uring-spin | 51 | 8200 | 4 |
| uring-spin | 52 | 8194 | 4 |
| uring-spin | 53 | 8198 | 4 |
| uring-spin | 54 | 8200 | 4 |
| uring-spin | 55 | 8193 | 0 |
| uring-spin | 56 | 8196 | 4 |
| uring-spin | 57 | 8195 | 0 |
| uring-spin | 58 | 8200 | 4 |
| uring-spin | 59 | 8199 | 0 |
| uring-spin | 60 | 8198 | 4 |
| uring-spin | 61 | 8196 | 4 |
| uring-spin | 62 | 8196 | 4 |
| uring-spin | 63 | 8193 | 0 |
| uring-spin | 64 | 8200 | 4 |
| uring-spin | 65 | 8197 | 0 |
| uring-spin | 66 | 8199 | 0 |
| uring-spin | 67 | 8195 | 0 |
| uring-spin | 68 | 8197 | 0 |
| uring-spin | 69 | 8193 | 0 |
| uring-spin | 70 | 8196 | 4 |
| uring-spin | 71 | 8199 | 0 |
| uring-spin | 72 | 8200 | 4 |
| uring-spin | 73 | 8198 | 4 |
| uring-spin | 74 | 8200 | 4 |
| uring-spin | 75 | 8193 | 0 |
| uring-spin | 76 | 8196 | 4 |
| uring-spin | 77 | 8194 | 4 |
| uring-spin | 78 | 8195 | 0 |
| uring-spin | 79 | 8199 | 0 |
| uring-spin | 80 | 8197 | 0 |
| uring-spin | 81 | 8198 | 4 |
| uring-spin | 82 | 8194 | 4 |
| uring-spin | 83 | 8195 | 0 |
| uring-spin | 84 | 8195 | 0 |
| uring-spin | 85 | 8194 | 4 |
| uring-spin | 86 | 8197 | 0 |
| uring-spin | 87 | 8195 | 0 |
| uring-spin | 88 | 8197 | 0 |
| uring-spin | 89 | 8193 | 0 |
| uring-spin | 90 | 8197 | 0 |
| uring-spin | 91 | 8194 | 4 |
| uring-spin | 92 | 8199 | 0 |
| uring-spin | 93 | 8194 | 4 |
| uring-spin | 94 | 8200 | 4 |
| uring-spin | 95 | 8199 | 0 |
| uring-spin | 96 | 8194 | 4 |
| uring-spin | 97 | 8193 | 0 |
| uring-spin | 98 | 8198 | 4 |
| uring-spin | 99 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 1017913568 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-spin (1683337 of 4386956 in 1-msg reads) | 1683337 | 5.12 | 3.94 | 8.32 | 16.87 | 27.14 | 39.41 | 71.45 |

#### Stalls: m1/lab-uring-spin-c100, loop 0 on CPU 1

416863808 iterations, longest 1346.6 µs; over 5 / 20 / 100 µs: 267601 / 1128 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 780 | 22641.8 | 1346.6 |
| loop work | 347 | 9947.8 | 664.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 31.9 | 31.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1346.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 664.9 | 3732 | loop work (3732 items) |  |
| 3 | 429.8 | 2347 | loop work (2347 items) |  |
| 4 | 66.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 63.1 | 7 | loop work (7 items) |  |
| 6 | 61.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 60.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 60.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 60.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 60.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=108524 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355391 | 125282 | 2.84 | 1 | 0.00 | 0 | 34635940 | 0.0 | 0.0 | true | 0 | 0 | 95.7 | 335 / 2 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8195 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79607584 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (63208 of 343094 in 1-msg reads) | 63208 | 4.15 | 3.85 | 5.30 | 8.07 | 11.92 | 24.75 | 44.77 |

#### Stalls: m1/lab-uring-sqpoll-c1, loop 0 on CPU 1

2078156416 iterations, longest 95.7 µs; over 5 / 20 / 100 µs: 335 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 95.7 | 95.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.5 | 35.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 95.7 | 473 | loop work (473 items) |  |
| 2 | 35.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 2 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=747431 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3162945 | 817844 | 3.87 | 1 | 0.00 | 0 | 30521209 | 0.0 | 0.0 | true | 0 | 0 | 644.3 | 4260 / 3 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8194 | 2 |
| uring-sqpoll | 1 | 8200 | 2 |
| uring-sqpoll | 2 | 8194 | 2 |
| uring-sqpoll | 3 | 8198 | 2 |
| uring-sqpoll | 4 | 8195 | 2 |
| uring-sqpoll | 5 | 8196 | 2 |
| uring-sqpoll | 6 | 8197 | 2 |
| uring-sqpoll | 7 | 8195 | 0 |
| uring-sqpoll | 8 | 8193 | 2 |
| uring-sqpoll | 9 | 8194 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708499680 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (233644 of 3053457 in 1-msg reads) | 233644 | 6.27 | 5.35 | 9.55 | 20.13 | 35.31 | 49.27 | 69.84 |

#### Stalls: m1/lab-uring-sqpoll-c10, loop 0 on CPU 1

1831272512 iterations, longest 644.3 µs; over 5 / 20 / 100 µs: 4260 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 644.3 | 644.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 60.8 | 60.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 40.3 | 40.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 644.3 | 4196 | loop work (4196 items) |  |
| 2 | 60.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 40.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1990166 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4544257 | 2201418 | 2.06 | 1 | 0.00 | 0 | 7594463 | 0.0 | 0.0 | true | 0 | 0 | 595.1 | 6560 / 5 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8194 | 2 |
| uring-sqpoll | 1 | 8194 | 4 |
| uring-sqpoll | 2 | 8197 | 2 |
| uring-sqpoll | 3 | 8198 | 2 |
| uring-sqpoll | 4 | 8195 | 2 |
| uring-sqpoll | 5 | 8200 | 4 |
| uring-sqpoll | 6 | 8195 | 2 |
| uring-sqpoll | 7 | 8197 | 0 |
| uring-sqpoll | 8 | 8199 | 2 |
| uring-sqpoll | 9 | 8193 | 2 |
| uring-sqpoll | 10 | 8197 | 2 |
| uring-sqpoll | 11 | 8195 | 2 |
| uring-sqpoll | 12 | 8199 | 2 |
| uring-sqpoll | 13 | 8200 | 2 |
| uring-sqpoll | 14 | 8193 | 2 |
| uring-sqpoll | 15 | 8198 | 2 |
| uring-sqpoll | 16 | 8196 | 2 |
| uring-sqpoll | 17 | 8193 | 2 |
| uring-sqpoll | 18 | 8194 | 2 |
| uring-sqpoll | 19 | 8197 | 2 |
| uring-sqpoll | 20 | 8198 | 2 |
| uring-sqpoll | 21 | 8197 | 2 |
| uring-sqpoll | 22 | 8200 | 2 |
| uring-sqpoll | 23 | 8194 | 2 |
| uring-sqpoll | 24 | 8195 | 2 |
| uring-sqpoll | 25 | 8194 | 2 |
| uring-sqpoll | 26 | 8198 | 2 |
| uring-sqpoll | 27 | 8199 | 2 |
| uring-sqpoll | 28 | 8197 | 2 |
| uring-sqpoll | 29 | 8199 | 2 |
| uring-sqpoll | 30 | 8197 | 2 |
| uring-sqpoll | 31 | 8195 | 2 |
| uring-sqpoll | 32 | 8200 | 2 |
| uring-sqpoll | 33 | 8198 | 2 |
| uring-sqpoll | 34 | 8199 | 2 |
| uring-sqpoll | 35 | 8200 | 2 |
| uring-sqpoll | 36 | 8198 | 2 |
| uring-sqpoll | 37 | 8194 | 2 |
| uring-sqpoll | 38 | 8193 | 2 |
| uring-sqpoll | 39 | 8193 | 2 |
| uring-sqpoll | 40 | 8198 | 2 |
| uring-sqpoll | 41 | 8193 | 2 |
| uring-sqpoll | 42 | 8199 | 2 |
| uring-sqpoll | 43 | 8195 | 2 |
| uring-sqpoll | 44 | 8194 | 2 |
| uring-sqpoll | 45 | 8197 | 2 |
| uring-sqpoll | 46 | 8197 | 2 |
| uring-sqpoll | 47 | 8197 | 2 |
| uring-sqpoll | 48 | 8200 | 2 |
| uring-sqpoll | 49 | 8193 | 2 |
| uring-sqpoll | 50 | 8193 | 2 |
| uring-sqpoll | 51 | 8198 | 2 |
| uring-sqpoll | 52 | 8194 | 2 |
| uring-sqpoll | 53 | 8193 | 2 |
| uring-sqpoll | 54 | 8199 | 2 |
| uring-sqpoll | 55 | 8198 | 2 |
| uring-sqpoll | 56 | 8195 | 2 |
| uring-sqpoll | 57 | 8196 | 2 |
| uring-sqpoll | 58 | 8200 | 2 |
| uring-sqpoll | 59 | 8196 | 2 |
| uring-sqpoll | 60 | 8198 | 2 |
| uring-sqpoll | 61 | 8195 | 2 |
| uring-sqpoll | 62 | 8194 | 2 |
| uring-sqpoll | 63 | 8198 | 2 |
| uring-sqpoll | 64 | 8194 | 2 |
| uring-sqpoll | 65 | 8200 | 2 |
| uring-sqpoll | 66 | 8198 | 2 |
| uring-sqpoll | 67 | 8199 | 2 |
| uring-sqpoll | 68 | 8197 | 2 |
| uring-sqpoll | 69 | 8199 | 2 |
| uring-sqpoll | 70 | 8193 | 2 |
| uring-sqpoll | 71 | 8197 | 0 |
| uring-sqpoll | 72 | 8198 | 2 |
| uring-sqpoll | 73 | 8198 | 2 |
| uring-sqpoll | 74 | 8195 | 2 |
| uring-sqpoll | 75 | 8198 | 4 |
| uring-sqpoll | 76 | 8199 | 2 |
| uring-sqpoll | 77 | 8195 | 2 |
| uring-sqpoll | 78 | 8193 | 2 |
| uring-sqpoll | 79 | 8197 | 0 |
| uring-sqpoll | 80 | 8199 | 2 |
| uring-sqpoll | 81 | 8198 | 4 |
| uring-sqpoll | 82 | 8197 | 2 |
| uring-sqpoll | 83 | 8195 | 2 |
| uring-sqpoll | 84 | 8194 | 2 |
| uring-sqpoll | 85 | 8193 | 2 |
| uring-sqpoll | 86 | 8200 | 2 |
| uring-sqpoll | 87 | 8198 | 2 |
| uring-sqpoll | 88 | 8199 | 2 |
| uring-sqpoll | 89 | 8200 | 2 |
| uring-sqpoll | 90 | 8195 | 2 |
| uring-sqpoll | 91 | 8199 | 2 |
| uring-sqpoll | 92 | 8193 | 2 |
| uring-sqpoll | 93 | 8199 | 2 |
| uring-sqpoll | 94 | 8196 | 2 |
| uring-sqpoll | 95 | 8197 | 2 |
| uring-sqpoll | 96 | 8198 | 2 |
| uring-sqpoll | 97 | 8196 | 2 |
| uring-sqpoll | 98 | 8197 | 2 |
| uring-sqpoll | 99 | 8195 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1017913568 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1569306 of 4386952 in 1-msg reads) | 1569306 | 4.96 | 4.18 | 7.28 | 17.14 | 32.01 | 46.01 | 95.17 |

#### Stalls: m1/lab-uring-sqpoll-c100, loop 0 on CPU 1

455667776 iterations, longest 595.1 µs; over 5 / 20 / 100 µs: 6560 / 5 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 967.5 | 595.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 114.3 | 114.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 62.9 | 39.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 595.1 | 3744 | loop work (3744 items) |  |
| 2 | 372.4 | 2336 | loop work (2336 items) |  |
| 3 | 114.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 39.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 23.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 355380 | 135553 | 2.62 | 238119553 | 670.04 | 3968659 | 3966400 | 0.0 | 33.5 | false | 0 | 0 | 177.7 | 26142 / 6 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 79605120 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (82529 of 343095 in 1-msg reads) | 82529 | 6.69 | 5.11 | 13.16 | 20.54 | 22.91 | 27.06 | 39.43 |

#### Stalls: m2/lab-epoll-spin-c1, loop 0 on CPU 1

237984000 iterations, longest 177.7 µs; over 5 / 20 / 100 µs: 26142 / 6 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 177.7 | 177.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 73.7 | 25.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 52.5 | 32.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 177.7 | 461 | loop work (461 items) |  |
| 2 | 32.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 25.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 25.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 23.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 20.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3163051 | 888553 | 3.56 | 224462441 | 70.96 | 3741041 | 3726231 | 0.0 | 35.3 | false | 0 | 0 | 1361.3 | 187008 / 172 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8200 | 4 |
| epoll-spin | 1 | 8193 | 0 |
| epoll-spin | 2 | 8198 | 4 |
| epoll-spin | 3 | 8193 | 0 |
| epoll-spin | 4 | 8198 | 4 |
| epoll-spin | 5 | 8195 | 0 |
| epoll-spin | 6 | 8199 | 0 |
| epoll-spin | 7 | 8196 | 4 |
| epoll-spin | 8 | 8199 | 0 |
| epoll-spin | 9 | 8193 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 708523424 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (338568 of 3053556 in 1-msg reads) | 338568 | 6.10 | 5.58 | 9.55 | 15.77 | 23.39 | 35.36 | 58.49 |

#### Stalls: m2/lab-epoll-spin-c10, loop 0 on CPU 1

223573888 iterations, longest 1361.3 µs; over 5 / 20 / 100 µs: 187008 / 172 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 104 | 4076.2 | 1361.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 66 | 1763.2 | 50.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 57.5 | 34.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1361.3 | 4301 | loop work (4301 items) |  |
| 2 | 57.4 | 14 | loop work (14 items) |  |
| 3 | 50.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 48.3 | 3 | loop work (3 items) |  |
| 5 | 45.1 | 19 | loop work (19 items) |  |
| 6 | 40.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 40.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 38.3 | 12 | loop work (12 items) |  |
| 9 | 37.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 36.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 3 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 4544251 | 2259705 | 2.01 | 181209337 | 39.88 | 3020156 | 2982494 | 0.0 | 33.8 | false | 0 | 0 | 1824.7 | 269122 / 968 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8195 | 0 |
| epoll-spin | 1 | 8194 | 4 |
| epoll-spin | 2 | 8198 | 4 |
| epoll-spin | 3 | 8194 | 4 |
| epoll-spin | 4 | 8200 | 4 |
| epoll-spin | 5 | 8199 | 0 |
| epoll-spin | 6 | 8198 | 4 |
| epoll-spin | 7 | 8194 | 4 |
| epoll-spin | 8 | 8200 | 4 |
| epoll-spin | 9 | 8200 | 4 |
| epoll-spin | 10 | 8200 | 4 |
| epoll-spin | 11 | 8196 | 4 |
| epoll-spin | 12 | 8198 | 4 |
| epoll-spin | 13 | 8194 | 4 |
| epoll-spin | 14 | 8200 | 4 |
| epoll-spin | 15 | 8199 | 0 |
| epoll-spin | 16 | 8200 | 4 |
| epoll-spin | 17 | 8200 | 4 |
| epoll-spin | 18 | 8195 | 0 |
| epoll-spin | 19 | 8200 | 4 |
| epoll-spin | 20 | 8197 | 0 |
| epoll-spin | 21 | 8193 | 0 |
| epoll-spin | 22 | 8196 | 4 |
| epoll-spin | 23 | 8195 | 0 |
| epoll-spin | 24 | 8200 | 4 |
| epoll-spin | 25 | 8193 | 0 |
| epoll-spin | 26 | 8197 | 0 |
| epoll-spin | 27 | 8194 | 4 |
| epoll-spin | 28 | 8196 | 4 |
| epoll-spin | 29 | 8198 | 4 |
| epoll-spin | 30 | 8194 | 4 |
| epoll-spin | 31 | 8199 | 0 |
| epoll-spin | 32 | 8200 | 4 |
| epoll-spin | 33 | 8200 | 4 |
| epoll-spin | 34 | 8196 | 4 |
| epoll-spin | 35 | 8198 | 4 |
| epoll-spin | 36 | 8196 | 4 |
| epoll-spin | 37 | 8199 | 0 |
| epoll-spin | 38 | 8197 | 0 |
| epoll-spin | 39 | 8200 | 4 |
| epoll-spin | 40 | 8195 | 0 |
| epoll-spin | 41 | 8200 | 4 |
| epoll-spin | 42 | 8199 | 0 |
| epoll-spin | 43 | 8196 | 4 |
| epoll-spin | 44 | 8198 | 4 |
| epoll-spin | 45 | 8193 | 0 |
| epoll-spin | 46 | 8194 | 4 |
| epoll-spin | 47 | 8200 | 4 |
| epoll-spin | 48 | 8194 | 4 |
| epoll-spin | 49 | 8195 | 0 |
| epoll-spin | 50 | 8198 | 4 |
| epoll-spin | 51 | 8197 | 0 |
| epoll-spin | 52 | 8196 | 4 |
| epoll-spin | 53 | 8199 | 0 |
| epoll-spin | 54 | 8193 | 0 |
| epoll-spin | 55 | 8199 | 0 |
| epoll-spin | 56 | 8196 | 4 |
| epoll-spin | 57 | 8200 | 4 |
| epoll-spin | 58 | 8198 | 4 |
| epoll-spin | 59 | 8195 | 0 |
| epoll-spin | 60 | 8200 | 4 |
| epoll-spin | 61 | 8194 | 4 |
| epoll-spin | 62 | 8196 | 4 |
| epoll-spin | 63 | 8194 | 4 |
| epoll-spin | 64 | 8199 | 0 |
| epoll-spin | 65 | 8198 | 4 |
| epoll-spin | 66 | 8195 | 0 |
| epoll-spin | 67 | 8193 | 0 |
| epoll-spin | 68 | 8196 | 4 |
| epoll-spin | 69 | 8200 | 4 |
| epoll-spin | 70 | 8199 | 0 |
| epoll-spin | 71 | 8196 | 4 |
| epoll-spin | 72 | 8193 | 0 |
| epoll-spin | 73 | 8197 | 0 |
| epoll-spin | 74 | 8200 | 4 |
| epoll-spin | 75 | 8194 | 4 |
| epoll-spin | 76 | 8198 | 4 |
| epoll-spin | 77 | 8198 | 4 |
| epoll-spin | 78 | 8197 | 0 |
| epoll-spin | 79 | 8196 | 4 |
| epoll-spin | 80 | 8193 | 0 |
| epoll-spin | 81 | 8198 | 4 |
| epoll-spin | 82 | 8194 | 4 |
| epoll-spin | 83 | 8195 | 0 |
| epoll-spin | 84 | 8200 | 4 |
| epoll-spin | 85 | 8193 | 0 |
| epoll-spin | 86 | 8194 | 4 |
| epoll-spin | 87 | 8197 | 0 |
| epoll-spin | 88 | 8193 | 0 |
| epoll-spin | 89 | 8199 | 0 |
| epoll-spin | 90 | 8198 | 4 |
| epoll-spin | 91 | 8196 | 4 |
| epoll-spin | 92 | 8194 | 4 |
| epoll-spin | 93 | 8200 | 4 |
| epoll-spin | 94 | 8196 | 4 |
| epoll-spin | 95 | 8200 | 4 |
| epoll-spin | 96 | 8199 | 0 |
| epoll-spin | 97 | 8195 | 0 |
| epoll-spin | 98 | 8194 | 4 |
| epoll-spin | 99 | 8193 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1017912224 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1664957 of 4387092 in 1-msg reads) | 1664957 | 4.73 | 3.72 | 7.75 | 15.26 | 24.76 | 36.91 | 72.94 |

#### Stalls: m2/lab-epoll-spin-c100, loop 0 on CPU 1

178949632 iterations, longest 1824.7 µs; over 5 / 20 / 100 µs: 269122 / 968 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 652 | 16984.0 | 60.3 |
| loop work | 315 | 10706.5 | 1824.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 39.2 | 39.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1824.7 | 4334 | loop work (4334 items) |  |
| 2 | 688.7 | 1739 | loop work (1739 items) |  |
| 3 | 60.3 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 57.9 | 9 | loop work (9 items) |  |
| 5 | 56.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 55.4 | 3 | loop work (3 items) |  |
| 7 | 50.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 48.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 45.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 45.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m2/lab-mio-c1: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m2/lab-mio-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 355379 | 0 | 0.00 | 267777514 | 753.50 | 4462959 | 4458714 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| mio | 0 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|

#### Stalls: m2/lab-mio-c1, loop 0 on CPU 1

267522835 iterations, longest 6607.5 µs; over 5 / 20 / 100 µs: 38874 / 91 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 79 | 8674.8 | 6607.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 10 | 231.3 | 28.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.8 | 35.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 6607.5 | 461 | loop work (461 items) |  |
| 2 | 43.7 | 10 | loop work (10 items) |  |
| 3 | 38.4 | 10 | loop work (10 items) |  |
| 4 | 38.1 | 10 | loop work (10 items) |  |
| 5 | 37.5 | 10 | loop work (10 items) |  |
| 6 | 37.4 | 11 | loop work (11 items) |  |
| 7 | 37.4 | 10 | loop work (10 items) |  |
| 8 | 36.7 | 10 | loop work (10 items) |  |
| 9 | 35.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 34.8 | 10 | loop work (10 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| LOC Local timer interrupts | 8 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |

## m2/lab-mio-c10: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m2/lab-mio-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 3163034 | 0 | 0.00 | 253803308 | 80.24 | 4230055 | 4203915 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| mio | 0 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|

#### Stalls: m2/lab-mio-c10, loop 0 on CPU 1

252234921 iterations, longest 8528.9 µs; over 5 / 20 / 100 µs: 276665 / 1123 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1096 | 34282.8 | 8528.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 26 | 744.0 | 141.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.4 | 33.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 8528.9 | 4228 | loop work (4228 items) |  |
| 2 | 141.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 60.6 | 7 | loop work (7 items) |  |
| 4 | 48.5 | 3 | loop work (3 items) |  |
| 5 | 47.1 | 3 | loop work (3 items) |  |
| 6 | 47.0 | 93 | loop work (93 items) |  |
| 7 | 45.9 | 3 | loop work (3 items) |  |
| 8 | 43.1 | 10 | loop work (10 items) |  |
| 9 | 42.8 | 3 | loop work (3 items) |  |
| 10 | 41.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 33 |
| LOC Local timer interrupts | 16 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## m2/lab-mio-c100: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m2/lab-mio-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 4544391 | 0 | 0.00 | 242296547 | 53.32 | 4038276 | 3965700 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| mio | 0 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|

#### Stalls: m2/lab-mio-c100, loop 0 on CPU 1

237941978 iterations, longest 9993.6 µs; over 5 / 20 / 100 µs: 343235 / 847 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 392 | 20464.5 | 9993.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 453 | 12027.5 | 78.2 |
| softirq TIMER | 1 | 284.3 | 284.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.8 | 35.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 9993.6 | 6216 | loop work (6216 items) |  |
| 2 | 284.3 | 8 | softirq TIMER | softirq TIMER 243, local timer tick 15, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 3 | 148.3 | 151 | loop work (151 items) |  |
| 4 | 78.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 60.5 | 9 | loop work (9 items) |  |
| 6 | 56.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 54.7 | 10 | loop work (10 items) |  |
| 8 | 50.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 50.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 50.0 | 10 | loop work (10 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 304 |
| LOC Local timer interrupts | 9 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

## m2/lab-recv-busy-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=113029 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355391 | 126830 | 2.80 | 110400576 | 310.65 | 1840010 | 1840010 | 99.9 | 64.8 | true | 0 | 234 | 180.1 | 37480 / 1382 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79607584 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (63173 of 343094 in 1-msg reads) | 63173 | 3.37 | 3.03 | 4.67 | 6.56 | 9.71 | 30.64 | 37.54 |

#### Stalls: m2/lab-recv-busy-c1, loop 0 on CPU 1

110400576 iterations, longest 180.1 µs; over 5 / 20 / 100 µs: 37480 / 1382 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 762 | 23711.2 | 180.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 397 | 10562.9 | 42.0 |
| mostly unexplained (+preempted by kworker/1:1) | 108 | 3111.9 | 44.5 |
| mostly unexplained (+softirq NET_RX) | 71 | 2014.8 | 38.3 |
| softirq NET_RX | 41 | 1180.4 | 38.1 |
| mostly unexplained (+local timer tick) | 2 | 78.8 | 40.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.7 | 35.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 180.1 | 472 | loop work (472 items) |  |
| 2 | 77.1 | 10 | loop work (10 items) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 62.4 | 7 | loop work (7 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 53.2 | 7 | loop work (7 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 52.1 | 10 | loop work (10 items) | preempted by kworker/1:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 48.6 | 9 | loop work (9 items) | softirq NET_RX 3 |
| 7 | 44.5 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 10, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 8 | 44.4 | 8 | loop work (8 items) |  |
| 9 | 44.1 | 3 | loop work (3 items) | softirq NET_RX 18, preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 44.0 | 10 | loop work (10 items) | softirq NET_RX 3 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| LOC Local timer interrupts | 243 |
| IWI IRQ work interrupts | 240 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=738947 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3163050 | 774338 | 4.08 | 106000000 | 33.51 | 1766667 | 176667 | 99.3 | 68.0 | true | 0 | 1237 | 1316.2 | 10424419 / 61228 / 16 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 1 |
| recv-busy | 1 | 8194 | 1 |
| recv-busy | 2 | 8196 | 1 |
| recv-busy | 3 | 8195 | 1 |
| recv-busy | 4 | 8198 | 1 |
| recv-busy | 5 | 8197 | 1 |
| recv-busy | 6 | 8194 | 1 |
| recv-busy | 7 | 8197 | 1 |
| recv-busy | 8 | 8195 | 0 |
| recv-busy | 9 | 8196 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708523200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (209761 of 3053557 in 1-msg reads) | 209761 | 5.48 | 4.88 | 8.49 | 18.39 | 36.75 | 48.05 | 71.66 |

#### Stalls: m2/lab-recv-busy-c10, loop 0 on CPU 1

10600000 iterations, longest 1316.2 µs; over 5 / 20 / 100 µs: 10424419 / 61228 / 16

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 59392 | 1608143.7 | 1316.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1229 | 41133.9 | 129.1 |
| mostly unexplained (+preempted by kworker/1:1) | 460 | 16774.5 | 69.4 |
| mostly unexplained (+softirq NET_RX) | 81 | 2764.9 | 56.1 |
| mostly unexplained (+local timer tick) | 38 | 979.3 | 52.6 |
| mostly unexplained (+irq work (IPI/vector)) | 21 | 532.4 | 35.7 |
| softirq NET_RX | 4 | 182.7 | 59.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 49.7 | 49.7 |
| mostly unexplained (+ipi_send_cpu) | 2 | 43.8 | 22.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1316.2 | 4249 | loop work (4249 items) |  |
| 2 | 129.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 127.2 | 39 | loop work (39 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 109.1 | 68 | loop work (68 items) | softirq NET_RX 19, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 107.9 | 36 | loop work (36 items) | softirq NET_RX 1 |
| 6 | 106.7 | 31 | loop work (31 items) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 105.8 | 26 | loop work (26 items) |  |
| 8 | 105.0 | 29 | loop work (29 items) | softirq NET_RX 5 |
| 9 | 104.6 | 32 | loop work (32 items) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 103.1 | 45 | loop work (45 items) | softirq NET_RX 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1266 |
| LOC Local timer interrupts | 1236 |
| CAL Function call interrupts | 991 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1544056 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4544419 | 1919111 | 2.37 | 95334400 | 20.98 | 1588907 | 15889 | 98.0 | 70.1 | true | 0 | 4827 | 2357.1 | 953343 / 953343 / 31368 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 1 |
| recv-busy | 1 | 8198 | 4 |
| recv-busy | 2 | 8196 | 4 |
| recv-busy | 3 | 8200 | 1 |
| recv-busy | 4 | 8196 | 1 |
| recv-busy | 5 | 8195 | 0 |
| recv-busy | 6 | 8193 | 0 |
| recv-busy | 7 | 8195 | 1 |
| recv-busy | 8 | 8194 | 1 |
| recv-busy | 9 | 8199 | 0 |
| recv-busy | 10 | 8194 | 1 |
| recv-busy | 11 | 8198 | 4 |
| recv-busy | 12 | 8200 | 4 |
| recv-busy | 13 | 8193 | 1 |
| recv-busy | 14 | 8195 | 1 |
| recv-busy | 15 | 8194 | 1 |
| recv-busy | 16 | 8196 | 4 |
| recv-busy | 17 | 8199 | 0 |
| recv-busy | 18 | 8199 | 1 |
| recv-busy | 19 | 8194 | 1 |
| recv-busy | 20 | 8200 | 1 |
| recv-busy | 21 | 8195 | 1 |
| recv-busy | 22 | 8194 | 4 |
| recv-busy | 23 | 8195 | 1 |
| recv-busy | 24 | 8193 | 1 |
| recv-busy | 25 | 8198 | 1 |
| recv-busy | 26 | 8200 | 1 |
| recv-busy | 27 | 8196 | 1 |
| recv-busy | 28 | 8197 | 0 |
| recv-busy | 29 | 8194 | 4 |
| recv-busy | 30 | 8200 | 4 |
| recv-busy | 31 | 8195 | 1 |
| recv-busy | 32 | 8197 | 1 |
| recv-busy | 33 | 8193 | 1 |
| recv-busy | 34 | 8194 | 1 |
| recv-busy | 35 | 8196 | 4 |
| recv-busy | 36 | 8198 | 4 |
| recv-busy | 37 | 8200 | 1 |
| recv-busy | 38 | 8199 | 1 |
| recv-busy | 39 | 8194 | 1 |
| recv-busy | 40 | 8200 | 1 |
| recv-busy | 41 | 8194 | 1 |
| recv-busy | 42 | 8193 | 0 |
| recv-busy | 43 | 8196 | 4 |
| recv-busy | 44 | 8197 | 0 |
| recv-busy | 45 | 8200 | 1 |
| recv-busy | 46 | 8193 | 0 |
| recv-busy | 47 | 8195 | 1 |
| recv-busy | 48 | 8194 | 4 |
| recv-busy | 49 | 8200 | 1 |
| recv-busy | 50 | 8198 | 1 |
| recv-busy | 51 | 8194 | 4 |
| recv-busy | 52 | 8200 | 1 |
| recv-busy | 53 | 8195 | 1 |
| recv-busy | 54 | 8196 | 1 |
| recv-busy | 55 | 8198 | 4 |
| recv-busy | 56 | 8200 | 1 |
| recv-busy | 57 | 8194 | 1 |
| recv-busy | 58 | 8196 | 1 |
| recv-busy | 59 | 8197 | 0 |
| recv-busy | 60 | 8198 | 1 |
| recv-busy | 61 | 8199 | 0 |
| recv-busy | 62 | 8198 | 1 |
| recv-busy | 63 | 8195 | 0 |
| recv-busy | 64 | 8193 | 1 |
| recv-busy | 65 | 8193 | 1 |
| recv-busy | 66 | 8198 | 1 |
| recv-busy | 67 | 8195 | 1 |
| recv-busy | 68 | 8199 | 1 |
| recv-busy | 69 | 8193 | 1 |
| recv-busy | 70 | 8200 | 1 |
| recv-busy | 71 | 8193 | 0 |
| recv-busy | 72 | 8196 | 1 |
| recv-busy | 73 | 8194 | 1 |
| recv-busy | 74 | 8194 | 1 |
| recv-busy | 75 | 8199 | 1 |
| recv-busy | 76 | 8193 | 1 |
| recv-busy | 77 | 8194 | 1 |
| recv-busy | 78 | 8198 | 1 |
| recv-busy | 79 | 8197 | 1 |
| recv-busy | 80 | 8197 | 0 |
| recv-busy | 81 | 8196 | 4 |
| recv-busy | 82 | 8194 | 4 |
| recv-busy | 83 | 8196 | 1 |
| recv-busy | 84 | 8197 | 1 |
| recv-busy | 85 | 8193 | 1 |
| recv-busy | 86 | 8196 | 4 |
| recv-busy | 87 | 8198 | 1 |
| recv-busy | 88 | 8196 | 1 |
| recv-busy | 89 | 8197 | 1 |
| recv-busy | 90 | 8195 | 1 |
| recv-busy | 91 | 8197 | 0 |
| recv-busy | 92 | 8198 | 4 |
| recv-busy | 93 | 8193 | 0 |
| recv-busy | 94 | 8199 | 0 |
| recv-busy | 95 | 8200 | 1 |
| recv-busy | 96 | 8197 | 0 |
| recv-busy | 97 | 8193 | 1 |
| recv-busy | 98 | 8199 | 0 |
| recv-busy | 99 | 8193 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1017949856 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1489316 of 4387259 in 1-msg reads) | 1489316 | 32.44 | 31.67 | 57.73 | 84.46 | 112.50 | 145.38 | 200.85 |

#### Stalls: m2/lab-recv-busy-c100, loop 0 on CPU 1

953344 iterations, longest 2357.1 µs; over 5 / 20 / 100 µs: 953343 / 953343 / 31368

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 752937 | 44613118.0 | 227.0 |
| loop work | 170002 | 13244273.3 | 2357.1 |
| mostly unexplained (+softirq NET_RX) | 23162 | 1496431.7 | 159.8 |
| mostly unexplained (+local timer tick) | 4226 | 304488.9 | 163.0 |
| mostly unexplained (+preempted by kworker/1:1) | 2775 | 238117.8 | 147.2 |
| mostly unexplained (+irq work (IPI/vector)) | 211 | 15456.3 | 110.7 |
| mostly unexplained (+softirq NET_TX) | 11 | 706.5 | 81.8 |
| mostly unexplained (+call function (IPI/vector)) | 10 | 646.0 | 68.0 |
| mostly unexplained (+ipi_send_cpu) | 7 | 500.5 | 93.4 |
| mostly unexplained (+call function single (IPI/vector)) | 1 | 72.9 | 72.9 |
| mostly unexplained (+softirq RCU) | 1 | 65.1 | 65.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2357.1 | 6014 | loop work (6014 items) |  |
| 2 | 227.0 | 7 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 221.6 | 55 | loop work (55 items) | softirq NET_RX 1 |
| 4 | 215.6 | 95 | loop work (95 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 215.1 | 70 | loop work (70 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, softirq NET_RX 1, workqueue [ena] 1 |
| 6 | 213.3 | 85 | loop work (85 items) | softirq NET_RX 2 |
| 7 | 203.8 | 73 | loop work (73 items) | softirq NET_RX 6 |
| 8 | 202.6 | 74 | loop work (74 items) | softirq NET_RX 4 |
| 9 | 200.3 | 82 | loop work (82 items) | softirq NET_RX 2 |
| 10 | 200.0 | 55 | loop work (55 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4869 |
| LOC Local timer interrupts | 4720 |
| CAL Function call interrupts | 992 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=115904 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355380 | 127176 | 2.79 | 153845504 | 432.90 | 2564092 | 1282046 | 99.8 | 56.6 | true | 0 | 148 | 184.8 | 45053 / 1350 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79605120 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (63038 of 343084 in 1-msg reads) | 63038 | 3.27 | 2.92 | 4.64 | 6.65 | 24.39 | 30.91 | 36.53 |

#### Stalls: m2/lab-recv-busy-pq-c1, loop 0 on CPU 1

76922752 iterations, longest 184.8 µs; over 5 / 20 / 100 µs: 45053 / 1350 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 805 | 24033.9 | 184.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 438 | 11469.2 | 61.0 |
| mostly unexplained (+preempted by kworker/1:1) | 59 | 1738.3 | 49.6 |
| softirq NET_RX | 25 | 721.4 | 36.0 |
| mostly unexplained (+softirq NET_RX) | 20 | 546.3 | 35.6 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 64.3 | 34.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.2 | 35.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 184.8 | 462 | loop work (462 items) |  |
| 2 | 69.1 | 10 | loop work (10 items) | softirq NET_RX 20, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 61.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 55.5 | 10 | loop work (10 items) | softirq NET_RX 17, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 51.6 | 7 | loop work (7 items) | preempted by kworker/1:1 11, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 49.6 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 44.0 | 3 | loop work (3 items) | softirq NET_RX 18, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 43.6 | 3 | loop work (3 items) | softirq NET_RX 20, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 43.0 | 10 | loop work (10 items) | softirq NET_RX 3 |
| 10 | 42.5 | 11 | loop work (11 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 164 |
| LOC Local timer interrupts | 151 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=728739 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3163366 | 778215 | 4.06 | 113396451 | 35.85 | 1889941 | 269312 | 99.2 | 66.0 | true | 0 | 1363 | 1496.2 | 476002 / 43108 / 11 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 1 |
| recv-busy | 1 | 8195 | 1 |
| recv-busy | 2 | 8193 | 0 |
| recv-busy | 3 | 8199 | 1 |
| recv-busy | 4 | 8198 | 1 |
| recv-busy | 5 | 8196 | 1 |
| recv-busy | 6 | 8195 | 0 |
| recv-busy | 7 | 8196 | 1 |
| recv-busy | 8 | 8195 | 1 |
| recv-busy | 9 | 8199 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708593984 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (214023 of 3053555 in 1-msg reads) | 214023 | 5.73 | 4.91 | 9.19 | 20.17 | 38.35 | 51.51 | 74.32 |

#### Stalls: m2/lab-recv-busy-pq-c10, loop 0 on CPU 1

16158720 iterations, longest 1496.2 µs; over 5 / 20 / 100 µs: 476002 / 43108 / 11

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 41483 | 1154588.4 | 1496.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1078 | 35095.2 | 150.9 |
| mostly unexplained (+preempted by kworker/1:1) | 467 | 16997.6 | 71.9 |
| mostly unexplained (+softirq NET_RX) | 34 | 1124.2 | 42.9 |
| mostly unexplained (+irq work (IPI/vector)) | 22 | 573.3 | 50.8 |
| mostly unexplained (+local timer tick) | 14 | 451.0 | 48.2 |
| softirq NET_RX | 7 | 302.7 | 61.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 64.5 | 39.9 |
| local timer tick | 1 | 39.6 | 39.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1496.2 | 4610 | loop work (4610 items) |  |
| 2 | 150.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 121.9 | 45 | loop work (45 items) |  |
| 4 | 112.1 | 54 | loop work (54 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 111.3 | 66 | loop work (66 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 107.0 | 66 | loop work (66 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 106.1 | 69 | loop work (69 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 104.0 | 53 | loop work (53 items) |  |
| 9 | 103.7 | 53 | loop work (53 items) |  |
| 10 | 101.2 | 45 | loop work (45 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1404 |
| LOC Local timer interrupts | 1362 |
| CAL Function call interrupts | 990 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1622476 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4544253 | 2109134 | 2.15 | 98945170 | 21.77 | 1649086 | 179662 | 97.6 | 68.2 | true | 0 | 5921 | 1986.6 | 3398237 / 81571 / 19 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 0 |
| recv-busy | 1 | 8195 | 0 |
| recv-busy | 2 | 8196 | 1 |
| recv-busy | 3 | 8200 | 4 |
| recv-busy | 4 | 8199 | 1 |
| recv-busy | 5 | 8198 | 1 |
| recv-busy | 6 | 8194 | 1 |
| recv-busy | 7 | 8193 | 1 |
| recv-busy | 8 | 8195 | 1 |
| recv-busy | 9 | 8200 | 4 |
| recv-busy | 10 | 8197 | 1 |
| recv-busy | 11 | 8195 | 1 |
| recv-busy | 12 | 8200 | 1 |
| recv-busy | 13 | 8197 | 1 |
| recv-busy | 14 | 8195 | 1 |
| recv-busy | 15 | 8197 | 0 |
| recv-busy | 16 | 8200 | 1 |
| recv-busy | 17 | 8199 | 0 |
| recv-busy | 18 | 8197 | 1 |
| recv-busy | 19 | 8196 | 1 |
| recv-busy | 20 | 8193 | 0 |
| recv-busy | 21 | 8194 | 1 |
| recv-busy | 22 | 8198 | 1 |
| recv-busy | 23 | 8198 | 1 |
| recv-busy | 24 | 8200 | 1 |
| recv-busy | 25 | 8199 | 0 |
| recv-busy | 26 | 8196 | 1 |
| recv-busy | 27 | 8197 | 1 |
| recv-busy | 28 | 8195 | 0 |
| recv-busy | 29 | 8195 | 1 |
| recv-busy | 30 | 8197 | 1 |
| recv-busy | 31 | 8196 | 1 |
| recv-busy | 32 | 8197 | 0 |
| recv-busy | 33 | 8195 | 1 |
| recv-busy | 34 | 8193 | 1 |
| recv-busy | 35 | 8199 | 1 |
| recv-busy | 36 | 8194 | 1 |
| recv-busy | 37 | 8196 | 4 |
| recv-busy | 38 | 8194 | 4 |
| recv-busy | 39 | 8200 | 4 |
| recv-busy | 40 | 8199 | 1 |
| recv-busy | 41 | 8198 | 1 |
| recv-busy | 42 | 8197 | 1 |
| recv-busy | 43 | 8200 | 1 |
| recv-busy | 44 | 8199 | 1 |
| recv-busy | 45 | 8193 | 1 |
| recv-busy | 46 | 8194 | 1 |
| recv-busy | 47 | 8195 | 0 |
| recv-busy | 48 | 8197 | 1 |
| recv-busy | 49 | 8200 | 1 |
| recv-busy | 50 | 8197 | 1 |
| recv-busy | 51 | 8195 | 0 |
| recv-busy | 52 | 8199 | 1 |
| recv-busy | 53 | 8196 | 1 |
| recv-busy | 54 | 8200 | 1 |
| recv-busy | 55 | 8194 | 1 |
| recv-busy | 56 | 8197 | 0 |
| recv-busy | 57 | 8195 | 1 |
| recv-busy | 58 | 8200 | 1 |
| recv-busy | 59 | 8199 | 1 |
| recv-busy | 60 | 8194 | 1 |
| recv-busy | 61 | 8199 | 1 |
| recv-busy | 62 | 8196 | 1 |
| recv-busy | 63 | 8200 | 4 |
| recv-busy | 64 | 8194 | 1 |
| recv-busy | 65 | 8199 | 0 |
| recv-busy | 66 | 8196 | 1 |
| recv-busy | 67 | 8195 | 0 |
| recv-busy | 68 | 8194 | 1 |
| recv-busy | 69 | 8196 | 1 |
| recv-busy | 70 | 8194 | 1 |
| recv-busy | 71 | 8199 | 1 |
| recv-busy | 72 | 8193 | 1 |
| recv-busy | 73 | 8195 | 1 |
| recv-busy | 74 | 8196 | 1 |
| recv-busy | 75 | 8195 | 0 |
| recv-busy | 76 | 8196 | 1 |
| recv-busy | 77 | 8198 | 1 |
| recv-busy | 78 | 8200 | 1 |
| recv-busy | 79 | 8198 | 4 |
| recv-busy | 80 | 8193 | 1 |
| recv-busy | 81 | 8198 | 4 |
| recv-busy | 82 | 8200 | 1 |
| recv-busy | 83 | 8194 | 1 |
| recv-busy | 84 | 8199 | 0 |
| recv-busy | 85 | 8195 | 1 |
| recv-busy | 86 | 8199 | 1 |
| recv-busy | 87 | 8200 | 1 |
| recv-busy | 88 | 8197 | 1 |
| recv-busy | 89 | 8196 | 1 |
| recv-busy | 90 | 8198 | 4 |
| recv-busy | 91 | 8199 | 1 |
| recv-busy | 92 | 8199 | 1 |
| recv-busy | 93 | 8193 | 1 |
| recv-busy | 94 | 8195 | 0 |
| recv-busy | 95 | 8196 | 1 |
| recv-busy | 96 | 8194 | 1 |
| recv-busy | 97 | 8196 | 4 |
| recv-busy | 98 | 8200 | 1 |
| recv-busy | 99 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1017912672 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1521970 of 4387092 in 1-msg reads) | 1521970 | 7.21 | 6.50 | 10.46 | 25.15 | 39.66 | 54.91 | 85.70 |

#### Stalls: m2/lab-recv-busy-pq-c100, loop 0 on CPU 1

10779712 iterations, longest 1986.6 µs; over 5 / 20 / 100 µs: 3398237 / 81571 / 19

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 73754 | 2117473.1 | 1986.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4776 | 157980.5 | 142.8 |
| mostly unexplained (+preempted by kworker/1:1) | 2494 | 73925.2 | 81.8 |
| mostly unexplained (+local timer tick) | 277 | 6632.9 | 66.0 |
| mostly unexplained (+softirq NET_RX) | 163 | 5432.7 | 51.0 |
| mostly unexplained (+irq work (IPI/vector)) | 98 | 2258.6 | 50.9 |
| softirq NET_RX | 6 | 225.9 | 49.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 49.4 | 49.4 |
| mostly unexplained (+softirq RCU) | 1 | 36.0 | 36.0 |
| mostly unexplained (+ipi_send_cpu) | 1 | 27.7 | 27.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1986.6 | 4770 | loop work (4770 items) |  |
| 2 | 539.1 | 1308 | loop work (1308 items) |  |
| 3 | 142.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 121.4 | 82 | loop work (82 items) |  |
| 5 | 121.2 | 80 | loop work (80 items) |  |
| 6 | 115.7 | 68 | loop work (68 items) |  |
| 7 | 111.5 | 85 | loop work (85 items) | softirq NET_RX 1 |
| 8 | 107.5 | 77 | loop work (77 items) |  |
| 9 | 107.4 | 84 | loop work (84 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 107.3 | 48 | loop work (48 items) | preempted by kworker/1:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5968 |
| LOC Local timer interrupts | 5843 |
| CAL Function call interrupts | 990 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-uring-spin-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 355391 | 122467 | 2.90 | 213940 | 0.60 | 3566 | 50607235 | 0.0 | 0.9 | false | 0 | 0 | 107.7 | 51407 / 11 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8196 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 79607584 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-spin (79164 of 343094 in 1-msg reads) | 79164 | 10.79 | 12.60 | 17.40 | 19.11 | 21.64 | 28.26 | 46.65 |

#### Stalls: m2/lab-uring-spin-c1, loop 0 on CPU 1

3036434112 iterations, longest 107.7 µs; over 5 / 20 / 100 µs: 51407 / 11 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 265.6 | 107.7 |
| loop work | 1 | 81.2 | 81.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 60.3 | 38.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 107.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 81.2 | 472 | loop work (472 items) |  |
| 3 | 38.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 25.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 23.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 22.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 22.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 21.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 21.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-spin-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-uring-spin-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 3163049 | 913615 | 3.46 | 1110273 | 0.35 | 18505 | 31510993 | 0.0 | 4.3 | false | 0 | 0 | 671.1 | 177232 / 406 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8197 | 0 |
| uring-spin | 1 | 8200 | 4 |
| uring-spin | 2 | 8200 | 4 |
| uring-spin | 3 | 8199 | 0 |
| uring-spin | 4 | 8195 | 0 |
| uring-spin | 5 | 8200 | 4 |
| uring-spin | 6 | 8194 | 4 |
| uring-spin | 7 | 8197 | 0 |
| uring-spin | 8 | 8200 | 4 |
| uring-spin | 9 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 708522976 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-spin (346870 of 3053555 in 1-msg reads) | 346870 | 7.19 | 6.16 | 12.04 | 22.18 | 32.36 | 45.22 | 72.58 |

#### Stalls: m2/lab-uring-spin-c10, loop 0 on CPU 1

1890659584 iterations, longest 671.1 µs; over 5 / 20 / 100 µs: 177232 / 406 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 214 | 6145.5 | 671.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 190 | 5737.6 | 665.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 57.0 | 35.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 671.1 | 4219 | loop work (4219 items) |  |
| 2 | 665.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 67.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 64.0 | 14 | loop work (14 items) |  |
| 5 | 53.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 43.4 | 7 | loop work (7 items) |  |
| 7 | 42.3 | 7 | loop work (7 items) |  |
| 8 | 42.2 | 7 | loop work (7 items) |  |
| 9 | 41.8 | 9 | loop work (9 items) |  |
| 10 | 41.5 | 7 | loop work (7 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-spin-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-uring-spin-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 4544241 | 2285884 | 1.99 | 3569165 | 0.79 | 59486 | 6872571 | 0.0 | 10.4 | false | 0 | 0 | 1316.4 | 275889 / 1058 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8196 | 4 |
| uring-spin | 1 | 8199 | 0 |
| uring-spin | 2 | 8196 | 4 |
| uring-spin | 3 | 8195 | 0 |
| uring-spin | 4 | 8200 | 4 |
| uring-spin | 5 | 8197 | 0 |
| uring-spin | 6 | 8195 | 0 |
| uring-spin | 7 | 8195 | 0 |
| uring-spin | 8 | 8197 | 0 |
| uring-spin | 9 | 8194 | 4 |
| uring-spin | 10 | 8199 | 0 |
| uring-spin | 11 | 8198 | 4 |
| uring-spin | 12 | 8194 | 4 |
| uring-spin | 13 | 8193 | 0 |
| uring-spin | 14 | 8194 | 4 |
| uring-spin | 15 | 8197 | 0 |
| uring-spin | 16 | 8194 | 4 |
| uring-spin | 17 | 8198 | 4 |
| uring-spin | 18 | 8197 | 0 |
| uring-spin | 19 | 8200 | 4 |
| uring-spin | 20 | 8199 | 0 |
| uring-spin | 21 | 8200 | 4 |
| uring-spin | 22 | 8193 | 0 |
| uring-spin | 23 | 8195 | 0 |
| uring-spin | 24 | 8198 | 4 |
| uring-spin | 25 | 8195 | 0 |
| uring-spin | 26 | 8197 | 0 |
| uring-spin | 27 | 8197 | 0 |
| uring-spin | 28 | 8194 | 4 |
| uring-spin | 29 | 8193 | 0 |
| uring-spin | 30 | 8199 | 0 |
| uring-spin | 31 | 8195 | 0 |
| uring-spin | 32 | 8194 | 4 |
| uring-spin | 33 | 8198 | 4 |
| uring-spin | 34 | 8195 | 0 |
| uring-spin | 35 | 8197 | 0 |
| uring-spin | 36 | 8193 | 0 |
| uring-spin | 37 | 8195 | 0 |
| uring-spin | 38 | 8197 | 0 |
| uring-spin | 39 | 8193 | 0 |
| uring-spin | 40 | 8200 | 4 |
| uring-spin | 41 | 8194 | 4 |
| uring-spin | 42 | 8197 | 0 |
| uring-spin | 43 | 8194 | 4 |
| uring-spin | 44 | 8196 | 4 |
| uring-spin | 45 | 8193 | 0 |
| uring-spin | 46 | 8199 | 0 |
| uring-spin | 47 | 8195 | 0 |
| uring-spin | 48 | 8200 | 4 |
| uring-spin | 49 | 8200 | 4 |
| uring-spin | 50 | 8196 | 4 |
| uring-spin | 51 | 8193 | 0 |
| uring-spin | 52 | 8197 | 0 |
| uring-spin | 53 | 8195 | 0 |
| uring-spin | 54 | 8199 | 0 |
| uring-spin | 55 | 8193 | 0 |
| uring-spin | 56 | 8193 | 0 |
| uring-spin | 57 | 8200 | 4 |
| uring-spin | 58 | 8199 | 0 |
| uring-spin | 59 | 8200 | 4 |
| uring-spin | 60 | 8196 | 4 |
| uring-spin | 61 | 8195 | 0 |
| uring-spin | 62 | 8197 | 0 |
| uring-spin | 63 | 8196 | 4 |
| uring-spin | 64 | 8198 | 4 |
| uring-spin | 65 | 8196 | 4 |
| uring-spin | 66 | 8199 | 0 |
| uring-spin | 67 | 8197 | 0 |
| uring-spin | 68 | 8199 | 0 |
| uring-spin | 69 | 8194 | 4 |
| uring-spin | 70 | 8197 | 0 |
| uring-spin | 71 | 8193 | 0 |
| uring-spin | 72 | 8195 | 0 |
| uring-spin | 73 | 8198 | 4 |
| uring-spin | 74 | 8195 | 0 |
| uring-spin | 75 | 8200 | 4 |
| uring-spin | 76 | 8193 | 0 |
| uring-spin | 77 | 8193 | 0 |
| uring-spin | 78 | 8193 | 0 |
| uring-spin | 79 | 8199 | 0 |
| uring-spin | 80 | 8199 | 0 |
| uring-spin | 81 | 8197 | 0 |
| uring-spin | 82 | 8196 | 4 |
| uring-spin | 83 | 8195 | 0 |
| uring-spin | 84 | 8199 | 0 |
| uring-spin | 85 | 8200 | 4 |
| uring-spin | 86 | 8197 | 0 |
| uring-spin | 87 | 8198 | 4 |
| uring-spin | 88 | 8197 | 0 |
| uring-spin | 89 | 8195 | 0 |
| uring-spin | 90 | 8194 | 4 |
| uring-spin | 91 | 8197 | 0 |
| uring-spin | 92 | 8197 | 0 |
| uring-spin | 93 | 8200 | 4 |
| uring-spin | 94 | 8197 | 0 |
| uring-spin | 95 | 8198 | 4 |
| uring-spin | 96 | 8196 | 4 |
| uring-spin | 97 | 8195 | 0 |
| uring-spin | 98 | 8196 | 4 |
| uring-spin | 99 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 1017909984 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-spin (1683317 of 4386984 in 1-msg reads) | 1683317 | 5.22 | 4.00 | 8.47 | 17.81 | 27.53 | 37.22 | 147.18 |

#### Stalls: m2/lab-uring-spin-c100, loop 0 on CPU 1

412354240 iterations, longest 1316.4 µs; over 5 / 20 / 100 µs: 275889 / 1058 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 598 | 17196.5 | 1316.4 |
| loop work | 458 | 11828.5 | 677.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 64.3 | 36.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1316.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 677.4 | 3693 | loop work (3693 items) |  |
| 3 | 424.1 | 2333 | loop work (2333 items) |  |
| 4 | 198.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 59.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 57.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 56.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 53.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 49.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 48.9 | 8 | loop work (8 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=110329 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355391 | 126080 | 2.82 | 1 | 0.00 | 0 | 31644921 | 0.0 | 0.0 | true | 0 | 0 | 101.9 | 222 / 3 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8196 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79607584 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (63354 of 343094 in 1-msg reads) | 63354 | 4.07 | 3.72 | 5.38 | 8.14 | 11.89 | 25.51 | 36.50 |

#### Stalls: m2/lab-uring-sqpoll-c1, loop 0 on CPU 1

1898695232 iterations, longest 101.9 µs; over 5 / 20 / 100 µs: 222 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 101.9 | 101.9 |
| loop work | 1 | 79.9 | 79.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 37.8 | 37.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 101.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 79.9 | 473 | loop work (473 items) |  |
| 3 | 37.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=785669 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3163153 | 826211 | 3.83 | 1 | 0.00 | 0 | 29456588 | 0.0 | 0.0 | true | 0 | 0 | 710.3 | 3656 / 6 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8195 | 0 |
| uring-sqpoll | 1 | 8193 | 2 |
| uring-sqpoll | 2 | 8200 | 2 |
| uring-sqpoll | 3 | 8194 | 2 |
| uring-sqpoll | 4 | 8195 | 2 |
| uring-sqpoll | 5 | 8200 | 2 |
| uring-sqpoll | 6 | 8197 | 2 |
| uring-sqpoll | 7 | 8193 | 2 |
| uring-sqpoll | 8 | 8194 | 2 |
| uring-sqpoll | 9 | 8196 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708546272 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (237165 of 3053556 in 1-msg reads) | 237165 | 6.39 | 5.67 | 9.36 | 17.33 | 34.88 | 50.99 | 77.50 |

#### Stalls: m2/lab-uring-sqpoll-c10, loop 0 on CPU 1

1767395264 iterations, longest 710.3 µs; over 5 / 20 / 100 µs: 3656 / 6 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 710.3 | 710.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 62.1 | 21.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 60.9 | 35.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 710.3 | 4322 | loop work (4322 items) |  |
| 2 | 35.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 25.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 21.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 20.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 20.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1001 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1981229 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4544393 | 2197304 | 2.07 | 1 | 0.00 | 0 | 7499050 | 0.0 | 0.0 | true | 0 | 0 | 646.8 | 6636 / 8 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8196 | 2 |
| uring-sqpoll | 1 | 8194 | 2 |
| uring-sqpoll | 2 | 8193 | 2 |
| uring-sqpoll | 3 | 8197 | 2 |
| uring-sqpoll | 4 | 8194 | 2 |
| uring-sqpoll | 5 | 8195 | 2 |
| uring-sqpoll | 6 | 8200 | 2 |
| uring-sqpoll | 7 | 8195 | 2 |
| uring-sqpoll | 8 | 8198 | 2 |
| uring-sqpoll | 9 | 8197 | 2 |
| uring-sqpoll | 10 | 8194 | 2 |
| uring-sqpoll | 11 | 8197 | 2 |
| uring-sqpoll | 12 | 8195 | 2 |
| uring-sqpoll | 13 | 8193 | 2 |
| uring-sqpoll | 14 | 8196 | 2 |
| uring-sqpoll | 15 | 8198 | 2 |
| uring-sqpoll | 16 | 8194 | 2 |
| uring-sqpoll | 17 | 8197 | 2 |
| uring-sqpoll | 18 | 8195 | 2 |
| uring-sqpoll | 19 | 8193 | 2 |
| uring-sqpoll | 20 | 8196 | 2 |
| uring-sqpoll | 21 | 8198 | 2 |
| uring-sqpoll | 22 | 8197 | 0 |
| uring-sqpoll | 23 | 8199 | 2 |
| uring-sqpoll | 24 | 8200 | 2 |
| uring-sqpoll | 25 | 8196 | 2 |
| uring-sqpoll | 26 | 8195 | 2 |
| uring-sqpoll | 27 | 8195 | 2 |
| uring-sqpoll | 28 | 8197 | 2 |
| uring-sqpoll | 29 | 8200 | 2 |
| uring-sqpoll | 30 | 8196 | 2 |
| uring-sqpoll | 31 | 8198 | 2 |
| uring-sqpoll | 32 | 8193 | 2 |
| uring-sqpoll | 33 | 8196 | 2 |
| uring-sqpoll | 34 | 8195 | 2 |
| uring-sqpoll | 35 | 8197 | 2 |
| uring-sqpoll | 36 | 8195 | 2 |
| uring-sqpoll | 37 | 8200 | 2 |
| uring-sqpoll | 38 | 8198 | 2 |
| uring-sqpoll | 39 | 8195 | 2 |
| uring-sqpoll | 40 | 8200 | 2 |
| uring-sqpoll | 41 | 8194 | 2 |
| uring-sqpoll | 42 | 8199 | 2 |
| uring-sqpoll | 43 | 8200 | 2 |
| uring-sqpoll | 44 | 8200 | 2 |
| uring-sqpoll | 45 | 8199 | 2 |
| uring-sqpoll | 46 | 8195 | 2 |
| uring-sqpoll | 47 | 8193 | 2 |
| uring-sqpoll | 48 | 8194 | 2 |
| uring-sqpoll | 49 | 8198 | 2 |
| uring-sqpoll | 50 | 8198 | 2 |
| uring-sqpoll | 51 | 8200 | 2 |
| uring-sqpoll | 52 | 8197 | 2 |
| uring-sqpoll | 53 | 8199 | 2 |
| uring-sqpoll | 54 | 8200 | 2 |
| uring-sqpoll | 55 | 8195 | 2 |
| uring-sqpoll | 56 | 8197 | 2 |
| uring-sqpoll | 57 | 8199 | 2 |
| uring-sqpoll | 58 | 8198 | 2 |
| uring-sqpoll | 59 | 8193 | 2 |
| uring-sqpoll | 60 | 8199 | 2 |
| uring-sqpoll | 61 | 8196 | 2 |
| uring-sqpoll | 62 | 8198 | 2 |
| uring-sqpoll | 63 | 8193 | 2 |
| uring-sqpoll | 64 | 8194 | 2 |
| uring-sqpoll | 65 | 8198 | 2 |
| uring-sqpoll | 66 | 8194 | 2 |
| uring-sqpoll | 67 | 8199 | 2 |
| uring-sqpoll | 68 | 8195 | 2 |
| uring-sqpoll | 69 | 8193 | 2 |
| uring-sqpoll | 70 | 8193 | 2 |
| uring-sqpoll | 71 | 8199 | 2 |
| uring-sqpoll | 72 | 8199 | 2 |
| uring-sqpoll | 73 | 8200 | 2 |
| uring-sqpoll | 74 | 8197 | 2 |
| uring-sqpoll | 75 | 8199 | 2 |
| uring-sqpoll | 76 | 8196 | 2 |
| uring-sqpoll | 77 | 8195 | 2 |
| uring-sqpoll | 78 | 8194 | 2 |
| uring-sqpoll | 79 | 8194 | 2 |
| uring-sqpoll | 80 | 8193 | 2 |
| uring-sqpoll | 81 | 8198 | 2 |
| uring-sqpoll | 82 | 8200 | 2 |
| uring-sqpoll | 83 | 8195 | 2 |
| uring-sqpoll | 84 | 8200 | 2 |
| uring-sqpoll | 85 | 8193 | 2 |
| uring-sqpoll | 86 | 8195 | 2 |
| uring-sqpoll | 87 | 8198 | 2 |
| uring-sqpoll | 88 | 8200 | 2 |
| uring-sqpoll | 89 | 8200 | 2 |
| uring-sqpoll | 90 | 8196 | 2 |
| uring-sqpoll | 91 | 8198 | 2 |
| uring-sqpoll | 92 | 8194 | 2 |
| uring-sqpoll | 93 | 8200 | 2 |
| uring-sqpoll | 94 | 8199 | 2 |
| uring-sqpoll | 95 | 8194 | 2 |
| uring-sqpoll | 96 | 8196 | 2 |
| uring-sqpoll | 97 | 8198 | 2 |
| uring-sqpoll | 98 | 8195 | 2 |
| uring-sqpoll | 99 | 8200 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1017944032 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1568418 of 4386938 in 1-msg reads) | 1568418 | 5.09 | 4.28 | 7.54 | 17.50 | 33.68 | 47.77 | 200.60 |

#### Stalls: m2/lab-uring-sqpoll-c100, loop 0 on CPU 1

449942976 iterations, longest 646.8 µs; over 5 / 20 / 100 µs: 6636 / 8 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1051.9 | 646.8 |
| softirq TIMER | 1 | 278.2 | 278.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 117.6 | 95.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 82.8 | 37.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 646.8 | 3814 | loop work (3814 items) |  |
| 2 | 405.1 | 2412 | loop work (2412 items) |  |
| 3 | 278.2 | 8 | softirq TIMER | softirq TIMER 245, local timer tick 16, timer tick_nohz_handler 9, irq work (IPI/vector) 1 |
| 4 | 95.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 37.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 24.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 21.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 21.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-epoll-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m3/lab-epoll-spin-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 355391 | 144612 | 2.46 | 234514468 | 659.88 | 3908574 | 3906164 | 0.0 | 33.5 | false | 0 | 0 | 160.0 | 30288 / 9 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8197 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 79607584 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (83187 of 343094 in 1-msg reads) | 83187 | 6.00 | 5.50 | 8.73 | 18.43 | 25.51 | 29.36 | 34.26 |

#### Stalls: m3/lab-epoll-spin-c1, loop 0 on CPU 1

234369856 iterations, longest 160.0 µs; over 5 / 20 / 100 µs: 30288 / 9 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 160.0 | 160.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 7 | 153.7 | 26.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 38.3 | 38.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 160.0 | 473 | loop work (473 items) |  |
| 2 | 38.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 26.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 23.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 21.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 21.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 20.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 20.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 20.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-epoll-spin-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m3/lab-epoll-spin-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 3163049 | 902540 | 3.50 | 226587852 | 71.64 | 3776464 | 3761422 | 0.0 | 35.4 | false | 0 | 0 | 1305.4 | 198197 / 515 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8196 | 4 |
| epoll-spin | 1 | 8198 | 4 |
| epoll-spin | 2 | 8199 | 0 |
| epoll-spin | 3 | 8194 | 4 |
| epoll-spin | 4 | 8200 | 4 |
| epoll-spin | 5 | 8195 | 0 |
| epoll-spin | 6 | 8193 | 0 |
| epoll-spin | 7 | 8196 | 4 |
| epoll-spin | 8 | 8194 | 4 |
| epoll-spin | 9 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 708522976 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (343035 of 3053560 in 1-msg reads) | 343035 | 6.28 | 5.73 | 9.88 | 15.64 | 28.42 | 41.81 | 64.81 |

#### Stalls: m3/lab-epoll-spin-c10, loop 0 on CPU 1

225685312 iterations, longest 1305.4 µs; over 5 / 20 / 100 µs: 198197 / 515 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 319 | 9952.4 | 1305.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 195 | 5757.8 | 63.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.8 | 33.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1305.4 | 4219 | loop work (4219 items) |  |
| 2 | 63.8 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 63.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 62.0 | 7 | loop work (7 items) |  |
| 5 | 61.6 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 60.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 57.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 56.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 54.1 | 12 | loop work (12 items) |  |
| 10 | 52.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 4 |
| TLB TLB shootdowns | 1 |

## m3/lab-epoll-spin-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m3/lab-epoll-spin-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 4544386 | 2279279 | 1.99 | 182480175 | 40.16 | 3041336 | 3003348 | 0.0 | 33.7 | false | 0 | 0 | 1848.8 | 267237 / 949 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8200 | 4 |
| epoll-spin | 1 | 8198 | 4 |
| epoll-spin | 2 | 8195 | 0 |
| epoll-spin | 3 | 8199 | 0 |
| epoll-spin | 4 | 8194 | 4 |
| epoll-spin | 5 | 8200 | 4 |
| epoll-spin | 6 | 8199 | 0 |
| epoll-spin | 7 | 8196 | 4 |
| epoll-spin | 8 | 8197 | 0 |
| epoll-spin | 9 | 8198 | 4 |
| epoll-spin | 10 | 8196 | 4 |
| epoll-spin | 11 | 8195 | 0 |
| epoll-spin | 12 | 8198 | 4 |
| epoll-spin | 13 | 8196 | 4 |
| epoll-spin | 14 | 8194 | 4 |
| epoll-spin | 15 | 8193 | 0 |
| epoll-spin | 16 | 8200 | 4 |
| epoll-spin | 17 | 8196 | 4 |
| epoll-spin | 18 | 8194 | 4 |
| epoll-spin | 19 | 8193 | 0 |
| epoll-spin | 20 | 8194 | 4 |
| epoll-spin | 21 | 8193 | 0 |
| epoll-spin | 22 | 8196 | 4 |
| epoll-spin | 23 | 8198 | 4 |
| epoll-spin | 24 | 8193 | 0 |
| epoll-spin | 25 | 8198 | 4 |
| epoll-spin | 26 | 8199 | 0 |
| epoll-spin | 27 | 8195 | 0 |
| epoll-spin | 28 | 8194 | 4 |
| epoll-spin | 29 | 8196 | 4 |
| epoll-spin | 30 | 8193 | 0 |
| epoll-spin | 31 | 8193 | 0 |
| epoll-spin | 32 | 8199 | 0 |
| epoll-spin | 33 | 8198 | 4 |
| epoll-spin | 34 | 8200 | 4 |
| epoll-spin | 35 | 8193 | 0 |
| epoll-spin | 36 | 8195 | 0 |
| epoll-spin | 37 | 8199 | 0 |
| epoll-spin | 38 | 8196 | 4 |
| epoll-spin | 39 | 8197 | 0 |
| epoll-spin | 40 | 8198 | 4 |
| epoll-spin | 41 | 8194 | 4 |
| epoll-spin | 42 | 8199 | 0 |
| epoll-spin | 43 | 8196 | 4 |
| epoll-spin | 44 | 8196 | 4 |
| epoll-spin | 45 | 8200 | 4 |
| epoll-spin | 46 | 8197 | 0 |
| epoll-spin | 47 | 8199 | 0 |
| epoll-spin | 48 | 8194 | 4 |
| epoll-spin | 49 | 8198 | 4 |
| epoll-spin | 50 | 8199 | 0 |
| epoll-spin | 51 | 8193 | 0 |
| epoll-spin | 52 | 8200 | 4 |
| epoll-spin | 53 | 8193 | 0 |
| epoll-spin | 54 | 8198 | 4 |
| epoll-spin | 55 | 8194 | 4 |
| epoll-spin | 56 | 8195 | 0 |
| epoll-spin | 57 | 8197 | 0 |
| epoll-spin | 58 | 8199 | 0 |
| epoll-spin | 59 | 8194 | 4 |
| epoll-spin | 60 | 8197 | 0 |
| epoll-spin | 61 | 8195 | 0 |
| epoll-spin | 62 | 8199 | 0 |
| epoll-spin | 63 | 8194 | 4 |
| epoll-spin | 64 | 8199 | 0 |
| epoll-spin | 65 | 8199 | 0 |
| epoll-spin | 66 | 8193 | 0 |
| epoll-spin | 67 | 8200 | 4 |
| epoll-spin | 68 | 8199 | 0 |
| epoll-spin | 69 | 8200 | 4 |
| epoll-spin | 70 | 8193 | 0 |
| epoll-spin | 71 | 8194 | 4 |
| epoll-spin | 72 | 8194 | 4 |
| epoll-spin | 73 | 8196 | 4 |
| epoll-spin | 74 | 8200 | 4 |
| epoll-spin | 75 | 8194 | 4 |
| epoll-spin | 76 | 8197 | 0 |
| epoll-spin | 77 | 8198 | 4 |
| epoll-spin | 78 | 8197 | 0 |
| epoll-spin | 79 | 8199 | 0 |
| epoll-spin | 80 | 8196 | 4 |
| epoll-spin | 81 | 8198 | 4 |
| epoll-spin | 82 | 8193 | 0 |
| epoll-spin | 83 | 8200 | 4 |
| epoll-spin | 84 | 8198 | 4 |
| epoll-spin | 85 | 8197 | 0 |
| epoll-spin | 86 | 8200 | 4 |
| epoll-spin | 87 | 8195 | 0 |
| epoll-spin | 88 | 8194 | 4 |
| epoll-spin | 89 | 8200 | 4 |
| epoll-spin | 90 | 8194 | 4 |
| epoll-spin | 91 | 8197 | 0 |
| epoll-spin | 92 | 8195 | 0 |
| epoll-spin | 93 | 8194 | 4 |
| epoll-spin | 94 | 8199 | 0 |
| epoll-spin | 95 | 8197 | 0 |
| epoll-spin | 96 | 8196 | 4 |
| epoll-spin | 97 | 8198 | 4 |
| epoll-spin | 98 | 8193 | 0 |
| epoll-spin | 99 | 8196 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1017942464 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1674603 of 4387091 in 1-msg reads) | 1674603 | 4.68 | 3.70 | 7.63 | 14.62 | 24.51 | 36.81 | 96.49 |

#### Stalls: m3/lab-epoll-spin-c100, loop 0 on CPU 1

180200896 iterations, longest 1848.8 µs; over 5 / 20 / 100 µs: 267237 / 949 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 616 | 16737.4 | 162.8 |
| loop work | 330 | 11422.1 | 1848.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 59.5 | 37.2 |
| mostly unexplained (+local timer tick) | 1 | 37.2 | 37.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1848.8 | 4403 | loop work (4403 items) |  |
| 2 | 693.7 | 1808 | loop work (1808 items) |  |
| 3 | 162.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 60.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 59.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 59.6 | 11 | loop work (11 items) |  |
| 7 | 56.6 | 10 | loop work (10 items) |  |
| 8 | 52.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 51.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 51.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 8 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m3/lab-mio-c1: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m3/lab-mio-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 355403 | 0 | 0.00 | 260057383 | 731.73 | 4334290 | 4330596 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| mio | 0 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|

#### Stalls: m3/lab-mio-c1, loop 0 on CPU 1

259835745 iterations, longest 6941.9 µs; over 5 / 20 / 100 µs: 55021 / 18 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4 | 7030.4 | 6941.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 13 | 301.5 | 29.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.3 | 34.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 6941.9 | 485 | loop work (485 items) | irq work (IPI/vector) 1, reschedule (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 34.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 33.1 | 10 | loop work (10 items) |  |
| 4 | 32.8 | 10 | loop work (10 items) |  |
| 5 | 29.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 26.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 25.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 25.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 23.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 23.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| LOC Local timer interrupts | 19 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |

## m3/lab-mio-c10: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m3/lab-mio-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 3163050 | 0 | 0.00 | 253236605 | 80.06 | 4220610 | 4193913 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| mio | 0 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|

#### Stalls: m3/lab-mio-c10, loop 0 on CPU 1

251634802 iterations, longest 8751.7 µs; over 5 / 20 / 100 µs: 263251 / 334 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 309 | 16931.3 | 8751.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 24 | 634.4 | 43.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 36.3 | 36.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 8751.7 | 4302 | loop work (4302 items) |  |
| 2 | 64.2 | 7 | loop work (7 items) |  |
| 3 | 56.4 | 3 | loop work (3 items) |  |
| 4 | 43.1 | 10 | loop work (10 items) |  |
| 5 | 43.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 41.6 | 10 | loop work (10 items) |  |
| 7 | 40.7 | 3 | loop work (3 items) |  |
| 8 | 40.6 | 3 | loop work (3 items) |  |
| 9 | 40.6 | 10 | loop work (10 items) |  |
| 10 | 40.4 | 10 | loop work (10 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 32 |
| LOC Local timer interrupts | 6 |
| RES Rescheduling interrupts | 1 |

## m3/lab-mio-c100: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m3/lab-mio-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 4544259 | 0 | 0.00 | 239598637 | 52.73 | 3993311 | 3920983 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| mio | 0 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|

#### Stalls: m3/lab-mio-c100, loop 0 on CPU 1

235259005 iterations, longest 9973.4 µs; over 5 / 20 / 100 µs: 353319 / 2396 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1992 | 55898.6 | 9973.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 403 | 11078.7 | 207.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.1 | 33.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 9973.4 | 6109 | loop work (6109 items) |  |
| 2 | 207.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 132.9 | 119 | loop work (119 items) |  |
| 4 | 55.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 51.0 | 7 | loop work (7 items) |  |
| 6 | 50.0 | 3 | loop work (3 items) |  |
| 7 | 49.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 48.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 47.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 46.8 | 9 | loop work (9 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 303 |
| LOC Local timer interrupts | 7 |
| RES Rescheduling interrupts | 2 |

## m3/lab-recv-busy-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=110592 clocksource=tsc

### Loop counters (m3/lab-recv-busy-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355391 | 125413 | 2.83 | 108995776 | 306.69 | 1816596 | 1816596 | 99.9 | 64.8 | true | 0 | 308 | 170.4 | 50415 / 1445 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79607584 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (62608 of 343095 in 1-msg reads) | 62608 | 3.67 | 3.38 | 5.00 | 7.07 | 23.96 | 36.13 | 40.42 |

#### Stalls: m3/lab-recv-busy-c1, loop 0 on CPU 1

108995776 iterations, longest 170.4 µs; over 5 / 20 / 100 µs: 50415 / 1445 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 812 | 25348.7 | 170.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 417 | 11359.7 | 110.3 |
| mostly unexplained (+preempted by kworker/1:1) | 98 | 2968.2 | 57.3 |
| mostly unexplained (+softirq NET_RX) | 59 | 1646.2 | 34.9 |
| softirq NET_RX | 56 | 1643.5 | 41.5 |
| softirq TIMER | 1 | 72.6 | 72.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.2 | 34.2 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 20.0 | 20.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 170.4 | 472 | loop work (472 items) |  |
| 2 | 110.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 72.6 | 0 | softirq TIMER | softirq TIMER 60, local timer tick 6, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 4 | 58.0 | 9 | loop work (9 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 57.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 55.0 | 10 | loop work (10 items) | softirq NET_RX 21, preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 52.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 12, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 51.1 | 10 | loop work (10 items) |  |
| 9 | 49.1 | 10 | loop work (10 items) | softirq NET_RX 23, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 45.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 314 |
| LOC Local timer interrupts | 312 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-recv-busy-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=721321 clocksource=tsc

### Loop counters (m3/lab-recv-busy-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3163049 | 768387 | 4.12 | 104058240 | 32.90 | 1734304 | 173430 | 99.3 | 67.9 | true | 0 | 1409 | 1265.5 | 10262814 / 73535 / 27 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 1 |
| recv-busy | 1 | 8195 | 1 |
| recv-busy | 2 | 8196 | 4 |
| recv-busy | 3 | 8195 | 1 |
| recv-busy | 4 | 8195 | 1 |
| recv-busy | 5 | 8196 | 1 |
| recv-busy | 6 | 8195 | 1 |
| recv-busy | 7 | 8197 | 1 |
| recv-busy | 8 | 8198 | 1 |
| recv-busy | 9 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708522976 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (219583 of 3053561 in 1-msg reads) | 219583 | 6.21 | 5.32 | 9.67 | 23.80 | 40.99 | 56.62 | 75.17 |

#### Stalls: m3/lab-recv-busy-c10, loop 0 on CPU 1

10405824 iterations, longest 1265.5 µs; over 5 / 20 / 100 µs: 10262814 / 73535 / 27

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 71447 | 1998592.9 | 1265.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1313 | 42776.1 | 143.3 |
| mostly unexplained (+preempted by kworker/1:1) | 571 | 20595.4 | 71.5 |
| mostly unexplained (+softirq NET_RX) | 112 | 3514.3 | 60.0 |
| mostly unexplained (+local timer tick) | 51 | 1397.0 | 52.0 |
| softirq NET_RX | 17 | 891.1 | 71.4 |
| mostly unexplained (+irq work (IPI/vector)) | 21 | 545.6 | 38.9 |
| softirq TIMER | 1 | 246.1 | 246.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 43.5 | 43.5 |
| mostly unexplained (+ipi_send_cpu) | 1 | 21.5 | 21.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1265.5 | 4218 | loop work (4218 items) |  |
| 2 | 246.1 | 0 | softirq TIMER | softirq TIMER 219, local timer tick 17, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 3 | 143.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 120.3 | 50 | loop work (50 items) |  |
| 5 | 118.7 | 64 | loop work (64 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 116.2 | 40 | loop work (40 items) | softirq NET_RX 22, preempted by kworker/1:1 6, local timer tick 5, timer tick_nohz_handler 3 |
| 7 | 115.7 | 48 | loop work (48 items) |  |
| 8 | 115.6 | 42 | loop work (42 items) | local timer tick 5, preempted by kworker/1:1 5, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 9 | 111.5 | 49 | loop work (49 items) | softirq NET_RX 6 |
| 10 | 109.7 | 32 | loop work (32 items) | local timer tick 5, timer tick_nohz_handler 2 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 1611 |
| IWI IRQ work interrupts | 1434 |
| CAL Function call interrupts | 992 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-recv-busy-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1521310 clocksource=tsc

### Loop counters (m3/lab-recv-busy-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4544390 | 1919102 | 2.37 | 95564800 | 21.03 | 1592747 | 15927 | 98.0 | 70.1 | true | 0 | 5165 | 2326.8 | 955647 / 955647 / 29420 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 1 |
| recv-busy | 1 | 8195 | 0 |
| recv-busy | 2 | 8197 | 1 |
| recv-busy | 3 | 8195 | 0 |
| recv-busy | 4 | 8193 | 1 |
| recv-busy | 5 | 8193 | 1 |
| recv-busy | 6 | 8196 | 1 |
| recv-busy | 7 | 8199 | 1 |
| recv-busy | 8 | 8200 | 1 |
| recv-busy | 9 | 8200 | 1 |
| recv-busy | 10 | 8194 | 1 |
| recv-busy | 11 | 8195 | 0 |
| recv-busy | 12 | 8200 | 1 |
| recv-busy | 13 | 8200 | 4 |
| recv-busy | 14 | 8194 | 1 |
| recv-busy | 15 | 8193 | 1 |
| recv-busy | 16 | 8198 | 4 |
| recv-busy | 17 | 8194 | 1 |
| recv-busy | 18 | 8196 | 1 |
| recv-busy | 19 | 8197 | 1 |
| recv-busy | 20 | 8199 | 0 |
| recv-busy | 21 | 8194 | 1 |
| recv-busy | 22 | 8200 | 4 |
| recv-busy | 23 | 8193 | 0 |
| recv-busy | 24 | 8200 | 1 |
| recv-busy | 25 | 8194 | 4 |
| recv-busy | 26 | 8199 | 1 |
| recv-busy | 27 | 8194 | 4 |
| recv-busy | 28 | 8197 | 1 |
| recv-busy | 29 | 8200 | 4 |
| recv-busy | 30 | 8194 | 4 |
| recv-busy | 31 | 8198 | 1 |
| recv-busy | 32 | 8197 | 1 |
| recv-busy | 33 | 8197 | 0 |
| recv-busy | 34 | 8197 | 1 |
| recv-busy | 35 | 8195 | 1 |
| recv-busy | 36 | 8193 | 1 |
| recv-busy | 37 | 8194 | 1 |
| recv-busy | 38 | 8200 | 4 |
| recv-busy | 39 | 8199 | 1 |
| recv-busy | 40 | 8198 | 1 |
| recv-busy | 41 | 8195 | 1 |
| recv-busy | 42 | 8195 | 1 |
| recv-busy | 43 | 8193 | 1 |
| recv-busy | 44 | 8196 | 4 |
| recv-busy | 45 | 8194 | 1 |
| recv-busy | 46 | 8200 | 1 |
| recv-busy | 47 | 8198 | 4 |
| recv-busy | 48 | 8197 | 1 |
| recv-busy | 49 | 8198 | 1 |
| recv-busy | 50 | 8195 | 0 |
| recv-busy | 51 | 8194 | 1 |
| recv-busy | 52 | 8196 | 4 |
| recv-busy | 53 | 8199 | 1 |
| recv-busy | 54 | 8198 | 4 |
| recv-busy | 55 | 8197 | 1 |
| recv-busy | 56 | 8195 | 0 |
| recv-busy | 57 | 8197 | 0 |
| recv-busy | 58 | 8194 | 1 |
| recv-busy | 59 | 8199 | 1 |
| recv-busy | 60 | 8195 | 1 |
| recv-busy | 61 | 8200 | 4 |
| recv-busy | 62 | 8193 | 1 |
| recv-busy | 63 | 8198 | 4 |
| recv-busy | 64 | 8195 | 1 |
| recv-busy | 65 | 8195 | 1 |
| recv-busy | 66 | 8199 | 0 |
| recv-busy | 67 | 8194 | 1 |
| recv-busy | 68 | 8198 | 4 |
| recv-busy | 69 | 8195 | 1 |
| recv-busy | 70 | 8195 | 1 |
| recv-busy | 71 | 8195 | 1 |
| recv-busy | 72 | 8197 | 1 |
| recv-busy | 73 | 8199 | 1 |
| recv-busy | 74 | 8197 | 0 |
| recv-busy | 75 | 8195 | 1 |
| recv-busy | 76 | 8194 | 4 |
| recv-busy | 77 | 8194 | 1 |
| recv-busy | 78 | 8193 | 1 |
| recv-busy | 79 | 8199 | 0 |
| recv-busy | 80 | 8197 | 1 |
| recv-busy | 81 | 8200 | 1 |
| recv-busy | 82 | 8196 | 1 |
| recv-busy | 83 | 8197 | 0 |
| recv-busy | 84 | 8196 | 4 |
| recv-busy | 85 | 8195 | 0 |
| recv-busy | 86 | 8196 | 1 |
| recv-busy | 87 | 8195 | 1 |
| recv-busy | 88 | 8199 | 1 |
| recv-busy | 89 | 8200 | 1 |
| recv-busy | 90 | 8193 | 0 |
| recv-busy | 91 | 8198 | 1 |
| recv-busy | 92 | 8196 | 1 |
| recv-busy | 93 | 8198 | 1 |
| recv-busy | 94 | 8197 | 1 |
| recv-busy | 95 | 8196 | 1 |
| recv-busy | 96 | 8195 | 1 |
| recv-busy | 97 | 8197 | 1 |
| recv-busy | 98 | 8196 | 1 |
| recv-busy | 99 | 8198 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1017943360 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1489623 of 4387126 in 1-msg reads) | 1489623 | 32.23 | 31.53 | 57.57 | 82.60 | 108.98 | 142.54 | 404.42 |

#### Stalls: m3/lab-recv-busy-c100, loop 0 on CPU 1

955648 iterations, longest 2326.8 µs; over 5 / 20 / 100 µs: 955647 / 955647 / 29420

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 751395 | 44501044.3 | 165.2 |
| loop work | 172755 | 13272953.2 | 2326.8 |
| mostly unexplained (+softirq NET_RX) | 23684 | 1531733.6 | 162.2 |
| mostly unexplained (+local timer tick) | 4506 | 324660.0 | 135.4 |
| mostly unexplained (+preempted by kworker/1:1) | 3073 | 263735.3 | 145.7 |
| mostly unexplained (+irq work (IPI/vector)) | 200 | 14550.4 | 110.0 |
| mostly unexplained (+ipi_send_cpu) | 13 | 875.1 | 75.4 |
| mostly unexplained (+softirq NET_TX) | 11 | 773.1 | 103.1 |
| mostly unexplained (+call function (IPI/vector)) | 8 | 517.0 | 71.9 |
| softirq TIMER | 1 | 388.1 | 388.1 |
| mostly unexplained (+softirq RCU) | 1 | 61.2 | 61.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2326.8 | 6079 | loop work (6079 items) |  |
| 2 | 388.1 | 42 | softirq TIMER | softirq TIMER 290, local timer tick 11, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 3 | 217.0 | 62 | loop work (62 items) | softirq NET_RX 1 |
| 4 | 210.4 | 138 | loop work (138 items) |  |
| 5 | 204.9 | 48 | loop work (48 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 199.4 | 39 | loop work (39 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 198.9 | 71 | loop work (71 items) | softirq NET_RX 1 |
| 8 | 196.9 | 82 | loop work (82 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 195.8 | 72 | loop work (72 items) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 195.5 | 56 | loop work (56 items) | softirq NET_RX 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5206 |
| LOC Local timer interrupts | 5043 |
| CAL Function call interrupts | 989 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-recv-busy-pq-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=113540 clocksource=tsc

### Loop counters (m3/lab-recv-busy-pq-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 355391 | 124258 | 2.86 | 150139776 | 422.46 | 2502330 | 1251165 | 99.8 | 56.8 | true | 0 | 328 | 173.7 | 59939 / 1910 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 79607584 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (61501 of 343094 in 1-msg reads) | 61501 | 3.72 | 3.38 | 5.12 | 7.35 | 32.00 | 41.78 | 49.68 |

#### Stalls: m3/lab-recv-busy-pq-c1, loop 0 on CPU 1

75069888 iterations, longest 173.7 µs; over 5 / 20 / 100 µs: 59939 / 1910 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1270 | 40262.7 | 173.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 442 | 12195.3 | 39.8 |
| mostly unexplained (+preempted by kworker/1:1) | 115 | 3483.0 | 61.0 |
| mostly unexplained (+softirq NET_RX) | 44 | 1280.6 | 38.8 |
| softirq NET_RX | 30 | 872.5 | 40.0 |
| mostly unexplained (+local timer tick) | 4 | 157.7 | 71.9 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 125.2 | 35.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.8 | 34.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 173.7 | 473 | loop work (473 items) |  |
| 2 | 84.9 | 10 | loop work (10 items) | local timer tick 9, preempted by kworker/1:1 7, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 3 | 71.9 | 3 | mostly unexplained (+local timer tick) | local timer tick 12, preempted by kworker/1:1 10, timer tick_nohz_handler 10, irq work (IPI/vector) 1 |
| 4 | 64.2 | 11 | loop work (11 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 64.2 | 10 | loop work (10 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 61.0 | 3 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 60.7 | 10 | loop work (10 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 59.4 | 3 | loop work (3 items) | local timer tick 6, preempted by kworker/1:1 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 9 | 58.0 | 10 | loop work (10 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 57.6 | 10 | loop work (10 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| LOC Local timer interrupts | 383 |
| IWI IRQ work interrupts | 346 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-recv-busy-pq-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=555065 clocksource=tsc

### Loop counters (m3/lab-recv-busy-pq-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3163097 | 778567 | 4.06 | 109141839 | 34.50 | 1819031 | 201866 | 99.2 | 66.6 | true | 0 | 1427 | 1362.3 | 895502 / 55375 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 4 |
| recv-busy | 1 | 8196 | 4 |
| recv-busy | 2 | 8193 | 0 |
| recv-busy | 3 | 8198 | 4 |
| recv-busy | 4 | 8195 | 0 |
| recv-busy | 5 | 8200 | 4 |
| recv-busy | 6 | 8193 | 1 |
| recv-busy | 7 | 8194 | 4 |
| recv-busy | 8 | 8197 | 0 |
| recv-busy | 9 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 708533728 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (226478 of 3053501 in 1-msg reads) | 226478 | 6.39 | 5.51 | 10.48 | 21.02 | 37.37 | 49.89 | 76.27 |

#### Stalls: m3/lab-recv-busy-pq-c10, loop 0 on CPU 1

12111936 iterations, longest 1362.3 µs; over 5 / 20 / 100 µs: 895502 / 55375 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 53733 | 1439298.3 | 1362.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 968 | 33055.3 | 49.4 |
| mostly unexplained (+preempted by kworker/1:1) | 569 | 20862.1 | 68.8 |
| mostly unexplained (+local timer tick) | 34 | 1042.2 | 50.8 |
| mostly unexplained (+softirq NET_RX) | 28 | 1015.0 | 45.8 |
| mostly unexplained (+irq work (IPI/vector)) | 33 | 917.6 | 68.8 |
| softirq NET_RX | 8 | 379.3 | 65.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 66.8 | 42.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1362.3 | 4313 | loop work (4313 items) |  |
| 2 | 105.9 | 54 | loop work (54 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 103.3 | 57 | loop work (57 items) |  |
| 4 | 97.6 | 52 | loop work (52 items) |  |
| 5 | 95.2 | 60 | loop work (60 items) | softirq NET_RX 4 |
| 6 | 93.8 | 56 | loop work (56 items) | softirq NET_RX 3 |
| 7 | 93.1 | 55 | loop work (55 items) |  |
| 8 | 92.1 | 56 | loop work (56 items) |  |
| 9 | 91.7 | 53 | loop work (53 items) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 89.6 | 61 | loop work (61 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1467 |
| LOC Local timer interrupts | 1425 |
| CAL Function call interrupts | 991 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-recv-busy-pq-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1635166 clocksource=tsc

### Loop counters (m3/lab-recv-busy-pq-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4544397 | 2112089 | 2.15 | 97389116 | 21.43 | 1623152 | 176763 | 97.6 | 68.8 | true | 0 | 5996 | 2076.7 | 3658280 / 83093 / 21 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 1 |
| recv-busy | 1 | 8197 | 1 |
| recv-busy | 2 | 8195 | 0 |
| recv-busy | 3 | 8197 | 1 |
| recv-busy | 4 | 8200 | 1 |
| recv-busy | 5 | 8198 | 1 |
| recv-busy | 6 | 8200 | 1 |
| recv-busy | 7 | 8198 | 1 |
| recv-busy | 8 | 8193 | 1 |
| recv-busy | 9 | 8196 | 4 |
| recv-busy | 10 | 8197 | 1 |
| recv-busy | 11 | 8193 | 0 |
| recv-busy | 12 | 8196 | 1 |
| recv-busy | 13 | 8195 | 1 |
| recv-busy | 14 | 8197 | 1 |
| recv-busy | 15 | 8199 | 1 |
| recv-busy | 16 | 8195 | 1 |
| recv-busy | 17 | 8198 | 1 |
| recv-busy | 18 | 8195 | 1 |
| recv-busy | 19 | 8194 | 4 |
| recv-busy | 20 | 8198 | 1 |
| recv-busy | 21 | 8199 | 0 |
| recv-busy | 22 | 8195 | 1 |
| recv-busy | 23 | 8193 | 0 |
| recv-busy | 24 | 8199 | 1 |
| recv-busy | 25 | 8197 | 1 |
| recv-busy | 26 | 8200 | 1 |
| recv-busy | 27 | 8193 | 1 |
| recv-busy | 28 | 8194 | 4 |
| recv-busy | 29 | 8199 | 0 |
| recv-busy | 30 | 8200 | 4 |
| recv-busy | 31 | 8194 | 4 |
| recv-busy | 32 | 8195 | 0 |
| recv-busy | 33 | 8200 | 1 |
| recv-busy | 34 | 8195 | 0 |
| recv-busy | 35 | 8196 | 4 |
| recv-busy | 36 | 8198 | 1 |
| recv-busy | 37 | 8200 | 4 |
| recv-busy | 38 | 8198 | 1 |
| recv-busy | 39 | 8195 | 1 |
| recv-busy | 40 | 8195 | 1 |
| recv-busy | 41 | 8196 | 1 |
| recv-busy | 42 | 8198 | 1 |
| recv-busy | 43 | 8197 | 0 |
| recv-busy | 44 | 8200 | 1 |
| recv-busy | 45 | 8198 | 4 |
| recv-busy | 46 | 8195 | 0 |
| recv-busy | 47 | 8193 | 1 |
| recv-busy | 48 | 8193 | 1 |
| recv-busy | 49 | 8199 | 1 |
| recv-busy | 50 | 8197 | 0 |
| recv-busy | 51 | 8194 | 1 |
| recv-busy | 52 | 8199 | 0 |
| recv-busy | 53 | 8194 | 1 |
| recv-busy | 54 | 8200 | 1 |
| recv-busy | 55 | 8195 | 1 |
| recv-busy | 56 | 8197 | 1 |
| recv-busy | 57 | 8198 | 1 |
| recv-busy | 58 | 8199 | 0 |
| recv-busy | 59 | 8197 | 0 |
| recv-busy | 60 | 8196 | 1 |
| recv-busy | 61 | 8197 | 0 |
| recv-busy | 62 | 8199 | 1 |
| recv-busy | 63 | 8200 | 1 |
| recv-busy | 64 | 8193 | 1 |
| recv-busy | 65 | 8194 | 4 |
| recv-busy | 66 | 8195 | 1 |
| recv-busy | 67 | 8193 | 0 |
| recv-busy | 68 | 8194 | 4 |
| recv-busy | 69 | 8200 | 4 |
| recv-busy | 70 | 8199 | 1 |
| recv-busy | 71 | 8198 | 1 |
| recv-busy | 72 | 8196 | 1 |
| recv-busy | 73 | 8195 | 1 |
| recv-busy | 74 | 8194 | 1 |
| recv-busy | 75 | 8194 | 4 |
| recv-busy | 76 | 8196 | 1 |
| recv-busy | 77 | 8199 | 1 |
| recv-busy | 78 | 8196 | 4 |
| recv-busy | 79 | 8194 | 1 |
| recv-busy | 80 | 8200 | 4 |
| recv-busy | 81 | 8199 | 0 |
| recv-busy | 82 | 8194 | 1 |
| recv-busy | 83 | 8193 | 1 |
| recv-busy | 84 | 8195 | 1 |
| recv-busy | 85 | 8197 | 1 |
| recv-busy | 86 | 8195 | 0 |
| recv-busy | 87 | 8196 | 4 |
| recv-busy | 88 | 8193 | 1 |
| recv-busy | 89 | 8197 | 1 |
| recv-busy | 90 | 8198 | 1 |
| recv-busy | 91 | 8198 | 1 |
| recv-busy | 92 | 8200 | 4 |
| recv-busy | 93 | 8199 | 1 |
| recv-busy | 94 | 8194 | 1 |
| recv-busy | 95 | 8195 | 1 |
| recv-busy | 96 | 8196 | 1 |
| recv-busy | 97 | 8197 | 1 |
| recv-busy | 98 | 8196 | 1 |
| recv-busy | 99 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1017944928 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1522003 of 4387088 in 1-msg reads) | 1522003 | 7.35 | 6.60 | 10.76 | 25.65 | 40.26 | 55.32 | 98.20 |

#### Stalls: m3/lab-recv-busy-pq-c100, loop 0 on CPU 1

10605760 iterations, longest 2076.7 µs; over 5 / 20 / 100 µs: 3658280 / 83093 / 21

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 75046 | 2151169.7 | 2076.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4736 | 159445.2 | 181.0 |
| mostly unexplained (+preempted by kworker/1:1) | 2464 | 75590.9 | 76.9 |
| mostly unexplained (+local timer tick) | 586 | 13801.7 | 51.2 |
| mostly unexplained (+softirq NET_RX) | 146 | 4835.4 | 59.1 |
| mostly unexplained (+irq work (IPI/vector)) | 102 | 2448.7 | 45.6 |
| softirq NET_RX | 11 | 376.8 | 62.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.8 | 23.8 |
| mostly unexplained (+softirq RCU) | 1 | 22.6 | 22.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2076.7 | 4836 | loop work (4836 items) |  |
| 2 | 549.2 | 1394 | loop work (1394 items) |  |
| 3 | 181.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 114.6 | 82 | loop work (82 items) |  |
| 5 | 114.5 | 65 | loop work (65 items) |  |
| 6 | 114.0 | 61 | loop work (61 items) |  |
| 7 | 111.5 | 58 | loop work (58 items) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, softirq NET_RX 1, workqueue [ena] 1 |
| 8 | 111.0 | 47 | loop work (47 items) |  |
| 9 | 107.0 | 51 | loop work (51 items) |  |
| 10 | 106.0 | 73 | loop work (73 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6035 |
| LOC Local timer interrupts | 5811 |
| CAL Function call interrupts | 992 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-spin-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m3/lab-uring-spin-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 355391 | 132992 | 2.67 | 234124 | 0.66 | 3902 | 50244631 | 0.0 | 0.7 | false | 0 | 0 | 277.6 | 23472 / 14 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8194 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 79607584 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-spin (82500 of 343094 in 1-msg reads) | 82500 | 6.89 | 5.66 | 13.16 | 18.90 | 23.16 | 28.77 | 52.52 |

#### Stalls: m3/lab-uring-spin-c1, loop 0 on CPU 1

3014677888 iterations, longest 277.6 µs; over 5 / 20 / 100 µs: 23472 / 14 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 10 | 319.8 | 101.0 |
| softirq TIMER | 1 | 277.6 | 277.6 |
| loop work | 1 | 82.3 | 82.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 60.0 | 35.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 277.6 | 0 | softirq TIMER | softirq TIMER 247, local timer tick 18, timer tick_nohz_handler 11, irq work (IPI/vector) 1 |
| 2 | 101.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 82.3 | 473 | loop work (473 items) |  |
| 4 | 35.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 34.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 30.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 24.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 24.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 22.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 22.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-spin-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m3/lab-uring-spin-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 3163050 | 923051 | 3.43 | 1183011 | 0.37 | 19717 | 31372015 | 0.0 | 4.5 | false | 0 | 0 | 671.8 | 189288 / 231 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8197 | 0 |
| uring-spin | 1 | 8195 | 0 |
| uring-spin | 2 | 8193 | 0 |
| uring-spin | 3 | 8194 | 4 |
| uring-spin | 4 | 8195 | 0 |
| uring-spin | 5 | 8197 | 0 |
| uring-spin | 6 | 8195 | 0 |
| uring-spin | 7 | 8199 | 0 |
| uring-spin | 8 | 8198 | 4 |
| uring-spin | 9 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 708523200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-spin (365543 of 3053555 in 1-msg reads) | 365543 | 7.12 | 6.32 | 11.43 | 18.51 | 25.84 | 36.46 | 70.14 |

#### Stalls: m3/lab-uring-spin-c10, loop 0 on CPU 1

1882320896 iterations, longest 671.8 µs; over 5 / 20 / 100 µs: 189288 / 231 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 126 | 3793.5 | 664.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 103 | 3370.9 | 671.8 |
| softirq TIMER | 1 | 118.7 | 118.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 36.2 | 36.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 671.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 664.8 | 4219 | loop work (4219 items) |  |
| 3 | 118.7 | 0 | softirq TIMER | softirq TIMER 92, local timer tick 17, timer tick_nohz_handler 10, irq work (IPI/vector) 1 |
| 4 | 56.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 38.7 | 7 | loop work (7 items) |  |
| 6 | 38.6 | 5 | loop work (5 items) |  |
| 7 | 38.5 | 9 | loop work (9 items) |  |
| 8 | 38.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 37.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-spin-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m3/lab-uring-spin-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 4544380 | 2290505 | 1.98 | 3587495 | 0.79 | 59792 | 6950368 | 0.0 | 10.2 | false | 0 | 0 | 1346.8 | 260994 / 840 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-spin | 0 | 8198 | 4 |
| uring-spin | 1 | 8200 | 4 |
| uring-spin | 2 | 8197 | 0 |
| uring-spin | 3 | 8193 | 0 |
| uring-spin | 4 | 8195 | 0 |
| uring-spin | 5 | 8200 | 4 |
| uring-spin | 6 | 8194 | 4 |
| uring-spin | 7 | 8199 | 0 |
| uring-spin | 8 | 8195 | 0 |
| uring-spin | 9 | 8198 | 4 |
| uring-spin | 10 | 8197 | 0 |
| uring-spin | 11 | 8198 | 4 |
| uring-spin | 12 | 8196 | 4 |
| uring-spin | 13 | 8199 | 0 |
| uring-spin | 14 | 8194 | 4 |
| uring-spin | 15 | 8198 | 4 |
| uring-spin | 16 | 8199 | 0 |
| uring-spin | 17 | 8200 | 4 |
| uring-spin | 18 | 8197 | 0 |
| uring-spin | 19 | 8195 | 0 |
| uring-spin | 20 | 8200 | 4 |
| uring-spin | 21 | 8198 | 4 |
| uring-spin | 22 | 8198 | 4 |
| uring-spin | 23 | 8198 | 4 |
| uring-spin | 24 | 8196 | 4 |
| uring-spin | 25 | 8193 | 0 |
| uring-spin | 26 | 8198 | 4 |
| uring-spin | 27 | 8199 | 0 |
| uring-spin | 28 | 8198 | 4 |
| uring-spin | 29 | 8199 | 0 |
| uring-spin | 30 | 8195 | 0 |
| uring-spin | 31 | 8193 | 0 |
| uring-spin | 32 | 8196 | 4 |
| uring-spin | 33 | 8197 | 0 |
| uring-spin | 34 | 8195 | 0 |
| uring-spin | 35 | 8196 | 4 |
| uring-spin | 36 | 8197 | 0 |
| uring-spin | 37 | 8196 | 4 |
| uring-spin | 38 | 8195 | 0 |
| uring-spin | 39 | 8196 | 4 |
| uring-spin | 40 | 8200 | 4 |
| uring-spin | 41 | 8200 | 4 |
| uring-spin | 42 | 8199 | 0 |
| uring-spin | 43 | 8196 | 4 |
| uring-spin | 44 | 8200 | 4 |
| uring-spin | 45 | 8195 | 0 |
| uring-spin | 46 | 8197 | 0 |
| uring-spin | 47 | 8196 | 4 |
| uring-spin | 48 | 8199 | 0 |
| uring-spin | 49 | 8200 | 4 |
| uring-spin | 50 | 8196 | 4 |
| uring-spin | 51 | 8194 | 4 |
| uring-spin | 52 | 8197 | 0 |
| uring-spin | 53 | 8193 | 0 |
| uring-spin | 54 | 8199 | 0 |
| uring-spin | 55 | 8197 | 0 |
| uring-spin | 56 | 8199 | 0 |
| uring-spin | 57 | 8194 | 4 |
| uring-spin | 58 | 8195 | 0 |
| uring-spin | 59 | 8193 | 0 |
| uring-spin | 60 | 8196 | 4 |
| uring-spin | 61 | 8200 | 4 |
| uring-spin | 62 | 8200 | 4 |
| uring-spin | 63 | 8195 | 0 |
| uring-spin | 64 | 8196 | 4 |
| uring-spin | 65 | 8194 | 4 |
| uring-spin | 66 | 8195 | 0 |
| uring-spin | 67 | 8199 | 0 |
| uring-spin | 68 | 8195 | 0 |
| uring-spin | 69 | 8197 | 0 |
| uring-spin | 70 | 8196 | 4 |
| uring-spin | 71 | 8200 | 4 |
| uring-spin | 72 | 8197 | 0 |
| uring-spin | 73 | 8193 | 0 |
| uring-spin | 74 | 8197 | 0 |
| uring-spin | 75 | 8200 | 4 |
| uring-spin | 76 | 8196 | 4 |
| uring-spin | 77 | 8197 | 0 |
| uring-spin | 78 | 8196 | 4 |
| uring-spin | 79 | 8194 | 4 |
| uring-spin | 80 | 8193 | 0 |
| uring-spin | 81 | 8194 | 4 |
| uring-spin | 82 | 8199 | 0 |
| uring-spin | 83 | 8199 | 0 |
| uring-spin | 84 | 8198 | 4 |
| uring-spin | 85 | 8200 | 4 |
| uring-spin | 86 | 8195 | 0 |
| uring-spin | 87 | 8197 | 0 |
| uring-spin | 88 | 8195 | 0 |
| uring-spin | 89 | 8197 | 0 |
| uring-spin | 90 | 8198 | 4 |
| uring-spin | 91 | 8195 | 0 |
| uring-spin | 92 | 8197 | 0 |
| uring-spin | 93 | 8198 | 4 |
| uring-spin | 94 | 8196 | 4 |
| uring-spin | 95 | 8193 | 0 |
| uring-spin | 96 | 8197 | 0 |
| uring-spin | 97 | 8199 | 0 |
| uring-spin | 98 | 8197 | 0 |
| uring-spin | 99 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-spin | 1017941120 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-spin (1684018 of 4387058 in 1-msg reads) | 1684018 | 5.19 | 4.07 | 8.32 | 16.45 | 25.74 | 36.88 | 343.30 |

#### Stalls: m3/lab-uring-spin-c100, loop 0 on CPU 1

417022080 iterations, longest 1346.8 µs; over 5 / 20 / 100 µs: 260994 / 840 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 634 | 18243.8 | 1346.8 |
| loop work | 204 | 6332.9 | 678.6 |
| softirq TIMER | 1 | 366.9 | 366.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 37.1 | 37.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1346.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 678.6 | 3785 | loop work (3785 items) |  |
| 3 | 424.3 | 2393 | loop work (2393 items) |  |
| 4 | 366.9 | 0 | softirq TIMER | softirq TIMER 327, local timer tick 15, timer tick_nohz_handler 8 |
| 5 | 70.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 62.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 59.6 | 12 | loop work (12 items) |  |
| 8 | 54.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 53.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 51.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-sqpoll-c1: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=107352 clocksource=tsc

### Loop counters (m3/lab-uring-sqpoll-c1)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 355391 | 124707 | 2.85 | 1 | 0.00 | 0 | 33423666 | 0.0 | 0.0 | true | 0 | 0 | 80.6 | 248 / 5 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8193 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 79607584 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (62300 of 343094 in 1-msg reads) | 62300 | 4.38 | 4.01 | 5.73 | 8.42 | 19.39 | 24.54 | 27.40 |

#### Stalls: m3/lab-uring-sqpoll-c1, loop 0 on CPU 1

2005419968 iterations, longest 80.6 µs; over 5 / 20 / 100 µs: 248 / 5 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 80.6 | 80.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 79.7 | 34.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 37.5 | 37.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 80.6 | 473 | loop work (473 items) |  |
| 2 | 37.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 34.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 24.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 20.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-sqpoll-c10: lab, 60 s (first 2 s dropped), 10 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=768517 clocksource=tsc

### Loop counters (m3/lab-uring-sqpoll-c10)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 3163050 | 820042 | 3.86 | 1 | 0.00 | 0 | 28785921 | 0.0 | 0.0 | true | 0 | 0 | 662.4 | 5354 / 3 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8197 | 2 |
| uring-sqpoll | 1 | 8194 | 4 |
| uring-sqpoll | 2 | 8197 | 2 |
| uring-sqpoll | 3 | 8199 | 2 |
| uring-sqpoll | 4 | 8196 | 2 |
| uring-sqpoll | 5 | 8196 | 2 |
| uring-sqpoll | 6 | 8200 | 2 |
| uring-sqpoll | 7 | 8199 | 2 |
| uring-sqpoll | 8 | 8200 | 2 |
| uring-sqpoll | 9 | 8200 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 708523200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (231795 of 3053455 in 1-msg reads) | 231795 | 6.03 | 5.32 | 8.91 | 16.65 | 35.25 | 49.66 | 73.50 |

#### Stalls: m3/lab-uring-sqpoll-c10, loop 0 on CPU 1

1727155264 iterations, longest 662.4 µs; over 5 / 20 / 100 µs: 5354 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 662.4 | 662.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 38.7 | 38.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 32.6 | 32.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 662.4 | 4229 | loop work (4229 items) |  |
| 2 | 38.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 32.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m3/lab-uring-sqpoll-c100: lab, 60 s (first 2 s dropped), 100 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1978341 clocksource=tsc

### Loop counters (m3/lab-uring-sqpoll-c100)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4544268 | 2196232 | 2.07 | 1 | 0.00 | 0 | 7589438 | 0.0 | 0.0 | true | 0 | 0 | 603.0 | 6483 / 3 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8198 | 2 |
| uring-sqpoll | 1 | 8199 | 2 |
| uring-sqpoll | 2 | 8195 | 0 |
| uring-sqpoll | 3 | 8196 | 2 |
| uring-sqpoll | 4 | 8200 | 2 |
| uring-sqpoll | 5 | 8195 | 2 |
| uring-sqpoll | 6 | 8199 | 2 |
| uring-sqpoll | 7 | 8198 | 4 |
| uring-sqpoll | 8 | 8197 | 2 |
| uring-sqpoll | 9 | 8197 | 2 |
| uring-sqpoll | 10 | 8193 | 2 |
| uring-sqpoll | 11 | 8198 | 2 |
| uring-sqpoll | 12 | 8196 | 4 |
| uring-sqpoll | 13 | 8196 | 2 |
| uring-sqpoll | 14 | 8195 | 2 |
| uring-sqpoll | 15 | 8198 | 2 |
| uring-sqpoll | 16 | 8197 | 2 |
| uring-sqpoll | 17 | 8197 | 2 |
| uring-sqpoll | 18 | 8199 | 2 |
| uring-sqpoll | 19 | 8200 | 2 |
| uring-sqpoll | 20 | 8195 | 2 |
| uring-sqpoll | 21 | 8193 | 2 |
| uring-sqpoll | 22 | 8200 | 2 |
| uring-sqpoll | 23 | 8200 | 2 |
| uring-sqpoll | 24 | 8198 | 2 |
| uring-sqpoll | 25 | 8194 | 2 |
| uring-sqpoll | 26 | 8197 | 2 |
| uring-sqpoll | 27 | 8194 | 2 |
| uring-sqpoll | 28 | 8199 | 2 |
| uring-sqpoll | 29 | 8200 | 4 |
| uring-sqpoll | 30 | 8197 | 2 |
| uring-sqpoll | 31 | 8193 | 2 |
| uring-sqpoll | 32 | 8199 | 2 |
| uring-sqpoll | 33 | 8193 | 2 |
| uring-sqpoll | 34 | 8193 | 2 |
| uring-sqpoll | 35 | 8195 | 2 |
| uring-sqpoll | 36 | 8194 | 2 |
| uring-sqpoll | 37 | 8195 | 2 |
| uring-sqpoll | 38 | 8195 | 2 |
| uring-sqpoll | 39 | 8196 | 2 |
| uring-sqpoll | 40 | 8193 | 2 |
| uring-sqpoll | 41 | 8200 | 2 |
| uring-sqpoll | 42 | 8200 | 2 |
| uring-sqpoll | 43 | 8198 | 2 |
| uring-sqpoll | 44 | 8194 | 2 |
| uring-sqpoll | 45 | 8196 | 2 |
| uring-sqpoll | 46 | 8193 | 2 |
| uring-sqpoll | 47 | 8199 | 2 |
| uring-sqpoll | 48 | 8194 | 2 |
| uring-sqpoll | 49 | 8200 | 2 |
| uring-sqpoll | 50 | 8200 | 4 |
| uring-sqpoll | 51 | 8199 | 2 |
| uring-sqpoll | 52 | 8200 | 2 |
| uring-sqpoll | 53 | 8194 | 2 |
| uring-sqpoll | 54 | 8199 | 2 |
| uring-sqpoll | 55 | 8200 | 2 |
| uring-sqpoll | 56 | 8195 | 2 |
| uring-sqpoll | 57 | 8194 | 2 |
| uring-sqpoll | 58 | 8194 | 2 |
| uring-sqpoll | 59 | 8196 | 2 |
| uring-sqpoll | 60 | 8200 | 2 |
| uring-sqpoll | 61 | 8195 | 0 |
| uring-sqpoll | 62 | 8198 | 2 |
| uring-sqpoll | 63 | 8194 | 2 |
| uring-sqpoll | 64 | 8194 | 2 |
| uring-sqpoll | 65 | 8199 | 2 |
| uring-sqpoll | 66 | 8193 | 2 |
| uring-sqpoll | 67 | 8200 | 2 |
| uring-sqpoll | 68 | 8198 | 2 |
| uring-sqpoll | 69 | 8195 | 2 |
| uring-sqpoll | 70 | 8199 | 2 |
| uring-sqpoll | 71 | 8194 | 2 |
| uring-sqpoll | 72 | 8198 | 2 |
| uring-sqpoll | 73 | 8197 | 2 |
| uring-sqpoll | 74 | 8198 | 2 |
| uring-sqpoll | 75 | 8197 | 2 |
| uring-sqpoll | 76 | 8193 | 2 |
| uring-sqpoll | 77 | 8197 | 2 |
| uring-sqpoll | 78 | 8196 | 2 |
| uring-sqpoll | 79 | 8193 | 0 |
| uring-sqpoll | 80 | 8199 | 2 |
| uring-sqpoll | 81 | 8194 | 4 |
| uring-sqpoll | 82 | 8195 | 2 |
| uring-sqpoll | 83 | 8200 | 2 |
| uring-sqpoll | 84 | 8199 | 2 |
| uring-sqpoll | 85 | 8198 | 2 |
| uring-sqpoll | 86 | 8195 | 0 |
| uring-sqpoll | 87 | 8194 | 2 |
| uring-sqpoll | 88 | 8195 | 2 |
| uring-sqpoll | 89 | 8197 | 2 |
| uring-sqpoll | 90 | 8198 | 4 |
| uring-sqpoll | 91 | 8198 | 2 |
| uring-sqpoll | 92 | 8193 | 2 |
| uring-sqpoll | 93 | 8194 | 2 |
| uring-sqpoll | 94 | 8195 | 2 |
| uring-sqpoll | 95 | 8196 | 2 |
| uring-sqpoll | 96 | 8193 | 2 |
| uring-sqpoll | 97 | 8198 | 2 |
| uring-sqpoll | 98 | 8197 | 2 |
| uring-sqpoll | 99 | 8193 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1017916032 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1567483 of 4386948 in 1-msg reads) | 1567483 | 5.03 | 4.23 | 7.45 | 17.58 | 33.78 | 48.33 | 75.06 |

#### Stalls: m3/lab-uring-sqpoll-c100, loop 0 on CPU 1

455366272 iterations, longest 603.0 µs; over 5 / 20 / 100 µs: 6483 / 3 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 970.0 | 603.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 38.6 | 38.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 603.0 | 3784 | loop work (3784 items) |  |
| 2 | 366.9 | 2327 | loop work (2327 items) |  |
| 3 | 38.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 3 |
| TLB TLB shootdowns | 1 |

