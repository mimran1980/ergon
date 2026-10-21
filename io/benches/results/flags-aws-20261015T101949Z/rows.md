# Rows quoted by the docs

Verbatim sections of this run's generated tables, recovered from git (`git show 128747b9^:io/benches/results/flags-aws-20261015T101949Z/tables.md`).
Only the sections `io/benches/BENCHMARK.md` and the book quote; nothing is recomputed. All values µs unless a column says otherwise.

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c1-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923927 | 2.31 | 6.34 | 13.72 | 22.00 | 32.74 | 0.0 | 0.0 | 0.0 | 0.0 | 2334.94 | 6033.0 | 2334.94 µs stall: loop work (9447 items) |
| m2 | 5923898 | 2.54 | 8.72 | 16.86 | 88.05 | 150.37 | 432.4 | 0.0 | 0.0 | 0.0 | 2674.10 | 4290.0 | 2674.10 µs stall: loop work (9656 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c1-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923927 | 10.24 | 25.44 | 37.66 | 71.25 | 163.72 | 149.0 | 0.0 | 0.0 | 0.0 |
| m2 | 5923898 | 21.08 | 60.38 | 69.34 | 115837.00 | 121411.18 | 2354.5 | 1841.4 | 1734.9 | 1734.9 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c16-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5924001 | 5.56 | 28.02 | 44.14 | 57.73 | 91.90 | 0.0 | 0.0 | 0.0 | 0.0 | 3194.67 | 113401.0 | 3194.67 µs stall: loop work (9590 items) |
| m2 | 5923972 | 6.59 | 29.14 | 46.22 | 62.08 | 157.22 | 4.1 | 0.0 | 0.0 | 0.0 | 3294.70 | 117250.0 | 3294.70 µs stall: loop work (9757 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c16-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5924001 | 20.33 | 76.26 | 94.86 | 113.10 | 153.04 | 3234.8 | 0.0 | 0.0 | 0.0 |
| m2 | 5923972 | 26.01 | 84.62 | 104.87 | 122.47 | 169.68 | 10887.9 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c256-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5924872 | 86.98 | 209.20 | 236.35 | 257.35 | 294.36 | 2606841.3 | 0.0 | 0.0 | 0.0 | 3655.03 | 384767.0 | 3655.03 µs stall: loop work (9819 items) |
| m2 | 5924399 | 87.01 | 209.07 | 236.17 | 258.21 | 309.15 | 2604505.5 | 0.0 | 0.0 | 0.0 | 3538.49 | 384511.0 | 3538.49 µs stall: loop work (9811 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c256-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5924872 | 123.23 | 258.25 | 286.93 | 309.31 | 350.55 | 3846146.6 | 0.0 | 0.0 | 0.0 |
| m2 | 5924399 | 129.95 | 265.24 | 293.19 | 316.41 | 368.98 | 4068421.6 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c4-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923970 | 2.65 | 12.69 | 22.79 | 34.55 | 56.14 | 0.0 | 0.0 | 0.0 | 0.0 | 2532.20 | 27676.0 | 2532.20 µs stall: loop work (9516 items) |
| m2 | 5923950 | 2.65 | 11.22 | 20.42 | 32.23 | 50.44 | 0.0 | 0.0 | 0.0 | 0.0 | 3008.69 | 23993.0 | 3008.69 µs stall: loop work (9749 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c4-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923970 | 12.92 | 54.20 | 67.84 | 83.04 | 142.27 | 68.3 | 0.0 | 0.0 | 0.0 |
| m2 | 5923950 | 26.47 | 69.90 | 82.97 | 96.99 | 189.56 | 356.9 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c64-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5924173 | 22.50 | 79.60 | 102.42 | 121.98 | 160.95 | 7977.7 | 0.0 | 0.0 | 0.0 | 3549.89 | 1535935.0 | 3549.89 µs stall: loop work (9749 items) |
| m2 | 5924004 | 22.66 | 75.42 | 97.79 | 117.56 | 154.76 | 4844.4 | 0.0 | 0.0 | 0.0 | 3427.84 | 1540927.0 | 3427.84 µs stall: loop work (9792 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-c64-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5924173 | 46.36 | 131.87 | 156.29 | 175.82 | 229.13 | 503255.4 | 0.0 | 0.0 | 0.0 |
| m2 | 5924004 | 60.85 | 146.00 | 170.00 | 190.01 | 228.16 | 914344.3 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c1-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923810 | 2.74 | 7.10 | 14.15 | 20.59 | 32.01 | 0.0 | 0.0 | 0.0 | 0.0 | 2470.68 | 7057.0 | 2470.68 µs stall: loop work (9498 items) |
| m2 | 5923894 | 2.48 | 5.74 | 12.35 | 19.31 | 30.68 | 0.0 | 0.0 | 0.0 | 0.0 | 2687.17 | 5515.0 | 2687.17 µs stall: loop work (9644 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c1-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923810 | 13.60 | 29.08 | 40.43 | 60.33 | 611.34 | 57.9 | 4.1 | 0.0 | 0.0 |
| m2 | 5923894 | 19.71 | 32.59 | 43.14 | 57.22 | 148.66 | 47.6 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c16-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923982 | 5.38 | 25.39 | 40.74 | 53.60 | 80.93 | 0.0 | 0.0 | 0.0 | 0.0 | 3294.62 | 54222.0 | 3294.62 µs stall: loop work (9612 items) |
| m2 | 5923943 | 5.51 | 23.22 | 38.83 | 50.38 | 84.63 | 0.0 | 0.0 | 0.0 | 0.0 | 3340.42 | 48000.0 | 3340.42 µs stall: loop work (9764 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c16-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923982 | 23.15 | 83.43 | 104.17 | 123.52 | 161.76 | 9796.6 | 0.0 | 0.0 | 0.0 |
| m2 | 5923943 | 21.72 | 80.24 | 99.79 | 117.27 | 297.58 | 5972.1 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c256-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923972 | 7.26 | 29.73 | 47.06 | 59.61 | 92.49 | 0.0 | 0.0 | 0.0 | 0.0 | 1416.93 | 80225.0 | 1416.93 µs stall: loop work (3032 items) |
| m2 | 5923975 | 7.34 | 28.87 | 45.95 | 58.39 | 85.56 | 0.0 | 0.0 | 0.0 | 0.0 | 1389.02 | 76041.0 | 1389.02 µs stall: loop work (3030 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c256-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923972 | 33.11 | 99.87 | 120.51 | 138.85 | 186.75 | 60366.2 | 0.0 | 0.0 | 0.0 |
| m2 | 5923975 | 35.79 | 102.42 | 122.31 | 140.48 | 287.94 | 79481.4 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c4-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923967 | 2.97 | 13.55 | 23.80 | 34.84 | 73.08 | 0.0 | 0.0 | 0.0 | 0.0 | 2862.34 | 25555.0 | 2862.34 µs stall: loop work (9584 items) |
| m2 | 5923944 | 2.62 | 13.00 | 23.73 | 38.44 | 183.36 | 89.0 | 0.0 | 0.0 | 0.0 | 3065.25 | 27600.0 | 3065.25 µs stall: loop work (9742 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c4-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923967 | 13.10 | 55.30 | 68.41 | 83.28 | 381.95 | 119.0 | 0.0 | 0.0 | 0.0 |
| m2 | 5923944 | 27.86 | 69.92 | 84.35 | 111.36 | 222.37 | 1057.3 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c64-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5924077 | 7.04 | 29.07 | 46.39 | 61.41 | 90.62 | 0.0 | 0.0 | 0.0 | 0.0 | 3657.00 | 73142.0 | 3657.00 µs stall: loop work (9743 items) |
| m2 | 5924039 | 6.70 | 27.51 | 44.18 | 56.94 | 76.98 | 0.0 | 0.0 | 0.0 | 0.0 | 3486.07 | 67818.0 | 3486.07 µs stall: loop work (9734 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-pq-c64-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5924077 | 23.33 | 88.74 | 109.95 | 127.99 | 368.78 | 18397.2 | 0.0 | 0.0 | 0.0 |
| m2 | 5924039 | 33.82 | 98.66 | 120.21 | 139.31 | 192.15 | 53012.9 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c1-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923904 | 3.32 | 8.99 | 14.84 | 19.03 | 34.72 | 0.0 | 0.0 | 0.0 | 0.0 | 1321.93 | 3.0 | 1321.93 µs stall: loop work (9464 items) |
| m2 | 5923910 | 3.54 | 9.61 | 13.09 | 17.03 | 35.55 | 0.0 | 0.0 | 0.0 | 0.0 | 1352.98 | 3.0 | 1352.98 µs stall: loop work (9686 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c1-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923904 | 16.85 | 55.84 | 63.29 | 74.05 | 244.77 | 61.0 | 0.0 | 0.0 | 0.0 |
| m2 | 5923910 | 22.30 | 60.71 | 67.48 | 77.30 | 168.94 | 113.8 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c16-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923770 | 4.44 | 19.51 | 35.52 | 47.07 | 75.74 | 0.0 | 0.0 | 0.0 | 0.0 | 1354.40 | 4.0 | 1354.40 µs stall: loop work (9584 items) |
| m2 | 5923756 | 4.22 | 17.96 | 34.87 | 46.12 | 88.50 | 0.0 | 0.0 | 0.0 | 0.0 | 1367.29 | 3.0 | 1367.29 µs stall: loop work (9764 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c16-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923770 | 20.02 | 76.72 | 94.78 | 112.61 | 285.19 | 3246.3 | 0.0 | 0.0 | 0.0 |
| m2 | 5923756 | 26.70 | 82.36 | 102.20 | 130.87 | 198.47 | 7613.0 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c256-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923734 | 4.82 | 24.91 | 42.46 | 56.18 | 84.73 | 0.0 | 0.0 | 0.0 | 0.0 | 400.67 | 7.0 | 400.67 µs stall: loop work (2719 items) |
| m2 | 5923745 | 4.80 | 25.27 | 42.75 | 55.88 | 78.19 | 0.0 | 0.0 | 0.0 | 0.0 | 410.04 | 8.0 | 410.04 µs stall: loop work (2726 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c256-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923734 | 28.63 | 93.62 | 114.53 | 132.49 | 187.78 | 30835.1 | 0.0 | 0.0 | 0.0 |
| m2 | 5923745 | 30.83 | 95.94 | 118.30 | 138.22 | 201.73 | 40820.3 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c4-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923779 | 3.62 | 12.33 | 21.24 | 31.20 | 48.19 | 0.0 | 0.0 | 0.0 | 0.0 | 1325.12 | 4.0 | 1325.12 µs stall: loop work (9489 items) |
| m2 | 5923770 | 3.73 | 12.50 | 24.59 | 34.61 | 102.43 | 2.1 | 0.0 | 0.0 | 0.0 | 1363.64 | 4.0 | 1363.64 µs stall: loop work (9704 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c4-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923779 | 19.43 | 59.17 | 74.04 | 127.02 | 620.57 | 1022.1 | 13.4 | 0.0 | 0.0 |
| m2 | 5923770 | 27.15 | 67.27 | 81.49 | 96.34 | 336.04 | 355.9 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c64-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923748 | 4.48 | 21.41 | 39.03 | 52.19 | 168.28 | 5.2 | 0.0 | 0.0 | 0.0 | 1375.25 | 3.0 | 1375.25 µs stall: loop work (9694 items) |
| m2 | 5923711 | 4.61 | 20.85 | 38.47 | 50.86 | 74.80 | 0.0 | 0.0 | 0.0 | 0.0 | 1376.50 | 3.0 | 1376.50 µs stall: loop work (9707 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-c64-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923748 | 30.49 | 91.86 | 112.09 | 129.84 | 222.74 | 23316.1 | 0.0 | 0.0 | 0.0 |
| m2 | 5923711 | 27.24 | 88.11 | 107.15 | 124.24 | 220.16 | 14677.8 | 0.0 | 0.0 | 0.0 |

### Loop counters (m1/lab-recv-busy-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137696 | 3034305 | 2.02 | 101053184 | 16.46 | 1684220 | 1684220 | 97.0 | 70.1 | true | 0 | 4533 | 2334.9 | 129071 / 6033 / 1 | 0 | 0 |

### Loop counters (m1/lab-recv-busy-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137715 | 3645551 | 1.68 | 99059712 | 16.14 | 1650995 | 103187 | 96.3 | 73.1 | true | 0 | 6070 | 3194.7 | 6191231 / 113401 / 2 | 0 | 0 |

### Loop counters (m1/lab-recv-busy-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6138778 | 3418932 | 1.80 | 98500608 | 16.05 | 1641677 | 6413 | 96.5 | 73.2 | true | 0 | 6352 | 3655.0 | 384767 / 384767 / 384767 | 0 | 0 |

### Loop counters (m1/lab-recv-busy-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137694 | 3509227 | 1.75 | 98688000 | 16.08 | 1644800 | 411200 | 96.4 | 72.5 | true | 0 | 5644 | 2532.2 | 887304 / 27676 / 1 | 0 | 0 |

### Loop counters (m1/lab-recv-busy-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137945 | 3519937 | 1.74 | 98299904 | 16.02 | 1638332 | 25599 | 96.4 | 73.2 | true | 0 | 6304 | 3549.9 | 1535935 / 1535935 / 2127 | 0 | 0 |

### Loop counters (m1/lab-recv-busy-pq-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137744 | 3028121 | 2.03 | 140097408 | 22.83 | 2334957 | 1167478 | 95.7 | 63.9 | true | 0 | 3642 | 2470.7 | 253772 / 7057 / 1 | 0 | 0 |

### Loop counters (m1/lab-recv-busy-pq-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137716 | 3677349 | 1.67 | 104429426 | 17.01 | 1740490 | 213292 | 96.0 | 72.0 | true | 0 | 6066 | 3294.6 | 2861442 / 54222 / 1 | 0 | 0 |

### Loop counters (m1/lab-recv-busy-pq-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137881 | 3729000 | 1.65 | 96089254 | 15.66 | 1601488 | 171267 | 95.7 | 69.5 | true | 0 | 6735 | 1416.9 | 3676211 / 80225 / 9 | 0 | 0 |

### Loop counters (m1/lab-recv-busy-pq-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137719 | 3399158 | 1.81 | 115106375 | 18.75 | 1918440 | 476045 | 96.1 | 69.4 | true | 0 | 6192 | 2862.3 | 731496 / 25555 / 1 | 0 | 0 |

### Loop counters (m1/lab-recv-busy-pq-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137868 | 3721778 | 1.65 | 101831634 | 16.59 | 1697194 | 182519 | 95.9 | 71.9 | true | 0 | 6736 | 3657.0 | 3177510 / 73142 / 3 | 0 | 0 |

### Loop counters (m1/lab-uring-sqpoll-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137706 | 2913070 | 2.11 | 1 | 0.00 | 0 | 38423132 | 0.0 | 0.0 | true | 0 | 0 | 1321.9 | 7458 / 3 / 1 | 0 | 0 |

### Loop counters (m1/lab-uring-sqpoll-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137719 | 3742781 | 1.64 | 1 | 0.00 | 0 | 26951393 | 0.0 | 0.0 | true | 0 | 0 | 1354.4 | 4792 / 4 / 2 | 0 | 0 |

### Loop counters (m1/lab-uring-sqpoll-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137900 | 3818064 | 1.61 | 1 | 0.00 | 0 | 2926192 | 0.0 | 0.0 | true | 0 | 0 | 400.7 | 8523 / 7 / 4 | 0 | 0 |

### Loop counters (m1/lab-uring-sqpoll-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137698 | 3427923 | 1.79 | 1 | 0.00 | 0 | 33719276 | 0.0 | 0.0 | true | 0 | 0 | 1325.1 | 2713 / 4 / 1 | 0 | 0 |

### Loop counters (m1/lab-uring-sqpoll-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137752 | 3814829 | 1.61 | 1 | 0.00 | 0 | 11677356 | 0.0 | 0.0 | true | 0 | 0 | 1375.3 | 4825 / 3 / 1 | 0 | 0 |

### Loop counters (m2/lab-recv-busy-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137888 | 2898572 | 2.12 | 101716864 | 16.57 | 1695281 | 1695281 | 97.2 | 69.9 | true | 0 | 4605 | 2674.1 | 173044 / 4290 / 5 | 0 | 0 |

### Loop counters (m2/lab-recv-busy-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137883 | 3637933 | 1.69 | 99392512 | 16.19 | 1656542 | 103534 | 96.3 | 73.0 | true | 0 | 5656 | 3294.7 | 6212031 / 117250 / 5 | 0 | 0 |

### Loop counters (m2/lab-recv-busy-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6138307 | 3419633 | 1.80 | 98435072 | 16.04 | 1640585 | 6409 | 96.5 | 73.1 | true | 0 | 6477 | 3538.5 | 384511 / 384511 / 384511 | 0 | 0 |

### Loop counters (m2/lab-recv-busy-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137892 | 3438314 | 1.79 | 100537344 | 16.38 | 1675622 | 418906 | 96.6 | 72.1 | true | 0 | 4970 | 3008.7 | 690461 / 23993 / 2 | 0 | 0 |

### Loop counters (m2/lab-recv-busy-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137920 | 3531249 | 1.74 | 98619392 | 16.07 | 1643657 | 25682 | 96.4 | 73.0 | true | 0 | 6445 | 3427.8 | 1540927 / 1540927 / 1431 | 0 | 0 |

### Loop counters (m2/lab-recv-busy-pq-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137878 | 3274000 | 1.87 | 139830912 | 22.78 | 2330515 | 1165258 | 95.3 | 63.7 | true | 0 | 3625 | 2687.2 | 174116 / 5515 / 1 | 0 | 0 |

### Loop counters (m2/lab-recv-busy-pq-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137853 | 3676834 | 1.67 | 104715239 | 17.06 | 1745254 | 213703 | 96.0 | 71.8 | true | 0 | 6333 | 3340.4 | 2861896 / 48000 / 1 | 0 | 0 |

### Loop counters (m2/lab-recv-busy-pq-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137912 | 3735430 | 1.64 | 96105413 | 15.66 | 1601757 | 171258 | 95.6 | 69.6 | true | 0 | 7039 | 1389.0 | 3706866 / 76041 / 6 | 0 | 0 |

### Loop counters (m2/lab-recv-busy-pq-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137874 | 3389770 | 1.81 | 111271040 | 18.13 | 1854517 | 370903 | 96.2 | 70.4 | true | 0 | 5843 | 3065.2 | 1221227 / 27600 / 19 | 0 | 0 |

### Loop counters (m2/lab-recv-busy-pq-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137883 | 3724021 | 1.65 | 101959917 | 16.61 | 1699332 | 182757 | 95.9 | 71.8 | true | 0 | 6522 | 3486.1 | 3255504 / 67818 / 4 | 0 | 0 |

### Loop counters (m2/lab-uring-sqpoll-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137906 | 2876494 | 2.13 | 1 | 0.00 | 0 | 36516161 | 0.0 | 0.0 | true | 0 | 0 | 1353.0 | 7877 / 3 / 1 | 0 | 0 |

### Loop counters (m2/lab-uring-sqpoll-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137907 | 3744042 | 1.64 | 1 | 0.00 | 0 | 26807829 | 0.0 | 0.0 | true | 0 | 0 | 1367.3 | 2596 / 3 / 1 | 0 | 0 |

### Loop counters (m2/lab-uring-sqpoll-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137937 | 3816206 | 1.61 | 1 | 0.00 | 0 | 3038640 | 0.0 | 0.0 | true | 0 | 0 | 410.0 | 7678 / 8 / 4 | 0 | 0 |

### Loop counters (m2/lab-uring-sqpoll-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137882 | 3453612 | 1.78 | 1 | 0.00 | 0 | 37018182 | 0.0 | 0.0 | true | 0 | 0 | 1363.6 | 1227 / 4 / 2 | 0 | 0 |

### Loop counters (m2/lab-uring-sqpoll-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137765 | 3806859 | 1.61 | 1 | 0.00 | 0 | 10980770 | 0.0 | 0.0 | true | 0 | 0 | 1376.5 | 4011 / 3 / 1 | 0 | 0 |
