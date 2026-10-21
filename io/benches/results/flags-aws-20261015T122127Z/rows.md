# Rows quoted by the docs

Verbatim sections of this run's generated tables, recovered from git (`git show 128747b9^:io/benches/results/flags-aws-20261015T122127Z/tables.md`).
Only the sections `io/benches/BENCHMARK.md` and the book quote; nothing is recomputed. All values µs unless a column says otherwise.

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
