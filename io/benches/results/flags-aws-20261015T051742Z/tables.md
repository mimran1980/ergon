# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|

## HTTP round trip, http-api-native-recv-busy

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1994 | 45.49 | 65.83 | 80.92 | 81.29 | 81.29 | 0.0 | 0.0 | 0.0 | 0.0 | 39.89 | 360.0 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-api-native-recv-busy, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1994 | 45.68 | 66.00 | 81.12 | 81.43 | 81.43 | 0.0 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-api-native-recv-busy-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1976874 | 42.98 | 50.76 | 59.34 | 72.24 | 349.71 | 48.8 | 0.0 | 0.0 | 0.0 | 343.75 | 87.5 | client: softirq TIMER |
| h2 | 1976787 | 42.75 | 50.32 | 59.19 | 89.26 | 1028.24 | 89.6 | 1.4 | 1.4 | 0.0 | 34.53 | 89.0 | neither loop stalled long enough: network, hypervisor or untraced |

## HTTP round trip, http-api-native-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189935 | 46.48 | 58.03 | 73.66 | 106.89 | 321.39 | 76.6 | 0.0 | 0.0 | 0.0 | 63.67 | 2654.0 | neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1189935 | 46.86 | 56.02 | 65.59 | 97.45 | 239.03 | 52.9 | 0.0 | 0.0 | 0.0 | 324.81 | 143.0 | server: softirq TIMER |

### HTTP round trip, http-api-native-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189935 | 46.66 | 58.26 | 74.09 | 119.71 | 450.56 | 114.0 | 0.0 | 0.0 | 0.0 |
| h2 | 1189935 | 47.01 | 56.19 | 65.98 | 99.46 | 340.23 | 58.0 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-curl-distro

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1992 | 89.16 | 138.96 | 186.54 | 200.14 | 200.14 | 4128.6 | 0.0 | 0.0 | 0.0 | 1417.67 | 140.0 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-curl-distro, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1992 | 89.39 | 139.21 | 186.80 | 200.34 | 200.34 | 4188.9 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-curl-distro-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1983571 | 58.12 | 75.10 | 96.44 | 108.48 | 209.20 | 467.8 | 0.0 | 0.0 | 0.0 | 1478.60 | 103.5 | neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1905900 | 61.89 | 75.40 | 98.20 | 129.94 | 296.39 | 696.9 | 0.0 | 0.0 | 0.0 | 1481.95 | 101.0 | neither loop stalled long enough: network, hypervisor or untraced |

## HTTP round trip, http-curl-distro-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189908 | 58.51 | 76.80 | 106.99 | 116.06 | 239.90 | 3973.4 | 0.0 | 0.0 | 0.0 | 1485.70 | 32.0 | server: softirq TIMER |
| h2 | 1189911 | 61.47 | 101.11 | 106.43 | 163.28 | 254.35 | 8604.9 | 0.0 | 0.0 | 0.0 | 1428.08 | 22.0 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-curl-distro-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189908 | 58.71 | 77.22 | 108.45 | 117.09 | 267.21 | 4154.4 | 0.0 | 0.0 | 0.0 |
| h2 | 1189911 | 61.72 | 102.89 | 122.04 | 172.43 | 254.55 | 17838.5 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-curl-source

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1992 | 58.42 | 85.80 | 99.47 | 101.63 | 101.63 | 30.1 | 0.0 | 0.0 | 0.0 | 839.24 | 100.0 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-curl-source, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1992 | 58.73 | 86.06 | 99.73 | 102.19 | 102.19 | 30.1 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-curl-source-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1982338 | 55.17 | 67.68 | 95.36 | 101.92 | 152.54 | 156.2 | 0.0 | 0.0 | 0.0 | 871.21 | 41.5 | neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1981759 | 54.31 | 68.61 | 96.12 | 102.54 | 269.40 | 203.8 | 0.0 | 0.0 | 0.0 | 865.58 | 85.5 | neither loop stalled long enough: network, hypervisor or untraced |

## HTTP round trip, http-curl-source-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189917 | 55.99 | 100.78 | 109.72 | 121.04 | 262.11 | 7608.9 | 0.0 | 0.0 | 0.0 | 846.24 | 41.5 | neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1189914 | 56.53 | 73.83 | 101.67 | 111.19 | 345.45 | 1490.5 | 0.0 | 0.0 | 0.0 | 863.80 | 53.5 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-curl-source-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189917 | 56.26 | 101.16 | 110.20 | 124.74 | 262.36 | 8464.1 | 0.0 | 0.0 | 0.0 |
| h2 | 1189914 | 56.79 | 74.26 | 102.03 | 113.25 | 345.71 | 1741.1 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-ergon-epoll-spin

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1994 | 48.59 | 97.11 | 100.84 | 104.73 | 104.73 | 60.2 | 0.0 | 0.0 | 0.0 | 22.48 | 20.0 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-ergon-epoll-spin, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1994 | 48.79 | 97.42 | 101.01 | 105.03 | 105.03 | 120.4 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-ergon-epoll-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1980506 | 48.96 | 96.85 | 103.82 | 109.81 | 226.33 | 1794.6 | 0.0 | 0.0 | 0.0 | 179.88 | 11.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| h2 | 1980380 | 48.92 | 96.53 | 99.87 | 108.40 | 193.86 | 1155.0 | 0.0 | 0.0 | 0.0 | 115.99 | 13.0 | neither loop stalled long enough: network, hypervisor or untraced |

## HTTP round trip, http-ergon-epoll-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189937 | 48.64 | 90.37 | 95.48 | 102.28 | 340.29 | 85.7 | 0.0 | 0.0 | 0.0 | 287.94 | 14.5 | client: softirq TIMER |
| h2 | 1189936 | 49.37 | 96.22 | 106.80 | 116.36 | 1013.67 | 2616.4 | 0.5 | 0.5 | 0.0 | 32.17 | 10.5 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-ergon-epoll-spin-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189937 | 48.80 | 90.53 | 95.67 | 104.16 | 379.30 | 99.3 | 0.0 | 0.0 | 0.0 |
| h2 | 1189936 | 49.52 | 96.44 | 106.98 | 120.80 | 1013.81 | 2725.9 | 5.0 | 0.5 | 0.0 |

## HTTP round trip, http-ergon-recv-busy

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1994 | 45.32 | 66.31 | 74.44 | 74.63 | 74.63 | 0.0 | 0.0 | 0.0 | 0.0 | 31.20 | 340.0 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-ergon-recv-busy, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1994 | 45.55 | 66.49 | 74.59 | 74.81 | 74.81 | 0.0 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-ergon-recv-busy-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1976339 | 42.04 | 50.30 | 58.55 | 71.31 | 287.81 | 21.4 | 0.0 | 0.0 | 0.0 | 111.17 | 219.0 | neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1976472 | 42.21 | 50.21 | 58.39 | 70.22 | 153.91 | 12.0 | 0.0 | 0.0 | 0.0 | 48.15 | 64.0 | neither loop stalled long enough: network, hypervisor or untraced |

## HTTP round trip, http-ergon-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189937 | 44.85 | 53.64 | 66.93 | 91.62 | 454.43 | 38.8 | 0.0 | 0.0 | 0.0 | 36.28 | 187.0 | neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1189936 | 44.56 | 53.06 | 62.82 | 95.72 | 226.04 | 52.9 | 0.0 | 0.0 | 0.0 | 40.39 | 189.5 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-ergon-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189937 | 44.99 | 53.81 | 67.17 | 107.62 | 1057.29 | 76.1 | 32.3 | 6.6 | 0.0 |
| h2 | 1189936 | 44.73 | 53.26 | 63.44 | 111.31 | 293.18 | 85.2 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-ergon-uring-spin

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1994 | 52.35 | 77.03 | 94.22 | 95.45 | 95.45 | 0.0 | 0.0 | 0.0 | 0.0 | 19.16 | 0.0 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-ergon-uring-spin, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1994 | 52.53 | 77.19 | 94.40 | 95.60 | 95.60 | 0.0 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-ergon-uring-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1980244 | 48.23 | 95.78 | 98.34 | 106.47 | 239.06 | 774.8 | 0.0 | 0.0 | 0.0 | 204.58 | 10.5 | neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1980769 | 48.42 | 96.14 | 99.03 | 109.78 | 237.88 | 984.8 | 0.0 | 0.0 | 0.0 | 36.66 | 11.0 | neither loop stalled long enough: network, hypervisor or untraced |

## HTTP round trip, http-ergon-uring-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189936 | 50.34 | 96.97 | 102.10 | 152.19 | 1054.83 | 950.0 | 0.5 | 0.5 | 0.0 | 35.96 | 8.0 | neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1189936 | 49.48 | 92.62 | 97.43 | 111.00 | 976.73 | 247.6 | 0.5 | 0.0 | 0.0 | 243.48 | 8.5 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-ergon-uring-spin-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189936 | 50.50 | 97.16 | 103.96 | 157.72 | 1054.96 | 1267.6 | 6.6 | 1.0 | 0.0 |
| h2 | 1189936 | 49.62 | 92.77 | 97.63 | 111.72 | 976.86 | 273.8 | 5.0 | 0.0 | 0.0 |

## HTTP round trip, http-ergon-uring-sqpoll

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1994 | 47.15 | 67.01 | 77.20 | 77.95 | 77.95 | 0.0 | 0.0 | 0.0 | 0.0 | 30.49 | 20.0 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-ergon-uring-sqpoll, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1994 | 47.31 | 67.16 | 77.33 | 78.08 | 78.08 | 0.0 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-ergon-uring-sqpoll-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1977595 | 44.27 | 51.87 | 60.63 | 77.80 | 213.19 | 71.5 | 0.0 | 0.0 | 0.0 | 31.79 | 6.0 | neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1977508 | 43.87 | 51.46 | 59.73 | 83.48 | 439.38 | 86.4 | 0.0 | 0.0 | 0.0 | 298.68 | 7.0 | neither loop stalled long enough: network, hypervisor or untraced |

## HTTP round trip, http-ergon-uring-sqpoll-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189938 | 45.27 | 53.20 | 67.93 | 84.26 | 287.73 | 13.1 | 0.0 | 0.0 | 0.0 | 253.32 | 7.0 | neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1189937 | 46.03 | 53.61 | 61.11 | 75.96 | 180.91 | 9.6 | 0.0 | 0.0 | 0.0 | 33.07 | 5.0 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-ergon-uring-sqpoll-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 1189938 | 45.42 | 53.37 | 68.12 | 85.74 | 448.04 | 27.2 | 0.0 | 0.0 | 0.0 |
| h2 | 1189937 | 46.19 | 53.78 | 61.32 | 76.28 | 183.58 | 10.1 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-reqwest

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1993 | 248.04 | 415.53 | 464.99 | 469.93 | 469.93 | 60029.2 | 0.0 | 0.0 | 0.0 | 1336.04 | 60140.0 | client: not traced |

### HTTP round trip, http-reqwest, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| smoke | 1993 | 248.10 | 415.61 | 465.06 | 470.00 | 470.00 | 60029.2 | 0.0 | 0.0 | 0.0 |

## HTTP round trip, http-reqwest-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 492099 | 245.27 | 351.10 | 390.48 | 425.49 | 485.10 | 230067.5 | 0.0 | 0.0 | 0.0 | 1351.80 | 248105.0 | client: preempted by swapper/2 |
| h2 | 490553 | 245.30 | 349.24 | 392.54 | 429.82 | 497.45 | 230854.7 | 0.0 | 0.0 | 0.0 | 1493.46 | 247343.5 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

## HTTP round trip, http-reqwest-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 470755 | 245.43 | 409.13 | 460.25 | 490.14 | 583.63 | 221520.6 | 17.6 | 0.0 | 0.0 | 1310.54 | 237412.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| h2 | 491191 | 245.37 | 352.89 | 390.39 | 430.02 | 482.73 | 227014.3 | 0.0 | 0.0 | 0.0 | 1507.90 | 247661.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### HTTP round trip, http-reqwest-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| h1 | 470755 | 35610754.81 | 71797778.31 | 72448489.63 | 72510739.93 | 72517805.74 | 237371.2 | 237371.2 | 237371.2 | 237371.2 |
| h2 | 491191 | 35531844.65 | 69777690.43 | 70399012.04 | 70460509.94 | 70467867.04 | 247676.4 | 247676.4 | 247676.4 | 247676.4 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### http-api-native-recv-busy

| condition | worst events |
|---|---|
| smoke | 81.29 at 2.7 s: neither loop stalled long enough: network, hypervisor or untraced; 80.92 at 1.1 s: client: not traced; 76.09 at 1.3 s: neither loop stalled long enough: network, hypervisor or untraced; 74.64 at 3.0 s: neither loop stalled long enough: network, hypervisor or untraced; 74.23 at 2.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-api-native-recv-busy-0

| condition | worst events |
|---|---|
| h1 | 349.71 at 85.6 s: client: softirq TIMER; 239.32 at 16.5 s: neither loop stalled long enough: network, hypervisor or untraced; 169.90 at 15.5 s: neither loop stalled long enough: network, hypervisor or untraced; 162.06 at 47.5 s: neither loop stalled long enough: network, hypervisor or untraced; 156.10 at 39.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1028.24 at 40.3 s: neither loop stalled long enough: network, hypervisor or untraced; 1017.42 at 43.1 s: neither loop stalled long enough: network, hypervisor or untraced; 284.15 at 26.3 s: neither loop stalled long enough: network, hypervisor or untraced; 234.35 at 30.3 s: neither loop stalled long enough: network, hypervisor or untraced; 217.71 at 26.3 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-api-native-recv-busy-10000

| condition | worst events |
|---|---|
| h1 | 321.39 at 2.8 s: neither loop stalled long enough: network, hypervisor or untraced; 248.30 at 71.1 s: neither loop stalled long enough: network, hypervisor or untraced; 215.46 at 71.1 s: neither loop stalled long enough: network, hypervisor or untraced; 191.21 at 71.1 s: neither loop stalled long enough: network, hypervisor or untraced; 178.92 at 71.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 239.03 at 51.7 s: server: softirq TIMER; 141.72 at 117.9 s: neither loop stalled long enough: network, hypervisor or untraced; 140.32 at 72.4 s: neither loop stalled long enough: network, hypervisor or untraced; 135.94 at 99.7 s: neither loop stalled long enough: network, hypervisor or untraced; 130.35 at 30.7 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-curl-distro

| condition | worst events |
|---|---|
| smoke | 200.14 at 2.9 s: neither loop stalled long enough: network, hypervisor or untraced; 186.54 at 1.6 s: neither loop stalled long enough: network, hypervisor or untraced; 182.58 at 2.5 s: neither loop stalled long enough: network, hypervisor or untraced; 159.79 at 1.7 s: neither loop stalled long enough: network, hypervisor or untraced; 152.68 at 1.1 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-curl-distro-0

| condition | worst events |
|---|---|
| h1 | 209.20 at 48.4 s: neither loop stalled long enough: network, hypervisor or untraced; 207.93 at 26.8 s: neither loop stalled long enough: network, hypervisor or untraced; 203.92 at 32.8 s: neither loop stalled long enough: network, hypervisor or untraced; 202.50 at 100.8 s: neither loop stalled long enough: network, hypervisor or untraced; 200.40 at 19.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 296.39 at 99.5 s: neither loop stalled long enough: network, hypervisor or untraced; 269.47 at 61.5 s: neither loop stalled long enough: network, hypervisor or untraced; 240.45 at 26.6 s: server: softirq TIMER; 219.07 at 67.5 s: neither loop stalled long enough: network, hypervisor or untraced; 211.54 at 85.5 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-curl-distro-10000

| condition | worst events |
|---|---|
| h1 | 239.90 at 119.9 s: server: softirq TIMER; 156.87 at 66.2 s: neither loop stalled long enough: network, hypervisor or untraced; 150.00 at 72.3 s: neither loop stalled long enough: network, hypervisor or untraced; 147.84 at 24.5 s: neither loop stalled long enough: network, hypervisor or untraced; 141.34 at 11.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 254.35 at 43.8 s: neither loop stalled long enough: network, hypervisor or untraced; 235.71 at 102.8 s: neither loop stalled long enough: network, hypervisor or untraced; 235.58 at 106.8 s: neither loop stalled long enough: network, hypervisor or untraced; 217.62 at 48.8 s: neither loop stalled long enough: network, hypervisor or untraced; 216.89 at 50.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-curl-source

| condition | worst events |
|---|---|
| smoke | 101.63 at 1.9 s: neither loop stalled long enough: network, hypervisor or untraced; 99.47 at 1.1 s: neither loop stalled long enough: network, hypervisor or untraced; 98.50 at 1.7 s: neither loop stalled long enough: network, hypervisor or untraced; 96.70 at 2.6 s: neither loop stalled long enough: network, hypervisor or untraced; 95.34 at 1.7 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-curl-source-0

| condition | worst events |
|---|---|
| h1 | 152.54 at 44.3 s: neither loop stalled long enough: network, hypervisor or untraced; 151.49 at 107.1 s: neither loop stalled long enough: network, hypervisor or untraced; 150.54 at 28.5 s: neither loop stalled long enough: network, hypervisor or untraced; 144.00 at 41.1 s: neither loop stalled long enough: network, hypervisor or untraced; 143.27 at 8.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 269.40 at 74.0 s: neither loop stalled long enough: network, hypervisor or untraced; 255.82 at 105.2 s: neither loop stalled long enough: network, hypervisor or untraced; 163.90 at 22.8 s: neither loop stalled long enough: network, hypervisor or untraced; 148.08 at 106.2 s: neither loop stalled long enough: network, hypervisor or untraced; 146.16 at 6.7 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-curl-source-10000

| condition | worst events |
|---|---|
| h1 | 262.11 at 59.9 s: neither loop stalled long enough: network, hypervisor or untraced; 230.05 at 99.9 s: neither loop stalled long enough: network, hypervisor or untraced; 215.73 at 76.9 s: neither loop stalled long enough: network, hypervisor or untraced; 213.76 at 46.9 s: neither loop stalled long enough: network, hypervisor or untraced; 210.40 at 30.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 345.45 at 27.6 s: neither loop stalled long enough: network, hypervisor or untraced; 251.11 at 14.4 s: server: softirq TIMER; 228.89 at 11.6 s: client: softirq TIMER; 157.30 at 119.8 s: neither loop stalled long enough: network, hypervisor or untraced; 136.62 at 7.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-ergon-epoll-spin

| condition | worst events |
|---|---|
| smoke | 104.73 at 1.2 s: neither loop stalled long enough: network, hypervisor or untraced; 100.84 at 2.9 s: neither loop stalled long enough: network, hypervisor or untraced; 99.99 at 1.6 s: neither loop stalled long enough: network, hypervisor or untraced; 99.89 at 1.6 s: neither loop stalled long enough: network, hypervisor or untraced; 99.38 at 2.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-ergon-epoll-spin-0

| condition | worst events |
|---|---|
| h1 | 226.33 at 41.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 158.44 at 76.8 s: neither loop stalled long enough: network, hypervisor or untraced; 157.04 at 23.8 s: neither loop stalled long enough: network, hypervisor or untraced; 147.86 at 22.6 s: neither loop stalled long enough: network, hypervisor or untraced; 146.10 at 88.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 193.86 at 91.7 s: neither loop stalled long enough: network, hypervisor or untraced; 170.30 at 21.1 s: neither loop stalled long enough: network, hypervisor or untraced; 167.23 at 9.4 s: neither loop stalled long enough: network, hypervisor or untraced; 163.31 at 64.9 s: neither loop stalled long enough: network, hypervisor or untraced; 160.17 at 17.2 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-ergon-epoll-spin-10000

| condition | worst events |
|---|---|
| h1 | 340.29 at 82.3 s: client: softirq TIMER; 223.32 at 71.7 s: neither loop stalled long enough: network, hypervisor or untraced; 205.07 at 85.1 s: server: softirq TIMER; 195.54 at 6.9 s: neither loop stalled long enough: network, hypervisor or untraced; 164.69 at 52.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 1013.67 at 70.5 s: neither loop stalled long enough: network, hypervisor or untraced; 212.57 at 30.2 s: neither loop stalled long enough: network, hypervisor or untraced; 211.25 at 77.5 s: neither loop stalled long enough: network, hypervisor or untraced; 201.00 at 67.4 s: neither loop stalled long enough: network, hypervisor or untraced; 157.65 at 96.4 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-ergon-recv-busy

| condition | worst events |
|---|---|
| smoke | 74.63 at 2.9 s: neither loop stalled long enough: network, hypervisor or untraced; 74.44 at 1.7 s: neither loop stalled long enough: network, hypervisor or untraced; 73.96 at 1.6 s: neither loop stalled long enough: network, hypervisor or untraced; 73.18 at 1.2 s: neither loop stalled long enough: network, hypervisor or untraced; 73.13 at 1.3 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-ergon-recv-busy-0

| condition | worst events |
|---|---|
| h1 | 287.81 at 4.9 s: neither loop stalled long enough: network, hypervisor or untraced; 165.77 at 7.8 s: server: softirq TIMER; 153.21 at 67.8 s: neither loop stalled long enough: network, hypervisor or untraced; 137.07 at 2.3 s: neither loop stalled long enough: network, hypervisor or untraced; 133.67 at 9.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 153.91 at 10.9 s: neither loop stalled long enough: network, hypervisor or untraced; 145.85 at 10.9 s: neither loop stalled long enough: network, hypervisor or untraced; 132.40 at 65.9 s: neither loop stalled long enough: network, hypervisor or untraced; 124.97 at 79.5 s: neither loop stalled long enough: network, hypervisor or untraced; 122.83 at 84.1 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-ergon-recv-busy-10000

| condition | worst events |
|---|---|
| h1 | 454.43 at 48.7 s: neither loop stalled long enough: network, hypervisor or untraced; 293.35 at 48.7 s: neither loop stalled long enough: network, hypervisor or untraced; 214.27 at 48.7 s: neither loop stalled long enough: network, hypervisor or untraced; 213.32 at 48.7 s: neither loop stalled long enough: network, hypervisor or untraced; 211.75 at 48.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 226.04 at 26.6 s: neither loop stalled long enough: network, hypervisor or untraced; 201.38 at 1.6 s: neither loop stalled long enough: network, hypervisor or untraced; 198.78 at 3.6 s: neither loop stalled long enough: network, hypervisor or untraced; 197.48 at 18.6 s: neither loop stalled long enough: network, hypervisor or untraced; 193.35 at 62.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-ergon-uring-spin

| condition | worst events |
|---|---|
| smoke | 95.45 at 2.5 s: neither loop stalled long enough: network, hypervisor or untraced; 94.22 at 2.5 s: neither loop stalled long enough: network, hypervisor or untraced; 85.19 at 2.8 s: neither loop stalled long enough: network, hypervisor or untraced; 83.78 at 2.7 s: neither loop stalled long enough: network, hypervisor or untraced; 83.54 at 3.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-ergon-uring-spin-0

| condition | worst events |
|---|---|
| h1 | 239.06 at 42.1 s: neither loop stalled long enough: network, hypervisor or untraced; 223.39 at 3.7 s: neither loop stalled long enough: network, hypervisor or untraced; 221.24 at 42.1 s: client: softirq TIMER; 220.23 at 45.0 s: server: softirq TIMER; 193.51 at 63.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 237.88 at 10.8 s: neither loop stalled long enough: network, hypervisor or untraced; 223.71 at 6.8 s: neither loop stalled long enough: network, hypervisor or untraced; 210.25 at 59.8 s: neither loop stalled long enough: network, hypervisor or untraced; 204.02 at 11.8 s: neither loop stalled long enough: network, hypervisor or untraced; 202.31 at 41.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-ergon-uring-spin-10000

| condition | worst events |
|---|---|
| h1 | 1054.83 at 107.0 s: neither loop stalled long enough: network, hypervisor or untraced; 274.10 at 24.3 s: neither loop stalled long enough: network, hypervisor or untraced; 241.36 at 72.5 s: neither loop stalled long enough: network, hypervisor or untraced; 216.23 at 14.5 s: neither loop stalled long enough: network, hypervisor or untraced; 215.79 at 12.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 976.73 at 53.5 s: neither loop stalled long enough: network, hypervisor or untraced; 291.25 at 91.6 s: client: softirq TIMER; 268.84 at 94.4 s: server: softirq TIMER; 139.71 at 84.8 s: neither loop stalled long enough: network, hypervisor or untraced; 138.37 at 94.3 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-ergon-uring-sqpoll

| condition | worst events |
|---|---|
| smoke | 77.95 at 2.8 s: neither loop stalled long enough: network, hypervisor or untraced; 77.20 at 1.3 s: neither loop stalled long enough: network, hypervisor or untraced; 76.59 at 1.5 s: neither loop stalled long enough: network, hypervisor or untraced; 75.11 at 1.4 s: neither loop stalled long enough: network, hypervisor or untraced; 74.53 at 2.7 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-ergon-uring-sqpoll-0

| condition | worst events |
|---|---|
| h1 | 213.19 at 48.1 s: neither loop stalled long enough: network, hypervisor or untraced; 193.47 at 6.1 s: neither loop stalled long enough: network, hypervisor or untraced; 179.24 at 43.1 s: neither loop stalled long enough: network, hypervisor or untraced; 176.53 at 56.1 s: neither loop stalled long enough: network, hypervisor or untraced; 170.69 at 42.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 439.38 at 33.5 s: neither loop stalled long enough: network, hypervisor or untraced; 323.11 at 86.2 s: neither loop stalled long enough: network, hypervisor or untraced; 195.02 at 78.6 s: neither loop stalled long enough: network, hypervisor or untraced; 189.24 at 45.4 s: neither loop stalled long enough: network, hypervisor or untraced; 184.80 at 86.2 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-ergon-uring-sqpoll-10000

| condition | worst events |
|---|---|
| h1 | 287.73 at 104.1 s: neither loop stalled long enough: network, hypervisor or untraced; 266.48 at 49.8 s: client: softirq TIMER; 233.78 at 52.7 s: server: softirq TIMER; 178.03 at 104.1 s: neither loop stalled long enough: network, hypervisor or untraced; 152.34 at 104.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| h2 | 180.91 at 64.0 s: neither loop stalled long enough: network, hypervisor or untraced; 135.83 at 68.7 s: neither loop stalled long enough: network, hypervisor or untraced; 133.06 at 23.8 s: neither loop stalled long enough: network, hypervisor or untraced; 123.60 at 52.2 s: neither loop stalled long enough: network, hypervisor or untraced; 121.62 at 50.7 s: neither loop stalled long enough: network, hypervisor or untraced |

### http-reqwest

| condition | worst events |
|---|---|
| smoke | 469.93 at 1.3 s: client: not traced; 464.99 at 1.2 s: client: not traced; 444.86 at 2.7 s: client: not traced; 434.28 at 2.4 s: client: not traced; 429.94 at 1.6 s: client: not traced |

### http-reqwest-0

| condition | worst events |
|---|---|
| h1 | 485.10 at 96.4 s: client: preempted by swapper/2; 480.98 at 90.4 s: client: preempted by swapper/2; 471.73 at 13.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 470.70 at 114.5 s: client: preempted by swapper/2; 470.37 at 76.8 s: client: preempted by swapper/2 |
| h2 | 497.45 at 46.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 491.33 at 91.8 s: client: preempted by swapper/2; 488.40 at 94.5 s: client: preempted by swapper/2; 487.15 at 46.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 484.15 at 46.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |

### http-reqwest-10000

| condition | worst events |
|---|---|
| h1 | 583.63 at 13.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 552.18 at 84.6 s: client: preempted by swapper/2; 552.17 at 32.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 531.99 at 50.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 531.86 at 94.4 s: client: preempted by swapper/2 |
| h2 | 482.73 at 24.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 478.30 at 47.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 474.49 at 113.4 s: client: preempted by swapper/2; 473.14 at 56.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 471.20 at 74.2 s: client: preempted by swapper/2 |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| smoke | not traced 824065.80; loop work 75.12 |
| h1 | preempted by swapper/2 121142957.73; unexplained: no kernel event (hypervisor or in-kernel time) 117095174.80; mostly unexplained (+preempted by swapper/2) 1600423.11; mostly unexplained (+softirq NET_RX) 30034.95; mostly unexplained (+preempted by kworker/2:1) 28126.79 |
| h2 | unexplained: no kernel event (hypervisor or in-kernel time) 120331503.79; preempted by swapper/2 117793758.89; mostly unexplained (+preempted by swapper/2) 1636502.46; mostly unexplained (+preempted by kworker/2:1) 8524.62; mostly unexplained (+irq work (IPI/vector)) 6743.95 |

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
| http-api-native-recv-busy | receiver loop | 0 | | | | | |
| http-api-native-recv-busy | slow samples | 0 | | | | | |
| http-api-native-recv-busy-0 | idle spare core | 0 | | | | | |
| http-api-native-recv-busy-0 | receiver loop | 1 | :53 | 1 (100%) | 1 of 4 | 343.75 | no |
| http-api-native-recv-busy-0 | sender loop | 0 | | | | | |
| http-api-native-recv-busy-0 | slow samples | 3 | :22 | 1 (33%) | 1 of 2 | 1028.24 | no |
| http-api-native-recv-busy-10000 | idle spare core | 0 | | | | | |
| http-api-native-recv-busy-10000 | receiver loop | 1 | :16 | 1 (100%) | 1 of 4 | 324.81 | no |
| http-api-native-recv-busy-10000 | sender loop | 0 | | | | | |
| http-api-native-recv-busy-10000 | slow samples | 1 | :10 | 1 (100%) | 1 of 4 | 321.39 | no |
| http-curl-distro | receiver loop | 0 | | | | | |
| http-curl-distro | slow samples | 0 | | | | | |
| http-curl-distro-0 | idle spare core | 0 | | | | | |
| http-curl-distro-0 | receiver loop | 0 | | | | | |
| http-curl-distro-0 | sender loop | 0 | | | | | |
| http-curl-distro-0 | slow samples | 0 | | | | | |
| http-curl-distro-10000 | idle spare core | 1 | :30 | 1 (100%) | 1 of 4 | 389.19 | no |
| http-curl-distro-10000 | receiver loop | 0 | | | | | |
| http-curl-distro-10000 | sender loop | 0 | | | | | |
| http-curl-distro-10000 | slow samples | 0 | | | | | |
| http-curl-source | receiver loop | 0 | | | | | |
| http-curl-source | slow samples | 0 | | | | | |
| http-curl-source-0 | idle spare core | 1 | :20 | 1 (100%) | 1 of 4 | 325.37 | no |
| http-curl-source-0 | receiver loop | 0 | | | | | |
| http-curl-source-0 | sender loop | 0 | | | | | |
| http-curl-source-0 | slow samples | 0 | | | | | |
| http-curl-source-10000 | idle spare core | 0 | | | | | |
| http-curl-source-10000 | receiver loop | 0 | | | | | |
| http-curl-source-10000 | sender loop | 0 | | | | | |
| http-curl-source-10000 | slow samples | 1 | :04 | 1 (100%) | 1 of 4 | 345.45 | no |
| http-ergon-epoll-spin | receiver loop | 0 | | | | | |
| http-ergon-epoll-spin | slow samples | 0 | | | | | |
| http-ergon-epoll-spin-0 | idle spare core | 1 | :06 | 1 (100%) | 1 of 3 | 333.12 | no |
| http-ergon-epoll-spin-0 | receiver loop | 0 | | | | | |
| http-ergon-epoll-spin-0 | sender loop | 0 | | | | | |
| http-ergon-epoll-spin-0 | slow samples | 0 | | | | | |
| http-ergon-epoll-spin-10000 | idle spare core | 0 | | | | | |
| http-ergon-epoll-spin-10000 | receiver loop | 0 | | | | | |
| http-ergon-epoll-spin-10000 | sender loop | 0 | | | | | |
| http-ergon-epoll-spin-10000 | slow samples | 2 | :02 | 1 (50%) | 1 of 4 | 340.29 | no |
| http-ergon-recv-busy | receiver loop | 0 | | | | | |
| http-ergon-recv-busy | slow samples | 0 | | | | | |
| http-ergon-recv-busy-0 | idle spare core | 0 | | | | | |
| http-ergon-recv-busy-0 | receiver loop | 0 | | | | | |
| http-ergon-recv-busy-0 | sender loop | 0 | | | | | |
| http-ergon-recv-busy-0 | slow samples | 0 | | | | | |
| http-ergon-recv-busy-10000 | idle spare core | 0 | | | | | |
| http-ergon-recv-busy-10000 | receiver loop | 0 | | | | | |
| http-ergon-recv-busy-10000 | sender loop | 0 | | | | | |
| http-ergon-recv-busy-10000 | slow samples | 1 | :05 | 1 (100%) | 1 of 4 | 454.43 | no |
| http-ergon-uring-spin | receiver loop | 0 | | | | | |
| http-ergon-uring-spin | slow samples | 0 | | | | | |
| http-ergon-uring-spin-0 | idle spare core | 0 | | | | | |
| http-ergon-uring-spin-0 | receiver loop | 0 | | | | | |
| http-ergon-uring-spin-0 | sender loop | 0 | | | | | |
| http-ergon-uring-spin-0 | slow samples | 0 | | | | | |
| http-ergon-uring-spin-10000 | idle spare core | 1 | :43 | 1 (100%) | 1 of 4 | 314.35 | no |
| http-ergon-uring-spin-10000 | receiver loop | 0 | | | | | |
| http-ergon-uring-spin-10000 | sender loop | 0 | | | | | |
| http-ergon-uring-spin-10000 | slow samples | 2 | :05 | 1 (50%) | 1 of 4 | 976.73 | no |
| http-ergon-uring-sqpoll | receiver loop | 0 | | | | | |
| http-ergon-uring-sqpoll | slow samples | 0 | | | | | |
| http-ergon-uring-sqpoll-0 | idle spare core | 1 | :34 | 1 (100%) | 1 of 3 | 333.42 | no |
| http-ergon-uring-sqpoll-0 | receiver loop | 0 | | | | | |
| http-ergon-uring-sqpoll-0 | sender loop | 0 | | | | | |
| http-ergon-uring-sqpoll-0 | slow samples | 2 | :25 | 1 (50%) | 1 of 4 | 323.11 | no |
| http-ergon-uring-sqpoll-10000 | idle spare core | 0 | | | | | |
| http-ergon-uring-sqpoll-10000 | receiver loop | 0 | | | | | |
| http-ergon-uring-sqpoll-10000 | sender loop | 0 | | | | | |
| http-ergon-uring-sqpoll-10000 | slow samples | 0 | | | | | |
| http-reqwest | receiver loop | 489 | :08 | 489 (100%) | 1 of 1 | 470.19 | **yes** |
| http-reqwest | slow samples | 489 | :08 | 489 (100%) | 1 of 1 | 469.93 | **yes** |
| http-reqwest-0 | idle spare core | 0 | | | | | |
| http-reqwest-0 | receiver loop | 60590 | :48 | 3825 (6%) | 4 of 4 | 470.71 | no |
| http-reqwest-0 | sender loop | 0 | | | | | |
| http-reqwest-0 | slow samples | 60398 | :48 | 3814 (6%) | 4 of 4 | 470.34 | no |
| http-reqwest-10000 | idle spare core | 0 | | | | | |
| http-reqwest-10000 | receiver loop | 75687 | :22 | 4507 (6%) | 4 of 4 | 531.75 | no |
| http-reqwest-10000 | sender loop | 0 | | | | | |
| http-reqwest-10000 | slow samples | 75424 | :22 | 4500 (6%) | 4 of 4 | 531.39 | no |

# Every run in detail

#### Stalls: smoke/http-api-native-recv-busy, loop 0 on CPU 2 (no kernel trace: causes limited to loop work)

81056602 iterations, longest 39.9 µs; over 5 / 20 / 100 µs: 81 / 18 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| not traced | 17 | 462.0 | 39.9 |
| loop work | 1 | 21.4 | 21.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 39.9 | 1 | not traced |  |
| 2 | 31.4 | 0 | not traced |  |
| 3 | 30.2 | 1 | not traced |  |
| 4 | 29.8 | 1 | not traced |  |
| 5 | 29.2 | 1 | not traced |  |
| 6 | 28.0 | 1 | not traced |  |
| 7 | 27.9 | 1 | not traced |  |
| 8 | 27.4 | 1 | not traced |  |
| 9 | 26.9 | 1 | not traced |  |
| 10 | 26.8 | 1 | not traced |  |

#### Slowest round trips: smoke/http-api-native-recv-busy (median 45.49)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 81.29 | 35.80 | 0.00 |  | 0.00 |  | 35.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 80.92 | 35.43 | 39.63 | not traced | 0.00 |  | 0.00 | client: not traced |
| 3 | 76.09 | 30.60 | 0.00 |  | 0.00 |  | 30.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 74.64 | 29.15 | 0.00 |  | 0.00 |  | 29.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 74.23 | 28.74 | 0.00 |  | 0.00 |  | 28.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 72.13 | 26.64 | 28.99 | not traced | 0.00 |  | 0.00 | client: not traced |
| 7 | 71.31 | 25.82 | 0.00 |  | 0.00 |  | 25.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 70.40 | 24.91 | 29.64 | not traced | 0.00 |  | 0.00 | client: not traced |
| 9 | 70.19 | 24.70 | 27.76 | not traced | 0.00 |  | 0.00 | client: not traced |
| 10 | 69.84 | 24.35 | 0.00 |  | 0.00 |  | 24.35 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: smoke/http-curl-distro, loop 0 on CPU 2 (no kernel trace: causes limited to loop work)

77952427 iterations, longest 1417.7 µs; over 5 / 20 / 100 µs: 5748 / 7 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| not traced | 7 | 2109.5 | 1417.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1417.7 | 0 | not traced |  |
| 2 | 509.7 | 0 | not traced |  |
| 3 | 68.4 | 0 | not traced |  |
| 4 | 47.4 | 1 | not traced |  |
| 5 | 24.5 | 1 | not traced |  |
| 6 | 21.4 | 1 | not traced |  |
| 7 | 20.4 | 0 | not traced |  |

#### Slowest round trips: smoke/http-curl-distro (median 89.16)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 200.14 | 110.98 | 0.00 |  | 0.00 |  | 110.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 186.54 | 97.38 | 0.00 |  | 0.00 |  | 97.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 182.58 | 93.42 | 0.00 |  | 0.00 |  | 93.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 159.79 | 70.63 | 0.00 |  | 0.00 |  | 70.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 152.68 | 63.52 | 0.00 |  | 0.00 |  | 63.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 150.81 | 61.65 | 0.00 |  | 0.00 |  | 61.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 149.63 | 60.47 | 0.00 |  | 0.00 |  | 60.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 147.33 | 58.17 | 0.00 |  | 0.00 |  | 58.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 147.25 | 58.09 | 0.00 |  | 0.00 |  | 58.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 145.12 | 55.96 | 0.00 |  | 0.00 |  | 55.96 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: smoke/http-curl-source, loop 0 on CPU 2 (no kernel trace: causes limited to loop work)

79882906 iterations, longest 839.2 µs; over 5 / 20 / 100 µs: 3067 / 5 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| not traced | 5 | 1494.8 | 839.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 839.2 | 0 | not traced |  |
| 2 | 490.4 | 0 | not traced |  |
| 3 | 92.6 | 0 | not traced |  |
| 4 | 51.7 | 1 | not traced |  |
| 5 | 20.8 | 0 | not traced |  |

#### Slowest round trips: smoke/http-curl-source (median 58.42)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 101.63 | 43.21 | 0.00 |  | 0.00 |  | 43.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 99.47 | 41.05 | 0.00 |  | 0.00 |  | 41.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 98.50 | 40.08 | 0.00 |  | 0.00 |  | 40.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 96.70 | 38.28 | 0.00 |  | 0.00 |  | 38.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 95.34 | 36.92 | 0.00 |  | 0.00 |  | 36.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 91.14 | 32.72 | 0.00 |  | 0.00 |  | 32.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 89.78 | 31.36 | 0.00 |  | 0.00 |  | 31.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 89.12 | 30.70 | 0.00 |  | 0.00 |  | 30.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 89.07 | 30.65 | 0.00 |  | 0.00 |  | 30.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 89.00 | 30.58 | 0.00 |  | 0.00 |  | 30.58 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: smoke/http-ergon-epoll-spin, loop 0 on CPU 2 (no kernel trace: causes limited to loop work)

81016258 iterations, longest 22.5 µs; over 5 / 20 / 100 µs: 200 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 22.5 | 22.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 22.5 | 2 | loop work (2 items) |  |

#### Slowest round trips: smoke/http-ergon-epoll-spin (median 48.59)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 104.73 | 56.14 | 0.00 |  | 0.00 |  | 56.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 100.84 | 52.25 | 0.00 |  | 0.00 |  | 52.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 99.99 | 51.40 | 0.00 |  | 0.00 |  | 51.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 99.89 | 51.29 | 0.00 |  | 0.00 |  | 51.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 99.38 | 50.78 | 0.00 |  | 0.00 |  | 50.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 98.22 | 49.63 | 0.00 |  | 0.00 |  | 49.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 98.15 | 49.56 | 0.00 |  | 0.00 |  | 49.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 98.07 | 49.48 | 0.00 |  | 0.00 |  | 49.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 98.00 | 49.40 | 0.00 |  | 0.00 |  | 49.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 97.95 | 49.35 | 0.00 |  | 0.00 |  | 49.35 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: smoke/http-ergon-recv-busy, loop 0 on CPU 2 (no kernel trace: causes limited to loop work)

80977795 iterations, longest 31.2 µs; over 5 / 20 / 100 µs: 131 / 17 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| not traced | 16 | 426.0 | 31.0 |
| loop work | 1 | 31.2 | 31.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 31.2 | 2 | loop work (2 items) |  |
| 2 | 31.0 | 1 | not traced |  |
| 3 | 30.1 | 1 | not traced |  |
| 4 | 29.9 | 1 | not traced |  |
| 5 | 29.8 | 1 | not traced |  |
| 6 | 28.7 | 1 | not traced |  |
| 7 | 28.3 | 1 | not traced |  |
| 8 | 27.5 | 1 | not traced |  |
| 9 | 26.9 | 1 | not traced |  |
| 10 | 26.5 | 1 | not traced |  |

#### Slowest round trips: smoke/http-ergon-recv-busy (median 45.32)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 74.63 | 29.30 | 0.00 |  | 0.00 |  | 29.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 74.44 | 29.12 | 0.00 |  | 0.00 |  | 29.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 73.96 | 28.63 | 0.00 |  | 0.00 |  | 28.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 73.18 | 27.85 | 0.00 |  | 0.00 |  | 27.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 73.13 | 27.81 | 0.00 |  | 0.00 |  | 27.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 72.45 | 27.12 | 28.44 | not traced | 0.00 |  | 0.00 | client: not traced |
| 7 | 72.06 | 26.74 | 0.00 |  | 0.00 |  | 26.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 71.88 | 26.56 | 29.66 | not traced | 0.00 |  | 0.00 | client: not traced |
| 9 | 71.57 | 26.25 | 0.00 |  | 0.00 |  | 26.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 70.42 | 25.09 | 0.00 |  | 0.00 |  | 25.09 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: smoke/http-ergon-uring-spin, loop 0 on CPU 2 (no kernel trace: causes limited to loop work)

84433236 iterations, longest 19.2 µs; over 5 / 20 / 100 µs: 26 / 0 / 0

#### Stalls: smoke/http-ergon-uring-sqpoll, loop 0 on CPU 2 (no kernel trace: causes limited to loop work)

83528157 iterations, longest 30.5 µs; over 5 / 20 / 100 µs: 9 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| not traced | 1 | 30.5 | 30.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 30.5 | 0 | not traced |  |

#### Slowest round trips: smoke/http-ergon-uring-sqpoll (median 47.15)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 77.95 | 30.81 | 0.00 |  | 0.00 |  | 30.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 77.20 | 30.06 | 0.00 |  | 0.00 |  | 30.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 76.59 | 29.44 | 0.00 |  | 0.00 |  | 29.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 75.11 | 27.97 | 0.00 |  | 0.00 |  | 27.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 74.53 | 27.39 | 0.00 |  | 0.00 |  | 27.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 73.05 | 25.91 | 0.00 |  | 0.00 |  | 25.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 72.02 | 24.87 | 0.00 |  | 0.00 |  | 24.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 71.96 | 24.81 | 0.00 |  | 0.00 |  | 24.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 71.65 | 24.51 | 0.00 |  | 0.00 |  | 24.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 71.02 | 23.88 | 0.00 |  | 0.00 |  | 23.88 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: smoke/http-reqwest, loop 0 on CPU 2 (no kernel trace: causes limited to loop work)

61632703 iterations, longest 1336.0 µs; over 5 / 20 / 100 µs: 3007 / 3007 / 3000

| cause | stalls | total µs | max µs |
|---|---|---|---|
| not traced | 3007 | 819543.0 | 1336.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1336.0 | 1 | not traced |  |
| 2 | 470.2 | 1 | not traced |  |
| 3 | 465.2 | 1 | not traced |  |
| 4 | 445.1 | 1 | not traced |  |
| 5 | 434.5 | 1 | not traced |  |
| 6 | 430.2 | 1 | not traced |  |
| 7 | 429.4 | 1 | not traced |  |
| 8 | 426.4 | 1 | not traced |  |
| 9 | 425.4 | 1 | not traced |  |
| 10 | 421.3 | 1 | not traced |  |

#### Slowest round trips: smoke/http-reqwest (median 248.04)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 469.93 | 221.89 | 469.80 | not traced | 0.00 |  | 0.00 | client: not traced |
| 2 | 464.99 | 216.95 | 464.86 | not traced | 0.00 |  | 0.00 | client: not traced |
| 3 | 444.86 | 196.82 | 444.73 | not traced | 0.00 |  | 0.00 | client: not traced |
| 4 | 434.28 | 186.24 | 434.15 | not traced | 0.00 |  | 0.00 | client: not traced |
| 5 | 429.94 | 181.90 | 429.81 | not traced | 0.00 |  | 0.00 | client: not traced |
| 6 | 426.09 | 178.05 | 425.96 | not traced | 0.00 |  | 0.00 | client: not traced |
| 7 | 420.71 | 172.67 | 420.57 | not traced | 0.00 |  | 0.00 | client: not traced |
| 8 | 420.60 | 172.56 | 420.47 | not traced | 0.00 |  | 0.00 | client: not traced |
| 9 | 420.29 | 172.25 | 420.16 | not traced | 0.00 |  | 0.00 | client: not traced |
| 10 | 420.28 | 172.24 | 420.17 | not traced | 0.00 |  | 0.00 | client: not traced |

#### Stalls: h1/http-api-native-recv-busy-0, loop 0 on CPU 2

197303443 iterations, longest 343.8 µs; over 5 / 20 / 100 µs: 141164 / 175 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 135 | 3053.7 | 35.2 |
| softirq TIMER | 1 | 343.8 | 343.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 11 | 278.4 | 31.6 |
| mostly unexplained (+local timer tick) | 8 | 174.8 | 24.0 |
| mostly unexplained (+preempted by kworker/2:1) | 7 | 154.0 | 24.3 |
| mostly unexplained (+softirq NET_RX) | 5 | 132.2 | 41.2 |
| softirq NET_RX | 5 | 112.3 | 25.1 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 83.1 | 51.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.4 | 28.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 343.8 | 0 | softirq TIMER | softirq TIMER 333, local timer tick 5, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 51.5 | 2 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 2, preempted by kworker/2:1 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 41.2 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 4 | 35.2 | 2 | loop work (2 items) | softirq NET_RX 5 |
| 5 | 33.5 | 2 | loop work (2 items) |  |
| 6 | 33.4 | 2 | loop work (2 items) |  |
| 7 | 32.2 | 2 | loop work (2 items) |  |
| 8 | 31.7 | 2 | loop work (2 items) |  |
| 9 | 31.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 31.6 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4567 |
| IWI IRQ work interrupts | 4529 |
| CAL Function call interrupts | 999 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: h1/http-api-native-recv-busy-0 (median 42.98)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 349.71 | 306.73 | 343.75 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 2 | 239.32 | 196.34 | 0.00 |  | 0.00 |  | 196.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 169.90 | 126.92 | 0.00 |  | 0.00 |  | 126.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 162.06 | 119.07 | 0.00 |  | 0.00 |  | 119.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 156.10 | 113.11 | 0.00 |  | 0.00 |  | 113.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 151.78 | 108.80 | 0.00 |  | 0.00 |  | 108.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 147.88 | 104.89 | 0.00 |  | 0.00 |  | 104.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 146.55 | 103.57 | 0.00 |  | 0.00 |  | 103.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 146.30 | 103.32 | 0.00 |  | 0.00 |  | 103.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 145.37 | 102.39 | 0.00 |  | 0.00 |  | 102.39 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-api-native-recv-busy-10000, loop 0 on CPU 2

1882618615 iterations, longest 63.7 µs; over 5 / 20 / 100 µs: 150534 / 5308 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 3207 | 80757.8 | 41.8 |
| mostly unexplained (+softirq NET_RX) | 1095 | 28398.7 | 58.6 |
| mostly unexplained (+preempted by kworker/2:1) | 893 | 26073.5 | 63.7 |
| softirq NET_RX | 92 | 2650.5 | 37.8 |
| mostly unexplained (+irq work (IPI/vector)) | 9 | 252.3 | 39.8 |
| loop work | 7 | 187.6 | 35.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 5 | 123.4 | 33.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 63.7 | 2 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 2 | 61.2 | 2 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 59.0 | 2 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 58.6 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 19, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 57.5 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 55.5 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 55.2 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 52.9 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 52.3 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 51.9 | 2 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 1536 |
| IWI IRQ work interrupts | 1505 |
| CAL Function call interrupts | 998 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: h1/http-api-native-recv-busy-10000 (median 46.48)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 321.39 | 274.91 | 0.00 |  | 0.00 |  | 274.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 248.30 | 201.81 | 0.00 |  | 0.00 |  | 201.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 215.46 | 168.98 | 0.00 |  | 0.00 |  | 168.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 191.21 | 144.73 | 0.00 |  | 0.00 |  | 144.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 178.92 | 132.44 | 0.00 |  | 0.00 |  | 132.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 173.38 | 126.90 | 0.00 |  | 0.00 |  | 126.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 173.34 | 126.85 | 0.00 |  | 0.00 |  | 126.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 168.93 | 122.45 | 0.00 |  | 0.00 |  | 122.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 164.21 | 117.73 | 0.00 |  | 0.00 |  | 117.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 164.06 | 117.58 | 0.00 |  | 0.00 |  | 117.58 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-curl-distro-0, loop 0 on CPU 2

213351470 iterations, longest 1478.6 µs; over 5 / 20 / 100 µs: 3033482 / 207 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 204 | 5354.3 | 550.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 1478.6 | 1478.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 82.5 | 42.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1478.6 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 550.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 69.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 55.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 50.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 42.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 41.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 40.2 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 38.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 38.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 1 |

#### Slowest round trips: h1/http-curl-distro-0 (median 58.12)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 209.20 | 151.08 | 0.00 |  | 0.00 |  | 151.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 207.93 | 149.81 | 0.00 |  | 0.00 |  | 149.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 203.92 | 145.80 | 0.00 |  | 0.00 |  | 145.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 202.50 | 144.38 | 0.00 |  | 0.00 |  | 144.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 200.40 | 142.28 | 0.00 |  | 23.61 | mostly unexplained (+preempted by kworker/1:1) | 118.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 195.56 | 137.44 | 0.00 |  | 0.00 |  | 137.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 193.90 | 135.78 | 0.00 |  | 0.00 |  | 135.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 191.70 | 133.57 | 0.00 |  | 0.00 |  | 133.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 190.32 | 132.20 | 0.00 |  | 0.00 |  | 132.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 188.51 | 130.38 | 0.00 |  | 0.00 |  | 130.38 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-curl-distro-10000, loop 0 on CPU 2

1501096625 iterations, longest 1485.7 µs; over 5 / 20 / 100 µs: 1280007 / 64 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 59 | 1950.9 | 549.4 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 1571.9 | 1485.7 |
| softirq TIMER | 1 | 208.2 | 208.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 56.0 | 32.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1485.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 549.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 208.2 | 0 | softirq TIMER | softirq TIMER 183, local timer tick 13, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 4 | 86.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 2, ipi_send_cpu 0 |
| 5 | 43.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 34.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 34.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 33.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 29.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: h1/http-curl-distro-10000 (median 58.51)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 239.90 | 181.39 | 0.00 |  | 146.38 | softirq TIMER | 35.01 | server: softirq TIMER |
| 2 | 156.87 | 98.37 | 0.00 |  | 0.00 |  | 98.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 150.00 | 91.50 | 0.00 |  | 0.00 |  | 91.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 147.84 | 89.34 | 0.00 |  | 0.00 |  | 89.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 141.34 | 82.83 | 0.00 |  | 0.00 |  | 82.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 140.70 | 82.19 | 0.00 |  | 0.00 |  | 82.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 140.47 | 81.96 | 0.00 |  | 0.00 |  | 81.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 138.60 | 80.10 | 0.00 |  | 0.00 |  | 80.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 130.79 | 72.29 | 0.00 |  | 0.00 |  | 72.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 130.10 | 71.59 | 0.00 |  | 0.00 |  | 71.59 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-curl-source-0, loop 0 on CPU 2

112686838 iterations, longest 871.2 µs; over 5 / 20 / 100 µs: 2334653 / 83 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 80 | 2001.8 | 93.5 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 1360.9 | 871.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 27.0 | 27.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 871.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 489.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 93.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 60.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 36.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 35.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 34.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 33.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 31.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| LOC Local timer interrupts | 5 |
| IWI IRQ work interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h1/http-curl-source-0 (median 55.17)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 152.54 | 97.37 | 0.00 |  | 0.00 |  | 97.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 151.49 | 96.32 | 0.00 |  | 0.00 |  | 96.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 150.54 | 95.37 | 0.00 |  | 0.00 |  | 95.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 144.00 | 88.83 | 0.00 |  | 0.00 |  | 88.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 143.27 | 88.10 | 0.00 |  | 0.00 |  | 88.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 142.44 | 87.27 | 0.00 |  | 0.00 |  | 87.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 141.71 | 86.54 | 0.00 |  | 0.00 |  | 86.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 137.53 | 82.36 | 0.00 |  | 0.00 |  | 82.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 136.81 | 81.64 | 0.00 |  | 0.00 |  | 81.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 136.77 | 81.60 | 0.00 |  | 0.00 |  | 81.60 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-curl-source-10000, loop 0 on CPU 2

1488576489 iterations, longest 846.2 µs; over 5 / 20 / 100 µs: 1255722 / 83 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 78 | 1801.5 | 51.5 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 1423.6 | 846.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.5 | 30.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 846.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 482.9 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 94.5 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 2, ipi_send_cpu 0 |
| 4 | 51.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 31.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 30.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 28.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 28.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 28.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 28.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h1/http-curl-source-10000 (median 55.99)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 262.11 | 206.12 | 0.00 |  | 0.00 |  | 206.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 230.05 | 174.05 | 0.00 |  | 0.00 |  | 174.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 215.73 | 159.74 | 0.00 |  | 0.00 |  | 159.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 213.76 | 157.76 | 0.00 |  | 0.00 |  | 157.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 210.40 | 154.40 | 0.00 |  | 0.00 |  | 154.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 193.24 | 137.24 | 0.00 |  | 0.00 |  | 137.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 166.54 | 110.55 | 0.00 |  | 0.00 |  | 110.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 164.85 | 108.86 | 0.00 |  | 0.00 |  | 108.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 162.26 | 106.26 | 0.00 |  | 0.00 |  | 106.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 160.01 | 104.02 | 0.00 |  | 0.00 |  | 104.02 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-ergon-epoll-spin-0, loop 0 on CPU 2

530224977 iterations, longest 179.9 µs; over 5 / 20 / 100 µs: 14507 / 22 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 13 | 461.5 | 179.9 |
| loop work | 8 | 185.0 | 26.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.6 | 34.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 179.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 34.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 31.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 26.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 26.7 | 2 | loop work (2 items) |  |
| 6 | 24.9 | 2 | loop work (2 items) |  |
| 7 | 24.6 | 2 | loop work (2 items) |  |
| 8 | 24.0 | 2 | loop work (2 items) |  |
| 9 | 23.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 23.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: h1/http-ergon-epoll-spin-0 (median 48.96)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 226.33 | 177.37 | 179.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 158.44 | 109.48 | 0.00 |  | 0.00 |  | 109.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 157.04 | 108.08 | 0.00 |  | 0.00 |  | 108.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 147.86 | 98.90 | 0.00 |  | 0.00 |  | 98.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 146.10 | 97.14 | 0.00 |  | 0.00 |  | 97.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 143.29 | 94.33 | 0.00 |  | 0.00 |  | 94.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 137.76 | 88.80 | 0.00 |  | 0.00 |  | 88.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 130.13 | 81.17 | 0.00 |  | 0.00 |  | 81.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 124.56 | 75.60 | 0.00 |  | 0.00 |  | 75.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 124.39 | 75.43 | 0.00 |  | 0.00 |  | 75.43 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-ergon-epoll-spin-10000, loop 0 on CPU 2

1958802120 iterations, longest 287.9 µs; over 5 / 20 / 100 µs: 9340 / 29 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 25 | 565.9 | 29.4 |
| softirq TIMER | 1 | 287.9 | 287.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 44.6 | 23.5 |
| loop work | 1 | 21.7 | 21.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 287.9 | 0 | softirq TIMER | softirq TIMER 265, local timer tick 14, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 29.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 28.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 27.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 26.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 24.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 24.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 23.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 23.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 23.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: h1/http-ergon-epoll-spin-10000 (median 48.64)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 340.29 | 291.64 | 287.94 | softirq TIMER | 0.00 |  | 3.70 | client: softirq TIMER |
| 2 | 223.32 | 174.68 | 0.00 |  | 0.00 |  | 174.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 205.07 | 156.43 | 0.00 |  | 126.04 | softirq TIMER | 30.39 | server: softirq TIMER |
| 4 | 195.54 | 146.90 | 0.00 |  | 0.00 |  | 146.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 164.69 | 116.05 | 23.46 | VM paused: the idle spare core stalled too (hypervisor) | 0.00 |  | 92.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 158.30 | 109.66 | 0.00 |  | 0.00 |  | 109.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 139.20 | 90.56 | 0.00 |  | 30.23 | mostly unexplained (+preempted by kworker/1:1) | 60.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 136.68 | 88.04 | 0.00 |  | 0.00 |  | 88.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 134.49 | 85.85 | 0.00 |  | 0.00 |  | 85.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 122.01 | 73.36 | 0.00 |  | 0.00 |  | 73.36 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-ergon-recv-busy-0, loop 0 on CPU 2

205877720 iterations, longest 111.2 µs; over 5 / 20 / 100 µs: 129995 / 438 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 362 | 8615.9 | 36.2 |
| mostly unexplained (+preempted by kworker/2:1) | 30 | 664.3 | 30.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 19 | 485.3 | 43.4 |
| softirq NET_RX | 16 | 385.9 | 28.2 |
| softirq TIMER | 1 | 111.2 | 111.2 |
| mostly unexplained (+local timer tick) | 4 | 84.5 | 22.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 72.8 | 28.4 |
| mostly unexplained (+softirq NET_RX) | 3 | 65.0 | 22.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 111.2 | 0 | softirq TIMER | softirq TIMER 100, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 2 | 43.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 36.2 | 2 | loop work (2 items) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 35.0 | 2 | loop work (2 items) |  |
| 5 | 35.0 | 2 | loop work (2 items) |  |
| 6 | 34.9 | 2 | loop work (2 items) |  |
| 7 | 34.2 | 2 | loop work (2 items) |  |
| 8 | 34.2 | 2 | loop work (2 items) |  |
| 9 | 33.9 | 2 | loop work (2 items) |  |
| 10 | 33.8 | 2 | loop work (2 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4561 |
| IWI IRQ work interrupts | 4511 |
| CAL Function call interrupts | 999 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h1/http-ergon-recv-busy-0 (median 42.04)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 287.81 | 245.77 | 111.17 | softirq TIMER | 0.00 |  | 134.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 165.77 | 123.73 | 0.00 |  | 139.16 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 153.21 | 111.17 | 0.00 |  | 0.00 |  | 111.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 137.07 | 95.03 | 0.00 |  | 0.00 |  | 95.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 133.67 | 91.62 | 0.00 |  | 0.00 |  | 91.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 133.28 | 91.24 | 0.00 |  | 0.00 |  | 91.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 122.55 | 80.51 | 0.00 |  | 0.00 |  | 80.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 120.15 | 78.11 | 0.00 |  | 0.00 |  | 78.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 119.64 | 77.60 | 0.00 |  | 0.00 |  | 77.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 115.37 | 73.32 | 0.00 |  | 0.00 |  | 73.32 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-ergon-recv-busy-10000, loop 0 on CPU 2

1941515772 iterations, longest 36.3 µs; over 5 / 20 / 100 µs: 40085 / 374 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 231 | 5554.1 | 33.1 |
| mostly unexplained (+softirq NET_RX) | 61 | 1439.1 | 34.9 |
| mostly unexplained (+preempted by kworker/2:1) | 54 | 1235.0 | 30.0 |
| mostly unexplained (+irq work (IPI/vector)) | 20 | 475.2 | 29.7 |
| loop work | 4 | 113.1 | 30.7 |
| softirq NET_RX | 2 | 42.1 | 21.6 |
| mostly unexplained (+local timer tick) | 1 | 36.3 | 36.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.9 | 21.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 36.3 | 1 | mostly unexplained (+local timer tick) | local timer tick 4, preempted by kworker/2:1 3, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 2 | 34.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 3 | 33.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 32.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 32.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 32.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 32.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 31.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 31.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 31.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 12, preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 3693 |
| LOC Local timer interrupts | 3691 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: h1/http-ergon-recv-busy-10000 (median 44.85)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 454.43 | 409.58 | 0.00 |  | 0.00 |  | 409.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 293.35 | 248.50 | 0.00 |  | 0.00 |  | 248.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 214.27 | 169.42 | 0.00 |  | 0.00 |  | 169.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 213.32 | 168.48 | 0.00 |  | 0.00 |  | 168.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 211.75 | 166.91 | 0.00 |  | 0.00 |  | 166.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 196.49 | 151.64 | 0.00 |  | 0.00 |  | 151.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 194.47 | 149.62 | 0.00 |  | 0.00 |  | 149.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 170.33 | 125.49 | 0.00 |  | 0.00 |  | 125.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 159.02 | 114.17 | 0.00 |  | 0.00 |  | 114.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 146.49 | 101.64 | 0.00 |  | 0.00 |  | 101.64 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-ergon-uring-spin-0, loop 0 on CPU 2

3295088816 iterations, longest 204.6 µs; over 5 / 20 / 100 µs: 40500 / 21 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 18 | 434.3 | 34.6 |
| softirq TIMER | 1 | 204.6 | 204.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 50.2 | 26.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 204.6 | 0 | softirq TIMER | softirq TIMER 181, local timer tick 13, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 34.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 32.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 29.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 27.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 26.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 24.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 23.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 23.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 23.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 5 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: h1/http-ergon-uring-spin-0 (median 48.23)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 239.06 | 190.83 | 0.00 |  | 0.00 |  | 190.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 223.39 | 175.17 | 0.00 |  | 0.00 |  | 175.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 221.24 | 173.02 | 204.58 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 4 | 220.23 | 172.00 | 0.00 |  | 191.07 | softirq TIMER | 0.00 | server: softirq TIMER |
| 5 | 193.51 | 145.28 | 0.00 |  | 0.00 |  | 145.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 155.29 | 107.06 | 26.45 | VM paused: the idle spare core stalled too (hypervisor) | 0.00 |  | 80.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 150.10 | 101.88 | 0.00 |  | 0.00 |  | 101.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 145.22 | 97.00 | 0.00 |  | 0.00 |  | 97.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 137.58 | 89.36 | 0.00 |  | 0.00 |  | 89.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 137.25 | 89.02 | 0.00 |  | 0.00 |  | 89.02 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-ergon-uring-spin-10000, loop 0 on CPU 2

3316194250 iterations, longest 36.0 µs; over 5 / 20 / 100 µs: 30554 / 16 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 14 | 356.6 | 36.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.0 | 29.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 36.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 34.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 29.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 29.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 27.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 26.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 25.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 24.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 23.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 23.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: h1/http-ergon-uring-spin-10000 (median 50.34)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1054.83 | 1004.48 | 0.00 |  | 0.00 |  | 1004.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 274.10 | 223.76 | 0.00 |  | 0.00 |  | 223.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 241.36 | 191.02 | 0.00 |  | 0.00 |  | 191.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 216.23 | 165.88 | 0.00 |  | 0.00 |  | 165.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 215.79 | 165.44 | 0.00 |  | 0.00 |  | 165.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 212.12 | 161.77 | 0.00 |  | 0.00 |  | 161.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 204.62 | 154.27 | 0.00 |  | 0.00 |  | 154.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 204.16 | 153.82 | 0.00 |  | 0.00 |  | 153.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 203.09 | 152.74 | 0.00 |  | 0.00 |  | 152.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 200.21 | 149.86 | 0.00 |  | 0.00 |  | 149.86 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-ergon-uring-sqpoll-0, loop 0 on CPU 2

2919997855 iterations, longest 31.8 µs; over 5 / 20 / 100 µs: 469 / 12 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 190.5 | 27.9 |
| loop work | 3 | 73.2 | 25.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 31.8 | 31.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 31.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 27.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 26.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 25.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 25.5 | 2 | loop work (2 items) |  |
| 6 | 24.1 | 2 | loop work (2 items) |  |
| 7 | 23.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 23.7 | 2 | loop work (2 items) |  |
| 9 | 22.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 22.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1007 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h1/http-ergon-uring-sqpoll-0 (median 44.27)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 213.19 | 168.91 | 0.00 |  | 0.00 |  | 168.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 193.47 | 149.19 | 0.00 |  | 0.00 |  | 149.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 179.24 | 134.97 | 0.00 |  | 0.00 |  | 134.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 176.53 | 132.26 | 0.00 |  | 0.00 |  | 132.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 170.69 | 126.41 | 0.00 |  | 0.00 |  | 126.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 167.51 | 123.24 | 0.00 |  | 0.00 |  | 123.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 165.28 | 121.00 | 0.00 |  | 0.00 |  | 121.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 164.62 | 120.35 | 0.00 |  | 0.00 |  | 120.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 163.38 | 119.11 | 0.00 |  | 0.00 |  | 119.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 163.35 | 119.07 | 0.00 |  | 0.00 |  | 119.07 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-ergon-uring-sqpoll-10000, loop 0 on CPU 2

3001806052 iterations, longest 253.3 µs; over 5 / 20 / 100 µs: 382 / 14 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 11 | 260.7 | 33.0 |
| softirq TIMER | 1 | 253.3 | 253.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 58.5 | 33.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 253.3 | 2 | softirq TIMER | softirq TIMER 230, local timer tick 12, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 33.1 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 33.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 26.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 25.3 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 24.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 24.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 23.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 22.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 22.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h1/http-ergon-uring-sqpoll-10000 (median 45.27)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 287.73 | 242.46 | 0.00 |  | 0.00 |  | 242.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 266.48 | 221.21 | 252.38 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 233.78 | 188.50 | 0.00 |  | 116.80 | softirq TIMER | 71.71 | server: softirq TIMER |
| 4 | 178.03 | 132.76 | 0.00 |  | 0.00 |  | 132.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 152.34 | 107.07 | 0.00 |  | 0.00 |  | 107.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 141.04 | 95.77 | 0.00 |  | 0.00 |  | 95.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 139.71 | 94.44 | 0.00 |  | 0.00 |  | 94.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 137.25 | 91.98 | 0.00 |  | 0.00 |  | 91.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 135.12 | 89.84 | 0.00 |  | 0.00 |  | 89.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 134.09 | 88.81 | 0.00 |  | 0.00 |  | 88.81 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h1/http-reqwest-0, loop 0 on CPU 2

496211 iterations, longest 1351.8 µs; over 5 / 20 / 100 µs: 496210 / 496210 / 460373

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 249775 | 60364015.7 | 1351.8 |
| preempted by swapper/2 | 238029 | 58750367.5 | 485.4 |
| mostly unexplained (+preempted by swapper/2) | 8406 | 751375.9 | 142.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1351.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 485.4 | 1 | preempted by swapper/2 | preempted by swapper/2 437 |
| 3 | 481.3 | 1 | preempted by swapper/2 | preempted by swapper/2 412 |
| 4 | 472.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 471.0 | 1 | preempted by swapper/2 | preempted by swapper/2 402 |
| 6 | 470.7 | 1 | preempted by swapper/2 | preempted by swapper/2 387 |
| 7 | 470.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 462.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 461.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 460.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 2196 |
| LOC Local timer interrupts | 10 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h1/http-reqwest-0 (median 245.27)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 485.10 | 239.83 | 484.92 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 2 | 480.98 | 235.72 | 480.88 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 3 | 471.73 | 226.46 | 471.59 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 470.70 | 225.43 | 470.59 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 5 | 470.37 | 225.10 | 470.25 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 6 | 470.36 | 225.09 | 470.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 462.09 | 216.82 | 461.94 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 461.06 | 215.80 | 460.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 459.65 | 214.38 | 459.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 459.42 | 214.15 | 459.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

#### Stalls: h1/http-reqwest-10000, loop 0 on CPU 2

474825 iterations, longest 1310.5 µs; over 5 / 20 / 100 µs: 474824 / 474824 / 443298

| cause | stalls | total µs | max µs |
|---|---|---|---|
| preempted by swapper/2 | 237010 | 62392590.3 | 552.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 228290 | 56630705.4 | 1310.5 |
| mostly unexplained (+preempted by swapper/2) | 9523 | 849047.2 | 114.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 244.2 | 244.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1310.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 584.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 552.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 552.5 | 1 | preempted by swapper/2 | preempted by swapper/2 484 |
| 5 | 532.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 532.2 | 1 | preempted by swapper/2 | preempted by swapper/2 465 |
| 7 | 532.2 | 1 | preempted by swapper/2 | preempted by swapper/2 464 |
| 8 | 532.2 | 1 | preempted by swapper/2 | preempted by swapper/2 464 |
| 9 | 531.8 | 1 | preempted by swapper/2 | preempted by swapper/2 463 |
| 10 | 531.6 | 1 | preempted by swapper/2 | preempted by swapper/2 463 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1665 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h1/http-reqwest-10000 (median 245.43)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 583.63 | 338.20 | 583.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 552.18 | 306.75 | 552.08 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 3 | 552.17 | 306.74 | 552.05 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 531.99 | 286.55 | 531.85 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 531.86 | 286.43 | 531.73 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 6 | 531.82 | 286.39 | 531.70 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 7 | 531.80 | 286.37 | 531.68 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 8 | 531.39 | 285.96 | 531.29 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 9 | 531.28 | 285.85 | 531.17 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 10 | 528.25 | 282.82 | 528.14 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |

#### Stalls: h2/http-api-native-recv-busy-0, loop 0 on CPU 2

197471804 iterations, longest 34.5 µs; over 5 / 20 / 100 µs: 134710 / 178 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 131 | 3010.0 | 34.5 |
| softirq NET_RX | 18 | 431.2 | 30.2 |
| mostly unexplained (+preempted by kworker/2:1) | 11 | 234.8 | 23.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 151.6 | 27.2 |
| mostly unexplained (+local timer tick) | 5 | 106.6 | 22.7 |
| mostly unexplained (+softirq NET_RX) | 4 | 100.4 | 30.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 52.2 | 26.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 23.5 | 23.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 34.5 | 2 | loop work (2 items) |  |
| 2 | 34.0 | 2 | loop work (2 items) | softirq NET_RX 4 |
| 3 | 33.5 | 2 | loop work (2 items) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 33.4 | 2 | loop work (2 items) |  |
| 5 | 33.3 | 2 | loop work (2 items) |  |
| 6 | 31.4 | 2 | loop work (2 items) | softirq NET_RX 3 |
| 7 | 31.3 | 2 | loop work (2 items) |  |
| 8 | 30.9 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 30.2 | 2 | softirq NET_RX | softirq NET_RX 20, preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 29.7 | 0 | softirq NET_RX | softirq NET_RX 16, preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4654 |
| IWI IRQ work interrupts | 4599 |
| CAL Function call interrupts | 1014 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: h2/http-api-native-recv-busy-0 (median 42.75)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1028.24 | 985.49 | 0.00 |  | 0.00 |  | 985.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1017.42 | 974.67 | 0.00 |  | 0.00 |  | 974.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 284.15 | 241.40 | 0.00 |  | 0.00 |  | 241.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 234.35 | 191.60 | 0.00 |  | 0.00 |  | 191.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 217.71 | 174.95 | 0.00 |  | 0.00 |  | 174.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 208.70 | 165.95 | 0.00 |  | 0.00 |  | 165.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 178.20 | 135.45 | 0.00 |  | 0.00 |  | 135.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 177.28 | 134.52 | 0.00 |  | 0.00 |  | 134.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 174.23 | 131.48 | 0.00 |  | 0.00 |  | 131.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 168.31 | 125.55 | 0.00 |  | 0.00 |  | 125.55 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-api-native-recv-busy-10000, loop 0 on CPU 2

1868393411 iterations, longest 324.8 µs; over 5 / 20 / 100 µs: 71673 / 286 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 102 | 2475.0 | 42.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 89 | 2115.3 | 35.9 |
| mostly unexplained (+softirq NET_RX) | 63 | 1479.3 | 41.4 |
| softirq TIMER | 1 | 324.8 | 324.8 |
| mostly unexplained (+irq work (IPI/vector)) | 11 | 275.7 | 35.6 |
| mostly unexplained (+local timer tick) | 7 | 201.6 | 30.5 |
| softirq NET_RX | 7 | 167.2 | 30.6 |
| loop work | 4 | 106.6 | 32.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 61.6 | 37.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 324.8 | 1 | softirq TIMER | softirq TIMER 313, local timer tick 6, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 42.9 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 14, local timer tick 6, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 3 | 42.5 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 41.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, local timer tick 6, timer tick_nohz_handler 3, preempted by kworker/2:1 2 |
| 5 | 39.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 37.9 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 37.2 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 7, local timer tick 3, irq work (IPI/vector) 1, softirq RCU 1 |
| 8 | 36.5 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 9 | 35.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 35.6 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, softirq NET_RX 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 963 |
| LOC Local timer interrupts | 962 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: h2/http-api-native-recv-busy-10000 (median 46.86)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 239.03 | 192.17 | 0.00 |  | 221.34 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 141.72 | 94.86 | 0.00 |  | 0.00 |  | 94.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 140.32 | 93.45 | 0.00 |  | 0.00 |  | 93.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 135.94 | 89.08 | 0.00 |  | 0.00 |  | 89.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 130.35 | 83.49 | 0.00 |  | 0.00 |  | 83.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 129.02 | 82.16 | 0.00 |  | 0.00 |  | 82.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 126.92 | 80.06 | 0.00 |  | 0.00 |  | 80.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 126.58 | 79.72 | 0.00 |  | 0.00 |  | 79.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 125.42 | 78.56 | 0.00 |  | 0.00 |  | 78.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 124.59 | 77.73 | 0.00 |  | 0.00 |  | 77.73 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-curl-distro-0, loop 0 on CPU 2

217375047 iterations, longest 1482.0 µs; over 5 / 20 / 100 µs: 2729650 / 202 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 196 | 4998.4 | 549.8 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 1558.6 | 1482.0 |
| softirq TIMER | 1 | 131.9 | 131.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 80.1 | 31.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1482.0 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 549.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 131.9 | 0 | softirq TIMER | softirq TIMER 91, local timer tick 14, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 4 | 76.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 41.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 35.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 32.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 32.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 31.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 31.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1000 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: h2/http-curl-distro-0 (median 61.89)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 296.39 | 234.49 | 0.00 |  | 0.00 |  | 234.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 269.47 | 207.57 | 0.00 |  | 0.00 |  | 207.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 240.45 | 178.55 | 0.00 |  | 226.93 | softirq TIMER | 0.00 | server: softirq TIMER |
| 4 | 219.07 | 157.18 | 0.00 |  | 0.00 |  | 157.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 211.54 | 149.65 | 0.00 |  | 0.00 |  | 149.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 209.79 | 147.90 | 0.00 |  | 0.00 |  | 147.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 199.28 | 137.39 | 0.00 |  | 0.00 |  | 137.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 198.51 | 136.62 | 0.00 |  | 0.00 |  | 136.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 197.68 | 135.78 | 0.00 |  | 0.00 |  | 135.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 195.81 | 133.91 | 0.00 |  | 0.00 |  | 133.91 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-curl-distro-10000, loop 0 on CPU 2

1238218681 iterations, longest 1428.1 µs; over 5 / 20 / 100 µs: 1447171 / 44 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+irq work (IPI/vector)) | 2 | 1500.8 | 1428.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 40 | 1485.9 | 554.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 46.7 | 24.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1428.1 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 554.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 72.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 33.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 31.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 31.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 30.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 26.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 26.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 25.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| LOC Local timer interrupts | 5 |
| IWI IRQ work interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h2/http-curl-distro-10000 (median 61.47)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 254.35 | 192.87 | 0.00 |  | 0.00 |  | 192.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 235.71 | 174.23 | 0.00 |  | 0.00 |  | 174.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 235.58 | 174.11 | 0.00 |  | 0.00 |  | 174.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 217.62 | 156.14 | 0.00 |  | 0.00 |  | 156.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 216.89 | 155.41 | 0.00 |  | 0.00 |  | 155.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 214.63 | 153.16 | 0.00 |  | 0.00 |  | 153.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 204.27 | 142.80 | 0.00 |  | 0.00 |  | 142.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 202.20 | 140.72 | 0.00 |  | 0.00 |  | 140.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 201.94 | 140.47 | 0.00 |  | 0.00 |  | 140.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 201.01 | 139.54 | 0.00 |  | 0.00 |  | 139.54 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-curl-source-0, loop 0 on CPU 2

110820487 iterations, longest 865.6 µs; over 5 / 20 / 100 µs: 2468590 / 171 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 166 | 3765.9 | 63.2 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 1463.9 | 865.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 63.7 | 35.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 865.6 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 500.0 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 98.3 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 63.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 37.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 36.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 35.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 34.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 31.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h2/http-curl-source-0 (median 54.31)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 269.40 | 215.09 | 0.00 |  | 0.00 |  | 215.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 255.82 | 201.51 | 0.00 |  | 25.96 | unexplained: no kernel event (hypervisor or in-kernel time) | 175.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 163.90 | 109.59 | 0.00 |  | 0.00 |  | 109.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 148.08 | 93.77 | 0.00 |  | 0.00 |  | 93.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 146.16 | 91.85 | 0.00 |  | 0.00 |  | 91.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 140.00 | 85.69 | 0.00 |  | 0.00 |  | 85.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 139.94 | 85.63 | 0.00 |  | 0.00 |  | 85.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 136.68 | 82.37 | 0.00 |  | 0.00 |  | 82.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 135.24 | 80.93 | 0.00 |  | 0.00 |  | 80.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 133.30 | 78.99 | 0.00 |  | 0.00 |  | 78.99 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-curl-source-10000, loop 0 on CPU 2

1502100340 iterations, longest 863.8 µs; over 5 / 20 / 100 µs: 1276152 / 107 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 101 | 2370.6 | 38.0 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 1453.3 | 863.8 |
| softirq TIMER | 1 | 167.5 | 167.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 51.8 | 26.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 863.8 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 496.3 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 167.5 | 0 | softirq TIMER | softirq TIMER 136, local timer tick 16, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 4 | 93.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 38.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 37.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 31.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 28.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 28.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 27.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h2/http-curl-source-10000 (median 56.53)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 345.45 | 288.92 | 0.00 |  | 0.00 |  | 288.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 251.11 | 194.59 | 0.00 |  | 233.88 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 228.89 | 172.36 | 167.46 | softirq TIMER | 0.00 |  | 4.90 | client: softirq TIMER |
| 4 | 157.30 | 100.77 | 0.00 |  | 0.00 |  | 100.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 136.62 | 80.09 | 0.00 |  | 0.00 |  | 80.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 134.38 | 77.86 | 0.00 |  | 0.00 |  | 77.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 133.82 | 77.30 | 0.00 |  | 44.40 | mostly unexplained (+preempted by kworker/1:1) | 32.89 | server: mostly unexplained (+preempted by kworker/1:1) |
| 8 | 132.81 | 76.28 | 0.00 |  | 0.00 |  | 76.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 131.12 | 74.60 | 0.00 |  | 0.00 |  | 74.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 130.56 | 74.03 | 0.00 |  | 0.00 |  | 74.03 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-ergon-epoll-spin-0, loop 0 on CPU 2

526802942 iterations, longest 116.0 µs; over 5 / 20 / 100 µs: 34610 / 26 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 12 | 315.1 | 58.4 |
| loop work | 11 | 246.5 | 27.6 |
| softirq TIMER | 1 | 116.0 | 116.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 59.7 | 35.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 116.0 | 2 | softirq TIMER | softirq TIMER 88, local timer tick 14, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 58.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 35.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 27.6 | 2 | loop work (2 items) |  |
| 5 | 26.7 | 2 | loop work (2 items) |  |
| 6 | 25.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 25.3 | 2 | loop work (2 items) |  |
| 8 | 24.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 24.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 24.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h2/http-ergon-epoll-spin-0 (median 48.92)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 193.86 | 144.94 | 0.00 |  | 0.00 |  | 144.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 170.30 | 121.38 | 0.00 |  | 0.00 |  | 121.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 167.23 | 118.31 | 0.00 |  | 0.00 |  | 118.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 163.31 | 114.39 | 0.00 |  | 0.00 |  | 114.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 160.17 | 111.25 | 0.00 |  | 0.00 |  | 111.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 159.88 | 110.96 | 0.00 |  | 73.03 | softirq TIMER | 37.93 | server: softirq TIMER |
| 7 | 156.24 | 107.32 | 115.10 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 8 | 151.40 | 102.48 | 0.00 |  | 0.00 |  | 102.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 149.94 | 101.02 | 0.00 |  | 0.00 |  | 101.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 149.85 | 100.92 | 0.00 |  | 0.00 |  | 100.92 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-ergon-epoll-spin-10000, loop 0 on CPU 2

1851570016 iterations, longest 32.2 µs; over 5 / 20 / 100 µs: 26326 / 21 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 14 | 335.7 | 30.1 |
| loop work | 6 | 125.9 | 22.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 32.2 | 32.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 32.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 30.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 26.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 25.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 25.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 24.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 23.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 23.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 23.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 23.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h2/http-ergon-epoll-spin-10000 (median 49.37)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1013.67 | 964.30 | 0.00 |  | 0.00 |  | 964.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 212.57 | 163.20 | 0.00 |  | 0.00 |  | 163.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 211.25 | 161.89 | 0.00 |  | 0.00 |  | 161.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 201.00 | 151.63 | 0.00 |  | 0.00 |  | 151.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 157.65 | 108.28 | 0.00 |  | 0.00 |  | 108.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 156.16 | 106.80 | 0.00 |  | 0.00 |  | 106.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 155.76 | 106.39 | 0.00 |  | 0.00 |  | 106.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 152.68 | 103.32 | 0.00 |  | 0.00 |  | 103.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 148.02 | 98.66 | 0.00 |  | 0.00 |  | 98.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 143.62 | 94.26 | 0.00 |  | 0.00 |  | 94.26 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-ergon-recv-busy-0, loop 0 on CPU 2

210257323 iterations, longest 48.1 µs; over 5 / 20 / 100 µs: 62762 / 128 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 86 | 1996.0 | 32.1 |
| mostly unexplained (+preempted by kworker/2:1) | 12 | 264.3 | 28.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 10 | 261.1 | 42.4 |
| softirq NET_RX | 8 | 180.8 | 29.8 |
| mostly unexplained (+softirq NET_RX) | 6 | 163.3 | 48.1 |
| mostly unexplained (+local timer tick) | 4 | 89.9 | 23.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 58.9 | 30.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 48.1 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2 |
| 2 | 42.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 32.1 | 2 | loop work (2 items) |  |
| 4 | 31.0 | 2 | loop work (2 items) |  |
| 5 | 30.9 | 2 | loop work (2 items) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 30.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 30.1 | 2 | loop work (2 items) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 30.0 | 2 | loop work (2 items) | softirq NET_RX 10, preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 29.8 | 0 | softirq NET_RX | softirq NET_RX 16, preempted by kworker/2:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 29.6 | 2 | loop work (2 items) | softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4501 |
| IWI IRQ work interrupts | 4463 |
| CAL Function call interrupts | 999 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h2/http-ergon-recv-busy-0 (median 42.21)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 153.91 | 111.70 | 0.00 |  | 0.00 |  | 111.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 145.85 | 103.64 | 0.00 |  | 0.00 |  | 103.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 132.40 | 90.19 | 0.00 |  | 0.00 |  | 90.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 124.97 | 82.75 | 0.00 |  | 0.00 |  | 82.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 122.83 | 80.62 | 0.00 |  | 0.00 |  | 80.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 122.41 | 80.19 | 0.00 |  | 0.00 |  | 80.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 120.19 | 77.98 | 0.00 |  | 0.00 |  | 77.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 117.10 | 74.89 | 0.00 |  | 0.00 |  | 74.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 114.95 | 72.73 | 0.00 |  | 0.00 |  | 72.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 110.88 | 68.67 | 0.00 |  | 0.00 |  | 68.67 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-ergon-recv-busy-10000, loop 0 on CPU 2

1956199140 iterations, longest 40.4 µs; over 5 / 20 / 100 µs: 59508 / 379 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 243 | 5550.5 | 40.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 42 | 976.4 | 34.6 |
| mostly unexplained (+softirq NET_RX) | 41 | 973.4 | 38.6 |
| softirq NET_RX | 29 | 660.6 | 28.7 |
| mostly unexplained (+irq work (IPI/vector)) | 21 | 468.0 | 29.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 63.7 | 32.6 |
| loop work | 1 | 38.6 | 38.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 40.4 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 2 | 38.6 | 2 | loop work (2 items) |  |
| 3 | 38.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 19, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 35.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 16, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 34.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 33.5 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 32.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 32.0 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 31.7 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 31.5 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2732 |
| IWI IRQ work interrupts | 2729 |
| CAL Function call interrupts | 999 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h2/http-ergon-recv-busy-10000 (median 44.56)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 226.04 | 181.48 | 0.00 |  | 0.00 |  | 181.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 201.38 | 156.82 | 0.00 |  | 0.00 |  | 156.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 198.78 | 154.21 | 0.00 |  | 0.00 |  | 154.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 197.48 | 152.92 | 0.00 |  | 0.00 |  | 152.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 193.35 | 148.79 | 0.00 |  | 0.00 |  | 148.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 182.39 | 137.83 | 0.00 |  | 0.00 |  | 137.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 177.01 | 132.44 | 0.00 |  | 0.00 |  | 132.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 171.22 | 126.66 | 0.00 |  | 0.00 |  | 126.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 167.17 | 122.61 | 0.00 |  | 0.00 |  | 122.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 166.50 | 121.94 | 0.00 |  | 0.00 |  | 121.94 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-ergon-uring-spin-0, loop 0 on CPU 2

3301982633 iterations, longest 36.7 µs; over 5 / 20 / 100 µs: 50973 / 22 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 19 | 441.9 | 32.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 66.5 | 36.7 |
| loop work | 1 | 25.4 | 25.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 36.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 32.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 32.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 29.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 29.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 25.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 25.4 | 2 | loop work (2 items) |  |
| 8 | 23.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 23.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 22.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h2/http-ergon-uring-spin-0 (median 48.42)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 237.88 | 189.46 | 0.00 |  | 0.00 |  | 189.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 223.71 | 175.28 | 0.00 |  | 0.00 |  | 175.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 210.25 | 161.83 | 0.00 |  | 0.00 |  | 161.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 204.02 | 155.60 | 0.00 |  | 0.00 |  | 155.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 202.31 | 153.89 | 0.00 |  | 0.00 |  | 153.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 199.93 | 151.51 | 0.00 |  | 0.00 |  | 151.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 198.82 | 150.40 | 0.00 |  | 25.19 | softirq NET_RX | 125.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 187.08 | 138.65 | 0.00 |  | 0.00 |  | 138.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 186.88 | 138.46 | 0.00 |  | 0.00 |  | 138.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 186.30 | 137.87 | 0.00 |  | 0.00 |  | 137.87 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-ergon-uring-spin-10000, loop 0 on CPU 2

3323291175 iterations, longest 243.5 µs; over 5 / 20 / 100 µs: 6970 / 17 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 14 | 334.9 | 31.1 |
| softirq TIMER | 1 | 243.5 | 243.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 57.8 | 34.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 243.5 | 0 | softirq TIMER | softirq TIMER 212, local timer tick 18, timer tick_nohz_handler 10, irq work (IPI/vector) 1 |
| 2 | 34.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 31.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 27.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 27.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 26.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 23.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 23.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 23.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 23.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h2/http-ergon-uring-spin-10000 (median 49.48)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 976.73 | 927.25 | 0.00 |  | 0.00 |  | 927.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 291.25 | 241.78 | 243.48 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 268.84 | 219.36 | 0.00 |  | 218.48 | softirq TIMER | 0.88 | server: softirq TIMER |
| 4 | 139.71 | 90.23 | 0.00 |  | 0.00 |  | 90.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 138.37 | 88.89 | 0.00 |  | 0.00 |  | 88.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 133.95 | 84.47 | 0.00 |  | 0.00 |  | 84.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 126.63 | 77.15 | 0.00 |  | 26.55 | mostly unexplained (+preempted by kworker/1:1) | 50.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 126.50 | 77.02 | 0.00 |  | 0.00 |  | 77.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 120.46 | 70.98 | 0.00 |  | 26.95 | mostly unexplained (+local timer tick) | 44.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 120.28 | 70.80 | 0.00 |  | 0.00 |  | 70.80 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-ergon-uring-sqpoll-0, loop 0 on CPU 2

2700708269 iterations, longest 298.7 µs; over 5 / 20 / 100 µs: 536 / 14 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 298.7 | 298.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 7 | 157.7 | 23.7 |
| loop work | 3 | 80.4 | 35.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 69.9 | 25.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 298.7 | 0 | softirq TIMER | softirq TIMER 279, local timer tick 11, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 35.5 | 2 | loop work (2 items) |  |
| 3 | 25.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 23.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 23.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 23.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 23.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 23.1 | 2 | loop work (2 items) |  |
| 9 | 23.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h2/http-ergon-uring-sqpoll-0 (median 43.87)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 439.38 | 395.51 | 0.00 |  | 0.00 |  | 395.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 323.11 | 279.25 | 0.00 |  | 0.00 |  | 279.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 195.02 | 151.16 | 0.00 |  | 0.00 |  | 151.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 189.24 | 145.37 | 0.00 |  | 0.00 |  | 145.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 184.80 | 140.94 | 0.00 |  | 0.00 |  | 140.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 172.10 | 128.24 | 0.00 |  | 0.00 |  | 128.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 170.82 | 126.95 | 0.00 |  | 0.00 |  | 126.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 163.95 | 120.08 | 0.00 |  | 0.00 |  | 120.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 162.37 | 118.50 | 0.00 |  | 0.00 |  | 118.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 156.76 | 112.89 | 0.00 |  | 0.00 |  | 112.89 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-ergon-uring-sqpoll-10000, loop 0 on CPU 2

2839507631 iterations, longest 33.1 µs; over 5 / 20 / 100 µs: 400 / 10 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 215.1 | 33.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 25.1 | 25.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 33.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 25.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 24.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 23.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 23.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 23.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 22.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 22.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 21.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: h2/http-ergon-uring-sqpoll-10000 (median 46.03)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 180.91 | 134.88 | 0.00 |  | 0.00 |  | 134.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 135.83 | 89.80 | 0.00 |  | 0.00 |  | 89.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 133.06 | 87.03 | 0.00 |  | 0.00 |  | 87.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 123.60 | 77.57 | 0.00 |  | 0.00 |  | 77.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 121.62 | 75.59 | 0.00 |  | 0.00 |  | 75.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 121.28 | 75.25 | 0.00 |  | 0.00 |  | 75.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 116.67 | 70.64 | 0.00 |  | 0.00 |  | 70.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 115.95 | 69.92 | 0.00 |  | 0.00 |  | 69.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 115.60 | 69.58 | 0.00 |  | 0.00 |  | 69.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 114.94 | 68.91 | 0.00 |  | 0.00 |  | 68.91 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: h2/http-reqwest-0, loop 0 on CPU 2

494688 iterations, longest 1493.5 µs; over 5 / 20 / 100 µs: 494687 / 494687 / 461962

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 248288 | 60110180.0 | 1493.5 |
| preempted by swapper/2 | 237958 | 59001399.5 | 491.6 |
| mostly unexplained (+preempted by swapper/2) | 8440 | 760486.8 | 122.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 276.0 | 276.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1493.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 497.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 491.6 | 1 | preempted by swapper/2 | preempted by swapper/2 422 |
| 4 | 488.6 | 1 | preempted by swapper/2 | preempted by swapper/2 438 |
| 5 | 487.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 484.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 478.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 474.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 474.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 471.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 2090 |
| LOC Local timer interrupts | 11 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: h2/http-reqwest-0 (median 245.30)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 497.45 | 252.15 | 497.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 491.33 | 246.04 | 491.22 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 3 | 488.40 | 243.10 | 488.24 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 4 | 487.15 | 241.86 | 487.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 484.15 | 238.85 | 484.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 478.57 | 233.28 | 478.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 473.94 | 228.65 | 473.79 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 473.72 | 228.42 | 473.59 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 471.30 | 226.00 | 471.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 471.21 | 225.91 | 471.04 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |

#### Stalls: h2/http-reqwest-10000, loop 0 on CPU 2

495323 iterations, longest 1507.9 µs; over 5 / 20 / 100 µs: 495322 / 495322 / 454187

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 248761 | 60203398.2 | 1507.9 |
| preempted by swapper/2 | 236640 | 58792359.4 | 474.8 |
| mostly unexplained (+preempted by swapper/2) | 9921 | 876015.7 | 116.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1507.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 483.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 478.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 474.8 | 1 | preempted by swapper/2 | preempted by swapper/2 425 |
| 5 | 473.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 471.5 | 1 | preempted by swapper/2 | preempted by swapper/2 423 |
| 7 | 470.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 470.7 | 1 | preempted by swapper/2 | preempted by swapper/2 403 |
| 9 | 470.5 | 1 | preempted by swapper/2 | preempted by swapper/2 403 |
| 10 | 470.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1669 |
| LOC Local timer interrupts | 19 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: h2/http-reqwest-10000 (median 245.37)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 482.73 | 237.36 | 482.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 478.30 | 232.93 | 478.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 474.49 | 229.12 | 474.31 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 4 | 473.14 | 227.77 | 473.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 471.20 | 225.83 | 471.03 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 6 | 470.34 | 224.97 | 470.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 470.30 | 224.93 | 470.21 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 8 | 470.16 | 224.79 | 470.06 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 9 | 469.97 | 224.60 | 469.89 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 469.88 | 224.51 | 469.77 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

## Ping-pong round trip (client send → echo decoded; "from when due": from the slot on the paced schedule, including any wait behind a stall)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| smoke/http-api-native-recv-busy: client-native/recv-busy @ 1000/s | 1994 | 45.92 | 45.49 | 47.93 | 65.83 | 80.92 | 81.29 | 81.29 |
| smoke/http-api-native-recv-busy: client-native/recv-busy @ 1000/s, from when due | 1994 | 46.11 | 45.68 | 48.12 | 66.00 | 81.12 | 81.43 | 81.43 |
| smoke/http-curl-distro: curl-distro/8.5.0 @ 1000/s | 1992 | 86.23 | 89.16 | 96.21 | 138.96 | 186.54 | 200.14 | 200.14 |
| smoke/http-curl-distro: curl-distro/8.5.0 @ 1000/s, from when due | 1992 | 86.52 | 89.39 | 96.45 | 139.21 | 186.80 | 200.34 | 200.34 |
| smoke/http-curl-source: curl-source/8.23.0 @ 1000/s | 1992 | 58.98 | 58.42 | 61.49 | 85.80 | 99.47 | 101.63 | 101.63 |
| smoke/http-curl-source: curl-source/8.23.0 @ 1000/s, from when due | 1992 | 59.32 | 58.73 | 61.91 | 86.06 | 99.73 | 102.19 | 102.19 |
| smoke/http-ergon-epoll-spin: ergon/epoll-spin @ 1000/s | 1994 | 52.17 | 48.59 | 58.66 | 97.11 | 100.84 | 104.73 | 104.73 |
| smoke/http-ergon-epoll-spin: ergon/epoll-spin @ 1000/s, from when due | 1994 | 52.37 | 48.79 | 58.81 | 97.42 | 101.01 | 105.03 | 105.03 |
| smoke/http-ergon-recv-busy: ergon/recv-busy @ 1000/s | 1994 | 46.01 | 45.32 | 48.18 | 66.31 | 74.44 | 74.63 | 74.63 |
| smoke/http-ergon-recv-busy: ergon/recv-busy @ 1000/s, from when due | 1994 | 46.21 | 45.55 | 48.37 | 66.49 | 74.59 | 74.81 | 74.81 |
| smoke/http-ergon-uring-spin: ergon/uring-spin @ 1000/s | 1994 | 52.83 | 52.35 | 54.78 | 77.03 | 94.22 | 95.45 | 95.45 |
| smoke/http-ergon-uring-spin: ergon/uring-spin @ 1000/s, from when due | 1994 | 53.01 | 52.53 | 54.94 | 77.19 | 94.40 | 95.60 | 95.60 |
| smoke/http-ergon-uring-sqpoll: ergon/uring-sqpoll @ 1000/s | 1994 | 47.63 | 47.15 | 49.54 | 67.01 | 77.20 | 77.95 | 77.95 |
| smoke/http-ergon-uring-sqpoll: ergon/uring-sqpoll @ 1000/s, from when due | 1994 | 47.81 | 47.31 | 49.75 | 67.16 | 77.33 | 78.08 | 78.08 |
| smoke/http-reqwest: reqwest/tokio-current-thread @ 1000/s | 1993 | 272.54 | 248.04 | 337.52 | 415.53 | 464.99 | 469.93 | 469.93 |
| smoke/http-reqwest: reqwest/tokio-current-thread @ 1000/s, from when due | 1993 | 272.61 | 248.10 | 337.59 | 415.61 | 465.06 | 470.00 | 470.00 |
| h1/http-api-native-recv-busy-0: client-native/recv-busy @ 0/s | 1976874 | 43.34 | 42.98 | 45.92 | 50.76 | 59.34 | 72.24 | 349.71 |
| h1/http-api-native-recv-busy-10000: client-native/recv-busy @ 10000/s | 1189935 | 47.15 | 46.48 | 50.64 | 58.03 | 73.66 | 106.89 | 321.39 |
| h1/http-api-native-recv-busy-10000: client-native/recv-busy @ 10000/s, from when due | 1189935 | 47.34 | 46.66 | 50.82 | 58.26 | 74.09 | 119.71 | 450.56 |
| h1/http-curl-distro-0: curl-distro/8.5.0 @ 0/s | 1983571 | 59.21 | 58.12 | 64.00 | 75.10 | 96.44 | 108.48 | 209.20 |
| h1/http-curl-distro-10000: curl-distro/8.5.0 @ 10000/s | 1189908 | 59.30 | 58.51 | 61.62 | 76.80 | 106.99 | 116.06 | 239.90 |
| h1/http-curl-distro-10000: curl-distro/8.5.0 @ 10000/s, from when due | 1189908 | 59.53 | 58.71 | 61.93 | 77.22 | 108.45 | 117.09 | 267.21 |
| h1/http-curl-source-0: curl-source/8.23.0 @ 0/s | 1982338 | 55.89 | 55.17 | 59.30 | 67.68 | 95.36 | 101.92 | 152.54 |
| h1/http-curl-source-10000: curl-source/8.23.0 @ 10000/s | 1189917 | 57.66 | 55.99 | 59.75 | 100.78 | 109.72 | 121.04 | 262.11 |
| h1/http-curl-source-10000: curl-source/8.23.0 @ 10000/s, from when due | 1189917 | 57.97 | 56.26 | 60.23 | 101.16 | 110.20 | 124.74 | 262.36 |
| h1/http-ergon-epoll-spin-0: ergon/epoll-spin @ 0/s | 1980506 | 51.31 | 48.96 | 51.57 | 96.85 | 103.82 | 109.81 | 226.33 |
| h1/http-ergon-epoll-spin-10000: ergon/epoll-spin @ 10000/s | 1189937 | 49.88 | 48.64 | 52.86 | 90.37 | 95.48 | 102.28 | 340.29 |
| h1/http-ergon-epoll-spin-10000: ergon/epoll-spin @ 10000/s, from when due | 1189937 | 50.03 | 48.80 | 53.02 | 90.53 | 95.67 | 104.16 | 379.30 |
| h1/http-ergon-recv-busy-0: ergon/recv-busy @ 0/s | 1976339 | 42.41 | 42.04 | 45.04 | 50.30 | 58.55 | 71.31 | 287.81 |
| h1/http-ergon-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189937 | 45.21 | 44.85 | 47.37 | 53.64 | 66.93 | 91.62 | 454.43 |
| h1/http-ergon-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189937 | 45.40 | 44.99 | 47.52 | 53.81 | 67.17 | 107.62 | 1057.29 |
| h1/http-ergon-uring-spin-0: ergon/uring-spin @ 0/s | 1980244 | 49.78 | 48.23 | 52.82 | 95.78 | 98.34 | 106.47 | 239.06 |
| h1/http-ergon-uring-spin-10000: ergon/uring-spin @ 10000/s | 1189936 | 53.98 | 50.34 | 58.92 | 96.97 | 102.10 | 152.19 | 1054.83 |
| h1/http-ergon-uring-spin-10000: ergon/uring-spin @ 10000/s, from when due | 1189936 | 54.16 | 50.50 | 59.13 | 97.16 | 103.96 | 157.72 | 1054.96 |
| h1/http-ergon-uring-sqpoll-0: ergon/uring-sqpoll @ 0/s | 1977595 | 44.62 | 44.27 | 47.14 | 51.87 | 60.63 | 77.80 | 213.19 |
| h1/http-ergon-uring-sqpoll-10000: ergon/uring-sqpoll @ 10000/s | 1189938 | 45.60 | 45.27 | 47.68 | 53.20 | 67.93 | 84.26 | 287.73 |
| h1/http-ergon-uring-sqpoll-10000: ergon/uring-sqpoll @ 10000/s, from when due | 1189938 | 45.76 | 45.42 | 47.83 | 53.37 | 68.12 | 85.74 | 448.04 |
| h1/http-reqwest-0: reqwest/tokio-current-thread @ 0/s | 492099 | 241.23 | 245.27 | 296.17 | 351.10 | 390.48 | 425.49 | 485.10 |
| h1/http-reqwest-10000: reqwest/tokio-current-thread @ 10000/s | 470755 | 252.16 | 245.43 | 297.27 | 409.13 | 460.25 | 490.14 | 583.63 |
| h1/http-reqwest-10000: reqwest/tokio-current-thread @ 10000/s, from when due | 470755 | 35933099.68 | 35610754.81 | 65183777.25 | 71797778.31 | 72448489.63 | 72510739.93 | 72517805.74 |
| h2/http-api-native-recv-busy-0: client-native/recv-busy @ 0/s | 1976787 | 43.12 | 42.75 | 45.68 | 50.32 | 59.19 | 89.26 | 1028.24 |
| h2/http-api-native-recv-busy-10000: client-native/recv-busy @ 10000/s | 1189935 | 47.43 | 46.86 | 50.94 | 56.02 | 65.59 | 97.45 | 239.03 |
| h2/http-api-native-recv-busy-10000: client-native/recv-busy @ 10000/s, from when due | 1189935 | 47.58 | 47.01 | 51.09 | 56.19 | 65.98 | 99.46 | 340.23 |
| h2/http-curl-distro-0: curl-distro/8.5.0 @ 0/s | 1905900 | 62.15 | 61.89 | 66.72 | 75.40 | 98.20 | 129.94 | 296.39 |
| h2/http-curl-distro-10000: curl-distro/8.5.0 @ 10000/s | 1189911 | 67.76 | 61.47 | 92.43 | 101.11 | 106.43 | 163.28 | 254.35 |
| h2/http-curl-distro-10000: curl-distro/8.5.0 @ 10000/s, from when due | 1189911 | 68.09 | 61.72 | 92.70 | 102.89 | 122.04 | 172.43 | 254.55 |
| h2/http-curl-source-0: curl-source/8.23.0 @ 0/s | 1981759 | 55.26 | 54.31 | 59.33 | 68.61 | 96.12 | 102.54 | 269.40 |
| h2/http-curl-source-10000: curl-source/8.23.0 @ 10000/s | 1189914 | 57.32 | 56.53 | 60.05 | 73.83 | 101.67 | 111.19 | 345.45 |
| h2/http-curl-source-10000: curl-source/8.23.0 @ 10000/s, from when due | 1189914 | 57.60 | 56.79 | 60.34 | 74.26 | 102.03 | 113.25 | 345.71 |
| h2/http-ergon-epoll-spin-0: ergon/epoll-spin @ 0/s | 1980380 | 50.69 | 48.92 | 52.50 | 96.53 | 99.87 | 108.40 | 193.86 |
| h2/http-ergon-epoll-spin-10000: ergon/epoll-spin @ 10000/s | 1189936 | 53.42 | 49.37 | 58.97 | 96.22 | 106.80 | 116.36 | 1013.67 |
| h2/http-ergon-epoll-spin-10000: ergon/epoll-spin @ 10000/s, from when due | 1189936 | 53.59 | 49.52 | 59.23 | 96.44 | 106.98 | 120.80 | 1013.81 |
| h2/http-ergon-recv-busy-0: ergon/recv-busy @ 0/s | 1976472 | 42.60 | 42.21 | 45.29 | 50.21 | 58.39 | 70.22 | 153.91 |
| h2/http-ergon-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189936 | 44.92 | 44.56 | 47.30 | 53.06 | 62.82 | 95.72 | 226.04 |
| h2/http-ergon-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189936 | 45.09 | 44.73 | 47.47 | 53.26 | 63.44 | 111.31 | 293.18 |
| h2/http-ergon-uring-spin-0: ergon/uring-spin @ 0/s | 1980769 | 50.44 | 48.42 | 53.89 | 96.14 | 99.03 | 109.78 | 237.88 |
| h2/http-ergon-uring-spin-10000: ergon/uring-spin @ 10000/s | 1189936 | 50.66 | 49.48 | 52.60 | 92.62 | 97.43 | 111.00 | 976.73 |
| h2/http-ergon-uring-spin-10000: ergon/uring-spin @ 10000/s, from when due | 1189936 | 50.82 | 49.62 | 52.76 | 92.77 | 97.63 | 111.72 | 976.86 |
| h2/http-ergon-uring-sqpoll-0: ergon/uring-sqpoll @ 0/s | 1977508 | 44.23 | 43.87 | 46.87 | 51.46 | 59.73 | 83.48 | 439.38 |
| h2/http-ergon-uring-sqpoll-10000: ergon/uring-sqpoll @ 10000/s | 1189937 | 46.34 | 46.03 | 48.51 | 53.61 | 61.11 | 75.96 | 180.91 |
| h2/http-ergon-uring-sqpoll-10000: ergon/uring-sqpoll @ 10000/s, from when due | 1189937 | 46.51 | 46.19 | 48.68 | 53.78 | 61.32 | 76.28 | 183.58 |
| h2/http-reqwest-0: reqwest/tokio-current-thread @ 0/s | 490553 | 242.02 | 245.30 | 296.09 | 349.24 | 392.54 | 429.82 | 497.45 |
| h2/http-reqwest-10000: reqwest/tokio-current-thread @ 10000/s | 491191 | 241.67 | 245.37 | 296.12 | 352.89 | 390.39 | 430.02 | 482.73 |
| h2/http-reqwest-10000: reqwest/tokio-current-thread @ 10000/s, from when due | 491191 | 35548309.24 | 35531844.65 | 63496451.59 | 69777690.43 | 70399012.04 | 70460509.94 | 70467867.04 |

