# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c1-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923844 | 6.87 | 15.61 | 19.05 | 34.05 | 51.63 | 0.0 | 0.0 | 0.0 | 0.0 | 2720.46 | 281.0 | 2720.46 µs stall: loop work (9455 items) |
| m2 | 5923799 | 6.50 | 15.24 | 17.40 | 20.50 | 47.65 | 0.0 | 0.0 | 0.0 | 0.0 | 2698.87 | 51.0 | 2698.87 µs stall: loop work (9497 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c1-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923844 | 38.90 | 82.38 | 101.52 | 111.20 | 1061.13 | 8139.5 | 41.4 | 6.2 | 0.0 |
| m2 | 5923799 | 46.30 | 93.21 | 110.03 | 121.00 | 240.50 | 30967.0 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c16-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923974 | 5.42 | 20.18 | 28.04 | 41.13 | 70.18 | 0.0 | 0.0 | 0.0 | 0.0 | 3235.50 | 1575.0 | 3235.50 µs stall: loop work (9647 items) |
| m2 | 5923963 | 5.22 | 19.12 | 26.75 | 37.51 | 63.40 | 0.0 | 0.0 | 0.0 | 0.0 | 3309.17 | 1489.0 | 3309.17 µs stall: loop work (9799 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c16-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923974 | 39.45 | 107.89 | 127.10 | 147.58 | 252.48 | 109545.9 | 0.0 | 0.0 | 0.0 |
| m2 | 5923963 | 47.62 | 115.19 | 135.06 | 152.87 | 202.74 | 184958.9 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c256-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923963 | 5.33 | 21.93 | 33.19 | 46.83 | 75.09 | 0.0 | 0.0 | 0.0 | 0.0 | 1141.80 | 2408.0 | 1141.80 µs stall: loop work (2756 items) |
| m2 | 5923963 | 5.19 | 20.66 | 29.46 | 39.33 | 70.83 | 0.0 | 0.0 | 0.0 | 0.0 | 1112.76 | 1627.0 | 1112.76 µs stall: loop work (2770 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c256-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923963 | 36.16 | 114.55 | 139.45 | 164.99 | 512.37 | 170137.6 | 7.2 | 0.0 | 0.0 |
| m2 | 5923963 | 40.88 | 116.90 | 138.64 | 159.56 | 200.12 | 201338.7 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c4-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923967 | 5.17 | 15.90 | 24.81 | 41.12 | 59.98 | 0.0 | 0.0 | 0.0 | 0.0 | 3029.72 | 1167.0 | 3029.72 µs stall: loop work (9570 items) |
| m2 | 5923949 | 5.75 | 21.45 | 32.46 | 89.56 | 133.23 | 357.9 | 0.0 | 0.0 | 0.0 | 3037.41 | 1175.0 | 3037.41 µs stall: loop work (9783 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c4-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923967 | 35.02 | 91.30 | 111.61 | 128.52 | 224.21 | 26803.7 | 0.0 | 0.0 | 0.0 |
| m2 | 5923949 | 47.56 | 104.14 | 1018.01 | 105260.17 | 111230.45 | 87449.8 | 6460.4 | 6132.5 | 5302.8 |

## Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c64-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923975 | 5.16 | 19.27 | 27.57 | 39.81 | 68.42 | 0.0 | 0.0 | 0.0 | 0.0 | 3460.97 | 1791.0 | 3460.97 µs stall: loop work (9700 items) |
| m2 | 5923967 | 5.04 | 18.75 | 26.19 | 36.59 | 71.09 | 0.0 | 0.0 | 0.0 | 0.0 | 3431.92 | 1397.0 | 3431.92 µs stall: loop work (9814 items) |

### Market-data burst, kernel arrival → decoded (every message), lab-epoll-spin-c64-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923975 | 54.41 | 128.00 | 154.74 | 174.89 | 234.07 | 431622.5 | 0.0 | 0.0 | 0.0 |
| m2 | 5923967 | 39.98 | 113.74 | 133.63 | 153.98 | 573.48 | 160273.8 | 1.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), lab-mio-c1-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|

### Market-data burst, kernel arrival → decoded (every message), lab-mio-c1-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923920 | 41.05 | 89.59 | 106.55 | 122.59 | 270.88 | 16874 | 0 | 0 | 0 |
| m2 | 5923820 | 43.52 | 92.49 | 107.75 | 122.43 | 236.45 | 25134 | 0 | 0 | 0 |

## Market-data burst, kernel arrival → decoded (every message), lab-mio-c16-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|

### Market-data burst, kernel arrival → decoded (every message), lab-mio-c16-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923971 | 36.44 | 106.44 | 126.18 | 147.04 | 251.98 | 95650 | 0 | 0 | 0 |
| m2 | 5923952 | 45.02 | 114.73 | 135.03 | 154.59 | 263.92 | 164033 | 0 | 0 | 0 |

## Market-data burst, kernel arrival → decoded (every message), lab-mio-c256-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|

### Market-data burst, kernel arrival → decoded (every message), lab-mio-c256-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923965 | 31.63 | 109.44 | 136.08 | 178.55 | 297.48 | 129456 | 0 | 0 | 0 |
| m2 | 5923964 | 43.78 | 118.88 | 141.39 | 161.24 | 217.89 | 223330 | 0 | 0 | 0 |

## Market-data burst, kernel arrival → decoded (every message), lab-mio-c4-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|

### Market-data burst, kernel arrival → decoded (every message), lab-mio-c4-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923816 | 39.70 | 94.73 | 113.58 | 129.57 | 385.25 | 34391 | 0 | 0 | 0 |
| m2 | 5923814 | 50.79 | 108.02 | 129.79 | 3222.55 | 137732.39 | 124389 | 2266 | 1898 | 1 |

## Market-data burst, kernel arrival → decoded (every message), lab-mio-c64-s1024

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|

### Market-data burst, kernel arrival → decoded (every message), lab-mio-c64-s1024, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| m1 | 5923980 | 38.60 | 113.18 | 134.20 | 153.37 | 201.48 | 155116 | 0 | 0 | 0 |
| m2 | 5923964 | 44.03 | 119.79 | 141.78 | 162.48 | 214.83 | 222875 | 0 | 0 | 0 |

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

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs, and system-management interrupts (SMI, bare metal only: firmware pauses). The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | SMI | receiving CPU | LOC | IWI | RES | CAL | SMI |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lab-epoll-spin-c1-s1024 | m1 | 3 | 0 | 0 | 0 | 16 | 0 | 4 | 999 | 0 | 1 | 5 | 0 |
| lab-epoll-spin-c1-s1024 | m2 | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 1 | 5 | 0 |
| lab-epoll-spin-c16-s1024 | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 0 | 1 | 13 | 0 |
| lab-epoll-spin-c16-s1024 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 4 | 953 | 0 | 1 | 7 | 0 |
| lab-epoll-spin-c256-s1024 | m1 | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 1 | 14 | 0 |
| lab-epoll-spin-c256-s1024 | m2 | 3 | 0 | 0 | 0 | 16 | 0 | 4 | 961 | 0 | 1 | 23 | 0 |
| lab-epoll-spin-c4-s1024 | m1 | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 1 | 14 | 0 |
| lab-epoll-spin-c4-s1024 | m2 | 3 | 0 | 0 | 0 | 16 | 0 | 4 | 992 | 0 | 1 | 23 | 0 |
| lab-epoll-spin-c64-s1024 | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 4 | 938 | 0 | 1 | 23 | 0 |
| lab-epoll-spin-c64-s1024 | m2 | 3 | 0 | 0 | 0 | 16 | 0 | 4 | 833 | 0 | 1 | 14 | 0 |
| lab-mio-c1-s1024 | m1 | 3 | 0 | 0 | 0 | 16 | 0 |  |  |  |  |  |  |
| lab-mio-c1-s1024 | m2 | 3 | 0 | 0 | 0 | 16 | 0 |  |  |  |  |  |  |
| lab-mio-c16-s1024 | m1 | 3 | 0 | 1 | 0 | 16 | 0 |  |  |  |  |  |  |
| lab-mio-c16-s1024 | m2 | 3 | 0 | 1 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-mio-c256-s1024 | m1 | 3 | 0 | 12 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-mio-c256-s1024 | m2 | 3 | 0 | 12 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-mio-c4-s1024 | m1 | 3 | 0 | 0 | 0 | 16 | 0 |  |  |  |  |  |  |
| lab-mio-c4-s1024 | m2 | 3 | 0 | 0 | 0 | 16 | 0 |  |  |  |  |  |  |
| lab-mio-c64-s1024 | m1 | 3 | 0 | 3 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-mio-c64-s1024 | m2 | 3 | 0 | 3 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c1-s1024 | m1 | 3 | 77 | 76 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c1-s1024 | m2 | 3 | 77 | 77 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-c16-s1024 | m1 | 3 | 98 | 102 | 0 | 17 | 0 | 4 | 827 | 0 | 1 | 13 | 0 |
| lab-recv-busy-c16-s1024 | m2 | 3 | 92 | 95 | 0 | 16 | 0 | 4 | 985 | 0 | 1 | 5 | 0 |
| lab-recv-busy-c256-s1024 | m1 | 3 | 102 | 107 | 0 | 17 | 0 | 4 | 792 | 0 | 1 | 24 | 0 |
| lab-recv-busy-c256-s1024 | m2 | 3 | 104 | 109 | 0 | 16 | 0 | 0 | 1143 | 0 | 1 | 22 | 0 |
| lab-recv-busy-c4-s1024 | m1 | 3 | 91 | 94 | 0 | 17 | 0 | 0 | 1144 | 0 | 1 | 12 | 0 |
| lab-recv-busy-c4-s1024 | m2 | 3 | 82 | 83 | 0 | 16 | 0 | 4 | 725 | 0 | 1 | 5 | 0 |
| lab-recv-busy-c64-s1024 | m1 | 3 | 101 | 106 | 0 | 17 | 0 | 4 | 781 | 0 | 1 | 14 | 0 |
| lab-recv-busy-c64-s1024 | m2 | 3 | 103 | 108 | 0 | 17 | 0 | 4 | 771 | 0 | 1 | 22 | 0 |
| lab-recv-busy-pq-c1-s1024 | m1 | 3 | 62 | 61 | 0 | 17 | 0 | 4 | 1009 | 0 | 1 | 6 | 0 |
| lab-recv-busy-pq-c1-s1024 | m2 | 3 | 61 | 61 | 0 | 17 | 0 | 4 | 983 | 0 | 0 | 12 | 0 |
| lab-recv-busy-pq-c16-s1024 | m1 | 3 | 98 | 102 | 0 | 17 | 0 | 4 | 849 | 0 | 1 | 6 | 0 |
| lab-recv-busy-pq-c16-s1024 | m2 | 3 | 102 | 106 | 0 | 17 | 0 | 4 | 840 | 0 | 1 | 14 | 0 |
| lab-recv-busy-pq-c256-s1024 | m1 | 3 | 108 | 113 | 0 | 17 | 0 | 0 | 1143 | 0 | 1 | 5 | 0 |
| lab-recv-busy-pq-c256-s1024 | m2 | 3 | 112 | 118 | 0 | 17 | 0 | 4 | 802 | 0 | 1 | 14 | 0 |
| lab-recv-busy-pq-c4-s1024 | m1 | 3 | 101 | 103 | 0 | 17 | 0 |  |  |  |  |  |  |
| lab-recv-busy-pq-c4-s1024 | m2 | 3 | 94 | 98 | 0 | 17 | 0 | 4 | 557 | 0 | 1 | 12 | 0 |
| lab-recv-busy-pq-c64-s1024 | m1 | 3 | 108 | 113 | 0 | 17 | 0 | 0 | 1144 | 0 | 1 | 4 | 0 |
| lab-recv-busy-pq-c64-s1024 | m2 | 3 | 105 | 109 | 0 | 17 | 0 | 4 | 910 | 0 | 1 | 24 | 0 |
| lab-uring-sqpoll-c1-s1024 | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 181 | 90 | 0 | 17 | 0 |
| lab-uring-sqpoll-c1-s1024 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 158 | 79 | 0 | 17 | 0 |
| lab-uring-sqpoll-c16-s1024 | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 170 | 88 | 0 | 17 | 0 |
| lab-uring-sqpoll-c16-s1024 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 179 | 93 | 0 | 17 | 0 |
| lab-uring-sqpoll-c256-s1024 | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 185 | 97 | 0 | 25 | 0 |
| lab-uring-sqpoll-c256-s1024 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 185 | 97 | 0 | 25 | 0 |
| lab-uring-sqpoll-c4-s1024 | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 157 | 80 | 0 | 17 | 0 |
| lab-uring-sqpoll-c4-s1024 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 163 | 84 | 0 | 17 | 0 |
| lab-uring-sqpoll-c64-s1024 | m1 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 181 | 94 | 0 | 19 | 0 |
| lab-uring-sqpoll-c64-s1024 | m2 | 3 | 0 | 0 | 0 | 17 | 0 | 2 | 177 | 91 | 0 | 19 | 0 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### lab-epoll-spin-c1-s1024

| condition | worst events |
|---|---|
| m1 | 51.63 at 23.8 s: client: loop work (5 items); 51.47 at 23.8 s: client: loop work (5 items); 51.30 at 23.8 s: client: loop work (5 items); 49.67 at 23.8 s: client: loop work (5 items); 49.51 at 23.8 s: client: loop work (5 items) |
| m2 | 47.65 at 5.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 46.19 at 5.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 46.02 at 5.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 37.94 at 5.5 s: neither loop stalled long enough: network, hypervisor or untraced; 35.25 at 30.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-epoll-spin-c16-s1024

| condition | worst events |
|---|---|
| m1 | 70.18 at 8.8 s: neither loop stalled long enough: network, hypervisor or untraced; 65.04 at 2.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 63.76 at 21.6 s: client: loop work (3 items); 63.62 at 21.6 s: client: loop work (3 items); 63.48 at 21.6 s: client: loop work (3 items) |
| m2 | 63.40 at 40.1 s: neither loop stalled long enough: network, hypervisor or untraced; 61.59 at 40.1 s: neither loop stalled long enough: network, hypervisor or untraced; 59.33 at 54.9 s: neither loop stalled long enough: network, hypervisor or untraced; 56.84 at 46.1 s: neither loop stalled long enough: network, hypervisor or untraced; 56.25 at 54.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-epoll-spin-c256-s1024

| condition | worst events |
|---|---|
| m1 | 75.09 at 47.5 s: neither loop stalled long enough: network, hypervisor or untraced; 74.94 at 47.5 s: neither loop stalled long enough: network, hypervisor or untraced; 72.82 at 47.5 s: neither loop stalled long enough: network, hypervisor or untraced; 69.04 at 7.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 68.90 at 7.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| m2 | 70.83 at 6.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 66.23 at 50.0 s: neither loop stalled long enough: network, hypervisor or untraced; 61.87 at 34.0 s: neither loop stalled long enough: network, hypervisor or untraced; 61.83 at 29.0 s: neither loop stalled long enough: network, hypervisor or untraced; 61.69 at 29.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-epoll-spin-c4-s1024

| condition | worst events |
|---|---|
| m1 | 59.98 at 19.4 s: neither loop stalled long enough: network, hypervisor or untraced; 59.81 at 3.1 s: neither loop stalled long enough: network, hypervisor or untraced; 58.92 at 43.0 s: neither loop stalled long enough: network, hypervisor or untraced; 58.76 at 43.0 s: neither loop stalled long enough: network, hypervisor or untraced; 57.76 at 45.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| m2 | 133.23 at 42.9 s: client: loop work (1443 items); 133.10 at 42.9 s: client: loop work (1443 items); 132.96 at 42.9 s: client: loop work (1443 items); 132.82 at 42.9 s: client: loop work (1443 items); 132.69 at 42.9 s: client: loop work (1443 items) |

### lab-epoll-spin-c64-s1024

| condition | worst events |
|---|---|
| m1 | 68.42 at 56.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 62.79 at 55.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 61.32 at 10.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 60.52 at 42.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 59.40 at 44.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| m2 | 71.09 at 25.1 s: neither loop stalled long enough: network, hypervisor or untraced; 68.99 at 25.1 s: neither loop stalled long enough: network, hypervisor or untraced; 62.09 at 11.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 55.01 at 42.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 54.82 at 58.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |

### lab-mio-c1-s1024

| condition | worst events |
|---|---|
| m1 |  |
| m2 |  |

### lab-mio-c16-s1024

| condition | worst events |
|---|---|
| m1 |  |
| m2 |  |

### lab-mio-c256-s1024

| condition | worst events |
|---|---|
| m1 |  |
| m2 |  |

### lab-mio-c4-s1024

| condition | worst events |
|---|---|
| m1 |  |
| m2 |  |

### lab-mio-c64-s1024

| condition | worst events |
|---|---|
| m1 |  |
| m2 |  |

### lab-recv-busy-c1-s1024

| condition | worst events |
|---|---|
| m1 | 32.74 at 25.1 s: client: loop work (16 items); 32.60 at 25.1 s: client: loop work (16 items); 32.47 at 25.1 s: client: loop work (16 items); 32.33 at 25.1 s: client: loop work (16 items); 32.33 at 41.3 s: client: loop work (2 items) |
| m2 | 150.37 at 59.8 s: client: loop work (399 items); 150.23 at 59.8 s: client: loop work (399 items); 150.09 at 59.8 s: client: loop work (399 items); 149.96 at 59.8 s: client: loop work (399 items); 149.82 at 59.8 s: client: loop work (399 items) |

### lab-recv-busy-c16-s1024

| condition | worst events |
|---|---|
| m1 | 91.90 at 32.1 s: client: loop work (43 items); 91.77 at 32.1 s: client: loop work (43 items); 91.64 at 32.1 s: client: loop work (43 items); 91.50 at 32.1 s: client: loop work (43 items); 91.37 at 32.1 s: client: loop work (43 items) |
| m2 | 157.22 at 24.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 102.71 at 17.8 s: client: loop work (46 items); 102.58 at 17.8 s: client: loop work (46 items); 102.57 at 23.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 93.04 at 45.3 s: client: loop work (40 items) |

### lab-recv-busy-c256-s1024

| condition | worst events |
|---|---|
| m1 | 294.36 at 29.3 s: client: loop work (52 items); 291.16 at 39.5 s: client: loop work (59 items); 291.01 at 39.5 s: client: loop work (59 items); 290.86 at 39.5 s: client: loop work (59 items); 290.71 at 39.5 s: client: loop work (59 items) |
| m2 | 309.15 at 6.2 s: client: loop work (43 items); 298.28 at 6.2 s: client: loop work (43 items); 298.13 at 6.2 s: client: loop work (43 items); 298.00 at 6.2 s: client: loop work (43 items); 297.86 at 6.2 s: client: loop work (43 items) |

### lab-recv-busy-c4-s1024

| condition | worst events |
|---|---|
| m1 | 56.14 at 26.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 49.75 at 14.9 s: client: loop work (9 items); 49.60 at 14.9 s: client: loop work (9 items); 45.98 at 26.1 s: neither loop stalled long enough: network, hypervisor or untraced; 45.83 at 26.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 50.44 at 31.9 s: client: loop work (25 items); 50.29 at 31.9 s: client: loop work (25 items); 48.95 at 16.1 s: neither loop stalled long enough: network, hypervisor or untraced; 48.81 at 16.1 s: neither loop stalled long enough: network, hypervisor or untraced; 48.67 at 16.1 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-recv-busy-c64-s1024

| condition | worst events |
|---|---|
| m1 | 160.95 at 56.5 s: neither loop stalled long enough: network, hypervisor or untraced; 160.81 at 56.5 s: neither loop stalled long enough: network, hypervisor or untraced; 160.68 at 56.5 s: neither loop stalled long enough: network, hypervisor or untraced; 160.54 at 56.5 s: neither loop stalled long enough: network, hypervisor or untraced; 160.41 at 56.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 154.76 at 47.0 s: neither loop stalled long enough: network, hypervisor or untraced; 154.62 at 47.0 s: neither loop stalled long enough: network, hypervisor or untraced; 154.49 at 47.0 s: neither loop stalled long enough: network, hypervisor or untraced; 154.34 at 47.0 s: neither loop stalled long enough: network, hypervisor or untraced; 154.20 at 47.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-recv-busy-pq-c1-s1024

| condition | worst events |
|---|---|
| m1 | 32.01 at 51.5 s: client: mostly unexplained (+preempted by kworker/3:1); 30.07 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced; 29.93 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced; 29.80 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced; 29.64 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 30.68 at 55.7 s: client: mostly unexplained (+preempted by kworker/3:1); 29.86 at 5.6 s: client: softirq NET_RX; 29.72 at 5.6 s: client: softirq NET_RX; 27.25 at 46.9 s: client: loop work (10 items); 27.11 at 46.9 s: client: loop work (10 items) |

### lab-recv-busy-pq-c16-s1024

| condition | worst events |
|---|---|
| m1 | 80.93 at 51.0 s: client: loop work (41 items); 80.79 at 51.0 s: client: loop work (41 items); 80.65 at 51.0 s: client: loop work (41 items); 80.51 at 51.0 s: client: loop work (41 items); 80.37 at 51.0 s: client: loop work (41 items) |
| m2 | 84.63 at 50.4 s: client: loop work (48 items); 84.49 at 50.4 s: client: loop work (48 items); 84.33 at 50.4 s: client: loop work (48 items); 84.17 at 50.4 s: client: loop work (48 items); 84.03 at 50.4 s: client: loop work (48 items) |

### lab-recv-busy-pq-c256-s1024

| condition | worst events |
|---|---|
| m1 | 92.49 at 22.0 s: client: loop work (49 items); 92.36 at 22.0 s: client: loop work (49 items); 92.22 at 22.0 s: client: loop work (49 items); 92.09 at 22.0 s: client: loop work (49 items); 91.95 at 22.0 s: client: loop work (49 items) |
| m2 | 85.56 at 42.8 s: client: loop work (69 items); 85.43 at 42.8 s: client: loop work (69 items); 85.39 at 48.9 s: client: loop work (56 items); 85.29 at 42.8 s: client: loop work (69 items); 85.25 at 48.9 s: client: loop work (56 items) |

### lab-recv-busy-pq-c4-s1024

| condition | worst events |
|---|---|
| m1 | 73.08 at 24.2 s: client: loop work (19 items); 72.94 at 24.2 s: client: loop work (19 items); 72.81 at 24.2 s: client: loop work (19 items); 72.67 at 24.2 s: client: loop work (19 items); 72.54 at 24.2 s: client: loop work (19 items) |
| m2 | 183.36 at 24.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 175.32 at 24.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 174.34 at 25.9 s: client: loop work (34 items); 174.18 at 25.9 s: client: loop work (34 items); 174.04 at 25.9 s: client: loop work (34 items) |

### lab-recv-busy-pq-c64-s1024

| condition | worst events |
|---|---|
| m1 | 90.62 at 38.9 s: client: loop work (43 items); 90.45 at 38.9 s: client: loop work (43 items); 83.18 at 39.0 s: client: loop work (8 items); 83.02 at 39.0 s: client: loop work (8 items); 82.34 at 30.4 s: client: loop work (56 items) |
| m2 | 76.98 at 5.8 s: client: loop work (55 items); 76.84 at 5.8 s: client: loop work (55 items); 76.77 at 8.0 s: client: loop work (44 items); 76.71 at 5.8 s: client: loop work (55 items); 76.64 at 8.0 s: client: loop work (44 items) |

### lab-uring-sqpoll-c1-s1024

| condition | worst events |
|---|---|
| m1 | 34.72 at 16.6 s: client: VM paused: the idle spare core stalled too (hypervisor); 27.41 at 41.3 s: neither loop stalled long enough: network, hypervisor or untraced; 27.26 at 41.3 s: neither loop stalled long enough: network, hypervisor or untraced; 27.12 at 41.3 s: neither loop stalled long enough: network, hypervisor or untraced; 26.88 at 41.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 35.55 at 39.4 s: neither loop stalled long enough: network, hypervisor or untraced; 28.21 at 50.7 s: neither loop stalled long enough: network, hypervisor or untraced; 27.94 at 50.7 s: neither loop stalled long enough: network, hypervisor or untraced; 27.08 at 17.8 s: neither loop stalled long enough: network, hypervisor or untraced; 27.06 at 11.7 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-c16-s1024

| condition | worst events |
|---|---|
| m1 | 75.74 at 5.6 s: neither loop stalled long enough: network, hypervisor or untraced; 75.60 at 5.6 s: neither loop stalled long enough: network, hypervisor or untraced; 75.47 at 5.6 s: neither loop stalled long enough: network, hypervisor or untraced; 75.32 at 5.6 s: neither loop stalled long enough: network, hypervisor or untraced; 75.18 at 5.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 88.50 at 43.9 s: neither loop stalled long enough: network, hypervisor or untraced; 88.35 at 43.9 s: neither loop stalled long enough: network, hypervisor or untraced; 88.21 at 43.9 s: neither loop stalled long enough: network, hypervisor or untraced; 88.03 at 43.9 s: neither loop stalled long enough: network, hypervisor or untraced; 87.89 at 43.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-c256-s1024

| condition | worst events |
|---|---|
| m1 | 84.73 at 23.3 s: neither loop stalled long enough: network, hypervisor or untraced; 84.60 at 23.3 s: neither loop stalled long enough: network, hypervisor or untraced; 84.46 at 23.3 s: neither loop stalled long enough: network, hypervisor or untraced; 84.32 at 23.3 s: neither loop stalled long enough: network, hypervisor or untraced; 84.19 at 23.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 78.19 at 20.7 s: neither loop stalled long enough: network, hypervisor or untraced; 78.04 at 20.7 s: neither loop stalled long enough: network, hypervisor or untraced; 77.84 at 20.7 s: neither loop stalled long enough: network, hypervisor or untraced; 77.70 at 20.7 s: neither loop stalled long enough: network, hypervisor or untraced; 77.47 at 20.7 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-c4-s1024

| condition | worst events |
|---|---|
| m1 | 48.19 at 3.5 s: neither loop stalled long enough: network, hypervisor or untraced; 48.05 at 3.5 s: neither loop stalled long enough: network, hypervisor or untraced; 47.84 at 3.5 s: neither loop stalled long enough: network, hypervisor or untraced; 47.70 at 3.5 s: neither loop stalled long enough: network, hypervisor or untraced; 47.56 at 3.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 102.43 at 40.8 s: client: softirq TIMER; 102.17 at 40.8 s: client: softirq TIMER; 83.55 at 48.8 s: neither loop stalled long enough: network, hypervisor or untraced; 58.36 at 48.8 s: neither loop stalled long enough: network, hypervisor or untraced; 56.64 at 48.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-c64-s1024

| condition | worst events |
|---|---|
| m1 | 168.28 at 49.8 s: neither loop stalled long enough: network, hypervisor or untraced; 111.08 at 49.8 s: neither loop stalled long enough: network, hypervisor or untraced; 110.77 at 49.8 s: neither loop stalled long enough: network, hypervisor or untraced; 108.44 at 49.8 s: neither loop stalled long enough: network, hypervisor or untraced; 108.10 at 49.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| m2 | 74.80 at 27.1 s: neither loop stalled long enough: network, hypervisor or untraced; 74.66 at 27.1 s: neither loop stalled long enough: network, hypervisor or untraced; 74.44 at 27.1 s: neither loop stalled long enough: network, hypervisor or untraced; 74.30 at 27.1 s: neither loop stalled long enough: network, hypervisor or untraced; 74.16 at 27.1 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| m1 | loop work 70897513.33; unexplained: no kernel event (hypervisor or in-kernel time) 36819429.86; mostly unexplained (+softirq NET_RX) 3265375.85; mostly unexplained (+local timer tick) 599108.09; mostly unexplained (+preempted by kworker/3:1) 421045.41 |
| m2 | loop work 70296740.07; unexplained: no kernel event (hypervisor or in-kernel time) 36817860.40; mostly unexplained (+softirq NET_RX) 3219704.03; mostly unexplained (+local timer tick) 599931.01; mostly unexplained (+preempted by kworker/3:1) 412468.35 |

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
| lab-epoll-spin-c1-s1024 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c1-s1024 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c1-s1024 | slow samples | 0 | | | | | |
| lab-epoll-spin-c16-s1024 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c16-s1024 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c16-s1024 | slow samples | 0 | | | | | |
| lab-epoll-spin-c256-s1024 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c256-s1024 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c256-s1024 | slow samples | 0 | | | | | |
| lab-epoll-spin-c4-s1024 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c4-s1024 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c4-s1024 | slow samples | 0 | | | | | |
| lab-epoll-spin-c64-s1024 | idle spare core | 0 | | | | | |
| lab-epoll-spin-c64-s1024 | receiver loop | 0 | | | | | |
| lab-epoll-spin-c64-s1024 | slow samples | 0 | | | | | |
| lab-mio-c1-s1024 | idle spare core | 0 | | | | | |
| lab-mio-c1-s1024 | receiver loop | 0 | | | | | |
| lab-mio-c1-s1024 | slow samples | 0 | | | | | |
| lab-mio-c16-s1024 | idle spare core | 1 | :38 | 1 (100%) | 1 of 2 | 395.34 | no |
| lab-mio-c16-s1024 | receiver loop | 0 | | | | | |
| lab-mio-c16-s1024 | slow samples | 0 | | | | | |
| lab-mio-c256-s1024 | idle spare core | 0 | | | | | |
| lab-mio-c256-s1024 | receiver loop | 0 | | | | | |
| lab-mio-c256-s1024 | slow samples | 0 | | | | | |
| lab-mio-c4-s1024 | idle spare core | 1 | :29 | 1 (100%) | 1 of 2 | 334.00 | no |
| lab-mio-c4-s1024 | receiver loop | 0 | | | | | |
| lab-mio-c4-s1024 | slow samples | 0 | | | | | |
| lab-mio-c64-s1024 | idle spare core | 0 | | | | | |
| lab-mio-c64-s1024 | receiver loop | 0 | | | | | |
| lab-mio-c64-s1024 | slow samples | 0 | | | | | |
| lab-recv-busy-c1-s1024 | idle spare core | 0 | | | | | |
| lab-recv-busy-c1-s1024 | receiver loop | 0 | | | | | |
| lab-recv-busy-c1-s1024 | slow samples | 0 | | | | | |
| lab-recv-busy-c16-s1024 | idle spare core | 0 | | | | | |
| lab-recv-busy-c16-s1024 | receiver loop | 0 | | | | | |
| lab-recv-busy-c16-s1024 | slow samples | 0 | | | | | |
| lab-recv-busy-c256-s1024 | idle spare core | 0 | | | | | |
| lab-recv-busy-c256-s1024 | receiver loop | 5 | :04 | 2 (40%) | 2 of 2 | 317.38 | no |
| lab-recv-busy-c256-s1024 | slow samples | 1 | :04 | 1 (100%) | 1 of 2 | 309.15 | no |
| lab-recv-busy-c4-s1024 | idle spare core | 0 | | | | | |
| lab-recv-busy-c4-s1024 | receiver loop | 0 | | | | | |
| lab-recv-busy-c4-s1024 | slow samples | 0 | | | | | |
| lab-recv-busy-c64-s1024 | idle spare core | 1 | :24 | 1 (100%) | 1 of 2 | 312.28 | no |
| lab-recv-busy-c64-s1024 | receiver loop | 0 | | | | | |
| lab-recv-busy-c64-s1024 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c1-s1024 | idle spare core | 0 | | | | | |
| lab-recv-busy-pq-c1-s1024 | receiver loop | 0 | | | | | |
| lab-recv-busy-pq-c1-s1024 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c16-s1024 | idle spare core | 1 | :05 | 1 (100%) | 1 of 2 | 311.14 | no |
| lab-recv-busy-pq-c16-s1024 | receiver loop | 0 | | | | | |
| lab-recv-busy-pq-c16-s1024 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c256-s1024 | idle spare core | 0 | | | | | |
| lab-recv-busy-pq-c256-s1024 | receiver loop | 0 | | | | | |
| lab-recv-busy-pq-c256-s1024 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c4-s1024 | idle spare core | 1 | :01 | 1 (100%) | 1 of 2 | 365.36 | no |
| lab-recv-busy-pq-c4-s1024 | receiver loop | 0 | | | | | |
| lab-recv-busy-pq-c4-s1024 | slow samples | 0 | | | | | |
| lab-recv-busy-pq-c64-s1024 | idle spare core | 0 | | | | | |
| lab-recv-busy-pq-c64-s1024 | receiver loop | 0 | | | | | |
| lab-recv-busy-pq-c64-s1024 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c1-s1024 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-c1-s1024 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-c1-s1024 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c16-s1024 | idle spare core | 1 | :56 | 1 (100%) | 1 of 2 | 354.22 | no |
| lab-uring-sqpoll-c16-s1024 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-c16-s1024 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c256-s1024 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-c256-s1024 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-c256-s1024 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c4-s1024 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-c4-s1024 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-c4-s1024 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-c64-s1024 | idle spare core | 0 | | | | | |
| lab-uring-sqpoll-c64-s1024 | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-c64-s1024 | slow samples | 0 | | | | | |

# Every run in detail

## m1/lab-epoll-spin-c1-s1024: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 6137704 | 2237498 | 2.74 | 244119290 | 39.77 | 4068655 | 4031363 | 0.0 | 39.9 | false | 0 | 0 | 2720.5 | 193188 / 281 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8196 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1374845696 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1390702 of 5923844 in 1-msg reads) | 1390702 | 4.68 | 4.76 | 6.28 | 9.72 | 17.51 | 31.18 | 48.05 |

#### Stalls: m1/lab-epoll-spin-c1-s1024, loop 0 on CPU 3

241881792 iterations, longest 2720.5 µs; over 5 / 20 / 100 µs: 193188 / 281 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 195 | 5161.8 | 45.8 |
| loop work | 85 | 4980.5 | 2720.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.1 | 26.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2720.5 | 9455 | loop work (9455 items) |  |
| 2 | 72.3 | 267 | loop work (267 items) |  |
| 3 | 48.4 | 5 | loop work (5 items) |  |
| 4 | 45.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 42.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 40.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 39.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 39.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 37.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 985 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-epoll-spin-c16-s1024: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 6137728 | 3747219 | 1.64 | 230943763 | 37.63 | 3849063 | 3786609 | 0.0 | 42.8 | false | 0 | 0 | 3235.5 | 319219 / 1575 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8199 | 0 |
| epoll-spin | 1 | 8197 | 0 |
| epoll-spin | 2 | 8196 | 4 |
| epoll-spin | 3 | 8198 | 4 |
| epoll-spin | 4 | 8199 | 0 |
| epoll-spin | 5 | 8198 | 4 |
| epoll-spin | 6 | 8194 | 4 |
| epoll-spin | 7 | 8198 | 4 |
| epoll-spin | 8 | 8193 | 0 |
| epoll-spin | 9 | 8199 | 0 |
| epoll-spin | 10 | 8198 | 4 |
| epoll-spin | 11 | 8194 | 4 |
| epoll-spin | 12 | 8198 | 4 |
| epoll-spin | 13 | 8200 | 4 |
| epoll-spin | 14 | 8199 | 0 |
| epoll-spin | 15 | 8197 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1374851072 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (3097268 of 5923974 in 1-msg reads) | 3097268 | 4.48 | 3.45 | 7.65 | 14.10 | 23.80 | 36.96 | 70.18 |

#### Stalls: m1/lab-epoll-spin-c16-s1024, loop 0 on CPU 3

227196544 iterations, longest 3235.5 µs; over 5 / 20 / 100 µs: 319219 / 1575 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1290 | 34455.0 | 63.9 |
| loop work | 283 | 10675.5 | 3235.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 53.9 | 31.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3235.5 | 9647 | loop work (9647 items) |  |
| 2 | 83.8 | 212 | loop work (212 items) |  |
| 3 | 63.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 57.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 57.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 56.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 54.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 52.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 51.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 50.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-epoll-spin-c256-s1024: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 6137871 | 3854359 | 1.59 | 128500567 | 20.94 | 2141676 | 2077437 | 0.0 | 31.1 | false | 0 | 0 | 1141.8 | 372705 / 2408 / 5 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8193 | 0 |
| epoll-spin | 1 | 8196 | 4 |
| epoll-spin | 2 | 8197 | 0 |
| epoll-spin | 3 | 8200 | 4 |
| epoll-spin | 4 | 8196 | 4 |
| epoll-spin | 5 | 8195 | 0 |
| epoll-spin | 6 | 8198 | 4 |
| epoll-spin | 7 | 8194 | 4 |
| epoll-spin | 8 | 8194 | 4 |
| epoll-spin | 9 | 8196 | 4 |
| epoll-spin | 10 | 8199 | 0 |
| epoll-spin | 11 | 8196 | 4 |
| epoll-spin | 12 | 8200 | 4 |
| epoll-spin | 13 | 8195 | 0 |
| epoll-spin | 14 | 8199 | 0 |
| epoll-spin | 15 | 8193 | 0 |
| epoll-spin | 16 | 8197 | 0 |
| epoll-spin | 17 | 8193 | 0 |
| epoll-spin | 18 | 8193 | 0 |
| epoll-spin | 19 | 8199 | 0 |
| epoll-spin | 20 | 8193 | 0 |
| epoll-spin | 21 | 8197 | 0 |
| epoll-spin | 22 | 8193 | 0 |
| epoll-spin | 23 | 8199 | 0 |
| epoll-spin | 24 | 8200 | 4 |
| epoll-spin | 25 | 8197 | 0 |
| epoll-spin | 26 | 8196 | 4 |
| epoll-spin | 27 | 8197 | 0 |
| epoll-spin | 28 | 8194 | 4 |
| epoll-spin | 29 | 8197 | 0 |
| epoll-spin | 30 | 8198 | 4 |
| epoll-spin | 31 | 8199 | 0 |
| epoll-spin | 32 | 8199 | 0 |
| epoll-spin | 33 | 8195 | 0 |
| epoll-spin | 34 | 8196 | 4 |
| epoll-spin | 35 | 8194 | 4 |
| epoll-spin | 36 | 8199 | 0 |
| epoll-spin | 37 | 8194 | 4 |
| epoll-spin | 38 | 8193 | 0 |
| epoll-spin | 39 | 8197 | 0 |
| epoll-spin | 40 | 8195 | 0 |
| epoll-spin | 41 | 8199 | 0 |
| epoll-spin | 42 | 8197 | 0 |
| epoll-spin | 43 | 8199 | 0 |
| epoll-spin | 44 | 8196 | 4 |
| epoll-spin | 45 | 8196 | 4 |
| epoll-spin | 46 | 8198 | 4 |
| epoll-spin | 47 | 8193 | 0 |
| epoll-spin | 48 | 8193 | 0 |
| epoll-spin | 49 | 8198 | 4 |
| epoll-spin | 50 | 8199 | 0 |
| epoll-spin | 51 | 8197 | 0 |
| epoll-spin | 52 | 8198 | 4 |
| epoll-spin | 53 | 8193 | 0 |
| epoll-spin | 54 | 8197 | 0 |
| epoll-spin | 55 | 8199 | 0 |
| epoll-spin | 56 | 8196 | 4 |
| epoll-spin | 57 | 8193 | 0 |
| epoll-spin | 58 | 8198 | 4 |
| epoll-spin | 59 | 8197 | 0 |
| epoll-spin | 60 | 8196 | 4 |
| epoll-spin | 61 | 8195 | 0 |
| epoll-spin | 62 | 8199 | 0 |
| epoll-spin | 63 | 8200 | 4 |
| epoll-spin | 64 | 8194 | 4 |
| epoll-spin | 65 | 8196 | 4 |
| epoll-spin | 66 | 8193 | 0 |
| epoll-spin | 67 | 8199 | 0 |
| epoll-spin | 68 | 8197 | 0 |
| epoll-spin | 69 | 8197 | 0 |
| epoll-spin | 70 | 8196 | 4 |
| epoll-spin | 71 | 8193 | 0 |
| epoll-spin | 72 | 8197 | 0 |
| epoll-spin | 73 | 8194 | 4 |
| epoll-spin | 74 | 8200 | 4 |
| epoll-spin | 75 | 8199 | 0 |
| epoll-spin | 76 | 8199 | 0 |
| epoll-spin | 77 | 8198 | 4 |
| epoll-spin | 78 | 8200 | 4 |
| epoll-spin | 79 | 8197 | 0 |
| epoll-spin | 80 | 8200 | 4 |
| epoll-spin | 81 | 8193 | 0 |
| epoll-spin | 82 | 8196 | 4 |
| epoll-spin | 83 | 8196 | 4 |
| epoll-spin | 84 | 8197 | 0 |
| epoll-spin | 85 | 8193 | 0 |
| epoll-spin | 86 | 8199 | 0 |
| epoll-spin | 87 | 8193 | 0 |
| epoll-spin | 88 | 8200 | 4 |
| epoll-spin | 89 | 8197 | 0 |
| epoll-spin | 90 | 8194 | 4 |
| epoll-spin | 91 | 8194 | 4 |
| epoll-spin | 92 | 8198 | 4 |
| epoll-spin | 93 | 8193 | 0 |
| epoll-spin | 94 | 8198 | 4 |
| epoll-spin | 95 | 8198 | 4 |
| epoll-spin | 96 | 8200 | 4 |
| epoll-spin | 97 | 8193 | 0 |
| epoll-spin | 98 | 8194 | 4 |
| epoll-spin | 99 | 8193 | 0 |
| epoll-spin | 100 | 8199 | 0 |
| epoll-spin | 101 | 8200 | 4 |
| epoll-spin | 102 | 8194 | 4 |
| epoll-spin | 103 | 8195 | 0 |
| epoll-spin | 104 | 8196 | 4 |
| epoll-spin | 105 | 8194 | 4 |
| epoll-spin | 106 | 8199 | 0 |
| epoll-spin | 107 | 8194 | 4 |
| epoll-spin | 108 | 8197 | 0 |
| epoll-spin | 109 | 8198 | 4 |
| epoll-spin | 110 | 8197 | 0 |
| epoll-spin | 111 | 8198 | 4 |
| epoll-spin | 112 | 8195 | 0 |
| epoll-spin | 113 | 8194 | 4 |
| epoll-spin | 114 | 8195 | 0 |
| epoll-spin | 115 | 8199 | 0 |
| epoll-spin | 116 | 8195 | 0 |
| epoll-spin | 117 | 8193 | 0 |
| epoll-spin | 118 | 8199 | 0 |
| epoll-spin | 119 | 8196 | 4 |
| epoll-spin | 120 | 8199 | 0 |
| epoll-spin | 121 | 8196 | 4 |
| epoll-spin | 122 | 8194 | 4 |
| epoll-spin | 123 | 8198 | 4 |
| epoll-spin | 124 | 8194 | 4 |
| epoll-spin | 125 | 8199 | 0 |
| epoll-spin | 126 | 8194 | 4 |
| epoll-spin | 127 | 8197 | 0 |
| epoll-spin | 128 | 8200 | 4 |
| epoll-spin | 129 | 8193 | 0 |
| epoll-spin | 130 | 8193 | 0 |
| epoll-spin | 131 | 8194 | 4 |
| epoll-spin | 132 | 8200 | 4 |
| epoll-spin | 133 | 8193 | 0 |
| epoll-spin | 134 | 8195 | 0 |
| epoll-spin | 135 | 8199 | 0 |
| epoll-spin | 136 | 8196 | 4 |
| epoll-spin | 137 | 8193 | 0 |
| epoll-spin | 138 | 8193 | 0 |
| epoll-spin | 139 | 8196 | 4 |
| epoll-spin | 140 | 8193 | 0 |
| epoll-spin | 141 | 8195 | 0 |
| epoll-spin | 142 | 8196 | 4 |
| epoll-spin | 143 | 8196 | 4 |
| epoll-spin | 144 | 8193 | 0 |
| epoll-spin | 145 | 8195 | 0 |
| epoll-spin | 146 | 8196 | 4 |
| epoll-spin | 147 | 8193 | 0 |
| epoll-spin | 148 | 8197 | 0 |
| epoll-spin | 149 | 8200 | 4 |
| epoll-spin | 150 | 8194 | 4 |
| epoll-spin | 151 | 8194 | 4 |
| epoll-spin | 152 | 8199 | 0 |
| epoll-spin | 153 | 8196 | 4 |
| epoll-spin | 154 | 8199 | 0 |
| epoll-spin | 155 | 8200 | 4 |
| epoll-spin | 156 | 8194 | 4 |
| epoll-spin | 157 | 8196 | 4 |
| epoll-spin | 158 | 8196 | 4 |
| epoll-spin | 159 | 8194 | 4 |
| epoll-spin | 160 | 8198 | 4 |
| epoll-spin | 161 | 8200 | 4 |
| epoll-spin | 162 | 8197 | 0 |
| epoll-spin | 163 | 8199 | 0 |
| epoll-spin | 164 | 8195 | 0 |
| epoll-spin | 165 | 8194 | 4 |
| epoll-spin | 166 | 8198 | 4 |
| epoll-spin | 167 | 8200 | 4 |
| epoll-spin | 168 | 8195 | 0 |
| epoll-spin | 169 | 8193 | 0 |
| epoll-spin | 170 | 8194 | 4 |
| epoll-spin | 171 | 8195 | 0 |
| epoll-spin | 172 | 8198 | 4 |
| epoll-spin | 173 | 8199 | 0 |
| epoll-spin | 174 | 8199 | 0 |
| epoll-spin | 175 | 8199 | 0 |
| epoll-spin | 176 | 8197 | 0 |
| epoll-spin | 177 | 8195 | 0 |
| epoll-spin | 178 | 8193 | 0 |
| epoll-spin | 179 | 8193 | 0 |
| epoll-spin | 180 | 8200 | 4 |
| epoll-spin | 181 | 8199 | 0 |
| epoll-spin | 182 | 8195 | 0 |
| epoll-spin | 183 | 8194 | 4 |
| epoll-spin | 184 | 8195 | 0 |
| epoll-spin | 185 | 8196 | 4 |
| epoll-spin | 186 | 8194 | 4 |
| epoll-spin | 187 | 8200 | 4 |
| epoll-spin | 188 | 8197 | 0 |
| epoll-spin | 189 | 8196 | 4 |
| epoll-spin | 190 | 8200 | 4 |
| epoll-spin | 191 | 8199 | 0 |
| epoll-spin | 192 | 8196 | 4 |
| epoll-spin | 193 | 8200 | 4 |
| epoll-spin | 194 | 8196 | 4 |
| epoll-spin | 195 | 8193 | 0 |
| epoll-spin | 196 | 8193 | 0 |
| epoll-spin | 197 | 8197 | 0 |
| epoll-spin | 198 | 8195 | 0 |
| epoll-spin | 199 | 8197 | 0 |
| epoll-spin | 200 | 8193 | 0 |
| epoll-spin | 201 | 8197 | 0 |
| epoll-spin | 202 | 8195 | 0 |
| epoll-spin | 203 | 8199 | 0 |
| epoll-spin | 204 | 8199 | 0 |
| epoll-spin | 205 | 8196 | 4 |
| epoll-spin | 206 | 8195 | 0 |
| epoll-spin | 207 | 8195 | 0 |
| epoll-spin | 208 | 8193 | 0 |
| epoll-spin | 209 | 8195 | 0 |
| epoll-spin | 210 | 8197 | 0 |
| epoll-spin | 211 | 8195 | 0 |
| epoll-spin | 212 | 8193 | 0 |
| epoll-spin | 213 | 8199 | 0 |
| epoll-spin | 214 | 8196 | 4 |
| epoll-spin | 215 | 8195 | 0 |
| epoll-spin | 216 | 8196 | 4 |
| epoll-spin | 217 | 8193 | 0 |
| epoll-spin | 218 | 8200 | 4 |
| epoll-spin | 219 | 8195 | 0 |
| epoll-spin | 220 | 8196 | 4 |
| epoll-spin | 221 | 8196 | 4 |
| epoll-spin | 222 | 8196 | 4 |
| epoll-spin | 223 | 8196 | 4 |
| epoll-spin | 224 | 8194 | 4 |
| epoll-spin | 225 | 8198 | 4 |
| epoll-spin | 226 | 8199 | 0 |
| epoll-spin | 227 | 8200 | 4 |
| epoll-spin | 228 | 8196 | 4 |
| epoll-spin | 229 | 8194 | 4 |
| epoll-spin | 230 | 8199 | 0 |
| epoll-spin | 231 | 8198 | 4 |
| epoll-spin | 232 | 8197 | 0 |
| epoll-spin | 233 | 8195 | 0 |
| epoll-spin | 234 | 8197 | 0 |
| epoll-spin | 235 | 8200 | 4 |
| epoll-spin | 236 | 8196 | 4 |
| epoll-spin | 237 | 8196 | 4 |
| epoll-spin | 238 | 8193 | 0 |
| epoll-spin | 239 | 8193 | 0 |
| epoll-spin | 240 | 8194 | 4 |
| epoll-spin | 241 | 8193 | 0 |
| epoll-spin | 242 | 8194 | 4 |
| epoll-spin | 243 | 8200 | 4 |
| epoll-spin | 244 | 8194 | 4 |
| epoll-spin | 245 | 8199 | 0 |
| epoll-spin | 246 | 8200 | 4 |
| epoll-spin | 247 | 8198 | 4 |
| epoll-spin | 248 | 8194 | 4 |
| epoll-spin | 249 | 8193 | 0 |
| epoll-spin | 250 | 8195 | 0 |
| epoll-spin | 251 | 8194 | 4 |
| epoll-spin | 252 | 8193 | 0 |
| epoll-spin | 253 | 8195 | 0 |
| epoll-spin | 254 | 8194 | 4 |
| epoll-spin | 255 | 8197 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1374883104 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (3194159 of 5923963 in 1-msg reads) | 3194159 | 4.58 | 3.45 | 7.87 | 16.03 | 28.55 | 40.35 | 72.82 |

#### Stalls: m1/lab-epoll-spin-c256-s1024, loop 0 on CPU 3

124646208 iterations, longest 1141.8 µs; over 5 / 20 / 100 µs: 372705 / 2408 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1798 | 47743.7 | 56.9 |
| loop work | 609 | 19567.6 | 1141.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 27.1 | 27.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1141.8 | 2756 | loop work (2756 items) |  |
| 2 | 1018.6 | 2518 | loop work (2518 items) |  |
| 3 | 941.3 | 2380 | loop work (2380 items) |  |
| 4 | 824.9 | 2137 | loop work (2137 items) |  |
| 5 | 150.0 | 193 | loop work (193 items) |  |
| 6 | 75.1 | 72 | loop work (72 items) |  |
| 7 | 59.2 | 8 | loop work (8 items) |  |
| 8 | 56.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 55.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 55.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 6 |
| TLB TLB shootdowns | 1 |

## m1/lab-epoll-spin-c4-s1024: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 6137705 | 3312211 | 1.85 | 234996307 | 38.29 | 3916605 | 3861402 | 0.0 | 42.4 | false | 0 | 0 | 3029.7 | 280777 / 1167 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8199 | 0 |
| epoll-spin | 1 | 8194 | 4 |
| epoll-spin | 2 | 8197 | 0 |
| epoll-spin | 3 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1374845920 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (2596392 of 5923967 in 1-msg reads) | 2596392 | 4.02 | 3.20 | 6.56 | 11.38 | 19.20 | 34.21 | 59.98 |

#### Stalls: m1/lab-epoll-spin-c4-s1024, loop 0 on CPU 3

231684096 iterations, longest 3029.7 µs; over 5 / 20 / 100 µs: 280777 / 1167 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 822 | 22057.3 | 57.1 |
| loop work | 344 | 12547.8 | 3029.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 32.0 | 32.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3029.7 | 9570 | loop work (9570 items) | local timer tick 15, timer tick_nohz_handler 8, softirq RCU 5, ipi_send_cpu 0 |
| 2 | 63.7 | 163 | loop work (163 items) |  |
| 3 | 57.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 55.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 49.5 | 3 | loop work (3 items) |  |
| 6 | 46.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 45.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 44.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 43.1 | 19 | loop work (19 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-epoll-spin-c64-s1024: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m1/lab-epoll-spin-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 6137728 | 3852251 | 1.59 | 208971035 | 34.05 | 3482851 | 3418646 | 0.0 | 40.5 | false | 0 | 0 | 3461.0 | 336203 / 1791 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8200 | 4 |
| epoll-spin | 1 | 8193 | 0 |
| epoll-spin | 2 | 8198 | 4 |
| epoll-spin | 3 | 8195 | 0 |
| epoll-spin | 4 | 8197 | 0 |
| epoll-spin | 5 | 8195 | 0 |
| epoll-spin | 6 | 8200 | 4 |
| epoll-spin | 7 | 8200 | 4 |
| epoll-spin | 8 | 8199 | 0 |
| epoll-spin | 9 | 8197 | 0 |
| epoll-spin | 10 | 8199 | 0 |
| epoll-spin | 11 | 8200 | 4 |
| epoll-spin | 12 | 8198 | 4 |
| epoll-spin | 13 | 8195 | 0 |
| epoll-spin | 14 | 8195 | 0 |
| epoll-spin | 15 | 8200 | 4 |
| epoll-spin | 16 | 8198 | 4 |
| epoll-spin | 17 | 8199 | 0 |
| epoll-spin | 18 | 8198 | 4 |
| epoll-spin | 19 | 8199 | 0 |
| epoll-spin | 20 | 8200 | 4 |
| epoll-spin | 21 | 8195 | 0 |
| epoll-spin | 22 | 8195 | 0 |
| epoll-spin | 23 | 8193 | 0 |
| epoll-spin | 24 | 8199 | 0 |
| epoll-spin | 25 | 8200 | 4 |
| epoll-spin | 26 | 8193 | 0 |
| epoll-spin | 27 | 8200 | 4 |
| epoll-spin | 28 | 8197 | 0 |
| epoll-spin | 29 | 8196 | 4 |
| epoll-spin | 30 | 8196 | 4 |
| epoll-spin | 31 | 8197 | 0 |
| epoll-spin | 32 | 8194 | 4 |
| epoll-spin | 33 | 8198 | 4 |
| epoll-spin | 34 | 8196 | 4 |
| epoll-spin | 35 | 8194 | 4 |
| epoll-spin | 36 | 8198 | 4 |
| epoll-spin | 37 | 8200 | 4 |
| epoll-spin | 38 | 8197 | 0 |
| epoll-spin | 39 | 8193 | 0 |
| epoll-spin | 40 | 8198 | 4 |
| epoll-spin | 41 | 8194 | 4 |
| epoll-spin | 42 | 8194 | 4 |
| epoll-spin | 43 | 8193 | 0 |
| epoll-spin | 44 | 8195 | 0 |
| epoll-spin | 45 | 8195 | 0 |
| epoll-spin | 46 | 8199 | 0 |
| epoll-spin | 47 | 8194 | 4 |
| epoll-spin | 48 | 8197 | 0 |
| epoll-spin | 49 | 8200 | 4 |
| epoll-spin | 50 | 8193 | 0 |
| epoll-spin | 51 | 8196 | 4 |
| epoll-spin | 52 | 8194 | 4 |
| epoll-spin | 53 | 8195 | 0 |
| epoll-spin | 54 | 8198 | 4 |
| epoll-spin | 55 | 8200 | 4 |
| epoll-spin | 56 | 8198 | 4 |
| epoll-spin | 57 | 8197 | 0 |
| epoll-spin | 58 | 8193 | 0 |
| epoll-spin | 59 | 8200 | 4 |
| epoll-spin | 60 | 8196 | 4 |
| epoll-spin | 61 | 8200 | 4 |
| epoll-spin | 62 | 8198 | 4 |
| epoll-spin | 63 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1374851072 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (3193128 of 5923975 in 1-msg reads) | 3193128 | 4.44 | 3.42 | 7.66 | 13.81 | 23.73 | 37.37 | 68.42 |

#### Stalls: m1/lab-epoll-spin-c64-s1024, loop 0 on CPU 3

205118784 iterations, longest 3461.0 µs; over 5 / 20 / 100 µs: 336203 / 1791 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1540 | 41868.7 | 67.7 |
| loop work | 250 | 10121.5 | 3461.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 24.9 | 24.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3461.0 | 9700 | loop work (9700 items) |  |
| 2 | 113.8 | 178 | loop work (178 items) |  |
| 3 | 67.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 61.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 60.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 59.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 58.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 56.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 55.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 55.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## m1/lab-mio-c1-s1024: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m1/lab-mio-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 6137689 | 0 | 0.00 | 254987711 | 41.54 | 4249795 | 4186074 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

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

#### Stalls: m1/lab-mio-c1-s1024, loop 0 on CPU 3

251164468 iterations, longest 7010.3 µs; over 5 / 20 / 100 µs: 612577 / 10816 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 10807 | 245899.2 | 7010.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 171.7 | 23.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.0 | 33.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 7010.3 | 3073 | loop work (3073 items) |  |
| 2 | 57.5 | 111 | loop work (111 items) |  |
| 3 | 50.7 | 66 | loop work (66 items) |  |
| 4 | 42.7 | 62 | loop work (62 items) |  |
| 5 | 42.7 | 56 | loop work (56 items) |  |
| 6 | 42.7 | 81 | loop work (81 items) |  |
| 7 | 42.4 | 88 | loop work (88 items) |  |
| 8 | 41.8 | 48 | loop work (48 items) |  |
| 9 | 41.6 | 87 | loop work (87 items) |  |
| 10 | 41.3 | 60 | loop work (60 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| LOC Local timer interrupts | 18 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

## m1/lab-mio-c16-s1024: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m1/lab-mio-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 6137701 | 0 | 0.00 | 241762574 | 39.39 | 4029376 | 3908397 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

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

#### Stalls: m1/lab-mio-c16-s1024, loop 0 on CPU 3

234503820 iterations, longest 10815.3 µs; over 5 / 20 / 100 µs: 478234 / 1719 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1250 | 33563.1 | 63.1 |
| loop work | 468 | 22484.9 | 10815.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.2 | 33.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 10815.3 | 9710 | loop work (9710 items) |  |
| 2 | 126.4 | 216 | loop work (216 items) |  |
| 3 | 63.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 62.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 55.0 | 3 | loop work (3 items) |  |
| 6 | 52.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 51.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 49.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 46.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 45.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 51 |
| LOC Local timer interrupts | 8 |
| RES Rescheduling interrupts | 2 |

## m1/lab-mio-c256-s1024: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m1/lab-mio-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 6137751 | 0 | 0.00 | 241342332 | 39.32 | 4022372 | 3897324 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

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

#### Stalls: m1/lab-mio-c256-s1024, loop 0 on CPU 3

233839448 iterations, longest 11232.3 µs; over 5 / 20 / 100 µs: 469940 / 4316 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 3129 | 91047.3 | 11232.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1186 | 31736.2 | 61.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.0 | 23.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 11232.3 | 9856 | loop work (9856 items) |  |
| 2 | 314.4 | 253 | loop work (253 items) |  |
| 3 | 75.0 | 23 | loop work (23 items) |  |
| 4 | 71.2 | 13 | loop work (13 items) |  |
| 5 | 66.6 | 20 | loop work (20 items) | ipi_send_cpu 0 |
| 6 | 64.3 | 7 | loop work (7 items) |  |
| 7 | 62.3 | 7 | loop work (7 items) |  |
| 8 | 61.7 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 60.2 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 58.7 | 7 | loop work (7 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 749 |
| LOC Local timer interrupts | 21 |
| RES Rescheduling interrupts | 1 |

## m1/lab-mio-c4-s1024: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m1/lab-mio-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 6137722 | 0 | 0.00 | 247160996 | 40.27 | 4119350 | 4015320 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

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

#### Stalls: m1/lab-mio-c4-s1024, loop 0 on CPU 3

240919204 iterations, longest 10052.1 µs; over 5 / 20 / 100 µs: 495675 / 1637 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1111 | 36316.9 | 10052.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 524 | 13818.5 | 55.4 |
| softirq TIMER | 1 | 210.6 | 210.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.6 | 28.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 10052.1 | 9770 | loop work (9770 items) |  |
| 2 | 210.6 | 7 | softirq TIMER | softirq TIMER 183, local timer tick 12, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 3 | 75.5 | 140 | loop work (140 items) |  |
| 4 | 55.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 51.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 50.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 50.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 47.8 | 36 | loop work (36 items) |  |
| 9 | 46.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 45.7 | 5 | loop work (5 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 16 |
| LOC Local timer interrupts | 10 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

## m1/lab-mio-c64-s1024: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m1/lab-mio-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 6137730 | 0 | 0.00 | 241554929 | 39.36 | 4025915 | 3901187 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

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

#### Stalls: m1/lab-mio-c64-s1024, loop 0 on CPU 3

234071208 iterations, longest 10648.1 µs; over 5 / 20 / 100 µs: 468581 / 1547 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1131 | 30244.5 | 52.2 |
| loop work | 414 | 21788.4 | 10648.1 |
| mostly unexplained (+local timer tick) | 1 | 32.7 | 32.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.4 | 26.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 10648.1 | 9751 | loop work (9751 items) |  |
| 2 | 212.8 | 300 | loop work (300 items) |  |
| 3 | 83.3 | 27 | loop work (27 items) |  |
| 4 | 53.7 | 19 | loop work (19 items) |  |
| 5 | 52.2 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 50.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 50.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 48.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 48.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 48.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 195 |
| LOC Local timer interrupts | 24 |
| RES Rescheduling interrupts | 2 |

## m1/lab-recv-busy-c1-s1024: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2644735 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137696 | 3034305 | 2.02 | 101053184 | 16.46 | 1684220 | 1684220 | 97.0 | 70.1 | true | 0 | 4533 | 2334.9 | 129071 / 6033 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374843904 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (2251222 of 5923927 in 1-msg reads) | 2251222 | 2.17 | 2.07 | 2.36 | 3.88 | 8.31 | 18.86 | 30.92 |

#### Stalls: m1/lab-recv-busy-c1-s1024, loop 0 on CPU 3

101053184 iterations, longest 2334.9 µs; over 5 / 20 / 100 µs: 129071 / 6033 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5204 | 123958.6 | 2334.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 319 | 7129.7 | 35.8 |
| mostly unexplained (+preempted by kworker/3:1) | 283 | 6662.9 | 45.9 |
| softirq NET_RX | 130 | 2950.6 | 30.8 |
| mostly unexplained (+softirq NET_RX) | 72 | 1705.8 | 31.7 |
| mostly unexplained (+irq work (IPI/vector)) | 15 | 394.0 | 35.6 |
| mostly unexplained (+local timer tick) | 10 | 238.2 | 37.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2334.9 | 9447 | loop work (9447 items) |  |
| 2 | 67.6 | 249 | loop work (249 items) |  |
| 3 | 58.5 | 100 | loop work (100 items) | softirq NET_RX 27 |
| 4 | 52.4 | 17 | loop work (17 items) | local timer tick 4, preempted by kworker/3:1 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 5 | 48.7 | 39 | loop work (39 items) | preempted by kworker/3:1 1, softirq NET_RX 1 |
| 6 | 47.9 | 47 | loop work (47 items) | softirq NET_RX 5 |
| 7 | 47.3 | 3 | loop work (3 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 45.9 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 44.4 | 26 | loop work (26 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 43.7 | 26 | loop work (26 items) | softirq NET_RX 6 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4605 |
| IWI IRQ work interrupts | 4550 |
| CAL Function call interrupts | 991 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-c16-s1024: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2925623 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137715 | 3645551 | 1.68 | 99059712 | 16.14 | 1650995 | 103187 | 96.3 | 73.1 | true | 0 | 6070 | 3194.7 | 6191231 / 113401 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 3 |
| recv-busy | 1 | 8197 | 3 |
| recv-busy | 2 | 8199 | 3 |
| recv-busy | 3 | 8198 | 3 |
| recv-busy | 4 | 8199 | 3 |
| recv-busy | 5 | 8194 | 3 |
| recv-busy | 6 | 8196 | 4 |
| recv-busy | 7 | 8196 | 4 |
| recv-busy | 8 | 8195 | 3 |
| recv-busy | 9 | 8193 | 3 |
| recv-busy | 10 | 8196 | 3 |
| recv-busy | 11 | 8194 | 3 |
| recv-busy | 12 | 8194 | 3 |
| recv-busy | 13 | 8193 | 3 |
| recv-busy | 14 | 8194 | 4 |
| recv-busy | 15 | 8195 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374848160 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (3003217 of 5924001 in 1-msg reads) | 3003217 | 5.28 | 4.02 | 9.86 | 18.15 | 31.19 | 43.18 | 75.90 |

#### Stalls: m1/lab-recv-busy-c16-s1024, loop 0 on CPU 3

6191232 iterations, longest 3194.7 µs; over 5 / 20 / 100 µs: 6191231 / 113401 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 102324 | 2718851.7 | 3194.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 8169 | 245891.4 | 69.6 |
| mostly unexplained (+preempted by kworker/3:1) | 1177 | 29963.1 | 55.7 |
| mostly unexplained (+irq work (IPI/vector)) | 737 | 17364.1 | 52.4 |
| mostly unexplained (+softirq NET_RX) | 523 | 15013.5 | 60.2 |
| mostly unexplained (+local timer tick) | 463 | 10893.8 | 55.2 |
| softirq NET_RX | 3 | 116.4 | 42.8 |
| mostly unexplained (+softirq RCU) | 2 | 40.6 | 20.4 |
| mostly unexplained (+ipi_send_cpu) | 1 | 33.8 | 33.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 31.6 | 31.6 |
| mostly unexplained (+softirq TIMER) | 1 | 29.1 | 29.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3194.7 | 9590 | loop work (9590 items) |  |
| 2 | 102.0 | 45 | loop work (45 items) | softirq NET_RX 9 |
| 3 | 98.2 | 38 | loop work (38 items) |  |
| 4 | 90.0 | 39 | loop work (39 items) | softirq NET_RX 1 |
| 5 | 87.3 | 35 | loop work (35 items) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 86.6 | 57 | loop work (57 items) |  |
| 7 | 86.4 | 40 | loop work (40 items) |  |
| 8 | 86.3 | 41 | loop work (41 items) |  |
| 9 | 85.9 | 41 | loop work (41 items) |  |
| 10 | 84.4 | 57 | loop work (57 items) | softirq NET_RX 11 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6112 |
| LOC Local timer interrupts | 5887 |
| CAL Function call interrupts | 991 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-c256-s1024: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2640836 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6138778 | 3418932 | 1.80 | 98500608 | 16.05 | 1641677 | 6413 | 96.5 | 73.2 | true | 0 | 6352 | 3655.0 | 384767 / 384767 / 384767 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 3 |
| recv-busy | 1 | 8200 | 3 |
| recv-busy | 2 | 8199 | 3 |
| recv-busy | 3 | 8193 | 3 |
| recv-busy | 4 | 8200 | 3 |
| recv-busy | 5 | 8199 | 3 |
| recv-busy | 6 | 8195 | 3 |
| recv-busy | 7 | 8200 | 3 |
| recv-busy | 8 | 8193 | 0 |
| recv-busy | 9 | 8197 | 3 |
| recv-busy | 10 | 8196 | 3 |
| recv-busy | 11 | 8196 | 4 |
| recv-busy | 12 | 8200 | 3 |
| recv-busy | 13 | 8197 | 3 |
| recv-busy | 14 | 8196 | 3 |
| recv-busy | 15 | 8195 | 3 |
| recv-busy | 16 | 8200 | 3 |
| recv-busy | 17 | 8193 | 3 |
| recv-busy | 18 | 8200 | 3 |
| recv-busy | 19 | 8200 | 4 |
| recv-busy | 20 | 8198 | 4 |
| recv-busy | 21 | 8193 | 3 |
| recv-busy | 22 | 8198 | 3 |
| recv-busy | 23 | 8196 | 3 |
| recv-busy | 24 | 8198 | 4 |
| recv-busy | 25 | 8200 | 3 |
| recv-busy | 26 | 8198 | 3 |
| recv-busy | 27 | 8197 | 3 |
| recv-busy | 28 | 8199 | 3 |
| recv-busy | 29 | 8199 | 3 |
| recv-busy | 30 | 8198 | 3 |
| recv-busy | 31 | 8194 | 3 |
| recv-busy | 32 | 8199 | 3 |
| recv-busy | 33 | 8197 | 3 |
| recv-busy | 34 | 8199 | 3 |
| recv-busy | 35 | 8193 | 0 |
| recv-busy | 36 | 8197 | 3 |
| recv-busy | 37 | 8200 | 3 |
| recv-busy | 38 | 8200 | 3 |
| recv-busy | 39 | 8200 | 3 |
| recv-busy | 40 | 8193 | 3 |
| recv-busy | 41 | 8198 | 3 |
| recv-busy | 42 | 8195 | 3 |
| recv-busy | 43 | 8193 | 3 |
| recv-busy | 44 | 8193 | 0 |
| recv-busy | 45 | 8196 | 3 |
| recv-busy | 46 | 8195 | 3 |
| recv-busy | 47 | 8200 | 3 |
| recv-busy | 48 | 8200 | 3 |
| recv-busy | 49 | 8200 | 3 |
| recv-busy | 50 | 8198 | 4 |
| recv-busy | 51 | 8196 | 3 |
| recv-busy | 52 | 8199 | 0 |
| recv-busy | 53 | 8194 | 3 |
| recv-busy | 54 | 8199 | 3 |
| recv-busy | 55 | 8195 | 0 |
| recv-busy | 56 | 8194 | 4 |
| recv-busy | 57 | 8196 | 3 |
| recv-busy | 58 | 8199 | 0 |
| recv-busy | 59 | 8197 | 0 |
| recv-busy | 60 | 8196 | 4 |
| recv-busy | 61 | 8194 | 4 |
| recv-busy | 62 | 8195 | 3 |
| recv-busy | 63 | 8196 | 4 |
| recv-busy | 64 | 8193 | 0 |
| recv-busy | 65 | 8195 | 3 |
| recv-busy | 66 | 8194 | 3 |
| recv-busy | 67 | 8197 | 3 |
| recv-busy | 68 | 8196 | 3 |
| recv-busy | 69 | 8197 | 0 |
| recv-busy | 70 | 8196 | 3 |
| recv-busy | 71 | 8194 | 3 |
| recv-busy | 72 | 8195 | 0 |
| recv-busy | 73 | 8193 | 3 |
| recv-busy | 74 | 8200 | 3 |
| recv-busy | 75 | 8198 | 4 |
| recv-busy | 76 | 8200 | 3 |
| recv-busy | 77 | 8197 | 3 |
| recv-busy | 78 | 8194 | 4 |
| recv-busy | 79 | 8195 | 3 |
| recv-busy | 80 | 8194 | 3 |
| recv-busy | 81 | 8196 | 3 |
| recv-busy | 82 | 8198 | 3 |
| recv-busy | 83 | 8193 | 3 |
| recv-busy | 84 | 8195 | 3 |
| recv-busy | 85 | 8195 | 3 |
| recv-busy | 86 | 8199 | 0 |
| recv-busy | 87 | 8196 | 4 |
| recv-busy | 88 | 8198 | 3 |
| recv-busy | 89 | 8195 | 3 |
| recv-busy | 90 | 8195 | 0 |
| recv-busy | 91 | 8200 | 4 |
| recv-busy | 92 | 8193 | 3 |
| recv-busy | 93 | 8199 | 3 |
| recv-busy | 94 | 8200 | 4 |
| recv-busy | 95 | 8197 | 3 |
| recv-busy | 96 | 8193 | 0 |
| recv-busy | 97 | 8197 | 0 |
| recv-busy | 98 | 8199 | 0 |
| recv-busy | 99 | 8196 | 3 |
| recv-busy | 100 | 8195 | 3 |
| recv-busy | 101 | 8197 | 3 |
| recv-busy | 102 | 8198 | 4 |
| recv-busy | 103 | 8196 | 3 |
| recv-busy | 104 | 8193 | 0 |
| recv-busy | 105 | 8198 | 3 |
| recv-busy | 106 | 8194 | 3 |
| recv-busy | 107 | 8199 | 3 |
| recv-busy | 108 | 8198 | 4 |
| recv-busy | 109 | 8198 | 4 |
| recv-busy | 110 | 8195 | 3 |
| recv-busy | 111 | 8195 | 3 |
| recv-busy | 112 | 8194 | 4 |
| recv-busy | 113 | 8196 | 3 |
| recv-busy | 114 | 8193 | 3 |
| recv-busy | 115 | 8196 | 3 |
| recv-busy | 116 | 8195 | 3 |
| recv-busy | 117 | 8196 | 3 |
| recv-busy | 118 | 8194 | 3 |
| recv-busy | 119 | 8197 | 0 |
| recv-busy | 120 | 8197 | 0 |
| recv-busy | 121 | 8195 | 3 |
| recv-busy | 122 | 8200 | 3 |
| recv-busy | 123 | 8199 | 0 |
| recv-busy | 124 | 8199 | 3 |
| recv-busy | 125 | 8199 | 3 |
| recv-busy | 126 | 8198 | 4 |
| recv-busy | 127 | 8194 | 4 |
| recv-busy | 128 | 8193 | 0 |
| recv-busy | 129 | 8194 | 3 |
| recv-busy | 130 | 8200 | 4 |
| recv-busy | 131 | 8200 | 3 |
| recv-busy | 132 | 8197 | 0 |
| recv-busy | 133 | 8196 | 4 |
| recv-busy | 134 | 8196 | 4 |
| recv-busy | 135 | 8194 | 4 |
| recv-busy | 136 | 8200 | 3 |
| recv-busy | 137 | 8196 | 3 |
| recv-busy | 138 | 8200 | 3 |
| recv-busy | 139 | 8200 | 3 |
| recv-busy | 140 | 8199 | 0 |
| recv-busy | 141 | 8195 | 0 |
| recv-busy | 142 | 8198 | 4 |
| recv-busy | 143 | 8198 | 3 |
| recv-busy | 144 | 8195 | 3 |
| recv-busy | 145 | 8193 | 0 |
| recv-busy | 146 | 8196 | 4 |
| recv-busy | 147 | 8193 | 3 |
| recv-busy | 148 | 8196 | 3 |
| recv-busy | 149 | 8194 | 3 |
| recv-busy | 150 | 8193 | 3 |
| recv-busy | 151 | 8199 | 3 |
| recv-busy | 152 | 8193 | 3 |
| recv-busy | 153 | 8196 | 3 |
| recv-busy | 154 | 8200 | 4 |
| recv-busy | 155 | 8196 | 3 |
| recv-busy | 156 | 8195 | 3 |
| recv-busy | 157 | 8194 | 3 |
| recv-busy | 158 | 8193 | 3 |
| recv-busy | 159 | 8200 | 3 |
| recv-busy | 160 | 8200 | 3 |
| recv-busy | 161 | 8198 | 4 |
| recv-busy | 162 | 8199 | 0 |
| recv-busy | 163 | 8193 | 0 |
| recv-busy | 164 | 8199 | 3 |
| recv-busy | 165 | 8199 | 3 |
| recv-busy | 166 | 8199 | 3 |
| recv-busy | 167 | 8195 | 0 |
| recv-busy | 168 | 8193 | 3 |
| recv-busy | 169 | 8200 | 3 |
| recv-busy | 170 | 8198 | 4 |
| recv-busy | 171 | 8200 | 3 |
| recv-busy | 172 | 8195 | 3 |
| recv-busy | 173 | 8200 | 3 |
| recv-busy | 174 | 8200 | 3 |
| recv-busy | 175 | 8194 | 4 |
| recv-busy | 176 | 8193 | 3 |
| recv-busy | 177 | 8195 | 3 |
| recv-busy | 178 | 8197 | 0 |
| recv-busy | 179 | 8198 | 4 |
| recv-busy | 180 | 8194 | 3 |
| recv-busy | 181 | 8196 | 3 |
| recv-busy | 182 | 8196 | 3 |
| recv-busy | 183 | 8194 | 3 |
| recv-busy | 184 | 8197 | 3 |
| recv-busy | 185 | 8200 | 3 |
| recv-busy | 186 | 8196 | 3 |
| recv-busy | 187 | 8195 | 3 |
| recv-busy | 188 | 8199 | 3 |
| recv-busy | 189 | 8200 | 3 |
| recv-busy | 190 | 8195 | 3 |
| recv-busy | 191 | 8195 | 3 |
| recv-busy | 192 | 8193 | 3 |
| recv-busy | 193 | 8195 | 3 |
| recv-busy | 194 | 8197 | 3 |
| recv-busy | 195 | 8194 | 4 |
| recv-busy | 196 | 8196 | 3 |
| recv-busy | 197 | 8197 | 3 |
| recv-busy | 198 | 8193 | 3 |
| recv-busy | 199 | 8198 | 3 |
| recv-busy | 200 | 8195 | 0 |
| recv-busy | 201 | 8193 | 3 |
| recv-busy | 202 | 8193 | 3 |
| recv-busy | 203 | 8197 | 3 |
| recv-busy | 204 | 8195 | 0 |
| recv-busy | 205 | 8199 | 0 |
| recv-busy | 206 | 8197 | 0 |
| recv-busy | 207 | 8195 | 3 |
| recv-busy | 208 | 8193 | 0 |
| recv-busy | 209 | 8197 | 0 |
| recv-busy | 210 | 8194 | 3 |
| recv-busy | 211 | 8200 | 4 |
| recv-busy | 212 | 8199 | 3 |
| recv-busy | 213 | 8197 | 3 |
| recv-busy | 214 | 8196 | 3 |
| recv-busy | 215 | 8193 | 3 |
| recv-busy | 216 | 8200 | 3 |
| recv-busy | 217 | 8193 | 0 |
| recv-busy | 218 | 8195 | 3 |
| recv-busy | 219 | 8200 | 3 |
| recv-busy | 220 | 8193 | 3 |
| recv-busy | 221 | 8197 | 0 |
| recv-busy | 222 | 8199 | 3 |
| recv-busy | 223 | 8197 | 3 |
| recv-busy | 224 | 8197 | 0 |
| recv-busy | 225 | 8198 | 3 |
| recv-busy | 226 | 8197 | 0 |
| recv-busy | 227 | 8195 | 3 |
| recv-busy | 228 | 8199 | 0 |
| recv-busy | 229 | 8199 | 3 |
| recv-busy | 230 | 8196 | 3 |
| recv-busy | 231 | 8198 | 3 |
| recv-busy | 232 | 8195 | 3 |
| recv-busy | 233 | 8200 | 3 |
| recv-busy | 234 | 8199 | 0 |
| recv-busy | 235 | 8198 | 3 |
| recv-busy | 236 | 8194 | 4 |
| recv-busy | 237 | 8195 | 3 |
| recv-busy | 238 | 8195 | 3 |
| recv-busy | 239 | 8197 | 0 |
| recv-busy | 240 | 8194 | 4 |
| recv-busy | 241 | 8193 | 3 |
| recv-busy | 242 | 8197 | 3 |
| recv-busy | 243 | 8199 | 3 |
| recv-busy | 244 | 8197 | 0 |
| recv-busy | 245 | 8193 | 3 |
| recv-busy | 246 | 8199 | 3 |
| recv-busy | 247 | 8199 | 0 |
| recv-busy | 248 | 8197 | 3 |
| recv-busy | 249 | 8195 | 0 |
| recv-busy | 250 | 8199 | 3 |
| recv-busy | 251 | 8200 | 3 |
| recv-busy | 252 | 8198 | 3 |
| recv-busy | 253 | 8197 | 0 |
| recv-busy | 254 | 8194 | 4 |
| recv-busy | 255 | 8195 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1375086272 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (2946431 of 5924872 in 1-msg reads) | 2946431 | 78.10 | 77.15 | 139.92 | 181.93 | 221.17 | 247.05 | 294.36 |

#### Stalls: m1/lab-recv-busy-c256-s1024, loop 0 on CPU 3

384768 iterations, longest 3655.0 µs; over 5 / 20 / 100 µs: 384767 / 384767 / 384767

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 227336 | 37078106.9 | 3655.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 134334 | 19399982.0 | 231.0 |
| mostly unexplained (+softirq NET_RX) | 18200 | 2713859.5 | 246.5 |
| mostly unexplained (+local timer tick) | 2974 | 468173.8 | 226.1 |
| mostly unexplained (+preempted by kworker/3:1) | 1433 | 235558.0 | 257.9 |
| mostly unexplained (+irq work (IPI/vector)) | 470 | 75923.8 | 221.4 |
| mostly unexplained (+ipi_send_cpu) | 7 | 1101.2 | 181.9 |
| mostly unexplained (+softirq NET_TX) | 6 | 1040.9 | 210.8 |
| mostly unexplained (+call function (IPI/vector)) | 3 | 457.9 | 167.3 |
| mostly unexplained (+softirq RCU) | 2 | 307.5 | 159.3 |
| mostly unexplained (+softirq TASKLET) | 1 | 209.6 | 209.6 |
| mostly unexplained (+softirq TIMER) | 1 | 167.4 | 167.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3655.0 | 9819 | loop work (9819 items) |  |
| 2 | 412.4 | 199 | loop work (199 items) |  |
| 3 | 308.7 | 93 | loop work (93 items) | irq work (IPI/vector) 1, preempted by kworker/3:1 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 304.3 | 109 | loop work (109 items) |  |
| 5 | 297.3 | 57 | loop work (57 items) | preempted by kworker/3:1 4, softirq NET_RX 4, irq work (IPI/vector) 2, workqueue [ena] 2 |
| 6 | 296.7 | 96 | loop work (96 items) | softirq NET_RX 1 |
| 7 | 295.8 | 74 | loop work (74 items) | softirq NET_RX 1 |
| 8 | 294.7 | 101 | loop work (101 items) |  |
| 9 | 294.3 | 97 | loop work (97 items) | softirq NET_RX 1 |
| 10 | 294.1 | 90 | loop work (90 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6397 |
| LOC Local timer interrupts | 6130 |
| CAL Function call interrupts | 995 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-c4-s1024: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3277884 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137694 | 3509227 | 1.75 | 98688000 | 16.08 | 1644800 | 411200 | 96.4 | 72.5 | true | 0 | 5644 | 2532.2 | 887304 / 27676 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 3 |
| recv-busy | 1 | 8193 | 0 |
| recv-busy | 2 | 8194 | 4 |
| recv-busy | 3 | 8199 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374843456 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (2762584 of 5923970 in 1-msg reads) | 2762584 | 2.60 | 2.41 | 2.95 | 7.65 | 17.48 | 26.29 | 56.14 |

#### Stalls: m1/lab-recv-busy-c4-s1024, loop 0 on CPU 3

24672000 iterations, longest 2532.2 µs; over 5 / 20 / 100 µs: 887304 / 27676 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 18469 | 516313.4 | 2532.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 8615 | 202483.3 | 55.4 |
| mostly unexplained (+softirq NET_RX) | 259 | 6189.9 | 46.1 |
| mostly unexplained (+preempted by kworker/3:1) | 251 | 6111.6 | 49.3 |
| mostly unexplained (+irq work (IPI/vector)) | 38 | 1194.6 | 44.4 |
| mostly unexplained (+local timer tick) | 29 | 829.4 | 35.4 |
| softirq NET_RX | 13 | 311.8 | 34.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 36.8 | 36.8 |
| softirq TIMER | 1 | 22.9 | 22.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2532.2 | 9516 | loop work (9516 items) |  |
| 2 | 82.6 | 57 | loop work (57 items) | softirq NET_RX 10 |
| 3 | 77.6 | 42 | loop work (42 items) |  |
| 4 | 70.1 | 44 | loop work (44 items) | softirq NET_RX 2, softirq NET_TX 1 |
| 5 | 69.8 | 30 | loop work (30 items) | softirq NET_RX 2 |
| 6 | 67.6 | 39 | loop work (39 items) |  |
| 7 | 64.6 | 28 | loop work (28 items) |  |
| 8 | 64.5 | 22 | loop work (22 items) |  |
| 9 | 62.4 | 20 | loop work (20 items) |  |
| 10 | 62.2 | 29 | loop work (29 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5669 |
| LOC Local timer interrupts | 5468 |
| CAL Function call interrupts | 991 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-c64-s1024: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2651835 clocksource=tsc

### Loop counters (m1/lab-recv-busy-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137945 | 3519937 | 1.74 | 98299904 | 16.02 | 1638332 | 25599 | 96.4 | 73.2 | true | 0 | 6304 | 3549.9 | 1535935 / 1535935 / 2127 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 4 |
| recv-busy | 1 | 8197 | 0 |
| recv-busy | 2 | 8196 | 3 |
| recv-busy | 3 | 8193 | 0 |
| recv-busy | 4 | 8199 | 3 |
| recv-busy | 5 | 8196 | 3 |
| recv-busy | 6 | 8200 | 3 |
| recv-busy | 7 | 8196 | 3 |
| recv-busy | 8 | 8200 | 3 |
| recv-busy | 9 | 8194 | 4 |
| recv-busy | 10 | 8199 | 3 |
| recv-busy | 11 | 8198 | 4 |
| recv-busy | 12 | 8195 | 3 |
| recv-busy | 13 | 8196 | 3 |
| recv-busy | 14 | 8200 | 3 |
| recv-busy | 15 | 8195 | 3 |
| recv-busy | 16 | 8199 | 3 |
| recv-busy | 17 | 8193 | 3 |
| recv-busy | 18 | 8197 | 3 |
| recv-busy | 19 | 8196 | 4 |
| recv-busy | 20 | 8193 | 3 |
| recv-busy | 21 | 8197 | 3 |
| recv-busy | 22 | 8200 | 4 |
| recv-busy | 23 | 8197 | 0 |
| recv-busy | 24 | 8199 | 3 |
| recv-busy | 25 | 8199 | 3 |
| recv-busy | 26 | 8200 | 3 |
| recv-busy | 27 | 8196 | 3 |
| recv-busy | 28 | 8196 | 3 |
| recv-busy | 29 | 8197 | 3 |
| recv-busy | 30 | 8200 | 3 |
| recv-busy | 31 | 8196 | 3 |
| recv-busy | 32 | 8198 | 3 |
| recv-busy | 33 | 8199 | 3 |
| recv-busy | 34 | 8198 | 4 |
| recv-busy | 35 | 8193 | 3 |
| recv-busy | 36 | 8200 | 3 |
| recv-busy | 37 | 8193 | 3 |
| recv-busy | 38 | 8193 | 3 |
| recv-busy | 39 | 8195 | 3 |
| recv-busy | 40 | 8197 | 3 |
| recv-busy | 41 | 8195 | 3 |
| recv-busy | 42 | 8196 | 3 |
| recv-busy | 43 | 8197 | 0 |
| recv-busy | 44 | 8199 | 3 |
| recv-busy | 45 | 8196 | 3 |
| recv-busy | 46 | 8199 | 3 |
| recv-busy | 47 | 8195 | 3 |
| recv-busy | 48 | 8197 | 3 |
| recv-busy | 49 | 8198 | 4 |
| recv-busy | 50 | 8196 | 3 |
| recv-busy | 51 | 8199 | 3 |
| recv-busy | 52 | 8197 | 3 |
| recv-busy | 53 | 8199 | 3 |
| recv-busy | 54 | 8197 | 3 |
| recv-busy | 55 | 8197 | 3 |
| recv-busy | 56 | 8197 | 0 |
| recv-busy | 57 | 8199 | 0 |
| recv-busy | 58 | 8193 | 3 |
| recv-busy | 59 | 8195 | 3 |
| recv-busy | 60 | 8199 | 0 |
| recv-busy | 61 | 8194 | 4 |
| recv-busy | 62 | 8193 | 0 |
| recv-busy | 63 | 8194 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374899680 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (2964456 of 5924173 in 1-msg reads) | 2964456 | 19.67 | 18.54 | 36.20 | 54.58 | 72.42 | 90.88 | 132.47 |

#### Stalls: m1/lab-recv-busy-c64-s1024, loop 0 on CPU 3

1535936 iterations, longest 3549.9 µs; over 5 / 20 / 100 µs: 1535935 / 1535935 / 2127 (487359 stalls not kept)

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 573942 | 24130811.9 | 3549.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 457571 | 16041131.9 | 100.4 |
| mostly unexplained (+softirq NET_RX) | 13104 | 510637.0 | 95.0 |
| mostly unexplained (+local timer tick) | 2462 | 111801.6 | 90.2 |
| mostly unexplained (+preempted by kworker/3:1) | 1071 | 58083.6 | 95.5 |
| mostly unexplained (+irq work (IPI/vector)) | 406 | 20064.0 | 83.4 |
| mostly unexplained (+ipi_send_cpu) | 7 | 304.8 | 46.3 |
| mostly unexplained (+call function (IPI/vector)) | 4 | 172.3 | 47.7 |
| mostly unexplained (+softirq NET_TX) | 4 | 167.0 | 52.8 |
| mostly unexplained (+softirq RCU) | 3 | 127.5 | 48.3 |
| softirq TIMER | 1 | 108.3 | 108.3 |
| mostly unexplained (+softirq TASKLET) | 1 | 37.5 | 37.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3549.9 | 9749 | loop work (9749 items) |  |
| 2 | 163.4 | 61 | loop work (61 items) |  |
| 3 | 156.3 | 87 | loop work (87 items) |  |
| 4 | 155.6 | 69 | loop work (69 items) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 151.9 | 72 | loop work (72 items) | softirq NET_RX 1 |
| 6 | 151.8 | 36 | loop work (36 items) | softirq NET_RX 1 |
| 7 | 149.9 | 58 | loop work (58 items) | softirq NET_RX 1 |
| 8 | 146.8 | 92 | loop work (92 items) | softirq NET_RX 2 |
| 9 | 146.5 | 71 | loop work (71 items) | softirq NET_RX 2 |
| 10 | 144.9 | 47 | loop work (47 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6346 |
| LOC Local timer interrupts | 6075 |
| CAL Function call interrupts | 991 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c1-s1024: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2731085 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137744 | 3028121 | 2.03 | 140097408 | 22.83 | 2334957 | 1167478 | 95.7 | 63.9 | true | 0 | 3642 | 2470.7 | 253772 / 7057 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374854656 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (2246279 of 5923810 in 1-msg reads) | 2246279 | 2.58 | 2.53 | 2.94 | 4.06 | 10.88 | 18.29 | 32.01 |

#### Stalls: m1/lab-recv-busy-pq-c1-s1024, loop 0 on CPU 3

70048704 iterations, longest 2470.7 µs; over 5 / 20 / 100 µs: 253772 / 7057 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 6561 | 155189.1 | 2470.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 216 | 4871.3 | 40.9 |
| mostly unexplained (+preempted by kworker/3:1) | 186 | 4351.1 | 43.0 |
| softirq NET_RX | 48 | 1062.4 | 30.8 |
| mostly unexplained (+softirq NET_RX) | 26 | 616.4 | 41.0 |
| mostly unexplained (+irq work (IPI/vector)) | 10 | 244.6 | 26.4 |
| mostly unexplained (+local timer tick) | 9 | 208.8 | 26.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 37.9 | 37.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2470.7 | 9498 | loop work (9498 items) |  |
| 2 | 56.3 | 258 | loop work (258 items) |  |
| 3 | 52.5 | 37 | loop work (37 items) | softirq NET_RX 3 |
| 4 | 51.1 | 32 | loop work (32 items) |  |
| 5 | 49.4 | 37 | loop work (37 items) | softirq NET_RX 4 |
| 6 | 48.9 | 31 | loop work (31 items) |  |
| 7 | 46.4 | 18 | loop work (18 items) | softirq NET_RX 15, preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 46.1 | 8 | loop work (8 items) | softirq NET_RX 1 |
| 9 | 45.1 | 40 | loop work (40 items) |  |
| 10 | 44.5 | 21 | loop work (21 items) | softirq NET_RX 12, local timer tick 3, preempted by kworker/3:1 2, irq work (IPI/vector) 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3691 |
| IWI IRQ work interrupts | 3652 |
| CAL Function call interrupts | 992 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c16-s1024: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3092811 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137716 | 3677349 | 1.67 | 104429426 | 17.01 | 1740490 | 213292 | 96.0 | 72.0 | true | 0 | 6066 | 3294.6 | 2861442 / 54222 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 3 |
| recv-busy | 1 | 8195 | 3 |
| recv-busy | 2 | 8193 | 3 |
| recv-busy | 3 | 8196 | 3 |
| recv-busy | 4 | 8200 | 3 |
| recv-busy | 5 | 8196 | 4 |
| recv-busy | 6 | 8193 | 3 |
| recv-busy | 7 | 8196 | 3 |
| recv-busy | 8 | 8198 | 3 |
| recv-busy | 9 | 8198 | 3 |
| recv-busy | 10 | 8195 | 3 |
| recv-busy | 11 | 8196 | 3 |
| recv-busy | 12 | 8197 | 3 |
| recv-busy | 13 | 8193 | 3 |
| recv-busy | 14 | 8200 | 4 |
| recv-busy | 15 | 8199 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374848384 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (3018181 of 5923982 in 1-msg reads) | 3018181 | 4.84 | 4.52 | 7.94 | 16.83 | 29.33 | 41.52 | 67.72 |

#### Stalls: m1/lab-recv-busy-pq-c16-s1024, loop 0 on CPU 3

12797504 iterations, longest 3294.6 µs; over 5 / 20 / 100 µs: 2861442 / 54222 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 46191 | 1293728.3 | 3294.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 7064 | 183572.3 | 51.8 |
| mostly unexplained (+preempted by kworker/3:1) | 655 | 15171.9 | 53.1 |
| mostly unexplained (+softirq NET_RX) | 173 | 4426.2 | 43.8 |
| mostly unexplained (+irq work (IPI/vector)) | 90 | 2293.5 | 44.5 |
| mostly unexplained (+local timer tick) | 41 | 1150.5 | 45.2 |
| softirq NET_RX | 7 | 231.2 | 39.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 31.4 | 31.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3294.6 | 9612 | loop work (9612 items) |  |
| 2 | 98.9 | 62 | loop work (62 items) | softirq NET_RX 1 |
| 3 | 87.8 | 61 | loop work (61 items) | local timer tick 3, preempted by kworker/3:1 2, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 4 | 86.6 | 55 | loop work (55 items) |  |
| 5 | 86.2 | 237 | loop work (237 items) |  |
| 6 | 85.2 | 57 | loop work (57 items) | softirq NET_RX 2 |
| 7 | 84.4 | 59 | loop work (59 items) | local timer tick 3, preempted by kworker/3:1 2, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 8 | 84.3 | 28 | loop work (28 items) | local timer tick 4, softirq RCU 1, timer tick_nohz_handler 1 |
| 9 | 81.7 | 51 | loop work (51 items) |  |
| 10 | 81.6 | 31 | loop work (31 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6101 |
| LOC Local timer interrupts | 5907 |
| CAL Function call interrupts | 992 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c256-s1024: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2909884 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137881 | 3729000 | 1.65 | 96089254 | 15.66 | 1601488 | 171267 | 95.7 | 69.5 | true | 0 | 6735 | 1416.9 | 3676211 / 80225 / 9 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 0 |
| recv-busy | 1 | 8195 | 0 |
| recv-busy | 2 | 8197 | 0 |
| recv-busy | 3 | 8193 | 0 |
| recv-busy | 4 | 8199 | 0 |
| recv-busy | 5 | 8193 | 3 |
| recv-busy | 6 | 8195 | 3 |
| recv-busy | 7 | 8196 | 3 |
| recv-busy | 8 | 8200 | 3 |
| recv-busy | 9 | 8196 | 3 |
| recv-busy | 10 | 8199 | 3 |
| recv-busy | 11 | 8198 | 3 |
| recv-busy | 12 | 8197 | 3 |
| recv-busy | 13 | 8198 | 3 |
| recv-busy | 14 | 8194 | 3 |
| recv-busy | 15 | 8193 | 3 |
| recv-busy | 16 | 8199 | 3 |
| recv-busy | 17 | 8198 | 3 |
| recv-busy | 18 | 8198 | 3 |
| recv-busy | 19 | 8194 | 4 |
| recv-busy | 20 | 8200 | 3 |
| recv-busy | 21 | 8197 | 3 |
| recv-busy | 22 | 8200 | 3 |
| recv-busy | 23 | 8199 | 3 |
| recv-busy | 24 | 8195 | 3 |
| recv-busy | 25 | 8197 | 3 |
| recv-busy | 26 | 8194 | 3 |
| recv-busy | 27 | 8198 | 3 |
| recv-busy | 28 | 8193 | 3 |
| recv-busy | 29 | 8193 | 3 |
| recv-busy | 30 | 8194 | 3 |
| recv-busy | 31 | 8197 | 3 |
| recv-busy | 32 | 8194 | 3 |
| recv-busy | 33 | 8196 | 4 |
| recv-busy | 34 | 8198 | 3 |
| recv-busy | 35 | 8196 | 3 |
| recv-busy | 36 | 8196 | 3 |
| recv-busy | 37 | 8198 | 3 |
| recv-busy | 38 | 8198 | 3 |
| recv-busy | 39 | 8196 | 3 |
| recv-busy | 40 | 8197 | 3 |
| recv-busy | 41 | 8198 | 4 |
| recv-busy | 42 | 8200 | 3 |
| recv-busy | 43 | 8193 | 0 |
| recv-busy | 44 | 8195 | 0 |
| recv-busy | 45 | 8198 | 3 |
| recv-busy | 46 | 8199 | 3 |
| recv-busy | 47 | 8193 | 3 |
| recv-busy | 48 | 8197 | 0 |
| recv-busy | 49 | 8199 | 3 |
| recv-busy | 50 | 8195 | 3 |
| recv-busy | 51 | 8193 | 3 |
| recv-busy | 52 | 8200 | 3 |
| recv-busy | 53 | 8195 | 3 |
| recv-busy | 54 | 8195 | 3 |
| recv-busy | 55 | 8194 | 4 |
| recv-busy | 56 | 8193 | 0 |
| recv-busy | 57 | 8197 | 3 |
| recv-busy | 58 | 8200 | 3 |
| recv-busy | 59 | 8200 | 3 |
| recv-busy | 60 | 8200 | 3 |
| recv-busy | 61 | 8193 | 3 |
| recv-busy | 62 | 8199 | 3 |
| recv-busy | 63 | 8193 | 3 |
| recv-busy | 64 | 8194 | 3 |
| recv-busy | 65 | 8198 | 3 |
| recv-busy | 66 | 8196 | 3 |
| recv-busy | 67 | 8196 | 3 |
| recv-busy | 68 | 8194 | 3 |
| recv-busy | 69 | 8193 | 3 |
| recv-busy | 70 | 8198 | 3 |
| recv-busy | 71 | 8197 | 3 |
| recv-busy | 72 | 8198 | 4 |
| recv-busy | 73 | 8196 | 4 |
| recv-busy | 74 | 8194 | 3 |
| recv-busy | 75 | 8194 | 4 |
| recv-busy | 76 | 8199 | 0 |
| recv-busy | 77 | 8198 | 3 |
| recv-busy | 78 | 8194 | 4 |
| recv-busy | 79 | 8200 | 3 |
| recv-busy | 80 | 8194 | 3 |
| recv-busy | 81 | 8197 | 3 |
| recv-busy | 82 | 8193 | 3 |
| recv-busy | 83 | 8200 | 3 |
| recv-busy | 84 | 8194 | 3 |
| recv-busy | 85 | 8198 | 3 |
| recv-busy | 86 | 8195 | 0 |
| recv-busy | 87 | 8200 | 3 |
| recv-busy | 88 | 8194 | 4 |
| recv-busy | 89 | 8200 | 3 |
| recv-busy | 90 | 8200 | 3 |
| recv-busy | 91 | 8199 | 3 |
| recv-busy | 92 | 8197 | 3 |
| recv-busy | 93 | 8197 | 3 |
| recv-busy | 94 | 8195 | 3 |
| recv-busy | 95 | 8198 | 3 |
| recv-busy | 96 | 8200 | 3 |
| recv-busy | 97 | 8196 | 3 |
| recv-busy | 98 | 8198 | 3 |
| recv-busy | 99 | 8200 | 3 |
| recv-busy | 100 | 8194 | 3 |
| recv-busy | 101 | 8196 | 3 |
| recv-busy | 102 | 8200 | 3 |
| recv-busy | 103 | 8199 | 3 |
| recv-busy | 104 | 8197 | 3 |
| recv-busy | 105 | 8197 | 3 |
| recv-busy | 106 | 8198 | 3 |
| recv-busy | 107 | 8200 | 3 |
| recv-busy | 108 | 8199 | 3 |
| recv-busy | 109 | 8200 | 4 |
| recv-busy | 110 | 8195 | 0 |
| recv-busy | 111 | 8196 | 4 |
| recv-busy | 112 | 8200 | 4 |
| recv-busy | 113 | 8197 | 3 |
| recv-busy | 114 | 8194 | 3 |
| recv-busy | 115 | 8193 | 3 |
| recv-busy | 116 | 8193 | 3 |
| recv-busy | 117 | 8198 | 4 |
| recv-busy | 118 | 8195 | 0 |
| recv-busy | 119 | 8197 | 0 |
| recv-busy | 120 | 8200 | 3 |
| recv-busy | 121 | 8200 | 3 |
| recv-busy | 122 | 8200 | 3 |
| recv-busy | 123 | 8197 | 3 |
| recv-busy | 124 | 8194 | 3 |
| recv-busy | 125 | 8193 | 0 |
| recv-busy | 126 | 8196 | 3 |
| recv-busy | 127 | 8198 | 4 |
| recv-busy | 128 | 8199 | 3 |
| recv-busy | 129 | 8200 | 3 |
| recv-busy | 130 | 8198 | 4 |
| recv-busy | 131 | 8196 | 3 |
| recv-busy | 132 | 8194 | 4 |
| recv-busy | 133 | 8199 | 0 |
| recv-busy | 134 | 8197 | 3 |
| recv-busy | 135 | 8194 | 3 |
| recv-busy | 136 | 8200 | 3 |
| recv-busy | 137 | 8197 | 3 |
| recv-busy | 138 | 8200 | 4 |
| recv-busy | 139 | 8198 | 3 |
| recv-busy | 140 | 8196 | 3 |
| recv-busy | 141 | 8197 | 3 |
| recv-busy | 142 | 8198 | 3 |
| recv-busy | 143 | 8193 | 0 |
| recv-busy | 144 | 8194 | 3 |
| recv-busy | 145 | 8193 | 3 |
| recv-busy | 146 | 8196 | 3 |
| recv-busy | 147 | 8194 | 3 |
| recv-busy | 148 | 8197 | 3 |
| recv-busy | 149 | 8195 | 3 |
| recv-busy | 150 | 8196 | 4 |
| recv-busy | 151 | 8198 | 4 |
| recv-busy | 152 | 8194 | 4 |
| recv-busy | 153 | 8198 | 3 |
| recv-busy | 154 | 8196 | 3 |
| recv-busy | 155 | 8197 | 3 |
| recv-busy | 156 | 8197 | 0 |
| recv-busy | 157 | 8195 | 3 |
| recv-busy | 158 | 8195 | 3 |
| recv-busy | 159 | 8196 | 3 |
| recv-busy | 160 | 8194 | 3 |
| recv-busy | 161 | 8195 | 3 |
| recv-busy | 162 | 8199 | 0 |
| recv-busy | 163 | 8193 | 3 |
| recv-busy | 164 | 8195 | 3 |
| recv-busy | 165 | 8199 | 0 |
| recv-busy | 166 | 8199 | 3 |
| recv-busy | 167 | 8195 | 0 |
| recv-busy | 168 | 8194 | 3 |
| recv-busy | 169 | 8193 | 3 |
| recv-busy | 170 | 8200 | 3 |
| recv-busy | 171 | 8200 | 3 |
| recv-busy | 172 | 8198 | 3 |
| recv-busy | 173 | 8199 | 3 |
| recv-busy | 174 | 8197 | 0 |
| recv-busy | 175 | 8199 | 3 |
| recv-busy | 176 | 8198 | 3 |
| recv-busy | 177 | 8194 | 3 |
| recv-busy | 178 | 8196 | 4 |
| recv-busy | 179 | 8196 | 3 |
| recv-busy | 180 | 8194 | 3 |
| recv-busy | 181 | 8193 | 3 |
| recv-busy | 182 | 8196 | 4 |
| recv-busy | 183 | 8199 | 3 |
| recv-busy | 184 | 8199 | 0 |
| recv-busy | 185 | 8199 | 3 |
| recv-busy | 186 | 8199 | 3 |
| recv-busy | 187 | 8196 | 3 |
| recv-busy | 188 | 8199 | 3 |
| recv-busy | 189 | 8193 | 3 |
| recv-busy | 190 | 8197 | 3 |
| recv-busy | 191 | 8200 | 3 |
| recv-busy | 192 | 8194 | 3 |
| recv-busy | 193 | 8198 | 3 |
| recv-busy | 194 | 8197 | 3 |
| recv-busy | 195 | 8196 | 3 |
| recv-busy | 196 | 8193 | 3 |
| recv-busy | 197 | 8197 | 0 |
| recv-busy | 198 | 8194 | 3 |
| recv-busy | 199 | 8196 | 4 |
| recv-busy | 200 | 8198 | 3 |
| recv-busy | 201 | 8197 | 3 |
| recv-busy | 202 | 8195 | 3 |
| recv-busy | 203 | 8197 | 3 |
| recv-busy | 204 | 8196 | 3 |
| recv-busy | 205 | 8194 | 3 |
| recv-busy | 206 | 8194 | 3 |
| recv-busy | 207 | 8198 | 3 |
| recv-busy | 208 | 8195 | 3 |
| recv-busy | 209 | 8198 | 3 |
| recv-busy | 210 | 8194 | 3 |
| recv-busy | 211 | 8198 | 3 |
| recv-busy | 212 | 8198 | 3 |
| recv-busy | 213 | 8195 | 3 |
| recv-busy | 214 | 8194 | 3 |
| recv-busy | 215 | 8200 | 3 |
| recv-busy | 216 | 8196 | 4 |
| recv-busy | 217 | 8193 | 3 |
| recv-busy | 218 | 8197 | 3 |
| recv-busy | 219 | 8197 | 3 |
| recv-busy | 220 | 8193 | 3 |
| recv-busy | 221 | 8194 | 3 |
| recv-busy | 222 | 8198 | 4 |
| recv-busy | 223 | 8199 | 3 |
| recv-busy | 224 | 8194 | 3 |
| recv-busy | 225 | 8196 | 4 |
| recv-busy | 226 | 8195 | 3 |
| recv-busy | 227 | 8198 | 3 |
| recv-busy | 228 | 8193 | 3 |
| recv-busy | 229 | 8197 | 3 |
| recv-busy | 230 | 8193 | 3 |
| recv-busy | 231 | 8194 | 3 |
| recv-busy | 232 | 8196 | 4 |
| recv-busy | 233 | 8197 | 3 |
| recv-busy | 234 | 8197 | 3 |
| recv-busy | 235 | 8198 | 4 |
| recv-busy | 236 | 8198 | 3 |
| recv-busy | 237 | 8200 | 3 |
| recv-busy | 238 | 8197 | 3 |
| recv-busy | 239 | 8196 | 4 |
| recv-busy | 240 | 8200 | 3 |
| recv-busy | 241 | 8194 | 3 |
| recv-busy | 242 | 8199 | 3 |
| recv-busy | 243 | 8198 | 3 |
| recv-busy | 244 | 8194 | 3 |
| recv-busy | 245 | 8198 | 3 |
| recv-busy | 246 | 8197 | 0 |
| recv-busy | 247 | 8193 | 0 |
| recv-busy | 248 | 8200 | 3 |
| recv-busy | 249 | 8200 | 3 |
| recv-busy | 250 | 8197 | 3 |
| recv-busy | 251 | 8199 | 0 |
| recv-busy | 252 | 8196 | 3 |
| recv-busy | 253 | 8197 | 3 |
| recv-busy | 254 | 8199 | 3 |
| recv-busy | 255 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374885344 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (3064734 of 5923972 in 1-msg reads) | 3064734 | 6.58 | 5.67 | 10.26 | 23.16 | 36.84 | 50.59 | 82.19 |

#### Stalls: m1/lab-recv-busy-pq-c256-s1024, loop 0 on CPU 3

10276032 iterations, longest 1416.9 µs; over 5 / 20 / 100 µs: 3676211 / 80225 / 9

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 72997 | 2044929.8 | 1416.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 5544 | 154261.7 | 62.1 |
| mostly unexplained (+preempted by kworker/3:1) | 1318 | 30434.0 | 50.4 |
| mostly unexplained (+irq work (IPI/vector)) | 165 | 3739.3 | 50.8 |
| mostly unexplained (+softirq NET_RX) | 124 | 3111.1 | 42.6 |
| mostly unexplained (+local timer tick) | 64 | 1938.8 | 60.1 |
| softirq NET_RX | 9 | 283.0 | 44.5 |
| mostly unexplained (+ipi_send_cpu) | 2 | 61.7 | 34.2 |
| local timer tick | 1 | 21.1 | 21.1 |
| mostly unexplained (+softirq RCU) | 1 | 20.4 | 20.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1416.9 | 3032 | loop work (3032 items) |  |
| 2 | 1029.6 | 2541 | loop work (2541 items) |  |
| 3 | 936.1 | 2339 | loop work (2339 items) |  |
| 4 | 723.8 | 1901 | loop work (1901 items) |  |
| 5 | 169.1 | 193 | loop work (193 items) |  |
| 6 | 116.4 | 56 | loop work (56 items) | local timer tick 4, preempted by kworker/3:1 2, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 7 | 103.6 | 75 | loop work (75 items) | softirq NET_RX 1 |
| 8 | 102.5 | 74 | loop work (74 items) | softirq NET_RX 1 |
| 9 | 100.3 | 55 | loop work (55 items) |  |
| 10 | 97.9 | 50 | loop work (50 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6785 |
| LOC Local timer interrupts | 6482 |
| CAL Function call interrupts | 991 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c4-s1024: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3116177 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137719 | 3399158 | 1.81 | 115106375 | 18.75 | 1918440 | 476045 | 96.1 | 69.4 | true | 0 | 6192 | 2862.3 | 731496 / 25555 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 3 |
| recv-busy | 1 | 8199 | 3 |
| recv-busy | 2 | 8195 | 3 |
| recv-busy | 3 | 8197 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374849056 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (2653961 of 5923967 in 1-msg reads) | 2653961 | 3.13 | 2.50 | 4.42 | 9.18 | 18.03 | 27.71 | 51.47 |

#### Stalls: m1/lab-recv-busy-pq-c4-s1024, loop 0 on CPU 3

28562688 iterations, longest 2862.3 µs; over 5 / 20 / 100 µs: 731496 / 25555 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 17919 | 490843.0 | 2862.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 7008 | 159835.1 | 68.2 |
| mostly unexplained (+preempted by kworker/3:1) | 268 | 6385.8 | 50.3 |
| mostly unexplained (+softirq NET_RX) | 258 | 5953.6 | 40.6 |
| mostly unexplained (+local timer tick) | 46 | 1232.7 | 44.7 |
| mostly unexplained (+irq work (IPI/vector)) | 27 | 813.3 | 42.5 |
| softirq NET_RX | 27 | 720.8 | 39.4 |
| softirq TIMER | 1 | 55.4 | 55.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.7 | 26.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2862.3 | 9584 | loop work (9584 items) |  |
| 2 | 73.7 | 19 | loop work (19 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 72.0 | 44 | loop work (44 items) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 68.2 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 67.1 | 37 | loop work (37 items) |  |
| 6 | 65.9 | 42 | loop work (42 items) |  |
| 7 | 64.8 | 33 | loop work (33 items) | softirq NET_RX 4 |
| 8 | 60.3 | 33 | loop work (33 items) |  |
| 9 | 59.9 | 37 | loop work (37 items) | softirq NET_TX 1 |
| 10 | 59.5 | 40 | loop work (40 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6207 |
| LOC Local timer interrupts | 6036 |
| CAL Function call interrupts | 992 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-recv-busy-pq-c64-s1024: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2966741 clocksource=tsc

### Loop counters (m1/lab-recv-busy-pq-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137868 | 3721778 | 1.65 | 101831634 | 16.59 | 1697194 | 182519 | 95.9 | 71.9 | true | 0 | 6736 | 3657.0 | 3177510 / 73142 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 3 |
| recv-busy | 1 | 8200 | 3 |
| recv-busy | 2 | 8198 | 3 |
| recv-busy | 3 | 8200 | 3 |
| recv-busy | 4 | 8193 | 0 |
| recv-busy | 5 | 8194 | 4 |
| recv-busy | 6 | 8195 | 3 |
| recv-busy | 7 | 8200 | 3 |
| recv-busy | 8 | 8193 | 3 |
| recv-busy | 9 | 8199 | 3 |
| recv-busy | 10 | 8199 | 3 |
| recv-busy | 11 | 8193 | 0 |
| recv-busy | 12 | 8195 | 3 |
| recv-busy | 13 | 8196 | 3 |
| recv-busy | 14 | 8199 | 0 |
| recv-busy | 15 | 8200 | 3 |
| recv-busy | 16 | 8197 | 3 |
| recv-busy | 17 | 8193 | 3 |
| recv-busy | 18 | 8199 | 3 |
| recv-busy | 19 | 8200 | 3 |
| recv-busy | 20 | 8194 | 3 |
| recv-busy | 21 | 8197 | 0 |
| recv-busy | 22 | 8199 | 3 |
| recv-busy | 23 | 8196 | 4 |
| recv-busy | 24 | 8194 | 4 |
| recv-busy | 25 | 8200 | 3 |
| recv-busy | 26 | 8193 | 0 |
| recv-busy | 27 | 8199 | 3 |
| recv-busy | 28 | 8197 | 0 |
| recv-busy | 29 | 8194 | 3 |
| recv-busy | 30 | 8193 | 3 |
| recv-busy | 31 | 8197 | 0 |
| recv-busy | 32 | 8194 | 3 |
| recv-busy | 33 | 8199 | 0 |
| recv-busy | 34 | 8197 | 0 |
| recv-busy | 35 | 8196 | 3 |
| recv-busy | 36 | 8196 | 4 |
| recv-busy | 37 | 8200 | 3 |
| recv-busy | 38 | 8197 | 3 |
| recv-busy | 39 | 8195 | 3 |
| recv-busy | 40 | 8198 | 4 |
| recv-busy | 41 | 8200 | 4 |
| recv-busy | 42 | 8198 | 4 |
| recv-busy | 43 | 8199 | 3 |
| recv-busy | 44 | 8199 | 0 |
| recv-busy | 45 | 8199 | 3 |
| recv-busy | 46 | 8193 | 3 |
| recv-busy | 47 | 8197 | 0 |
| recv-busy | 48 | 8193 | 3 |
| recv-busy | 49 | 8198 | 3 |
| recv-busy | 50 | 8197 | 0 |
| recv-busy | 51 | 8200 | 3 |
| recv-busy | 52 | 8194 | 3 |
| recv-busy | 53 | 8193 | 3 |
| recv-busy | 54 | 8194 | 3 |
| recv-busy | 55 | 8197 | 3 |
| recv-busy | 56 | 8197 | 0 |
| recv-busy | 57 | 8200 | 3 |
| recv-busy | 58 | 8196 | 3 |
| recv-busy | 59 | 8200 | 3 |
| recv-busy | 60 | 8194 | 3 |
| recv-busy | 61 | 8197 | 3 |
| recv-busy | 62 | 8198 | 3 |
| recv-busy | 63 | 8194 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374882432 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (3059885 of 5924077 in 1-msg reads) | 3059885 | 6.39 | 5.82 | 9.76 | 21.80 | 35.63 | 49.01 | 74.80 |

#### Stalls: m1/lab-recv-busy-pq-c64-s1024, loop 0 on CPU 3

10951168 iterations, longest 3657.0 µs; over 5 / 20 / 100 µs: 3177510 / 73142 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 65944 | 1862304.9 | 3657.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 5656 | 159425.1 | 64.4 |
| mostly unexplained (+preempted by kworker/3:1) | 1173 | 28323.3 | 67.0 |
| mostly unexplained (+softirq NET_RX) | 145 | 3862.8 | 61.2 |
| mostly unexplained (+irq work (IPI/vector)) | 133 | 3279.7 | 48.2 |
| mostly unexplained (+local timer tick) | 84 | 2607.7 | 61.5 |
| softirq NET_RX | 6 | 185.4 | 63.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 40.2 | 40.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3657.0 | 9743 | loop work (9743 items) |  |
| 2 | 125.0 | 173 | loop work (173 items) |  |
| 3 | 111.2 | 65 | loop work (65 items) |  |
| 4 | 100.0 | 69 | loop work (69 items) |  |
| 5 | 98.7 | 8 | loop work (8 items) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 96.1 | 56 | loop work (56 items) |  |
| 7 | 96.0 | 59 | loop work (59 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, softirq NET_RX 1, workqueue [ena] 1 |
| 8 | 92.9 | 59 | loop work (59 items) |  |
| 9 | 92.6 | 60 | loop work (60 items) |  |
| 10 | 91.6 | 53 | loop work (53 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6782 |
| LOC Local timer interrupts | 6475 |
| CAL Function call interrupts | 991 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c1-s1024: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2470545 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137706 | 2913070 | 2.11 | 1 | 0.00 | 0 | 38423132 | 0.0 | 0.0 | true | 0 | 0 | 1321.9 | 7458 / 3 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8196 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1374846144 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1938385 of 5923904 in 1-msg reads) | 1938385 | 2.99 | 2.85 | 3.37 | 4.31 | 14.87 | 18.00 | 34.72 |

#### Stalls: m1/lab-uring-sqpoll-c1-s1024, loop 0 on CPU 3

2305387904 iterations, longest 1321.9 µs; over 5 / 20 / 100 µs: 7458 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1358.1 | 1321.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.5 | 28.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1321.9 | 9464 | loop work (9464 items) |  |
| 2 | 36.2 | 255 | loop work (255 items) |  |
| 3 | 28.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 2 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c16-s1024: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3405109 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137719 | 3742781 | 1.64 | 1 | 0.00 | 0 | 26951393 | 0.0 | 0.0 | true | 0 | 0 | 1354.4 | 4792 / 4 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8195 | 2 |
| uring-sqpoll | 1 | 8193 | 2 |
| uring-sqpoll | 2 | 8197 | 2 |
| uring-sqpoll | 3 | 8200 | 2 |
| uring-sqpoll | 4 | 8196 | 2 |
| uring-sqpoll | 5 | 8195 | 2 |
| uring-sqpoll | 6 | 8198 | 2 |
| uring-sqpoll | 7 | 8194 | 2 |
| uring-sqpoll | 8 | 8196 | 2 |
| uring-sqpoll | 9 | 8197 | 2 |
| uring-sqpoll | 10 | 8193 | 2 |
| uring-sqpoll | 11 | 8200 | 2 |
| uring-sqpoll | 12 | 8197 | 2 |
| uring-sqpoll | 13 | 8200 | 2 |
| uring-sqpoll | 14 | 8195 | 2 |
| uring-sqpoll | 15 | 8199 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1374849056 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (3055560 of 5923770 in 1-msg reads) | 3055560 | 4.16 | 3.58 | 5.93 | 12.63 | 25.45 | 36.65 | 72.31 |

#### Stalls: m1/lab-uring-sqpoll-c16-s1024, loop 0 on CPU 3

1617083584 iterations, longest 1354.4 µs; over 5 / 20 / 100 µs: 4792 / 4 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1382.4 | 1354.4 |
| softirq TIMER | 1 | 162.7 | 162.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.7 | 34.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1354.4 | 9584 | loop work (9584 items) |  |
| 2 | 162.7 | 0 | softirq TIMER | softirq TIMER 144, local timer tick 9, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 3 | 34.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 28.0 | 175 | loop work (175 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| LOC Local timer interrupts | 11 |
| IWI IRQ work interrupts | 6 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c256-s1024: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3402437 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137900 | 3818064 | 1.61 | 1 | 0.00 | 0 | 2926192 | 0.0 | 0.0 | true | 0 | 0 | 400.7 | 8523 / 7 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8195 | 2 |
| uring-sqpoll | 1 | 8194 | 2 |
| uring-sqpoll | 2 | 8196 | 2 |
| uring-sqpoll | 3 | 8200 | 2 |
| uring-sqpoll | 4 | 8195 | 2 |
| uring-sqpoll | 5 | 8195 | 2 |
| uring-sqpoll | 6 | 8197 | 2 |
| uring-sqpoll | 7 | 8199 | 2 |
| uring-sqpoll | 8 | 8200 | 2 |
| uring-sqpoll | 9 | 8194 | 2 |
| uring-sqpoll | 10 | 8195 | 2 |
| uring-sqpoll | 11 | 8193 | 2 |
| uring-sqpoll | 12 | 8198 | 4 |
| uring-sqpoll | 13 | 8193 | 2 |
| uring-sqpoll | 14 | 8196 | 2 |
| uring-sqpoll | 15 | 8193 | 2 |
| uring-sqpoll | 16 | 8194 | 2 |
| uring-sqpoll | 17 | 8199 | 2 |
| uring-sqpoll | 18 | 8195 | 2 |
| uring-sqpoll | 19 | 8195 | 2 |
| uring-sqpoll | 20 | 8193 | 2 |
| uring-sqpoll | 21 | 8200 | 2 |
| uring-sqpoll | 22 | 8197 | 2 |
| uring-sqpoll | 23 | 8199 | 2 |
| uring-sqpoll | 24 | 8200 | 2 |
| uring-sqpoll | 25 | 8195 | 2 |
| uring-sqpoll | 26 | 8194 | 2 |
| uring-sqpoll | 27 | 8198 | 2 |
| uring-sqpoll | 28 | 8200 | 2 |
| uring-sqpoll | 29 | 8197 | 2 |
| uring-sqpoll | 30 | 8197 | 2 |
| uring-sqpoll | 31 | 8196 | 2 |
| uring-sqpoll | 32 | 8193 | 2 |
| uring-sqpoll | 33 | 8196 | 2 |
| uring-sqpoll | 34 | 8200 | 2 |
| uring-sqpoll | 35 | 8194 | 2 |
| uring-sqpoll | 36 | 8195 | 2 |
| uring-sqpoll | 37 | 8197 | 2 |
| uring-sqpoll | 38 | 8195 | 2 |
| uring-sqpoll | 39 | 8199 | 2 |
| uring-sqpoll | 40 | 8199 | 2 |
| uring-sqpoll | 41 | 8199 | 2 |
| uring-sqpoll | 42 | 8195 | 2 |
| uring-sqpoll | 43 | 8198 | 2 |
| uring-sqpoll | 44 | 8194 | 2 |
| uring-sqpoll | 45 | 8199 | 2 |
| uring-sqpoll | 46 | 8198 | 2 |
| uring-sqpoll | 47 | 8198 | 2 |
| uring-sqpoll | 48 | 8199 | 2 |
| uring-sqpoll | 49 | 8198 | 2 |
| uring-sqpoll | 50 | 8194 | 4 |
| uring-sqpoll | 51 | 8200 | 2 |
| uring-sqpoll | 52 | 8193 | 2 |
| uring-sqpoll | 53 | 8200 | 2 |
| uring-sqpoll | 54 | 8198 | 2 |
| uring-sqpoll | 55 | 8195 | 2 |
| uring-sqpoll | 56 | 8195 | 0 |
| uring-sqpoll | 57 | 8195 | 2 |
| uring-sqpoll | 58 | 8200 | 2 |
| uring-sqpoll | 59 | 8197 | 0 |
| uring-sqpoll | 60 | 8198 | 4 |
| uring-sqpoll | 61 | 8199 | 2 |
| uring-sqpoll | 62 | 8195 | 2 |
| uring-sqpoll | 63 | 8194 | 2 |
| uring-sqpoll | 64 | 8193 | 2 |
| uring-sqpoll | 65 | 8200 | 2 |
| uring-sqpoll | 66 | 8193 | 2 |
| uring-sqpoll | 67 | 8198 | 2 |
| uring-sqpoll | 68 | 8194 | 2 |
| uring-sqpoll | 69 | 8198 | 2 |
| uring-sqpoll | 70 | 8196 | 2 |
| uring-sqpoll | 71 | 8199 | 2 |
| uring-sqpoll | 72 | 8200 | 2 |
| uring-sqpoll | 73 | 8198 | 2 |
| uring-sqpoll | 74 | 8194 | 2 |
| uring-sqpoll | 75 | 8199 | 2 |
| uring-sqpoll | 76 | 8196 | 2 |
| uring-sqpoll | 77 | 8196 | 2 |
| uring-sqpoll | 78 | 8200 | 2 |
| uring-sqpoll | 79 | 8194 | 2 |
| uring-sqpoll | 80 | 8200 | 2 |
| uring-sqpoll | 81 | 8197 | 2 |
| uring-sqpoll | 82 | 8193 | 2 |
| uring-sqpoll | 83 | 8196 | 2 |
| uring-sqpoll | 84 | 8198 | 2 |
| uring-sqpoll | 85 | 8196 | 2 |
| uring-sqpoll | 86 | 8193 | 0 |
| uring-sqpoll | 87 | 8197 | 2 |
| uring-sqpoll | 88 | 8197 | 2 |
| uring-sqpoll | 89 | 8200 | 2 |
| uring-sqpoll | 90 | 8196 | 2 |
| uring-sqpoll | 91 | 8200 | 2 |
| uring-sqpoll | 92 | 8194 | 2 |
| uring-sqpoll | 93 | 8197 | 2 |
| uring-sqpoll | 94 | 8200 | 4 |
| uring-sqpoll | 95 | 8198 | 4 |
| uring-sqpoll | 96 | 8196 | 4 |
| uring-sqpoll | 97 | 8200 | 2 |
| uring-sqpoll | 98 | 8198 | 2 |
| uring-sqpoll | 99 | 8194 | 2 |
| uring-sqpoll | 100 | 8196 | 4 |
| uring-sqpoll | 101 | 8193 | 2 |
| uring-sqpoll | 102 | 8199 | 2 |
| uring-sqpoll | 103 | 8195 | 2 |
| uring-sqpoll | 104 | 8193 | 2 |
| uring-sqpoll | 105 | 8193 | 2 |
| uring-sqpoll | 106 | 8193 | 2 |
| uring-sqpoll | 107 | 8196 | 2 |
| uring-sqpoll | 108 | 8198 | 2 |
| uring-sqpoll | 109 | 8195 | 2 |
| uring-sqpoll | 110 | 8198 | 2 |
| uring-sqpoll | 111 | 8199 | 2 |
| uring-sqpoll | 112 | 8197 | 2 |
| uring-sqpoll | 113 | 8195 | 2 |
| uring-sqpoll | 114 | 8195 | 0 |
| uring-sqpoll | 115 | 8198 | 2 |
| uring-sqpoll | 116 | 8196 | 4 |
| uring-sqpoll | 117 | 8198 | 2 |
| uring-sqpoll | 118 | 8199 | 2 |
| uring-sqpoll | 119 | 8196 | 2 |
| uring-sqpoll | 120 | 8198 | 4 |
| uring-sqpoll | 121 | 8196 | 2 |
| uring-sqpoll | 122 | 8199 | 2 |
| uring-sqpoll | 123 | 8194 | 2 |
| uring-sqpoll | 124 | 8199 | 0 |
| uring-sqpoll | 125 | 8194 | 2 |
| uring-sqpoll | 126 | 8200 | 2 |
| uring-sqpoll | 127 | 8199 | 2 |
| uring-sqpoll | 128 | 8195 | 2 |
| uring-sqpoll | 129 | 8194 | 2 |
| uring-sqpoll | 130 | 8199 | 2 |
| uring-sqpoll | 131 | 8196 | 2 |
| uring-sqpoll | 132 | 8195 | 2 |
| uring-sqpoll | 133 | 8198 | 2 |
| uring-sqpoll | 134 | 8193 | 2 |
| uring-sqpoll | 135 | 8199 | 2 |
| uring-sqpoll | 136 | 8194 | 2 |
| uring-sqpoll | 137 | 8198 | 2 |
| uring-sqpoll | 138 | 8195 | 2 |
| uring-sqpoll | 139 | 8199 | 2 |
| uring-sqpoll | 140 | 8200 | 2 |
| uring-sqpoll | 141 | 8193 | 2 |
| uring-sqpoll | 142 | 8193 | 2 |
| uring-sqpoll | 143 | 8200 | 2 |
| uring-sqpoll | 144 | 8193 | 0 |
| uring-sqpoll | 145 | 8198 | 2 |
| uring-sqpoll | 146 | 8194 | 2 |
| uring-sqpoll | 147 | 8199 | 2 |
| uring-sqpoll | 148 | 8198 | 2 |
| uring-sqpoll | 149 | 8199 | 2 |
| uring-sqpoll | 150 | 8194 | 2 |
| uring-sqpoll | 151 | 8199 | 0 |
| uring-sqpoll | 152 | 8196 | 2 |
| uring-sqpoll | 153 | 8193 | 2 |
| uring-sqpoll | 154 | 8196 | 2 |
| uring-sqpoll | 155 | 8193 | 2 |
| uring-sqpoll | 156 | 8199 | 2 |
| uring-sqpoll | 157 | 8197 | 2 |
| uring-sqpoll | 158 | 8197 | 2 |
| uring-sqpoll | 159 | 8195 | 0 |
| uring-sqpoll | 160 | 8193 | 2 |
| uring-sqpoll | 161 | 8200 | 2 |
| uring-sqpoll | 162 | 8196 | 2 |
| uring-sqpoll | 163 | 8196 | 2 |
| uring-sqpoll | 164 | 8194 | 2 |
| uring-sqpoll | 165 | 8196 | 2 |
| uring-sqpoll | 166 | 8196 | 2 |
| uring-sqpoll | 167 | 8199 | 2 |
| uring-sqpoll | 168 | 8197 | 2 |
| uring-sqpoll | 169 | 8194 | 2 |
| uring-sqpoll | 170 | 8195 | 2 |
| uring-sqpoll | 171 | 8198 | 2 |
| uring-sqpoll | 172 | 8197 | 2 |
| uring-sqpoll | 173 | 8195 | 2 |
| uring-sqpoll | 174 | 8199 | 2 |
| uring-sqpoll | 175 | 8199 | 2 |
| uring-sqpoll | 176 | 8197 | 2 |
| uring-sqpoll | 177 | 8199 | 2 |
| uring-sqpoll | 178 | 8200 | 2 |
| uring-sqpoll | 179 | 8199 | 2 |
| uring-sqpoll | 180 | 8199 | 2 |
| uring-sqpoll | 181 | 8200 | 2 |
| uring-sqpoll | 182 | 8198 | 2 |
| uring-sqpoll | 183 | 8198 | 2 |
| uring-sqpoll | 184 | 8196 | 2 |
| uring-sqpoll | 185 | 8198 | 2 |
| uring-sqpoll | 186 | 8199 | 2 |
| uring-sqpoll | 187 | 8196 | 2 |
| uring-sqpoll | 188 | 8200 | 2 |
| uring-sqpoll | 189 | 8193 | 2 |
| uring-sqpoll | 190 | 8197 | 2 |
| uring-sqpoll | 191 | 8196 | 2 |
| uring-sqpoll | 192 | 8198 | 2 |
| uring-sqpoll | 193 | 8196 | 2 |
| uring-sqpoll | 194 | 8198 | 2 |
| uring-sqpoll | 195 | 8197 | 2 |
| uring-sqpoll | 196 | 8197 | 2 |
| uring-sqpoll | 197 | 8195 | 2 |
| uring-sqpoll | 198 | 8197 | 2 |
| uring-sqpoll | 199 | 8197 | 2 |
| uring-sqpoll | 200 | 8198 | 2 |
| uring-sqpoll | 201 | 8193 | 2 |
| uring-sqpoll | 202 | 8195 | 2 |
| uring-sqpoll | 203 | 8195 | 2 |
| uring-sqpoll | 204 | 8195 | 2 |
| uring-sqpoll | 205 | 8199 | 2 |
| uring-sqpoll | 206 | 8195 | 2 |
| uring-sqpoll | 207 | 8197 | 2 |
| uring-sqpoll | 208 | 8193 | 2 |
| uring-sqpoll | 209 | 8195 | 0 |
| uring-sqpoll | 210 | 8198 | 4 |
| uring-sqpoll | 211 | 8197 | 2 |
| uring-sqpoll | 212 | 8194 | 2 |
| uring-sqpoll | 213 | 8196 | 2 |
| uring-sqpoll | 214 | 8194 | 2 |
| uring-sqpoll | 215 | 8196 | 2 |
| uring-sqpoll | 216 | 8198 | 2 |
| uring-sqpoll | 217 | 8198 | 2 |
| uring-sqpoll | 218 | 8200 | 2 |
| uring-sqpoll | 219 | 8194 | 4 |
| uring-sqpoll | 220 | 8195 | 2 |
| uring-sqpoll | 221 | 8198 | 2 |
| uring-sqpoll | 222 | 8198 | 2 |
| uring-sqpoll | 223 | 8198 | 2 |
| uring-sqpoll | 224 | 8194 | 2 |
| uring-sqpoll | 225 | 8198 | 2 |
| uring-sqpoll | 226 | 8196 | 2 |
| uring-sqpoll | 227 | 8194 | 2 |
| uring-sqpoll | 228 | 8197 | 0 |
| uring-sqpoll | 229 | 8199 | 2 |
| uring-sqpoll | 230 | 8193 | 2 |
| uring-sqpoll | 231 | 8196 | 2 |
| uring-sqpoll | 232 | 8195 | 2 |
| uring-sqpoll | 233 | 8198 | 2 |
| uring-sqpoll | 234 | 8198 | 2 |
| uring-sqpoll | 235 | 8200 | 2 |
| uring-sqpoll | 236 | 8196 | 2 |
| uring-sqpoll | 237 | 8194 | 2 |
| uring-sqpoll | 238 | 8196 | 2 |
| uring-sqpoll | 239 | 8195 | 2 |
| uring-sqpoll | 240 | 8193 | 2 |
| uring-sqpoll | 241 | 8194 | 2 |
| uring-sqpoll | 242 | 8197 | 2 |
| uring-sqpoll | 243 | 8197 | 2 |
| uring-sqpoll | 244 | 8195 | 2 |
| uring-sqpoll | 245 | 8195 | 2 |
| uring-sqpoll | 246 | 8195 | 2 |
| uring-sqpoll | 247 | 8197 | 2 |
| uring-sqpoll | 248 | 8193 | 2 |
| uring-sqpoll | 249 | 8196 | 2 |
| uring-sqpoll | 250 | 8200 | 2 |
| uring-sqpoll | 251 | 8199 | 2 |
| uring-sqpoll | 252 | 8198 | 2 |
| uring-sqpoll | 253 | 8199 | 2 |
| uring-sqpoll | 254 | 8195 | 2 |
| uring-sqpoll | 255 | 8200 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1374889600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (3123254 of 5923734 in 1-msg reads) | 3123254 | 4.66 | 3.81 | 6.91 | 17.52 | 31.62 | 45.53 | 76.00 |

#### Stalls: m1/lab-uring-sqpoll-c256-s1024, loop 0 on CPU 3

175571520 iterations, longest 400.7 µs; over 5 / 20 / 100 µs: 8523 / 7 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 6 | 1543.2 | 400.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.0 | 23.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 400.7 | 2719 | loop work (2719 items) |  |
| 2 | 369.1 | 2463 | loop work (2463 items) |  |
| 3 | 356.8 | 2338 | loop work (2338 items) |  |
| 4 | 341.9 | 2266 | loop work (2266 items) |  |
| 5 | 41.3 | 114 | loop work (114 items) |  |
| 6 | 33.4 | 125 | loop work (125 items) |  |
| 7 | 23.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| IWI IRQ work interrupts | 10 |
| LOC Local timer interrupts | 7 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c4-s1024: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2911442 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137698 | 3427923 | 1.79 | 1 | 0.00 | 0 | 33719276 | 0.0 | 0.0 | true | 0 | 0 | 1325.1 | 2713 / 4 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8198 | 2 |
| uring-sqpoll | 1 | 8198 | 2 |
| uring-sqpoll | 2 | 8198 | 2 |
| uring-sqpoll | 3 | 8196 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1374844352 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (2677215 of 5923779 in 1-msg reads) | 2677215 | 3.45 | 3.11 | 4.42 | 8.40 | 14.98 | 25.00 | 38.82 |

#### Stalls: m1/lab-uring-sqpoll-c4-s1024, loop 0 on CPU 3

2023156544 iterations, longest 1325.1 µs; over 5 / 20 / 100 µs: 2713 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1350.4 | 1325.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 25.6 | 25.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.7 | 23.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1325.1 | 9489 | loop work (9489 items) |  |
| 2 | 25.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 25.2 | 173 | loop work (173 items) |  |
| 4 | 23.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m1/lab-uring-sqpoll-c64-s1024: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3429280 clocksource=tsc

### Loop counters (m1/lab-uring-sqpoll-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137752 | 3814829 | 1.61 | 1 | 0.00 | 0 | 11677356 | 0.0 | 0.0 | true | 0 | 0 | 1375.3 | 4825 / 3 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8200 | 2 |
| uring-sqpoll | 1 | 8194 | 2 |
| uring-sqpoll | 2 | 8197 | 2 |
| uring-sqpoll | 3 | 8196 | 2 |
| uring-sqpoll | 4 | 8197 | 2 |
| uring-sqpoll | 5 | 8196 | 2 |
| uring-sqpoll | 6 | 8200 | 2 |
| uring-sqpoll | 7 | 8200 | 2 |
| uring-sqpoll | 8 | 8198 | 2 |
| uring-sqpoll | 9 | 8193 | 2 |
| uring-sqpoll | 10 | 8195 | 2 |
| uring-sqpoll | 11 | 8199 | 2 |
| uring-sqpoll | 12 | 8194 | 2 |
| uring-sqpoll | 13 | 8196 | 2 |
| uring-sqpoll | 14 | 8197 | 2 |
| uring-sqpoll | 15 | 8193 | 2 |
| uring-sqpoll | 16 | 8194 | 2 |
| uring-sqpoll | 17 | 8199 | 2 |
| uring-sqpoll | 18 | 8194 | 2 |
| uring-sqpoll | 19 | 8198 | 2 |
| uring-sqpoll | 20 | 8196 | 2 |
| uring-sqpoll | 21 | 8194 | 2 |
| uring-sqpoll | 22 | 8198 | 2 |
| uring-sqpoll | 23 | 8194 | 2 |
| uring-sqpoll | 24 | 8198 | 2 |
| uring-sqpoll | 25 | 8193 | 2 |
| uring-sqpoll | 26 | 8198 | 4 |
| uring-sqpoll | 27 | 8195 | 2 |
| uring-sqpoll | 28 | 8197 | 2 |
| uring-sqpoll | 29 | 8199 | 2 |
| uring-sqpoll | 30 | 8197 | 2 |
| uring-sqpoll | 31 | 8196 | 2 |
| uring-sqpoll | 32 | 8200 | 2 |
| uring-sqpoll | 33 | 8195 | 2 |
| uring-sqpoll | 34 | 8194 | 2 |
| uring-sqpoll | 35 | 8199 | 2 |
| uring-sqpoll | 36 | 8193 | 2 |
| uring-sqpoll | 37 | 8198 | 2 |
| uring-sqpoll | 38 | 8196 | 2 |
| uring-sqpoll | 39 | 8193 | 2 |
| uring-sqpoll | 40 | 8196 | 2 |
| uring-sqpoll | 41 | 8196 | 2 |
| uring-sqpoll | 42 | 8199 | 2 |
| uring-sqpoll | 43 | 8200 | 2 |
| uring-sqpoll | 44 | 8199 | 2 |
| uring-sqpoll | 45 | 8200 | 2 |
| uring-sqpoll | 46 | 8200 | 4 |
| uring-sqpoll | 47 | 8196 | 2 |
| uring-sqpoll | 48 | 8199 | 2 |
| uring-sqpoll | 49 | 8198 | 2 |
| uring-sqpoll | 50 | 8196 | 2 |
| uring-sqpoll | 51 | 8196 | 2 |
| uring-sqpoll | 52 | 8197 | 2 |
| uring-sqpoll | 53 | 8200 | 2 |
| uring-sqpoll | 54 | 8197 | 2 |
| uring-sqpoll | 55 | 8200 | 2 |
| uring-sqpoll | 56 | 8198 | 2 |
| uring-sqpoll | 57 | 8195 | 2 |
| uring-sqpoll | 58 | 8197 | 2 |
| uring-sqpoll | 59 | 8196 | 2 |
| uring-sqpoll | 60 | 8196 | 2 |
| uring-sqpoll | 61 | 8200 | 2 |
| uring-sqpoll | 62 | 8193 | 2 |
| uring-sqpoll | 63 | 8193 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1374856448 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (3120215 of 5923748 in 1-msg reads) | 3120215 | 4.32 | 3.59 | 6.34 | 15.47 | 30.17 | 43.56 | 168.28 |

#### Stalls: m1/lab-uring-sqpoll-c64-s1024, loop 0 on CPU 3

700641344 iterations, longest 1375.3 µs; over 5 / 20 / 100 µs: 4825 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1412.0 | 1375.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 27.6 | 27.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1375.3 | 9694 | loop work (9694 items) |  |
| 2 | 36.7 | 172 | loop work (172 items) |  |
| 3 | 27.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 3 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c1-s1024: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 6137744 | 2191157 | 2.80 | 244483957 | 39.83 | 4074733 | 4038213 | 0.0 | 40.0 | false | 0 | 0 | 2698.9 | 215781 / 51 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1374854656 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (1304510 of 5923799 in 1-msg reads) | 1304510 | 4.58 | 4.58 | 6.34 | 10.09 | 16.90 | 20.65 | 47.65 |

#### Stalls: m2/lab-epoll-spin-c1-s1024, loop 0 on CPU 3

242292800 iterations, longest 2698.9 µs; over 5 / 20 / 100 µs: 215781 / 51 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 18 | 3198.8 | 2698.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 32 | 758.0 | 45.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.7 | 20.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2698.9 | 9497 | loop work (9497 items) |  |
| 2 | 82.4 | 355 | loop work (355 items) |  |
| 3 | 67.5 | 6 | loop work (6 items) |  |
| 4 | 45.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 32.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 32.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 30.2 | 3 | loop work (3 items) |  |
| 8 | 29.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 29.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 29.0 | 3 | loop work (3 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c16-s1024: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 6137900 | 3746070 | 1.64 | 230932118 | 37.62 | 3848869 | 3786434 | 0.0 | 42.7 | false | 0 | 0 | 3309.2 | 302678 / 1489 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8196 | 4 |
| epoll-spin | 1 | 8196 | 4 |
| epoll-spin | 2 | 8196 | 4 |
| epoll-spin | 3 | 8200 | 4 |
| epoll-spin | 4 | 8197 | 0 |
| epoll-spin | 5 | 8193 | 0 |
| epoll-spin | 6 | 8200 | 4 |
| epoll-spin | 7 | 8198 | 4 |
| epoll-spin | 8 | 8200 | 4 |
| epoll-spin | 9 | 8193 | 0 |
| epoll-spin | 10 | 8199 | 0 |
| epoll-spin | 11 | 8198 | 4 |
| epoll-spin | 12 | 8198 | 4 |
| epoll-spin | 13 | 8199 | 0 |
| epoll-spin | 14 | 8196 | 4 |
| epoll-spin | 15 | 8198 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1374889600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (3100793 of 5923963 in 1-msg reads) | 3100793 | 4.34 | 3.33 | 7.48 | 13.41 | 22.41 | 35.19 | 63.40 |

#### Stalls: m2/lab-epoll-spin-c16-s1024, loop 0 on CPU 3

227186048 iterations, longest 3309.2 µs; over 5 / 20 / 100 µs: 302678 / 1489 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1292 | 34543.7 | 54.0 |
| loop work | 196 | 8228.9 | 3309.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.3 | 23.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3309.2 | 9799 | loop work (9799 items) |  |
| 2 | 58.1 | 133 | loop work (133 items) |  |
| 3 | 54.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 48.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 48.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 47.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 46.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 46.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 45.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 43.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c256-s1024: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 6137953 | 3855002 | 1.59 | 128519834 | 20.94 | 2141997 | 2077747 | 0.0 | 30.8 | false | 0 | 0 | 1112.8 | 359549 / 1627 / 5 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8198 | 4 |
| epoll-spin | 1 | 8200 | 4 |
| epoll-spin | 2 | 8200 | 4 |
| epoll-spin | 3 | 8194 | 4 |
| epoll-spin | 4 | 8194 | 4 |
| epoll-spin | 5 | 8195 | 0 |
| epoll-spin | 6 | 8194 | 4 |
| epoll-spin | 7 | 8200 | 4 |
| epoll-spin | 8 | 8199 | 0 |
| epoll-spin | 9 | 8194 | 4 |
| epoll-spin | 10 | 8194 | 4 |
| epoll-spin | 11 | 8193 | 0 |
| epoll-spin | 12 | 8195 | 0 |
| epoll-spin | 13 | 8199 | 0 |
| epoll-spin | 14 | 8193 | 0 |
| epoll-spin | 15 | 8199 | 0 |
| epoll-spin | 16 | 8198 | 4 |
| epoll-spin | 17 | 8194 | 4 |
| epoll-spin | 18 | 8197 | 0 |
| epoll-spin | 19 | 8194 | 4 |
| epoll-spin | 20 | 8197 | 0 |
| epoll-spin | 21 | 8193 | 0 |
| epoll-spin | 22 | 8198 | 4 |
| epoll-spin | 23 | 8199 | 0 |
| epoll-spin | 24 | 8194 | 4 |
| epoll-spin | 25 | 8194 | 4 |
| epoll-spin | 26 | 8198 | 4 |
| epoll-spin | 27 | 8194 | 4 |
| epoll-spin | 28 | 8198 | 4 |
| epoll-spin | 29 | 8199 | 0 |
| epoll-spin | 30 | 8198 | 4 |
| epoll-spin | 31 | 8198 | 4 |
| epoll-spin | 32 | 8196 | 4 |
| epoll-spin | 33 | 8198 | 4 |
| epoll-spin | 34 | 8197 | 0 |
| epoll-spin | 35 | 8200 | 4 |
| epoll-spin | 36 | 8197 | 0 |
| epoll-spin | 37 | 8197 | 0 |
| epoll-spin | 38 | 8196 | 4 |
| epoll-spin | 39 | 8194 | 4 |
| epoll-spin | 40 | 8196 | 4 |
| epoll-spin | 41 | 8198 | 4 |
| epoll-spin | 42 | 8196 | 4 |
| epoll-spin | 43 | 8200 | 4 |
| epoll-spin | 44 | 8193 | 0 |
| epoll-spin | 45 | 8195 | 0 |
| epoll-spin | 46 | 8193 | 0 |
| epoll-spin | 47 | 8194 | 4 |
| epoll-spin | 48 | 8196 | 4 |
| epoll-spin | 49 | 8198 | 4 |
| epoll-spin | 50 | 8200 | 4 |
| epoll-spin | 51 | 8198 | 4 |
| epoll-spin | 52 | 8196 | 4 |
| epoll-spin | 53 | 8200 | 4 |
| epoll-spin | 54 | 8196 | 4 |
| epoll-spin | 55 | 8194 | 4 |
| epoll-spin | 56 | 8195 | 0 |
| epoll-spin | 57 | 8199 | 0 |
| epoll-spin | 58 | 8193 | 0 |
| epoll-spin | 59 | 8195 | 0 |
| epoll-spin | 60 | 8200 | 4 |
| epoll-spin | 61 | 8200 | 4 |
| epoll-spin | 62 | 8199 | 0 |
| epoll-spin | 63 | 8193 | 0 |
| epoll-spin | 64 | 8200 | 4 |
| epoll-spin | 65 | 8199 | 0 |
| epoll-spin | 66 | 8198 | 4 |
| epoll-spin | 67 | 8196 | 4 |
| epoll-spin | 68 | 8196 | 4 |
| epoll-spin | 69 | 8198 | 4 |
| epoll-spin | 70 | 8196 | 4 |
| epoll-spin | 71 | 8198 | 4 |
| epoll-spin | 72 | 8196 | 4 |
| epoll-spin | 73 | 8193 | 0 |
| epoll-spin | 74 | 8193 | 0 |
| epoll-spin | 75 | 8200 | 4 |
| epoll-spin | 76 | 8198 | 4 |
| epoll-spin | 77 | 8195 | 0 |
| epoll-spin | 78 | 8196 | 4 |
| epoll-spin | 79 | 8200 | 4 |
| epoll-spin | 80 | 8195 | 0 |
| epoll-spin | 81 | 8197 | 0 |
| epoll-spin | 82 | 8199 | 0 |
| epoll-spin | 83 | 8195 | 0 |
| epoll-spin | 84 | 8196 | 4 |
| epoll-spin | 85 | 8193 | 0 |
| epoll-spin | 86 | 8200 | 4 |
| epoll-spin | 87 | 8199 | 0 |
| epoll-spin | 88 | 8195 | 0 |
| epoll-spin | 89 | 8196 | 4 |
| epoll-spin | 90 | 8194 | 4 |
| epoll-spin | 91 | 8197 | 0 |
| epoll-spin | 92 | 8196 | 4 |
| epoll-spin | 93 | 8197 | 0 |
| epoll-spin | 94 | 8194 | 4 |
| epoll-spin | 95 | 8197 | 0 |
| epoll-spin | 96 | 8200 | 4 |
| epoll-spin | 97 | 8200 | 4 |
| epoll-spin | 98 | 8193 | 0 |
| epoll-spin | 99 | 8198 | 4 |
| epoll-spin | 100 | 8193 | 0 |
| epoll-spin | 101 | 8197 | 0 |
| epoll-spin | 102 | 8196 | 4 |
| epoll-spin | 103 | 8198 | 4 |
| epoll-spin | 104 | 8193 | 0 |
| epoll-spin | 105 | 8193 | 0 |
| epoll-spin | 106 | 8197 | 0 |
| epoll-spin | 107 | 8200 | 4 |
| epoll-spin | 108 | 8196 | 4 |
| epoll-spin | 109 | 8198 | 4 |
| epoll-spin | 110 | 8198 | 4 |
| epoll-spin | 111 | 8195 | 0 |
| epoll-spin | 112 | 8197 | 0 |
| epoll-spin | 113 | 8193 | 0 |
| epoll-spin | 114 | 8197 | 0 |
| epoll-spin | 115 | 8193 | 0 |
| epoll-spin | 116 | 8194 | 4 |
| epoll-spin | 117 | 8197 | 0 |
| epoll-spin | 118 | 8200 | 4 |
| epoll-spin | 119 | 8198 | 4 |
| epoll-spin | 120 | 8196 | 4 |
| epoll-spin | 121 | 8194 | 4 |
| epoll-spin | 122 | 8199 | 0 |
| epoll-spin | 123 | 8194 | 4 |
| epoll-spin | 124 | 8199 | 0 |
| epoll-spin | 125 | 8197 | 0 |
| epoll-spin | 126 | 8193 | 0 |
| epoll-spin | 127 | 8199 | 0 |
| epoll-spin | 128 | 8200 | 4 |
| epoll-spin | 129 | 8193 | 0 |
| epoll-spin | 130 | 8197 | 0 |
| epoll-spin | 131 | 8200 | 4 |
| epoll-spin | 132 | 8200 | 4 |
| epoll-spin | 133 | 8197 | 0 |
| epoll-spin | 134 | 8197 | 0 |
| epoll-spin | 135 | 8194 | 4 |
| epoll-spin | 136 | 8198 | 4 |
| epoll-spin | 137 | 8193 | 0 |
| epoll-spin | 138 | 8198 | 4 |
| epoll-spin | 139 | 8196 | 4 |
| epoll-spin | 140 | 8197 | 0 |
| epoll-spin | 141 | 8198 | 4 |
| epoll-spin | 142 | 8196 | 4 |
| epoll-spin | 143 | 8194 | 4 |
| epoll-spin | 144 | 8195 | 0 |
| epoll-spin | 145 | 8193 | 0 |
| epoll-spin | 146 | 8195 | 0 |
| epoll-spin | 147 | 8195 | 0 |
| epoll-spin | 148 | 8200 | 4 |
| epoll-spin | 149 | 8193 | 0 |
| epoll-spin | 150 | 8200 | 4 |
| epoll-spin | 151 | 8194 | 4 |
| epoll-spin | 152 | 8194 | 4 |
| epoll-spin | 153 | 8200 | 4 |
| epoll-spin | 154 | 8196 | 4 |
| epoll-spin | 155 | 8200 | 4 |
| epoll-spin | 156 | 8194 | 4 |
| epoll-spin | 157 | 8196 | 4 |
| epoll-spin | 158 | 8195 | 0 |
| epoll-spin | 159 | 8196 | 4 |
| epoll-spin | 160 | 8197 | 0 |
| epoll-spin | 161 | 8197 | 0 |
| epoll-spin | 162 | 8198 | 4 |
| epoll-spin | 163 | 8198 | 4 |
| epoll-spin | 164 | 8194 | 4 |
| epoll-spin | 165 | 8197 | 0 |
| epoll-spin | 166 | 8200 | 4 |
| epoll-spin | 167 | 8196 | 4 |
| epoll-spin | 168 | 8195 | 0 |
| epoll-spin | 169 | 8196 | 4 |
| epoll-spin | 170 | 8198 | 4 |
| epoll-spin | 171 | 8193 | 0 |
| epoll-spin | 172 | 8194 | 4 |
| epoll-spin | 173 | 8194 | 4 |
| epoll-spin | 174 | 8195 | 0 |
| epoll-spin | 175 | 8199 | 0 |
| epoll-spin | 176 | 8194 | 4 |
| epoll-spin | 177 | 8195 | 0 |
| epoll-spin | 178 | 8193 | 0 |
| epoll-spin | 179 | 8199 | 0 |
| epoll-spin | 180 | 8200 | 4 |
| epoll-spin | 181 | 8197 | 0 |
| epoll-spin | 182 | 8197 | 0 |
| epoll-spin | 183 | 8196 | 4 |
| epoll-spin | 184 | 8197 | 0 |
| epoll-spin | 185 | 8198 | 4 |
| epoll-spin | 186 | 8195 | 0 |
| epoll-spin | 187 | 8193 | 0 |
| epoll-spin | 188 | 8199 | 0 |
| epoll-spin | 189 | 8200 | 4 |
| epoll-spin | 190 | 8195 | 0 |
| epoll-spin | 191 | 8196 | 4 |
| epoll-spin | 192 | 8198 | 4 |
| epoll-spin | 193 | 8195 | 0 |
| epoll-spin | 194 | 8198 | 4 |
| epoll-spin | 195 | 8200 | 4 |
| epoll-spin | 196 | 8195 | 0 |
| epoll-spin | 197 | 8200 | 4 |
| epoll-spin | 198 | 8199 | 0 |
| epoll-spin | 199 | 8198 | 4 |
| epoll-spin | 200 | 8198 | 4 |
| epoll-spin | 201 | 8196 | 4 |
| epoll-spin | 202 | 8197 | 0 |
| epoll-spin | 203 | 8194 | 4 |
| epoll-spin | 204 | 8194 | 4 |
| epoll-spin | 205 | 8196 | 4 |
| epoll-spin | 206 | 8195 | 0 |
| epoll-spin | 207 | 8199 | 0 |
| epoll-spin | 208 | 8197 | 0 |
| epoll-spin | 209 | 8195 | 0 |
| epoll-spin | 210 | 8195 | 0 |
| epoll-spin | 211 | 8195 | 0 |
| epoll-spin | 212 | 8196 | 4 |
| epoll-spin | 213 | 8194 | 4 |
| epoll-spin | 214 | 8200 | 4 |
| epoll-spin | 215 | 8196 | 4 |
| epoll-spin | 216 | 8195 | 0 |
| epoll-spin | 217 | 8199 | 0 |
| epoll-spin | 218 | 8195 | 0 |
| epoll-spin | 219 | 8196 | 4 |
| epoll-spin | 220 | 8199 | 0 |
| epoll-spin | 221 | 8198 | 4 |
| epoll-spin | 222 | 8200 | 4 |
| epoll-spin | 223 | 8196 | 4 |
| epoll-spin | 224 | 8194 | 4 |
| epoll-spin | 225 | 8195 | 0 |
| epoll-spin | 226 | 8196 | 4 |
| epoll-spin | 227 | 8193 | 0 |
| epoll-spin | 228 | 8196 | 4 |
| epoll-spin | 229 | 8196 | 4 |
| epoll-spin | 230 | 8194 | 4 |
| epoll-spin | 231 | 8199 | 0 |
| epoll-spin | 232 | 8197 | 0 |
| epoll-spin | 233 | 8193 | 0 |
| epoll-spin | 234 | 8194 | 4 |
| epoll-spin | 235 | 8198 | 4 |
| epoll-spin | 236 | 8200 | 4 |
| epoll-spin | 237 | 8199 | 0 |
| epoll-spin | 238 | 8199 | 0 |
| epoll-spin | 239 | 8194 | 4 |
| epoll-spin | 240 | 8193 | 0 |
| epoll-spin | 241 | 8194 | 4 |
| epoll-spin | 242 | 8196 | 4 |
| epoll-spin | 243 | 8193 | 0 |
| epoll-spin | 244 | 8197 | 0 |
| epoll-spin | 245 | 8199 | 0 |
| epoll-spin | 246 | 8195 | 0 |
| epoll-spin | 247 | 8199 | 0 |
| epoll-spin | 248 | 8198 | 4 |
| epoll-spin | 249 | 8200 | 4 |
| epoll-spin | 250 | 8197 | 0 |
| epoll-spin | 251 | 8195 | 0 |
| epoll-spin | 252 | 8200 | 4 |
| epoll-spin | 253 | 8193 | 0 |
| epoll-spin | 254 | 8198 | 4 |
| epoll-spin | 255 | 8196 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1374901472 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (3193909 of 5923963 in 1-msg reads) | 3193909 | 4.45 | 3.39 | 7.72 | 14.27 | 23.82 | 35.73 | 70.83 |

#### Stalls: m2/lab-epoll-spin-c256-s1024, loop 0 on CPU 3

124664832 iterations, longest 1112.8 µs; over 5 / 20 / 100 µs: 359549 / 1627 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1370 | 36465.1 | 66.7 |
| loop work | 256 | 10334.8 | 1112.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.5 | 33.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1112.8 | 2770 | loop work (2770 items) |  |
| 2 | 1006.7 | 2573 | loop work (2573 items) |  |
| 3 | 941.7 | 2384 | loop work (2384 items) |  |
| 4 | 818.7 | 2213 | loop work (2213 items) |  |
| 5 | 152.7 | 147 | loop work (147 items) |  |
| 6 | 66.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 63.3 | 40 | loop work (40 items) |  |
| 8 | 55.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 54.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 51.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 10 |
| LOC Local timer interrupts | 4 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c4-s1024: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 6137907 | 3316751 | 1.85 | 236586831 | 38.55 | 3943114 | 3887835 | 0.0 | 41.7 | false | 0 | 0 | 3037.4 | 263444 / 1175 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8196 | 4 |
| epoll-spin | 1 | 8200 | 4 |
| epoll-spin | 2 | 8196 | 4 |
| epoll-spin | 3 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1374891168 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (2608697 of 5923949 in 1-msg reads) | 2608697 | 4.26 | 3.35 | 7.13 | 12.48 | 21.80 | 34.12 | 55.69 |

#### Stalls: m2/lab-epoll-spin-c4-s1024, loop 0 on CPU 3

233270080 iterations, longest 3037.4 µs; over 5 / 20 / 100 µs: 263444 / 1175 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 681 | 18237.2 | 54.9 |
| loop work | 493 | 15266.2 | 3037.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 25.3 | 25.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3037.4 | 9783 | loop work (9783 items) |  |
| 2 | 240.8 | 1443 | loop work (1443 items) |  |
| 3 | 139.5 | 783 | loop work (783 items) |  |
| 4 | 133.1 | 771 | loop work (771 items) |  |
| 5 | 70.5 | 419 | loop work (419 items) |  |
| 6 | 54.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 53.2 | 20 | loop work (20 items) |  |
| 8 | 50.5 | 11 | loop work (11 items) |  |
| 9 | 49.2 | 145 | loop work (145 items) |  |
| 10 | 47.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 4 |
| TLB TLB shootdowns | 1 |

## m2/lab-epoll-spin-c64-s1024: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=0 clocksource=tsc

### Loop counters (m2/lab-epoll-spin-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 6137909 | 3850306 | 1.59 | 209004994 | 34.05 | 3483417 | 3419245 | 0.0 | 40.3 | false | 0 | 0 | 3431.9 | 318534 / 1397 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| epoll-spin | 0 | 8198 | 4 |
| epoll-spin | 1 | 8198 | 4 |
| epoll-spin | 2 | 8195 | 0 |
| epoll-spin | 3 | 8200 | 4 |
| epoll-spin | 4 | 8199 | 0 |
| epoll-spin | 5 | 8200 | 4 |
| epoll-spin | 6 | 8196 | 4 |
| epoll-spin | 7 | 8197 | 0 |
| epoll-spin | 8 | 8199 | 0 |
| epoll-spin | 9 | 8199 | 0 |
| epoll-spin | 10 | 8198 | 4 |
| epoll-spin | 11 | 8197 | 0 |
| epoll-spin | 12 | 8198 | 4 |
| epoll-spin | 13 | 8195 | 0 |
| epoll-spin | 14 | 8193 | 0 |
| epoll-spin | 15 | 8195 | 0 |
| epoll-spin | 16 | 8200 | 4 |
| epoll-spin | 17 | 8194 | 4 |
| epoll-spin | 18 | 8199 | 0 |
| epoll-spin | 19 | 8198 | 4 |
| epoll-spin | 20 | 8198 | 4 |
| epoll-spin | 21 | 8193 | 0 |
| epoll-spin | 22 | 8194 | 4 |
| epoll-spin | 23 | 8193 | 0 |
| epoll-spin | 24 | 8194 | 4 |
| epoll-spin | 25 | 8195 | 0 |
| epoll-spin | 26 | 8198 | 4 |
| epoll-spin | 27 | 8193 | 0 |
| epoll-spin | 28 | 8198 | 4 |
| epoll-spin | 29 | 8194 | 4 |
| epoll-spin | 30 | 8197 | 0 |
| epoll-spin | 31 | 8195 | 0 |
| epoll-spin | 32 | 8196 | 4 |
| epoll-spin | 33 | 8193 | 0 |
| epoll-spin | 34 | 8197 | 0 |
| epoll-spin | 35 | 8195 | 0 |
| epoll-spin | 36 | 8196 | 4 |
| epoll-spin | 37 | 8197 | 0 |
| epoll-spin | 38 | 8196 | 4 |
| epoll-spin | 39 | 8199 | 0 |
| epoll-spin | 40 | 8198 | 4 |
| epoll-spin | 41 | 8199 | 0 |
| epoll-spin | 42 | 8200 | 4 |
| epoll-spin | 43 | 8193 | 0 |
| epoll-spin | 44 | 8195 | 0 |
| epoll-spin | 45 | 8198 | 4 |
| epoll-spin | 46 | 8197 | 0 |
| epoll-spin | 47 | 8199 | 0 |
| epoll-spin | 48 | 8196 | 4 |
| epoll-spin | 49 | 8199 | 0 |
| epoll-spin | 50 | 8195 | 0 |
| epoll-spin | 51 | 8197 | 0 |
| epoll-spin | 52 | 8194 | 4 |
| epoll-spin | 53 | 8193 | 0 |
| epoll-spin | 54 | 8197 | 0 |
| epoll-spin | 55 | 8196 | 4 |
| epoll-spin | 56 | 8196 | 4 |
| epoll-spin | 57 | 8193 | 0 |
| epoll-spin | 58 | 8200 | 4 |
| epoll-spin | 59 | 8195 | 0 |
| epoll-spin | 60 | 8199 | 0 |
| epoll-spin | 61 | 8199 | 0 |
| epoll-spin | 62 | 8197 | 0 |
| epoll-spin | 63 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| epoll-spin | 1374891616 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| epoll-spin (3194459 of 5923967 in 1-msg reads) | 3194459 | 4.36 | 3.36 | 7.53 | 13.46 | 22.18 | 35.03 | 71.09 |

#### Stalls: m2/lab-epoll-spin-c64-s1024, loop 0 on CPU 3

205154688 iterations, longest 3431.9 µs; over 5 / 20 / 100 µs: 318534 / 1397 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1212 | 32188.5 | 61.3 |
| loop work | 184 | 8224.7 | 3431.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.6 | 21.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3431.9 | 9814 | loop work (9814 items) |  |
| 2 | 128.1 | 241 | loop work (241 items) |  |
| 3 | 61.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 54.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 52.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 50.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 49.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 48.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 46.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 46.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-mio-c1-s1024: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m2/lab-mio-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 6137762 | 0 | 0.00 | 255030224 | 41.55 | 4250504 | 4188095 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

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

#### Stalls: m2/lab-mio-c1-s1024, loop 0 on CPU 3

251285683 iterations, longest 6838.8 µs; over 5 / 20 / 100 µs: 634967 / 13561 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 13556 | 303788.7 | 6838.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 84.7 | 22.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.1 | 23.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 6838.8 | 2738 | loop work (2738 items) |  |
| 2 | 60.4 | 74 | loop work (74 items) |  |
| 3 | 49.2 | 76 | loop work (76 items) |  |
| 4 | 48.7 | 79 | loop work (79 items) |  |
| 5 | 42.8 | 54 | loop work (54 items) |  |
| 6 | 40.2 | 97 | loop work (97 items) |  |
| 7 | 40.0 | 62 | loop work (62 items) |  |
| 8 | 39.4 | 61 | loop work (61 items) |  |
| 9 | 39.1 | 65 | loop work (65 items) |  |
| 10 | 38.4 | 84 | loop work (84 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| LOC Local timer interrupts | 17 |
| IWI IRQ work interrupts | 7 |
| RES Rescheduling interrupts | 2 |

## m2/lab-mio-c16-s1024: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m2/lab-mio-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 6137874 | 0 | 0.00 | 242112635 | 39.45 | 4035211 | 3914390 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

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

#### Stalls: m2/lab-mio-c16-s1024, loop 0 on CPU 3

234863385 iterations, longest 10935.7 µs; over 5 / 20 / 100 µs: 479303 / 1903 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1314 | 34793.5 | 58.0 |
| loop work | 587 | 26182.0 | 10935.7 |
| softirq TIMER | 1 | 173.2 | 173.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.4 | 23.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 10935.7 | 9864 | loop work (9864 items) |  |
| 2 | 173.2 | 0 | softirq TIMER | softirq TIMER 149, local timer tick 15, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 3 | 147.2 | 254 | loop work (254 items) |  |
| 4 | 58.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 48.3 | 30 | loop work (30 items) |  |
| 6 | 48.1 | 7 | loop work (7 items) |  |
| 7 | 46.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 43.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 43.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 43.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 52 |
| LOC Local timer interrupts | 18 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

## m2/lab-mio-c256-s1024: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m2/lab-mio-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 6137879 | 0 | 0.00 | 240697905 | 39.22 | 4011632 | 3885960 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

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

#### Stalls: m2/lab-mio-c256-s1024, loop 0 on CPU 3

233157627 iterations, longest 11193.7 µs; over 5 / 20 / 100 µs: 464008 / 1567 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1267 | 33817.8 | 62.4 |
| loop work | 299 | 19060.6 | 11193.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 25.5 | 25.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 11193.7 | 9885 | loop work (9885 items) | ipi_send_cpu 0 |
| 2 | 288.9 | 244 | loop work (244 items) |  |
| 3 | 72.2 | 20 | loop work (20 items) | ipi_send_cpu 0 |
| 4 | 67.8 | 20 | loop work (20 items) |  |
| 5 | 62.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 57.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 53.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 49.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 48.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 47.7 | 7 | loop work (7 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| IWI IRQ work interrupts | 738 |
| LOC Local timer interrupts | 23 |
| RES Rescheduling interrupts | 1 |

## m2/lab-mio-c4-s1024: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m2/lab-mio-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 6137762 | 0 | 0.00 | 246972528 | 40.24 | 4116209 | 4013594 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

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

#### Stalls: m2/lab-mio-c4-s1024, loop 0 on CPU 3

240815659 iterations, longest 10034.6 µs; over 5 / 20 / 100 µs: 456229 / 2073 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1574 | 46701.9 | 10034.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 498 | 13149.4 | 49.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.5 | 23.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 10034.6 | 9869 | loop work (9869 items) | ipi_send_cpu 0 |
| 2 | 81.0 | 184 | loop work (184 items) |  |
| 3 | 72.6 | 247 | loop work (247 items) |  |
| 4 | 69.5 | 226 | loop work (226 items) |  |
| 5 | 65.1 | 208 | loop work (208 items) |  |
| 6 | 49.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 47.8 | 149 | loop work (149 items) |  |
| 8 | 46.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 45.0 | 28 | loop work (28 items) |  |
| 10 | 44.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| LOC Local timer interrupts | 21 |
| IWI IRQ work interrupts | 14 |
| RES Rescheduling interrupts | 1 |

## m2/lab-mio-c64-s1024: lab, 60 s (first 2 s dropped),  connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us= wait_us= mlockall= BusyPollRxPackets during run= clocksource=

### Loop counters (m2/lab-mio-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| mio | 6137880 | 0 | 0.00 | 241916033 | 39.41 | 4031934 | 3907191 | 0.0 | 0.0 |  |  |  |  |  /  /  | 0 | 0 |

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

#### Stalls: m2/lab-mio-c64-s1024, loop 0 on CPU 3

234431441 iterations, longest 10591.3 µs; over 5 / 20 / 100 µs: 457690 / 1612 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1163 | 31050.8 | 63.1 |
| loop work | 447 | 22716.1 | 10591.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 52.2 | 28.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 10591.3 | 9843 | loop work (9843 items) |  |
| 2 | 197.0 | 260 | loop work (260 items) |  |
| 3 | 70.1 | 20 | loop work (20 items) |  |
| 4 | 69.5 | 20 | loop work (20 items) | ipi_send_cpu 0 |
| 5 | 63.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 61.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 54.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 51.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 48.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 46.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 193 |
| LOC Local timer interrupts | 17 |
| RES Rescheduling interrupts | 2 |

## m2/lab-recv-busy-c1-s1024: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2529444 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137888 | 2898572 | 2.12 | 101716864 | 16.57 | 1695281 | 1695281 | 97.2 | 69.9 | true | 0 | 4605 | 2674.1 | 173044 / 4290 / 5 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374886912 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (1941309 of 5923898 in 1-msg reads) | 1941309 | 2.37 | 2.23 | 2.73 | 3.48 | 13.43 | 17.25 | 30.02 |

#### Stalls: m2/lab-recv-busy-c1-s1024, loop 0 on CPU 3

101716864 iterations, longest 2674.1 µs; over 5 / 20 / 100 µs: 173044 / 4290 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4100 | 99252.3 | 2674.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 89 | 1948.4 | 28.4 |
| mostly unexplained (+preempted by kworker/3:1) | 40 | 955.4 | 32.6 |
| softirq NET_RX | 21 | 465.2 | 29.7 |
| mostly unexplained (+local timer tick) | 13 | 301.1 | 28.3 |
| mostly unexplained (+softirq NET_RX) | 13 | 294.4 | 30.0 |
| softirq TIMER | 2 | 286.8 | 264.2 |
| mostly unexplained (+irq work (IPI/vector)) | 10 | 239.0 | 28.6 |
| mostly unexplained (+ipi_send_cpu) | 1 | 30.9 | 30.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 25.2 | 25.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2674.1 | 9656 | loop work (9656 items) | ipi_send_cpu 0 |
| 2 | 264.2 | 7 | softirq TIMER | softirq TIMER 243, local timer tick 7, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 3 | 117.6 | 399 | loop work (399 items) |  |
| 4 | 105.2 | 599 | loop work (599 items) |  |
| 5 | 101.5 | 589 | loop work (589 items) |  |
| 6 | 62.7 | 232 | loop work (232 items) |  |
| 7 | 47.1 | 47 | loop work (47 items) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 45.4 | 5 | loop work (5 items) | softirq NET_RX 2, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 44.1 | 17 | loop work (17 items) | softirq NET_RX 14, preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 43.7 | 22 | loop work (22 items) | irq work (IPI/vector) 1, preempted by kworker/3:1 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4645 |
| IWI IRQ work interrupts | 4621 |
| CAL Function call interrupts | 991 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-c16-s1024: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3165754 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137883 | 3637933 | 1.69 | 99392512 | 16.19 | 1656542 | 103534 | 96.3 | 73.0 | true | 0 | 5656 | 3294.7 | 6212031 / 117250 / 5 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 4 |
| recv-busy | 1 | 8195 | 0 |
| recv-busy | 2 | 8198 | 3 |
| recv-busy | 3 | 8196 | 3 |
| recv-busy | 4 | 8200 | 3 |
| recv-busy | 5 | 8200 | 3 |
| recv-busy | 6 | 8193 | 3 |
| recv-busy | 7 | 8198 | 3 |
| recv-busy | 8 | 8196 | 4 |
| recv-busy | 9 | 8196 | 3 |
| recv-busy | 10 | 8196 | 3 |
| recv-busy | 11 | 8195 | 3 |
| recv-busy | 12 | 8196 | 3 |
| recv-busy | 13 | 8200 | 4 |
| recv-busy | 14 | 8196 | 4 |
| recv-busy | 15 | 8198 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374885792 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (2994720 of 5923972 in 1-msg reads) | 2994720 | 5.82 | 5.24 | 10.14 | 18.76 | 32.21 | 45.25 | 157.22 |

#### Stalls: m2/lab-recv-busy-c16-s1024, loop 0 on CPU 3

6212032 iterations, longest 3294.7 µs; over 5 / 20 / 100 µs: 6212031 / 117250 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 103814 | 2785828.0 | 3294.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 10143 | 300336.5 | 169.1 |
| mostly unexplained (+preempted by kworker/3:1) | 1348 | 34756.4 | 66.3 |
| mostly unexplained (+softirq NET_RX) | 843 | 24127.8 | 72.3 |
| mostly unexplained (+irq work (IPI/vector)) | 640 | 15625.3 | 53.1 |
| mostly unexplained (+local timer tick) | 451 | 10452.8 | 63.7 |
| softirq NET_RX | 4 | 140.0 | 38.6 |
| mostly unexplained (+softirq NET_TX) | 3 | 115.4 | 44.7 |
| mostly unexplained (+ipi_send_cpu) | 1 | 44.4 | 44.4 |
| mostly unexplained (+softirq RCU) | 2 | 42.6 | 22.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 37.1 | 37.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3294.7 | 9757 | loop work (9757 items) |  |
| 2 | 169.1 | 5 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 128.4 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 119.4 | 46 | loop work (46 items) |  |
| 5 | 101.4 | 32 | loop work (32 items) |  |
| 6 | 99.9 | 47 | loop work (47 items) | softirq NET_RX 1 |
| 7 | 98.7 | 60 | loop work (60 items) |  |
| 8 | 93.7 | 40 | loop work (40 items) |  |
| 9 | 93.2 | 36 | loop work (36 items) | local timer tick 3, timer tick_nohz_handler 1 |
| 10 | 89.3 | 35 | loop work (35 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5674 |
| LOC Local timer interrupts | 5517 |
| CAL Function call interrupts | 990 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-c256-s1024: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2642534 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6138307 | 3419633 | 1.80 | 98435072 | 16.04 | 1640585 | 6409 | 96.5 | 73.1 | true | 0 | 6477 | 3538.5 | 384511 / 384511 / 384511 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 3 |
| recv-busy | 1 | 8197 | 3 |
| recv-busy | 2 | 8198 | 3 |
| recv-busy | 3 | 8199 | 3 |
| recv-busy | 4 | 8193 | 3 |
| recv-busy | 5 | 8195 | 0 |
| recv-busy | 6 | 8197 | 3 |
| recv-busy | 7 | 8193 | 3 |
| recv-busy | 8 | 8193 | 3 |
| recv-busy | 9 | 8197 | 3 |
| recv-busy | 10 | 8198 | 3 |
| recv-busy | 11 | 8200 | 3 |
| recv-busy | 12 | 8200 | 4 |
| recv-busy | 13 | 8195 | 0 |
| recv-busy | 14 | 8198 | 4 |
| recv-busy | 15 | 8195 | 3 |
| recv-busy | 16 | 8197 | 3 |
| recv-busy | 17 | 8195 | 0 |
| recv-busy | 18 | 8195 | 0 |
| recv-busy | 19 | 8193 | 3 |
| recv-busy | 20 | 8200 | 4 |
| recv-busy | 21 | 8198 | 4 |
| recv-busy | 22 | 8197 | 0 |
| recv-busy | 23 | 8193 | 3 |
| recv-busy | 24 | 8196 | 4 |
| recv-busy | 25 | 8199 | 3 |
| recv-busy | 26 | 8200 | 3 |
| recv-busy | 27 | 8200 | 3 |
| recv-busy | 28 | 8196 | 3 |
| recv-busy | 29 | 8200 | 3 |
| recv-busy | 30 | 8197 | 0 |
| recv-busy | 31 | 8198 | 4 |
| recv-busy | 32 | 8200 | 4 |
| recv-busy | 33 | 8196 | 3 |
| recv-busy | 34 | 8200 | 4 |
| recv-busy | 35 | 8198 | 4 |
| recv-busy | 36 | 8198 | 3 |
| recv-busy | 37 | 8193 | 0 |
| recv-busy | 38 | 8198 | 3 |
| recv-busy | 39 | 8197 | 3 |
| recv-busy | 40 | 8196 | 3 |
| recv-busy | 41 | 8196 | 4 |
| recv-busy | 42 | 8200 | 3 |
| recv-busy | 43 | 8194 | 3 |
| recv-busy | 44 | 8195 | 3 |
| recv-busy | 45 | 8197 | 3 |
| recv-busy | 46 | 8193 | 3 |
| recv-busy | 47 | 8196 | 3 |
| recv-busy | 48 | 8194 | 3 |
| recv-busy | 49 | 8196 | 3 |
| recv-busy | 50 | 8194 | 4 |
| recv-busy | 51 | 8196 | 3 |
| recv-busy | 52 | 8199 | 3 |
| recv-busy | 53 | 8198 | 3 |
| recv-busy | 54 | 8200 | 3 |
| recv-busy | 55 | 8195 | 3 |
| recv-busy | 56 | 8196 | 3 |
| recv-busy | 57 | 8194 | 3 |
| recv-busy | 58 | 8193 | 3 |
| recv-busy | 59 | 8196 | 4 |
| recv-busy | 60 | 8195 | 0 |
| recv-busy | 61 | 8196 | 3 |
| recv-busy | 62 | 8193 | 3 |
| recv-busy | 63 | 8198 | 3 |
| recv-busy | 64 | 8194 | 3 |
| recv-busy | 65 | 8200 | 4 |
| recv-busy | 66 | 8196 | 4 |
| recv-busy | 67 | 8197 | 3 |
| recv-busy | 68 | 8200 | 4 |
| recv-busy | 69 | 8194 | 3 |
| recv-busy | 70 | 8199 | 3 |
| recv-busy | 71 | 8198 | 4 |
| recv-busy | 72 | 8194 | 3 |
| recv-busy | 73 | 8198 | 4 |
| recv-busy | 74 | 8199 | 3 |
| recv-busy | 75 | 8193 | 3 |
| recv-busy | 76 | 8199 | 3 |
| recv-busy | 77 | 8194 | 3 |
| recv-busy | 78 | 8197 | 3 |
| recv-busy | 79 | 8199 | 0 |
| recv-busy | 80 | 8199 | 0 |
| recv-busy | 81 | 8199 | 0 |
| recv-busy | 82 | 8195 | 0 |
| recv-busy | 83 | 8195 | 0 |
| recv-busy | 84 | 8197 | 3 |
| recv-busy | 85 | 8198 | 3 |
| recv-busy | 86 | 8194 | 3 |
| recv-busy | 87 | 8195 | 0 |
| recv-busy | 88 | 8195 | 3 |
| recv-busy | 89 | 8194 | 4 |
| recv-busy | 90 | 8196 | 4 |
| recv-busy | 91 | 8193 | 0 |
| recv-busy | 92 | 8193 | 3 |
| recv-busy | 93 | 8199 | 3 |
| recv-busy | 94 | 8198 | 3 |
| recv-busy | 95 | 8200 | 4 |
| recv-busy | 96 | 8198 | 4 |
| recv-busy | 97 | 8198 | 4 |
| recv-busy | 98 | 8197 | 3 |
| recv-busy | 99 | 8200 | 4 |
| recv-busy | 100 | 8197 | 3 |
| recv-busy | 101 | 8199 | 0 |
| recv-busy | 102 | 8195 | 0 |
| recv-busy | 103 | 8194 | 3 |
| recv-busy | 104 | 8197 | 3 |
| recv-busy | 105 | 8198 | 3 |
| recv-busy | 106 | 8194 | 4 |
| recv-busy | 107 | 8197 | 0 |
| recv-busy | 108 | 8193 | 3 |
| recv-busy | 109 | 8196 | 4 |
| recv-busy | 110 | 8198 | 4 |
| recv-busy | 111 | 8199 | 0 |
| recv-busy | 112 | 8193 | 3 |
| recv-busy | 113 | 8194 | 3 |
| recv-busy | 114 | 8198 | 3 |
| recv-busy | 115 | 8193 | 3 |
| recv-busy | 116 | 8199 | 0 |
| recv-busy | 117 | 8198 | 3 |
| recv-busy | 118 | 8200 | 4 |
| recv-busy | 119 | 8193 | 3 |
| recv-busy | 120 | 8197 | 0 |
| recv-busy | 121 | 8199 | 3 |
| recv-busy | 122 | 8194 | 3 |
| recv-busy | 123 | 8198 | 4 |
| recv-busy | 124 | 8197 | 3 |
| recv-busy | 125 | 8193 | 0 |
| recv-busy | 126 | 8197 | 3 |
| recv-busy | 127 | 8198 | 4 |
| recv-busy | 128 | 8199 | 0 |
| recv-busy | 129 | 8200 | 3 |
| recv-busy | 130 | 8196 | 3 |
| recv-busy | 131 | 8200 | 3 |
| recv-busy | 132 | 8196 | 3 |
| recv-busy | 133 | 8195 | 0 |
| recv-busy | 134 | 8200 | 3 |
| recv-busy | 135 | 8195 | 0 |
| recv-busy | 136 | 8194 | 3 |
| recv-busy | 137 | 8196 | 3 |
| recv-busy | 138 | 8197 | 3 |
| recv-busy | 139 | 8195 | 0 |
| recv-busy | 140 | 8198 | 3 |
| recv-busy | 141 | 8200 | 4 |
| recv-busy | 142 | 8195 | 0 |
| recv-busy | 143 | 8194 | 3 |
| recv-busy | 144 | 8195 | 3 |
| recv-busy | 145 | 8197 | 3 |
| recv-busy | 146 | 8194 | 3 |
| recv-busy | 147 | 8199 | 3 |
| recv-busy | 148 | 8199 | 3 |
| recv-busy | 149 | 8200 | 4 |
| recv-busy | 150 | 8199 | 3 |
| recv-busy | 151 | 8196 | 4 |
| recv-busy | 152 | 8198 | 3 |
| recv-busy | 153 | 8196 | 3 |
| recv-busy | 154 | 8193 | 3 |
| recv-busy | 155 | 8194 | 3 |
| recv-busy | 156 | 8195 | 0 |
| recv-busy | 157 | 8196 | 3 |
| recv-busy | 158 | 8199 | 0 |
| recv-busy | 159 | 8194 | 3 |
| recv-busy | 160 | 8193 | 3 |
| recv-busy | 161 | 8193 | 3 |
| recv-busy | 162 | 8194 | 3 |
| recv-busy | 163 | 8197 | 3 |
| recv-busy | 164 | 8197 | 0 |
| recv-busy | 165 | 8196 | 3 |
| recv-busy | 166 | 8193 | 3 |
| recv-busy | 167 | 8193 | 3 |
| recv-busy | 168 | 8199 | 3 |
| recv-busy | 169 | 8195 | 0 |
| recv-busy | 170 | 8193 | 3 |
| recv-busy | 171 | 8198 | 4 |
| recv-busy | 172 | 8194 | 3 |
| recv-busy | 173 | 8193 | 3 |
| recv-busy | 174 | 8198 | 3 |
| recv-busy | 175 | 8195 | 0 |
| recv-busy | 176 | 8193 | 3 |
| recv-busy | 177 | 8198 | 3 |
| recv-busy | 178 | 8198 | 4 |
| recv-busy | 179 | 8197 | 3 |
| recv-busy | 180 | 8197 | 3 |
| recv-busy | 181 | 8199 | 0 |
| recv-busy | 182 | 8194 | 3 |
| recv-busy | 183 | 8196 | 3 |
| recv-busy | 184 | 8200 | 3 |
| recv-busy | 185 | 8199 | 3 |
| recv-busy | 186 | 8194 | 4 |
| recv-busy | 187 | 8193 | 3 |
| recv-busy | 188 | 8200 | 3 |
| recv-busy | 189 | 8198 | 3 |
| recv-busy | 190 | 8196 | 4 |
| recv-busy | 191 | 8194 | 4 |
| recv-busy | 192 | 8195 | 0 |
| recv-busy | 193 | 8196 | 3 |
| recv-busy | 194 | 8199 | 0 |
| recv-busy | 195 | 8193 | 3 |
| recv-busy | 196 | 8193 | 3 |
| recv-busy | 197 | 8196 | 3 |
| recv-busy | 198 | 8199 | 0 |
| recv-busy | 199 | 8198 | 4 |
| recv-busy | 200 | 8199 | 3 |
| recv-busy | 201 | 8194 | 3 |
| recv-busy | 202 | 8200 | 4 |
| recv-busy | 203 | 8193 | 3 |
| recv-busy | 204 | 8196 | 3 |
| recv-busy | 205 | 8195 | 3 |
| recv-busy | 206 | 8198 | 3 |
| recv-busy | 207 | 8194 | 3 |
| recv-busy | 208 | 8193 | 3 |
| recv-busy | 209 | 8195 | 3 |
| recv-busy | 210 | 8197 | 3 |
| recv-busy | 211 | 8193 | 0 |
| recv-busy | 212 | 8194 | 3 |
| recv-busy | 213 | 8199 | 3 |
| recv-busy | 214 | 8193 | 0 |
| recv-busy | 215 | 8197 | 3 |
| recv-busy | 216 | 8197 | 3 |
| recv-busy | 217 | 8193 | 3 |
| recv-busy | 218 | 8199 | 3 |
| recv-busy | 219 | 8193 | 3 |
| recv-busy | 220 | 8196 | 3 |
| recv-busy | 221 | 8196 | 3 |
| recv-busy | 222 | 8197 | 0 |
| recv-busy | 223 | 8194 | 4 |
| recv-busy | 224 | 8197 | 3 |
| recv-busy | 225 | 8193 | 3 |
| recv-busy | 226 | 8200 | 4 |
| recv-busy | 227 | 8194 | 3 |
| recv-busy | 228 | 8199 | 3 |
| recv-busy | 229 | 8199 | 0 |
| recv-busy | 230 | 8194 | 3 |
| recv-busy | 231 | 8199 | 0 |
| recv-busy | 232 | 8194 | 4 |
| recv-busy | 233 | 8196 | 3 |
| recv-busy | 234 | 8197 | 0 |
| recv-busy | 235 | 8199 | 3 |
| recv-busy | 236 | 8200 | 3 |
| recv-busy | 237 | 8195 | 3 |
| recv-busy | 238 | 8200 | 3 |
| recv-busy | 239 | 8200 | 3 |
| recv-busy | 240 | 8199 | 3 |
| recv-busy | 241 | 8200 | 3 |
| recv-busy | 242 | 8194 | 3 |
| recv-busy | 243 | 8200 | 3 |
| recv-busy | 244 | 8200 | 4 |
| recv-busy | 245 | 8198 | 3 |
| recv-busy | 246 | 8197 | 3 |
| recv-busy | 247 | 8198 | 4 |
| recv-busy | 248 | 8193 | 3 |
| recv-busy | 249 | 8194 | 4 |
| recv-busy | 250 | 8199 | 3 |
| recv-busy | 251 | 8198 | 3 |
| recv-busy | 252 | 8194 | 4 |
| recv-busy | 253 | 8199 | 3 |
| recv-busy | 254 | 8196 | 4 |
| recv-busy | 255 | 8193 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374980768 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (2946329 of 5924399 in 1-msg reads) | 2946329 | 78.09 | 76.97 | 139.98 | 181.93 | 220.81 | 248.50 | 309.15 |

#### Stalls: m2/lab-recv-busy-c256-s1024, loop 0 on CPU 3

384512 iterations, longest 3538.5 µs; over 5 / 20 / 100 µs: 384511 / 384511 / 384511

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 227550 | 37125466.1 | 3538.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 134175 | 19390814.2 | 251.1 |
| mostly unexplained (+softirq NET_RX) | 17910 | 2672167.5 | 255.4 |
| mostly unexplained (+local timer tick) | 2970 | 467948.6 | 214.7 |
| mostly unexplained (+preempted by kworker/3:1) | 1509 | 248010.1 | 265.4 |
| mostly unexplained (+irq work (IPI/vector)) | 372 | 60077.4 | 219.3 |
| mostly unexplained (+softirq NET_TX) | 10 | 1700.8 | 210.5 |
| mostly unexplained (+ipi_send_cpu) | 10 | 1510.2 | 163.2 |
| mostly unexplained (+call function (IPI/vector)) | 4 | 615.6 | 169.5 |
| mostly unexplained (+softirq TIMER) | 1 | 189.7 | 189.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3538.5 | 9811 | loop work (9811 items) |  |
| 2 | 405.9 | 159 | loop work (159 items) | softirq NET_TX 1 |
| 3 | 317.4 | 43 | loop work (43 items) | softirq NET_RX 1, softirq NET_TX 1 |
| 4 | 305.2 | 61 | loop work (61 items) | softirq NET_RX 2 |
| 5 | 301.8 | 87 | loop work (87 items) |  |
| 6 | 299.4 | 104 | loop work (104 items) | softirq NET_RX 1 |
| 7 | 298.7 | 106 | loop work (106 items) | softirq NET_RX 18, preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 297.4 | 110 | loop work (110 items) |  |
| 9 | 295.7 | 49 | loop work (49 items) |  |
| 10 | 295.1 | 83 | loop work (83 items) | softirq NET_RX 7 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6523 |
| LOC Local timer interrupts | 6252 |
| CAL Function call interrupts | 990 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-c4-s1024: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3141323 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137892 | 3438314 | 1.79 | 100537344 | 16.38 | 1675622 | 418906 | 96.6 | 72.1 | true | 0 | 4970 | 3008.7 | 690461 / 23993 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 3 |
| recv-busy | 1 | 8200 | 4 |
| recv-busy | 2 | 8200 | 4 |
| recv-busy | 3 | 8193 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374887808 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (2688112 of 5923950 in 1-msg reads) | 2688112 | 2.64 | 2.39 | 3.52 | 7.62 | 16.09 | 25.25 | 44.68 |

#### Stalls: m2/lab-recv-busy-c4-s1024, loop 0 on CPU 3

25134336 iterations, longest 3008.7 µs; over 5 / 20 / 100 µs: 690461 / 23993 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 16407 | 437665.2 | 3008.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 6929 | 156882.4 | 54.2 |
| mostly unexplained (+softirq NET_RX) | 365 | 8579.2 | 44.7 |
| mostly unexplained (+preempted by kworker/3:1) | 187 | 4588.8 | 46.1 |
| mostly unexplained (+irq work (IPI/vector)) | 39 | 1152.5 | 41.2 |
| mostly unexplained (+local timer tick) | 42 | 1124.8 | 46.3 |
| softirq NET_RX | 21 | 472.2 | 31.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.7 | 32.6 |
| mostly unexplained (+softirq NET_TX) | 1 | 39.4 | 39.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3008.7 | 9749 | loop work (9749 items) |  |
| 2 | 114.8 | 6 | loop work (6 items) |  |
| 3 | 71.9 | 30 | loop work (30 items) |  |
| 4 | 64.7 | 35 | loop work (35 items) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 62.7 | 24 | loop work (24 items) |  |
| 6 | 61.7 | 39 | loop work (39 items) |  |
| 7 | 61.5 | 40 | loop work (40 items) | softirq NET_RX 13, preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 61.3 | 21 | loop work (21 items) |  |
| 9 | 59.8 | 51 | loop work (51 items) |  |
| 10 | 59.0 | 40 | loop work (40 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4994 |
| LOC Local timer interrupts | 4896 |
| CAL Function call interrupts | 990 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-c64-s1024: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2748437 clocksource=tsc

### Loop counters (m2/lab-recv-busy-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137920 | 3531249 | 1.74 | 98619392 | 16.07 | 1643657 | 25682 | 96.4 | 73.0 | true | 0 | 6445 | 3427.8 | 1540927 / 1540927 / 1431 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 3 |
| recv-busy | 1 | 8197 | 3 |
| recv-busy | 2 | 8193 | 3 |
| recv-busy | 3 | 8194 | 3 |
| recv-busy | 4 | 8198 | 4 |
| recv-busy | 5 | 8196 | 3 |
| recv-busy | 6 | 8197 | 0 |
| recv-busy | 7 | 8197 | 3 |
| recv-busy | 8 | 8193 | 3 |
| recv-busy | 9 | 8194 | 3 |
| recv-busy | 10 | 8200 | 4 |
| recv-busy | 11 | 8194 | 3 |
| recv-busy | 12 | 8197 | 3 |
| recv-busy | 13 | 8200 | 4 |
| recv-busy | 14 | 8193 | 3 |
| recv-busy | 15 | 8195 | 0 |
| recv-busy | 16 | 8195 | 0 |
| recv-busy | 17 | 8199 | 3 |
| recv-busy | 18 | 8200 | 4 |
| recv-busy | 19 | 8198 | 4 |
| recv-busy | 20 | 8196 | 4 |
| recv-busy | 21 | 8198 | 3 |
| recv-busy | 22 | 8196 | 3 |
| recv-busy | 23 | 8196 | 3 |
| recv-busy | 24 | 8194 | 3 |
| recv-busy | 25 | 8196 | 3 |
| recv-busy | 26 | 8196 | 3 |
| recv-busy | 27 | 8199 | 3 |
| recv-busy | 28 | 8197 | 3 |
| recv-busy | 29 | 8197 | 3 |
| recv-busy | 30 | 8193 | 3 |
| recv-busy | 31 | 8199 | 3 |
| recv-busy | 32 | 8198 | 4 |
| recv-busy | 33 | 8195 | 3 |
| recv-busy | 34 | 8199 | 3 |
| recv-busy | 35 | 8197 | 3 |
| recv-busy | 36 | 8196 | 4 |
| recv-busy | 37 | 8193 | 0 |
| recv-busy | 38 | 8194 | 3 |
| recv-busy | 39 | 8196 | 3 |
| recv-busy | 40 | 8200 | 3 |
| recv-busy | 41 | 8196 | 4 |
| recv-busy | 42 | 8193 | 3 |
| recv-busy | 43 | 8196 | 4 |
| recv-busy | 44 | 8194 | 3 |
| recv-busy | 45 | 8193 | 0 |
| recv-busy | 46 | 8199 | 3 |
| recv-busy | 47 | 8194 | 3 |
| recv-busy | 48 | 8200 | 3 |
| recv-busy | 49 | 8197 | 0 |
| recv-busy | 50 | 8196 | 3 |
| recv-busy | 51 | 8197 | 3 |
| recv-busy | 52 | 8197 | 3 |
| recv-busy | 53 | 8193 | 0 |
| recv-busy | 54 | 8195 | 3 |
| recv-busy | 55 | 8194 | 3 |
| recv-busy | 56 | 8198 | 4 |
| recv-busy | 57 | 8199 | 3 |
| recv-busy | 58 | 8196 | 3 |
| recv-busy | 59 | 8200 | 4 |
| recv-busy | 60 | 8193 | 3 |
| recv-busy | 61 | 8195 | 0 |
| recv-busy | 62 | 8195 | 3 |
| recv-busy | 63 | 8195 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374894080 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (2968479 of 5924004 in 1-msg reads) | 2968479 | 19.67 | 19.00 | 35.78 | 53.43 | 69.97 | 87.70 | 130.88 |

#### Stalls: m2/lab-recv-busy-c64-s1024, loop 0 on CPU 3

1540928 iterations, longest 3427.8 µs; over 5 / 20 / 100 µs: 1540927 / 1540927 / 1431 (492351 stalls not kept)

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 575006 | 23988860.0 | 3427.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 456752 | 16032762.6 | 102.4 |
| mostly unexplained (+softirq NET_RX) | 12810 | 495812.3 | 91.3 |
| mostly unexplained (+local timer tick) | 2491 | 112621.2 | 100.0 |
| mostly unexplained (+preempted by kworker/3:1) | 933 | 48904.3 | 107.0 |
| mostly unexplained (+irq work (IPI/vector)) | 566 | 28028.0 | 81.0 |
| mostly unexplained (+ipi_send_cpu) | 10 | 416.9 | 58.0 |
| mostly unexplained (+softirq NET_TX) | 4 | 178.4 | 55.6 |
| mostly unexplained (+call function (IPI/vector)) | 3 | 121.7 | 41.5 |
| mostly unexplained (+softirq RCU) | 1 | 46.1 | 46.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3427.8 | 9792 | loop work (9792 items) |  |
| 2 | 163.9 | 105 | loop work (105 items) |  |
| 3 | 156.8 | 68 | loop work (68 items) | softirq NET_RX 2 |
| 4 | 154.6 | 113 | loop work (113 items) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 151.9 | 101 | loop work (101 items) |  |
| 6 | 149.0 | 26 | loop work (26 items) |  |
| 7 | 147.0 | 74 | loop work (74 items) |  |
| 8 | 144.3 | 81 | loop work (81 items) |  |
| 9 | 144.3 | 64 | loop work (64 items) | preempted by kworker/3:1 2, softirq NET_RX 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 144.0 | 81 | loop work (81 items) | softirq NET_RX 4 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6490 |
| LOC Local timer interrupts | 6183 |
| CAL Function call interrupts | 992 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c1-s1024: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2956879 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137878 | 3274000 | 1.87 | 139830912 | 22.78 | 2330515 | 1165258 | 95.3 | 63.7 | true | 0 | 3625 | 2687.2 | 174116 / 5515 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374884672 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (2364012 of 5923894 in 1-msg reads) | 2364012 | 2.40 | 2.28 | 2.77 | 3.71 | 7.87 | 16.90 | 30.68 |

#### Stalls: m2/lab-recv-busy-pq-c1-s1024, loop 0 on CPU 3

69915456 iterations, longest 2687.2 µs; over 5 / 20 / 100 µs: 174116 / 5515 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5181 | 122696.6 | 2687.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 137 | 3190.5 | 47.2 |
| mostly unexplained (+preempted by kworker/3:1) | 114 | 2724.0 | 41.4 |
| softirq NET_RX | 32 | 707.9 | 31.2 |
| mostly unexplained (+local timer tick) | 19 | 455.9 | 35.4 |
| mostly unexplained (+irq work (IPI/vector)) | 16 | 419.3 | 36.2 |
| mostly unexplained (+softirq NET_RX) | 15 | 347.5 | 30.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 25.2 | 25.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2687.2 | 9644 | loop work (9644 items) | ipi_send_cpu 0 |
| 2 | 61.2 | 237 | loop work (237 items) |  |
| 3 | 56.5 | 35 | loop work (35 items) | softirq NET_RX 2 |
| 4 | 52.0 | 20 | loop work (20 items) | softirq NET_RX 4 |
| 5 | 50.1 | 87 | loop work (87 items) | softirq NET_RX 5 |
| 6 | 49.1 | 26 | loop work (26 items) | softirq NET_RX 11 |
| 7 | 48.5 | 24 | loop work (24 items) |  |
| 8 | 48.2 | 33 | loop work (33 items) | softirq NET_RX 3 |
| 9 | 47.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 46.7 | 28 | loop work (28 items) | softirq NET_RX 2 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3665 |
| IWI IRQ work interrupts | 3635 |
| CAL Function call interrupts | 992 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c16-s1024: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3094133 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137853 | 3676834 | 1.67 | 104715239 | 17.06 | 1745254 | 213703 | 96.0 | 71.8 | true | 0 | 6333 | 3340.4 | 2861896 / 48000 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 3 |
| recv-busy | 1 | 8194 | 4 |
| recv-busy | 2 | 8198 | 4 |
| recv-busy | 3 | 8197 | 0 |
| recv-busy | 4 | 8195 | 0 |
| recv-busy | 5 | 8197 | 0 |
| recv-busy | 6 | 8193 | 3 |
| recv-busy | 7 | 8199 | 3 |
| recv-busy | 8 | 8194 | 4 |
| recv-busy | 9 | 8194 | 3 |
| recv-busy | 10 | 8198 | 4 |
| recv-busy | 11 | 8195 | 0 |
| recv-busy | 12 | 8194 | 3 |
| recv-busy | 13 | 8199 | 0 |
| recv-busy | 14 | 8197 | 0 |
| recv-busy | 15 | 8196 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374879072 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (3014034 of 5923943 in 1-msg reads) | 3014034 | 4.92 | 4.64 | 8.06 | 16.02 | 28.72 | 40.27 | 64.81 |

#### Stalls: m2/lab-recv-busy-pq-c16-s1024, loop 0 on CPU 3

12822208 iterations, longest 3340.4 µs; over 5 / 20 / 100 µs: 2861896 / 48000 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 40020 | 1120772.7 | 3340.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 7027 | 181518.3 | 50.3 |
| mostly unexplained (+preempted by kworker/3:1) | 652 | 14952.8 | 48.0 |
| mostly unexplained (+softirq NET_RX) | 140 | 3627.6 | 60.2 |
| mostly unexplained (+irq work (IPI/vector)) | 100 | 2602.6 | 45.9 |
| mostly unexplained (+local timer tick) | 52 | 1585.0 | 51.3 |
| softirq NET_RX | 6 | 179.7 | 39.7 |
| softirq TIMER | 1 | 74.6 | 74.6 |
| mostly unexplained (+ipi_send_cpu) | 2 | 59.6 | 34.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3340.4 | 9764 | loop work (9764 items) |  |
| 2 | 98.4 | 59 | loop work (59 items) |  |
| 3 | 91.8 | 48 | loop work (48 items) | softirq NET_RX 1 |
| 4 | 87.2 | 52 | loop work (52 items) |  |
| 5 | 82.1 | 33 | loop work (33 items) |  |
| 6 | 81.5 | 52 | loop work (52 items) | irq work (IPI/vector) 1, preempted by kworker/3:1 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 80.5 | 49 | loop work (49 items) |  |
| 8 | 78.1 | 58 | loop work (58 items) |  |
| 9 | 77.5 | 38 | loop work (38 items) |  |
| 10 | 76.8 | 51 | loop work (51 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6371 |
| LOC Local timer interrupts | 6092 |
| CAL Function call interrupts | 991 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c256-s1024: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2896297 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137912 | 3735430 | 1.64 | 96105413 | 15.66 | 1601757 | 171258 | 95.6 | 69.6 | true | 0 | 7039 | 1389.0 | 3706866 / 76041 / 6 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 3 |
| recv-busy | 1 | 8197 | 3 |
| recv-busy | 2 | 8195 | 3 |
| recv-busy | 3 | 8193 | 3 |
| recv-busy | 4 | 8198 | 4 |
| recv-busy | 5 | 8196 | 3 |
| recv-busy | 6 | 8200 | 3 |
| recv-busy | 7 | 8200 | 3 |
| recv-busy | 8 | 8194 | 3 |
| recv-busy | 9 | 8195 | 3 |
| recv-busy | 10 | 8198 | 4 |
| recv-busy | 11 | 8200 | 3 |
| recv-busy | 12 | 8196 | 4 |
| recv-busy | 13 | 8199 | 3 |
| recv-busy | 14 | 8200 | 3 |
| recv-busy | 15 | 8198 | 3 |
| recv-busy | 16 | 8193 | 3 |
| recv-busy | 17 | 8195 | 3 |
| recv-busy | 18 | 8200 | 3 |
| recv-busy | 19 | 8197 | 3 |
| recv-busy | 20 | 8194 | 4 |
| recv-busy | 21 | 8196 | 3 |
| recv-busy | 22 | 8193 | 0 |
| recv-busy | 23 | 8194 | 4 |
| recv-busy | 24 | 8196 | 3 |
| recv-busy | 25 | 8193 | 3 |
| recv-busy | 26 | 8200 | 3 |
| recv-busy | 27 | 8195 | 3 |
| recv-busy | 28 | 8198 | 3 |
| recv-busy | 29 | 8196 | 3 |
| recv-busy | 30 | 8198 | 3 |
| recv-busy | 31 | 8193 | 3 |
| recv-busy | 32 | 8195 | 0 |
| recv-busy | 33 | 8197 | 3 |
| recv-busy | 34 | 8193 | 3 |
| recv-busy | 35 | 8199 | 3 |
| recv-busy | 36 | 8200 | 3 |
| recv-busy | 37 | 8194 | 3 |
| recv-busy | 38 | 8197 | 0 |
| recv-busy | 39 | 8198 | 4 |
| recv-busy | 40 | 8196 | 4 |
| recv-busy | 41 | 8197 | 0 |
| recv-busy | 42 | 8195 | 3 |
| recv-busy | 43 | 8197 | 3 |
| recv-busy | 44 | 8196 | 4 |
| recv-busy | 45 | 8193 | 3 |
| recv-busy | 46 | 8198 | 4 |
| recv-busy | 47 | 8193 | 3 |
| recv-busy | 48 | 8197 | 3 |
| recv-busy | 49 | 8197 | 0 |
| recv-busy | 50 | 8199 | 3 |
| recv-busy | 51 | 8197 | 0 |
| recv-busy | 52 | 8194 | 3 |
| recv-busy | 53 | 8199 | 3 |
| recv-busy | 54 | 8200 | 3 |
| recv-busy | 55 | 8193 | 3 |
| recv-busy | 56 | 8195 | 0 |
| recv-busy | 57 | 8199 | 3 |
| recv-busy | 58 | 8197 | 3 |
| recv-busy | 59 | 8200 | 4 |
| recv-busy | 60 | 8197 | 0 |
| recv-busy | 61 | 8199 | 3 |
| recv-busy | 62 | 8193 | 3 |
| recv-busy | 63 | 8200 | 4 |
| recv-busy | 64 | 8194 | 3 |
| recv-busy | 65 | 8196 | 4 |
| recv-busy | 66 | 8200 | 3 |
| recv-busy | 67 | 8195 | 3 |
| recv-busy | 68 | 8199 | 3 |
| recv-busy | 69 | 8194 | 4 |
| recv-busy | 70 | 8198 | 4 |
| recv-busy | 71 | 8199 | 3 |
| recv-busy | 72 | 8194 | 3 |
| recv-busy | 73 | 8197 | 0 |
| recv-busy | 74 | 8200 | 3 |
| recv-busy | 75 | 8195 | 3 |
| recv-busy | 76 | 8199 | 3 |
| recv-busy | 77 | 8196 | 3 |
| recv-busy | 78 | 8195 | 3 |
| recv-busy | 79 | 8200 | 3 |
| recv-busy | 80 | 8196 | 3 |
| recv-busy | 81 | 8194 | 3 |
| recv-busy | 82 | 8196 | 3 |
| recv-busy | 83 | 8200 | 3 |
| recv-busy | 84 | 8195 | 0 |
| recv-busy | 85 | 8197 | 3 |
| recv-busy | 86 | 8195 | 3 |
| recv-busy | 87 | 8193 | 3 |
| recv-busy | 88 | 8193 | 3 |
| recv-busy | 89 | 8200 | 3 |
| recv-busy | 90 | 8198 | 4 |
| recv-busy | 91 | 8197 | 0 |
| recv-busy | 92 | 8199 | 3 |
| recv-busy | 93 | 8194 | 3 |
| recv-busy | 94 | 8194 | 3 |
| recv-busy | 95 | 8197 | 3 |
| recv-busy | 96 | 8193 | 3 |
| recv-busy | 97 | 8197 | 3 |
| recv-busy | 98 | 8197 | 0 |
| recv-busy | 99 | 8199 | 3 |
| recv-busy | 100 | 8195 | 0 |
| recv-busy | 101 | 8199 | 0 |
| recv-busy | 102 | 8193 | 0 |
| recv-busy | 103 | 8200 | 4 |
| recv-busy | 104 | 8197 | 0 |
| recv-busy | 105 | 8195 | 3 |
| recv-busy | 106 | 8196 | 3 |
| recv-busy | 107 | 8195 | 3 |
| recv-busy | 108 | 8198 | 3 |
| recv-busy | 109 | 8194 | 4 |
| recv-busy | 110 | 8199 | 3 |
| recv-busy | 111 | 8198 | 3 |
| recv-busy | 112 | 8194 | 3 |
| recv-busy | 113 | 8196 | 3 |
| recv-busy | 114 | 8193 | 3 |
| recv-busy | 115 | 8198 | 3 |
| recv-busy | 116 | 8195 | 3 |
| recv-busy | 117 | 8196 | 3 |
| recv-busy | 118 | 8195 | 3 |
| recv-busy | 119 | 8194 | 4 |
| recv-busy | 120 | 8195 | 3 |
| recv-busy | 121 | 8200 | 3 |
| recv-busy | 122 | 8197 | 3 |
| recv-busy | 123 | 8198 | 4 |
| recv-busy | 124 | 8198 | 3 |
| recv-busy | 125 | 8198 | 3 |
| recv-busy | 126 | 8198 | 4 |
| recv-busy | 127 | 8194 | 3 |
| recv-busy | 128 | 8197 | 3 |
| recv-busy | 129 | 8200 | 3 |
| recv-busy | 130 | 8193 | 3 |
| recv-busy | 131 | 8194 | 3 |
| recv-busy | 132 | 8193 | 3 |
| recv-busy | 133 | 8193 | 0 |
| recv-busy | 134 | 8194 | 3 |
| recv-busy | 135 | 8199 | 3 |
| recv-busy | 136 | 8199 | 3 |
| recv-busy | 137 | 8198 | 4 |
| recv-busy | 138 | 8198 | 3 |
| recv-busy | 139 | 8199 | 0 |
| recv-busy | 140 | 8194 | 3 |
| recv-busy | 141 | 8197 | 3 |
| recv-busy | 142 | 8194 | 3 |
| recv-busy | 143 | 8193 | 3 |
| recv-busy | 144 | 8193 | 3 |
| recv-busy | 145 | 8196 | 3 |
| recv-busy | 146 | 8195 | 3 |
| recv-busy | 147 | 8194 | 3 |
| recv-busy | 148 | 8198 | 3 |
| recv-busy | 149 | 8197 | 3 |
| recv-busy | 150 | 8197 | 0 |
| recv-busy | 151 | 8200 | 4 |
| recv-busy | 152 | 8197 | 3 |
| recv-busy | 153 | 8196 | 3 |
| recv-busy | 154 | 8198 | 4 |
| recv-busy | 155 | 8193 | 3 |
| recv-busy | 156 | 8193 | 0 |
| recv-busy | 157 | 8199 | 3 |
| recv-busy | 158 | 8200 | 4 |
| recv-busy | 159 | 8196 | 4 |
| recv-busy | 160 | 8200 | 3 |
| recv-busy | 161 | 8195 | 0 |
| recv-busy | 162 | 8197 | 3 |
| recv-busy | 163 | 8193 | 0 |
| recv-busy | 164 | 8200 | 3 |
| recv-busy | 165 | 8195 | 3 |
| recv-busy | 166 | 8198 | 3 |
| recv-busy | 167 | 8195 | 3 |
| recv-busy | 168 | 8199 | 3 |
| recv-busy | 169 | 8199 | 3 |
| recv-busy | 170 | 8198 | 4 |
| recv-busy | 171 | 8197 | 3 |
| recv-busy | 172 | 8194 | 3 |
| recv-busy | 173 | 8194 | 3 |
| recv-busy | 174 | 8200 | 3 |
| recv-busy | 175 | 8195 | 3 |
| recv-busy | 176 | 8197 | 3 |
| recv-busy | 177 | 8196 | 3 |
| recv-busy | 178 | 8193 | 3 |
| recv-busy | 179 | 8195 | 3 |
| recv-busy | 180 | 8196 | 4 |
| recv-busy | 181 | 8194 | 3 |
| recv-busy | 182 | 8195 | 3 |
| recv-busy | 183 | 8197 | 3 |
| recv-busy | 184 | 8198 | 3 |
| recv-busy | 185 | 8200 | 3 |
| recv-busy | 186 | 8197 | 3 |
| recv-busy | 187 | 8193 | 3 |
| recv-busy | 188 | 8194 | 3 |
| recv-busy | 189 | 8199 | 3 |
| recv-busy | 190 | 8193 | 0 |
| recv-busy | 191 | 8195 | 3 |
| recv-busy | 192 | 8199 | 3 |
| recv-busy | 193 | 8199 | 0 |
| recv-busy | 194 | 8197 | 3 |
| recv-busy | 195 | 8200 | 3 |
| recv-busy | 196 | 8198 | 4 |
| recv-busy | 197 | 8195 | 3 |
| recv-busy | 198 | 8199 | 3 |
| recv-busy | 199 | 8194 | 3 |
| recv-busy | 200 | 8194 | 3 |
| recv-busy | 201 | 8195 | 3 |
| recv-busy | 202 | 8194 | 3 |
| recv-busy | 203 | 8197 | 0 |
| recv-busy | 204 | 8199 | 3 |
| recv-busy | 205 | 8196 | 3 |
| recv-busy | 206 | 8194 | 4 |
| recv-busy | 207 | 8193 | 3 |
| recv-busy | 208 | 8193 | 3 |
| recv-busy | 209 | 8200 | 4 |
| recv-busy | 210 | 8197 | 3 |
| recv-busy | 211 | 8193 | 0 |
| recv-busy | 212 | 8197 | 3 |
| recv-busy | 213 | 8195 | 3 |
| recv-busy | 214 | 8196 | 4 |
| recv-busy | 215 | 8195 | 0 |
| recv-busy | 216 | 8197 | 3 |
| recv-busy | 217 | 8195 | 3 |
| recv-busy | 218 | 8195 | 3 |
| recv-busy | 219 | 8200 | 3 |
| recv-busy | 220 | 8193 | 0 |
| recv-busy | 221 | 8198 | 4 |
| recv-busy | 222 | 8193 | 3 |
| recv-busy | 223 | 8197 | 3 |
| recv-busy | 224 | 8197 | 3 |
| recv-busy | 225 | 8199 | 0 |
| recv-busy | 226 | 8198 | 3 |
| recv-busy | 227 | 8195 | 3 |
| recv-busy | 228 | 8200 | 3 |
| recv-busy | 229 | 8195 | 3 |
| recv-busy | 230 | 8199 | 3 |
| recv-busy | 231 | 8200 | 4 |
| recv-busy | 232 | 8200 | 3 |
| recv-busy | 233 | 8197 | 3 |
| recv-busy | 234 | 8199 | 3 |
| recv-busy | 235 | 8197 | 3 |
| recv-busy | 236 | 8196 | 3 |
| recv-busy | 237 | 8200 | 3 |
| recv-busy | 238 | 8193 | 3 |
| recv-busy | 239 | 8200 | 3 |
| recv-busy | 240 | 8196 | 4 |
| recv-busy | 241 | 8196 | 3 |
| recv-busy | 242 | 8198 | 4 |
| recv-busy | 243 | 8199 | 3 |
| recv-busy | 244 | 8200 | 3 |
| recv-busy | 245 | 8200 | 3 |
| recv-busy | 246 | 8193 | 3 |
| recv-busy | 247 | 8200 | 3 |
| recv-busy | 248 | 8200 | 3 |
| recv-busy | 249 | 8194 | 3 |
| recv-busy | 250 | 8198 | 3 |
| recv-busy | 251 | 8195 | 0 |
| recv-busy | 252 | 8196 | 3 |
| recv-busy | 253 | 8196 | 3 |
| recv-busy | 254 | 8195 | 3 |
| recv-busy | 255 | 8200 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374892288 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (3065720 of 5923975 in 1-msg reads) | 3065720 | 6.72 | 5.91 | 10.31 | 22.62 | 36.14 | 49.19 | 83.38 |

#### Stalls: m2/lab-recv-busy-pq-c256-s1024, loop 0 on CPU 3

10275456 iterations, longest 1389.0 µs; over 5 / 20 / 100 µs: 3706866 / 76041 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 68980 | 1931115.3 | 1389.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 5276 | 147203.1 | 53.0 |
| mostly unexplained (+preempted by kworker/3:1) | 1363 | 31401.1 | 56.6 |
| mostly unexplained (+softirq NET_RX) | 163 | 4277.2 | 52.8 |
| mostly unexplained (+irq work (IPI/vector)) | 159 | 3802.7 | 55.5 |
| mostly unexplained (+local timer tick) | 91 | 2725.1 | 51.7 |
| softirq NET_RX | 6 | 188.9 | 45.0 |
| mostly unexplained (+ipi_send_cpu) | 1 | 32.4 | 32.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 29.0 | 29.0 |
| mostly unexplained (+softirq NET_TX) | 1 | 23.9 | 23.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1389.0 | 3030 | loop work (3030 items) |  |
| 2 | 992.2 | 2557 | loop work (2557 items) |  |
| 3 | 902.7 | 2342 | loop work (2342 items) |  |
| 4 | 735.8 | 1935 | loop work (1935 items) |  |
| 5 | 164.1 | 172 | loop work (172 items) |  |
| 6 | 104.1 | 69 | loop work (69 items) |  |
| 7 | 99.7 | 70 | loop work (70 items) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 99.1 | 49 | loop work (49 items) |  |
| 9 | 98.6 | 74 | loop work (74 items) |  |
| 10 | 98.3 | 56 | loop work (56 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 7086 |
| LOC Local timer interrupts | 6732 |
| CAL Function call interrupts | 994 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c4-s1024: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3160626 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137874 | 3389770 | 1.81 | 111271040 | 18.13 | 1854517 | 370903 | 96.2 | 70.4 | true | 0 | 5843 | 3065.2 | 1221227 / 27600 / 19 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 4 |
| recv-busy | 1 | 8193 | 0 |
| recv-busy | 2 | 8195 | 3 |
| recv-busy | 3 | 8198 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374883776 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (2646761 of 5923944 in 1-msg reads) | 2646761 | 2.56 | 2.35 | 2.88 | 7.92 | 17.94 | 27.81 | 183.36 |

#### Stalls: m2/lab-recv-busy-pq-c4-s1024, loop 0 on CPU 3

22254208 iterations, longest 3065.2 µs; over 5 / 20 / 100 µs: 1221227 / 27600 / 19

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 18240 | 518008.5 | 3065.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 8712 | 211340.4 | 191.6 |
| mostly unexplained (+preempted by kworker/3:1) | 298 | 7441.9 | 47.6 |
| mostly unexplained (+softirq NET_RX) | 271 | 6618.9 | 45.3 |
| mostly unexplained (+local timer tick) | 42 | 1179.3 | 50.3 |
| mostly unexplained (+irq work (IPI/vector)) | 25 | 828.1 | 64.5 |
| softirq NET_RX | 10 | 246.6 | 33.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 29.3 | 29.3 |
| local timer tick | 1 | 29.1 | 29.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3065.2 | 9742 | loop work (9742 items) |  |
| 2 | 191.6 | 5 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 187.8 | 34 | loop work (34 items) |  |
| 4 | 177.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 164.3 | 87 | loop work (87 items) |  |
| 6 | 161.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 160.5 | 60 | loop work (60 items) |  |
| 8 | 157.1 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 149.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 149.2 | 9 | loop work (9 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 5864 |
| LOC Local timer interrupts | 5656 |
| CAL Function call interrupts | 992 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-recv-busy-pq-c64-s1024: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2975469 clocksource=tsc

### Loop counters (m2/lab-recv-busy-pq-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 6137883 | 3724021 | 1.65 | 101959917 | 16.61 | 1699332 | 182757 | 95.9 | 71.8 | true | 0 | 6522 | 3486.1 | 3255504 / 67818 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 3 |
| recv-busy | 1 | 8196 | 3 |
| recv-busy | 2 | 8193 | 3 |
| recv-busy | 3 | 8197 | 3 |
| recv-busy | 4 | 8199 | 3 |
| recv-busy | 5 | 8193 | 3 |
| recv-busy | 6 | 8198 | 3 |
| recv-busy | 7 | 8194 | 4 |
| recv-busy | 8 | 8195 | 3 |
| recv-busy | 9 | 8198 | 4 |
| recv-busy | 10 | 8199 | 3 |
| recv-busy | 11 | 8197 | 3 |
| recv-busy | 12 | 8193 | 3 |
| recv-busy | 13 | 8200 | 3 |
| recv-busy | 14 | 8198 | 3 |
| recv-busy | 15 | 8200 | 4 |
| recv-busy | 16 | 8196 | 3 |
| recv-busy | 17 | 8198 | 4 |
| recv-busy | 18 | 8193 | 3 |
| recv-busy | 19 | 8200 | 4 |
| recv-busy | 20 | 8194 | 4 |
| recv-busy | 21 | 8195 | 3 |
| recv-busy | 22 | 8196 | 3 |
| recv-busy | 23 | 8196 | 3 |
| recv-busy | 24 | 8193 | 0 |
| recv-busy | 25 | 8193 | 3 |
| recv-busy | 26 | 8196 | 3 |
| recv-busy | 27 | 8195 | 3 |
| recv-busy | 28 | 8197 | 3 |
| recv-busy | 29 | 8198 | 3 |
| recv-busy | 30 | 8197 | 0 |
| recv-busy | 31 | 8194 | 3 |
| recv-busy | 32 | 8194 | 3 |
| recv-busy | 33 | 8197 | 0 |
| recv-busy | 34 | 8193 | 3 |
| recv-busy | 35 | 8196 | 3 |
| recv-busy | 36 | 8193 | 3 |
| recv-busy | 37 | 8200 | 4 |
| recv-busy | 38 | 8193 | 3 |
| recv-busy | 39 | 8200 | 3 |
| recv-busy | 40 | 8194 | 4 |
| recv-busy | 41 | 8200 | 3 |
| recv-busy | 42 | 8199 | 0 |
| recv-busy | 43 | 8194 | 3 |
| recv-busy | 44 | 8197 | 3 |
| recv-busy | 45 | 8195 | 0 |
| recv-busy | 46 | 8197 | 0 |
| recv-busy | 47 | 8196 | 3 |
| recv-busy | 48 | 8198 | 4 |
| recv-busy | 49 | 8195 | 3 |
| recv-busy | 50 | 8194 | 4 |
| recv-busy | 51 | 8198 | 4 |
| recv-busy | 52 | 8200 | 3 |
| recv-busy | 53 | 8198 | 3 |
| recv-busy | 54 | 8195 | 3 |
| recv-busy | 55 | 8197 | 3 |
| recv-busy | 56 | 8199 | 0 |
| recv-busy | 57 | 8194 | 3 |
| recv-busy | 58 | 8193 | 3 |
| recv-busy | 59 | 8196 | 3 |
| recv-busy | 60 | 8197 | 3 |
| recv-busy | 61 | 8200 | 4 |
| recv-busy | 62 | 8200 | 3 |
| recv-busy | 63 | 8196 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1374885792 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (3060685 of 5924039 in 1-msg reads) | 3060685 | 6.07 | 5.48 | 9.34 | 20.94 | 35.01 | 48.62 | 69.00 |

#### Stalls: m2/lab-recv-busy-pq-c64-s1024, loop 0 on CPU 3

10965440 iterations, longest 3486.1 µs; over 5 / 20 / 100 µs: 3255504 / 67818 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 60901 | 1696211.2 | 3486.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 5744 | 156664.7 | 55.0 |
| mostly unexplained (+preempted by kworker/3:1) | 817 | 18733.6 | 52.9 |
| mostly unexplained (+softirq NET_RX) | 144 | 3851.6 | 47.9 |
| mostly unexplained (+irq work (IPI/vector)) | 151 | 3485.4 | 43.8 |
| mostly unexplained (+local timer tick) | 55 | 1537.3 | 49.0 |
| softirq NET_RX | 4 | 129.9 | 41.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.3 | 34.3 |
| mostly unexplained (+ipi_send_cpu) | 1 | 30.3 | 30.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3486.1 | 9734 | loop work (9734 items) |  |
| 2 | 132.0 | 182 | loop work (182 items) |  |
| 3 | 102.7 | 48 | loop work (48 items) | local timer tick 3, timer tick_nohz_handler 1 |
| 4 | 101.6 | 70 | loop work (70 items) |  |
| 5 | 98.3 | 83 | loop work (83 items) |  |
| 6 | 95.1 | 64 | loop work (64 items) |  |
| 7 | 91.9 | 55 | loop work (55 items) | local timer tick 3, preempted by kworker/3:1 2, irq work (IPI/vector) 1, softirq NET_RX 1 |
| 8 | 90.5 | 53 | loop work (53 items) |  |
| 9 | 89.1 | 64 | loop work (64 items) |  |
| 10 | 88.6 | 45 | loop work (45 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 6562 |
| LOC Local timer interrupts | 6289 |
| CAL Function call interrupts | 991 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c1-s1024: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2450577 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c1-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137906 | 2876494 | 2.13 | 1 | 0.00 | 0 | 36516161 | 0.0 | 0.0 | true | 0 | 0 | 1353.0 | 7877 / 3 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8195 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1374890944 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (1941244 of 5923910 in 1-msg reads) | 1941244 | 3.22 | 3.19 | 3.58 | 6.30 | 11.14 | 14.88 | 35.55 |

#### Stalls: m2/lab-uring-sqpoll-c1-s1024, loop 0 on CPU 3

2190969664 iterations, longest 1353.0 µs; over 5 / 20 / 100 µs: 7877 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1382.6 | 1353.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.9 | 28.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1353.0 | 9686 | loop work (9686 items) |  |
| 2 | 29.6 | 208 | loop work (208 items) |  |
| 3 | 28.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| LOC Local timer interrupts | 4 |
| IWI IRQ work interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c16-s1024: lab, 60 s (first 2 s dropped), 16 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3387986 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c16-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137907 | 3744042 | 1.64 | 1 | 0.00 | 0 | 26807829 | 0.0 | 0.0 | true | 0 | 0 | 1367.3 | 2596 / 3 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8195 | 2 |
| uring-sqpoll | 1 | 8194 | 2 |
| uring-sqpoll | 2 | 8199 | 2 |
| uring-sqpoll | 3 | 8197 | 2 |
| uring-sqpoll | 4 | 8197 | 2 |
| uring-sqpoll | 5 | 8199 | 2 |
| uring-sqpoll | 6 | 8193 | 2 |
| uring-sqpoll | 7 | 8196 | 2 |
| uring-sqpoll | 8 | 8194 | 2 |
| uring-sqpoll | 9 | 8199 | 2 |
| uring-sqpoll | 10 | 8193 | 2 |
| uring-sqpoll | 11 | 8194 | 2 |
| uring-sqpoll | 12 | 8200 | 2 |
| uring-sqpoll | 13 | 8195 | 2 |
| uring-sqpoll | 14 | 8193 | 2 |
| uring-sqpoll | 15 | 8196 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1374891168 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (3056301 of 5923756 in 1-msg reads) | 3056301 | 4.00 | 3.47 | 5.78 | 12.05 | 24.56 | 36.70 | 66.40 |

#### Stalls: m2/lab-uring-sqpoll-c16-s1024, loop 0 on CPU 3

1608469760 iterations, longest 1367.3 µs; over 5 / 20 / 100 µs: 2596 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1391.2 | 1367.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.5 | 23.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1367.3 | 9764 | loop work (9764 items) |  |
| 2 | 23.9 | 148 | loop work (148 items) |  |
| 3 | 23.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c256-s1024: lab, 60 s (first 2 s dropped), 256 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3414314 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c256-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137937 | 3816206 | 1.61 | 1 | 0.00 | 0 | 3038640 | 0.0 | 0.0 | true | 0 | 0 | 410.0 | 7678 / 8 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8196 | 2 |
| uring-sqpoll | 1 | 8196 | 2 |
| uring-sqpoll | 2 | 8194 | 2 |
| uring-sqpoll | 3 | 8196 | 2 |
| uring-sqpoll | 4 | 8197 | 2 |
| uring-sqpoll | 5 | 8197 | 2 |
| uring-sqpoll | 6 | 8196 | 2 |
| uring-sqpoll | 7 | 8193 | 2 |
| uring-sqpoll | 8 | 8199 | 2 |
| uring-sqpoll | 9 | 8194 | 2 |
| uring-sqpoll | 10 | 8194 | 2 |
| uring-sqpoll | 11 | 8200 | 2 |
| uring-sqpoll | 12 | 8196 | 2 |
| uring-sqpoll | 13 | 8194 | 2 |
| uring-sqpoll | 14 | 8194 | 2 |
| uring-sqpoll | 15 | 8197 | 2 |
| uring-sqpoll | 16 | 8199 | 2 |
| uring-sqpoll | 17 | 8194 | 2 |
| uring-sqpoll | 18 | 8198 | 2 |
| uring-sqpoll | 19 | 8193 | 2 |
| uring-sqpoll | 20 | 8195 | 2 |
| uring-sqpoll | 21 | 8198 | 2 |
| uring-sqpoll | 22 | 8197 | 2 |
| uring-sqpoll | 23 | 8199 | 2 |
| uring-sqpoll | 24 | 8196 | 2 |
| uring-sqpoll | 25 | 8199 | 2 |
| uring-sqpoll | 26 | 8200 | 2 |
| uring-sqpoll | 27 | 8196 | 2 |
| uring-sqpoll | 28 | 8194 | 2 |
| uring-sqpoll | 29 | 8196 | 2 |
| uring-sqpoll | 30 | 8196 | 2 |
| uring-sqpoll | 31 | 8197 | 2 |
| uring-sqpoll | 32 | 8198 | 2 |
| uring-sqpoll | 33 | 8194 | 2 |
| uring-sqpoll | 34 | 8197 | 2 |
| uring-sqpoll | 35 | 8198 | 2 |
| uring-sqpoll | 36 | 8196 | 2 |
| uring-sqpoll | 37 | 8198 | 2 |
| uring-sqpoll | 38 | 8199 | 2 |
| uring-sqpoll | 39 | 8193 | 2 |
| uring-sqpoll | 40 | 8197 | 2 |
| uring-sqpoll | 41 | 8197 | 2 |
| uring-sqpoll | 42 | 8196 | 2 |
| uring-sqpoll | 43 | 8195 | 2 |
| uring-sqpoll | 44 | 8197 | 2 |
| uring-sqpoll | 45 | 8199 | 2 |
| uring-sqpoll | 46 | 8194 | 2 |
| uring-sqpoll | 47 | 8195 | 2 |
| uring-sqpoll | 48 | 8198 | 4 |
| uring-sqpoll | 49 | 8197 | 2 |
| uring-sqpoll | 50 | 8199 | 2 |
| uring-sqpoll | 51 | 8196 | 2 |
| uring-sqpoll | 52 | 8195 | 2 |
| uring-sqpoll | 53 | 8198 | 2 |
| uring-sqpoll | 54 | 8200 | 2 |
| uring-sqpoll | 55 | 8196 | 4 |
| uring-sqpoll | 56 | 8197 | 2 |
| uring-sqpoll | 57 | 8195 | 2 |
| uring-sqpoll | 58 | 8198 | 4 |
| uring-sqpoll | 59 | 8200 | 2 |
| uring-sqpoll | 60 | 8194 | 2 |
| uring-sqpoll | 61 | 8197 | 2 |
| uring-sqpoll | 62 | 8195 | 2 |
| uring-sqpoll | 63 | 8196 | 2 |
| uring-sqpoll | 64 | 8199 | 2 |
| uring-sqpoll | 65 | 8195 | 2 |
| uring-sqpoll | 66 | 8198 | 2 |
| uring-sqpoll | 67 | 8194 | 2 |
| uring-sqpoll | 68 | 8198 | 2 |
| uring-sqpoll | 69 | 8196 | 4 |
| uring-sqpoll | 70 | 8194 | 2 |
| uring-sqpoll | 71 | 8195 | 2 |
| uring-sqpoll | 72 | 8196 | 2 |
| uring-sqpoll | 73 | 8197 | 2 |
| uring-sqpoll | 74 | 8193 | 2 |
| uring-sqpoll | 75 | 8196 | 2 |
| uring-sqpoll | 76 | 8196 | 2 |
| uring-sqpoll | 77 | 8194 | 2 |
| uring-sqpoll | 78 | 8196 | 2 |
| uring-sqpoll | 79 | 8195 | 2 |
| uring-sqpoll | 80 | 8195 | 2 |
| uring-sqpoll | 81 | 8199 | 2 |
| uring-sqpoll | 82 | 8195 | 2 |
| uring-sqpoll | 83 | 8194 | 2 |
| uring-sqpoll | 84 | 8193 | 2 |
| uring-sqpoll | 85 | 8196 | 2 |
| uring-sqpoll | 86 | 8198 | 2 |
| uring-sqpoll | 87 | 8193 | 2 |
| uring-sqpoll | 88 | 8193 | 2 |
| uring-sqpoll | 89 | 8198 | 2 |
| uring-sqpoll | 90 | 8200 | 2 |
| uring-sqpoll | 91 | 8195 | 2 |
| uring-sqpoll | 92 | 8195 | 2 |
| uring-sqpoll | 93 | 8198 | 2 |
| uring-sqpoll | 94 | 8197 | 2 |
| uring-sqpoll | 95 | 8193 | 2 |
| uring-sqpoll | 96 | 8193 | 2 |
| uring-sqpoll | 97 | 8194 | 2 |
| uring-sqpoll | 98 | 8199 | 2 |
| uring-sqpoll | 99 | 8194 | 2 |
| uring-sqpoll | 100 | 8197 | 2 |
| uring-sqpoll | 101 | 8198 | 2 |
| uring-sqpoll | 102 | 8199 | 2 |
| uring-sqpoll | 103 | 8195 | 2 |
| uring-sqpoll | 104 | 8198 | 2 |
| uring-sqpoll | 105 | 8197 | 2 |
| uring-sqpoll | 106 | 8198 | 2 |
| uring-sqpoll | 107 | 8198 | 2 |
| uring-sqpoll | 108 | 8200 | 2 |
| uring-sqpoll | 109 | 8199 | 2 |
| uring-sqpoll | 110 | 8194 | 2 |
| uring-sqpoll | 111 | 8199 | 2 |
| uring-sqpoll | 112 | 8200 | 2 |
| uring-sqpoll | 113 | 8200 | 2 |
| uring-sqpoll | 114 | 8193 | 2 |
| uring-sqpoll | 115 | 8196 | 2 |
| uring-sqpoll | 116 | 8194 | 2 |
| uring-sqpoll | 117 | 8200 | 2 |
| uring-sqpoll | 118 | 8199 | 0 |
| uring-sqpoll | 119 | 8199 | 2 |
| uring-sqpoll | 120 | 8200 | 2 |
| uring-sqpoll | 121 | 8194 | 2 |
| uring-sqpoll | 122 | 8196 | 2 |
| uring-sqpoll | 123 | 8197 | 2 |
| uring-sqpoll | 124 | 8199 | 2 |
| uring-sqpoll | 125 | 8194 | 2 |
| uring-sqpoll | 126 | 8198 | 2 |
| uring-sqpoll | 127 | 8199 | 2 |
| uring-sqpoll | 128 | 8199 | 2 |
| uring-sqpoll | 129 | 8197 | 2 |
| uring-sqpoll | 130 | 8196 | 2 |
| uring-sqpoll | 131 | 8193 | 2 |
| uring-sqpoll | 132 | 8194 | 2 |
| uring-sqpoll | 133 | 8196 | 2 |
| uring-sqpoll | 134 | 8200 | 2 |
| uring-sqpoll | 135 | 8196 | 2 |
| uring-sqpoll | 136 | 8200 | 2 |
| uring-sqpoll | 137 | 8193 | 2 |
| uring-sqpoll | 138 | 8196 | 2 |
| uring-sqpoll | 139 | 8198 | 2 |
| uring-sqpoll | 140 | 8197 | 2 |
| uring-sqpoll | 141 | 8195 | 2 |
| uring-sqpoll | 142 | 8199 | 2 |
| uring-sqpoll | 143 | 8198 | 2 |
| uring-sqpoll | 144 | 8197 | 2 |
| uring-sqpoll | 145 | 8193 | 2 |
| uring-sqpoll | 146 | 8200 | 2 |
| uring-sqpoll | 147 | 8193 | 2 |
| uring-sqpoll | 148 | 8193 | 2 |
| uring-sqpoll | 149 | 8197 | 2 |
| uring-sqpoll | 150 | 8200 | 2 |
| uring-sqpoll | 151 | 8195 | 2 |
| uring-sqpoll | 152 | 8198 | 2 |
| uring-sqpoll | 153 | 8198 | 2 |
| uring-sqpoll | 154 | 8199 | 2 |
| uring-sqpoll | 155 | 8194 | 2 |
| uring-sqpoll | 156 | 8198 | 4 |
| uring-sqpoll | 157 | 8195 | 2 |
| uring-sqpoll | 158 | 8195 | 2 |
| uring-sqpoll | 159 | 8200 | 2 |
| uring-sqpoll | 160 | 8196 | 2 |
| uring-sqpoll | 161 | 8198 | 4 |
| uring-sqpoll | 162 | 8196 | 2 |
| uring-sqpoll | 163 | 8193 | 2 |
| uring-sqpoll | 164 | 8193 | 2 |
| uring-sqpoll | 165 | 8195 | 2 |
| uring-sqpoll | 166 | 8198 | 2 |
| uring-sqpoll | 167 | 8198 | 2 |
| uring-sqpoll | 168 | 8200 | 2 |
| uring-sqpoll | 169 | 8199 | 2 |
| uring-sqpoll | 170 | 8198 | 4 |
| uring-sqpoll | 171 | 8198 | 2 |
| uring-sqpoll | 172 | 8196 | 2 |
| uring-sqpoll | 173 | 8196 | 2 |
| uring-sqpoll | 174 | 8200 | 2 |
| uring-sqpoll | 175 | 8197 | 2 |
| uring-sqpoll | 176 | 8200 | 2 |
| uring-sqpoll | 177 | 8200 | 2 |
| uring-sqpoll | 178 | 8198 | 2 |
| uring-sqpoll | 179 | 8197 | 2 |
| uring-sqpoll | 180 | 8199 | 2 |
| uring-sqpoll | 181 | 8194 | 2 |
| uring-sqpoll | 182 | 8196 | 2 |
| uring-sqpoll | 183 | 8195 | 2 |
| uring-sqpoll | 184 | 8193 | 2 |
| uring-sqpoll | 185 | 8200 | 2 |
| uring-sqpoll | 186 | 8197 | 2 |
| uring-sqpoll | 187 | 8193 | 2 |
| uring-sqpoll | 188 | 8199 | 2 |
| uring-sqpoll | 189 | 8197 | 2 |
| uring-sqpoll | 190 | 8200 | 2 |
| uring-sqpoll | 191 | 8195 | 2 |
| uring-sqpoll | 192 | 8194 | 2 |
| uring-sqpoll | 193 | 8194 | 2 |
| uring-sqpoll | 194 | 8199 | 2 |
| uring-sqpoll | 195 | 8200 | 2 |
| uring-sqpoll | 196 | 8196 | 2 |
| uring-sqpoll | 197 | 8198 | 2 |
| uring-sqpoll | 198 | 8194 | 2 |
| uring-sqpoll | 199 | 8195 | 2 |
| uring-sqpoll | 200 | 8193 | 2 |
| uring-sqpoll | 201 | 8197 | 2 |
| uring-sqpoll | 202 | 8193 | 2 |
| uring-sqpoll | 203 | 8200 | 2 |
| uring-sqpoll | 204 | 8195 | 2 |
| uring-sqpoll | 205 | 8194 | 2 |
| uring-sqpoll | 206 | 8196 | 4 |
| uring-sqpoll | 207 | 8195 | 2 |
| uring-sqpoll | 208 | 8193 | 2 |
| uring-sqpoll | 209 | 8198 | 2 |
| uring-sqpoll | 210 | 8197 | 2 |
| uring-sqpoll | 211 | 8195 | 2 |
| uring-sqpoll | 212 | 8195 | 2 |
| uring-sqpoll | 213 | 8193 | 2 |
| uring-sqpoll | 214 | 8196 | 2 |
| uring-sqpoll | 215 | 8199 | 2 |
| uring-sqpoll | 216 | 8197 | 2 |
| uring-sqpoll | 217 | 8196 | 2 |
| uring-sqpoll | 218 | 8199 | 2 |
| uring-sqpoll | 219 | 8194 | 2 |
| uring-sqpoll | 220 | 8194 | 2 |
| uring-sqpoll | 221 | 8197 | 2 |
| uring-sqpoll | 222 | 8197 | 2 |
| uring-sqpoll | 223 | 8198 | 2 |
| uring-sqpoll | 224 | 8200 | 2 |
| uring-sqpoll | 225 | 8198 | 2 |
| uring-sqpoll | 226 | 8198 | 2 |
| uring-sqpoll | 227 | 8195 | 2 |
| uring-sqpoll | 228 | 8195 | 2 |
| uring-sqpoll | 229 | 8198 | 2 |
| uring-sqpoll | 230 | 8196 | 2 |
| uring-sqpoll | 231 | 8196 | 2 |
| uring-sqpoll | 232 | 8193 | 2 |
| uring-sqpoll | 233 | 8196 | 2 |
| uring-sqpoll | 234 | 8199 | 2 |
| uring-sqpoll | 235 | 8194 | 2 |
| uring-sqpoll | 236 | 8200 | 2 |
| uring-sqpoll | 237 | 8197 | 2 |
| uring-sqpoll | 238 | 8198 | 2 |
| uring-sqpoll | 239 | 8194 | 2 |
| uring-sqpoll | 240 | 8193 | 2 |
| uring-sqpoll | 241 | 8193 | 2 |
| uring-sqpoll | 242 | 8200 | 2 |
| uring-sqpoll | 243 | 8197 | 2 |
| uring-sqpoll | 244 | 8193 | 2 |
| uring-sqpoll | 245 | 8194 | 2 |
| uring-sqpoll | 246 | 8198 | 2 |
| uring-sqpoll | 247 | 8197 | 2 |
| uring-sqpoll | 248 | 8196 | 2 |
| uring-sqpoll | 249 | 8196 | 2 |
| uring-sqpoll | 250 | 8195 | 2 |
| uring-sqpoll | 251 | 8199 | 2 |
| uring-sqpoll | 252 | 8194 | 2 |
| uring-sqpoll | 253 | 8197 | 0 |
| uring-sqpoll | 254 | 8196 | 2 |
| uring-sqpoll | 255 | 8195 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1374897888 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (3121642 of 5923745 in 1-msg reads) | 3121642 | 4.63 | 3.74 | 6.83 | 19.08 | 34.72 | 48.38 | 72.55 |

#### Stalls: m2/lab-uring-sqpoll-c256-s1024, loop 0 on CPU 3

182318400 iterations, longest 410.0 µs; over 5 / 20 / 100 µs: 7678 / 8 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 6 | 1609.6 | 410.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 40.4 | 40.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.8 | 26.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 410.0 | 2726 | loop work (2726 items) |  |
| 2 | 391.8 | 2499 | loop work (2499 items) |  |
| 3 | 367.0 | 2345 | loop work (2345 items) |  |
| 4 | 363.0 | 2293 | loop work (2293 items) |  |
| 5 | 40.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 40.0 | 109 | loop work (109 items) |  |
| 7 | 37.9 | 130 | loop work (130 items) |  |
| 8 | 26.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 8 |
| LOC Local timer interrupts | 6 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c4-s1024: lab, 60 s (first 2 s dropped), 4 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3148314 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c4-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137882 | 3453612 | 1.78 | 1 | 0.00 | 0 | 37018182 | 0.0 | 0.0 | true | 0 | 0 | 1363.6 | 1227 / 4 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8193 | 2 |
| uring-sqpoll | 1 | 8200 | 2 |
| uring-sqpoll | 2 | 8197 | 2 |
| uring-sqpoll | 3 | 8194 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1374885568 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (2712064 of 5923770 in 1-msg reads) | 2712064 | 3.56 | 3.25 | 4.24 | 8.53 | 17.54 | 28.87 | 83.55 |

#### Stalls: m2/lab-uring-sqpoll-c4-s1024, loop 0 on CPU 3

2221090944 iterations, longest 1363.6 µs; over 5 / 20 / 100 µs: 1227 / 4 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 1363.6 | 1363.6 |
| softirq TIMER | 1 | 169.7 | 169.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 70.2 | 70.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 30.3 | 30.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1363.6 | 9704 | loop work (9704 items) |  |
| 2 | 169.7 | 3 | softirq TIMER | softirq TIMER 150, local timer tick 11, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 3 | 70.2 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 30.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## m2/lab-uring-sqpoll-c64-s1024: lab, 60 s (first 2 s dropped), 64 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3430493 clocksource=tsc

### Loop counters (m2/lab-uring-sqpoll-c64-s1024)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 6137765 | 3806859 | 1.61 | 1 | 0.00 | 0 | 10980770 | 0.0 | 0.0 | true | 0 | 0 | 1376.5 | 4011 / 3 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8193 | 2 |
| uring-sqpoll | 1 | 8195 | 2 |
| uring-sqpoll | 2 | 8197 | 2 |
| uring-sqpoll | 3 | 8197 | 2 |
| uring-sqpoll | 4 | 8200 | 2 |
| uring-sqpoll | 5 | 8195 | 2 |
| uring-sqpoll | 6 | 8198 | 2 |
| uring-sqpoll | 7 | 8198 | 2 |
| uring-sqpoll | 8 | 8198 | 2 |
| uring-sqpoll | 9 | 8197 | 2 |
| uring-sqpoll | 10 | 8197 | 2 |
| uring-sqpoll | 11 | 8198 | 2 |
| uring-sqpoll | 12 | 8194 | 2 |
| uring-sqpoll | 13 | 8196 | 2 |
| uring-sqpoll | 14 | 8200 | 2 |
| uring-sqpoll | 15 | 8195 | 2 |
| uring-sqpoll | 16 | 8199 | 2 |
| uring-sqpoll | 17 | 8196 | 2 |
| uring-sqpoll | 18 | 8195 | 2 |
| uring-sqpoll | 19 | 8197 | 2 |
| uring-sqpoll | 20 | 8194 | 2 |
| uring-sqpoll | 21 | 8197 | 2 |
| uring-sqpoll | 22 | 8195 | 2 |
| uring-sqpoll | 23 | 8193 | 2 |
| uring-sqpoll | 24 | 8194 | 4 |
| uring-sqpoll | 25 | 8200 | 2 |
| uring-sqpoll | 26 | 8198 | 2 |
| uring-sqpoll | 27 | 8196 | 2 |
| uring-sqpoll | 28 | 8197 | 2 |
| uring-sqpoll | 29 | 8197 | 2 |
| uring-sqpoll | 30 | 8197 | 0 |
| uring-sqpoll | 31 | 8193 | 2 |
| uring-sqpoll | 32 | 8196 | 2 |
| uring-sqpoll | 33 | 8198 | 2 |
| uring-sqpoll | 34 | 8195 | 2 |
| uring-sqpoll | 35 | 8200 | 2 |
| uring-sqpoll | 36 | 8194 | 2 |
| uring-sqpoll | 37 | 8196 | 2 |
| uring-sqpoll | 38 | 8200 | 2 |
| uring-sqpoll | 39 | 8197 | 2 |
| uring-sqpoll | 40 | 8193 | 2 |
| uring-sqpoll | 41 | 8199 | 2 |
| uring-sqpoll | 42 | 8194 | 2 |
| uring-sqpoll | 43 | 8198 | 2 |
| uring-sqpoll | 44 | 8198 | 2 |
| uring-sqpoll | 45 | 8195 | 2 |
| uring-sqpoll | 46 | 8194 | 2 |
| uring-sqpoll | 47 | 8194 | 4 |
| uring-sqpoll | 48 | 8193 | 2 |
| uring-sqpoll | 49 | 8195 | 2 |
| uring-sqpoll | 50 | 8198 | 2 |
| uring-sqpoll | 51 | 8200 | 2 |
| uring-sqpoll | 52 | 8197 | 2 |
| uring-sqpoll | 53 | 8194 | 2 |
| uring-sqpoll | 54 | 8194 | 2 |
| uring-sqpoll | 55 | 8197 | 2 |
| uring-sqpoll | 56 | 8200 | 2 |
| uring-sqpoll | 57 | 8194 | 4 |
| uring-sqpoll | 58 | 8196 | 2 |
| uring-sqpoll | 59 | 8196 | 2 |
| uring-sqpoll | 60 | 8196 | 2 |
| uring-sqpoll | 61 | 8198 | 2 |
| uring-sqpoll | 62 | 8193 | 2 |
| uring-sqpoll | 63 | 8197 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1374859360 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (3115050 of 5923711 in 1-msg reads) | 3115050 | 4.35 | 3.62 | 6.38 | 14.71 | 28.48 | 41.13 | 73.20 |

#### Stalls: m2/lab-uring-sqpoll-c64-s1024, loop 0 on CPU 3

658846208 iterations, longest 1376.5 µs; over 5 / 20 / 100 µs: 4011 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 1414.6 | 1376.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.4 | 28.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1376.5 | 9707 | loop work (9707 items) |  |
| 2 | 38.1 | 175 | loop work (175 items) |  |
| 3 | 28.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 4 |
| TLB TLB shootdowns | 1 |

