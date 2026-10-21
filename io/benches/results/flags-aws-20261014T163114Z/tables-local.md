# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|
| r1-base-0 | 26.77 | 33.5 | 0.5 | 0.0 | 26.77 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-housekeeping | 30.58 | 38.0 | 1.0 | 0.0 | 30.58 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-prefer | 33.10 | 25.5 | 1.5 | 0.0 | 33.10 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-2 | 37.85 | 31.5 | 1.5 | 0.0 | 37.85 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-noquickack | 140.90 | 38.5 | 2.5 | 1.0 | 140.90 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nogro | 164.47 | 26.0 | 2.5 | 2.0 | 164.47 µs stall: softirq TIMER |
| r1-base-4 | 128.10 | 24.5 | 2.0 | 1.5 | 128.10 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nocoalesce | 142.72 | 25.5 | 2.5 | 1.5 | 142.72 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-irqlocal | 146.40 | 411.0 | 35.5 | 1.5 | 146.40 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-6 | 3813.30 | 43.0 | 3.0 | 2.0 | 3813.30 µs stall: mostly unexplained (+softirq NET_RX) |
| r1-rt | 53.12 | 40.0 | 2.0 | 0.0 | 53.12 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-sibling | 328.24 | 29.0 | 4.0 | 1.0 | 328.24 µs stall: softirq TIMER |
| r1-base-8 | 139.89 | 30.5 | 2.0 | 1.0 | 139.89 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-busypoll50 | 255.03 | 43.5 | 4.0 | 2.0 | 255.03 µs stall: softirq TIMER |
| r1-budget64 | 122.67 | 31.0 | 4.0 | 2.0 | 122.67 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 121.99 | 31.5 | 3.5 | 1.5 | 121.99 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-cstate | 304.11 | 36.0 | 4.0 | 1.5 | 304.11 µs stall: softirq TIMER |
| r1-base-11 | 347.61 | 37.5 | 4.5 | 2.0 | 347.61 µs stall: softirq TIMER |
| boot-stock | 146.95 | 60120.0 | 120.0 | 34.5 | 146.95 µs stall: softirq TIMER |
| boot-isolcpus | 52.30 | 60063.5 | 4.0 | 0.0 | 52.30 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full-nomitig | 57.67 | 39.0 | 2.0 | 0.0 | 57.67 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full | 133.90 | 50.0 | 4.5 | 1.0 | 133.90 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |

## HTTP round trip, http-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189938 | 46.37 | 61.96 | 75.61 | 110.51 | 338.88 | 74.1 | 0.0 | 0.0 | 0.0 | 129.17 | 2194.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1189937 | 48.11 (+1.92) | 62.60 (+2.51) | 78.88 (+1.80) | 121.61 (+11.10) | 1051.57 (+712.69) | 116.0 | 1.0 | 1.0 | 0.0 | 256.60 | 849.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 1189938 | 49.13 (+2.93) | 62.46 (+2.37) | 77.00 (−0.07) | 91.58 (−18.93) | 231.76 (−107.12) | 30.8 | 0.0 | 0.0 | 0.0 | 117.26 | 50.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1189938 | 46.04 | 57.97 | 68.92 | 85.43 | 160.64 | 12.6 | 0.0 | 0.0 | 0.0 | 147.51 | 1436.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 1189939 | 46.91 (+0.72) | 286.27 (+226.18) | 321.46 (+244.38) | 350.64 (+240.13) | 394.24 (+55.35) | 127212.3 | 0.0 | 0.0 | 0.0 | 266.20 | 556.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 1189938 | 47.80 (+1.60) | 92.50 (+32.41) | 228.13 (+151.06) | 287.31 (+176.80) | 363.37 (+24.49) | 1444.6 | 0.0 | 0.0 | 0.0 | 149.56 | 590.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1189938 | 48.05 | 64.75 | 83.26 | 123.14 | 984.67 | 151.8 | 0.5 | 0.0 | 0.0 | 150.56 | 844.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 1189939 | 47.39 (+1.20) | 89.96 (+29.86) | 95.71 (+18.64) | 118.65 (+8.14) | 191.63 (−147.25) | 183.0 | 0.0 | 0.0 | 0.0 | 155.95 | 40.5 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-irqlocal | 1189936 | 48.93 (+2.74) | 241.11 (+181.01) | 297.04 (+219.96) | 335.89 (+225.38) | 371.58 (+32.70) | 22007.1 | 0.0 | 0.0 | 0.0 | 152.71 | 2673.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1189937 | 46.19 | 88.47 | 94.99 | 111.64 | 510.88 | 224.9 | 0.5 | 0.0 | 0.0 | 151.22 | 385.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 1189938 | 45.42 (−0.77) | 55.17 (−4.92) | 65.54 (−11.54) | 103.19 (−7.31) | 231.15 (−107.73) | 68.1 | 0.0 | 0.0 | 0.0 | 254.67 | 93.0 | server: softirq TIMER |
| r1-sibling | 1189917 | 47.92 (+1.72) | 61.01 (+0.92) | 75.91 (−1.16) | 227.03 (+116.52) | 1054.43 (+715.55) | 156.3 | 0.5 | 0.5 | 0.0 | 176.09 | 1240.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1189938 | 46.10 | 56.68 | 67.90 | 92.60 | 155.22 | 26.7 | 0.0 | 0.0 | 0.0 | 152.14 | 517.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-busypoll50 | 1189936 | 46.72 (+0.53) | 58.23 (−1.86) | 69.65 (−7.42) | 115.71 (+5.21) | 193.17 (−145.72) | 78.2 | 0.0 | 0.0 | 0.0 | 157.09 | 681.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-budget64 | 1189934 | 46.33 (+0.13) | 59.10 (−0.99) | 76.62 (−0.45) | 110.33 (−0.18) | 436.45 (+97.57) | 102.9 | 0.0 | 0.0 | 0.0 | 400.06 | 501.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1189937 | 46.67 | 60.09 | 77.07 | 106.84 | 263.97 | 76.1 | 0.0 | 0.0 | 0.0 | 151.49 | 641.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 1189937 | 42.78 (−3.42) | 53.38 (−6.71) | 62.61 (−14.47) | 80.25 (−30.26) | 243.46 (−95.43) | 14.1 | 0.0 | 0.0 | 0.0 | 148.86 | 561.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 1189936 | 46.11 | 58.90 | 89.93 | 272.13 | 1011.07 | 568.8 | 0.5 | 0.5 | 0.0 | 151.28 | 504.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1189936 | 44.29 (−1.90) | 58.55 (−1.54) | 70.70 (−6.38) | 109.43 (−1.08) | 255.53 (−83.35) | 81.7 | 0.0 | 0.0 | 0.0 | 59.15 | 1249.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1189937 | 46.14 (−0.05) | 57.60 (−2.49) | 69.77 (−7.30) | 92.50 (−18.01) | 166.57 (−172.31) | 33.3 | 0.0 | 0.0 | 0.0 | 62.28 | 1798.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full-nomitig | 1189938 | 46.10 (−0.09) | 58.65 (−1.44) | 70.45 (−6.62) | 88.99 (−21.52) | 1045.66 (+706.78) | 20.2 | 0.5 | 0.5 | 0.0 | 58.03 | 1120.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1189937 | 46.31 (+0.11) | 58.80 (−1.29) | 72.65 (−4.43) | 119.69 (+9.19) | 269.07 (−69.82) | 154.3 | 0.0 | 0.0 | 0.0 | 155.98 | 1159.5 | neither loop stalled long enough: network, hypervisor or untraced |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 2499970 | 3.34 | 6.85 | 15.59 | 30.43 | 38.49 | 0.0 | 0.0 | 0.0 | 0.0 | 547.52 | 4651.0 | 547.52 µs stall: loop work (2700 items) |
| r1-housekeeping | 2500000 | 3.00 (+0.04) | 7.24 (+0.24) | 12.38 (−2.49) | 30.88 (+0.45) | 38.47 (−2.98) | 0.0 | 0.0 | 0.0 | 0.0 | 595.88 | 6127.0 | 595.88 µs stall: loop work (2750 items) |
| r1-prefer | 2500000 | 5.05 (+2.10) | 8.34 (+1.35) | 10.25 (−4.62) | 12.17 (−18.27) | 14.15 (−27.30) | 0.0 | 0.0 | 0.0 | 0.0 | 569.41 | 4415.0 | 569.41 µs stall: loop work (2700 items) |
| r1-base-2 | 2500000 | 2.92 | 6.79 | 13.16 | 33.07 | 42.90 | 0.0 | 0.0 | 0.0 | 0.0 | 606.11 | 4702.0 | 606.11 µs stall: loop work (2750 items) |
| r1-noquickack | 2500000 | 3.29 (+0.34) | 6.24 (−0.75) | 8.35 (−6.52) | 34.28 (+3.84) | 45.37 (+3.93) | 0.0 | 0.0 | 0.0 | 0.0 | 623.55 | 3716.0 | 623.55 µs stall: loop work (2750 items) |
| r1-nogro | 2500000 | 3.44 (+0.48) | 6.64 (−0.35) | 13.81 (−1.06) | 28.75 (−1.68) | 38.02 (−3.43) | 0.0 | 0.0 | 0.0 | 0.0 | 547.88 | 4306.0 | 547.88 µs stall: loop work (2750 items) |
| r1-base-4 | 2500000 | 3.07 | 7.00 | 14.13 | 30.11 | 42.19 | 0.0 | 0.0 | 0.0 | 0.0 | 577.73 | 5433.0 | 577.73 µs stall: loop work (2750 items) |
| r1-nocoalesce | 2500000 | 3.19 (+0.23) | 7.94 (+0.95) | 11.62 (−3.25) | 15.82 (−14.62) | 22.38 (−19.07) | 0.0 | 0.0 | 0.0 | 0.0 | 576.64 | 4638.0 | 576.64 µs stall: loop work (2750 items) |
| r1-irqlocal | 2499970 | 2.88 (−0.07) | 9.72 (+2.72) | 20.06 (+5.19) | 31.21 (+0.78) | 52.25 (+10.80) | 0.0 | 0.0 | 0.0 | 0.0 | 675.28 | 6842.0 | 675.28 µs stall: loop work (2881 items) |
| r1-base-6 | 2499950 | 2.62 | 6.30 | 10.62 | 32.72 | 46.95 | 0.0 | 0.0 | 0.0 | 0.0 | 669.56 | 6118.0 | 669.56 µs stall: loop work (2700 items) |
| r1-rt | 2500000 | 2.93 (−0.03) | 7.95 (+0.95) | 14.33 (−0.54) | 19.96 (−10.47) | 41.13 (−0.31) | 0.0 | 0.0 | 0.0 | 0.0 | 579.61 | 5973.0 | 579.61 µs stall: loop work (2750 items) |
| r1-sibling | 2499997 | 3.60 (+0.64) | 9.26 (+2.27) | 19.52 (+4.64) | 35.59 (+5.16) | 43.59 (+2.14) | 0.0 | 0.0 | 0.0 | 0.0 | 863.61 | 5839.0 | 863.61 µs stall: loop work (3600 items) |
| r1-base-8 | 2500000 | 3.12 | 12.04 | 16.67 | 25.35 | 32.19 | 0.0 | 0.0 | 0.0 | 0.0 | 525.98 | 4238.0 | 525.98 µs stall: loop work (2750 items) |
| r1-busypoll50 | 2499997 | 2.90 (−0.06) | 7.45 (+0.45) | 12.56 (−2.32) | 30.59 (+0.16) | 41.25 (−0.20) | 0.0 | 0.0 | 0.0 | 0.0 | 650.85 | 6070.0 | 650.85 µs stall: loop work (2731 items) |
| r1-budget64 | 2499982 | 2.93 (−0.02) | 7.50 (+0.51) | 14.12 (−0.75) | 31.62 (+1.18) | 41.32 (−0.13) | 0.0 | 0.0 | 0.0 | 0.0 | 623.86 | 5701.0 | 623.86 µs stall: loop work (2750 items) |
| r1-base-10 | 2499986 | 2.69 | 15.77 | 20.13 | 27.01 | 41.45 | 0.0 | 0.0 | 0.0 | 0.0 | 636.34 | 5920.0 | 636.34 µs stall: loop work (2800 items) |
| r1-cstate | 2500000 | 2.87 (−0.09) | 7.01 (+0.02) | 15.29 (+0.42) | 31.79 (+1.35) | 41.24 (−0.21) | 0.0 | 0.0 | 0.0 | 0.0 | 597.60 | 6597.0 | 597.60 µs stall: loop work (2800 items) |
| r1-base-11 | 2500000 | 2.96 | 7.81 | 14.87 | 30.97 | 40.79 | 0.0 | 0.0 | 0.0 | 0.0 | 694.80 | 5866.0 | 694.80 µs stall: loop work (3100 items) |
| boot-stock | 2500000 | 2.80 (−0.16) | 6.21 (−0.79) | 9.73 (−5.14) | 19.95 (−10.48) | 51.91 (+10.46) | 0.0 | 0.0 | 0.0 | 0.0 | 545.48 | 4213.0 | 545.48 µs stall: loop work (2850 items) |
| boot-isolcpus | 2500000 | 2.72 (−0.24) | 6.43 (−0.57) | 15.43 (+0.55) | 21.57 (−8.87) | 27.78 (−13.67) | 0.0 | 0.0 | 0.0 | 0.0 | 555.29 | 5023.0 | 555.29 µs stall: loop work (2750 items) |
| boot-full-nomitig | 2499950 | 2.44 (−0.52) | 6.71 (−0.29) | 11.84 (−3.03) | 30.71 (+0.27) | 38.20 (−3.25) | 0.0 | 0.0 | 0.0 | 0.0 | 580.50 | 6433.0 | 580.50 µs stall: loop work (2700 items) |
| boot-full | 2500000 | 3.30 (+0.34) | 7.22 (+0.22) | 14.43 (−0.45) | 33.43 (+3.00) | 39.73 (−1.72) | 0.0 | 0.0 | 0.0 | 0.0 | 532.31 | 4683.0 | 532.31 µs stall: loop work (2700 items) |

## WebSocket round trip, pp-epoll-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1137716 | 48.94 | 97.37 | 108.04 | 286.57 | 438.11 | 3480.0 | 0.0 | 0.0 | 0.0 | 46.57 | 9.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1146655 | 49.03 | 96.59 | 102.48 | 202.65 | 630.86 | 1862.0 | 3.1 | 0.0 | 0.0 | 149.02 | 17.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1129179 | 49.10 | 97.81 | 106.41 | 142.01 | 421.13 | 3573.6 | 0.0 | 0.0 | 0.0 | 142.25 | 23.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 1108612 | 49.08 (+0.04) | 99.15 (+1.58) | 135.69 (+28.26) | 141.66 (−60.98) | 275.95 (−214.11) | 10473.6 | 0.0 | 0.0 | 0.0 | 157.15 | 2164.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1100305 | 49.41 | 99.48 | 241.04 | 343.35 | 490.06 | 10294.6 | 0.0 | 0.0 | 0.0 | 145.48 | 19.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1127171 | 49.05 | 97.56 | 107.44 | 276.32 | 894.63 | 3671.2 | 1.0 | 0.0 | 0.0 | 155.92 | 16.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1121246 | 50.16 | 95.27 | 104.51 | 143.09 | 218.94 | 2385.8 | 0.0 | 0.0 | 0.0 | 132.48 | 24.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 1138830 | 49.01 (−0.03) | 123.67 (+26.11) | 132.78 (+25.34) | 140.54 (−62.10) | 304.29 (−185.76) | 13546.8 | 0.0 | 0.0 | 0.0 | 143.22 | 8.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 1124260 | 49.04 | 100.43 | 108.24 | 140.04 | 1041.06 | 19777.7 | 1.0 | 1.0 | 0.0 | 142.67 | 13.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 819759 | 49.24 (+0.19) | 328.19 (+230.63) | 403.64 (+296.20) | 456.11 (+253.47) | 549.86 (+59.80) | 66615.6 | 3.1 | 0.0 | 0.0 | 153.87 | 102.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1100902 | 50.57 (+1.52) | 97.14 (−0.42) | 104.61 (−2.83) | 235.01 (+32.37) | 510.03 (+19.97) | 2490.5 | 1.0 | 0.0 | 0.0 | 55.92 | 26.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1114043 | 49.09 (+0.04) | 100.81 (+3.25) | 134.97 (+27.54) | 143.46 (−59.19) | 245.79 (−244.27) | 22362.9 | 0.0 | 0.0 | 0.0 | 61.87 | 12.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1144070 | 48.89 (−0.16) | 97.47 (−0.09) | 146.58 (+39.15) | 320.70 (+118.06) | 1035.94 (+545.88) | 3026.5 | 8.1 | 1.0 | 0.0 | 150.84 | 8.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-epoll-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1146636 | 51.56 | 232.20 | 295.26 | 342.17 | 411.31 | 30125.6 | 0.0 | 0.0 | 0.0 | 352.78 | 10.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1168493 | 60.65 | 137.18 | 238.39 | 305.27 | 395.01 | 33220.4 | 0.0 | 0.0 | 0.0 | 148.24 | 9.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1185864 | 51.01 | 103.70 | 134.62 | 143.29 | 1053.62 | 8123.2 | 0.5 | 0.5 | 0.0 | 263.69 | 10.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 1186090 | 50.68 (−0.33) | 100.82 (−2.88) | 131.51 (−3.11) | 138.46 (−4.83) | 335.65 (−643.08) | 6967.6 | 0.0 | 0.0 | 0.0 | 156.66 | 1076.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1188213 | 50.33 | 98.06 | 109.32 | 138.27 | 1011.40 | 3069.6 | 0.5 | 0.5 | 0.0 | 242.49 | 15.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1187801 | 50.64 | 94.30 | 128.81 | 135.71 | 186.72 | 2313.8 | 0.0 | 0.0 | 0.0 | 150.00 | 8.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1182868 | 51.75 | 129.63 | 140.54 | 149.21 | 978.73 | 19352.8 | 0.5 | 0.0 | 0.0 | 141.57 | 8.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 1188524 | 48.16 (−2.85) | 95.67 (−8.03) | 102.91 (−31.71) | 126.58 (−16.71) | 204.20 (−774.53) | 1269.1 | 0.0 | 0.0 | 0.0 | 138.82 | 9.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 1186876 | 50.84 | 97.17 | 133.77 | 138.94 | 999.45 | 4009.4 | 0.5 | 0.0 | 0.0 | 149.56 | 8.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 493856 | 256.32 (+205.31) | 404.35 (+300.65) | 464.62 (+330.00) | 531.64 (+388.35) | 662.28 (−316.45) | 199336.2 | 64.5 | 0.0 | 0.0 | 52.48 | 22.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1103391 | 52.47 (+1.46) | 297.06 (+193.36) | 368.99 (+234.38) | 432.55 (+289.26) | 538.73 (−440.00) | 41167.2 | 3.5 | 0.0 | 0.0 | 65.04 | 13.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1188628 | 50.02 (−0.99) | 93.77 (−9.93) | 99.67 (−34.94) | 109.31 (−33.98) | 166.61 (−812.12) | 533.4 | 0.0 | 0.0 | 0.0 | 195.15 | 7.0 | server: softirq TIMER |
| boot-full | 1181101 | 74.04 (+23.03) | 113.56 (+9.86) | 233.68 (+99.06) | 293.07 (+149.78) | 534.91 (−443.82) | 18139.2 | 0.5 | 0.0 | 0.0 | 254.23 | 7.5 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-recv-busy-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1270218 | 45.25 | 65.56 | 93.96 | 101.94 | 199.64 | 209.5 | 0.0 | 0.0 | 0.0 | 40.95 | 651.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1258068 | 46.04 (+0.30) | 63.37 (+0.39) | 95.90 (+3.34) | 109.95 (+5.04) | 260.84 (+34.09) | 433.2 | 0.0 | 0.0 | 0.0 | 133.05 | 486.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 1085440 | 52.30 (+6.56) | 96.51 (+33.52) | 103.47 (+10.91) | 128.03 (+23.12) | 256.81 (+30.06) | 3445.4 | 0.0 | 0.0 | 0.0 | 126.63 | 10.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-2 | 1284155 | 44.58 | 62.77 | 92.03 | 101.61 | 1042.84 | 179.0 | 1.0 | 1.0 | 0.0 | 151.51 | 521.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 1309119 | 44.42 (−1.31) | 56.07 (−6.92) | 64.16 (−28.40) | 80.21 (−24.69) | 366.12 (+139.37) | 26.4 | 0.0 | 0.0 | 0.0 | 148.84 | 455.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 1197015 | 48.91 (+3.17) | 67.90 (+4.92) | 96.68 (+4.12) | 105.58 (+0.68) | 415.77 (+189.02) | 444.4 | 0.0 | 0.0 | 0.0 | 149.45 | 484.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1224046 | 48.26 | 62.98 | 95.50 | 106.21 | 192.07 | 342.7 | 0.0 | 0.0 | 0.0 | 154.33 | 540.0 | client: mostly unexplained (+softirq NET_RX) |
| r1-nocoalesce | 1317229 | 43.54 (−2.20) | 59.60 (−3.38) | 77.98 (−14.58) | 98.31 (−6.60) | 1049.85 (+823.11) | 107.8 | 1.0 | 1.0 | 0.0 | 147.32 | 19.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 1215891 | 48.46 (+2.72) | 72.76 (+9.78) | 96.82 (+4.26) | 105.47 (+0.56) | 206.16 (−20.59) | 432.2 | 0.0 | 0.0 | 0.0 | 155.99 | 873.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1301906 | 43.96 | 61.37 | 91.17 | 104.91 | 226.75 | 251.2 | 0.0 | 0.0 | 0.0 | 147.91 | 508.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 1303582 | 43.87 (−1.87) | 62.19 (−0.80) | 85.77 (−6.78) | 100.41 (−4.50) | 184.61 (−42.14) | 140.3 | 0.0 | 0.0 | 0.0 | 363.91 | 48.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-sibling | 1226617 | 47.92 (+2.18) | 65.10 (+2.12) | 96.12 (+3.56) | 104.90 (−0.01) | 241.07 (+14.32) | 358.0 | 0.0 | 0.0 | 0.0 | 56.42 | 1094.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1253980 | 45.74 | 63.86 | 91.86 | 120.68 | 280.77 | 348.8 | 0.0 | 0.0 | 0.0 | 152.00 | 498.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-busypoll50 | 1281699 | 44.70 (−1.04) | 63.11 (+0.13) | 92.93 (+0.37) | 150.55 (+45.64) | 1049.93 (+823.19) | 389.5 | 2.0 | 2.0 | 0.0 | 157.97 | 513.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-budget64 | 1251157 | 45.71 (−0.03) | 78.21 (+15.23) | 99.48 (+6.92) | 120.35 (+15.44) | 317.46 (+90.72) | 1142.0 | 0.0 | 0.0 | 0.0 | 151.71 | 594.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1242073 | 47.32 | 80.00 | 95.11 | 106.42 | 205.42 | 351.9 | 0.0 | 0.0 | 0.0 | 144.11 | 413.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 1220516 | 48.65 (+2.91) | 60.57 (−2.41) | 94.21 (+1.65) | 106.27 (+1.36) | 482.76 (+256.01) | 286.8 | 0.0 | 0.0 | 0.0 | 144.32 | 223.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 1258423 | 46.78 | 60.36 | 92.56 | 101.03 | 275.21 | 167.8 | 0.0 | 0.0 | 0.0 | 152.66 | 574.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1282803 | 45.19 (−0.55) | 57.19 (−5.80) | 70.43 (−22.13) | 95.57 (−9.33) | 188.57 (−38.17) | 87.5 | 0.0 | 0.0 | 0.0 | 151.11 | 166.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-isolcpus | 1275992 | 45.35 (−0.39) | 60.13 (−2.85) | 94.32 (+1.77) | 109.35 (+4.44) | 984.62 (+757.87) | 378.3 | 1.0 | 0.0 | 0.0 | 68.49 | 22.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1310151 | 43.72 (−2.02) | 60.06 (−2.92) | 90.44 (−2.11) | 102.83 (−2.07) | 416.89 (+190.14) | 209.5 | 0.0 | 0.0 | 0.0 | 256.23 | 563.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1274731 | 45.23 (−0.51) | 61.51 (−1.48) | 92.67 (+0.11) | 101.01 (−3.89) | 694.49 (+467.75) | 161.7 | 1.0 | 0.0 | 0.0 | 163.29 | 745.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1186907 | 45.46 | 57.91 | 70.69 | 85.60 | 331.27 | 12.6 | 0.0 | 0.0 | 0.0 | 63.37 | 497.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1186892 | 46.55 (+0.99) | 59.63 (+1.73) | 75.62 (+1.07) | 101.07 (+3.31) | 994.56 (+663.30) | 63.0 | 0.5 | 0.0 | 0.0 | 174.50 | 508.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 1180068 | 50.43 (+4.86) | 59.66 (+1.75) | 68.95 (−5.60) | 94.66 (−3.11) | 392.79 (+61.52) | 46.9 | 0.0 | 0.0 | 0.0 | 361.86 | 9.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1186892 | 46.67 | 59.67 | 74.55 | 93.98 | 254.20 | 43.9 | 0.0 | 0.0 | 0.0 | 48.98 | 511.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 1044377 | 44.51 (−1.06) | 240.28 (+182.37) | 294.93 (+220.38) | 337.47 (+239.70) | 401.18 (+69.92) | 68234.9 | 0.0 | 0.0 | 0.0 | 153.72 | 551.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 1186888 | 44.39 (−1.17) | 56.89 (−1.01) | 73.47 (−1.08) | 99.31 (+1.54) | 196.23 (−135.03) | 57.5 | 0.0 | 0.0 | 0.0 | 149.40 | 473.0 | client: mostly unexplained (+softirq NET_RX) |
| r1-base-4 | 1186906 | 44.10 | 55.32 | 69.16 | 97.77 | 322.58 | 56.5 | 0.0 | 0.0 | 0.0 | 405.13 | 424.5 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nocoalesce | 1175528 | 46.35 (+0.79) | 92.12 (+34.21) | 240.31 (+165.76) | 298.71 (+200.95) | 1042.13 (+710.86) | 5036.0 | 0.5 | 0.5 | 0.0 | 267.26 | 10.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 1174948 | 46.99 (+1.43) | 94.67 (+36.76) | 242.27 (+167.72) | 300.83 (+203.07) | 359.49 (+28.22) | 5206.9 | 0.0 | 0.0 | 0.0 | 160.67 | 1929.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1186924 | 46.00 | 85.65 | 92.60 | 102.40 | 160.35 | 96.3 | 0.0 | 0.0 | 0.0 | 145.93 | 117.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 1186854 | 43.42 (−2.15) | 53.47 (−4.44) | 63.62 (−10.93) | 103.14 (+5.37) | 287.06 (−44.20) | 64.5 | 0.0 | 0.0 | 0.0 | 58.10 | 49.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1179036 | 48.28 (+2.72) | 91.72 (+33.81) | 231.72 (+157.18) | 290.97 (+193.20) | 1050.71 (+719.45) | 3102.9 | 1.0 | 1.0 | 0.0 | 71.49 | 1131.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1162328 | 45.87 | 212.81 | 273.04 | 307.27 | 369.23 | 11189.3 | 0.0 | 0.0 | 0.0 | 150.94 | 76.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-busypoll50 | 1186885 | 47.58 (+2.02) | 61.30 (+3.39) | 71.27 (−3.28) | 117.12 (+19.36) | 308.13 (−23.14) | 104.9 | 0.0 | 0.0 | 0.0 | 148.31 | 101.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-budget64 | 1186876 | 46.75 (+1.19) | 57.98 (+0.07) | 70.62 (−3.93) | 122.05 (+24.29) | 320.50 (−10.76) | 147.2 | 0.0 | 0.0 | 0.0 | 146.53 | 371.5 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 1186875 | 45.56 | 57.77 | 75.40 | 110.11 | 1056.92 | 79.2 | 0.5 | 0.5 | 0.0 | 155.38 | 465.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 1186877 | 42.91 (−2.65) | 53.85 (−4.06) | 70.01 (−4.54) | 99.38 (+1.61) | 187.88 (−143.39) | 58.0 | 0.0 | 0.0 | 0.0 | 155.26 | 368.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 1186916 | 43.83 | 54.79 | 68.85 | 86.82 | 996.67 | 29.2 | 0.5 | 0.0 | 0.0 | 143.35 | 426.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1187077 | 42.09 (−3.48) | 53.02 (−4.89) | 68.41 (−6.14) | 119.56 (+21.80) | 1047.96 (+716.70) | 196.6 | 6.1 | 0.5 | 0.0 | 437.14 | 62.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1174859 | 45.37 (−0.19) | 94.89 (+36.98) | 243.32 (+168.77) | 300.67 (+202.90) | 359.96 (+28.69) | 5505.4 | 0.0 | 0.0 | 0.0 | 36.78 | 13.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1179773 | 46.77 (+1.21) | 91.20 (+33.29) | 233.37 (+158.82) | 291.76 (+193.99) | 730.42 (+399.15) | 3407.9 | 2.5 | 0.0 | 0.0 | 60.42 | 615.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1186828 | 46.80 (+1.24) | 88.76 (+30.85) | 95.20 (+20.66) | 117.26 (+19.49) | 311.56 (−19.71) | 223.4 | 0.0 | 0.0 | 0.0 | 148.09 | 558.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1132952 | 48.31 | 96.52 | 103.05 | 134.84 | 247.40 | 2339.0 | 0.0 | 0.0 | 0.0 | 42.40 | 13.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1131504 | 48.42 (+0.02) | 96.19 (−0.46) | 102.78 (−1.37) | 141.60 (−0.99) | 229.31 (−245.08) | 2134.6 | 0.0 | 0.0 | 0.0 | 133.65 | 21.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1135209 | 48.46 | 96.41 | 101.90 | 134.82 | 266.89 | 1738.0 | 0.0 | 0.0 | 0.0 | 124.81 | 13.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 1141256 | 49.84 (+1.44) | 65.04 (−31.60) | 96.34 (−7.81) | 110.27 (−32.33) | 375.07 (−99.32) | 247.1 | 0.0 | 0.0 | 0.0 | 143.59 | 6.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 1099431 | 48.88 (+0.48) | 97.19 (+0.55) | 106.39 (+2.25) | 139.93 (−2.66) | 228.53 (−245.85) | 3800.4 | 0.0 | 0.0 | 0.0 | 151.24 | 13.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1111759 | 48.62 | 96.94 | 104.19 | 145.04 | 331.27 | 2972.6 | 0.0 | 0.0 | 0.0 | 147.77 | 11.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 1088623 | 52.26 (+3.85) | 97.01 (+0.37) | 104.43 (+0.28) | 125.74 (−16.85) | 295.06 (−179.33) | 5105.1 | 0.0 | 0.0 | 0.0 | 283.72 | 32.0 | client: softirq TIMER |
| r1-irqlocal | 1119776 | 48.55 (+0.14) | 97.64 (+1.00) | 131.76 (+27.61) | 137.59 (−5.00) | 201.62 (−272.76) | 7508.2 | 0.0 | 0.0 | 0.0 | 152.00 | 3522.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1149513 | 48.02 | 96.06 | 101.70 | 135.28 | 474.38 | 1582.4 | 0.0 | 0.0 | 0.0 | 142.85 | 12.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 1102745 | 48.46 (+0.05) | 100.85 (+4.21) | 137.51 (+33.37) | 146.87 (+4.28) | 380.33 (−94.06) | 15763.8 | 0.0 | 0.0 | 0.0 | 75.18 | 12.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1033966 | 53.57 (+5.17) | 99.54 (+2.89) | 137.81 (+33.66) | 148.81 (+6.22) | 325.31 (−149.08) | 9649.9 | 0.0 | 0.0 | 0.0 | 176.19 | 21.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1127438 | 48.40 | 96.65 | 104.15 | 142.59 | 1041.29 | 2637.0 | 1.0 | 1.0 | 0.0 | 137.52 | 27.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1103197 | 49.01 | 97.01 | 107.45 | 145.11 | 514.72 | 3396.6 | 1.0 | 0.0 | 0.0 | 203.47 | 18.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 1114740 | 48.87 (+0.47) | 97.69 (+1.04) | 133.78 (+29.64) | 140.94 (−1.65) | 445.55 (−28.83) | 7126.8 | 0.0 | 0.0 | 0.0 | 388.33 | 11.0 | client: softirq TIMER |
| r1-base-11 | 1129328 | 48.35 | 96.65 | 106.80 | 296.19 | 1062.24 | 3000.0 | 1.0 | 1.0 | 0.0 | 392.88 | 25.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1035989 | 48.73 (+0.33) | 207.41 (+110.76) | 327.47 (+223.32) | 407.09 (+264.49) | 494.21 (+19.82) | 15442.5 | 0.0 | 0.0 | 0.0 | 81.79 | 32.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1108845 | 48.58 (+0.18) | 97.56 (+0.91) | 306.99 (+202.84) | 417.09 (+274.50) | 1061.57 (+587.19) | 8013.6 | 18.3 | 1.0 | 0.0 | 44.39 | 25.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1087910 | 49.15 (+0.75) | 98.91 (+2.26) | 111.86 (+7.71) | 144.16 (+1.56) | 1107.96 (+633.57) | 8051.2 | 1.0 | 1.0 | 0.0 | 98.45 | 13.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1156769 | 47.90 (−0.50) | 94.81 (−1.84) | 100.76 (−3.39) | 224.70 (+82.11) | 338.92 (−135.46) | 1324.1 | 0.0 | 0.0 | 0.0 | 149.67 | 24.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1187447 | 51.18 | 100.97 | 134.51 | 141.55 | 233.45 | 7095.7 | 0.0 | 0.0 | 0.0 | 75.77 | 11.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1182178 | 55.41 (+3.55) | 124.22 (+16.24) | 228.34 (−0.70) | 288.36 (+0.81) | 1054.08 (+677.35) | 11253.3 | 0.5 | 0.5 | 0.0 | 79.09 | 5.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1181232 | 51.87 | 131.16 | 141.17 | 269.32 | 376.73 | 22587.8 | 0.0 | 0.0 | 0.0 | 254.17 | 6.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 1189587 | 53.17 (+1.31) | 95.94 (−12.04) | 102.34 (−126.70) | 124.16 (−163.39) | 207.05 (−169.69) | 1331.1 | 0.0 | 0.0 | 0.0 | 149.62 | 5.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 1164937 | 49.36 (−2.51) | 214.53 (+106.55) | 320.86 (+91.82) | 388.87 (+101.32) | 507.14 (+130.41) | 13940.7 | 0.5 | 0.0 | 0.0 | 145.24 | 9.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1183429 | 53.95 | 107.98 | 229.04 | 287.55 | 352.39 | 8184.7 | 0.0 | 0.0 | 0.0 | 145.56 | 7.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 1189652 | 50.93 (−0.93) | 71.33 (−36.65) | 89.33 (−139.71) | 128.07 (−159.48) | 288.47 (−88.26) | 165.9 | 0.0 | 0.0 | 0.0 | 157.32 | 8.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-irqlocal | 1157892 | 57.29 (+5.42) | 137.81 (+29.83) | 245.13 (+16.09) | 308.88 (+21.33) | 379.99 (+3.26) | 47953.3 | 0.0 | 0.0 | 0.0 | 156.93 | 1332.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1163799 | 50.10 | 221.06 | 323.06 | 388.35 | 999.60 | 14204.9 | 1.0 | 0.0 | 0.0 | 393.62 | 10.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 1108556 | 50.92 (−0.95) | 289.00 (+181.01) | 396.41 (+167.38) | 486.31 (+198.76) | 654.04 (+277.31) | 35227.7 | 38.3 | 0.0 | 0.0 | 131.28 | 9.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1189125 | 51.37 (−0.50) | 99.83 (−8.15) | 107.07 (−121.97) | 124.89 (−162.67) | 331.94 (−44.80) | 5611.3 | 0.0 | 0.0 | 0.0 | 167.55 | 8.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1147118 | 51.87 | 232.23 | 296.28 | 344.49 | 478.31 | 30458.9 | 0.0 | 0.0 | 0.0 | 363.05 | 9.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1189133 | 52.02 | 99.53 | 109.47 | 140.82 | 269.39 | 5187.2 | 0.0 | 0.0 | 0.0 | 155.07 | 13.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 1189654 | 48.40 (−3.47) | 93.42 (−14.56) | 99.95 (−129.08) | 114.23 (−173.32) | 237.17 (−139.56) | 590.9 | 0.0 | 0.0 | 0.0 | 141.27 | 8.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 1184099 | 54.80 | 101.66 | 231.46 | 292.06 | 377.08 | 6826.9 | 0.0 | 0.0 | 0.0 | 145.89 | 9.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 749532 | 51.56 (−0.30) | 360.42 (+252.44) | 421.34 (+192.30) | 469.88 (+182.32) | 5116.32 (+4739.59) | 135785.2 | 7.1 | 0.5 | 0.0 | 107.55 | 29.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 874736 | 55.79 (+3.93) | 372.17 (+264.19) | 444.00 (+214.96) | 511.76 (+224.21) | 1101.91 (+725.18) | 87948.3 | 69.6 | 1.0 | 0.0 | 65.56 | 12.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1189221 | 50.27 (−1.59) | 97.48 (−10.50) | 115.57 (−113.47) | 135.74 (−151.81) | 210.34 (−166.39) | 2382.4 | 0.0 | 0.0 | 0.0 | 119.67 | 4.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1177205 | 51.48 (−0.39) | 130.46 (+22.48) | 261.03 (+31.99) | 314.97 (+27.42) | 380.39 (+3.66) | 9818.3 | 0.0 | 0.0 | 0.0 | 216.18 | 7.5 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-sqpoll-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1239942 | 46.43 | 84.38 | 96.70 | 106.70 | 1048.50 | 493.2 | 1.0 | 1.0 | 0.0 | 393.00 | 4.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1217121 | 47.89 (+0.28) | 87.91 (+3.52) | 97.80 (+0.84) | 106.37 (−0.33) | 205.90 (−835.24) | 578.6 | 0.0 | 0.0 | 0.0 | 39.78 | 3.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 1132025 | 49.48 (+1.87) | 96.23 (+11.85) | 101.70 (+4.74) | 110.76 (+4.06) | 277.65 (−763.49) | 2437.6 | 0.0 | 0.0 | 0.0 | 40.64 | 3.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1245154 | 46.77 | 63.72 | 95.28 | 106.58 | 1041.15 | 340.7 | 1.0 | 1.0 | 0.0 | 132.59 | 4.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1166808 | 48.65 | 94.74 | 102.78 | 128.60 | 287.42 | 2576.0 | 0.0 | 0.0 | 0.0 | 212.70 | 5.0 | server: softirq TIMER |
| r1-base-6 | 1211750 | 47.60 | 89.92 | 99.03 | 109.58 | 332.90 | 915.3 | 0.0 | 0.0 | 0.0 | 143.05 | 5.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1196858 | 48.63 (+1.02) | 89.77 (+5.38) | 100.11 (+3.15) | 117.06 (+10.36) | 260.90 (−780.24) | 1255.9 | 0.0 | 0.0 | 0.0 | 159.87 | 5.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1219432 | 47.93 | 86.02 | 96.96 | 106.00 | 1049.67 | 512.5 | 1.0 | 1.0 | 0.0 | 145.12 | 3.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1228848 | 47.61 | 72.78 | 97.51 | 110.09 | 1044.55 | 613.2 | 1.0 | 1.0 | 0.0 | 140.11 | 6.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 1190968 | 48.81 (+1.20) | 86.59 (+2.20) | 96.79 (−0.17) | 106.51 (−0.19) | 196.93 (−844.22) | 412.9 | 0.0 | 0.0 | 0.0 | 151.11 | 4.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-11 | 1221351 | 47.95 | 82.96 | 96.83 | 104.84 | 323.85 | 427.1 | 0.0 | 0.0 | 0.0 | 149.85 | 7.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1274231 | 45.41 (−2.20) | 57.38 (−27.00) | 66.14 (−30.82) | 91.19 (−15.51) | 342.18 (−698.97) | 84.4 | 0.0 | 0.0 | 0.0 | 62.79 | 34.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1251583 | 46.80 (−0.81) | 62.08 (−22.30) | 94.32 (−2.64) | 104.55 (−2.15) | 181.53 (−859.62) | 274.6 | 0.0 | 0.0 | 0.0 | 35.56 | 3.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full-nomitig | 1208416 | 47.34 (−0.27) | 91.08 (+6.70) | 100.58 (+3.62) | 124.22 (+17.52) | 305.72 (−735.42) | 1446.1 | 0.0 | 0.0 | 0.0 | 55.12 | 2.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full | 1227519 | 47.62 (+0.01) | 76.05 (−8.34) | 98.42 (+1.46) | 107.09 (+0.39) | 1052.82 (+11.68) | 747.5 | 1.0 | 1.0 | 0.0 | 144.38 | 4.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-sqpoll-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189549 | 46.11 | 58.53 | 74.41 | 107.36 | 292.05 | 64.5 | 0.0 | 0.0 | 0.0 | 32.67 | 0.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1172998 | 47.29 (+0.85) | 179.86 (+22.80) | 254.59 (+6.52) | 305.98 (+6.04) | 381.92 (−14.51) | 7227.8 | 0.0 | 0.0 | 0.0 | 130.03 | 3.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 1189617 | 50.83 (+4.39) | 92.95 (−64.11) | 98.29 (−149.78) | 116.45 (−183.49) | 998.23 (+601.80) | 340.3 | 0.5 | 0.0 | 0.0 | 107.26 | 3.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1171496 | 46.44 | 191.08 | 260.81 | 305.85 | 396.43 | 7798.5 | 0.0 | 0.0 | 0.0 | 128.85 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1189475 | 46.70 | 58.59 | 69.23 | 125.33 | 202.81 | 140.7 | 0.0 | 0.0 | 0.0 | 137.75 | 4.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1173366 | 46.43 | 181.11 | 252.26 | 303.73 | 993.19 | 6988.8 | 0.5 | 0.0 | 0.0 | 156.45 | 4.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1189297 | 47.41 (+0.97) | 56.65 (−100.42) | 66.84 (−181.23) | 96.32 (−203.63) | 454.83 (+58.41) | 48.4 | 0.0 | 0.0 | 0.0 | 366.74 | 3.5 | client: softirq TIMER |
| r1-base-8 | 1174494 | 46.04 | 170.43 | 248.43 | 301.37 | 433.71 | 6726.1 | 0.0 | 0.0 | 0.0 | 151.21 | 3.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1189507 | 47.46 | 63.72 | 82.12 | 125.03 | 1037.29 | 147.7 | 0.5 | 0.5 | 0.0 | 273.41 | 5.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 1189306 | 45.47 (−0.97) | 56.45 (−100.61) | 65.89 (−182.17) | 116.28 (−183.66) | 1055.23 (+658.80) | 106.9 | 0.5 | 0.5 | 0.0 | 201.12 | 6.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 1175543 | 46.55 | 157.06 | 248.07 | 299.95 | 369.08 | 6155.3 | 0.0 | 0.0 | 0.0 | 255.82 | 5.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1184423 | 44.59 (−1.85) | 58.21 (−98.85) | 228.37 (−19.70) | 286.56 (−13.39) | 353.90 (−42.52) | 2157.5 | 0.0 | 0.0 | 0.0 | 64.72 | 42.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1189043 | 45.61 (−0.84) | 56.33 (−100.73) | 70.91 (−177.16) | 107.14 (−192.81) | 339.15 (−57.27) | 78.2 | 0.0 | 0.0 | 0.0 | 91.11 | 3.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1177394 | 47.27 (+0.83) | 95.64 (−61.42) | 242.10 (−5.97) | 298.82 (−1.12) | 1003.40 (+606.98) | 5320.9 | 0.5 | 0.5 | 0.0 | 55.56 | 3.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1189463 | 47.86 (+1.42) | 89.92 (−67.14) | 96.32 (−151.75) | 105.44 (−194.51) | 302.19 (−94.24) | 207.7 | 0.0 | 0.0 | 0.0 | 144.65 | 6.0 | neither loop stalled long enough: network, hypervisor or untraced |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs. The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | receiving CPU | LOC | IWI | RES | CAL |
|---|---|---|---|---|---|---|---|---|---|---|---|
| http-recv-busy-10000 | r1-base-0 | 1 | 32 | 32 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-housekeeping | 1 | 8 | 8 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-prefer | 1 | 1 | 0 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-base-2 | 1 | 31 | 30 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-noquickack | 1 | 17 | 25 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-nogro | 1 | 22 | 21 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-base-4 | 1 | 21 | 21 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-nocoalesce | 1 | 0 | 0 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-irqlocal | 1 | 46 | 146 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-base-6 | 1 | 10 | 10 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-rt | 1 | 550 | 1 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-sibling | 1 | 37 | 36 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-base-8 | 1 | 21 | 21 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-busypoll50 | 1 | 33 | 32 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-budget64 | 1 | 19 | 19 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-base-10 | 1 | 31 | 31 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-cstate | 1 | 36 | 36 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | r1-base-11 | 1 | 32 | 32 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | boot-stock | 1 | 1003 | 0 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | boot-isolcpus | 1 | 1001 | 0 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | boot-full-nomitig | 1 | 28 | 28 | 0 | 8 | | | | | | |
| http-recv-busy-10000 | boot-full | 1 | 18 | 18 | 0 | 8 | | | | | | |
| md-recv-busy | r1-base-0 | 1 | 13 | 13 | 0 | 17 | | | | | | |
| md-recv-busy | r1-housekeeping | 1 | 10 | 10 | 0 | 17 | | | | | | |
| md-recv-busy | r1-prefer | 1 | 2 | 0 | 0 | 17 | | | | | | |
| md-recv-busy | r1-base-2 | 1 | 12 | 12 | 0 | 16 | | | | | | |
| md-recv-busy | r1-noquickack | 1 | 5 | 10 | 0 | 16 | | | | | | |
| md-recv-busy | r1-nogro | 1 | 11 | 12 | 0 | 17 | | | | | | |
| md-recv-busy | r1-base-4 | 1 | 13 | 13 | 0 | 17 | | | | | | |
| md-recv-busy | r1-nocoalesce | 1 | 0 | 0 | 0 | 17 | | | | | | |
| md-recv-busy | r1-irqlocal | 1 | 13 | 39 | 0 | 16 | | | | | | |
| md-recv-busy | r1-base-6 | 1 | 10 | 10 | 0 | 17 | | | | | | |
| md-recv-busy | r1-rt | 1 | 547 | 1 | 0 | 17 | | | | | | |
| md-recv-busy | r1-sibling | 1 | 15 | 15 | 0 | 16 | | | | | | |
| md-recv-busy | r1-base-8 | 1 | 7 | 7 | 0 | 17 | | | | | | |
| md-recv-busy | r1-busypoll50 | 1 | 12 | 12 | 0 | 17 | 0 | 1140 | 0 | 0 | 19 |
| md-recv-busy | r1-budget64 | 1 | 11 | 12 | 0 | 17 | | | | | | |
| md-recv-busy | r1-base-10 | 1 | 8 | 8 | 0 | 17 | | | | | | |
| md-recv-busy | r1-cstate | 1 | 16 | 14 | 0 | 17 | | | | | | |
| md-recv-busy | r1-base-11 | 1 | 14 | 14 | 0 | 17 | | | | | | |
| md-recv-busy | boot-stock | 1 | 1010 | 0 | 0 | 17 | | | | | | |
| md-recv-busy | boot-isolcpus | 1 | 1002 | 0 | 0 | 16 | | | | | | |
| md-recv-busy | boot-full-nomitig | 1 | 10 | 10 | 0 | 16 | | | | | | |
| md-recv-busy | boot-full | 1 | 11 | 11 | 0 | 17 | | | | | | |
| noise | r1-base-0 | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-housekeeping | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-prefer | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-base-2 | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-noquickack | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-nogro | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-base-4 | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-nocoalesce | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-irqlocal | 1 | 1 | 2 | 0 | 8 | | | | | | |
| noise | r1-base-6 | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-rt | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-sibling | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-base-8 | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-busypoll50 | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-budget64 | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-base-10 | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-cstate | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | r1-base-11 | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | boot-stock | 1 | 1002 | 0 | 0 | 4 | | | | | | |
| noise | boot-isolcpus | 1 | 1001 | 0 | 0 | 8 | | | | | | |
| noise | boot-full-nomitig | 1 | 0 | 0 | 0 | 8 | | | | | | |
| noise | boot-full | 1 | 0 | 0 | 0 | 8 | | | | | | |
| pp-epoll-spin-0 | r1-base-0 | 1 | 0 | 0 | 0 | 16 | 4 | 1010 | 0 | 1 | 15 |
| pp-epoll-spin-0 | r1-base-2 | 1 | 0 | 0 | 0 | 16 | 4 | 731 | 0 | 1 | 22 |
| pp-epoll-spin-0 | r1-base-4 | 1 | 0 | 0 | 0 | 16 | 0 | 1140 | 0 | 1 | 6 |
| pp-epoll-spin-0 | r1-irqlocal | 1 | 87 | 206 | 0 | 16 | | | | | | |
| pp-epoll-spin-0 | r1-base-6 | 1 | 0 | 0 | 0 | 16 | 4 | 997 | 0 | 1 | 14 |
| pp-epoll-spin-0 | r1-base-8 | 1 | 0 | 0 | 0 | 16 | 4 | 1012 | 0 | 1 | 6 |
| pp-epoll-spin-0 | r1-base-10 | 1 | 0 | 0 | 0 | 16 | 0 | 1140 | 0 | 1 | 15 |
| pp-epoll-spin-0 | r1-cstate | 1 | 0 | 0 | 0 | 16 | 0 | 1138 | 0 | 1 | 13 |
| pp-epoll-spin-0 | r1-base-11 | 1 | 0 | 0 | 0 | 16 | 0 | 1140 | 0 | 1 | 22 |
| pp-epoll-spin-0 | boot-stock | 1 | 1017 | 0 | 0 | 17 | 3 | 694 | 0 | 0 | 23 |
| pp-epoll-spin-0 | boot-isolcpus | 1 | 1004 | 0 | 0 | 16 | 4 | 1013 | 0 | 1 | 6 |
| pp-epoll-spin-0 | boot-full-nomitig | 1 | 0 | 0 | 0 | 16 | 0 | 1140 | 0 | 1 | 13 |
| pp-epoll-spin-0 | boot-full | 1 | 0 | 0 | 0 | 16 | 4 | 1006 | 0 | 1 | 23 |
| pp-epoll-spin-10000 | r1-base-0 | 1 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 0 | 3 |
| pp-epoll-spin-10000 | r1-base-2 | 1 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 0 | 12 |
| pp-epoll-spin-10000 | r1-base-4 | 1 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 1 | 13 |
| pp-epoll-spin-10000 | r1-irqlocal | 1 | 52 | 154 | 0 | 8 | | | | | | |
| pp-epoll-spin-10000 | r1-base-6 | 1 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 1 | 4 |
| pp-epoll-spin-10000 | r1-base-8 | 1 | 0 | 0 | 0 | 8 | 4 | 266 | 0 | 0 | 7 |
| pp-epoll-spin-10000 | r1-base-10 | 1 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 0 | 8 |
| pp-epoll-spin-10000 | r1-cstate | 1 | 0 | 0 | 0 | 8 | 4 | 864 | 0 | 1 | 10 |
| pp-epoll-spin-10000 | r1-base-11 | 1 | 0 | 0 | 0 | 8 | 4 | 319 | 0 | 0 | 9 |
| pp-epoll-spin-10000 | boot-stock | 1 | 1003 | 0 | 0 | 8 | 3 | 101 | 0 | 0 | 13 |
| pp-epoll-spin-10000 | boot-isolcpus | 1 | 1002 | 0 | 0 | 8 | 0 | 905 | 0 | 1 | 5 |
| pp-epoll-spin-10000 | boot-full-nomitig | 1 | 0 | 0 | 0 | 8 | 4 | 308 | 0 | 1 | 9 |
| pp-epoll-spin-10000 | boot-full | 1 | 0 | 0 | 0 | 8 | 0 | 1072 | 0 | 0 | 9 |
| pp-recv-busy-0 | r1-base-0 | 1 | 48 | 47 | 0 | 16 | 4 | 1011 | 0 | 0 | 11 |
| pp-recv-busy-0 | r1-housekeeping | 1 | 39 | 38 | 0 | 16 | 0 | 1142 | 0 | 1 | 14 |
| pp-recv-busy-0 | r1-prefer | 1 | 2 | 0 | 0 | 16 | | | | | | |
| pp-recv-busy-0 | r1-base-2 | 1 | 42 | 41 | 0 | 16 | 4 | 1012 | 0 | 0 | 12 |
| pp-recv-busy-0 | r1-noquickack | 1 | 41 | 44 | 0 | 16 | 4 | 1011 | 0 | 1 | 3 |
| pp-recv-busy-0 | r1-nogro | 1 | 45 | 44 | 0 | 16 | 4 | 1011 | 0 | 1 | 5 |
| pp-recv-busy-0 | r1-base-4 | 1 | 48 | 47 | 0 | 16 | 4 | 1010 | 0 | 0 | 12 |
| pp-recv-busy-0 | r1-nocoalesce | 1 | 0 | 0 | 0 | 16 | 4 | 808 | 0 | 0 | 12 |
| pp-recv-busy-0 | r1-irqlocal | 1 | 55 | 194 | 0 | 16 | | | | | | |
| pp-recv-busy-0 | r1-base-6 | 1 | 45 | 44 | 0 | 16 | | | | | | |
| pp-recv-busy-0 | r1-rt | 1 | 534 | 1 | 1 | 16 | 4 | 1006 | 0 | 1 | 14 |
| pp-recv-busy-0 | r1-sibling | 1 | 51 | 50 | 0 | 16 | 4 | 1015 | 0 | 0 | 11 |
| pp-recv-busy-0 | r1-base-8 | 1 | 41 | 40 | 0 | 16 | 4 | 1006 | 0 | 0 | 19 |
| pp-recv-busy-0 | r1-busypoll50 | 1 | 44 | 42 | 0 | 16 | 0 | 1143 | 0 | 0 | 12 |
| pp-recv-busy-0 | r1-budget64 | 1 | 44 | 44 | 0 | 16 | 4 | 1005 | 0 | 0 | 11 |
| pp-recv-busy-0 | r1-base-10 | 1 | 38 | 37 | 0 | 16 | 0 | 1143 | 0 | 0 | 11 |
| pp-recv-busy-0 | r1-cstate | 1 | 42 | 41 | 0 | 16 | | | | | | |
| pp-recv-busy-0 | r1-base-11 | 1 | 43 | 42 | 0 | 16 | 4 | 1011 | 0 | 0 | 4 |
| pp-recv-busy-0 | boot-stock | 1 | 1009 | 0 | 0 | 8 | | | | | | |
| pp-recv-busy-0 | boot-isolcpus | 1 | 1004 | 0 | 0 | 16 | 0 | 1002 | 0 | 0 | 11 |
| pp-recv-busy-0 | boot-full-nomitig | 1 | 41 | 40 | 0 | 16 | 4 | 1012 | 0 | 1 | 14 |
| pp-recv-busy-0 | boot-full | 1 | 43 | 42 | 0 | 16 | 4 | 1010 | 0 | 0 | 11 |
| pp-recv-busy-10000 | r1-base-0 | 1 | 29 | 28 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-housekeeping | 1 | 30 | 29 | 0 | 9 | 4 | 290 | 0 | 0 | 9 |
| pp-recv-busy-10000 | r1-prefer | 1 | 1 | 0 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-base-2 | 1 | 35 | 34 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-noquickack | 1 | 29 | 41 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-nogro | 1 | 34 | 33 | 0 | 8 | 0 | 1073 | 0 | 0 | 8 |
| pp-recv-busy-10000 | r1-base-4 | 1 | 29 | 28 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-nocoalesce | 1 | 0 | 0 | 0 | 8 | 0 | 1073 | 0 | 0 | 11 |
| pp-recv-busy-10000 | r1-irqlocal | 1 | 50 | 157 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-base-6 | 1 | 35 | 34 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-rt | 1 | 542 | 1 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-sibling | 1 | 46 | 44 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-base-8 | 1 | 35 | 35 | 0 | 8 | 4 | 721 | 0 | 0 | 12 |
| pp-recv-busy-10000 | r1-busypoll50 | 1 | 31 | 31 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-budget64 | 1 | 16 | 16 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-base-10 | 1 | 34 | 34 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-cstate | 1 | 42 | 41 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | r1-base-11 | 1 | 27 | 26 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | boot-stock | 1 | 1005 | 0 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | boot-isolcpus | 1 | 1001 | 0 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | boot-full-nomitig | 1 | 48 | 47 | 0 | 8 | | | | | | |
| pp-recv-busy-10000 | boot-full | 1 | 34 | 33 | 0 | 8 | 0 | 1074 | 0 | 0 | 8 |
| pp-uring-spin-0 | r1-base-0 | 1 | 0 | 0 | 0 | 17 | 0 | 1140 | 0 | 1 | 24 |
| pp-uring-spin-0 | r1-housekeeping | 1 | 0 | 0 | 0 | 17 | 4 | 787 | 0 | 1 | 14 |
| pp-uring-spin-0 | r1-base-2 | 1 | 0 | 0 | 0 | 17 | 0 | 1140 | 0 | 1 | 15 |
| pp-uring-spin-0 | r1-noquickack | 1 | 0 | 0 | 0 | 16 | 0 | 1140 | 0 | 1 | 21 |
| pp-uring-spin-0 | r1-nogro | 1 | 0 | 0 | 0 | 17 | 4 | 830 | 0 | 1 | 15 |
| pp-uring-spin-0 | r1-base-4 | 1 | 0 | 0 | 0 | 17 | 0 | 1140 | 0 | 1 | 15 |
| pp-uring-spin-0 | r1-nocoalesce | 1 | 0 | 0 | 0 | 17 | 0 | 1140 | 0 | 1 | 7 |
| pp-uring-spin-0 | r1-irqlocal | 1 | 76 | 258 | 0 | 17 | | | | | | |
| pp-uring-spin-0 | r1-base-6 | 1 | 0 | 0 | 0 | 17 | 0 | 1140 | 0 | 1 | 7 |
| pp-uring-spin-0 | r1-rt | 1 | 0 | 0 | 0 | 17 | 0 | 1140 | 0 | 1 | 14 |
| pp-uring-spin-0 | r1-sibling | 1 | 0 | 0 | 0 | 17 | 0 | 1140 | 0 | 1 | 14 |
| pp-uring-spin-0 | r1-base-8 | 1 | 0 | 0 | 0 | 17 | 0 | 1140 | 0 | 1 | 5 |
| pp-uring-spin-0 | r1-base-10 | 1 | 0 | 0 | 0 | 17 | 4 | 667 | 0 | 1 | 5 |
| pp-uring-spin-0 | r1-cstate | 1 | 0 | 0 | 0 | 17 | 4 | 1012 | 0 | 1 | 14 |
| pp-uring-spin-0 | r1-base-11 | 1 | 0 | 0 | 0 | 17 | 4 | 1010 | 0 | 1 | 15 |
| pp-uring-spin-0 | boot-stock | 1 | 1006 | 0 | 0 | 17 | 0 | 953 | 0 | 0 | 483 |
| pp-uring-spin-0 | boot-isolcpus | 1 | 1004 | 0 | 0 | 17 | 4 | 812 | 0 | 1 | 13 |
| pp-uring-spin-0 | boot-full-nomitig | 1 | 0 | 0 | 0 | 16 | 4 | 607 | 0 | 1 | 14 |
| pp-uring-spin-0 | boot-full | 1 | 0 | 0 | 0 | 17 | 4 | 840 | 0 | 1 | 23 |
| pp-uring-spin-10000 | r1-base-0 | 1 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 0 | 4 |
| pp-uring-spin-10000 | r1-housekeeping | 1 | 0 | 0 | 0 | 8 | 4 | 298 | 0 | 0 | 12 |
| pp-uring-spin-10000 | r1-base-2 | 1 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 1 | 4 |
| pp-uring-spin-10000 | r1-noquickack | 1 | 0 | 0 | 0 | 8 | 4 | 231 | 0 | 0 | 7 |
| pp-uring-spin-10000 | r1-nogro | 1 | 0 | 0 | 0 | 8 | 4 | 923 | 0 | 1 | 9 |
| pp-uring-spin-10000 | r1-base-4 | 1 | 0 | 0 | 0 | 8 | 4 | 267 | 0 | 0 | 9 |
| pp-uring-spin-10000 | r1-nocoalesce | 1 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 1 | 9 |
| pp-uring-spin-10000 | r1-irqlocal | 1 | 50 | 137 | 0 | 8 | | | | | | |
| pp-uring-spin-10000 | r1-base-6 | 1 | 0 | 0 | 0 | 8 | 4 | 920 | 0 | 1 | 8 |
| pp-uring-spin-10000 | r1-rt | 1 | 0 | 0 | 0 | 8 | 4 | 834 | 0 | 1 | 8 |
| pp-uring-spin-10000 | r1-sibling | 1 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 0 | 5 |
| pp-uring-spin-10000 | r1-base-8 | 1 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 0 | 12 |
| pp-uring-spin-10000 | r1-base-10 | 1 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 0 | 4 |
| pp-uring-spin-10000 | r1-cstate | 1 | 0 | 0 | 0 | 8 | 4 | 474 | 0 | 1 | 8 |
| pp-uring-spin-10000 | r1-base-11 | 1 | 0 | 0 | 0 | 8 | 4 | 246 | 0 | 0 | 8 |
| pp-uring-spin-10000 | boot-stock | 1 | 1005 | 0 | 0 | 9 | 7 | 409 | 0 | 0 | 6 |
| pp-uring-spin-10000 | boot-isolcpus | 1 | 1001 | 0 | 0 | 8 | 4 | 163 | 0 | 0 | 4 |
| pp-uring-spin-10000 | boot-full-nomitig | 1 | 0 | 0 | 0 | 8 | 0 | 1071 | 0 | 1 | 12 |
| pp-uring-spin-10000 | boot-full | 1 | 0 | 0 | 0 | 8 | 4 | 241 | 0 | 1 | 12 |
| pp-uring-sqpoll-0 | r1-base-0 | 1 | 0 | 0 | 0 | 17 | 3 | 99 | 47 | 0 | 17 |
| pp-uring-sqpoll-0 | r1-housekeeping | 1 | 0 | 0 | 0 | 17 | 3 | 105 | 50 | 0 | 17 |
| pp-uring-sqpoll-0 | r1-prefer | 1 | 0 | 0 | 0 | 17 | 3 | 14 | 3 | 3 | 17 |
| pp-uring-sqpoll-0 | r1-base-2 | 1 | 0 | 0 | 0 | 17 | 3 | 98 | 46 | 1 | 17 |
| pp-uring-sqpoll-0 | r1-base-4 | 1 | 0 | 0 | 0 | 17 | 3 | 98 | 46 | 1 | 17 |
| pp-uring-sqpoll-0 | r1-base-6 | 1 | 0 | 0 | 0 | 17 | 3 | 92 | 44 | 1 | 17 |
| pp-uring-sqpoll-0 | r1-sibling | 1 | 0 | 0 | 0 | 17 | 3 | 94 | 45 | 0 | 17 |
| pp-uring-sqpoll-0 | r1-base-8 | 1 | 0 | 0 | 0 | 17 | 4 | 995 | 0 | 0 | 4 |
| pp-uring-sqpoll-0 | r1-base-10 | 1 | 0 | 0 | 0 | 17 | 3 | 106 | 50 | 1 | 17 |
| pp-uring-sqpoll-0 | r1-cstate | 1 | 0 | 0 | 0 | 17 | 3 | 108 | 51 | 0 | 17 |
| pp-uring-sqpoll-0 | r1-base-11 | 1 | 0 | 0 | 0 | 17 | 3 | 103 | 49 | 1 | 17 |
| pp-uring-sqpoll-0 | boot-stock | 1 | 1008 | 0 | 0 | 17 | 3 | 1005 | 0 | 0 | 17 |
| pp-uring-sqpoll-0 | boot-isolcpus | 1 | 1004 | 0 | 0 | 17 | 3 | 1003 | 0 | 0 | 17 |
| pp-uring-sqpoll-0 | boot-full-nomitig | 1 | 0 | 0 | 0 | 16 | 3 | 86 | 41 | 1 | 17 |
| pp-uring-sqpoll-0 | boot-full | 1 | 0 | 0 | 0 | 17 | 3 | 102 | 48 | 1 | 17 |
| pp-uring-sqpoll-10000 | r1-base-0 | 1 | 0 | 0 | 0 | 8 | 3 | 61 | 28 | 1 | 8 |
| pp-uring-sqpoll-10000 | r1-housekeeping | 1 | 0 | 0 | 0 | 8 | 3 | 70 | 32 | 1 | 9 |
| pp-uring-sqpoll-10000 | r1-prefer | 1 | 0 | 0 | 0 | 8 | 3 | 12 | 3 | 3 | 8 |
| pp-uring-sqpoll-10000 | r1-base-2 | 1 | 0 | 0 | 0 | 8 | 3 | 58 | 26 | 1 | 8 |
| pp-uring-sqpoll-10000 | r1-base-4 | 1 | 0 | 0 | 0 | 8 | 3 | 58 | 26 | 1 | 8 |
| pp-uring-sqpoll-10000 | r1-base-6 | 1 | 0 | 0 | 0 | 8 | 3 | 59 | 27 | 1 | 8 |
| pp-uring-sqpoll-10000 | r1-sibling | 1 | 0 | 0 | 0 | 8 | 3 | 57 | 26 | 1 | 8 |
| pp-uring-sqpoll-10000 | r1-base-8 | 1 | 0 | 0 | 0 | 8 | 3 | 60 | 27 | 1 | 8 |
| pp-uring-sqpoll-10000 | r1-base-10 | 1 | 0 | 0 | 0 | 8 | 3 | 52 | 23 | 1 | 8 |
| pp-uring-sqpoll-10000 | r1-cstate | 1 | 0 | 0 | 0 | 8 | 3 | 116 | 56 | 0 | 8 |
| pp-uring-sqpoll-10000 | r1-base-11 | 1 | 0 | 0 | 0 | 8 | 3 | 79 | 36 | 1 | 8 |
| pp-uring-sqpoll-10000 | boot-stock | 1 | 1003 | 0 | 0 | 8 | 3 | 1004 | 0 | 0 | 8 |
| pp-uring-sqpoll-10000 | boot-isolcpus | 1 | 1001 | 0 | 0 | 8 | 3 | 1001 | 0 | 0 | 8 |
| pp-uring-sqpoll-10000 | boot-full-nomitig | 1 | 0 | 0 | 0 | 8 | 3 | 66 | 31 | 1 | 8 |
| pp-uring-sqpoll-10000 | boot-full | 1 | 0 | 0 | 0 | 8 | 3 | 74 | 34 | 1 | 8 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### http-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 338.88 at 24.0 s: neither loop stalled long enough: network, hypervisor or untraced; 296.17 at 24.0 s: neither loop stalled long enough: network, hypervisor or untraced; 254.52 at 102.9 s: neither loop stalled long enough: network, hypervisor or untraced; 242.42 at 24.0 s: neither loop stalled long enough: network, hypervisor or untraced; 233.01 at 116.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1051.57 at 68.9 s: neither loop stalled long enough: network, hypervisor or untraced; 1028.07 at 101.7 s: neither loop stalled long enough: network, hypervisor or untraced; 278.22 at 101.7 s: neither loop stalled long enough: network, hypervisor or untraced; 265.77 at 86.4 s: client: softirq TIMER; 251.30 at 101.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 231.76 at 6.1 s: neither loop stalled long enough: network, hypervisor or untraced; 160.76 at 69.4 s: neither loop stalled long enough: network, hypervisor or untraced; 159.60 at 67.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 159.37 at 85.1 s: neither loop stalled long enough: network, hypervisor or untraced; 155.52 at 25.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 160.64 at 36.2 s: neither loop stalled long enough: network, hypervisor or untraced; 153.87 at 96.2 s: neither loop stalled long enough: network, hypervisor or untraced; 136.96 at 93.0 s: neither loop stalled long enough: network, hypervisor or untraced; 131.97 at 93.0 s: neither loop stalled long enough: network, hypervisor or untraced; 130.45 at 93.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 394.24 at 36.6 s: neither loop stalled long enough: network, hypervisor or untraced; 389.89 at 26.1 s: neither loop stalled long enough: network, hypervisor or untraced; 379.68 at 115.8 s: neither loop stalled long enough: network, hypervisor or untraced; 378.17 at 107.7 s: neither loop stalled long enough: network, hypervisor or untraced; 371.42 at 9.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 363.37 at 11.7 s: neither loop stalled long enough: network, hypervisor or untraced; 352.78 at 40.4 s: neither loop stalled long enough: network, hypervisor or untraced; 350.47 at 37.9 s: neither loop stalled long enough: network, hypervisor or untraced; 347.12 at 34.2 s: neither loop stalled long enough: network, hypervisor or untraced; 338.55 at 35.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 984.67 at 115.9 s: neither loop stalled long enough: network, hypervisor or untraced; 398.03 at 83.8 s: neither loop stalled long enough: network, hypervisor or untraced; 396.80 at 83.8 s: neither loop stalled long enough: network, hypervisor or untraced; 361.67 at 83.8 s: neither loop stalled long enough: network, hypervisor or untraced; 356.38 at 83.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 191.63 at 8.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 186.52 at 68.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 147.89 at 114.6 s: neither loop stalled long enough: network, hypervisor or untraced; 147.20 at 71.6 s: neither loop stalled long enough: network, hypervisor or untraced; 144.42 at 85.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 371.58 at 44.8 s: neither loop stalled long enough: network, hypervisor or untraced; 368.86 at 96.4 s: neither loop stalled long enough: network, hypervisor or untraced; 368.63 at 89.6 s: neither loop stalled long enough: network, hypervisor or untraced; 368.19 at 58.7 s: neither loop stalled long enough: network, hypervisor or untraced; 366.92 at 10.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 510.88 at 21.3 s: neither loop stalled long enough: network, hypervisor or untraced; 306.29 at 90.5 s: neither loop stalled long enough: network, hypervisor or untraced; 267.18 at 59.3 s: neither loop stalled long enough: network, hypervisor or untraced; 235.87 at 4.7 s: neither loop stalled long enough: network, hypervisor or untraced; 231.49 at 1.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 231.15 at 13.6 s: server: softirq TIMER; 210.07 at 31.8 s: neither loop stalled long enough: network, hypervisor or untraced; 205.27 at 84.9 s: neither loop stalled long enough: network, hypervisor or untraced; 202.45 at 109.5 s: neither loop stalled long enough: network, hypervisor or untraced; 202.43 at 73.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1054.43 at 8.4 s: neither loop stalled long enough: network, hypervisor or untraced; 346.80 at 88.6 s: neither loop stalled long enough: network, hypervisor or untraced; 299.72 at 88.7 s: neither loop stalled long enough: network, hypervisor or untraced; 299.64 at 47.2 s: neither loop stalled long enough: network, hypervisor or untraced; 299.16 at 88.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 155.22 at 97.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 145.74 at 37.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 133.43 at 1.1 s: neither loop stalled long enough: network, hypervisor or untraced; 129.78 at 89.0 s: neither loop stalled long enough: network, hypervisor or untraced; 125.28 at 95.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-busypoll50 | 193.17 at 31.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 187.75 at 91.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 147.00 at 89.3 s: neither loop stalled long enough: network, hypervisor or untraced; 146.43 at 84.3 s: neither loop stalled long enough: network, hypervisor or untraced; 143.17 at 66.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-budget64 | 436.45 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 390.46 at 21.5 s: neither loop stalled long enough: network, hypervisor or untraced; 350.69 at 86.6 s: neither loop stalled long enough: network, hypervisor or untraced; 293.27 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 283.32 at 86.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 263.97 at 30.1 s: neither loop stalled long enough: network, hypervisor or untraced; 236.58 at 22.6 s: neither loop stalled long enough: network, hypervisor or untraced; 216.90 at 25.6 s: server: softirq TIMER; 210.18 at 30.1 s: neither loop stalled long enough: network, hypervisor or untraced; 210.00 at 91.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 243.46 at 26.5 s: neither loop stalled long enough: network, hypervisor or untraced; 236.77 at 29.4 s: server: softirq TIMER; 222.69 at 7.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 180.71 at 68.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 171.08 at 103.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 1011.07 at 79.1 s: neither loop stalled long enough: network, hypervisor or untraced; 347.70 at 48.6 s: neither loop stalled long enough: network, hypervisor or untraced; 347.59 at 89.1 s: neither loop stalled long enough: network, hypervisor or untraced; 334.28 at 87.8 s: neither loop stalled long enough: network, hypervisor or untraced; 334.10 at 62.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 255.53 at 94.7 s: neither loop stalled long enough: network, hypervisor or untraced; 154.94 at 86.7 s: neither loop stalled long enough: network, hypervisor or untraced; 146.11 at 54.7 s: neither loop stalled long enough: network, hypervisor or untraced; 145.97 at 118.7 s: neither loop stalled long enough: network, hypervisor or untraced; 140.90 at 19.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 166.57 at 10.9 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 165.32 at 18.5 s: neither loop stalled long enough: network, hypervisor or untraced; 152.22 at 71.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 139.20 at 78.8 s: neither loop stalled long enough: network, hypervisor or untraced; 136.51 at 70.9 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full-nomitig | 1045.66 at 25.7 s: neither loop stalled long enough: network, hypervisor or untraced; 251.98 at 7.4 s: neither loop stalled long enough: network, hypervisor or untraced; 205.58 at 7.4 s: neither loop stalled long enough: network, hypervisor or untraced; 195.44 at 103.0 s: neither loop stalled long enough: network, hypervisor or untraced; 193.78 at 103.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 269.07 at 109.3 s: neither loop stalled long enough: network, hypervisor or untraced; 178.37 at 48.3 s: neither loop stalled long enough: network, hypervisor or untraced; 177.93 at 53.3 s: neither loop stalled long enough: network, hypervisor or untraced; 165.11 at 61.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 164.30 at 1.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |

### md-recv-busy

| condition | worst events |
|---|---|
| r1-base-0 | 38.49 at 46.8 s: client: loop work (13 items); 38.30 at 46.8 s: client: loop work (13 items); 38.10 at 46.8 s: client: loop work (13 items); 37.93 at 17.3 s: client: loop work (10 items); 37.89 at 46.8 s: client: loop work (13 items) |
| r1-housekeeping | 38.47 at 48.3 s: client: loop work (2 items); 38.30 at 48.3 s: client: loop work (2 items); 38.21 at 28.7 s: client: loop work (2 items); 38.06 at 28.7 s: client: loop work (2 items); 38.04 at 32.2 s: client: loop work (6 items) |
| r1-prefer | 14.15 at 19.0 s: client: loop work (43 items); 14.12 at 41.6 s: neither loop stalled long enough: network, hypervisor or untraced; 14.08 at 40.7 s: client: loop work (30 items); 14.01 at 19.0 s: client: loop work (43 items); 13.98 at 41.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 42.90 at 12.5 s: client: loop work (30 items); 42.75 at 12.5 s: client: loop work (30 items); 42.60 at 12.5 s: client: loop work (30 items); 42.46 at 12.5 s: client: loop work (30 items); 42.32 at 12.5 s: client: loop work (30 items) |
| r1-noquickack | 45.37 at 36.5 s: client: loop work (19 items); 45.23 at 36.5 s: client: loop work (19 items); 45.08 at 36.5 s: client: loop work (19 items); 44.94 at 36.5 s: client: loop work (19 items); 44.79 at 36.5 s: client: loop work (19 items) |
| r1-nogro | 38.02 at 37.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 37.63 at 54.9 s: neither loop stalled long enough: network, hypervisor or untraced; 37.45 at 54.9 s: neither loop stalled long enough: network, hypervisor or untraced; 37.27 at 54.9 s: neither loop stalled long enough: network, hypervisor or untraced; 37.10 at 54.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 42.19 at 35.4 s: client: loop work (3 items); 42.03 at 35.4 s: client: loop work (3 items); 41.87 at 35.4 s: client: loop work (3 items); 38.98 at 34.8 s: client: softirq NET_RX; 38.79 at 34.8 s: client: softirq NET_RX |
| r1-nocoalesce | 22.38 at 53.8 s: neither loop stalled long enough: network, hypervisor or untraced; 21.44 at 59.9 s: neither loop stalled long enough: network, hypervisor or untraced; 21.30 at 59.9 s: neither loop stalled long enough: network, hypervisor or untraced; 21.16 at 59.9 s: neither loop stalled long enough: network, hypervisor or untraced; 20.86 at 48.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 52.25 at 47.5 s: client: softirq NET_RX; 49.40 at 41.0 s: client: softirq NET_RX; 49.20 at 41.0 s: client: softirq NET_RX; 46.84 at 56.4 s: client: loop work (20 items); 46.70 at 56.4 s: client: loop work (20 items) |
| r1-base-6 | 46.95 at 13.7 s: client: mostly unexplained (+preempted by kworker/1:1); 46.79 at 13.7 s: client: mostly unexplained (+preempted by kworker/1:1); 42.38 at 20.4 s: client: loop work (9 items); 42.23 at 20.4 s: client: loop work (9 items); 42.09 at 20.4 s: client: loop work (9 items) |
| r1-rt | 41.13 at 17.7 s: client: mostly unexplained (+preempted by kworker/1:1); 40.97 at 17.7 s: client: mostly unexplained (+preempted by kworker/1:1); 29.93 at 42.6 s: client: loop work (8 items); 29.77 at 42.6 s: client: loop work (8 items); 29.63 at 42.6 s: client: loop work (8 items) |
| r1-sibling | 43.59 at 20.0 s: client: loop work (11 items); 43.41 at 20.0 s: client: loop work (11 items); 43.21 at 20.0 s: client: loop work (11 items); 43.00 at 20.0 s: client: loop work (11 items); 42.87 at 18.2 s: client: loop work (21 items) |
| r1-base-8 | 32.19 at 30.8 s: client: loop work (19 items); 32.03 at 30.8 s: client: loop work (19 items); 31.89 at 30.8 s: client: loop work (19 items); 31.75 at 30.8 s: client: loop work (19 items); 31.61 at 30.8 s: client: loop work (19 items) |
| r1-busypoll50 | 41.25 at 30.2 s: client: mostly unexplained (+softirq NET_RX); 40.99 at 32.6 s: client: loop work (17 items); 40.85 at 32.6 s: client: loop work (17 items); 40.71 at 32.6 s: client: loop work (17 items); 40.58 at 32.6 s: client: loop work (17 items) |
| r1-budget64 | 41.32 at 20.0 s: client: loop work (19 items); 41.17 at 20.0 s: client: loop work (19 items); 41.02 at 20.0 s: client: loop work (19 items); 40.88 at 20.0 s: client: loop work (19 items); 40.73 at 20.0 s: client: loop work (19 items) |
| r1-base-10 | 41.45 at 17.4 s: client: loop work (11 items); 41.30 at 17.4 s: client: loop work (11 items); 41.15 at 17.4 s: client: loop work (11 items); 40.93 at 17.4 s: client: loop work (11 items); 40.75 at 49.0 s: client: loop work (11 items) |
| r1-cstate | 41.24 at 42.1 s: client: loop work (3 items); 41.08 at 42.1 s: client: loop work (3 items); 40.92 at 42.1 s: client: loop work (3 items); 40.10 at 49.2 s: client: loop work (5 items); 39.96 at 49.2 s: client: loop work (5 items) |
| r1-base-11 | 40.79 at 11.5 s: client: loop work (3 items); 40.63 at 11.5 s: client: loop work (3 items); 40.47 at 11.5 s: client: loop work (3 items); 40.38 at 15.6 s: client: mostly unexplained (+preempted by kworker/1:1); 40.23 at 15.6 s: client: mostly unexplained (+preempted by kworker/1:1) |
| boot-stock | 51.91 at 26.8 s: client: loop work (8 items); 51.69 at 26.8 s: client: loop work (8 items); 51.49 at 26.8 s: client: loop work (8 items); 51.29 at 26.8 s: client: loop work (8 items); 51.09 at 26.8 s: client: loop work (8 items) |
| boot-isolcpus | 27.78 at 51.2 s: neither loop stalled long enough: network, hypervisor or untraced; 27.45 at 13.6 s: neither loop stalled long enough: network, hypervisor or untraced; 27.32 at 13.6 s: neither loop stalled long enough: network, hypervisor or untraced; 27.18 at 13.6 s: neither loop stalled long enough: network, hypervisor or untraced; 27.12 at 51.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 38.20 at 47.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 38.13 at 36.8 s: neither loop stalled long enough: network, hypervisor or untraced; 37.94 at 57.4 s: neither loop stalled long enough: network, hypervisor or untraced; 37.83 at 20.6 s: client: loop work (13 items); 37.75 at 57.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 39.73 at 14.6 s: client: loop work (10 items); 39.56 at 14.6 s: client: loop work (10 items); 39.55 at 37.6 s: client: loop work (20 items); 39.39 at 37.6 s: client: loop work (20 items); 39.38 at 14.6 s: client: loop work (10 items) |

### noise

| condition | worst events |
|---|---|
| r1-base-0 | 26.77 at 27.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.62 at 5.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 14.53 at 87.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 12.04 at 52.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 11.28 at 0.7 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-housekeeping | 30.58 at 51.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 28.86 at 46.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.01 at 1.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 13.67 at 106.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 11.95 at 50.4 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-prefer | 33.10 at 11.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 24.95 at 56.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 20.16 at 80.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.71 at 2.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 12.04 at 116.2 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-2 | 37.85 at 1.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 33.94 at 0.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 22.44 at 70.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.01 at 60.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 15.92 at 41.7 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-noquickack | 140.90 at 17.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 110.20 at 77.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 98.17 at 48.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 68.79 at 108.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 24.16 at 43.2 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nogro | 164.47 at 90.7 s: softirq TIMER; 150.81 at 2.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 110.00 at 62.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 104.08 at 33.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 75.70 at 93.7 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-4 | 128.10 at 49.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 116.32 at 20.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 109.88 at 109.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 70.19 at 80.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 19.86 at 11.5 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nocoalesce | 142.72 at 9.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 101.03 at 69.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 101.02 at 40.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 71.78 at 100.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 41.48 at 2.4 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-irqlocal | 146.40 at 2.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 120.73 at 62.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 108.17 at 33.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 78.62 at 93.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 62.64 at 40.1 s: softirq TIMER |
| r1-base-6 | 3813.30 at 9.3 s: mostly unexplained (+softirq NET_RX); 147.19 at 2.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 122.01 at 62.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 113.15 at 33.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 77.84 at 93.7 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-rt | 53.12 at 67.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 30.70 at 47.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 26.46 at 77.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 23.44 at 56.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 19.05 at 1.9 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-sibling | 328.24 at 91.0 s: softirq TIMER; 116.66 at 47.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 50.80 at 54.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 28.27 at 41.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 25.99 at 84.9 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-8 | 139.89 at 20.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 104.25 at 80.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 97.84 at 51.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 72.43 at 111.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 19.47 at 6.2 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-busypoll50 | 255.03 at 2.3 s: softirq TIMER; 130.79 at 33.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 115.97 at 5.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 108.48 at 93.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 71.43 at 65.4 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-budget64 | 122.67 at 24.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 112.04 at 48.7 s: softirq TIMER; 106.08 at 84.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 100.15 at 56.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 74.73 at 116.0 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 121.99 at 16.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 111.14 at 76.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 110.81 at 95.3 s: softirq TIMER; 98.36 at 47.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 70.06 at 107.7 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-cstate | 304.11 at 97.3 s: softirq TIMER; 125.81 at 27.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 100.86 at 87.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 94.51 at 58.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 80.52 at 118.5 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-11 | 347.61 at 101.8 s: softirq TIMER; 127.68 at 41.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 103.53 at 101.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 101.30 at 12.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 99.67 at 72.3 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-stock | 146.95 at 116.9 s: softirq TIMER; 146.11 at 86.9 s: softirq TIMER; 144.29 at 78.9 s: softirq TIMER; 143.02 at 60.9 s: softirq TIMER; 141.44 at 110.9 s: softirq TIMER |
| boot-isolcpus | 52.30 at 46.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 41.22 at 79.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 38.58 at 117.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 34.89 at 56.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 29.97 at 13.0 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full-nomitig | 57.67 at 22.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 33.55 at 13.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 32.58 at 42.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 30.99 at 62.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.75 at 73.2 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full | 133.90 at 44.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 108.30 at 104.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 80.06 at 72.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 57.17 at 0.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 40.36 at 79.0 s: unexplained: no kernel event (hypervisor or in-kernel time) |

### pp-epoll-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 438.11 at 51.6 s: neither loop stalled long enough: network, hypervisor or untraced; 424.99 at 51.6 s: neither loop stalled long enough: network, hypervisor or untraced; 408.06 at 55.0 s: neither loop stalled long enough: network, hypervisor or untraced; 402.03 at 29.0 s: neither loop stalled long enough: network, hypervisor or untraced; 399.28 at 43.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 630.86 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced; 532.76 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced; 509.38 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced; 482.38 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced; 466.18 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 421.13 at 16.6 s: neither loop stalled long enough: network, hypervisor or untraced; 345.94 at 17.4 s: neither loop stalled long enough: network, hypervisor or untraced; 308.40 at 17.0 s: neither loop stalled long enough: network, hypervisor or untraced; 282.90 at 17.8 s: neither loop stalled long enough: network, hypervisor or untraced; 234.28 at 43.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 275.95 at 27.1 s: neither loop stalled long enough: network, hypervisor or untraced; 244.74 at 47.1 s: neither loop stalled long enough: network, hypervisor or untraced; 242.31 at 47.2 s: neither loop stalled long enough: network, hypervisor or untraced; 241.03 at 20.6 s: neither loop stalled long enough: network, hypervisor or untraced; 229.15 at 47.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 490.06 at 55.6 s: neither loop stalled long enough: network, hypervisor or untraced; 460.64 at 43.4 s: neither loop stalled long enough: network, hypervisor or untraced; 432.25 at 28.8 s: neither loop stalled long enough: network, hypervisor or untraced; 416.92 at 55.6 s: neither loop stalled long enough: network, hypervisor or untraced; 414.12 at 18.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 894.63 at 8.1 s: neither loop stalled long enough: network, hypervisor or untraced; 439.16 at 8.5 s: neither loop stalled long enough: network, hypervisor or untraced; 420.88 at 8.6 s: neither loop stalled long enough: network, hypervisor or untraced; 417.96 at 41.2 s: neither loop stalled long enough: network, hypervisor or untraced; 403.68 at 8.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 218.94 at 12.4 s: neither loop stalled long enough: network, hypervisor or untraced; 199.00 at 54.4 s: neither loop stalled long enough: network, hypervisor or untraced; 197.44 at 50.4 s: neither loop stalled long enough: network, hypervisor or untraced; 194.33 at 28.4 s: neither loop stalled long enough: network, hypervisor or untraced; 194.11 at 52.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 304.29 at 32.3 s: neither loop stalled long enough: network, hypervisor or untraced; 219.25 at 34.1 s: neither loop stalled long enough: network, hypervisor or untraced; 213.45 at 21.1 s: neither loop stalled long enough: network, hypervisor or untraced; 196.24 at 1.1 s: neither loop stalled long enough: network, hypervisor or untraced; 194.54 at 33.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 1041.06 at 52.8 s: neither loop stalled long enough: network, hypervisor or untraced; 340.68 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced; 300.36 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced; 241.37 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced; 233.14 at 51.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 549.86 at 18.1 s: neither loop stalled long enough: network, hypervisor or untraced; 503.01 at 29.1 s: neither loop stalled long enough: network, hypervisor or untraced; 500.87 at 29.2 s: neither loop stalled long enough: network, hypervisor or untraced; 498.85 at 16.0 s: neither loop stalled long enough: network, hypervisor or untraced; 493.62 at 39.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 510.03 at 38.5 s: neither loop stalled long enough: network, hypervisor or untraced; 468.54 at 38.5 s: neither loop stalled long enough: network, hypervisor or untraced; 406.44 at 38.4 s: neither loop stalled long enough: network, hypervisor or untraced; 399.25 at 38.5 s: neither loop stalled long enough: network, hypervisor or untraced; 386.85 at 38.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 245.79 at 22.2 s: neither loop stalled long enough: network, hypervisor or untraced; 243.85 at 31.3 s: neither loop stalled long enough: network, hypervisor or untraced; 232.84 at 33.3 s: neither loop stalled long enough: network, hypervisor or untraced; 224.64 at 35.2 s: neither loop stalled long enough: network, hypervisor or untraced; 209.55 at 51.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1035.94 at 26.4 s: neither loop stalled long enough: network, hypervisor or untraced; 571.10 at 31.7 s: neither loop stalled long enough: network, hypervisor or untraced; 535.87 at 49.2 s: neither loop stalled long enough: network, hypervisor or untraced; 519.18 at 27.4 s: neither loop stalled long enough: network, hypervisor or untraced; 515.06 at 27.4 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-epoll-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 411.31 at 95.5 s: neither loop stalled long enough: network, hypervisor or untraced; 397.88 at 75.1 s: neither loop stalled long enough: network, hypervisor or untraced; 395.98 at 79.8 s: neither loop stalled long enough: network, hypervisor or untraced; 389.61 at 66.0 s: neither loop stalled long enough: network, hypervisor or untraced; 379.97 at 108.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 395.01 at 54.5 s: neither loop stalled long enough: network, hypervisor or untraced; 370.38 at 52.6 s: neither loop stalled long enough: network, hypervisor or untraced; 365.10 at 83.1 s: neither loop stalled long enough: network, hypervisor or untraced; 361.78 at 54.5 s: neither loop stalled long enough: network, hypervisor or untraced; 357.46 at 35.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1053.62 at 34.6 s: neither loop stalled long enough: network, hypervisor or untraced; 298.80 at 55.1 s: client: softirq TIMER; 268.22 at 58.1 s: server: softirq TIMER; 243.95 at 6.8 s: neither loop stalled long enough: network, hypervisor or untraced; 243.95 at 15.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-irqlocal | 335.65 at 114.2 s: neither loop stalled long enough: network, hypervisor or untraced; 213.84 at 90.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 185.82 at 30.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 184.08 at 95.7 s: server: softirq TIMER; 175.13 at 119.5 s: client: mostly unexplained (+softirq NET_RX) |
| r1-base-6 | 1011.40 at 57.7 s: neither loop stalled long enough: network, hypervisor or untraced; 333.27 at 23.2 s: neither loop stalled long enough: network, hypervisor or untraced; 265.37 at 45.5 s: neither loop stalled long enough: network, hypervisor or untraced; 253.32 at 39.5 s: neither loop stalled long enough: network, hypervisor or untraced; 243.56 at 23.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 186.72 at 76.6 s: neither loop stalled long enough: network, hypervisor or untraced; 183.24 at 86.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 178.21 at 15.7 s: neither loop stalled long enough: network, hypervisor or untraced; 173.69 at 15.7 s: neither loop stalled long enough: network, hypervisor or untraced; 162.07 at 118.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 978.73 at 90.8 s: neither loop stalled long enough: network, hypervisor or untraced; 242.24 at 110.3 s: neither loop stalled long enough: network, hypervisor or untraced; 240.42 at 105.3 s: neither loop stalled long enough: network, hypervisor or untraced; 239.21 at 11.6 s: neither loop stalled long enough: network, hypervisor or untraced; 231.28 at 28.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 204.20 at 21.9 s: neither loop stalled long enough: network, hypervisor or untraced; 198.29 at 56.7 s: neither loop stalled long enough: network, hypervisor or untraced; 194.65 at 94.6 s: neither loop stalled long enough: network, hypervisor or untraced; 192.18 at 21.9 s: neither loop stalled long enough: network, hypervisor or untraced; 190.93 at 21.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 999.45 at 62.3 s: neither loop stalled long enough: network, hypervisor or untraced; 220.92 at 53.9 s: neither loop stalled long enough: network, hypervisor or untraced; 176.55 at 9.3 s: neither loop stalled long enough: network, hypervisor or untraced; 167.24 at 33.8 s: neither loop stalled long enough: network, hypervisor or untraced; 155.54 at 78.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 662.28 at 100.1 s: neither loop stalled long enough: network, hypervisor or untraced; 656.64 at 118.8 s: neither loop stalled long enough: network, hypervisor or untraced; 636.72 at 70.3 s: neither loop stalled long enough: network, hypervisor or untraced; 603.59 at 20.2 s: neither loop stalled long enough: network, hypervisor or untraced; 601.76 at 96.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 538.73 at 31.7 s: neither loop stalled long enough: network, hypervisor or untraced; 513.73 at 59.5 s: neither loop stalled long enough: network, hypervisor or untraced; 506.48 at 82.6 s: neither loop stalled long enough: network, hypervisor or untraced; 505.71 at 58.9 s: neither loop stalled long enough: network, hypervisor or untraced; 504.74 at 38.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 166.61 at 91.0 s: server: softirq TIMER; 162.01 at 80.1 s: neither loop stalled long enough: network, hypervisor or untraced; 161.25 at 42.6 s: neither loop stalled long enough: network, hypervisor or untraced; 153.75 at 83.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 147.44 at 26.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 534.91 at 83.5 s: neither loop stalled long enough: network, hypervisor or untraced; 453.49 at 102.8 s: neither loop stalled long enough: network, hypervisor or untraced; 391.64 at 68.1 s: neither loop stalled long enough: network, hypervisor or untraced; 381.90 at 68.1 s: neither loop stalled long enough: network, hypervisor or untraced; 379.83 at 50.2 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-0

| condition | worst events |
|---|---|
| r1-base-0 | 199.64 at 16.1 s: neither loop stalled long enough: network, hypervisor or untraced; 194.42 at 7.9 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 186.41 at 23.3 s: neither loop stalled long enough: network, hypervisor or untraced; 170.70 at 53.8 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 146.55 at 15.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 260.84 at 39.0 s: neither loop stalled long enough: network, hypervisor or untraced; 180.22 at 18.7 s: neither loop stalled long enough: network, hypervisor or untraced; 178.81 at 45.7 s: neither loop stalled long enough: network, hypervisor or untraced; 171.70 at 11.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 166.81 at 29.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 256.81 at 13.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 188.48 at 1.9 s: neither loop stalled long enough: network, hypervisor or untraced; 179.92 at 5.9 s: neither loop stalled long enough: network, hypervisor or untraced; 175.56 at 3.9 s: neither loop stalled long enough: network, hypervisor or untraced; 173.36 at 20.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-2 | 1042.84 at 59.4 s: neither loop stalled long enough: network, hypervisor or untraced; 231.53 at 24.6 s: neither loop stalled long enough: network, hypervisor or untraced; 217.88 at 38.9 s: server: softirq TIMER; 184.93 at 43.1 s: neither loop stalled long enough: network, hypervisor or untraced; 173.41 at 25.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-noquickack | 366.12 at 14.6 s: neither loop stalled long enough: network, hypervisor or untraced; 199.41 at 41.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 175.04 at 17.6 s: server: softirq TIMER; 171.39 at 31.0 s: neither loop stalled long enough: network, hypervisor or untraced; 166.14 at 17.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 415.77 at 17.3 s: neither loop stalled long enough: network, hypervisor or untraced; 310.57 at 55.6 s: neither loop stalled long enough: network, hypervisor or untraced; 295.10 at 17.3 s: neither loop stalled long enough: network, hypervisor or untraced; 251.41 at 17.3 s: neither loop stalled long enough: network, hypervisor or untraced; 193.91 at 17.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 192.07 at 15.3 s: client: mostly unexplained (+softirq NET_RX); 168.76 at 46.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 163.94 at 9.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 145.93 at 12.0 s: neither loop stalled long enough: network, hypervisor or untraced; 145.00 at 44.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 1049.85 at 17.0 s: neither loop stalled long enough: network, hypervisor or untraced; 463.08 at 52.6 s: neither loop stalled long enough: network, hypervisor or untraced; 428.85 at 52.6 s: neither loop stalled long enough: network, hypervisor or untraced; 349.39 at 53.7 s: neither loop stalled long enough: network, hypervisor or untraced; 234.67 at 53.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 206.16 at 12.5 s: neither loop stalled long enough: network, hypervisor or untraced; 202.43 at 22.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 180.55 at 54.3 s: client: mostly unexplained (+irq enp39s0-Tx-Rx-1); 168.71 at 1.5 s: neither loop stalled long enough: network, hypervisor or untraced; 168.51 at 28.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 226.75 at 9.9 s: neither loop stalled long enough: network, hypervisor or untraced; 206.31 at 5.8 s: neither loop stalled long enough: network, hypervisor or untraced; 184.12 at 28.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 183.78 at 42.4 s: neither loop stalled long enough: network, hypervisor or untraced; 176.57 at 12.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 184.61 at 12.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 181.52 at 30.8 s: neither loop stalled long enough: network, hypervisor or untraced; 172.00 at 18.9 s: neither loop stalled long enough: network, hypervisor or untraced; 162.75 at 20.0 s: neither loop stalled long enough: network, hypervisor or untraced; 159.19 at 3.3 s: server: softirq TIMER |
| r1-sibling | 241.07 at 45.2 s: neither loop stalled long enough: network, hypervisor or untraced; 217.09 at 12.6 s: neither loop stalled long enough: network, hypervisor or untraced; 185.77 at 19.0 s: neither loop stalled long enough: network, hypervisor or untraced; 173.72 at 32.4 s: neither loop stalled long enough: network, hypervisor or untraced; 142.35 at 22.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 280.77 at 3.8 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 260.79 at 46.6 s: server: softirq TIMER; 195.20 at 38.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 170.99 at 16.2 s: neither loop stalled long enough: network, hypervisor or untraced; 166.16 at 9.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-busypoll50 | 1049.93 at 14.1 s: neither loop stalled long enough: network, hypervisor or untraced; 1035.17 at 17.1 s: neither loop stalled long enough: network, hypervisor or untraced; 298.36 at 14.1 s: neither loop stalled long enough: network, hypervisor or untraced; 298.05 at 14.1 s: neither loop stalled long enough: network, hypervisor or untraced; 292.03 at 14.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-budget64 | 317.46 at 58.5 s: neither loop stalled long enough: network, hypervisor or untraced; 235.44 at 15.9 s: neither loop stalled long enough: network, hypervisor or untraced; 230.13 at 37.9 s: neither loop stalled long enough: network, hypervisor or untraced; 207.62 at 36.2 s: neither loop stalled long enough: network, hypervisor or untraced; 184.66 at 32.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 205.42 at 18.2 s: neither loop stalled long enough: network, hypervisor or untraced; 192.83 at 28.3 s: neither loop stalled long enough: network, hypervisor or untraced; 192.47 at 38.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 185.82 at 57.4 s: neither loop stalled long enough: network, hypervisor or untraced; 185.54 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 482.76 at 16.0 s: neither loop stalled long enough: network, hypervisor or untraced; 278.42 at 48.3 s: neither loop stalled long enough: network, hypervisor or untraced; 192.70 at 46.9 s: neither loop stalled long enough: network, hypervisor or untraced; 191.36 at 43.9 s: neither loop stalled long enough: network, hypervisor or untraced; 181.92 at 38.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 275.21 at 25.2 s: neither loop stalled long enough: network, hypervisor or untraced; 207.76 at 32.2 s: neither loop stalled long enough: network, hypervisor or untraced; 200.23 at 56.2 s: neither loop stalled long enough: network, hypervisor or untraced; 184.92 at 6.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 167.03 at 32.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 188.57 at 14.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 181.06 at 26.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 180.36 at 10.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 177.69 at 41.5 s: client: softirq TIMER; 175.30 at 15.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-isolcpus | 984.62 at 52.4 s: neither loop stalled long enough: network, hypervisor or untraced; 265.43 at 11.4 s: neither loop stalled long enough: network, hypervisor or untraced; 265.39 at 45.3 s: server: softirq TIMER; 217.85 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced; 207.47 at 5.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 416.89 at 54.7 s: neither loop stalled long enough: network, hypervisor or untraced; 299.42 at 9.1 s: client: softirq TIMER; 287.71 at 54.7 s: neither loop stalled long enough: network, hypervisor or untraced; 246.42 at 54.7 s: neither loop stalled long enough: network, hypervisor or untraced; 217.65 at 54.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 694.49 at 52.7 s: neither loop stalled long enough: network, hypervisor or untraced; 335.02 at 10.0 s: client: softirq TIMER; 311.31 at 35.8 s: neither loop stalled long enough: network, hypervisor or untraced; 176.88 at 23.8 s: neither loop stalled long enough: network, hypervisor or untraced; 164.11 at 7.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |

### pp-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 331.27 at 9.9 s: neither loop stalled long enough: network, hypervisor or untraced; 281.36 at 111.3 s: server: softirq TIMER; 279.25 at 9.9 s: neither loop stalled long enough: network, hypervisor or untraced; 244.87 at 7.0 s: neither loop stalled long enough: network, hypervisor or untraced; 225.67 at 9.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 994.56 at 81.9 s: neither loop stalled long enough: network, hypervisor or untraced; 234.20 at 102.5 s: neither loop stalled long enough: network, hypervisor or untraced; 216.91 at 70.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 213.06 at 54.4 s: neither loop stalled long enough: network, hypervisor or untraced; 175.62 at 102.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 392.79 at 100.6 s: neither loop stalled long enough: network, hypervisor or untraced; 369.73 at 34.5 s: client: softirq TIMER; 354.87 at 100.6 s: neither loop stalled long enough: network, hypervisor or untraced; 318.21 at 100.6 s: neither loop stalled long enough: network, hypervisor or untraced; 268.23 at 100.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 254.20 at 58.6 s: neither loop stalled long enough: network, hypervisor or untraced; 205.16 at 36.4 s: neither loop stalled long enough: network, hypervisor or untraced; 205.04 at 45.6 s: neither loop stalled long enough: network, hypervisor or untraced; 185.77 at 15.7 s: neither loop stalled long enough: network, hypervisor or untraced; 185.62 at 50.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 401.18 at 77.2 s: neither loop stalled long enough: network, hypervisor or untraced; 361.08 at 78.4 s: neither loop stalled long enough: network, hypervisor or untraced; 360.31 at 6.3 s: neither loop stalled long enough: network, hypervisor or untraced; 358.10 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced; 357.87 at 22.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 196.23 at 45.5 s: client: mostly unexplained (+softirq NET_RX); 175.38 at 93.3 s: neither loop stalled long enough: network, hypervisor or untraced; 166.76 at 105.3 s: neither loop stalled long enough: network, hypervisor or untraced; 165.54 at 77.3 s: neither loop stalled long enough: network, hypervisor or untraced; 162.10 at 112.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 322.58 at 29.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 277.15 at 90.5 s: neither loop stalled long enough: network, hypervisor or untraced; 211.67 at 92.4 s: server: softirq TIMER; 208.69 at 4.0 s: neither loop stalled long enough: network, hypervisor or untraced; 204.86 at 2.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 1042.13 at 103.9 s: neither loop stalled long enough: network, hypervisor or untraced; 363.11 at 49.2 s: neither loop stalled long enough: network, hypervisor or untraced; 355.29 at 41.0 s: neither loop stalled long enough: network, hypervisor or untraced; 354.42 at 62.8 s: neither loop stalled long enough: network, hypervisor or untraced; 354.19 at 20.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 359.49 at 72.1 s: neither loop stalled long enough: network, hypervisor or untraced; 357.11 at 79.0 s: neither loop stalled long enough: network, hypervisor or untraced; 356.11 at 14.2 s: neither loop stalled long enough: network, hypervisor or untraced; 351.36 at 85.9 s: neither loop stalled long enough: network, hypervisor or untraced; 350.38 at 79.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 160.35 at 89.7 s: neither loop stalled long enough: network, hypervisor or untraced; 155.97 at 82.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 150.45 at 108.7 s: neither loop stalled long enough: network, hypervisor or untraced; 149.92 at 106.6 s: neither loop stalled long enough: network, hypervisor or untraced; 147.41 at 55.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 287.06 at 18.7 s: neither loop stalled long enough: network, hypervisor or untraced; 233.32 at 117.6 s: neither loop stalled long enough: network, hypervisor or untraced; 225.15 at 41.8 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 205.59 at 34.6 s: neither loop stalled long enough: network, hypervisor or untraced; 192.95 at 55.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1050.71 at 85.5 s: neither loop stalled long enough: network, hypervisor or untraced; 1004.11 at 24.0 s: neither loop stalled long enough: network, hypervisor or untraced; 363.04 at 97.9 s: neither loop stalled long enough: network, hypervisor or untraced; 360.12 at 18.5 s: neither loop stalled long enough: network, hypervisor or untraced; 356.66 at 109.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 369.23 at 91.6 s: neither loop stalled long enough: network, hypervisor or untraced; 368.69 at 79.8 s: neither loop stalled long enough: network, hypervisor or untraced; 363.24 at 30.1 s: neither loop stalled long enough: network, hypervisor or untraced; 358.35 at 83.5 s: neither loop stalled long enough: network, hypervisor or untraced; 357.48 at 103.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-busypoll50 | 308.13 at 77.3 s: neither loop stalled long enough: network, hypervisor or untraced; 213.94 at 55.2 s: neither loop stalled long enough: network, hypervisor or untraced; 193.85 at 78.1 s: neither loop stalled long enough: network, hypervisor or untraced; 178.87 at 72.7 s: neither loop stalled long enough: network, hypervisor or untraced; 169.76 at 83.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-budget64 | 320.50 at 82.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 268.16 at 46.2 s: neither loop stalled long enough: network, hypervisor or untraced; 236.74 at 105.1 s: neither loop stalled long enough: network, hypervisor or untraced; 183.05 at 96.2 s: neither loop stalled long enough: network, hypervisor or untraced; 182.91 at 34.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1056.92 at 110.6 s: neither loop stalled long enough: network, hypervisor or untraced; 279.61 at 102.0 s: neither loop stalled long enough: network, hypervisor or untraced; 251.24 at 102.0 s: neither loop stalled long enough: network, hypervisor or untraced; 249.80 at 88.3 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 249.74 at 102.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 187.88 at 69.3 s: neither loop stalled long enough: network, hypervisor or untraced; 178.87 at 75.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 177.45 at 15.4 s: client: mostly unexplained (+softirq NET_RX); 172.87 at 76.2 s: neither loop stalled long enough: network, hypervisor or untraced; 171.99 at 34.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 996.67 at 13.6 s: neither loop stalled long enough: network, hypervisor or untraced; 204.35 at 26.9 s: neither loop stalled long enough: network, hypervisor or untraced; 187.78 at 52.6 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 158.75 at 29.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 147.91 at 103.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1047.96 at 76.2 s: neither loop stalled long enough: network, hypervisor or untraced; 858.50 at 56.2 s: neither loop stalled long enough: network, hypervisor or untraced; 771.41 at 56.2 s: neither loop stalled long enough: network, hypervisor or untraced; 744.89 at 56.2 s: neither loop stalled long enough: network, hypervisor or untraced; 714.68 at 56.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 359.96 at 69.9 s: neither loop stalled long enough: network, hypervisor or untraced; 359.62 at 73.6 s: neither loop stalled long enough: network, hypervisor or untraced; 358.61 at 55.6 s: neither loop stalled long enough: network, hypervisor or untraced; 357.33 at 69.1 s: neither loop stalled long enough: network, hypervisor or untraced; 354.05 at 90.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 730.42 at 104.3 s: neither loop stalled long enough: network, hypervisor or untraced; 712.73 at 104.3 s: neither loop stalled long enough: network, hypervisor or untraced; 638.33 at 16.9 s: neither loop stalled long enough: network, hypervisor or untraced; 596.93 at 16.9 s: neither loop stalled long enough: network, hypervisor or untraced; 546.76 at 16.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 311.56 at 59.6 s: neither loop stalled long enough: network, hypervisor or untraced; 267.88 at 67.7 s: neither loop stalled long enough: network, hypervisor or untraced; 239.77 at 81.8 s: neither loop stalled long enough: network, hypervisor or untraced; 232.66 at 82.0 s: server: softirq TIMER; 208.21 at 91.7 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 247.40 at 12.4 s: neither loop stalled long enough: network, hypervisor or untraced; 217.81 at 30.3 s: neither loop stalled long enough: network, hypervisor or untraced; 203.62 at 55.4 s: neither loop stalled long enough: network, hypervisor or untraced; 190.67 at 56.3 s: neither loop stalled long enough: network, hypervisor or untraced; 186.00 at 44.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 229.31 at 29.0 s: neither loop stalled long enough: network, hypervisor or untraced; 211.12 at 52.1 s: neither loop stalled long enough: network, hypervisor or untraced; 208.35 at 29.0 s: neither loop stalled long enough: network, hypervisor or untraced; 205.73 at 29.0 s: neither loop stalled long enough: network, hypervisor or untraced; 203.30 at 37.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 266.89 at 49.2 s: neither loop stalled long enough: network, hypervisor or untraced; 202.84 at 14.0 s: neither loop stalled long enough: network, hypervisor or untraced; 198.59 at 3.7 s: neither loop stalled long enough: network, hypervisor or untraced; 195.48 at 7.8 s: neither loop stalled long enough: network, hypervisor or untraced; 195.11 at 8.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 375.07 at 54.0 s: neither loop stalled long enough: network, hypervisor or untraced; 280.46 at 54.0 s: neither loop stalled long enough: network, hypervisor or untraced; 235.97 at 54.0 s: neither loop stalled long enough: network, hypervisor or untraced; 227.08 at 54.0 s: neither loop stalled long enough: network, hypervisor or untraced; 217.38 at 49.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 228.53 at 33.6 s: neither loop stalled long enough: network, hypervisor or untraced; 194.47 at 34.7 s: neither loop stalled long enough: network, hypervisor or untraced; 186.05 at 47.3 s: neither loop stalled long enough: network, hypervisor or untraced; 183.88 at 41.4 s: neither loop stalled long enough: network, hypervisor or untraced; 183.68 at 6.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 331.27 at 35.0 s: neither loop stalled long enough: network, hypervisor or untraced; 295.66 at 35.0 s: neither loop stalled long enough: network, hypervisor or untraced; 295.29 at 35.0 s: neither loop stalled long enough: network, hypervisor or untraced; 292.26 at 35.0 s: neither loop stalled long enough: network, hypervisor or untraced; 287.05 at 35.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 295.06 at 45.9 s: client: softirq TIMER; 256.75 at 5.5 s: neither loop stalled long enough: network, hypervisor or untraced; 244.53 at 48.9 s: server: softirq TIMER; 230.56 at 57.6 s: neither loop stalled long enough: network, hypervisor or untraced; 229.61 at 23.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 201.62 at 16.2 s: neither loop stalled long enough: network, hypervisor or untraced; 190.12 at 2.7 s: neither loop stalled long enough: network, hypervisor or untraced; 178.16 at 57.6 s: client: mostly unexplained (+softirq NET_RX); 169.70 at 26.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 157.04 at 10.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 474.38 at 5.5 s: neither loop stalled long enough: network, hypervisor or untraced; 440.01 at 5.5 s: neither loop stalled long enough: network, hypervisor or untraced; 403.62 at 5.5 s: neither loop stalled long enough: network, hypervisor or untraced; 392.62 at 5.5 s: neither loop stalled long enough: network, hypervisor or untraced; 325.93 at 5.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 380.33 at 38.3 s: neither loop stalled long enough: network, hypervisor or untraced; 357.94 at 21.4 s: neither loop stalled long enough: network, hypervisor or untraced; 278.00 at 39.0 s: neither loop stalled long enough: network, hypervisor or untraced; 252.05 at 6.6 s: neither loop stalled long enough: network, hypervisor or untraced; 250.34 at 38.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 325.31 at 47.4 s: neither loop stalled long enough: network, hypervisor or untraced; 239.77 at 1.4 s: neither loop stalled long enough: network, hypervisor or untraced; 235.53 at 3.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 218.30 at 8.4 s: neither loop stalled long enough: network, hypervisor or untraced; 216.68 at 56.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1041.29 at 53.4 s: neither loop stalled long enough: network, hypervisor or untraced; 342.10 at 43.3 s: neither loop stalled long enough: network, hypervisor or untraced; 291.87 at 43.6 s: neither loop stalled long enough: network, hypervisor or untraced; 257.55 at 43.4 s: neither loop stalled long enough: network, hypervisor or untraced; 235.21 at 31.0 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 514.72 at 12.1 s: neither loop stalled long enough: network, hypervisor or untraced; 334.54 at 10.6 s: client: softirq TIMER; 310.05 at 38.0 s: neither loop stalled long enough: network, hypervisor or untraced; 285.17 at 38.0 s: neither loop stalled long enough: network, hypervisor or untraced; 246.57 at 32.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 445.55 at 19.4 s: client: softirq TIMER; 314.12 at 21.1 s: neither loop stalled long enough: network, hypervisor or untraced; 221.81 at 22.4 s: server: softirq TIMER; 218.33 at 45.1 s: neither loop stalled long enough: network, hypervisor or untraced; 210.40 at 20.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-11 | 1062.24 at 53.8 s: neither loop stalled long enough: network, hypervisor or untraced; 412.12 at 22.5 s: neither loop stalled long enough: network, hypervisor or untraced; 411.59 at 23.0 s: client: softirq TIMER; 407.36 at 22.5 s: neither loop stalled long enough: network, hypervisor or untraced; 400.95 at 22.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 494.21 at 55.5 s: neither loop stalled long enough: network, hypervisor or untraced; 488.83 at 25.1 s: neither loop stalled long enough: network, hypervisor or untraced; 483.37 at 53.4 s: neither loop stalled long enough: network, hypervisor or untraced; 470.47 at 37.4 s: neither loop stalled long enough: network, hypervisor or untraced; 465.14 at 43.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1061.57 at 46.5 s: neither loop stalled long enough: network, hypervisor or untraced; 638.34 at 5.3 s: neither loop stalled long enough: network, hypervisor or untraced; 570.99 at 35.7 s: neither loop stalled long enough: network, hypervisor or untraced; 529.51 at 58.4 s: neither loop stalled long enough: network, hypervisor or untraced; 522.47 at 20.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1107.96 at 34.2 s: neither loop stalled long enough: network, hypervisor or untraced; 480.57 at 5.7 s: neither loop stalled long enough: network, hypervisor or untraced; 478.94 at 5.7 s: neither loop stalled long enough: network, hypervisor or untraced; 438.92 at 5.7 s: neither loop stalled long enough: network, hypervisor or untraced; 391.51 at 34.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 338.92 at 48.4 s: neither loop stalled long enough: network, hypervisor or untraced; 337.21 at 58.5 s: neither loop stalled long enough: network, hypervisor or untraced; 333.65 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 302.54 at 5.9 s: neither loop stalled long enough: network, hypervisor or untraced; 295.88 at 59.7 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 233.45 at 50.5 s: neither loop stalled long enough: network, hypervisor or untraced; 231.75 at 111.5 s: neither loop stalled long enough: network, hypervisor or untraced; 221.52 at 61.7 s: neither loop stalled long enough: network, hypervisor or untraced; 199.36 at 77.5 s: neither loop stalled long enough: network, hypervisor or untraced; 191.71 at 9.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1054.08 at 53.9 s: neither loop stalled long enough: network, hypervisor or untraced; 367.64 at 54.0 s: neither loop stalled long enough: network, hypervisor or untraced; 358.02 at 54.0 s: neither loop stalled long enough: network, hypervisor or untraced; 357.52 at 23.8 s: neither loop stalled long enough: network, hypervisor or untraced; 349.97 at 76.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 376.73 at 51.8 s: neither loop stalled long enough: network, hypervisor or untraced; 346.50 at 59.0 s: neither loop stalled long enough: network, hypervisor or untraced; 345.64 at 51.8 s: neither loop stalled long enough: network, hypervisor or untraced; 340.88 at 65.6 s: neither loop stalled long enough: network, hypervisor or untraced; 334.50 at 57.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 207.05 at 3.4 s: neither loop stalled long enough: network, hypervisor or untraced; 206.01 at 13.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 196.27 at 4.1 s: neither loop stalled long enough: network, hypervisor or untraced; 188.62 at 4.1 s: neither loop stalled long enough: network, hypervisor or untraced; 186.35 at 4.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 507.14 at 109.3 s: neither loop stalled long enough: network, hypervisor or untraced; 476.85 at 110.5 s: neither loop stalled long enough: network, hypervisor or untraced; 468.01 at 2.3 s: neither loop stalled long enough: network, hypervisor or untraced; 465.87 at 82.5 s: neither loop stalled long enough: network, hypervisor or untraced; 460.57 at 35.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 352.39 at 12.4 s: neither loop stalled long enough: network, hypervisor or untraced; 350.83 at 29.8 s: neither loop stalled long enough: network, hypervisor or untraced; 347.42 at 12.8 s: neither loop stalled long enough: network, hypervisor or untraced; 341.93 at 45.8 s: neither loop stalled long enough: network, hypervisor or untraced; 338.30 at 92.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 288.47 at 30.3 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 237.27 at 103.8 s: neither loop stalled long enough: network, hypervisor or untraced; 226.91 at 47.8 s: neither loop stalled long enough: network, hypervisor or untraced; 224.25 at 90.9 s: neither loop stalled long enough: network, hypervisor or untraced; 219.22 at 42.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 379.99 at 19.9 s: neither loop stalled long enough: network, hypervisor or untraced; 360.72 at 19.4 s: neither loop stalled long enough: network, hypervisor or untraced; 360.33 at 106.4 s: neither loop stalled long enough: network, hypervisor or untraced; 359.77 at 37.7 s: neither loop stalled long enough: network, hypervisor or untraced; 353.92 at 97.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 999.60 at 111.9 s: neither loop stalled long enough: network, hypervisor or untraced; 524.55 at 98.9 s: neither loop stalled long enough: network, hypervisor or untraced; 477.08 at 6.6 s: client: softirq TIMER; 475.36 at 115.5 s: neither loop stalled long enough: network, hypervisor or untraced; 458.36 at 16.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 654.04 at 27.1 s: neither loop stalled long enough: network, hypervisor or untraced; 620.62 at 30.6 s: neither loop stalled long enough: network, hypervisor or untraced; 617.18 at 28.4 s: neither loop stalled long enough: network, hypervisor or untraced; 608.18 at 14.6 s: neither loop stalled long enough: network, hypervisor or untraced; 605.53 at 33.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 331.94 at 25.1 s: neither loop stalled long enough: network, hypervisor or untraced; 302.42 at 26.2 s: neither loop stalled long enough: network, hypervisor or untraced; 237.48 at 97.9 s: neither loop stalled long enough: network, hypervisor or untraced; 218.57 at 24.1 s: neither loop stalled long enough: network, hypervisor or untraced; 217.48 at 66.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 478.31 at 116.8 s: neither loop stalled long enough: network, hypervisor or untraced; 424.99 at 67.9 s: neither loop stalled long enough: network, hypervisor or untraced; 408.81 at 75.7 s: neither loop stalled long enough: network, hypervisor or untraced; 405.16 at 42.5 s: neither loop stalled long enough: network, hypervisor or untraced; 404.37 at 49.8 s: client: softirq TIMER |
| r1-base-10 | 269.39 at 117.6 s: neither loop stalled long enough: network, hypervisor or untraced; 222.15 at 42.8 s: neither loop stalled long enough: network, hypervisor or untraced; 214.69 at 83.8 s: neither loop stalled long enough: network, hypervisor or untraced; 203.73 at 51.8 s: neither loop stalled long enough: network, hypervisor or untraced; 201.68 at 20.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 237.17 at 101.5 s: neither loop stalled long enough: network, hypervisor or untraced; 233.52 at 4.5 s: neither loop stalled long enough: network, hypervisor or untraced; 176.93 at 55.3 s: neither loop stalled long enough: network, hypervisor or untraced; 172.38 at 44.5 s: neither loop stalled long enough: network, hypervisor or untraced; 172.12 at 62.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 377.08 at 114.0 s: neither loop stalled long enough: network, hypervisor or untraced; 369.86 at 6.6 s: neither loop stalled long enough: network, hypervisor or untraced; 358.24 at 65.1 s: neither loop stalled long enough: network, hypervisor or untraced; 353.13 at 42.7 s: neither loop stalled long enough: network, hypervisor or untraced; 345.62 at 45.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 5116.32 at 73.3 s: neither loop stalled long enough: network, hypervisor or untraced; 533.87 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced; 527.80 at 12.1 s: neither loop stalled long enough: network, hypervisor or untraced; 521.19 at 107.8 s: neither loop stalled long enough: network, hypervisor or untraced; 519.86 at 16.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1101.91 at 37.2 s: neither loop stalled long enough: network, hypervisor or untraced; 1044.99 at 98.7 s: neither loop stalled long enough: network, hypervisor or untraced; 626.22 at 1.2 s: neither loop stalled long enough: network, hypervisor or untraced; 620.45 at 16.6 s: neither loop stalled long enough: network, hypervisor or untraced; 600.54 at 4.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 210.34 at 61.5 s: neither loop stalled long enough: network, hypervisor or untraced; 205.32 at 27.0 s: neither loop stalled long enough: network, hypervisor or untraced; 204.10 at 89.0 s: neither loop stalled long enough: network, hypervisor or untraced; 201.84 at 101.0 s: neither loop stalled long enough: network, hypervisor or untraced; 201.51 at 87.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 380.39 at 17.2 s: neither loop stalled long enough: network, hypervisor or untraced; 372.53 at 69.5 s: neither loop stalled long enough: network, hypervisor or untraced; 371.30 at 45.7 s: neither loop stalled long enough: network, hypervisor or untraced; 368.36 at 23.5 s: neither loop stalled long enough: network, hypervisor or untraced; 364.17 at 84.2 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-0

| condition | worst events |
|---|---|
| r1-base-0 | 1048.50 at 19.8 s: neither loop stalled long enough: network, hypervisor or untraced; 426.72 at 57.9 s: client: softirq TIMER; 258.13 at 58.9 s: neither loop stalled long enough: network, hypervisor or untraced; 206.77 at 36.7 s: neither loop stalled long enough: network, hypervisor or untraced; 200.43 at 46.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 205.90 at 52.3 s: neither loop stalled long enough: network, hypervisor or untraced; 195.15 at 1.9 s: neither loop stalled long enough: network, hypervisor or untraced; 193.65 at 50.3 s: neither loop stalled long enough: network, hypervisor or untraced; 185.93 at 32.2 s: neither loop stalled long enough: network, hypervisor or untraced; 180.82 at 44.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 277.65 at 48.1 s: neither loop stalled long enough: network, hypervisor or untraced; 197.88 at 28.3 s: neither loop stalled long enough: network, hypervisor or untraced; 195.94 at 9.5 s: neither loop stalled long enough: network, hypervisor or untraced; 191.20 at 28.0 s: neither loop stalled long enough: network, hypervisor or untraced; 190.91 at 20.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1041.15 at 16.6 s: neither loop stalled long enough: network, hypervisor or untraced; 267.71 at 43.1 s: neither loop stalled long enough: network, hypervisor or untraced; 236.16 at 55.7 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 219.22 at 47.0 s: neither loop stalled long enough: network, hypervisor or untraced; 199.81 at 55.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 287.42 at 47.2 s: server: softirq TIMER; 219.24 at 44.2 s: client: softirq TIMER; 195.75 at 24.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 189.00 at 38.6 s: neither loop stalled long enough: network, hypervisor or untraced; 186.21 at 14.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 332.90 at 54.2 s: neither loop stalled long enough: network, hypervisor or untraced; 199.66 at 47.7 s: neither loop stalled long enough: network, hypervisor or untraced; 195.96 at 27.6 s: neither loop stalled long enough: network, hypervisor or untraced; 185.09 at 49.7 s: neither loop stalled long enough: network, hypervisor or untraced; 184.39 at 36.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 260.90 at 58.3 s: neither loop stalled long enough: network, hypervisor or untraced; 258.38 at 8.0 s: neither loop stalled long enough: network, hypervisor or untraced; 238.63 at 26.8 s: neither loop stalled long enough: network, hypervisor or untraced; 223.41 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced; 216.93 at 1.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1049.67 at 36.6 s: neither loop stalled long enough: network, hypervisor or untraced; 326.38 at 14.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 248.04 at 1.6 s: neither loop stalled long enough: network, hypervisor or untraced; 188.62 at 36.6 s: neither loop stalled long enough: network, hypervisor or untraced; 179.36 at 32.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1044.55 at 42.9 s: neither loop stalled long enough: network, hypervisor or untraced; 300.37 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced; 184.96 at 55.1 s: neither loop stalled long enough: network, hypervisor or untraced; 180.65 at 12.1 s: neither loop stalled long enough: network, hypervisor or untraced; 179.33 at 21.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-cstate | 196.93 at 3.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 185.66 at 27.8 s: neither loop stalled long enough: network, hypervisor or untraced; 175.02 at 23.6 s: neither loop stalled long enough: network, hypervisor or untraced; 163.63 at 3.8 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 162.82 at 34.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-11 | 323.85 at 8.3 s: neither loop stalled long enough: network, hypervisor or untraced; 255.62 at 8.3 s: neither loop stalled long enough: network, hypervisor or untraced; 224.12 at 39.2 s: neither loop stalled long enough: network, hypervisor or untraced; 205.05 at 6.5 s: neither loop stalled long enough: network, hypervisor or untraced; 180.82 at 16.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-stock | 342.18 at 37.3 s: neither loop stalled long enough: network, hypervisor or untraced; 151.01 at 23.9 s: neither loop stalled long enough: network, hypervisor or untraced; 149.42 at 3.2 s: neither loop stalled long enough: network, hypervisor or untraced; 136.31 at 35.1 s: neither loop stalled long enough: network, hypervisor or untraced; 134.97 at 53.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 181.53 at 27.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 151.61 at 44.2 s: neither loop stalled long enough: network, hypervisor or untraced; 150.50 at 49.7 s: neither loop stalled long enough: network, hypervisor or untraced; 147.14 at 31.3 s: neither loop stalled long enough: network, hypervisor or untraced; 146.38 at 32.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 305.72 at 56.4 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 229.07 at 48.1 s: neither loop stalled long enough: network, hypervisor or untraced; 222.98 at 50.1 s: neither loop stalled long enough: network, hypervisor or untraced; 209.53 at 6.0 s: neither loop stalled long enough: network, hypervisor or untraced; 202.37 at 55.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1052.82 at 8.5 s: neither loop stalled long enough: network, hypervisor or untraced; 277.53 at 29.0 s: server: softirq TIMER; 180.18 at 13.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 158.39 at 55.1 s: neither loop stalled long enough: network, hypervisor or untraced; 150.31 at 30.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-10000

| condition | worst events |
|---|---|
| r1-base-0 | 292.05 at 14.4 s: neither loop stalled long enough: network, hypervisor or untraced; 283.85 at 24.2 s: neither loop stalled long enough: network, hypervisor or untraced; 279.81 at 116.6 s: neither loop stalled long enough: network, hypervisor or untraced; 269.31 at 14.4 s: neither loop stalled long enough: network, hypervisor or untraced; 253.48 at 8.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 381.92 at 105.1 s: neither loop stalled long enough: network, hypervisor or untraced; 368.64 at 27.4 s: neither loop stalled long enough: network, hypervisor or untraced; 363.08 at 3.7 s: neither loop stalled long enough: network, hypervisor or untraced; 359.11 at 83.5 s: neither loop stalled long enough: network, hypervisor or untraced; 358.58 at 116.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 998.23 at 14.1 s: neither loop stalled long enough: network, hypervisor or untraced; 215.32 at 60.9 s: neither loop stalled long enough: network, hypervisor or untraced; 206.96 at 75.0 s: neither loop stalled long enough: network, hypervisor or untraced; 199.98 at 8.5 s: neither loop stalled long enough: network, hypervisor or untraced; 197.59 at 2.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 396.43 at 52.2 s: neither loop stalled long enough: network, hypervisor or untraced; 365.86 at 41.7 s: neither loop stalled long enough: network, hypervisor or untraced; 352.46 at 50.5 s: neither loop stalled long enough: network, hypervisor or untraced; 351.06 at 20.1 s: neither loop stalled long enough: network, hypervisor or untraced; 350.82 at 81.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 202.81 at 29.5 s: neither loop stalled long enough: network, hypervisor or untraced; 183.27 at 44.8 s: neither loop stalled long enough: network, hypervisor or untraced; 172.94 at 86.5 s: neither loop stalled long enough: network, hypervisor or untraced; 172.27 at 42.5 s: neither loop stalled long enough: network, hypervisor or untraced; 163.79 at 40.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 993.19 at 35.0 s: neither loop stalled long enough: network, hypervisor or untraced; 364.65 at 58.4 s: neither loop stalled long enough: network, hypervisor or untraced; 358.79 at 84.8 s: neither loop stalled long enough: network, hypervisor or untraced; 358.47 at 34.4 s: neither loop stalled long enough: network, hypervisor or untraced; 355.44 at 83.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 454.83 at 58.4 s: client: softirq TIMER; 255.89 at 79.9 s: neither loop stalled long enough: network, hypervisor or untraced; 212.26 at 91.6 s: neither loop stalled long enough: network, hypervisor or untraced; 197.41 at 61.3 s: server: softirq TIMER; 195.12 at 98.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 433.71 at 102.0 s: neither loop stalled long enough: network, hypervisor or untraced; 370.62 at 47.0 s: neither loop stalled long enough: network, hypervisor or untraced; 364.75 at 117.2 s: neither loop stalled long enough: network, hypervisor or untraced; 358.12 at 64.4 s: neither loop stalled long enough: network, hypervisor or untraced; 348.96 at 32.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1037.29 at 65.5 s: neither loop stalled long enough: network, hypervisor or untraced; 278.79 at 62.5 s: client: softirq TIMER; 175.90 at 87.6 s: neither loop stalled long enough: network, hypervisor or untraced; 174.49 at 59.5 s: neither loop stalled long enough: network, hypervisor or untraced; 170.77 at 94.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-cstate | 1055.23 at 45.2 s: neither loop stalled long enough: network, hypervisor or untraced; 260.67 at 65.7 s: server: softirq TIMER; 204.74 at 90.8 s: neither loop stalled long enough: network, hypervisor or untraced; 204.64 at 22.9 s: neither loop stalled long enough: network, hypervisor or untraced; 182.74 at 67.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-11 | 369.08 at 66.0 s: neither loop stalled long enough: network, hypervisor or untraced; 357.85 at 105.3 s: neither loop stalled long enough: network, hypervisor or untraced; 351.22 at 3.9 s: neither loop stalled long enough: network, hypervisor or untraced; 348.99 at 102.4 s: neither loop stalled long enough: network, hypervisor or untraced; 346.37 at 103.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 353.90 at 9.8 s: neither loop stalled long enough: network, hypervisor or untraced; 348.45 at 2.4 s: neither loop stalled long enough: network, hypervisor or untraced; 342.78 at 42.0 s: neither loop stalled long enough: network, hypervisor or untraced; 340.66 at 12.6 s: neither loop stalled long enough: network, hypervisor or untraced; 331.07 at 9.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 339.15 at 78.9 s: neither loop stalled long enough: network, hypervisor or untraced; 286.11 at 78.9 s: neither loop stalled long enough: network, hypervisor or untraced; 271.88 at 25.6 s: neither loop stalled long enough: network, hypervisor or untraced; 244.18 at 78.9 s: neither loop stalled long enough: network, hypervisor or untraced; 233.29 at 25.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1003.40 at 42.0 s: neither loop stalled long enough: network, hypervisor or untraced; 368.83 at 52.8 s: neither loop stalled long enough: network, hypervisor or untraced; 361.85 at 24.0 s: neither loop stalled long enough: network, hypervisor or untraced; 356.15 at 22.8 s: neither loop stalled long enough: network, hypervisor or untraced; 354.18 at 24.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 302.19 at 28.6 s: neither loop stalled long enough: network, hypervisor or untraced; 281.91 at 14.1 s: neither loop stalled long enough: network, hypervisor or untraced; 239.01 at 14.1 s: neither loop stalled long enough: network, hypervisor or untraced; 208.78 at 14.1 s: neither loop stalled long enough: network, hypervisor or untraced; 177.60 at 10.8 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| r1-base-0 | loop work 125399.34; unexplained: no kernel event (hypervisor or in-kernel time) 71807.98; mostly unexplained (+preempted by kworker/1:1) 53082.14; mostly unexplained (+softirq NET_RX) 20047.53; softirq NET_RX 9980.61 |
| r1-housekeeping | loop work 171039.93; mostly unexplained (+preempted by kworker/1:1) 39898.51; unexplained: no kernel event (hypervisor or in-kernel time) 26750.77; mostly unexplained (+softirq NET_RX) 11132.26; softirq NET_RX 6915.00 |
| r1-prefer | loop work 129596.20; unexplained: no kernel event (hypervisor or in-kernel time) 5712.64; softirq TIMER 361.86; mostly unexplained (+irq work (IPI/vector)) 303.80; VM paused: the idle spare core stalled too (hypervisor) 280.06 |
| r1-base-2 | loop work 125486.07; unexplained: no kernel event (hypervisor or in-kernel time) 43734.04; mostly unexplained (+preempted by kworker/1:1) 42609.95; mostly unexplained (+softirq NET_RX) 15064.38; softirq NET_RX 8002.63 |
| r1-noquickack | loop work 107328.58; mostly unexplained (+preempted by kworker/1:1) 34456.33; softirq NET_RX 10045.92; mostly unexplained (+softirq NET_RX) 9015.28; unexplained: no kernel event (hypervisor or in-kernel time) 6537.41 |
| r1-nogro | loop work 119549.50; mostly unexplained (+preempted by kworker/1:1) 43868.30; softirq NET_RX 8163.98; unexplained: no kernel event (hypervisor or in-kernel time) 7437.55; mostly unexplained (+softirq NET_RX) 5905.04 |
| r1-base-4 | loop work 148756.04; mostly unexplained (+preempted by kworker/1:1) 47072.25; unexplained: no kernel event (hypervisor or in-kernel time) 16930.31; mostly unexplained (+softirq NET_RX) 11138.62; softirq NET_RX 8922.81 |
| r1-nocoalesce | loop work 137018.87; unexplained: no kernel event (hypervisor or in-kernel time) 7838.10; softirq NET_RX 790.43; softirq TIMER 550.97; mostly unexplained (+irq work (IPI/vector)) 333.08 |
| r1-irqlocal | unexplained: no kernel event (hypervisor or in-kernel time) 419065.58; loop work 179459.02; softirq NET_RX 52267.55; mostly unexplained (+local timer tick) 15320.52; mostly unexplained (+softirq NET_RX) 10947.92 |
| r1-base-6 | loop work 161204.99; mostly unexplained (+preempted by kworker/1:1) 27229.56; unexplained: no kernel event (hypervisor or in-kernel time) 10447.20; mostly unexplained (+softirq NET_RX) 8618.64; softirq NET_RX 5214.76 |
| r1-rt | loop work 170054.53; unexplained: no kernel event (hypervisor or in-kernel time) 6149.68; mostly unexplained (+preempted by kworker/1:1) 4087.72; preempted by kworker/1:1 2530.32; softirq TIMER 618.58 |
| r1-sibling | loop work 189204.39; mostly unexplained (+preempted by kworker/1:1) 96706.01; softirq NET_RX 28189.31; mostly unexplained (+softirq NET_RX) 19976.65; unexplained: no kernel event (hypervisor or in-kernel time) 14536.36 |
| r1-base-8 | loop work 112713.93; mostly unexplained (+preempted by kworker/1:1) 29591.77; unexplained: no kernel event (hypervisor or in-kernel time) 10096.38; softirq NET_RX 5346.75; mostly unexplained (+softirq NET_RX) 4799.84 |
| r1-busypoll50 | loop work 172687.55; mostly unexplained (+preempted by kworker/1:1) 35395.67; unexplained: no kernel event (hypervisor or in-kernel time) 8745.72; softirq NET_RX 6276.44; mostly unexplained (+softirq NET_RX) 5781.48 |
| r1-budget64 | loop work 161922.08; mostly unexplained (+preempted by kworker/1:1) 42093.42; mostly unexplained (+softirq NET_RX) 8452.84; softirq NET_RX 7317.18; unexplained: no kernel event (hypervisor or in-kernel time) 7241.33 |
| r1-base-10 | loop work 158649.67; mostly unexplained (+preempted by kworker/1:1) 46301.29; unexplained: no kernel event (hypervisor or in-kernel time) 13518.62; softirq NET_RX 7388.73; mostly unexplained (+softirq NET_RX) 6995.04 |
| r1-cstate | loop work 176742.63; mostly unexplained (+preempted by kworker/1:1) 31092.86; unexplained: no kernel event (hypervisor or in-kernel time) 12627.25; softirq NET_RX 9256.16; mostly unexplained (+softirq NET_RX) 7358.45 |
| r1-base-11 | loop work 170178.50; mostly unexplained (+preempted by kworker/1:1) 42605.65; unexplained: no kernel event (hypervisor or in-kernel time) 13051.42; softirq NET_RX 7457.53; mostly unexplained (+softirq NET_RX) 6842.18 |
| boot-stock | local timer tick 873133.67; loop work 112779.19; unexplained: no kernel event (hypervisor or in-kernel time) 60977.58; softirq TIMER 17207.57; mostly unexplained (+preempted by kworker/1:1) 10850.71 |
| boot-isolcpus | local timer tick 878488.81; loop work 133090.72; unexplained: no kernel event (hypervisor or in-kernel time) 63700.17; mostly unexplained (+local timer tick) 16153.88; mostly unexplained (+softirq NET_RX) 13976.03 |
| boot-full-nomitig | loop work 176661.95; mostly unexplained (+preempted by kworker/1:1) 44999.03; unexplained: no kernel event (hypervisor or in-kernel time) 34421.52; mostly unexplained (+softirq NET_RX) 12967.83; softirq NET_RX 10682.97 |
| boot-full | loop work 128339.64; mostly unexplained (+preempted by kworker/1:1) 52902.75; unexplained: no kernel event (hypervisor or in-kernel time) 32139.17; mostly unexplained (+softirq NET_RX) 15464.07; softirq NET_RX 11046.95 |

## Once-a-minute stalls

Whether each source's events of 300 µs or more (loop stalls, or samples that slow) bunch at one second of the wall-clock minute. The busiest second is the middle of the busiest three; three seconds hold one event in twenty by chance, so a share of 25% or more over at least five events is marked periodic. Each host has its own second (its agents' timers start at boot).

| run | source | events | busiest second | events there (share) | minutes with one there | longest there (µs) | periodic |
|---|---|---|---|---|---|---|---|
| http-recv-busy-10000 | idle spare core | 3 | :05 | 1 (33%) | 1 of 45 | 399.04 | no |
| http-recv-busy-10000 | receiver loop | 1 | :55 | 1 (100%) | 1 of 45 | 400.06 | no |
| http-recv-busy-10000 | sender loop | 0 | | | | | |
| http-recv-busy-10000 | slow samples | 3263 | :26 | 191 (6%) | 5 of 45 | 394.24 | no |
| md-recv-busy | idle spare core | 3 | :32 | 1 (33%) | 1 of 24 | 336.37 | no |
| md-recv-busy | receiver loop | 2 | :04 | 1 (50%) | 1 of 24 | 301.14 | no |
| md-recv-busy | slow samples | 0 | | | | | |
| noise | noise probe (loop CPU) | 4 | :04 | 1 (25%) | 1 of 44 | 347.61 | no |
| pp-epoll-spin-0 | idle spare core | 0 | | | | | |
| pp-epoll-spin-0 | receiver loop | 0 | | | | | |
| pp-epoll-spin-0 | sender loop | 0 | | | | | |
| pp-epoll-spin-0 | slow samples | 22179 | :16 | 1727 (8%) | 4 of 14 | 485.06 | no |
| pp-epoll-spin-10000 | idle spare core | 4 | :04 | 1 (25%) | 1 of 28 | 4079.17 | no |
| pp-epoll-spin-10000 | receiver loop | 1 | :05 | 1 (100%) | 1 of 27 | 352.78 | no |
| pp-epoll-spin-10000 | sender loop | 0 | | | | | |
| pp-epoll-spin-10000 | slow samples | 113672 | :59 | 6182 (5%) | 10 of 26 | 553.79 | no |
| pp-recv-busy-0 | idle spare core | 1 | :37 | 1 (100%) | 1 of 22 | 363.19 | no |
| pp-recv-busy-0 | receiver loop | 0 | | | | | |
| pp-recv-busy-0 | sender loop | 0 | | | | | |
| pp-recv-busy-0 | slow samples | 17 | :26 | 4 (24%) | 2 of 24 | 482.76 | no |
| pp-recv-busy-10000 | idle spare core | 1 | :00 | 1 (100%) | 1 of 48 | 348.43 | no |
| pp-recv-busy-10000 | receiver loop | 3 | :23 | 1 (33%) | 1 of 44 | 361.86 | no |
| pp-recv-busy-10000 | sender loop | 0 | | | | | |
| pp-recv-busy-10000 | slow samples | 1226 | :01 | 87 (7%) | 12 of 46 | 858.50 | no |
| pp-uring-spin-0 | idle spare core | 1 | :14 | 1 (100%) | 1 of 20 | 359.42 | no |
| pp-uring-spin-0 | receiver loop | 2 | :09 | 1 (50%) | 1 of 19 | 388.33 | no |
| pp-uring-spin-0 | sender loop | 0 | | | | | |
| pp-uring-spin-0 | slow samples | 3807 | :36 | 387 (10%) | 2 of 19 | 520.50 | no |
| pp-uring-spin-10000 | idle spare core | 1 | :00 | 1 (100%) | 1 of 39 | 619.20 | no |
| pp-uring-spin-10000 | receiver loop | 2 | :00 | 1 (50%) | 1 of 39 | 393.62 | no |
| pp-uring-spin-10000 | sender loop | 0 | | | | | |
| pp-uring-spin-10000 | slow samples | 116692 | :01 | 7795 (7%) | 20 of 39 | 549.32 | no |
| pp-uring-sqpoll-0 | idle spare core | 1 | :31 | 1 (100%) | 1 of 16 | 478.17 | no |
| pp-uring-sqpoll-0 | receiver loop | 1 | :37 | 1 (100%) | 1 of 15 | 393.00 | no |
| pp-uring-sqpoll-0 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-0 | slow samples | 12 | :56 | 3 (25%) | 3 of 16 | 1049.67 | no |
| pp-uring-sqpoll-10000 | idle spare core | 0 | | | | | |
| pp-uring-sqpoll-10000 | receiver loop | 1 | :46 | 1 (100%) | 1 of 30 | 366.74 | no |
| pp-uring-sqpoll-10000 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-10000 | slow samples | 905 | :11 | 65 (7%) | 12 of 30 | 361.85 | no |

# Every run in detail

#### Stalls: r1-base-0/http-recv-busy-10000, loop 0 on CPU 1

1819365271 iterations, longest 129.2 µs; over 5 / 20 / 100 µs: 76755 / 4389 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2731 | 67556.6 | 129.2 |
| mostly unexplained (+preempted by kworker/1:1) | 933 | 23392.8 | 55.2 |
| mostly unexplained (+softirq NET_RX) | 588 | 14842.2 | 44.8 |
| softirq NET_RX | 121 | 2959.5 | 37.5 |
| mostly unexplained (+irq work (IPI/vector)) | 10 | 287.7 | 43.1 |
| loop work | 5 | 112.0 | 30.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 49.1 | 49.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 129.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 121.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 72.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 55.2 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 54.1 | 2 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 52.5 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 52.5 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 49.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 48.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 47.5 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 3872 |
| LOC Local timer interrupts | 3870 |
| CAL Function call interrupts | 998 |

#### Slowest round trips: r1-base-0/http-recv-busy-10000 (median 46.37)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 338.88 | 292.51 | 0.00 |  | 0.00 |  | 292.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 296.17 | 249.79 | 0.00 |  | 0.00 |  | 249.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 254.52 | 208.15 | 0.00 |  | 0.00 |  | 208.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 242.42 | 196.04 | 0.00 |  | 0.00 |  | 196.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 233.01 | 186.63 | 0.00 |  | 0.00 |  | 186.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 217.76 | 171.39 | 0.00 |  | 0.00 |  | 171.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 209.82 | 163.44 | 0.00 |  | 0.00 |  | 163.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 204.94 | 158.57 | 0.00 |  | 0.00 |  | 158.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 197.23 | 150.86 | 0.00 |  | 0.00 |  | 150.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 196.70 | 150.33 | 0.00 |  | 0.00 |  | 150.33 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-0/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=380352 clocksource=tsc

### Loop counters (r1-base-0/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002700 | 424794 | 7.07 | 108714432 | 36.21 | 1811907 | 1811907 | 99.6 | 64.9 | true | 0 | 747 | 547.5 | 130646 / 4651 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672604800 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (50097 of 2499970 in 1-msg reads) | 50097 | 3.02 | 2.83 | 3.89 | 6.71 | 21.20 | 29.81 | 32.98 |

#### Stalls: r1-base-0/md-recv-busy, loop 0 on CPU 1

108714432 iterations, longest 547.5 µs; over 5 / 20 / 100 µs: 130646 / 4651 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4392 | 125287.4 | 547.5 |
| mostly unexplained (+preempted by kworker/1:1) | 102 | 2598.8 | 48.4 |
| softirq NET_RX | 85 | 2203.2 | 35.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 59 | 1507.1 | 127.5 |
| mostly unexplained (+softirq NET_RX) | 8 | 194.6 | 30.2 |
| softirq TIMER | 1 | 162.9 | 162.9 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 77.8 | 32.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.3 | 21.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 547.5 | 2700 | loop work (2700 items) |  |
| 2 | 162.9 | 0 | softirq TIMER | softirq TIMER 152, local timer tick 5, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 3 | 127.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 63.3 | 34 | loop work (34 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 59.5 | 20 | loop work (20 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 57.6 | 20 | loop work (20 items) | softirq NET_RX 16, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 56.6 | 18 | loop work (18 items) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 56.5 | 23 | loop work (23 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 56.2 | 20 | loop work (20 items) | softirq NET_RX 20, preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 56.2 | 13 | loop work (13 items) | preempted by kworker/1:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 761 |
| LOC Local timer interrupts | 751 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-0/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-0/noise, loop 0 on CPU 1

9004986368 iterations, longest 26.8 µs; over 5 / 20 / 100 µs: 67 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 66 | 608.6 | 26.8 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 6.4 | 6.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 26.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 17.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 14.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 12.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 11.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 11.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 10.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 10.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 10.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |

### Loop counters (r1-base-0/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1156500 | 1156500 | 1.00 | 279999785 | 242.11 | 4666663 | 4628113 | 0.0 | 40.5 | false | 0 | 0 | 46.6 | 36119 / 9 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-0, loop 0 on CPU 1

277686784 iterations, longest 46.6 µs; over 5 / 20 / 100 µs: 36119 / 9 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 202.8 | 46.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 33.8 | 33.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 46.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 33.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 23.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 23.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 23.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 21.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 21.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 21.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 20.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 20 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-0 (median 48.94)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 438.11 | 389.16 | 0.00 |  | 0.00 |  | 389.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 424.99 | 376.05 | 0.00 |  | 0.00 |  | 376.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 408.06 | 359.12 | 0.00 |  | 0.00 |  | 359.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 402.03 | 353.09 | 0.00 |  | 0.00 |  | 353.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 399.28 | 350.34 | 0.00 |  | 0.00 |  | 350.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 398.77 | 349.83 | 0.00 |  | 0.00 |  | 349.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 388.86 | 339.91 | 0.00 |  | 0.00 |  | 339.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 377.28 | 328.34 | 0.00 |  | 0.00 |  | 328.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 370.92 | 321.98 | 0.00 |  | 0.00 |  | 321.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 369.14 | 320.20 | 0.00 |  | 0.00 |  | 320.20 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1156271 | 1156271 | 1.00 | 555957086 | 480.82 | 4632976 | 4613705 | 0.0 | 37.2 | false | 0 | 0 | 352.8 | 57696 / 21 / 1 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-10000, loop 0 on CPU 1

553644544 iterations, longest 352.8 µs; over 5 / 20 / 100 µs: 57696 / 21 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 18 | 538.5 | 67.0 |
| softirq TIMER | 1 | 352.8 | 352.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.9 | 32.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 352.8 | 0 | softirq TIMER | softirq TIMER 319, local timer tick 14, timer tick_nohz_handler 8 |
| 2 | 67.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 44.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 41.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 39.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 33.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 33.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 32.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 29.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 28.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-10000 (median 51.56)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 411.31 | 359.75 | 0.00 |  | 0.00 |  | 359.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 397.88 | 346.33 | 0.00 |  | 0.00 |  | 346.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 395.98 | 344.42 | 0.00 |  | 0.00 |  | 344.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 389.61 | 338.06 | 0.00 |  | 0.00 |  | 338.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 379.97 | 328.41 | 0.00 |  | 0.00 |  | 328.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 375.17 | 323.62 | 0.00 |  | 0.00 |  | 323.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 374.85 | 323.30 | 0.00 |  | 0.00 |  | 323.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 374.40 | 322.85 | 0.00 |  | 0.00 |  | 322.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 374.40 | 322.84 | 0.00 |  | 0.00 |  | 322.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 371.52 | 319.96 | 0.00 |  | 0.00 |  | 319.96 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1292224 | 1292224 | 1.00 | 106688513 | 82.56 | 1778142 | 1756605 | 98.8 | 71.4 | true | 0 | 2813 | 41.0 | 46058 / 651 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-0, loop 0 on CPU 1

105396288 iterations, longest 41.0 µs; over 5 / 20 / 100 µs: 46058 / 651 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 437 | 9710.4 | 31.1 |
| softirq NET_RX | 96 | 2153.3 | 35.1 |
| mostly unexplained (+softirq NET_RX) | 89 | 2022.4 | 29.6 |
| mostly unexplained (+local timer tick) | 14 | 343.4 | 31.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 11 | 279.9 | 41.0 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 85.5 | 36.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 32.4 | 32.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 41.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 36.3 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 35.1 | 0 | softirq NET_RX | softirq NET_RX 21, preempted by ksoftirqd/1 8, local timer tick 7, timer tick_nohz_handler 5 |
| 4 | 32.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 31.7 | 1 | mostly unexplained (+local timer tick) | local timer tick 4, preempted by kworker/1:1 3, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 6 | 31.7 | 0 | softirq NET_RX | softirq NET_RX 16, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 31.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 30.8 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, local timer tick 3, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 9 | 30.8 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, preempted by kworker/1:1 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 30.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2858 |
| IWI IRQ work interrupts | 2819 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-recv-busy-0 (median 45.25)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 199.64 | 154.39 | 0.00 |  | 0.00 |  | 154.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 194.42 | 149.17 | 0.00 |  | 147.69 | unexplained: no kernel event (hypervisor or in-kernel time) | 1.48 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 186.41 | 141.16 | 0.00 |  | 0.00 |  | 141.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 170.70 | 125.45 | 0.00 |  | 123.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 1.65 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 146.55 | 101.30 | 0.00 |  | 0.00 |  | 101.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 143.67 | 98.42 | 0.00 |  | 0.00 |  | 98.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 138.82 | 93.57 | 0.00 |  | 0.00 |  | 93.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 135.14 | 89.89 | 0.00 |  | 0.00 |  | 89.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 132.46 | 87.21 | 0.00 |  | 0.00 |  | 87.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 128.76 | 83.51 | 0.00 |  | 0.00 |  | 83.51 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196884 | 1196884 | 1.00 | 222860564 | 186.20 | 1857171 | 1847197 | 99.5 | 68.2 | true | 0 | 3334 | 63.4 | 46588 / 995 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-10000, loop 0 on CPU 1

221663680 iterations, longest 63.4 µs; over 5 / 20 / 100 µs: 46588 / 995 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 726 | 17380.1 | 35.6 |
| mostly unexplained (+softirq NET_RX) | 119 | 2988.3 | 31.3 |
| softirq NET_RX | 115 | 2664.7 | 32.1 |
| mostly unexplained (+local timer tick) | 16 | 425.5 | 33.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 10 | 278.0 | 63.4 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 103.7 | 29.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 98.0 | 30.1 |
| softirq TIMER | 1 | 55.2 | 55.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 63.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 55.2 | 0 | softirq TIMER | softirq TIMER 47, local timer tick 3, timer tick_nohz_handler 1 |
| 3 | 35.6 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 4 | 33.6 | 1 | mostly unexplained (+local timer tick) | local timer tick 9, timer tick_nohz_handler 5, preempted by kworker/1:1 4, irq work (IPI/vector) 1 |
| 5 | 33.0 | 1 | mostly unexplained (+local timer tick) | local timer tick 7, timer tick_nohz_handler 5, preempted by kworker/1:1 4, irq work (IPI/vector) 1 |
| 6 | 32.8 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 32.7 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 8 | 32.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.1 | 1 | softirq NET_RX | softirq NET_RX 19, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 32.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3495 |
| IWI IRQ work interrupts | 3343 |
| CAL Function call interrupts | 1002 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/pp-recv-busy-10000 (median 45.46)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 331.27 | 285.81 | 0.00 |  | 0.00 |  | 285.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 281.36 | 235.91 | 0.00 |  | 235.98 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 279.25 | 233.79 | 0.00 |  | 0.00 |  | 233.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 244.87 | 199.41 | 0.00 |  | 8.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 190.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 225.67 | 180.22 | 0.00 |  | 0.00 |  | 180.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 202.69 | 157.23 | 0.00 |  | 0.00 |  | 157.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 199.96 | 154.50 | 0.00 |  | 0.00 |  | 154.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 160.39 | 114.93 | 0.00 |  | 27.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 87.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 156.98 | 111.52 | 0.00 |  | 124.63 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 146.87 | 101.42 | 55.21 | softirq TIMER | 0.00 |  | 46.20 | client: softirq TIMER |

### Loop counters (r1-base-0/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1152594 | 1152594 | 1.00 | 3457783 | 3.00 | 57630 | 44578220 | 0.0 | 9.5 | false | 0 | 0 | 42.4 | 44466 / 13 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-spin-0, loop 0 on CPU 1

2674693184 iterations, longest 42.4 µs; over 5 / 20 / 100 µs: 44466 / 13 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 10 | 244.4 | 42.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 50.8 | 30.4 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 37.9 | 37.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 42.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 37.9 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 30.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 25.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 25.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 23.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 22.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 21.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 21.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 20.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 10 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-uring-spin-0 (median 48.31)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 247.40 | 199.09 | 0.00 |  | 0.00 |  | 199.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 217.81 | 169.50 | 0.00 |  | 0.00 |  | 169.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 203.62 | 155.31 | 0.00 |  | 0.00 |  | 155.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 190.67 | 142.37 | 0.00 |  | 0.00 |  | 142.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 186.00 | 137.70 | 0.00 |  | 0.00 |  | 137.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 183.85 | 135.54 | 0.00 |  | 146.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 183.06 | 134.75 | 0.00 |  | 0.00 |  | 134.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 182.11 | 133.81 | 0.00 |  | 0.00 |  | 133.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 179.05 | 130.74 | 0.00 |  | 0.00 |  | 130.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 177.84 | 129.53 | 0.00 |  | 0.00 |  | 129.53 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1197327 | 1197327 | 1.00 | 3591982 | 3.00 | 29933 | 35528635 | 0.0 | 5.6 | false | 0 | 0 | 75.8 | 40529 / 22 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-spin-10000, loop 0 on CPU 1

4263436224 iterations, longest 75.8 µs; over 5 / 20 / 100 µs: 40529 / 22 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 19 | 554.7 | 75.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 50.7 | 30.2 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 33.3 | 33.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 75.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 49.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 39.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 38.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 33.3 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 32.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 30.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 26.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 25.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 24.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 3 |

#### Slowest round trips: r1-base-0/pp-uring-spin-10000 (median 51.18)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 233.45 | 182.27 | 0.00 |  | 0.00 |  | 182.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 231.75 | 180.58 | 0.00 |  | 0.00 |  | 180.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 221.52 | 170.35 | 0.00 |  | 0.00 |  | 170.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 199.36 | 148.18 | 0.00 |  | 0.00 |  | 148.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 191.71 | 140.53 | 0.00 |  | 0.00 |  | 140.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 191.64 | 140.47 | 0.00 |  | 0.00 |  | 140.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 190.36 | 139.18 | 0.00 |  | 0.00 |  | 139.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 189.33 | 138.16 | 0.00 |  | 0.00 |  | 138.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 189.24 | 138.06 | 0.00 |  | 0.00 |  | 138.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 188.90 | 137.72 | 0.00 |  | 0.00 |  | 137.72 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1259973 | 1259973 | 1.00 | 1 | 0.00 | 0 | 31181546 | 0.0 | 0.0 | true | 0 | 0 | 393.0 | 61 / 4 / 1 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-0, loop 0 on CPU 1

1870892736 iterations, longest 393.0 µs; over 5 / 20 / 100 µs: 61 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 393.0 | 393.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 81.0 | 47.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 37.3 | 37.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 393.0 | 1 | softirq TIMER | softirq TIMER 369, local timer tick 13, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 47.8 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 37.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 33.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 15 |
| IWI IRQ work interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-0 (median 46.43)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1048.50 | 1002.07 | 0.00 |  | 0.00 |  | 1002.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 426.72 | 380.30 | 392.39 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 258.13 | 211.70 | 0.00 |  | 0.00 |  | 211.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 206.77 | 160.34 | 0.00 |  | 0.00 |  | 160.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 200.43 | 154.01 | 0.00 |  | 0.00 |  | 154.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 199.79 | 153.37 | 0.00 |  | 0.00 |  | 153.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 187.22 | 140.79 | 0.00 |  | 0.00 |  | 140.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 182.68 | 136.25 | 0.00 |  | 143.74 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 181.32 | 134.89 | 0.00 |  | 0.00 |  | 134.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 180.53 | 134.10 | 0.00 |  | 0.00 |  | 134.10 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199479 | 1199479 | 1.00 | 1 | 0.00 | 0 | 28452593 | 0.0 | 0.0 | true | 0 | 0 | 32.7 | 100 / 1 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-10000, loop 0 on CPU 1

3414311104 iterations, longest 32.7 µs; over 5 / 20 / 100 µs: 100 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 32.7 | 32.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 32.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1001 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-10000 (median 46.11)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 292.05 | 245.94 | 0.00 |  | 0.00 |  | 245.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 283.85 | 237.74 | 0.00 |  | 0.00 |  | 237.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 279.81 | 233.69 | 0.00 |  | 0.00 |  | 233.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 269.31 | 223.19 | 0.00 |  | 0.00 |  | 223.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 253.48 | 207.37 | 0.00 |  | 0.00 |  | 207.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 243.85 | 197.74 | 0.00 |  | 0.00 |  | 197.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 241.33 | 195.21 | 0.00 |  | 0.00 |  | 195.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 238.94 | 192.82 | 0.00 |  | 0.00 |  | 192.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 235.89 | 189.78 | 0.00 |  | 0.00 |  | 189.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 234.48 | 188.37 | 0.00 |  | 0.00 |  | 188.37 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-housekeeping/http-recv-busy-10000, loop 0 on CPU 1

1773899651 iterations, longest 256.6 µs; over 5 / 20 / 100 µs: 50273 / 1698 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 952 | 21294.4 | 133.4 |
| mostly unexplained (+preempted by kworker/1:1) | 451 | 12511.4 | 50.6 |
| mostly unexplained (+softirq NET_RX) | 216 | 5379.2 | 52.6 |
| softirq NET_RX | 70 | 1826.9 | 35.7 |
| softirq TIMER | 1 | 256.6 | 256.6 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 85.0 | 34.5 |
| loop work | 3 | 84.4 | 34.4 |
| mostly unexplained (+local timer tick) | 1 | 28.9 | 28.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 22.4 | 22.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 256.6 | 2 | softirq TIMER | softirq TIMER 229, local timer tick 12, timer tick_nohz_handler 6, softirq NET_RX 4 |
| 2 | 133.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 124.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 70.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 52.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 18, preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 50.6 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 50.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, workqueue [ena] 2, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 48.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 48.5 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 45.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| LOC Local timer interrupts | 998 |
| IWI IRQ work interrupts | 994 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-housekeeping/http-recv-busy-10000 (median 48.11)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1051.57 | 1003.46 | 0.00 |  | 0.00 |  | 1003.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1028.07 | 979.96 | 0.00 |  | 0.00 |  | 979.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 278.22 | 230.11 | 0.00 |  | 0.00 |  | 230.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 265.77 | 217.66 | 255.81 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 5 | 251.30 | 203.19 | 0.00 |  | 0.00 |  | 203.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 245.42 | 197.31 | 0.00 |  | 0.00 |  | 197.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 233.94 | 185.84 | 0.00 |  | 0.00 |  | 185.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 227.15 | 179.04 | 0.00 |  | 0.00 |  | 179.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 226.03 | 177.92 | 0.00 |  | 0.00 |  | 177.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 218.55 | 170.44 | 0.00 |  | 0.00 |  | 170.44 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-housekeeping/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=584029 clocksource=tsc

### Loop counters (r1-housekeeping/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002750 | 645520 | 4.65 | 107587648 | 35.83 | 1793127 | 1793127 | 99.4 | 65.2 | true | 0 | 597 | 595.9 | 109856 / 6127 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (89052 of 2500000 in 1-msg reads) | 89052 | 2.86 | 2.61 | 3.73 | 6.49 | 13.53 | 30.29 | 36.22 |

#### Stalls: r1-housekeeping/md-recv-busy, loop 0 on CPU 1

107587648 iterations, longest 595.9 µs; over 5 / 20 / 100 µs: 109856 / 6127 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5869 | 170955.6 | 595.9 |
| mostly unexplained (+preempted by kworker/1:1) | 90 | 2436.9 | 37.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 100 | 2338.1 | 45.8 |
| softirq NET_RX | 56 | 1470.6 | 34.4 |
| mostly unexplained (+softirq NET_RX) | 9 | 228.2 | 35.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 53.5 | 33.1 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 31.5 | 31.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 595.9 | 2750 | loop work (2750 items) |  |
| 2 | 60.7 | 18 | loop work (18 items) | softirq NET_RX 20, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 58.0 | 16 | loop work (16 items) | softirq NET_RX 18, preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 57.7 | 13 | loop work (13 items) | softirq NET_RX 20, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 57.5 | 16 | loop work (16 items) | softirq NET_RX 12, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 57.4 | 7 | loop work (7 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 57.3 | 12 | loop work (12 items) | softirq NET_RX 20, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 57.2 | 13 | loop work (13 items) | softirq NET_RX 20, preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 55.6 | 3 | loop work (3 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 55.2 | 23 | loop work (23 items) | softirq NET_RX 15, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 606 |
| LOC Local timer interrupts | 600 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-housekeeping/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-housekeeping/noise, loop 0 on CPU 1

9011695616 iterations, longest 30.6 µs; over 5 / 20 / 100 µs: 76 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 75 | 680.4 | 30.6 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 6.1 | 6.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 30.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 28.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 18.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 13.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 12.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 11.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 10.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 10.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 10.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |

### Loop counters (r1-housekeeping/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1280003 | 1280003 | 1.00 | 106708036 | 83.37 | 1778467 | 1757134 | 98.8 | 71.4 | true | 0 | 2304 | 133.1 | 42032 / 486 / 1 | 0 | 0 |

#### Stalls: r1-housekeeping/pp-recv-busy-0, loop 0 on CPU 1

105428032 iterations, longest 133.1 µs; over 5 / 20 / 100 µs: 42032 / 486 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 329 | 7425.5 | 38.1 |
| softirq NET_RX | 77 | 1726.7 | 39.5 |
| mostly unexplained (+softirq NET_RX) | 65 | 1498.1 | 27.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 325.2 | 133.1 |
| mostly unexplained (+local timer tick) | 4 | 96.7 | 26.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 44.4 | 23.4 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 21.5 | 21.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 133.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 47.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 39.5 | 1 | softirq NET_RX | softirq NET_RX 21, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 38.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 35.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 33.3 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 30.6 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 30.6 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 29.8 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 29.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2343 |
| IWI IRQ work interrupts | 2307 |
| CAL Function call interrupts | 989 |

#### Slowest round trips: r1-housekeeping/pp-recv-busy-0 (median 46.04)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 260.84 | 214.80 | 0.00 |  | 52.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 162.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 180.22 | 134.18 | 0.00 |  | 0.00 |  | 134.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 178.81 | 132.77 | 0.00 |  | 0.00 |  | 132.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 171.70 | 125.66 | 132.43 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 166.81 | 120.77 | 0.00 |  | 0.00 |  | 120.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 163.71 | 117.67 | 0.00 |  | 0.00 |  | 117.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 162.15 | 116.11 | 0.00 |  | 0.00 |  | 116.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 160.33 | 114.29 | 0.00 |  | 0.00 |  | 114.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 157.89 | 111.85 | 0.00 |  | 0.00 |  | 111.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 157.82 | 111.78 | 0.00 |  | 0.00 |  | 111.78 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-housekeeping/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196868 | 1196868 | 1.00 | 223627845 | 186.84 | 1863565 | 1853591 | 99.5 | 68.3 | true | 0 | 3492 | 174.5 | 61237 / 1017 / 2 | 0 | 0 |

#### Stalls: r1-housekeeping/pp-recv-busy-10000, loop 0 on CPU 1

222430976 iterations, longest 174.5 µs; over 5 / 20 / 100 µs: 61237 / 1017 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 730 | 17524.8 | 34.7 |
| mostly unexplained (+softirq NET_RX) | 161 | 4026.8 | 35.5 |
| softirq NET_RX | 84 | 1890.8 | 32.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 20 | 804.0 | 174.5 |
| mostly unexplained (+local timer tick) | 14 | 360.3 | 35.9 |
| softirq TIMER | 1 | 111.9 | 111.9 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 104.5 | 30.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 81.2 | 33.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 174.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 111.9 | 0 | softirq TIMER | softirq TIMER 99, local timer tick 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 3 | 92.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 76.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 70.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 44.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 35.9 | 1 | mostly unexplained (+local timer tick) | local timer tick 9, timer tick_nohz_handler 6, preempted by kworker/1:1 4, irq work (IPI/vector) 1 |
| 8 | 35.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 17, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 34.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 34.6 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3654 |
| IWI IRQ work interrupts | 3502 |
| CAL Function call interrupts | 1038 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-housekeeping/pp-recv-busy-10000 (median 46.55)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 994.56 | 948.01 | 0.00 |  | 0.00 |  | 948.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 234.20 | 187.65 | 0.00 |  | 0.00 |  | 187.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 216.91 | 170.36 | 174.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 213.06 | 166.50 | 0.00 |  | 0.00 |  | 166.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 175.62 | 129.06 | 0.00 |  | 0.00 |  | 129.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 173.60 | 127.05 | 0.00 |  | 128.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 172.38 | 125.83 | 0.00 |  | 0.00 |  | 125.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 168.12 | 121.57 | 0.00 |  | 0.00 |  | 121.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 163.19 | 116.64 | 0.00 |  | 0.00 |  | 116.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 151.62 | 105.06 | 0.00 |  | 0.00 |  | 105.06 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-housekeeping/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1150542 | 1150542 | 1.00 | 3451627 | 3.00 | 57527 | 44599789 | 0.0 | 9.6 | false | 0 | 0 | 133.7 | 27882 / 21 / 1 | 0 | 0 |

#### Stalls: r1-housekeeping/pp-uring-spin-0, loop 0 on CPU 1

2675987328 iterations, longest 133.7 µs; over 5 / 20 / 100 µs: 27882 / 21 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 18 | 581.6 | 133.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 46.3 | 24.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 33.2 | 33.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 133.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 45.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 41.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 33.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 31.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 27.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 26.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 25.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 24.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 24.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| LOC Local timer interrupts | 8 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-housekeeping/pp-uring-spin-0 (median 48.42)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 229.31 | 180.88 | 0.00 |  | 0.00 |  | 180.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 211.12 | 162.69 | 0.00 |  | 0.00 |  | 162.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 208.35 | 159.93 | 0.00 |  | 0.00 |  | 159.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 205.73 | 157.30 | 0.00 |  | 0.00 |  | 157.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 203.30 | 154.88 | 0.00 |  | 0.00 |  | 154.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 201.23 | 152.80 | 0.00 |  | 0.00 |  | 152.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 200.12 | 151.70 | 0.00 |  | 0.00 |  | 151.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 197.54 | 149.11 | 0.00 |  | 0.00 |  | 149.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 197.46 | 149.04 | 0.00 |  | 0.00 |  | 149.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 197.31 | 148.88 | 0.00 |  | 0.00 |  | 148.88 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-housekeeping/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1192082 | 1192082 | 1.00 | 3576247 | 3.00 | 29802 | 37665383 | 0.0 | 5.6 | false | 0 | 0 | 79.1 | 9918 / 11 / 0 | 0 | 0 |

#### Stalls: r1-housekeeping/pp-uring-spin-10000, loop 0 on CPU 1

4519846016 iterations, longest 79.1 µs; over 5 / 20 / 100 µs: 9918 / 11 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 284.6 | 79.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.0 | 32.4 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 28.8 | 28.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 79.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 41.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 39.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 32.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 29.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 28.8 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 26.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 25.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 22.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1001 |
| IWI IRQ work interrupts | 4 |

#### Slowest round trips: r1-housekeeping/pp-uring-spin-10000 (median 55.41)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1054.08 | 998.67 | 0.00 |  | 0.00 |  | 998.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 367.64 | 312.23 | 0.00 |  | 0.00 |  | 312.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 358.02 | 302.61 | 0.00 |  | 0.00 |  | 302.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 357.52 | 302.11 | 0.00 |  | 0.00 |  | 302.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 349.97 | 294.56 | 0.00 |  | 0.00 |  | 294.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 349.65 | 294.24 | 0.00 |  | 0.00 |  | 294.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 347.98 | 292.57 | 0.00 |  | 0.00 |  | 292.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 347.37 | 291.96 | 0.00 |  | 0.00 |  | 291.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 346.85 | 291.44 | 0.00 |  | 0.00 |  | 291.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 346.30 | 290.89 | 0.00 |  | 0.00 |  | 290.89 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-housekeeping/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1238300 | 1238300 | 1.00 | 1 | 0.00 | 0 | 35447271 | 0.0 | 0.0 | true | 0 | 0 | 39.8 | 50 / 3 / 0 | 0 | 0 |

#### Stalls: r1-housekeeping/pp-uring-sqpoll-0, loop 0 on CPU 1

2126836288 iterations, longest 39.8 µs; over 5 / 20 / 100 µs: 50 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 62.2 | 39.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.1 | 35.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 39.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 35.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 22.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| LOC Local timer interrupts | 12 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-housekeeping/pp-uring-sqpoll-0 (median 47.89)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 205.90 | 158.01 | 0.00 |  | 0.00 |  | 158.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 195.15 | 147.26 | 0.00 |  | 0.00 |  | 147.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 193.65 | 145.76 | 0.00 |  | 0.00 |  | 145.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 185.93 | 138.04 | 0.00 |  | 0.00 |  | 138.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 180.82 | 132.93 | 0.00 |  | 0.00 |  | 132.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 180.12 | 132.23 | 0.00 |  | 0.00 |  | 132.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 179.89 | 132.00 | 0.00 |  | 0.00 |  | 132.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 179.00 | 131.11 | 0.00 |  | 0.00 |  | 131.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 177.35 | 129.46 | 0.00 |  | 0.00 |  | 129.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 174.60 | 126.71 | 0.00 |  | 0.00 |  | 126.71 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-housekeeping/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1182817 | 1182817 | 1.00 | 1 | 0.00 | 0 | 24450405 | 0.0 | 0.0 | true | 0 | 0 | 130.0 | 107 / 7 / 2 | 0 | 0 |

#### Stalls: r1-housekeeping/pp-uring-sqpoll-10000, loop 0 on CPU 1

2934048576 iterations, longest 130.0 µs; over 5 / 20 / 100 µs: 107 / 7 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 380.3 | 130.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.7 | 20.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 130.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 110.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 49.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 37.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 32.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 20.7 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 20.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1016 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: r1-housekeeping/pp-uring-sqpoll-10000 (median 47.29)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 381.92 | 334.63 | 0.00 |  | 0.00 |  | 334.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 368.64 | 321.35 | 0.00 |  | 0.00 |  | 321.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 363.08 | 315.79 | 0.00 |  | 0.00 |  | 315.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 359.11 | 311.82 | 0.00 |  | 0.00 |  | 311.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 358.58 | 311.30 | 0.00 |  | 0.00 |  | 311.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 358.28 | 310.99 | 0.00 |  | 0.00 |  | 310.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 356.09 | 308.80 | 0.00 |  | 0.00 |  | 308.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 356.07 | 308.78 | 0.00 |  | 0.00 |  | 308.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 355.49 | 308.20 | 0.00 |  | 0.00 |  | 308.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 353.37 | 306.07 | 0.00 |  | 0.00 |  | 306.07 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-prefer/http-recv-busy-10000, loop 0 on CPU 1

1664952255 iterations, longest 117.3 µs; over 5 / 20 / 100 µs: 1165741 / 100 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 93 | 2181.4 | 117.3 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 93.6 | 33.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 81.3 | 36.8 |
| loop work | 1 | 25.9 | 25.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 117.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 46.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 44.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 40.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 36.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 33.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 32.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 31.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 31.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 28.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| LOC Local timer interrupts | 101 |
| IWI IRQ work interrupts | 6 |

#### Slowest round trips: r1-prefer/http-recv-busy-10000 (median 49.13)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 231.76 | 182.63 | 0.00 |  | 43.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 138.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 160.76 | 111.64 | 0.00 |  | 0.00 |  | 111.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 159.60 | 110.48 | 117.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 159.37 | 110.25 | 0.00 |  | 0.00 |  | 110.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 155.52 | 106.40 | 0.00 |  | 0.00 |  | 106.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 153.78 | 104.65 | 0.00 |  | 0.00 |  | 104.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 152.12 | 102.99 | 0.00 |  | 0.00 |  | 102.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 142.54 | 93.42 | 0.00 |  | 0.00 |  | 93.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 136.43 | 87.31 | 0.00 |  | 0.00 |  | 87.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 136.32 | 87.19 | 0.00 |  | 0.00 |  | 87.19 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-prefer/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=449749 clocksource=tsc

### Loop counters (r1-prefer/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002750 | 432755 | 6.94 | 24688000 | 8.22 | 411467 | 411467 | 98.2 | 89.2 | true | 0 | 0 | 569.4 | 381496 / 4415 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (45123 of 2500000 in 1-msg reads) | 45123 | 4.47 | 4.46 | 4.93 | 5.50 | 6.17 | 6.86 | 8.08 |

#### Stalls: r1-prefer/md-recv-busy, loop 0 on CPU 1

24688000 iterations, longest 569.4 µs; over 5 / 20 / 100 µs: 381496 / 4415 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4337 | 129570.3 | 569.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 75 | 1963.8 | 120.8 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 61.3 | 37.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.4 | 21.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 569.4 | 2700 | loop work (2700 items) |  |
| 2 | 120.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 45.1 | 26 | loop work (26 items) |  |
| 4 | 45.1 | 16 | loop work (16 items) |  |
| 5 | 43.8 | 26 | loop work (26 items) |  |
| 6 | 43.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 43.4 | 29 | loop work (29 items) |  |
| 8 | 43.2 | 29 | loop work (29 items) |  |
| 9 | 42.9 | 25 | loop work (25 items) |  |
| 10 | 42.6 | 24 | loop work (24 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| LOC Local timer interrupts | 113 |
| IWI IRQ work interrupts | 11 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-prefer/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-prefer/noise, loop 0 on CPU 1

9028313088 iterations, longest 33.1 µs; over 5 / 20 / 100 µs: 51 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 49 | 436.9 | 33.1 |
| mostly unexplained (+call function (IPI/vector)) | 2 | 11.2 | 6.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 33.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 25.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 20.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 16.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 12.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 11.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 10.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 10.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 10.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |

### Loop counters (r1-prefer/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1104333 | 1104333 | 1.00 | 23975950 | 21.71 | 399599 | 381194 | 95.2 | 90.1 | true | 0 | 0 | 126.6 | 744594 / 10 / 1 | 0 | 0 |

#### Stalls: r1-prefer/pp-recv-busy-0, loop 0 on CPU 1

22871616 iterations, longest 126.6 µs; over 5 / 20 / 100 µs: 744594 / 10 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 347.6 | 126.6 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 29.1 | 29.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.8 | 20.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 126.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 52.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 49.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 29.1 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 27.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 25.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 24.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 22.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 20.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 20.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 104 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-prefer/pp-recv-busy-0 (median 52.30)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 256.81 | 204.51 | 0.00 |  | 246.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 188.48 | 136.19 | 0.00 |  | 0.00 |  | 136.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 179.92 | 127.62 | 0.00 |  | 0.00 |  | 127.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 175.56 | 123.27 | 0.00 |  | 0.00 |  | 123.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 173.36 | 121.06 | 126.63 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 173.18 | 120.88 | 0.00 |  | 0.00 |  | 120.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 172.77 | 120.47 | 0.00 |  | 0.00 |  | 120.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 167.93 | 115.63 | 0.00 |  | 0.00 |  | 115.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 165.16 | 112.87 | 0.00 |  | 0.00 |  | 112.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 162.74 | 110.44 | 0.00 |  | 0.00 |  | 110.44 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-prefer/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1189936 | 1189936 | 1.00 | 48653168 | 40.89 | 405443 | 395527 | 97.5 | 89.4 | true | 0 | 0 | 361.9 | 1524172 / 19 / 3 | 0 | 0 |

#### Stalls: r1-prefer/pp-recv-busy-10000, loop 0 on CPU 1

47463232 iterations, longest 361.9 µs; over 5 / 20 / 100 µs: 1524172 / 19 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 13 | 567.0 | 121.3 |
| softirq TIMER | 1 | 361.9 | 361.9 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 119.9 | 33.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.1 | 20.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 361.9 | 0 | softirq TIMER | softirq TIMER 334, local timer tick 17, timer tick_nohz_handler 10, softirq NET_RX 6 |
| 2 | 121.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 112.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 55.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 46.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 46.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 33.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 32.6 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 31.3 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 2, ipi_send_cpu 0 |
| 10 | 31.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| LOC Local timer interrupts | 103 |
| IWI IRQ work interrupts | 9 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-prefer/pp-recv-busy-10000 (median 50.43)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 392.79 | 342.36 | 0.00 |  | 0.00 |  | 342.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 369.73 | 319.30 | 361.86 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 354.87 | 304.44 | 0.00 |  | 0.00 |  | 304.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 318.21 | 267.79 | 0.00 |  | 0.00 |  | 267.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 268.23 | 217.80 | 0.00 |  | 0.00 |  | 217.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 234.79 | 184.37 | 0.00 |  | 0.00 |  | 184.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 197.12 | 146.69 | 0.00 |  | 0.00 |  | 146.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 182.16 | 131.73 | 0.00 |  | 136.94 | softirq TIMER | 0.00 | server: softirq TIMER |
| 9 | 156.39 | 105.96 | 0.00 |  | 0.00 |  | 105.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 154.35 | 103.92 | 0.00 |  | 0.00 |  | 103.92 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-prefer/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1150967 | 1150967 | 1.00 | 1 | 0.00 | 0 | 46943474 | 0.0 | 0.0 | true | 0 | 0 | 40.6 | 51 / 3 / 0 | 0 | 0 |

#### Stalls: r1-prefer/pp-uring-sqpoll-0, loop 0 on CPU 1

2816608448 iterations, longest 40.6 µs; over 5 / 20 / 100 µs: 51 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 58.8 | 36.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 40.6 | 40.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 40.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 36.3 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 22.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| LOC Local timer interrupts | 11 |
| IWI IRQ work interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-prefer/pp-uring-sqpoll-0 (median 49.48)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 277.65 | 228.17 | 0.00 |  | 0.00 |  | 228.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 197.88 | 148.40 | 0.00 |  | 0.00 |  | 148.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 195.94 | 146.45 | 0.00 |  | 0.00 |  | 146.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 191.20 | 141.72 | 0.00 |  | 0.00 |  | 141.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 190.91 | 141.43 | 0.00 |  | 0.00 |  | 141.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 186.96 | 137.47 | 0.00 |  | 0.00 |  | 137.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 184.18 | 134.70 | 0.00 |  | 0.00 |  | 134.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 178.41 | 128.93 | 0.00 |  | 0.00 |  | 128.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 177.76 | 128.28 | 0.00 |  | 0.00 |  | 128.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 174.63 | 125.15 | 0.00 |  | 0.00 |  | 125.15 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-prefer/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199614 | 1199614 | 1.00 | 1 | 0.00 | 0 | 35784525 | 0.0 | 0.0 | true | 0 | 0 | 107.3 | 86 / 6 / 1 | 0 | 0 |

#### Stalls: r1-prefer/pp-uring-sqpoll-10000, loop 0 on CPU 1

4294143040 iterations, longest 107.3 µs; over 5 / 20 / 100 µs: 86 / 6 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 175.3 | 107.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 77.7 | 36.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 107.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 41.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 36.1 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 26.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 21.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 20.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-prefer/pp-uring-sqpoll-10000 (median 50.83)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 998.23 | 947.40 | 0.00 |  | 0.00 |  | 947.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 215.32 | 164.49 | 0.00 |  | 0.00 |  | 164.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 206.96 | 156.13 | 0.00 |  | 0.00 |  | 156.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 199.98 | 149.15 | 0.00 |  | 0.00 |  | 149.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 197.59 | 146.76 | 0.00 |  | 0.00 |  | 146.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 193.74 | 142.91 | 0.00 |  | 0.00 |  | 142.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 193.65 | 142.82 | 0.00 |  | 0.00 |  | 142.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 184.19 | 133.36 | 0.00 |  | 0.00 |  | 133.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 183.05 | 132.22 | 0.00 |  | 0.00 |  | 132.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 180.04 | 129.22 | 0.00 |  | 0.00 |  | 129.22 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-2/http-recv-busy-10000, loop 0 on CPU 1

1840120992 iterations, longest 147.5 µs; over 5 / 20 / 100 µs: 68113 / 2873 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1696 | 38359.5 | 147.5 |
| mostly unexplained (+preempted by kworker/1:1) | 588 | 14212.3 | 46.2 |
| mostly unexplained (+softirq NET_RX) | 415 | 9523.7 | 47.9 |
| softirq NET_RX | 86 | 2012.5 | 42.3 |
| mostly unexplained (+local timer tick) | 67 | 1882.8 | 43.4 |
| mostly unexplained (+irq work (IPI/vector)) | 16 | 434.2 | 38.9 |
| loop work | 4 | 85.7 | 25.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.1 | 20.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 147.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 143.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 136.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 122.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 56.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 47.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 46.2 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 45.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 44.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 44.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3702 |
| IWI IRQ work interrupts | 3593 |
| CAL Function call interrupts | 995 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-2/http-recv-busy-10000 (median 46.04)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 160.64 | 114.60 | 0.00 |  | 0.00 |  | 114.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 153.87 | 107.83 | 0.00 |  | 0.00 |  | 107.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 136.96 | 90.91 | 0.00 |  | 0.00 |  | 90.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 131.97 | 85.92 | 0.00 |  | 0.00 |  | 85.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 130.45 | 84.41 | 0.00 |  | 0.00 |  | 84.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 126.62 | 80.57 | 0.00 |  | 0.00 |  | 80.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 125.97 | 79.93 | 0.00 |  | 0.00 |  | 79.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 122.36 | 76.31 | 0.00 |  | 0.00 |  | 76.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 120.26 | 74.22 | 0.00 |  | 0.00 |  | 74.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 118.73 | 72.68 | 0.00 |  | 29.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 43.48 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-2/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=412931 clocksource=tsc

### Loop counters (r1-base-2/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002750 | 466471 | 6.44 | 109430080 | 36.44 | 1823835 | 1823835 | 99.6 | 65.0 | true | 0 | 726 | 606.1 | 88342 / 4702 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (56075 of 2500000 in 1-msg reads) | 56075 | 2.79 | 2.55 | 3.68 | 5.39 | 20.45 | 35.88 | 37.33 |

#### Stalls: r1-base-2/md-recv-busy, loop 0 on CPU 1

109430080 iterations, longest 606.1 µs; over 5 / 20 / 100 µs: 88342 / 4702 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4420 | 125400.3 | 606.1 |
| mostly unexplained (+preempted by kworker/1:1) | 153 | 4094.9 | 38.3 |
| softirq NET_RX | 69 | 1822.6 | 39.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 47 | 1213.0 | 151.3 |
| softirq TIMER | 1 | 346.2 | 346.2 |
| mostly unexplained (+softirq NET_RX) | 10 | 264.4 | 32.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 44.1 | 23.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 606.1 | 2750 | loop work (2750 items) |  |
| 2 | 346.2 | 0 | softirq TIMER | softirq TIMER 335, local timer tick 6, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 3 | 151.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 140.8 | 12 | loop work (12 items) |  |
| 5 | 68.1 | 30 | loop work (30 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 62.4 | 12 | loop work (12 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 61.6 | 26 | loop work (26 items) | softirq NET_RX 20, preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 60.7 | 25 | loop work (25 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 59.8 | 10 | loop work (10 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 57.4 | 12 | loop work (12 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 736 |
| IWI IRQ work interrupts | 732 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-2/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-2/noise, loop 0 on CPU 1

9020960768 iterations, longest 37.8 µs; over 5 / 20 / 100 µs: 63 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 62 | 594.9 | 37.8 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 6.2 | 6.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 37.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 33.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 22.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 16.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 15.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 14.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 13.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 12.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 10.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |

### Loop counters (r1-base-2/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1166142 | 1166142 | 1.00 | 282329533 | 242.11 | 4705492 | 4666621 | 0.0 | 40.2 | false | 0 | 0 | 149.0 | 18314 / 17 / 2 | 0 | 0 |

#### Stalls: r1-base-2/pp-epoll-spin-0, loop 0 on CPU 1

279997248 iterations, longest 149.0 µs; over 5 / 20 / 100 µs: 18314 / 17 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 16 | 664.5 | 149.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 149.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 126.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 40.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 38.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 35.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 31.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 30.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 29.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 26.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 24.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 987 |
| LOC Local timer interrupts | 7 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-2/pp-epoll-spin-0 (median 49.03)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 630.86 | 581.82 | 0.00 |  | 0.00 |  | 581.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 532.76 | 483.73 | 0.00 |  | 0.00 |  | 483.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 509.38 | 460.35 | 0.00 |  | 0.00 |  | 460.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 482.38 | 433.35 | 0.00 |  | 0.00 |  | 433.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 466.18 | 417.15 | 0.00 |  | 0.00 |  | 417.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 428.72 | 379.68 | 0.00 |  | 0.00 |  | 379.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 426.55 | 377.51 | 0.00 |  | 0.00 |  | 377.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 332.78 | 283.74 | 0.00 |  | 0.00 |  | 283.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 295.79 | 246.76 | 0.00 |  | 0.00 |  | 246.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 295.14 | 246.10 | 0.00 |  | 0.00 |  | 246.10 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1178350 | 1178350 | 1.00 | 563568605 | 478.27 | 4696405 | 4676766 | 0.0 | 37.4 | false | 0 | 0 | 148.2 | 27387 / 18 / 4 | 0 | 0 |

#### Stalls: r1-base-2/pp-epoll-spin-10000, loop 0 on CPU 1

561211904 iterations, longest 148.2 µs; over 5 / 20 / 100 µs: 27387 / 18 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 16 | 840.1 | 148.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 65.1 | 44.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 148.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 135.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 121.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 118.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 44.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 40.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 34.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 34.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 29.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 26.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| IWI IRQ work interrupts | 4 |

#### Slowest round trips: r1-base-2/pp-epoll-spin-10000 (median 60.65)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 395.01 | 334.37 | 0.00 |  | 0.00 |  | 334.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 370.38 | 309.74 | 0.00 |  | 0.00 |  | 309.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 365.10 | 304.45 | 0.00 |  | 0.00 |  | 304.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 361.78 | 301.13 | 0.00 |  | 0.00 |  | 301.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 357.46 | 296.82 | 0.00 |  | 0.00 |  | 296.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 356.05 | 295.40 | 0.00 |  | 0.00 |  | 295.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 354.80 | 294.15 | 0.00 |  | 0.00 |  | 294.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 350.73 | 290.08 | 0.00 |  | 0.00 |  | 290.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 350.41 | 289.76 | 0.00 |  | 0.00 |  | 289.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 348.30 | 287.65 | 0.00 |  | 0.00 |  | 287.65 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1306501 | 1306501 | 1.00 | 106990918 | 81.89 | 1783182 | 1761407 | 98.8 | 71.6 | true | 0 | 2453 | 151.5 | 40043 / 521 / 1 | 0 | 0 |

#### Stalls: r1-base-2/pp-recv-busy-0, loop 0 on CPU 1

105684416 iterations, longest 151.5 µs; over 5 / 20 / 100 µs: 40043 / 521 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 343 | 7703.1 | 32.7 |
| mostly unexplained (+softirq NET_RX) | 85 | 1978.2 | 37.3 |
| softirq NET_RX | 67 | 1503.6 | 36.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 13 | 470.4 | 151.5 |
| mostly unexplained (+local timer tick) | 7 | 170.0 | 29.8 |
| mostly unexplained (+irq work (IPI/vector)) | 5 | 132.0 | 31.2 |
| softirq TIMER | 1 | 78.2 | 78.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 151.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 78.2 | 1 | softirq TIMER | softirq TIMER 58, local timer tick 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 3 | 39.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 37.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 11, preempted by kworker/1:1 8, local timer tick 4, irq work (IPI/vector) 1 |
| 5 | 36.0 | 0 | softirq NET_RX | softirq NET_RX 18, preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 34.6 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 17, preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 33.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 32.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 32.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2500 |
| IWI IRQ work interrupts | 2463 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-2/pp-recv-busy-0 (median 44.58)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1042.84 | 998.26 | 0.00 |  | 0.00 |  | 998.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 231.53 | 186.95 | 0.00 |  | 0.00 |  | 186.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 217.88 | 173.31 | 0.00 |  | 132.80 | softirq TIMER | 40.50 | server: softirq TIMER |
| 4 | 184.93 | 140.36 | 0.00 |  | 0.00 |  | 140.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 173.41 | 128.83 | 151.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 145.77 | 101.19 | 0.00 |  | 29.74 | unexplained: no kernel event (hypervisor or in-kernel time) | 71.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 144.07 | 99.50 | 0.00 |  | 0.00 |  | 99.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 143.92 | 99.34 | 0.00 |  | 0.00 |  | 99.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 135.98 | 91.41 | 0.00 |  | 0.00 |  | 91.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 135.56 | 90.99 | 0.00 |  | 0.00 |  | 90.99 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196868 | 1196868 | 1.00 | 223920644 | 187.09 | 1866005 | 1856031 | 99.5 | 68.2 | true | 0 | 4109 | 49.0 | 44663 / 1022 / 0 | 0 | 0 |

#### Stalls: r1-base-2/pp-recv-busy-10000, loop 0 on CPU 1

222723776 iterations, longest 49.0 µs; over 5 / 20 / 100 µs: 44663 / 1022 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 722 | 16599.6 | 41.4 |
| mostly unexplained (+softirq NET_RX) | 137 | 3298.1 | 34.9 |
| softirq NET_RX | 117 | 2663.9 | 30.3 |
| mostly unexplained (+local timer tick) | 20 | 517.6 | 32.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 19 | 508.7 | 49.0 |
| mostly unexplained (+irq work (IPI/vector)) | 5 | 129.3 | 34.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 59.9 | 35.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 49.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 45.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 41.4 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, local timer tick 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 4 | 40.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 35.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 35.2 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 7 | 34.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 12, preempted by kworker/1:1 5, local timer tick 4, irq work (IPI/vector) 1 |
| 8 | 34.3 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 34.2 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 33.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 11, local timer tick 4, preempted by kworker/1:1 4, timer tick_nohz_handler 2 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4177 |
| IWI IRQ work interrupts | 4118 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-base-2/pp-recv-busy-10000 (median 46.67)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 254.20 | 207.53 | 0.00 |  | 0.00 |  | 207.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 205.16 | 158.49 | 0.00 |  | 0.00 |  | 158.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 205.04 | 158.38 | 0.00 |  | 0.00 |  | 158.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 185.77 | 139.10 | 0.00 |  | 0.00 |  | 139.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 185.62 | 138.96 | 0.00 |  | 0.00 |  | 138.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 174.63 | 127.96 | 0.00 |  | 0.00 |  | 127.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 174.26 | 127.59 | 0.00 |  | 0.00 |  | 127.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 171.88 | 125.22 | 0.00 |  | 0.00 |  | 125.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 170.61 | 123.94 | 0.00 |  | 0.00 |  | 123.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 170.24 | 123.57 | 0.00 |  | 0.00 |  | 123.57 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1154963 | 1154963 | 1.00 | 3464890 | 3.00 | 57748 | 44591423 | 0.0 | 9.7 | false | 0 | 0 | 124.8 | 47575 / 13 / 2 | 0 | 0 |

#### Stalls: r1-base-2/pp-uring-spin-0, loop 0 on CPU 1

2675485376 iterations, longest 124.8 µs; over 5 / 20 / 100 µs: 47575 / 13 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 10 | 356.3 | 124.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 134.8 | 111.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 35.7 | 35.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 124.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 111.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 46.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 35.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 31.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 24.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 23.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 22.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 22.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 22.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 11 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-2/pp-uring-spin-0 (median 48.46)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 266.89 | 218.44 | 0.00 |  | 0.00 |  | 218.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 202.84 | 154.39 | 0.00 |  | 0.00 |  | 154.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 198.59 | 150.13 | 0.00 |  | 0.00 |  | 150.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 195.48 | 147.03 | 0.00 |  | 0.00 |  | 147.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 195.11 | 146.66 | 0.00 |  | 0.00 |  | 146.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 193.62 | 145.17 | 0.00 |  | 0.00 |  | 145.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 192.77 | 144.31 | 0.00 |  | 0.00 |  | 144.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 190.77 | 142.31 | 0.00 |  | 0.00 |  | 142.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 187.18 | 138.72 | 0.00 |  | 0.00 |  | 138.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 186.18 | 137.73 | 0.00 |  | 0.00 |  | 137.73 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1191029 | 1191029 | 1.00 | 3573088 | 3.00 | 29776 | 37226473 | 0.0 | 5.5 | false | 0 | 0 | 254.2 | 43636 / 13 / 1 | 0 | 0 |

#### Stalls: r1-base-2/pp-uring-spin-10000, loop 0 on CPU 1

4467176768 iterations, longest 254.2 µs; over 5 / 20 / 100 µs: 43636 / 13 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 254.2 | 254.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 245.2 | 43.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 56.2 | 31.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 31.2 | 31.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 254.2 | 0 | softirq TIMER | softirq TIMER 229, local timer tick 14, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 43.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 39.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 31.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 31.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 26.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 26.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 24.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 24.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 22.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-2/pp-uring-spin-10000 (median 51.87)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 376.73 | 324.87 | 0.00 |  | 0.00 |  | 324.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 346.50 | 294.63 | 0.00 |  | 0.00 |  | 294.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 345.64 | 293.77 | 0.00 |  | 0.00 |  | 293.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 340.88 | 289.01 | 0.00 |  | 0.00 |  | 289.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 334.50 | 282.64 | 0.00 |  | 0.00 |  | 282.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 334.11 | 282.24 | 0.00 |  | 0.00 |  | 282.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 332.66 | 280.79 | 0.00 |  | 0.00 |  | 280.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 332.62 | 280.76 | 0.00 |  | 0.00 |  | 280.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 332.34 | 280.47 | 0.00 |  | 0.00 |  | 280.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 329.72 | 277.85 | 0.00 |  | 0.00 |  | 277.85 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1266255 | 1266255 | 1.00 | 1 | 0.00 | 0 | 32268115 | 0.0 | 0.0 | true | 0 | 0 | 132.6 | 60 / 4 / 1 | 0 | 0 |

#### Stalls: r1-base-2/pp-uring-sqpoll-0, loop 0 on CPU 1

1936086912 iterations, longest 132.6 µs; over 5 / 20 / 100 µs: 60 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 177.3 | 132.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.1 | 35.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 132.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 44.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 35.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 20.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| LOC Local timer interrupts | 12 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-2/pp-uring-sqpoll-0 (median 46.77)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1041.15 | 994.37 | 0.00 |  | 0.00 |  | 994.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 267.71 | 220.94 | 0.00 |  | 0.00 |  | 220.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 236.16 | 189.39 | 0.00 |  | 222.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 219.22 | 172.45 | 0.00 |  | 0.00 |  | 172.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 199.81 | 153.03 | 0.00 |  | 0.00 |  | 153.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 199.11 | 152.34 | 0.00 |  | 0.00 |  | 152.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 187.93 | 141.15 | 0.00 |  | 0.00 |  | 141.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 187.79 | 141.01 | 0.00 |  | 0.00 |  | 141.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 179.39 | 132.62 | 0.00 |  | 0.00 |  | 132.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 175.82 | 129.04 | 132.59 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-2/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1181201 | 1181201 | 1.00 | 1 | 0.00 | 0 | 24632622 | 0.0 | 0.0 | true | 0 | 0 | 128.9 | 94 / 4 / 2 | 0 | 0 |

#### Stalls: r1-base-2/pp-uring-sqpoll-10000, loop 0 on CPU 1

2955914624 iterations, longest 128.9 µs; over 5 / 20 / 100 µs: 94 / 4 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 304.0 | 128.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 128.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 118.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 36.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 20.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-base-2/pp-uring-sqpoll-10000 (median 46.44)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 396.43 | 349.98 | 0.00 |  | 0.00 |  | 349.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 365.86 | 319.41 | 0.00 |  | 0.00 |  | 319.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 352.46 | 306.01 | 0.00 |  | 0.00 |  | 306.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 351.06 | 304.62 | 0.00 |  | 0.00 |  | 304.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 350.82 | 304.38 | 0.00 |  | 0.00 |  | 304.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 350.52 | 304.08 | 0.00 |  | 0.00 |  | 304.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 350.05 | 303.61 | 0.00 |  | 0.00 |  | 303.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 346.59 | 300.14 | 0.00 |  | 0.00 |  | 300.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 345.98 | 299.54 | 0.00 |  | 0.00 |  | 299.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 345.79 | 299.35 | 0.00 |  | 0.00 |  | 299.35 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-noquickack/http-recv-busy-10000, loop 0 on CPU 1

682706749 iterations, longest 266.2 µs; over 5 / 20 / 100 µs: 27909 / 1112 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 470 | 11119.8 | 38.6 |
| mostly unexplained (+preempted by kworker/1:1) | 327 | 7802.3 | 40.5 |
| softirq NET_RX | 169 | 3963.3 | 41.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 73 | 2063.8 | 149.5 |
| mostly unexplained (+softirq NET_RX) | 65 | 1616.1 | 41.9 |
| softirq TIMER | 1 | 266.2 | 266.2 |
| mostly unexplained (+local timer tick) | 5 | 136.8 | 31.4 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 43.5 | 22.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 266.2 | 0 | softirq TIMER | softirq TIMER 249, local timer tick 11, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 149.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 147.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 130.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 129.2 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 41.9 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 16, preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 41.5 | 0 | softirq NET_RX | softirq NET_RX 25, local timer tick 9, preempted by ksoftirqd/1 8, timer tick_nohz_handler 5 |
| 8 | 40.5 | 2 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 10, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 9 | 38.6 | 2 | loop work (2 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 38.5 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 3055 |
| LOC Local timer interrupts | 2004 |
| CAL Function call interrupts | 995 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-noquickack/http-recv-busy-10000 (median 46.91)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 394.24 | 347.32 | 0.00 |  | 0.00 |  | 347.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 389.89 | 342.97 | 0.00 |  | 0.00 |  | 342.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 379.68 | 332.77 | 0.00 |  | 0.00 |  | 332.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 378.17 | 331.26 | 0.00 |  | 0.00 |  | 331.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 371.42 | 324.51 | 39.90 | mostly unexplained (+preempted by kworker/1:1) | 0.00 |  | 284.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 369.89 | 322.98 | 266.20 | softirq TIMER | 0.00 |  | 56.78 | client: softirq TIMER |
| 7 | 369.02 | 322.11 | 0.00 |  | 0.00 |  | 322.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 368.77 | 321.86 | 0.00 |  | 0.00 |  | 321.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 368.68 | 321.76 | 0.00 |  | 0.00 |  | 321.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 368.22 | 321.30 | 0.00 |  | 0.00 |  | 321.30 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-noquickack/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=310328 clocksource=tsc

### Loop counters (r1-noquickack/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002750 | 352658 | 8.51 | 110077184 | 36.66 | 1834620 | 1834620 | 99.7 | 64.7 | true | 0 | 276 | 623.5 | 91836 / 3716 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (46618 of 2500000 in 1-msg reads) | 46618 | 2.40 | 2.54 | 3.50 | 4.12 | 5.44 | 10.55 | 36.39 |

#### Stalls: r1-noquickack/md-recv-busy, loop 0 on CPU 1

110077184 iterations, longest 623.5 µs; over 5 / 20 / 100 µs: 91836 / 3716 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 3498 | 96208.7 | 623.5 |
| mostly unexplained (+preempted by kworker/1:1) | 98 | 2860.3 | 40.7 |
| softirq NET_RX | 50 | 1482.1 | 40.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 54 | 1277.0 | 30.1 |
| mostly unexplained (+softirq NET_RX) | 15 | 440.7 | 38.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.3 | 20.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 623.5 | 2750 | loop work (2750 items) |  |
| 2 | 158.2 | 50 | loop work (50 items) |  |
| 3 | 154.8 | 31 | loop work (31 items) |  |
| 4 | 65.8 | 19 | loop work (19 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 65.6 | 24 | loop work (24 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 64.3 | 22 | loop work (22 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 62.8 | 29 | loop work (29 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 60.4 | 19 | loop work (19 items) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 59.9 | 11 | loop work (11 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 59.4 | 11 | loop work (11 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 988 |
| IWI IRQ work interrupts | 600 |
| LOC Local timer interrupts | 292 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-noquickack/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-noquickack/noise, loop 0 on CPU 1

9037619200 iterations, longest 140.9 µs; over 5 / 20 / 100 µs: 77 / 5 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 76 | 1082.4 | 140.9 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 7.2 | 7.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 140.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 110.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 98.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 68.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 24.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 19.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 18.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 14.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 14.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 12.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |

### Loop counters (r1-noquickack/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1331180 | 1331180 | 1.00 | 110004909 | 82.64 | 1833415 | 1811229 | 98.8 | 70.7 | true | 0 | 2419 | 148.8 | 7380 / 455 / 2 | 0 | 0 |

#### Stalls: r1-noquickack/pp-recv-busy-0, loop 0 on CPU 1

108673728 iterations, longest 148.8 µs; over 5 / 20 / 100 µs: 7380 / 455 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 272 | 6004.8 | 34.0 |
| mostly unexplained (+softirq NET_RX) | 87 | 1978.9 | 32.4 |
| softirq NET_RX | 83 | 1814.9 | 29.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 399.1 | 148.8 |
| softirq TIMER | 1 | 70.7 | 70.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 66.5 | 41.9 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 51.5 | 31.3 |
| mostly unexplained (+local timer tick) | 2 | 50.9 | 27.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 148.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 137.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 70.7 | 0 | softirq TIMER | softirq TIMER 59, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 4 | 41.9 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 34.0 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 13, softirq NET_RX 12, workqueue [ena] 9, local timer tick 4 |
| 6 | 32.4 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 12, preempted by kworker/1:1 10, local timer tick 4, irq work (IPI/vector) 1 |
| 7 | 31.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 31.4 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 31.3 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 31.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2612 |
| LOC Local timer interrupts | 2449 |
| CAL Function call interrupts | 988 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-noquickack/pp-recv-busy-0 (median 44.42)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 366.12 | 321.69 | 70.74 | softirq TIMER | 0.00 |  | 250.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 199.41 | 154.98 | 148.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 6.14 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 175.04 | 130.62 | 0.00 |  | 135.79 | softirq TIMER | 0.00 | server: softirq TIMER |
| 4 | 171.39 | 126.97 | 0.00 |  | 0.00 |  | 126.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 166.14 | 121.71 | 0.00 |  | 26.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 95.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 149.36 | 104.93 | 137.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 143.31 | 98.88 | 0.00 |  | 0.00 |  | 98.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 135.53 | 91.10 | 0.00 |  | 0.00 |  | 91.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 132.77 | 88.35 | 0.00 |  | 0.00 |  | 88.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 129.86 | 85.44 | 0.00 |  | 0.00 |  | 85.44 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-noquickack/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1052896 | 1052896 | 1.00 | 230309344 | 218.74 | 1919245 | 1910470 | 99.5 | 68.1 | true | 0 | 3381 | 153.7 | 11019 / 1102 / 4 | 0 | 0 |

#### Stalls: r1-noquickack/pp-recv-busy-10000, loop 0 on CPU 1

229256448 iterations, longest 153.7 µs; over 5 / 20 / 100 µs: 11019 / 1102 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 751 | 17789.0 | 153.7 |
| mostly unexplained (+softirq NET_RX) | 203 | 4979.7 | 34.1 |
| softirq NET_RX | 122 | 2785.6 | 35.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 10 | 611.6 | 151.3 |
| mostly unexplained (+local timer tick) | 14 | 358.8 | 30.1 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 32.5 | 32.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.3 | 23.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 153.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 2 | 151.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 145.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 124.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 38.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 35.6 | 0 | softirq NET_RX | softirq NET_RX 19, preempted by kworker/1:1 9, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 34.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/1:1 10, local timer tick 3, irq work (IPI/vector) 1 |
| 8 | 33.8 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 9 | 33.2 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 11, preempted by kworker/1:1 8, local timer tick 4, irq work (IPI/vector) 1 |
| 10 | 32.5 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4943 |
| LOC Local timer interrupts | 3466 |
| CAL Function call interrupts | 996 |

#### Slowest round trips: r1-noquickack/pp-recv-busy-10000 (median 44.51)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 401.18 | 356.68 | 0.00 |  | 0.00 |  | 356.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 361.08 | 316.57 | 0.00 |  | 0.00 |  | 316.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 360.31 | 315.81 | 0.00 |  | 0.00 |  | 315.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 358.10 | 313.60 | 0.00 |  | 0.00 |  | 313.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 357.87 | 313.37 | 0.00 |  | 0.00 |  | 313.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 356.37 | 311.86 | 0.00 |  | 0.00 |  | 311.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 355.00 | 310.50 | 0.00 |  | 0.00 |  | 310.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 353.87 | 309.37 | 0.00 |  | 0.00 |  | 309.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 353.68 | 309.18 | 0.00 |  | 0.00 |  | 309.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 353.16 | 308.66 | 0.00 |  | 0.00 |  | 308.66 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-noquickack/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1160519 | 1160519 | 1.00 | 3481558 | 3.00 | 58026 | 45676705 | 0.0 | 7.5 | false | 0 | 0 | 143.6 | 1422 / 6 / 2 | 0 | 0 |

#### Stalls: r1-noquickack/pp-uring-spin-0, loop 0 on CPU 1

2740602304 iterations, longest 143.6 µs; over 5 / 20 / 100 µs: 1422 / 6 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 370.7 | 143.6 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 33.8 | 33.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.4 | 21.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 143.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 125.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 70.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 33.8 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 30.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 21.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| LOC Local timer interrupts | 13 |
| IWI IRQ work interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-noquickack/pp-uring-spin-0 (median 49.84)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 375.07 | 325.22 | 0.00 |  | 0.00 |  | 325.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 280.46 | 230.62 | 0.00 |  | 0.00 |  | 230.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 235.97 | 186.12 | 0.00 |  | 0.00 |  | 186.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 227.08 | 177.24 | 0.00 |  | 0.00 |  | 177.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 217.38 | 167.54 | 0.00 |  | 0.00 |  | 167.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 210.01 | 160.16 | 0.00 |  | 0.00 |  | 160.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 207.00 | 157.15 | 0.00 |  | 0.00 |  | 157.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 204.17 | 154.33 | 0.00 |  | 0.00 |  | 154.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 199.92 | 150.08 | 0.00 |  | 0.00 |  | 150.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 199.74 | 149.90 | 0.00 |  | 0.00 |  | 149.90 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-noquickack/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199585 | 1199585 | 1.00 | 3598756 | 3.00 | 29990 | 36163826 | 0.0 | 4.0 | false | 0 | 0 | 149.6 | 1293 / 11 / 5 | 0 | 0 |

#### Stalls: r1-noquickack/pp-uring-spin-10000, loop 0 on CPU 1

4339659072 iterations, longest 149.6 µs; over 5 / 20 / 100 µs: 1293 / 11 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 732.9 | 149.6 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 30.4 | 30.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.9 | 21.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 149.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 139.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 123.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 115.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 106.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 30.4 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 27.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 26.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 23.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 3 |

#### Slowest round trips: r1-noquickack/pp-uring-spin-10000 (median 53.17)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 207.05 | 153.87 | 0.00 |  | 0.00 |  | 153.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 206.01 | 152.84 | 149.62 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 3.22 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 196.27 | 143.10 | 0.00 |  | 0.00 |  | 143.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 188.62 | 135.45 | 0.00 |  | 0.00 |  | 135.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 186.35 | 133.18 | 0.00 |  | 0.00 |  | 133.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 182.74 | 129.57 | 0.00 |  | 0.00 |  | 129.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 182.11 | 128.94 | 0.00 |  | 0.00 |  | 128.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 178.97 | 125.79 | 0.00 |  | 0.00 |  | 125.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 169.36 | 116.19 | 0.00 |  | 0.00 |  | 116.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 168.35 | 115.18 | 115.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

#### Stalls: r1-nogro/http-recv-busy-10000, loop 0 on CPU 1

1585632306 iterations, longest 149.6 µs; over 5 / 20 / 100 µs: 63591 / 1180 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 652 | 16725.6 | 42.9 |
| loop work | 131 | 3432.8 | 38.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 115 | 3153.1 | 149.6 |
| mostly unexplained (+softirq NET_RX) | 100 | 2578.5 | 37.2 |
| mostly unexplained (+local timer tick) | 88 | 2319.7 | 41.7 |
| softirq NET_RX | 76 | 1868.4 | 37.3 |
| mostly unexplained (+irq work (IPI/vector)) | 17 | 405.6 | 31.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 149.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 141.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 125.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 120.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 80.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 42.9 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 42.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 41.7 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, preempted by kworker/1:1 5, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 9 | 41.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 40.9 | 1 | mostly unexplained (+local timer tick) | local timer tick 8, preempted by kworker/1:1 6, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2625 |
| IWI IRQ work interrupts | 2467 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-nogro/http-recv-busy-10000 (median 47.80)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 363.37 | 315.58 | 0.00 |  | 0.00 |  | 315.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 352.78 | 304.98 | 0.00 |  | 0.00 |  | 304.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 350.47 | 302.67 | 0.00 |  | 0.00 |  | 302.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 347.12 | 299.32 | 0.00 |  | 0.00 |  | 299.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 338.55 | 290.75 | 0.00 |  | 0.00 |  | 290.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 335.79 | 287.99 | 0.00 |  | 0.00 |  | 287.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 335.49 | 287.69 | 0.00 |  | 0.00 |  | 287.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 334.59 | 286.79 | 0.00 |  | 0.00 |  | 286.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 332.56 | 284.77 | 0.00 |  | 0.00 |  | 284.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 332.42 | 284.62 | 0.00 |  | 0.00 |  | 284.62 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-nogro/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=330038 clocksource=tsc

### Loop counters (r1-nogro/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002750 | 371440 | 8.08 | 108650944 | 36.18 | 1810849 | 1810849 | 99.7 | 64.4 | true | 0 | 681 | 547.9 | 135323 / 4306 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (42132 of 2500000 in 1-msg reads) | 42132 | 3.02 | 2.84 | 3.90 | 5.97 | 21.61 | 31.33 | 38.02 |

#### Stalls: r1-nogro/md-recv-busy, loop 0 on CPU 1

108650944 iterations, longest 547.9 µs; over 5 / 20 / 100 µs: 135323 / 4306 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4074 | 116116.7 | 547.9 |
| mostly unexplained (+preempted by kworker/1:1) | 123 | 3156.3 | 51.5 |
| softirq NET_RX | 70 | 1782.5 | 35.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 29 | 915.6 | 144.7 |
| mostly unexplained (+softirq NET_RX) | 7 | 175.7 | 28.8 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 81.8 | 32.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 547.9 | 2750 | loop work (2750 items) |  |
| 2 | 144.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 125.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 59.1 | 25 | loop work (25 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 54.2 | 12 | loop work (12 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 54.1 | 14 | loop work (14 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 54.1 | 11 | loop work (11 items) | softirq NET_RX 19, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 52.9 | 20 | loop work (20 items) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 52.7 | 18 | loop work (18 items) | softirq NET_RX 16, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 52.4 | 20 | loop work (20 items) | softirq NET_RX 15, preempted by kworker/1:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 701 |
| LOC Local timer interrupts | 683 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-nogro/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-nogro/noise, loop 0 on CPU 1

9037488128 iterations, longest 164.5 µs; over 5 / 20 / 100 µs: 52 / 5 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 50 | 818.3 | 150.8 |
| softirq TIMER | 1 | 164.5 | 164.5 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 6.6 | 6.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 164.5 | 0 | softirq TIMER | softirq TIMER 143, local timer tick 13, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 150.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 110.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 104.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 75.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 15.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 15.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 13.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 13.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 12.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |
| IWI IRQ work interrupts | 1 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (r1-nogro/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1217179 | 1217179 | 1.00 | 108941084 | 89.50 | 1815685 | 1795398 | 98.9 | 71.1 | true | 0 | 2649 | 149.5 | 36341 / 484 / 2 | 0 | 0 |

#### Stalls: r1-nogro/pp-recv-busy-0, loop 0 on CPU 1

107723904 iterations, longest 149.5 µs; over 5 / 20 / 100 µs: 36341 / 484 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 330 | 7325.3 | 39.3 |
| softirq NET_RX | 70 | 1549.4 | 39.6 |
| mostly unexplained (+softirq NET_RX) | 59 | 1339.8 | 31.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 482.5 | 149.5 |
| mostly unexplained (+local timer tick) | 12 | 320.3 | 33.6 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 80.0 | 30.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 149.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 126.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 50.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 39.6 | 0 | softirq NET_RX | softirq NET_RX 20, preempted by kworker/1:1 9, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 39.3 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 10, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 34.4 | 1 | softirq NET_RX | softirq NET_RX 20, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 33.6 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, preempted by kworker/1:1 5, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 8 | 33.2 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 10, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 9 | 33.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 32.8 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, preempted by kworker/1:1 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2701 |
| IWI IRQ work interrupts | 2655 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-nogro/pp-recv-busy-0 (median 48.91)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 415.77 | 366.86 | 0.00 |  | 0.00 |  | 366.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 310.57 | 261.66 | 0.00 |  | 0.00 |  | 261.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 295.10 | 246.19 | 0.00 |  | 0.00 |  | 246.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 251.41 | 202.50 | 0.00 |  | 0.00 |  | 202.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 193.91 | 145.00 | 0.00 |  | 0.00 |  | 145.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 187.05 | 138.14 | 0.00 |  | 0.00 |  | 138.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 179.31 | 130.40 | 0.00 |  | 0.00 |  | 130.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 179.13 | 130.22 | 149.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 178.12 | 129.21 | 0.00 |  | 27.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 101.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 173.97 | 125.06 | 0.00 |  | 0.00 |  | 125.06 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-nogro/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196864 | 1196864 | 1.00 | 222311553 | 185.75 | 1852596 | 1842622 | 99.5 | 68.1 | true | 0 | 3955 | 149.4 | 47996 / 946 / 4 | 0 | 0 |

#### Stalls: r1-nogro/pp-recv-busy-10000, loop 0 on CPU 1

221114688 iterations, longest 149.4 µs; over 5 / 20 / 100 µs: 47996 / 946 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 716 | 16661.0 | 38.9 |
| softirq NET_RX | 128 | 2963.7 | 38.6 |
| mostly unexplained (+softirq NET_RX) | 69 | 1811.0 | 149.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 15 | 700.2 | 148.2 |
| mostly unexplained (+local timer tick) | 15 | 392.4 | 34.5 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 61.8 | 32.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.5 | 21.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 149.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 148.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 122.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 115.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 43.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 38.9 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 12, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 7 | 38.6 | 1 | softirq NET_RX | softirq NET_RX 22, preempted by ksoftirqd/1 9, local timer tick 6, timer tick_nohz_handler 3 |
| 8 | 36.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 12, preempted by kworker/1:1 5, local timer tick 3, timer tick_nohz_handler 2 |
| 9 | 35.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 35.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4088 |
| IWI IRQ work interrupts | 3961 |
| CAL Function call interrupts | 997 |

#### Slowest round trips: r1-nogro/pp-recv-busy-10000 (median 44.39)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 196.23 | 151.84 | 149.26 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 2.58 | client: mostly unexplained (+softirq NET_RX) |
| 2 | 175.38 | 130.99 | 0.00 |  | 0.00 |  | 130.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 166.76 | 122.37 | 0.00 |  | 0.00 |  | 122.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 165.54 | 121.15 | 0.00 |  | 0.00 |  | 121.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 162.10 | 117.71 | 0.00 |  | 0.00 |  | 117.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 160.31 | 115.92 | 0.00 |  | 0.00 |  | 115.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 159.38 | 114.99 | 0.00 |  | 0.00 |  | 114.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 156.70 | 112.31 | 0.00 |  | 0.00 |  | 112.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 155.47 | 111.08 | 0.00 |  | 0.00 |  | 111.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 154.50 | 110.11 | 0.00 |  | 0.00 |  | 110.11 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-nogro/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1117566 | 1117566 | 1.00 | 3352699 | 3.00 | 55878 | 44982626 | 0.0 | 9.0 | false | 0 | 0 | 151.2 | 28163 / 13 / 3 | 0 | 0 |

#### Stalls: r1-nogro/pp-uring-spin-0, loop 0 on CPU 1

2698957568 iterations, longest 151.2 µs; over 5 / 20 / 100 µs: 28163 / 13 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 11 | 579.6 | 151.2 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 35.8 | 35.8 |
| mostly unexplained (+local timer tick) | 1 | 31.5 | 31.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 151.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 122.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 112.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 35.8 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 31.5 | 0 | mostly unexplained (+local timer tick) | local timer tick 13, timer tick_nohz_handler 8, softirq TIMER 2 |
| 6 | 29.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 28.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 25.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 24.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 5 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-nogro/pp-uring-spin-0 (median 48.88)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 228.53 | 179.65 | 0.00 |  | 0.00 |  | 179.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 194.47 | 145.59 | 0.00 |  | 0.00 |  | 145.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 186.05 | 137.17 | 0.00 |  | 0.00 |  | 137.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 183.88 | 135.00 | 0.00 |  | 0.00 |  | 135.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 183.68 | 134.80 | 0.00 |  | 0.00 |  | 134.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 179.08 | 130.20 | 0.00 |  | 0.00 |  | 130.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 179.02 | 130.14 | 0.00 |  | 0.00 |  | 130.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 177.57 | 128.69 | 0.00 |  | 0.00 |  | 128.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 173.13 | 124.25 | 0.00 |  | 0.00 |  | 124.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 167.17 | 118.29 | 151.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-nogro/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1174600 | 1174600 | 1.00 | 3523801 | 3.00 | 29365 | 35816022 | 0.0 | 5.3 | false | 0 | 0 | 145.2 | 27765 / 18 / 5 | 0 | 0 |

#### Stalls: r1-nogro/pp-uring-spin-10000, loop 0 on CPU 1

4297922688 iterations, longest 145.2 µs; over 5 / 20 / 100 µs: 27765 / 18 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 16 | 788.2 | 145.2 |
| softirq TIMER | 1 | 111.1 | 111.1 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 26.3 | 26.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 145.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 138.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 136.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 113.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 111.1 | 0 | softirq TIMER | softirq TIMER 76, local timer tick 17, timer tick_nohz_handler 10 |
| 6 | 26.3 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 25.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 21.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 21.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-nogro/pp-uring-spin-10000 (median 49.36)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 507.14 | 457.78 | 0.00 |  | 0.00 |  | 457.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 476.85 | 427.49 | 0.00 |  | 0.00 |  | 427.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 468.01 | 418.65 | 0.00 |  | 0.00 |  | 418.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 465.87 | 416.51 | 0.00 |  | 0.00 |  | 416.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 460.57 | 411.21 | 0.00 |  | 0.00 |  | 411.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 459.46 | 410.10 | 0.00 |  | 0.00 |  | 410.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 450.39 | 401.03 | 0.00 |  | 0.00 |  | 401.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 449.84 | 400.48 | 0.00 |  | 0.00 |  | 400.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 447.89 | 398.53 | 0.00 |  | 0.00 |  | 398.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 446.10 | 396.74 | 0.00 |  | 0.00 |  | 396.74 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-4/http-recv-busy-10000, loop 0 on CPU 1

1775417520 iterations, longest 150.6 µs; over 5 / 20 / 100 µs: 163683 / 1688 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 815 | 20718.7 | 49.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 398 | 9192.6 | 150.6 |
| mostly unexplained (+softirq NET_RX) | 299 | 7241.4 | 45.0 |
| softirq NET_RX | 140 | 3494.2 | 43.0 |
| mostly unexplained (+irq work (IPI/vector)) | 17 | 381.3 | 27.7 |
| mostly unexplained (+local timer tick) | 15 | 343.9 | 29.6 |
| loop work | 3 | 81.1 | 39.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.8 | 20.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 150.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 149.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 133.5 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 129.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 55.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 49.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 46.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 45.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 17, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 44.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 43.2 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2551 |
| IWI IRQ work interrupts | 2532 |
| CAL Function call interrupts | 1000 |

#### Slowest round trips: r1-base-4/http-recv-busy-10000 (median 48.05)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 984.67 | 936.62 | 0.00 |  | 0.00 |  | 936.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 398.03 | 349.98 | 0.00 |  | 0.00 |  | 349.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 396.80 | 348.76 | 0.00 |  | 0.00 |  | 348.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 361.67 | 313.62 | 0.00 |  | 0.00 |  | 313.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 356.38 | 308.33 | 0.00 |  | 0.00 |  | 308.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 356.35 | 308.30 | 0.00 |  | 0.00 |  | 308.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 323.06 | 275.01 | 0.00 |  | 0.00 |  | 275.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 277.17 | 229.12 | 0.00 |  | 0.00 |  | 229.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 270.88 | 222.83 | 0.00 |  | 0.00 |  | 222.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 205.61 | 157.56 | 0.00 |  | 0.00 |  | 157.56 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-4/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=485733 clocksource=tsc

### Loop counters (r1-base-4/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002750 | 542313 | 5.54 | 108659520 | 36.19 | 1810992 | 1810992 | 99.5 | 65.2 | true | 0 | 799 | 577.7 | 114863 / 5433 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (89692 of 2500000 in 1-msg reads) | 89692 | 2.70 | 2.40 | 3.54 | 5.93 | 15.20 | 30.14 | 36.16 |

#### Stalls: r1-base-4/md-recv-busy, loop 0 on CPU 1

108659520 iterations, longest 577.7 µs; over 5 / 20 / 100 µs: 114863 / 5433 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5162 | 148675.0 | 577.7 |
| mostly unexplained (+preempted by kworker/1:1) | 129 | 3434.5 | 60.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 71 | 1858.8 | 146.7 |
| softirq NET_RX | 59 | 1536.5 | 44.7 |
| mostly unexplained (+softirq NET_RX) | 11 | 282.1 | 33.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.7 | 20.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 577.7 | 2750 | loop work (2750 items) |  |
| 2 | 146.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 134.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 64.3 | 22 | loop work (22 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 60.8 | 3 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 59.9 | 19 | loop work (19 items) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 58.9 | 32 | loop work (32 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 56.2 | 7 | loop work (7 items) | softirq NET_RX 22, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 56.1 | 3 | loop work (3 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 53.9 | 16 | loop work (16 items) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 806 |
| LOC Local timer interrupts | 801 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-4/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-4/noise, loop 0 on CPU 1

9034391552 iterations, longest 128.1 µs; over 5 / 20 / 100 µs: 49 / 4 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 48 | 797.2 | 128.1 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 6.8 | 6.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 128.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 116.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 109.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 70.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 19.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 16.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 16.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 14.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 13.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |

### Loop counters (r1-base-4/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1148914 | 1148914 | 1.00 | 281148645 | 244.71 | 4685811 | 4647514 | 0.0 | 40.6 | false | 0 | 0 | 142.2 | 34773 / 23 / 2 | 0 | 0 |

#### Stalls: r1-base-4/pp-epoll-spin-0, loop 0 on CPU 1

278850816 iterations, longest 142.2 µs; over 5 / 20 / 100 µs: 34773 / 23 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 22 | 778.8 | 142.2 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 24.0 | 24.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 142.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 130.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 35.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 33.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 29.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 28.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 27.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 26.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 26.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 26.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 5 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-4/pp-epoll-spin-0 (median 49.10)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 421.13 | 372.03 | 0.00 |  | 0.00 |  | 372.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 345.94 | 296.84 | 0.00 |  | 0.00 |  | 296.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 308.40 | 259.30 | 0.00 |  | 0.00 |  | 259.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 282.90 | 233.80 | 0.00 |  | 0.00 |  | 233.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 234.28 | 185.18 | 0.00 |  | 0.00 |  | 185.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 216.80 | 167.70 | 0.00 |  | 0.00 |  | 167.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 201.68 | 152.58 | 0.00 |  | 0.00 |  | 152.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 201.49 | 152.39 | 0.00 |  | 0.00 |  | 152.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 201.25 | 152.15 | 0.00 |  | 0.00 |  | 152.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 198.66 | 149.56 | 0.00 |  | 0.00 |  | 149.56 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-4/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1195847 | 1195847 | 1.00 | 554401103 | 463.61 | 4620009 | 4600078 | 0.0 | 37.2 | false | 0 | 0 | 263.7 | 30819 / 20 / 5 | 0 | 0 |

#### Stalls: r1-base-4/pp-epoll-spin-10000, loop 0 on CPU 1

552009408 iterations, longest 263.7 µs; over 5 / 20 / 100 µs: 30819 / 20 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 18 | 914.8 | 153.1 |
| softirq TIMER | 1 | 263.7 | 263.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.9 | 20.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 263.7 | 1 | softirq TIMER | softirq TIMER 232, local timer tick 17, timer tick_nohz_handler 9, irq work (IPI/vector) 1 |
| 2 | 153.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 144.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 136.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 113.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 79.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 34.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 27.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 22.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-4/pp-epoll-spin-10000 (median 51.01)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1053.62 | 1002.61 | 0.00 |  | 0.00 |  | 1002.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 298.80 | 247.78 | 263.20 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 268.22 | 217.21 | 0.00 |  | 230.85 | softirq TIMER | 0.00 | server: softirq TIMER |
| 4 | 243.95 | 192.94 | 0.00 |  | 0.00 |  | 192.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 243.95 | 192.94 | 0.00 |  | 219.61 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 222.87 | 171.86 | 0.00 |  | 0.00 |  | 171.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 217.88 | 166.87 | 0.00 |  | 0.00 |  | 166.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 215.09 | 164.07 | 0.00 |  | 0.00 |  | 164.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 214.27 | 163.26 | 0.00 |  | 0.00 |  | 163.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 208.27 | 157.26 | 0.00 |  | 0.00 |  | 157.26 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-4/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1246668 | 1246668 | 1.00 | 107835853 | 86.50 | 1797264 | 1776486 | 98.8 | 71.0 | true | 0 | 2842 | 154.3 | 35393 / 540 / 2 | 0 | 0 |

#### Stalls: r1-base-4/pp-recv-busy-0, loop 0 on CPU 1

106589184 iterations, longest 154.3 µs; over 5 / 20 / 100 µs: 35393 / 540 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 389 | 8717.3 | 38.0 |
| mostly unexplained (+softirq NET_RX) | 72 | 1804.5 | 154.3 |
| softirq NET_RX | 61 | 1344.7 | 27.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 255.5 | 130.9 |
| mostly unexplained (+local timer tick) | 8 | 191.8 | 36.5 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 52.1 | 27.8 |
| mostly unexplained (+ipi_send_cpu) | 1 | 29.6 | 29.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.2 | 20.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 154.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 130.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 38.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 36.5 | 1 | mostly unexplained (+local timer tick) | local timer tick 7, preempted by kworker/1:1 5, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 5 | 34.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 11, local timer tick 5, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 6 | 32.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 16, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 32.2 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 31.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 30.7 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 10 | 29.9 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2901 |
| IWI IRQ work interrupts | 2848 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-4/pp-recv-busy-0 (median 48.26)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 192.07 | 143.82 | 153.97 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 2 | 168.76 | 120.51 | 130.77 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 163.94 | 115.68 | 0.00 |  | 153.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 145.93 | 97.67 | 0.00 |  | 0.00 |  | 97.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 145.00 | 96.75 | 0.00 |  | 0.00 |  | 96.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 142.43 | 94.18 | 0.00 |  | 0.00 |  | 94.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 141.14 | 92.88 | 0.00 |  | 0.00 |  | 92.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 140.44 | 92.18 | 0.00 |  | 0.00 |  | 92.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 138.80 | 90.54 | 0.00 |  | 0.00 |  | 90.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 138.31 | 90.06 | 0.00 |  | 0.00 |  | 90.06 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-4/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196882 | 1196882 | 1.00 | 223225682 | 186.51 | 1860214 | 1850240 | 99.5 | 68.5 | true | 0 | 3357 | 405.1 | 59107 / 849 / 5 | 0 | 0 |

#### Stalls: r1-base-4/pp-recv-busy-10000, loop 0 on CPU 1

222028800 iterations, longest 405.1 µs; over 5 / 20 / 100 µs: 59107 / 849 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 617 | 14201.7 | 37.1 |
| softirq NET_RX | 112 | 2547.4 | 33.0 |
| mostly unexplained (+softirq NET_RX) | 75 | 1810.7 | 34.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 21 | 991.4 | 146.4 |
| mostly unexplained (+local timer tick) | 20 | 492.4 | 31.7 |
| softirq TIMER | 1 | 405.1 | 405.1 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 37.2 | 37.2 |
| mostly unexplained (+ipi_send_cpu) | 1 | 26.2 | 26.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.5 | 21.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 405.1 | 0 | softirq TIMER | softirq TIMER 393, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 2 | 146.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 144.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 124.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 114.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 73.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 46.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 37.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 37.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 34.6 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3529 |
| IWI IRQ work interrupts | 3364 |
| CAL Function call interrupts | 999 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-4/pp-recv-busy-10000 (median 44.10)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 322.58 | 278.48 | 0.00 |  | 286.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 277.15 | 233.05 | 0.00 |  | 0.00 |  | 233.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 211.67 | 167.57 | 0.00 |  | 134.01 | softirq TIMER | 33.56 | server: softirq TIMER |
| 4 | 208.69 | 164.59 | 0.00 |  | 0.00 |  | 164.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 204.86 | 160.76 | 0.00 |  | 0.00 |  | 160.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 197.01 | 152.91 | 0.00 |  | 0.00 |  | 152.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 188.60 | 144.50 | 0.00 |  | 0.00 |  | 144.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 185.82 | 141.72 | 0.00 |  | 0.00 |  | 141.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 183.34 | 139.24 | 0.00 |  | 0.00 |  | 139.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 177.12 | 133.02 | 0.00 |  | 0.00 |  | 133.02 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-4/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1128837 | 1128837 | 1.00 | 3386512 | 3.00 | 56442 | 44680810 | 0.0 | 9.6 | false | 0 | 0 | 147.8 | 42066 / 11 / 1 | 0 | 0 |

#### Stalls: r1-base-4/pp-uring-spin-0, loop 0 on CPU 1

2680848576 iterations, longest 147.8 µs; over 5 / 20 / 100 µs: 42066 / 11 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 335.4 | 147.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 60.2 | 36.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 32.8 | 32.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 147.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 36.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 36.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 33.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 32.8 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 27.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 24.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 23.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 23.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 21.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-4/pp-uring-spin-0 (median 48.62)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 331.27 | 282.65 | 0.00 |  | 0.00 |  | 282.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 295.66 | 247.04 | 0.00 |  | 0.00 |  | 247.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 295.29 | 246.67 | 0.00 |  | 0.00 |  | 246.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 292.26 | 243.64 | 0.00 |  | 0.00 |  | 243.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 287.05 | 238.43 | 0.00 |  | 0.00 |  | 238.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 278.37 | 229.75 | 0.00 |  | 0.00 |  | 229.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 270.29 | 221.67 | 0.00 |  | 0.00 |  | 221.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 269.43 | 220.81 | 0.00 |  | 0.00 |  | 220.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 267.44 | 218.82 | 0.00 |  | 0.00 |  | 218.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 265.29 | 216.68 | 0.00 |  | 0.00 |  | 216.68 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-4/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1193370 | 1193370 | 1.00 | 3580111 | 3.00 | 29834 | 37393531 | 0.0 | 5.5 | false | 0 | 0 | 145.6 | 8971 / 15 / 4 | 0 | 0 |

#### Stalls: r1-base-4/pp-uring-spin-10000, loop 0 on CPU 1

4487223680 iterations, longest 145.6 µs; over 5 / 20 / 100 µs: 8971 / 15 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 13 | 801.4 | 145.6 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 25.7 | 25.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.6 | 21.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 145.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 135.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 132.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 118.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 59.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 44.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 33.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 25.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 23.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 22.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 5 |

#### Slowest round trips: r1-base-4/pp-uring-spin-10000 (median 53.95)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 352.39 | 298.44 | 0.00 |  | 0.00 |  | 298.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 350.83 | 296.88 | 0.00 |  | 0.00 |  | 296.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 347.42 | 293.47 | 0.00 |  | 0.00 |  | 293.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 341.93 | 287.98 | 0.00 |  | 0.00 |  | 287.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 338.30 | 284.34 | 0.00 |  | 0.00 |  | 284.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 333.45 | 279.49 | 0.00 |  | 0.00 |  | 279.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 333.16 | 279.21 | 0.00 |  | 0.00 |  | 279.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 332.39 | 278.44 | 0.00 |  | 0.00 |  | 278.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 332.21 | 278.25 | 0.00 |  | 0.00 |  | 278.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 331.34 | 277.39 | 0.00 |  | 0.00 |  | 277.39 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-4/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1187242 | 1187242 | 1.00 | 1 | 0.00 | 0 | 39275533 | 0.0 | 0.0 | true | 0 | 0 | 212.7 | 47 / 5 / 3 | 0 | 0 |

#### Stalls: r1-base-4/pp-uring-sqpoll-0, loop 0 on CPU 1

2356531968 iterations, longest 212.7 µs; over 5 / 20 / 100 µs: 47 / 5 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 327.6 | 150.0 |
| softirq TIMER | 1 | 212.7 | 212.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 212.7 | 1 | softirq TIMER | softirq TIMER 186, local timer tick 14, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 150.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 126.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 27.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 24.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| LOC Local timer interrupts | 13 |
| IWI IRQ work interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-4/pp-uring-sqpoll-0 (median 48.65)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 287.42 | 238.77 | 0.00 |  | 135.69 | softirq TIMER | 103.08 | server: softirq TIMER |
| 2 | 219.24 | 170.59 | 211.93 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 195.75 | 147.10 | 149.96 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 189.00 | 140.35 | 0.00 |  | 0.00 |  | 140.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 186.21 | 137.56 | 0.00 |  | 0.00 |  | 137.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 176.88 | 128.22 | 0.00 |  | 0.00 |  | 128.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 172.94 | 124.29 | 0.00 |  | 0.00 |  | 124.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 171.76 | 123.11 | 0.00 |  | 0.00 |  | 123.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 170.79 | 122.14 | 0.00 |  | 0.00 |  | 122.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 168.01 | 119.36 | 0.00 |  | 0.00 |  | 119.36 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-4/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199472 | 1199472 | 1.00 | 1 | 0.00 | 0 | 25735737 | 0.0 | 0.0 | true | 0 | 0 | 137.8 | 97 / 9 / 4 | 0 | 0 |

#### Stalls: r1-base-4/pp-uring-sqpoll-10000, loop 0 on CPU 1

3088288384 iterations, longest 137.8 µs; over 5 / 20 / 100 µs: 97 / 9 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 676.9 | 137.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.3 | 21.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 137.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 136.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 130.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 117.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 74.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 31.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 26.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 23.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 21.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1009 |
| LOC Local timer interrupts | 5 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-4/pp-uring-sqpoll-10000 (median 46.70)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 202.81 | 156.11 | 0.00 |  | 0.00 |  | 156.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 183.27 | 136.57 | 0.00 |  | 51.70 | unexplained: no kernel event (hypervisor or in-kernel time) | 84.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 172.94 | 126.24 | 0.00 |  | 0.00 |  | 126.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 172.27 | 125.57 | 0.00 |  | 0.00 |  | 125.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 163.79 | 117.09 | 0.00 |  | 0.00 |  | 117.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 157.48 | 110.78 | 0.00 |  | 0.00 |  | 110.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 157.45 | 110.75 | 0.00 |  | 0.00 |  | 110.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 155.62 | 108.92 | 0.00 |  | 0.00 |  | 108.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 153.73 | 107.03 | 0.00 |  | 0.00 |  | 107.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 152.63 | 105.93 | 0.00 |  | 0.00 |  | 105.93 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-nocoalesce/http-recv-busy-10000, loop 0 on CPU 1

1678108940 iterations, longest 155.9 µs; over 5 / 20 / 100 µs: 80149 / 81 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 61 | 1893.5 | 155.9 |
| mostly unexplained (+softirq NET_RX) | 12 | 268.8 | 27.4 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 132.3 | 36.5 |
| loop work | 3 | 66.5 | 22.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.9 | 20.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 155.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 149.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 128.1 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 125.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 67.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 41.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 36.5 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 2, ipi_send_cpu 0 |
| 8 | 34.6 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 32.3 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 32.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 6 |

#### Slowest round trips: r1-nocoalesce/http-recv-busy-10000 (median 47.39)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 191.63 | 144.24 | 154.98 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 186.52 | 139.13 | 148.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 147.89 | 100.50 | 0.00 |  | 0.00 |  | 100.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 147.20 | 99.81 | 0.00 |  | 0.00 |  | 99.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 144.42 | 97.03 | 0.00 |  | 0.00 |  | 97.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 142.17 | 94.78 | 0.00 |  | 0.00 |  | 94.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 142.07 | 94.69 | 0.00 |  | 0.00 |  | 94.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 140.86 | 93.47 | 0.00 |  | 0.00 |  | 93.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 140.69 | 93.31 | 0.00 |  | 0.00 |  | 93.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 140.54 | 93.15 | 0.00 |  | 0.00 |  | 93.15 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-nocoalesce/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=445628 clocksource=tsc

### Loop counters (r1-nocoalesce/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002750 | 542287 | 5.54 | 108656384 | 36.19 | 1810940 | 1810940 | 99.5 | 65.2 | true | 0 | 0 | 576.6 | 117315 / 4638 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (66221 of 2500000 in 1-msg reads) | 66221 | 3.07 | 2.77 | 3.98 | 7.87 | 11.46 | 19.12 | 22.38 |

#### Stalls: r1-nocoalesce/md-recv-busy, loop 0 on CPU 1

108656384 iterations, longest 576.6 µs; over 5 / 20 / 100 µs: 117315 / 4638 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4542 | 136952.4 | 576.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 64 | 1791.7 | 151.8 |
| softirq NET_RX | 28 | 790.4 | 37.8 |
| mostly unexplained (+softirq NET_RX) | 2 | 51.8 | 31.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 21.2 | 21.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.2 | 20.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 576.6 | 2750 | loop work (2750 items) |  |
| 2 | 151.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 132.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 58.7 | 18 | loop work (18 items) | softirq NET_RX 7 |
| 5 | 48.3 | 28 | loop work (28 items) |  |
| 6 | 48.0 | 16 | loop work (16 items) | softirq NET_RX 10 |
| 7 | 47.8 | 20 | loop work (20 items) | softirq NET_RX 8 |
| 8 | 47.7 | 28 | loop work (28 items) |  |
| 9 | 46.8 | 20 | loop work (20 items) |  |
| 10 | 46.1 | 20 | loop work (20 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 10 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-nocoalesce/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-nocoalesce/noise, loop 0 on CPU 1

9033764864 iterations, longest 142.7 µs; over 5 / 20 / 100 µs: 51 / 5 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 50 | 830.5 | 142.7 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 6.0 | 6.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 142.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 101.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 101.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 71.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 41.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 18.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 16.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 15.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 13.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 11.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |

### Loop counters (r1-nocoalesce/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1339621 | 1339621 | 1.00 | 106604326 | 79.58 | 1776739 | 1754412 | 98.7 | 71.6 | true | 0 | 0 | 147.3 | 43987 / 19 / 2 | 0 | 0 |

#### Stalls: r1-nocoalesce/pp-recv-busy-0, loop 0 on CPU 1

105264704 iterations, longest 147.3 µs; over 5 / 20 / 100 µs: 43987 / 19 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 17 | 703.8 | 147.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 40.3 | 40.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 22.8 | 22.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 147.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 135.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 40.3 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 2, softirq NET_RX 1, ipi_send_cpu 0 |
| 4 | 36.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 36.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 34.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 33.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 31.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 30.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 28.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 16 |
| IWI IRQ work interrupts | 7 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-nocoalesce/pp-recv-busy-0 (median 43.54)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1049.85 | 1006.31 | 0.00 |  | 0.00 |  | 1006.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 463.08 | 419.54 | 0.00 |  | 0.00 |  | 419.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 428.85 | 385.31 | 0.00 |  | 0.00 |  | 385.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 349.39 | 305.85 | 0.00 |  | 0.00 |  | 305.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 234.67 | 191.12 | 0.00 |  | 0.00 |  | 191.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 230.97 | 187.43 | 0.00 |  | 0.00 |  | 187.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 225.17 | 181.63 | 0.00 |  | 0.00 |  | 181.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 214.11 | 170.57 | 0.00 |  | 0.00 |  | 170.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 207.62 | 164.08 | 0.00 |  | 0.00 |  | 164.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 203.99 | 160.44 | 0.00 |  | 0.00 |  | 160.44 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-nocoalesce/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1185435 | 1185435 | 1.00 | 226297820 | 190.90 | 1885815 | 1875937 | 99.5 | 68.6 | true | 0 | 0 | 267.3 | 36124 / 20 / 5 | 0 | 0 |

#### Stalls: r1-nocoalesce/pp-recv-busy-10000, loop 0 on CPU 1

225112384 iterations, longest 267.3 µs; over 5 / 20 / 100 µs: 36124 / 20 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 16 | 891.8 | 145.2 |
| softirq TIMER | 1 | 267.3 | 267.3 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 73.9 | 38.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.0 | 21.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 267.3 | 0 | softirq TIMER | softirq TIMER 243, local timer tick 14, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 145.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 143.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 138.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 121.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 65.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 38.0 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 2, ipi_send_cpu 0 |
| 8 | 36.0 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 33.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 30.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-nocoalesce/pp-recv-busy-10000 (median 46.35)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1042.13 | 995.78 | 0.00 |  | 220.06 | softirq TIMER | 775.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 363.11 | 316.75 | 0.00 |  | 0.00 |  | 316.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 355.29 | 308.94 | 0.00 |  | 0.00 |  | 308.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 354.42 | 308.07 | 0.00 |  | 0.00 |  | 308.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 354.19 | 307.84 | 0.00 |  | 0.00 |  | 307.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 353.62 | 307.27 | 0.00 |  | 0.00 |  | 307.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 353.33 | 306.98 | 0.00 |  | 0.00 |  | 306.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 344.54 | 298.19 | 0.00 |  | 0.00 |  | 298.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 344.12 | 297.77 | 0.00 |  | 0.00 |  | 297.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 344.06 | 297.70 | 0.00 |  | 0.00 |  | 297.70 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-nocoalesce/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1107251 | 1107251 | 1.00 | 3321754 | 3.00 | 55363 | 44135719 | 0.0 | 10.6 | false | 0 | 0 | 283.7 | 179989 / 32 / 3 | 0 | 0 |

#### Stalls: r1-nocoalesce/pp-uring-spin-0, loop 0 on CPU 1

2648143168 iterations, longest 283.7 µs; over 5 / 20 / 100 µs: 179989 / 32 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 29 | 935.8 | 143.1 |
| softirq TIMER | 1 | 283.7 | 283.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 35.2 | 35.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 22.7 | 22.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 283.7 | 0 | softirq TIMER | softirq TIMER 259, local timer tick 15, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 143.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 132.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 35.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 34.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 33.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 29.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 29.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 29.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 27.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 13 |
| IWI IRQ work interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-nocoalesce/pp-uring-spin-0 (median 52.26)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 295.06 | 242.80 | 283.72 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 2 | 256.75 | 204.49 | 0.00 |  | 0.00 |  | 204.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 244.53 | 192.27 | 0.00 |  | 239.52 | softirq TIMER | 0.00 | server: softirq TIMER |
| 4 | 230.56 | 178.30 | 0.00 |  | 0.00 |  | 178.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 229.61 | 177.35 | 0.00 |  | 0.00 |  | 177.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 224.00 | 171.74 | 0.00 |  | 0.00 |  | 171.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 218.76 | 166.51 | 0.00 |  | 0.00 |  | 166.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 211.29 | 159.03 | 0.00 |  | 0.00 |  | 159.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 210.92 | 158.67 | 0.00 |  | 0.00 |  | 158.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 206.60 | 154.34 | 0.00 |  | 0.00 |  | 154.34 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-nocoalesce/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199650 | 1199650 | 1.00 | 3598951 | 3.00 | 29991 | 34481597 | 0.0 | 6.1 | false | 0 | 0 | 157.3 | 52576 / 16 / 4 | 0 | 0 |

#### Stalls: r1-nocoalesce/pp-uring-spin-10000, loop 0 on CPU 1

4137791680 iterations, longest 157.3 µs; over 5 / 20 / 100 µs: 52576 / 16 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 15 | 791.0 | 157.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 30.1 | 30.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 157.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 133.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 125.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 114.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 30.1 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 29.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 27.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 26.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 24.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 23.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-nocoalesce/pp-uring-spin-10000 (median 50.93)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 288.47 | 237.54 | 0.00 |  | 273.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 237.27 | 186.33 | 0.00 |  | 0.00 |  | 186.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 226.91 | 175.98 | 0.00 |  | 0.00 |  | 175.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 224.25 | 173.31 | 0.00 |  | 0.00 |  | 173.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 219.22 | 168.29 | 0.00 |  | 0.00 |  | 168.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 202.02 | 151.09 | 0.00 |  | 0.00 |  | 151.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 200.60 | 149.67 | 0.00 |  | 0.00 |  | 149.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 195.06 | 144.13 | 0.00 |  | 0.00 |  | 144.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 194.91 | 143.97 | 0.00 |  | 0.00 |  | 143.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 192.10 | 141.16 | 0.00 |  | 0.00 |  | 141.16 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-irqlocal/http-recv-busy-10000, loop 0 on CPU 1

1364225069 iterations, longest 152.7 µs; over 5 / 20 / 100 µs: 1127222 / 5346 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4389 | 102263.8 | 152.7 |
| loop work | 563 | 12996.2 | 38.8 |
| softirq NET_RX | 113 | 2536.2 | 35.8 |
| mostly unexplained (+softirq NET_RX) | 86 | 1966.0 | 34.8 |
| mostly unexplained (+local timer tick) | 78 | 1959.1 | 32.2 |
| mostly unexplained (+preempted by kworker/1:1) | 66 | 1455.6 | 31.6 |
| mostly unexplained (+irq enp39s0-Tx-Rx-6) | 48 | 1061.9 | 29.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 27.6 | 27.6 |
| mostly unexplained (+softirq TIMER) | 1 | 26.6 | 26.6 |
| softirq TIMER | 1 | 26.6 | 26.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 152.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 147.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 139.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 116.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 49.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 45.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 45.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 45.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 44.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 2340882 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 2173296 |
| IWI IRQ work interrupts | 17473 |
| LOC Local timer interrupts | 5462 |
| CAL Function call interrupts | 1002 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 198 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 145 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 138 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 130 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 129 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 113 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 112 |

#### Slowest round trips: r1-irqlocal/http-recv-busy-10000 (median 48.93)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 371.58 | 322.65 | 0.00 |  | 0.00 |  | 322.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 368.86 | 319.93 | 0.00 |  | 0.00 |  | 319.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 368.63 | 319.69 | 0.00 |  | 0.00 |  | 319.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 368.19 | 319.26 | 0.00 |  | 0.00 |  | 319.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 366.92 | 317.99 | 0.00 |  | 0.00 |  | 317.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 365.83 | 316.90 | 0.00 |  | 0.00 |  | 316.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 362.09 | 313.15 | 0.00 |  | 0.00 |  | 313.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 361.42 | 312.49 | 0.00 |  | 0.00 |  | 312.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 360.50 | 311.57 | 0.00 |  | 0.00 |  | 311.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 360.09 | 311.15 | 0.00 |  | 0.00 |  | 311.15 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-irqlocal/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=477509 clocksource=tsc

### Loop counters (r1-irqlocal/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002870 | 564301 | 5.32 | 106786496 | 35.56 | 1779775 | 1779775 | 99.5 | 65.0 | true | 0 | 1870 | 675.3 | 220658 / 6842 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672642880 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (73066 of 2499970 in 1-msg reads) | 73066 | 2.80 | 2.28 | 4.05 | 7.26 | 19.63 | 27.11 | 41.76 |

#### Stalls: r1-irqlocal/md-recv-busy, loop 0 on CPU 1

106786496 iterations, longest 675.3 µs; over 5 / 20 / 100 µs: 220658 / 6842 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5524 | 166422.8 | 675.3 |
| softirq NET_RX | 1099 | 29395.5 | 69.6 |
| mostly unexplained (+irq enp39s0-Tx-Rx-6) | 59 | 1610.1 | 34.2 |
| mostly unexplained (+softirq NET_RX) | 60 | 1436.2 | 36.2 |
| mostly unexplained (+preempted by kworker/1:1) | 60 | 1365.8 | 32.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 22 | 777.7 | 148.4 |
| mostly unexplained (+irq enp39s0-Tx-Rx-7) | 15 | 353.8 | 31.1 |
| softirq TIMER | 3 | 113.5 | 63.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 675.3 | 2881 | loop work (2881 items) |  |
| 2 | 148.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 126.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 84.0 | 20 | loop work (20 items) | softirq NET_RX 17, irq enp39s0-Tx-Rx-7 6, preempted by kworker/1:1 3, irq enp39s0-Tx-Rx-6 2 |
| 5 | 69.6 | 26 | softirq NET_RX | softirq NET_RX 53, irq enp39s0-Tx-Rx-7 4, irq enp39s0-Tx-Rx-6 2, irq enp39s0-Tx-Rx-3 1 |
| 6 | 65.1 | 15 | loop work (15 items) | softirq NET_RX 18, irq enp39s0-Tx-Rx-6 2, irq enp39s0-Tx-Rx-7 2, irq enp39s0-Tx-Rx-1 1 |
| 7 | 63.4 | 28 | softirq NET_RX | softirq NET_RX 37, preempted by kworker/1:1 4, irq enp39s0-Tx-Rx-7 2, irq enp39s0-Tx-Rx-6 1 |
| 8 | 63.1 | 18 | loop work (18 items) | softirq NET_RX 8, preempted by kworker/1:1 6, irq enp39s0-Tx-Rx-7 3, irq enp39s0-Tx-Rx-6 1 |
| 9 | 63.0 | 0 | softirq TIMER | softirq TIMER 54, local timer tick 4, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 10 | 60.5 | 8 | softirq NET_RX | softirq NET_RX 43, preempted by kworker/1:1 29, irq enp39s0-Tx-Rx-7 3, irq enp39s0-Tx-Rx-6 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 219134 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 186310 |
| IWI IRQ work interrupts | 2331 |
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 777 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 111 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 110 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 73 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 69 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 59 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 59 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 31 |

## r1-irqlocal/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-irqlocal/noise, loop 0 on CPU 1

9039577088 iterations, longest 146.4 µs; over 5 / 20 / 100 µs: 822 / 71 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq NET_RX | 400 | 5277.8 | 52.9 |
| mostly unexplained (+softirq NET_RX) | 152 | 1447.6 | 22.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 71 | 1045.6 | 146.4 |
| mostly unexplained (+local timer tick) | 91 | 1024.8 | 31.9 |
| local timer tick | 37 | 427.1 | 23.9 |
| mostly unexplained (+irq ena-mgmnt@pci:0000:27:00.0) | 53 | 404.7 | 13.6 |
| softirq TIMER | 1 | 62.6 | 62.6 |
| mostly unexplained (+irq enp39s0-Tx-Rx-0) | 3 | 19.6 | 8.0 |
| mostly unexplained (+irq enp39s0-Tx-Rx-4) | 3 | 18.1 | 7.6 |
| mostly unexplained (+irq enp39s0-Tx-Rx-3) | 2 | 16.9 | 11.6 |
| mostly unexplained (+irq enp39s0-Tx-Rx-2) | 3 | 15.4 | 5.3 |
| mostly unexplained (+irq enp39s0-Tx-Rx-7) | 2 | 12.6 | 7.5 |
| mostly unexplained (+irq enp39s0-Tx-Rx-1) | 2 | 11.0 | 6.0 |
| mostly unexplained (+softirq TASKLET) | 1 | 9.8 | 9.8 |
| mostly unexplained (+irq enp39s0-Tx-Rx-5) | 1 | 5.1 | 5.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 146.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 120.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 108.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 78.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 62.6 | 0 | softirq TIMER | softirq TIMER 54, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 6 | 52.9 | 0 | softirq NET_RX | softirq NET_RX 41, irq enp39s0-Tx-Rx-0 1, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 45.1 | 0 | softirq NET_RX | softirq NET_RX 39, irq enp39s0-Tx-Rx-6 1 |
| 8 | 43.9 | 0 | softirq NET_RX | softirq NET_RX 29, local timer tick 7, timer tick_nohz_handler 4, softirq RCU 2 |
| 9 | 40.0 | 0 | softirq NET_RX | softirq NET_RX 22, preempted by kworker/1:1 8, irq enp39s0-Tx-Rx-6 1, irq work (IPI/vector) 1 |
| 10 | 36.0 | 0 | softirq NET_RX | softirq NET_RX 20, preempted by kworker/1:1 5, irq enp39s0-Tx-Rx-1 1, irq enp39s0-Tx-Rx-2 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 195 |
| IWI IRQ work interrupts | 185 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 171 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 169 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 147 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 136 |
| LOC Local timer interrupts | 132 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 120 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 96 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 87 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 83 |

### Loop counters (r1-irqlocal/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1127656 | 1127656 | 1.00 | 219580177 | 194.72 | 3659670 | 3622081 | 0.0 | 44.9 | false | 0 | 8825 | 157.2 | 1237227 / 2164 / 2 | 0 | 0 |

#### Stalls: r1-irqlocal/pp-epoll-spin-0, loop 0 on CPU 1

217324864 iterations, longest 157.2 µs; over 5 / 20 / 100 µs: 1237227 / 2164 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1924 | 44369.7 | 157.2 |
| mostly unexplained (+local timer tick) | 123 | 2885.7 | 31.3 |
| softirq NET_RX | 103 | 2263.9 | 33.5 |
| mostly unexplained (+softirq NET_RX) | 12 | 268.2 | 32.9 |
| softirq TIMER | 1 | 24.3 | 24.3 |
| mostly unexplained (+irq enp39s0-Tx-Rx-4) | 1 | 20.4 | 20.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 157.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 134.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 44.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 42.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 41.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 39.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 38.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 36.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 35.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 35.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 2239569 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 1469715 |
| IWI IRQ work interrupts | 12352 |
| LOC Local timer interrupts | 5197 |
| CAL Function call interrupts | 989 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 174 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 142 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 121 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 111 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 69 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 58 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 2 |

#### Slowest round trips: r1-irqlocal/pp-epoll-spin-0 (median 49.08)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 275.95 | 226.87 | 0.00 |  | 23.55 | unexplained: no kernel event (hypervisor or in-kernel time) | 203.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 244.74 | 195.66 | 0.00 |  | 0.00 |  | 195.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 242.31 | 193.23 | 0.00 |  | 0.00 |  | 193.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 241.03 | 191.95 | 0.00 |  | 0.00 |  | 191.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 229.15 | 180.06 | 0.00 |  | 0.00 |  | 180.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 210.53 | 161.44 | 0.00 |  | 0.00 |  | 161.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 208.28 | 159.19 | 0.00 |  | 0.00 |  | 159.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 207.10 | 158.02 | 0.00 |  | 0.00 |  | 158.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 190.68 | 141.60 | 0.00 |  | 0.00 |  | 141.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 186.15 | 137.06 | 0.00 |  | 0.00 |  | 137.06 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-irqlocal/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1195851 | 1195851 | 1.00 | 492661207 | 411.98 | 4105510 | 4085579 | 0.0 | 40.1 | false | 0 | 13935 | 156.7 | 1551467 / 2153 / 4 | 0 | 0 |

#### Stalls: r1-irqlocal/pp-epoll-spin-10000, loop 0 on CPU 1

490269504 iterations, longest 156.7 µs; over 5 / 20 / 100 µs: 1551467 / 2153 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1997 | 46592.0 | 156.7 |
| softirq NET_RX | 79 | 1775.3 | 35.6 |
| mostly unexplained (+local timer tick) | 54 | 1342.0 | 35.5 |
| mostly unexplained (+softirq NET_RX) | 20 | 590.2 | 155.8 |
| softirq TIMER | 1 | 28.3 | 28.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.1 | 23.1 |
| loop work | 1 | 20.0 | 20.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 156.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 155.8 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 4, irq enp39s0-Tx-Rx-3 1, irq enp39s0-Tx-Rx-6 1 |
| 3 | 146.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 130.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 65.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 43.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 41.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 40.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 39.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 38.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 2389386 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 2344704 |
| IWI IRQ work interrupts | 18521 |
| LOC Local timer interrupts | 6286 |
| CAL Function call interrupts | 999 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 289 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 226 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 147 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 140 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 138 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 130 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 58 |

#### Slowest round trips: r1-irqlocal/pp-epoll-spin-10000 (median 50.68)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 335.65 | 284.97 | 0.00 |  | 0.00 |  | 284.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 213.84 | 163.16 | 130.31 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 32.84 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 185.82 | 135.14 | 146.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 184.08 | 133.40 | 0.00 |  | 166.12 | softirq TIMER | 0.00 | server: softirq TIMER |
| 5 | 175.13 | 124.45 | 155.83 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 6 | 172.72 | 122.04 | 0.00 |  | 0.00 |  | 122.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 169.50 | 118.82 | 0.00 |  | 0.00 |  | 118.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 167.25 | 116.57 | 0.00 |  | 0.00 |  | 116.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 167.24 | 116.56 | 0.00 |  | 0.00 |  | 116.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 161.36 | 110.68 | 0.00 |  | 0.00 |  | 110.68 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-irqlocal/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1237499 | 1237499 | 1.00 | 96405756 | 77.90 | 1606763 | 1586138 | 98.7 | 71.5 | true | 0 | 9318 | 156.0 | 518873 / 873 / 2 | 0 | 0 |

#### Stalls: r1-irqlocal/pp-recv-busy-0, loop 0 on CPU 1

95168256 iterations, longest 156.0 µs; over 5 / 20 / 100 µs: 518873 / 873 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 716 | 17028.6 | 156.0 |
| mostly unexplained (+local timer tick) | 73 | 1793.6 | 28.8 |
| mostly unexplained (+softirq NET_RX) | 35 | 779.6 | 25.6 |
| softirq NET_RX | 34 | 750.6 | 31.2 |
| mostly unexplained (+irq enp39s0-Tx-Rx-1) | 5 | 226.4 | 138.6 |
| mostly unexplained (+preempted by kworker/1:1) | 8 | 175.7 | 23.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 25.9 | 25.9 |
| mostly unexplained (+irq enp39s0-Tx-Rx-2) | 1 | 25.9 | 25.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 156.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 138.6 | 0 | mostly unexplained (+irq enp39s0-Tx-Rx-1) | irq enp39s0-Tx-Rx-1 1, irq enp39s0-Tx-Rx-2 1, softirq NET_RX 1 |
| 3 | 97.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 44.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 43.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 41.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 40.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 38.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 37.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 1230536 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 1219386 |
| IWI IRQ work interrupts | 11646 |
| LOC Local timer interrupts | 3295 |
| CAL Function call interrupts | 989 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 85 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 85 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 82 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 69 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 60 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 57 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 30 |

#### Slowest round trips: r1-irqlocal/pp-recv-busy-0 (median 48.46)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 206.16 | 157.70 | 0.00 |  | 0.00 |  | 157.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 202.43 | 153.97 | 155.99 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 180.55 | 132.09 | 138.57 | mostly unexplained (+irq enp39s0-Tx-Rx-1) | 0.00 |  | 0.00 | client: mostly unexplained (+irq enp39s0-Tx-Rx-1) |
| 4 | 168.71 | 120.25 | 0.00 |  | 28.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 92.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 168.51 | 120.05 | 0.00 |  | 0.00 |  | 120.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 139.65 | 91.19 | 0.00 |  | 0.00 |  | 91.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 134.37 | 85.91 | 0.00 |  | 0.00 |  | 85.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 129.78 | 81.32 | 0.00 |  | 0.00 |  | 81.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 129.17 | 80.72 | 0.00 |  | 0.00 |  | 80.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 129.09 | 80.63 | 96.93 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-irqlocal/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1184842 | 1184842 | 1.00 | 202434442 | 170.85 | 1686954 | 1677080 | 99.4 | 69.6 | true | 0 | 14319 | 160.7 | 1189579 / 3859 / 4 | 0 | 0 |

#### Stalls: r1-irqlocal/pp-recv-busy-10000, loop 0 on CPU 1

201249600 iterations, longest 160.7 µs; over 5 / 20 / 100 µs: 1189579 / 3859 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 3551 | 80638.7 | 160.7 |
| mostly unexplained (+softirq NET_RX) | 76 | 1862.3 | 155.5 |
| mostly unexplained (+local timer tick) | 75 | 1759.1 | 33.1 |
| softirq NET_RX | 74 | 1699.0 | 38.4 |
| mostly unexplained (+preempted by kworker/1:1) | 50 | 1110.7 | 29.0 |
| mostly unexplained (+irq enp39s0-Tx-Rx-5) | 30 | 661.3 | 27.6 |
| mostly unexplained (+irq enp39s0-Tx-Rx-4) | 2 | 43.3 | 22.8 |
| loop work | 1 | 20.0 | 20.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 160.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 155.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 3, irq enp39s0-Tx-Rx-5 1 |
| 3 | 137.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 128.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 45.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 40.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 38.4 | 0 | softirq NET_RX | softirq NET_RX 31, irq enp39s0-Tx-Rx-3 1, irq enp39s0-Tx-Rx-4 1, irq enp39s0-Tx-Rx-5 1 |
| 8 | 38.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 37.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 2369731 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 2317174 |
| IWI IRQ work interrupts | 18847 |
| LOC Local timer interrupts | 6035 |
| CAL Function call interrupts | 999 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 162 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 147 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 137 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 136 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 129 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 116 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 90 |

#### Slowest round trips: r1-irqlocal/pp-recv-busy-10000 (median 46.99)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 359.49 | 312.50 | 0.00 |  | 0.00 |  | 312.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 357.11 | 310.12 | 0.00 |  | 0.00 |  | 310.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 356.11 | 309.12 | 0.00 |  | 0.00 |  | 309.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 351.36 | 304.36 | 0.00 |  | 0.00 |  | 304.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 350.38 | 303.39 | 0.00 |  | 0.00 |  | 303.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 349.94 | 302.95 | 0.00 |  | 0.00 |  | 302.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 349.32 | 302.33 | 0.00 |  | 0.00 |  | 302.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 348.61 | 301.62 | 0.00 |  | 0.00 |  | 301.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 348.40 | 301.41 | 0.00 |  | 0.00 |  | 301.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 348.09 | 301.10 | 0.00 |  | 0.00 |  | 301.10 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-irqlocal/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1138606 | 1138606 | 1.00 | 3415819 | 3.00 | 56930 | 35978032 | 0.0 | 16.6 | false | 0 | 12444 | 152.0 | 1307159 / 3522 / 2 | 0 | 0 |

#### Stalls: r1-irqlocal/pp-uring-spin-0, loop 0 on CPU 1

2158681920 iterations, longest 152.0 µs; over 5 / 20 / 100 µs: 1307159 / 3522 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 3085 | 70652.6 | 152.0 |
| softirq NET_RX | 259 | 5755.7 | 36.1 |
| mostly unexplained (+local timer tick) | 103 | 2495.5 | 33.4 |
| mostly unexplained (+softirq NET_RX) | 72 | 1727.2 | 142.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.9 | 28.9 |
| mostly unexplained (+softirq TIMER) | 1 | 22.1 | 22.1 |
| mostly unexplained (+irq enp39s0-Tx-Rx-7) | 1 | 21.1 | 21.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 152.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 142.1 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 4, irq enp39s0-Tx-Rx-3 2, irq enp39s0-Tx-Rx-7 1 |
| 3 | 40.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 40.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 39.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 38.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 38.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 37.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 37.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 1995117 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 1225736 |
| IWI IRQ work interrupts | 15479 |
| LOC Local timer interrupts | 4574 |
| CAL Function call interrupts | 992 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 120 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 84 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 81 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 70 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 59 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 5 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 1 |

#### Slowest round trips: r1-irqlocal/pp-uring-spin-0 (median 48.55)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 201.62 | 153.08 | 0.00 |  | 0.00 |  | 153.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 190.12 | 141.57 | 0.00 |  | 0.00 |  | 141.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 178.16 | 129.61 | 142.14 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 4 | 169.70 | 121.15 | 152.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 157.04 | 108.49 | 0.00 |  | 0.00 |  | 108.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 152.28 | 103.73 | 0.00 |  | 0.00 |  | 103.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 151.22 | 102.67 | 0.00 |  | 0.00 |  | 102.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 149.92 | 101.38 | 0.00 |  | 0.00 |  | 101.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 147.69 | 99.14 | 0.00 |  | 0.00 |  | 99.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 147.55 | 99.00 | 0.00 |  | 0.00 |  | 99.00 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-irqlocal/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1167605 | 1167605 | 1.00 | 3502816 | 3.00 | 29190 | 34580497 | 0.0 | 9.5 | false | 0 | 12161 | 156.9 | 1379529 / 2664 / 4 | 0 | 0 |

#### Stalls: r1-irqlocal/pp-uring-spin-10000, loop 0 on CPU 1

4149659584 iterations, longest 156.9 µs; over 5 / 20 / 100 µs: 1379529 / 2664 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2414 | 55697.0 | 153.5 |
| softirq NET_RX | 128 | 2813.5 | 32.8 |
| mostly unexplained (+local timer tick) | 85 | 2060.8 | 34.5 |
| mostly unexplained (+softirq NET_RX) | 33 | 870.7 | 156.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 46.3 | 24.9 |
| softirq TIMER | 1 | 25.3 | 25.3 |
| mostly unexplained (+preempted by kworker/1:1) | 1 | 22.1 | 22.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 156.9 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 3, irq enp39s0-Tx-Rx-7 1 |
| 2 | 153.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 137.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 131.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 65.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 47.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 44.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 43.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 42.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 41.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 2322659 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 2152457 |
| IWI IRQ work interrupts | 16462 |
| LOC Local timer interrupts | 6042 |
| CAL Function call interrupts | 1002 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 249 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 187 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 167 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 143 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 130 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 113 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 112 |

#### Slowest round trips: r1-irqlocal/pp-uring-spin-10000 (median 57.29)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 379.99 | 322.70 | 0.00 |  | 0.00 |  | 322.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 360.72 | 303.43 | 0.00 |  | 0.00 |  | 303.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 360.33 | 303.04 | 0.00 |  | 0.00 |  | 303.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 359.77 | 302.49 | 0.00 |  | 0.00 |  | 302.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 353.92 | 296.63 | 0.00 |  | 0.00 |  | 296.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 352.84 | 295.55 | 0.00 |  | 0.00 |  | 295.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 352.09 | 294.80 | 0.00 |  | 0.00 |  | 294.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 351.67 | 294.38 | 0.00 |  | 0.00 |  | 294.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 350.03 | 292.74 | 0.00 |  | 0.00 |  | 292.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 349.60 | 292.31 | 0.00 |  | 0.00 |  | 292.31 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-6/http-recv-busy-10000, loop 0 on CPU 1

1735886950 iterations, longest 151.2 µs; over 5 / 20 / 100 µs: 49452 / 770 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 494 | 13769.6 | 48.1 |
| loop work | 89 | 2655.5 | 39.0 |
| mostly unexplained (+softirq NET_RX) | 81 | 2269.8 | 58.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 46 | 1556.2 | 151.2 |
| softirq NET_RX | 47 | 1284.6 | 44.4 |
| mostly unexplained (+local timer tick) | 9 | 218.3 | 34.8 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 115.2 | 36.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 151.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 126.5 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 125.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 86.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 85.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 59.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 58.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 54.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 49.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 18, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 48.1 | 2 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 1225 |
| IWI IRQ work interrupts | 1158 |
| CAL Function call interrupts | 999 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-6/http-recv-busy-10000 (median 46.19)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 510.88 | 464.69 | 0.00 |  | 0.00 |  | 464.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 306.29 | 260.09 | 0.00 |  | 0.00 |  | 260.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 267.18 | 220.98 | 0.00 |  | 0.00 |  | 220.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 235.87 | 189.68 | 0.00 |  | 0.00 |  | 189.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 231.49 | 185.30 | 0.00 |  | 0.00 |  | 185.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 216.73 | 170.54 | 0.00 |  | 0.00 |  | 170.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 216.58 | 170.38 | 0.00 |  | 0.00 |  | 170.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 211.41 | 165.21 | 0.00 |  | 0.00 |  | 165.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 211.27 | 165.07 | 0.00 |  | 0.00 |  | 165.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 203.81 | 157.62 | 0.00 |  | 0.00 |  | 157.62 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-6/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=628223 clocksource=tsc

### Loop counters (r1-base-6/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002700 | 701701 | 4.28 | 108666688 | 36.19 | 1811111 | 1811111 | 99.4 | 65.3 | true | 0 | 585 | 669.6 | 76348 / 6118 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672604800 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (106215 of 2499950 in 1-msg reads) | 106215 | 2.46 | 2.26 | 3.14 | 4.48 | 9.18 | 28.06 | 36.05 |

#### Stalls: r1-base-6/md-recv-busy, loop 0 on CPU 1

108666688 iterations, longest 669.6 µs; over 5 / 20 / 100 µs: 76348 / 6118 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5875 | 158529.5 | 669.6 |
| mostly unexplained (+preempted by kworker/1:1) | 87 | 2362.2 | 47.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 78 | 1922.6 | 150.4 |
| softirq NET_RX | 65 | 1768.9 | 39.7 |
| mostly unexplained (+softirq NET_RX) | 11 | 318.9 | 36.0 |
| softirq TIMER | 1 | 246.5 | 246.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.0 | 26.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 669.6 | 2700 | loop work (2700 items) |  |
| 2 | 246.5 | 0 | softirq TIMER | softirq TIMER 235, local timer tick 6, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 3 | 150.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 65.1 | 9 | loop work (9 items) | softirq NET_RX 19, preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 62.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 61.8 | 18 | loop work (18 items) | softirq NET_RX 16, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 61.0 | 11 | loop work (11 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 60.2 | 16 | loop work (16 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 58.0 | 7 | loop work (7 items) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 57.2 | 8 | loop work (8 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 592 |
| LOC Local timer interrupts | 589 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-6/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-6/noise, loop 0 on CPU 1

9040560128 iterations, longest 3813.3 µs; over 5 / 20 / 100 µs: 86 / 6 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+softirq NET_RX) | 1 | 3813.3 | 3813.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 72 | 1074.1 | 147.2 |
| local timer tick | 3 | 45.2 | 19.4 |
| mostly unexplained (+local timer tick) | 5 | 43.8 | 12.6 |
| mostly unexplained (+call function single (IPI/vector)) | 4 | 24.9 | 6.4 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 6.4 | 6.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 3813.3 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 52, irq enp39s0-Tx-Rx-7 2, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 147.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 122.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 113.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 77.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 36.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 19.4 | 0 | local timer tick | local timer tick 11, timer tick_nohz_handler 9, softirq TIMER 1 |
| 8 | 17.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 16.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 16.3 | 0 | local timer tick | local timer tick 9, timer tick_nohz_handler 5, softirq TIMER 1, vector_free_moved 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| LOC Local timer interrupts | 9 |
| IWI IRQ work interrupts | 2 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 1 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 1 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 1 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 1 |

### Loop counters (r1-base-6/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1119682 | 1119682 | 1.00 | 282189061 | 252.03 | 4703151 | 4665828 | 0.0 | 40.4 | false | 0 | 0 | 145.5 | 35266 / 19 / 2 | 0 | 0 |

#### Stalls: r1-base-6/pp-epoll-spin-0, loop 0 on CPU 1

279949696 iterations, longest 145.5 µs; over 5 / 20 / 100 µs: 35266 / 19 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 18 | 691.4 | 145.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.7 | 20.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 145.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 135.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 54.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 33.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 28.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 27.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 26.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 25.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 25.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 22.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 5 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-6/pp-epoll-spin-0 (median 49.41)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 490.06 | 440.65 | 0.00 |  | 0.00 |  | 440.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 460.64 | 411.23 | 0.00 |  | 0.00 |  | 411.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 432.25 | 382.85 | 0.00 |  | 0.00 |  | 382.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 416.92 | 367.51 | 0.00 |  | 0.00 |  | 367.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 414.12 | 364.71 | 0.00 |  | 0.00 |  | 364.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 410.02 | 360.61 | 0.00 |  | 0.00 |  | 360.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 409.24 | 359.84 | 0.00 |  | 0.00 |  | 359.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 408.66 | 359.25 | 0.00 |  | 0.00 |  | 359.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 408.64 | 359.23 | 0.00 |  | 0.00 |  | 359.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 406.74 | 357.33 | 0.00 |  | 0.00 |  | 357.33 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198140 | 1198140 | 1.00 | 553618040 | 462.06 | 4613484 | 4593515 | 0.0 | 37.1 | false | 0 | 0 | 242.5 | 31840 / 30 / 5 | 0 | 0 |

#### Stalls: r1-base-6/pp-epoll-spin-10000, loop 0 on CPU 1

551221760 iterations, longest 242.5 µs; over 5 / 20 / 100 µs: 31840 / 30 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 28 | 1190.6 | 155.1 |
| softirq TIMER | 1 | 242.5 | 242.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 24.9 | 24.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 242.5 | 0 | softirq TIMER | softirq TIMER 215, local timer tick 14, timer tick_nohz_handler 8 |
| 2 | 155.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 139.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 127.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 116.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 69.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 58.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 33.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 31.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 28.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-6/pp-epoll-spin-10000 (median 50.33)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1011.40 | 961.07 | 0.00 |  | 0.00 |  | 961.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 333.27 | 282.94 | 0.00 |  | 0.00 |  | 282.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 265.37 | 215.04 | 0.00 |  | 0.00 |  | 215.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 253.32 | 202.99 | 0.00 |  | 0.00 |  | 202.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 243.56 | 193.23 | 0.00 |  | 0.00 |  | 193.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 206.74 | 156.41 | 0.00 |  | 0.00 |  | 156.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 200.06 | 149.72 | 0.00 |  | 0.00 |  | 149.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 198.85 | 148.52 | 116.30 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 32.22 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 197.45 | 147.12 | 0.00 |  | 0.00 |  | 147.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 194.25 | 143.92 | 0.00 |  | 0.00 |  | 143.92 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1324342 | 1324342 | 1.00 | 106280631 | 80.25 | 1771344 | 1749271 | 98.7 | 71.6 | true | 0 | 2657 | 147.9 | 43425 / 508 / 2 | 0 | 0 |

#### Stalls: r1-base-6/pp-recv-busy-0, loop 0 on CPU 1

104956288 iterations, longest 147.9 µs; over 5 / 20 / 100 µs: 43425 / 508 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 338 | 7656.2 | 38.5 |
| softirq NET_RX | 75 | 1634.4 | 33.7 |
| mostly unexplained (+softirq NET_RX) | 69 | 1566.7 | 29.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 14 | 735.3 | 147.9 |
| mostly unexplained (+local timer tick) | 9 | 213.3 | 29.2 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 55.5 | 29.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 22.1 | 22.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 147.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 110.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 74.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 64.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 58.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 51.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 41.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 38.5 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 35.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 33.7 | 0 | softirq NET_RX | softirq NET_RX 20, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2706 |
| IWI IRQ work interrupts | 2663 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-6/pp-recv-busy-0 (median 43.96)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 226.75 | 182.79 | 0.00 |  | 0.00 |  | 182.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 206.31 | 162.35 | 0.00 |  | 0.00 |  | 162.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 184.12 | 140.16 | 147.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 183.78 | 139.82 | 0.00 |  | 0.00 |  | 139.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 176.57 | 132.61 | 0.00 |  | 0.00 |  | 132.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 163.29 | 119.32 | 0.00 |  | 0.00 |  | 119.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 162.21 | 118.25 | 0.00 |  | 0.00 |  | 118.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 161.85 | 117.89 | 0.00 |  | 0.00 |  | 117.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 161.69 | 117.72 | 0.00 |  | 0.00 |  | 117.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 157.65 | 113.69 | 0.00 |  | 0.00 |  | 113.69 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196900 | 1196900 | 1.00 | 226348516 | 189.11 | 1886238 | 1876263 | 99.5 | 68.3 | true | 0 | 4106 | 145.9 | 44727 / 234 / 4 | 0 | 0 |

#### Stalls: r1-base-6/pp-recv-busy-10000, loop 0 on CPU 1

225151616 iterations, longest 145.9 µs; over 5 / 20 / 100 µs: 44727 / 234 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 153 | 3441.5 | 32.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 15 | 829.0 | 145.9 |
| mostly unexplained (+softirq NET_RX) | 28 | 650.0 | 30.4 |
| softirq NET_RX | 24 | 526.9 | 28.2 |
| mostly unexplained (+local timer tick) | 10 | 222.1 | 28.5 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 53.2 | 26.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 22.8 | 22.8 |
| loop work | 1 | 20.0 | 20.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 145.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 141.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 131.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 125.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 39.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 32.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 32.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 30.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 29.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 9, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 28.5 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, preempted by kworker/1:1 4, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4164 |
| IWI IRQ work interrupts | 4114 |
| CAL Function call interrupts | 1001 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-6/pp-recv-busy-10000 (median 46.00)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 160.35 | 114.34 | 0.00 |  | 0.00 |  | 114.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 155.97 | 109.97 | 125.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 150.45 | 104.45 | 0.00 |  | 0.00 |  | 104.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 149.92 | 103.91 | 0.00 |  | 0.00 |  | 103.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 147.41 | 101.41 | 0.00 |  | 0.00 |  | 101.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 143.93 | 97.93 | 0.00 |  | 0.00 |  | 97.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 143.37 | 97.36 | 0.00 |  | 0.00 |  | 97.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 142.79 | 96.79 | 0.00 |  | 152.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 140.37 | 94.37 | 0.00 |  | 0.00 |  | 94.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 138.91 | 92.91 | 0.00 |  | 0.00 |  | 92.91 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1169412 | 1169412 | 1.00 | 3508237 | 3.00 | 58471 | 44658269 | 0.0 | 9.7 | false | 0 | 0 | 142.9 | 40604 / 12 / 2 | 0 | 0 |

#### Stalls: r1-base-6/pp-uring-spin-0, loop 0 on CPU 1

2679496128 iterations, longest 142.9 µs; over 5 / 20 / 100 µs: 40604 / 12 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 10 | 456.2 | 142.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 31.0 | 31.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.8 | 23.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 142.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 126.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 32.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 31.0 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 27.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 23.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 21.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 21.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 21.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 14 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-6/pp-uring-spin-0 (median 48.02)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 474.38 | 426.36 | 0.00 |  | 0.00 |  | 426.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 440.01 | 391.99 | 0.00 |  | 0.00 |  | 391.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 403.62 | 355.60 | 0.00 |  | 0.00 |  | 355.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 392.62 | 344.60 | 0.00 |  | 0.00 |  | 344.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 325.93 | 277.91 | 0.00 |  | 0.00 |  | 277.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 280.07 | 232.05 | 0.00 |  | 0.00 |  | 232.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 276.50 | 228.48 | 0.00 |  | 224.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 4.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 213.30 | 165.28 | 0.00 |  | 0.00 |  | 165.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 203.01 | 154.99 | 142.85 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 12.13 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 193.77 | 145.75 | 0.00 |  | 0.00 |  | 145.75 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1173578 | 1173578 | 1.00 | 3520735 | 3.00 | 29339 | 35943049 | 0.0 | 5.5 | false | 0 | 0 | 393.6 | 30479 / 20 / 5 | 0 | 0 |

#### Stalls: r1-base-6/pp-uring-spin-10000, loop 0 on CPU 1

4313165824 iterations, longest 393.6 µs; over 5 / 20 / 100 µs: 30479 / 20 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 18 | 1000.9 | 158.0 |
| softirq TIMER | 1 | 393.6 | 393.6 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 28.5 | 28.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 393.6 | 0 | softirq TIMER | softirq TIMER 365, local timer tick 17, timer tick_nohz_handler 9, irq work (IPI/vector) 1 |
| 2 | 158.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 138.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 125.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 112.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 79.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 60.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 52.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 30.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 28.5 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-6/pp-uring-spin-10000 (median 50.10)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 999.60 | 949.49 | 0.00 |  | 0.00 |  | 949.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 524.55 | 474.45 | 0.00 |  | 0.00 |  | 474.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 477.08 | 426.98 | 393.62 | softirq TIMER | 0.00 |  | 33.36 | client: softirq TIMER |
| 4 | 475.36 | 425.26 | 0.00 |  | 0.00 |  | 425.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 458.36 | 408.26 | 0.00 |  | 0.00 |  | 408.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 456.90 | 406.80 | 0.00 |  | 0.00 |  | 406.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 456.57 | 406.47 | 0.00 |  | 0.00 |  | 406.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 450.90 | 400.80 | 0.00 |  | 0.00 |  | 400.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 447.32 | 397.22 | 0.00 |  | 0.00 |  | 397.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 445.42 | 395.31 | 0.00 |  | 0.00 |  | 395.31 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1233425 | 1233425 | 1.00 | 1 | 0.00 | 0 | 29161149 | 0.0 | 0.0 | true | 0 | 0 | 143.0 | 75 / 5 / 2 | 0 | 0 |

#### Stalls: r1-base-6/pp-uring-sqpoll-0, loop 0 on CPU 1

1749668928 iterations, longest 143.0 µs; over 5 / 20 / 100 µs: 75 / 5 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 304.5 | 143.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 115.3 | 93.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 143.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 135.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 93.4 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 26.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 21.9 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 5 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-6/pp-uring-sqpoll-0 (median 47.60)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 332.90 | 285.29 | 0.00 |  | 0.00 |  | 285.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 199.66 | 152.05 | 0.00 |  | 0.00 |  | 152.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 195.96 | 148.35 | 0.00 |  | 0.00 |  | 148.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 185.09 | 137.48 | 0.00 |  | 0.00 |  | 137.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 184.39 | 136.78 | 0.00 |  | 0.00 |  | 136.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 183.73 | 136.13 | 0.00 |  | 0.00 |  | 136.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 180.81 | 133.20 | 0.00 |  | 0.00 |  | 133.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 179.05 | 131.44 | 0.00 |  | 0.00 |  | 131.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 177.13 | 129.53 | 0.00 |  | 0.00 |  | 129.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 177.01 | 129.41 | 0.00 |  | 0.00 |  | 129.41 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1183138 | 1183138 | 1.00 | 1 | 0.00 | 0 | 24783085 | 0.0 | 0.0 | true | 0 | 0 | 156.5 | 136 / 8 / 4 | 0 | 0 |

#### Stalls: r1-base-6/pp-uring-sqpoll-10000, loop 0 on CPU 1

2973970240 iterations, longest 156.5 µs; over 5 / 20 / 100 µs: 136 / 8 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 686.5 | 156.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 156.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 132.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 130.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 116.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 54.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 53.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 21.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 20.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1009 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-base-6/pp-uring-sqpoll-10000 (median 46.43)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 993.19 | 946.75 | 0.00 |  | 0.00 |  | 946.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 364.65 | 318.21 | 0.00 |  | 0.00 |  | 318.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 358.79 | 312.36 | 0.00 |  | 0.00 |  | 312.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 358.47 | 312.03 | 0.00 |  | 0.00 |  | 312.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 355.44 | 309.01 | 0.00 |  | 0.00 |  | 309.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 353.93 | 307.50 | 0.00 |  | 0.00 |  | 307.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 351.61 | 305.18 | 0.00 |  | 0.00 |  | 305.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 349.14 | 302.71 | 0.00 |  | 0.00 |  | 302.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 348.88 | 302.44 | 0.00 |  | 0.00 |  | 302.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 347.26 | 300.83 | 0.00 |  | 0.00 |  | 300.83 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-rt/http-recv-busy-10000, loop 0 on CPU 1

1855598246 iterations, longest 254.7 µs; over 5 / 20 / 100 µs: 131456 / 186 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 93 | 2298.7 | 49.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 54 | 1308.2 | 116.4 |
| preempted by kworker/1:1 | 17 | 473.7 | 33.6 |
| mostly unexplained (+softirq NET_RX) | 16 | 354.8 | 26.7 |
| softirq TIMER | 1 | 254.7 | 254.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 65.7 | 34.7 |
| mostly unexplained (+local timer tick) | 1 | 61.4 | 61.4 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 35.6 | 35.6 |
| loop work | 1 | 22.9 | 22.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 254.7 | 1 | softirq TIMER | softirq TIMER 239, local timer tick 8, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 2 | 116.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 61.4 | 1 | mostly unexplained (+local timer tick) | local timer tick 3, timer tick_nohz_handler 1 |
| 4 | 49.2 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 19, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 36.4 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 15, local timer tick 4, irq work (IPI/vector) 1, reschedule (IPI/vector) 1 |
| 6 | 36.2 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, local timer tick 4, irq work (IPI/vector) 1, reschedule (IPI/vector) 1 |
| 7 | 35.6 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 34.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 33.8 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 9, workqueue [ena] 1 |
| 10 | 33.6 | 0 | preempted by kworker/1:1 | preempted by kworker/1:1 17, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 65994 |
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 63 |
| RES Rescheduling interrupts | 58 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-rt/http-recv-busy-10000 (median 45.42)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 231.15 | 185.73 | 0.00 |  | 232.56 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 210.07 | 164.65 | 0.00 |  | 0.00 |  | 164.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 205.27 | 159.85 | 0.00 |  | 0.00 |  | 159.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 202.45 | 157.03 | 0.00 |  | 0.00 |  | 157.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 202.43 | 157.01 | 0.00 |  | 0.00 |  | 157.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 195.34 | 149.91 | 0.00 |  | 0.00 |  | 149.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 193.24 | 147.81 | 0.00 |  | 0.00 |  | 147.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 192.57 | 147.15 | 0.00 |  | 0.00 |  | 147.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 191.72 | 146.30 | 0.00 |  | 0.00 |  | 146.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 186.97 | 141.55 | 0.00 |  | 0.00 |  | 141.55 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-rt/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=599022 clocksource=tsc

### Loop counters (r1-rt/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002750 | 671031 | 4.47 | 108045248 | 35.98 | 1800754 | 1800754 | 99.4 | 65.7 | true | 0 | 399 | 579.6 | 143439 / 5973 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (100334 of 2500000 in 1-msg reads) | 100334 | 2.74 | 2.48 | 3.53 | 7.25 | 14.95 | 20.13 | 23.46 |

#### Stalls: r1-rt/md-recv-busy, loop 0 on CPU 1

108045248 iterations, longest 579.6 µs; over 5 / 20 / 100 µs: 143439 / 5973 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5802 | 170031.6 | 579.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 108 | 2518.6 | 58.1 |
| preempted by kworker/1:1 | 23 | 600.6 | 30.9 |
| softirq NET_RX | 16 | 441.2 | 36.3 |
| mostly unexplained (+preempted by kworker/1:1) | 13 | 323.9 | 40.9 |
| mostly unexplained (+softirq NET_RX) | 7 | 160.9 | 27.6 |
| mostly unexplained (+local timer tick) | 1 | 123.5 | 123.5 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 52.9 | 32.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.9 | 20.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 579.6 | 2750 | loop work (2750 items) |  |
| 2 | 123.5 | 0 | mostly unexplained (+local timer tick) | local timer tick 3, timer tick_nohz_handler 1 |
| 3 | 58.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 54.0 | 25 | loop work (25 items) | softirq NET_RX 9 |
| 5 | 52.0 | 17 | loop work (17 items) | softirq NET_RX 11, preempted by kworker/1:1 6, workqueue [ena] 1 |
| 6 | 51.8 | 26 | loop work (26 items) | softirq NET_RX 2 |
| 7 | 48.1 | 18 | loop work (18 items) | preempted by kworker/1:1 6, workqueue [ena] 1 |
| 8 | 47.9 | 27 | loop work (27 items) |  |
| 9 | 47.6 | 3 | loop work (3 items) | irq work (IPI/vector) 2, ipi_send_cpu 0 |
| 10 | 46.8 | 29 | loop work (29 items) | softirq NET_RX 3 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 32846 |
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 41 |
| RES Rescheduling interrupts | 30 |
| TLB TLB shootdowns | 1 |

## r1-rt/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-rt/noise, loop 0 on CPU 1

9044877312 iterations, longest 53.1 µs; over 5 / 20 / 100 µs: 80 / 4 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 74 | 748.3 | 53.1 |
| mostly unexplained (+call function (IPI/vector)) | 3 | 16.1 | 5.7 |
| mostly unexplained (+local timer tick) | 2 | 14.8 | 9.5 |
| local timer tick | 1 | 10.2 | 10.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 53.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 30.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 26.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 23.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 19.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 16.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 11.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 10.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 10.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| LOC Local timer interrupts | 5 |
| IWI IRQ work interrupts | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-rt/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1326100 | 1326100 | 1.00 | 107386453 | 80.98 | 1789774 | 1767673 | 98.7 | 71.7 | true | 0 | 1415 | 363.9 | 78892 / 48 / 2 | 0 | 0 |

#### Stalls: r1-rt/pp-recv-busy-0, loop 0 on CPU 1

106060352 iterations, longest 363.9 µs; over 5 / 20 / 100 µs: 78892 / 48 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 19 | 556.2 | 42.3 |
| preempted by kworker/1:1 | 15 | 408.7 | 31.0 |
| softirq TIMER | 1 | 363.9 | 363.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 5 | 237.9 | 139.0 |
| mostly unexplained (+local timer tick) | 4 | 92.5 | 29.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 45.6 | 25.0 |
| mostly unexplained (+softirq NET_RX) | 1 | 36.0 | 36.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 23.4 | 23.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 363.9 | 1 | softirq TIMER | softirq TIMER 344, local timer tick 8, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 139.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 42.3 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 20, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 41.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 11, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 39.6 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 19, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 36.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 33.6 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 15, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 33.4 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 14, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 32.7 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 14, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 32.5 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 14, irq work (IPI/vector) 2, reschedule (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 32032 |
| CAL Function call interrupts | 989 |
| IWI IRQ work interrupts | 37 |
| RES Rescheduling interrupts | 32 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-rt/pp-recv-busy-0 (median 43.87)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 184.61 | 140.74 | 138.06 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 2.68 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 181.52 | 137.65 | 0.00 |  | 0.00 |  | 137.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 172.00 | 128.13 | 0.00 |  | 0.00 |  | 128.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 162.75 | 118.88 | 0.00 |  | 26.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 92.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 159.19 | 115.33 | 0.00 |  | 136.44 | softirq TIMER | 0.00 | server: softirq TIMER |
| 6 | 152.94 | 109.07 | 0.00 |  | 0.00 |  | 109.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 147.15 | 103.28 | 0.00 |  | 0.00 |  | 103.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 146.98 | 103.11 | 0.00 |  | 0.00 |  | 103.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 132.89 | 89.02 | 0.00 |  | 0.00 |  | 89.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 131.95 | 88.08 | 0.00 |  | 0.00 |  | 88.08 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-rt/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196829 | 1196829 | 1.00 | 224614557 | 187.67 | 1871788 | 1861814 | 99.5 | 68.3 | true | 0 | 2073 | 58.1 | 110733 / 98 / 0 | 0 | 0 |

#### Stalls: r1-rt/pp-recv-busy-10000, loop 0 on CPU 1

223417728 iterations, longest 58.1 µs; over 5 / 20 / 100 µs: 110733 / 98 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| preempted by kworker/1:1 | 42 | 1047.4 | 29.8 |
| mostly unexplained (+preempted by kworker/1:1) | 34 | 908.9 | 44.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 295.7 | 58.1 |
| mostly unexplained (+local timer tick) | 6 | 126.5 | 23.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 115.3 | 36.1 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 59.5 | 31.9 |
| mostly unexplained (+softirq NET_RX) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 58.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 57.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 44.7 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 18, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 38.5 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 13, local timer tick 7, timer dl_task_timer 3, irq work (IPI/vector) 1 |
| 5 | 36.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 35.6 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 11, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 7 | 33.0 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 16, irq work (IPI/vector) 2, workqueue [ena] 2, reschedule (IPI/vector) 1 |
| 8 | 32.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.1 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 14, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 32.1 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 14, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 65071 |
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 66 |
| RES Rescheduling interrupts | 59 |

#### Slowest round trips: r1-rt/pp-recv-busy-10000 (median 43.42)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 287.06 | 243.65 | 0.00 |  | 0.00 |  | 243.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 233.32 | 189.91 | 0.00 |  | 0.00 |  | 189.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 225.15 | 181.73 | 0.00 |  | 225.94 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 205.59 | 162.18 | 0.00 |  | 0.00 |  | 162.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 192.95 | 149.54 | 0.00 |  | 0.00 |  | 149.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 169.96 | 126.54 | 0.00 |  | 0.00 |  | 126.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 169.24 | 125.83 | 0.00 |  | 0.00 |  | 125.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 167.43 | 124.02 | 0.00 |  | 0.00 |  | 124.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 163.37 | 119.95 | 0.00 |  | 0.00 |  | 119.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 160.19 | 116.77 | 0.00 |  | 0.00 |  | 116.77 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-rt/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1121695 | 1121695 | 1.00 | 3365086 | 3.00 | 56085 | 44670031 | 0.0 | 9.6 | false | 0 | 0 | 75.2 | 83262 / 12 / 0 | 0 | 0 |

#### Stalls: r1-rt/pp-uring-spin-0, loop 0 on CPU 1

2680201856 iterations, longest 75.2 µs; over 5 / 20 / 100 µs: 83262 / 12 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 10 | 328.5 | 75.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 42.0 | 42.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 39.3 | 39.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 75.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 42.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 41.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 39.3 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 36.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 30.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 26.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 25.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 24.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 24.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 16 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-rt/pp-uring-spin-0 (median 48.46)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 380.33 | 331.87 | 0.00 |  | 0.00 |  | 331.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 357.94 | 309.49 | 0.00 |  | 0.00 |  | 309.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 278.00 | 229.54 | 0.00 |  | 0.00 |  | 229.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 252.05 | 203.59 | 0.00 |  | 0.00 |  | 203.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 250.34 | 201.88 | 0.00 |  | 0.00 |  | 201.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 246.31 | 197.85 | 0.00 |  | 67.99 | unexplained: no kernel event (hypervisor or in-kernel time) | 129.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 227.18 | 178.72 | 0.00 |  | 0.00 |  | 178.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 218.78 | 170.32 | 0.00 |  | 0.00 |  | 170.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 218.37 | 169.92 | 0.00 |  | 0.00 |  | 169.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 207.41 | 158.96 | 0.00 |  | 0.00 |  | 158.96 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-rt/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1117567 | 1117567 | 1.00 | 3352701 | 3.00 | 27939 | 37373807 | 0.0 | 5.2 | false | 0 | 0 | 131.3 | 43627 / 18 / 2 | 0 | 0 |

#### Stalls: r1-rt/pp-uring-spin-10000, loop 0 on CPU 1

4484856896 iterations, longest 131.3 µs; over 5 / 20 / 100 µs: 43627 / 18 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 16 | 712.4 | 131.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 28.4 | 28.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.7 | 23.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 131.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 122.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 63.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 53.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 48.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 44.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 41.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 32.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 28.4 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 25.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 6 |

#### Slowest round trips: r1-rt/pp-uring-spin-10000 (median 50.92)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 654.04 | 603.12 | 0.00 |  | 0.00 |  | 603.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 620.62 | 569.70 | 0.00 |  | 0.00 |  | 569.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 617.18 | 566.26 | 0.00 |  | 0.00 |  | 566.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 608.18 | 557.26 | 0.00 |  | 0.00 |  | 557.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 605.53 | 554.62 | 0.00 |  | 0.00 |  | 554.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 597.22 | 546.30 | 0.00 |  | 0.00 |  | 546.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 596.20 | 545.28 | 0.00 |  | 0.00 |  | 545.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 595.40 | 544.49 | 0.00 |  | 0.00 |  | 544.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 589.03 | 538.11 | 0.00 |  | 0.00 |  | 538.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 587.73 | 536.81 | 0.00 |  | 0.00 |  | 536.81 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-sibling/http-recv-busy-10000, loop 0 on CPU 1

1429813460 iterations, longest 176.1 µs; over 5 / 20 / 100 µs: 321607 / 2481 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 1528 | 40572.7 | 53.1 |
| mostly unexplained (+softirq NET_RX) | 380 | 9492.4 | 48.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 311 | 7646.9 | 176.1 |
| softirq NET_RX | 249 | 6140.5 | 42.0 |
| mostly unexplained (+irq work (IPI/vector)) | 9 | 228.2 | 32.4 |
| loop work | 4 | 94.5 | 27.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 176.1 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 173.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 146.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 139.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 80.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 66.5 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 66.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 55.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 53.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 48.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 16, preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4394 |
| IWI IRQ work interrupts | 4324 |
| CAL Function call interrupts | 995 |

#### Slowest round trips: r1-sibling/http-recv-busy-10000 (median 47.92)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1054.43 | 1006.52 | 0.00 |  | 0.00 |  | 1006.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 346.80 | 298.89 | 0.00 |  | 0.00 |  | 298.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 299.72 | 251.80 | 0.00 |  | 0.00 |  | 251.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 299.64 | 251.72 | 0.00 |  | 0.00 |  | 251.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 299.16 | 251.24 | 0.00 |  | 0.00 |  | 251.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 296.50 | 248.58 | 0.00 |  | 0.00 |  | 248.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 292.85 | 244.93 | 0.00 |  | 0.00 |  | 244.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 292.62 | 244.70 | 0.00 |  | 0.00 |  | 244.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 290.12 | 242.21 | 0.00 |  | 0.00 |  | 242.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 288.77 | 240.86 | 0.00 |  | 0.00 |  | 240.86 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-sibling/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=473725 clocksource=tsc

### Loop counters (r1-sibling/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3003600 | 537087 | 5.59 | 75350144 | 25.09 | 1255836 | 1255836 | 99.3 | 70.3 | true | 0 | 916 | 863.6 | 202971 / 5839 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672806400 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (57608 of 2499997 in 1-msg reads) | 57608 | 3.36 | 3.03 | 4.38 | 6.01 | 14.87 | 35.02 | 39.82 |

#### Stalls: r1-sibling/md-recv-busy, loop 0 on CPU 1

75350144 iterations, longest 863.6 µs; over 5 / 20 / 100 µs: 202971 / 5839 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5514 | 189109.9 | 863.6 |
| softirq NET_RX | 117 | 3456.4 | 48.2 |
| mostly unexplained (+preempted by kworker/1:1) | 104 | 2800.0 | 44.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 89 | 2677.2 | 171.6 |
| mostly unexplained (+softirq NET_RX) | 12 | 340.0 | 38.4 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 62.4 | 39.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.1 | 21.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 863.6 | 3600 | loop work (3600 items) |  |
| 2 | 171.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 145.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 77.2 | 18 | loop work (18 items) | softirq NET_RX 27, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 77.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 74.9 | 16 | loop work (16 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 69.9 | 17 | loop work (17 items) | softirq NET_RX 26, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 69.9 | 21 | loop work (21 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 68.1 | 18 | loop work (18 items) | softirq NET_RX 20, preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 66.6 | 11 | loop work (11 items) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 926 |
| LOC Local timer interrupts | 918 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-sibling/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-sibling/noise, loop 0 on CPU 1

7486124032 iterations, longest 328.2 µs; over 5 / 20 / 100 µs: 58 / 8 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 56 | 740.3 | 116.7 |
| softirq TIMER | 1 | 328.2 | 328.2 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 6.0 | 6.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 328.2 | 0 | softirq TIMER | softirq TIMER 307, local timer tick 13, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 116.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 50.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 28.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 26.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 25.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 25.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 21.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 18.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 16.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| LOC Local timer interrupts | 3 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-sibling/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1248889 | 1248889 | 1.00 | 73534970 | 58.88 | 1225583 | 1204768 | 98.3 | 75.2 | true | 0 | 3008 | 56.4 | 58051 / 1094 / 0 | 0 | 0 |

#### Stalls: r1-sibling/pp-recv-busy-0, loop 0 on CPU 1

72286080 iterations, longest 56.4 µs; over 5 / 20 / 100 µs: 58051 / 1094 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 700 | 17920.1 | 48.7 |
| softirq NET_RX | 238 | 5769.6 | 40.0 |
| mostly unexplained (+softirq NET_RX) | 128 | 3560.0 | 37.0 |
| mostly unexplained (+local timer tick) | 20 | 527.0 | 36.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 238.2 | 56.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 38.4 | 38.4 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 25.7 | 25.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 56.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 48.7 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 10, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 40.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 40.0 | 1 | softirq NET_RX | softirq NET_RX 28, local timer tick 7, preempted by kworker/1:1 5, timer tick_nohz_handler 4 |
| 5 | 39.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 38.7 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 38.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 38.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 37.8 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 37.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3061 |
| IWI IRQ work interrupts | 3013 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-sibling/pp-recv-busy-0 (median 47.92)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 241.07 | 193.15 | 0.00 |  | 0.00 |  | 193.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 217.09 | 169.16 | 0.00 |  | 0.00 |  | 169.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 185.77 | 137.85 | 0.00 |  | 0.00 |  | 137.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 173.72 | 125.80 | 0.00 |  | 0.00 |  | 125.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 142.35 | 94.43 | 0.00 |  | 27.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 67.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 140.93 | 93.01 | 0.00 |  | 46.66 | unexplained: no kernel event (hypervisor or in-kernel time) | 46.35 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 138.91 | 90.99 | 0.00 |  | 0.00 |  | 90.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 137.51 | 89.59 | 0.00 |  | 0.00 |  | 89.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 136.17 | 88.25 | 0.00 |  | 0.00 |  | 88.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 134.82 | 86.89 | 0.00 |  | 0.00 |  | 86.89 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-sibling/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1188908 | 1188908 | 1.00 | 152812012 | 128.53 | 1273433 | 1263526 | 99.2 | 72.7 | true | 0 | 5293 | 71.5 | 221786 / 2263 / 0 | 0 | 0 |

#### Stalls: r1-sibling/pp-recv-busy-10000, loop 0 on CPU 1

151623104 iterations, longest 71.5 µs; over 5 / 20 / 100 µs: 221786 / 2263 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 1415 | 35413.2 | 41.4 |
| softirq NET_RX | 525 | 12822.8 | 35.7 |
| mostly unexplained (+softirq NET_RX) | 255 | 6584.3 | 39.9 |
| mostly unexplained (+local timer tick) | 39 | 1004.5 | 41.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 20 | 584.0 | 59.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 173.9 | 71.5 |
| mostly unexplained (+irq work (IPI/vector)) | 5 | 126.5 | 36.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 71.5 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 59.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 52.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 42.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 41.4 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 41.3 | 1 | mostly unexplained (+local timer tick) | local timer tick 8, preempted by kworker/1:1 6, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 7 | 41.0 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 39.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/1:1 6, local timer tick 5, irq work (IPI/vector) 1 |
| 9 | 39.4 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 38.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 19, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 5473 |
| IWI IRQ work interrupts | 5300 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-sibling/pp-recv-busy-10000 (median 48.28)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1050.71 | 1002.43 | 0.00 |  | 0.00 |  | 1002.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1004.11 | 955.83 | 0.00 |  | 0.00 |  | 955.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 363.04 | 314.76 | 0.00 |  | 0.00 |  | 314.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 360.12 | 311.84 | 0.00 |  | 0.00 |  | 311.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 356.66 | 308.38 | 0.00 |  | 0.00 |  | 308.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 353.75 | 305.47 | 0.00 |  | 0.00 |  | 305.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 351.99 | 303.71 | 0.00 |  | 0.00 |  | 303.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 348.68 | 300.39 | 0.00 |  | 0.00 |  | 300.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 346.07 | 297.79 | 0.00 |  | 0.00 |  | 297.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 345.64 | 297.36 | 0.00 |  | 0.00 |  | 297.36 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-sibling/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1051732 | 1051732 | 1.00 | 3155197 | 3.00 | 52587 | 32865407 | 0.0 | 10.3 | false | 0 | 0 | 176.2 | 71878 / 21 / 3 | 0 | 0 |

#### Stalls: r1-sibling/pp-uring-spin-0, loop 0 on CPU 1

1971924416 iterations, longest 176.2 µs; over 5 / 20 / 100 µs: 71878 / 21 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 19 | 753.3 | 176.2 |
| softirq TIMER | 1 | 116.9 | 116.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 39.4 | 39.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 176.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 139.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 116.9 | 0 | softirq TIMER | softirq TIMER 88, local timer tick 16, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 4 | 57.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 39.4 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 30.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 26.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 25.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 25.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 25.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-sibling/pp-uring-spin-0 (median 53.57)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 325.31 | 271.74 | 0.00 |  | 0.00 |  | 271.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 239.77 | 186.20 | 0.00 |  | 0.00 |  | 186.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 235.53 | 181.96 | 176.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 5.77 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 218.30 | 164.72 | 0.00 |  | 0.00 |  | 164.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 216.68 | 163.10 | 0.00 |  | 0.00 |  | 163.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 214.98 | 161.41 | 0.00 |  | 0.00 |  | 161.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 214.95 | 161.38 | 0.00 |  | 229.56 | softirq TIMER | 0.00 | server: softirq TIMER |
| 8 | 210.28 | 156.71 | 0.00 |  | 0.00 |  | 156.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 203.68 | 150.10 | 0.00 |  | 0.00 |  | 150.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 203.54 | 149.97 | 0.00 |  | 0.00 |  | 149.97 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-sibling/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199112 | 1199112 | 1.00 | 3597337 | 3.00 | 29978 | 26040817 | 0.0 | 6.8 | false | 0 | 0 | 167.5 | 111199 / 17 / 4 | 0 | 0 |

#### Stalls: r1-sibling/pp-uring-spin-10000, loop 0 on CPU 1

3124897984 iterations, longest 167.5 µs; over 5 / 20 / 100 µs: 111199 / 17 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 15 | 980.9 | 167.5 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 30.7 | 30.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.0 | 28.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 167.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 163.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 138.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 128.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 57.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 56.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 47.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 45.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 33.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 30.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-sibling/pp-uring-spin-10000 (median 51.37)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 331.94 | 280.57 | 0.00 |  | 0.00 |  | 280.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 302.42 | 251.05 | 0.00 |  | 0.00 |  | 251.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 237.48 | 186.11 | 0.00 |  | 0.00 |  | 186.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 218.57 | 167.20 | 0.00 |  | 0.00 |  | 167.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 217.48 | 166.11 | 0.00 |  | 0.00 |  | 166.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 203.16 | 151.80 | 0.00 |  | 0.00 |  | 151.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 201.01 | 149.65 | 167.55 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 197.43 | 146.06 | 0.00 |  | 0.00 |  | 146.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 190.16 | 138.79 | 0.00 |  | 0.00 |  | 138.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 189.51 | 138.14 | 0.00 |  | 0.00 |  | 138.14 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-sibling/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1218734 | 1218734 | 1.00 | 1 | 0.00 | 0 | 26257337 | 0.0 | 0.0 | true | 0 | 0 | 159.9 | 59 / 5 / 2 | 0 | 0 |

#### Stalls: r1-sibling/pp-uring-sqpoll-0, loop 0 on CPU 1

1575440192 iterations, longest 159.9 µs; over 5 / 20 / 100 µs: 59 / 5 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 385.6 | 159.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 31.0 | 31.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 159.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 144.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 55.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 31.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 26.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| LOC Local timer interrupts | 10 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-sibling/pp-uring-sqpoll-0 (median 48.63)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 260.90 | 212.28 | 0.00 |  | 0.00 |  | 212.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 258.38 | 209.75 | 0.00 |  | 0.00 |  | 209.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 238.63 | 190.00 | 0.00 |  | 0.00 |  | 190.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 223.41 | 174.78 | 0.00 |  | 0.00 |  | 174.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 216.93 | 168.30 | 0.00 |  | 0.00 |  | 168.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 206.08 | 157.45 | 0.00 |  | 0.00 |  | 157.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 200.21 | 151.58 | 159.15 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 190.30 | 141.67 | 0.00 |  | 0.00 |  | 141.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 180.61 | 131.98 | 0.00 |  | 0.00 |  | 131.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 179.43 | 130.80 | 0.00 |  | 0.00 |  | 130.80 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-sibling/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199292 | 1199292 | 1.00 | 1 | 0.00 | 0 | 16783651 | 0.0 | 0.0 | true | 0 | 0 | 366.7 | 97 / 7 / 4 | 0 | 0 |

#### Stalls: r1-sibling/pp-uring-sqpoll-10000, loop 0 on CPU 1

2014038080 iterations, longest 366.7 µs; over 5 / 20 / 100 µs: 97 / 7 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 529.9 | 155.0 |
| softirq TIMER | 1 | 366.7 | 366.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 366.7 | 0 | softirq TIMER | softirq TIMER 347, local timer tick 13, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 155.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 137.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 121.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 59.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 31.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 23.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1005 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-sibling/pp-uring-sqpoll-10000 (median 47.41)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 454.83 | 407.42 | 366.74 | softirq TIMER | 0.00 |  | 40.68 | client: softirq TIMER |
| 2 | 255.89 | 208.47 | 0.00 |  | 0.00 |  | 208.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 212.26 | 164.85 | 0.00 |  | 0.00 |  | 164.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 197.41 | 150.00 | 0.00 |  | 134.56 | softirq TIMER | 15.44 | server: softirq TIMER |
| 5 | 195.12 | 147.71 | 0.00 |  | 0.00 |  | 147.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 190.89 | 143.48 | 0.00 |  | 0.00 |  | 143.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 189.23 | 141.82 | 154.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 184.90 | 137.49 | 0.00 |  | 0.00 |  | 137.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 171.49 | 124.08 | 0.00 |  | 0.00 |  | 124.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 162.55 | 115.14 | 0.00 |  | 0.00 |  | 115.14 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-8/http-recv-busy-10000, loop 0 on CPU 1

1844773078 iterations, longest 152.1 µs; over 5 / 20 / 100 µs: 86205 / 1034 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 767 | 19027.3 | 46.5 |
| mostly unexplained (+softirq NET_RX) | 80 | 2111.1 | 43.3 |
| softirq NET_RX | 87 | 2085.3 | 38.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 60 | 1935.6 | 152.1 |
| mostly unexplained (+local timer tick) | 17 | 579.8 | 48.8 |
| mostly unexplained (+irq work (IPI/vector)) | 21 | 476.3 | 27.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 58.1 | 58.1 |
| loop work | 1 | 21.7 | 21.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 152.1 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 143.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 130.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 128.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 61.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 58.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 58.1 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 58.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 53.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 48.8 | 1 | mostly unexplained (+local timer tick) | local timer tick 9, timer tick_nohz_handler 6, preempted by kworker/1:1 5, irq work (IPI/vector) 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2538 |
| IWI IRQ work interrupts | 2523 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-base-8/http-recv-busy-10000 (median 46.10)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 155.22 | 109.12 | 151.65 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 145.74 | 99.63 | 142.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 133.43 | 87.33 | 0.00 |  | 0.00 |  | 87.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 129.78 | 83.68 | 0.00 |  | 0.00 |  | 83.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 125.28 | 79.18 | 0.00 |  | 0.00 |  | 79.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 124.49 | 78.39 | 0.00 |  | 0.00 |  | 78.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 124.24 | 78.14 | 0.00 |  | 0.00 |  | 78.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 123.91 | 77.81 | 0.00 |  | 0.00 |  | 77.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 121.04 | 74.94 | 0.00 |  | 0.00 |  | 74.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 119.24 | 73.13 | 0.00 |  | 0.00 |  | 73.13 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-8/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=393463 clocksource=tsc

### Loop counters (r1-base-8/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002750 | 443995 | 6.76 | 110254592 | 36.72 | 1837577 | 1837577 | 99.6 | 65.1 | true | 0 | 424 | 526.0 | 118918 / 4238 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (53347 of 2500000 in 1-msg reads) | 53347 | 2.52 | 2.35 | 2.79 | 7.83 | 17.31 | 20.82 | 28.75 |

#### Stalls: r1-base-8/md-recv-busy, loop 0 on CPU 1

110254592 iterations, longest 526.0 µs; over 5 / 20 / 100 µs: 118918 / 4238 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4060 | 112652.2 | 526.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 54 | 1661.3 | 151.7 |
| mostly unexplained (+preempted by kworker/1:1) | 59 | 1389.6 | 29.4 |
| softirq NET_RX | 56 | 1389.4 | 29.9 |
| mostly unexplained (+softirq NET_RX) | 6 | 147.6 | 30.7 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 71.4 | 28.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 526.0 | 2750 | loop work (2750 items) |  |
| 2 | 151.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 126.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 64.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 61.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 57.2 | 30 | loop work (30 items) |  |
| 7 | 55.4 | 17 | loop work (17 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 52.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 51.0 | 13 | loop work (13 items) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 49.5 | 10 | loop work (10 items) | preempted by kworker/1:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 439 |
| LOC Local timer interrupts | 431 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-8/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-8/noise, loop 0 on CPU 1

9041371136 iterations, longest 139.9 µs; over 5 / 20 / 100 µs: 61 / 4 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 59 | 906.2 | 139.9 |
| mostly unexplained (+call function (IPI/vector)) | 2 | 13.5 | 7.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 139.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 104.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 97.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 72.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 19.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 16.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 14.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 14.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 13.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 13.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |

### Loop counters (r1-base-8/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1145949 | 1145949 | 1.00 | 281692347 | 245.82 | 4694872 | 4656674 | 0.0 | 40.6 | false | 0 | 0 | 155.9 | 33935 / 16 / 3 | 0 | 0 |

#### Stalls: r1-base-8/pp-epoll-spin-0, loop 0 on CPU 1

279400448 iterations, longest 155.9 µs; over 5 / 20 / 100 µs: 33935 / 16 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 13 | 576.8 | 155.9 |
| softirq TIMER | 1 | 121.1 | 121.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 47.1 | 26.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 155.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 130.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 121.1 | 1 | softirq TIMER | softirq TIMER 96, local timer tick 14, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 4 | 60.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 27.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 27.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 27.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 26.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 22.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 13 |
| IWI IRQ work interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-8/pp-epoll-spin-0 (median 49.05)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 894.63 | 845.59 | 0.00 |  | 28.48 | softirq TIMER | 817.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 439.16 | 390.12 | 0.00 |  | 0.00 |  | 390.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 420.88 | 371.83 | 0.00 |  | 0.00 |  | 371.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 417.96 | 368.91 | 0.00 |  | 0.00 |  | 368.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 403.68 | 354.64 | 0.00 |  | 0.00 |  | 354.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 397.07 | 348.02 | 0.00 |  | 0.00 |  | 348.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 377.97 | 328.93 | 0.00 |  | 0.00 |  | 328.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 368.29 | 319.24 | 0.00 |  | 0.00 |  | 319.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 366.18 | 317.13 | 0.00 |  | 0.00 |  | 317.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 361.67 | 312.62 | 0.00 |  | 0.00 |  | 312.62 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-8/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1197791 | 1197791 | 1.00 | 555133950 | 463.46 | 4626116 | 4606153 | 0.0 | 36.8 | false | 0 | 0 | 150.0 | 7645 / 16 / 4 | 0 | 0 |

#### Stalls: r1-base-8/pp-epoll-spin-10000, loop 0 on CPU 1

552738368 iterations, longest 150.0 µs; over 5 / 20 / 100 µs: 7645 / 16 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 16 | 965.7 | 150.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 150.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 134.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 115.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 115.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 91.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 61.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 51.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 49.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 29.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 28.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| IWI IRQ work interrupts | 4 |

#### Slowest round trips: r1-base-8/pp-epoll-spin-10000 (median 50.64)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 186.72 | 136.08 | 0.00 |  | 0.00 |  | 136.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 183.24 | 132.60 | 134.42 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 178.21 | 127.57 | 0.00 |  | 0.00 |  | 127.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 173.69 | 123.06 | 0.00 |  | 0.00 |  | 123.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 162.07 | 111.43 | 114.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 157.67 | 107.03 | 0.00 |  | 0.00 |  | 107.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 152.91 | 102.28 | 0.00 |  | 0.00 |  | 102.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 152.32 | 101.68 | 0.00 |  | 0.00 |  | 101.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 150.68 | 100.04 | 0.00 |  | 0.00 |  | 100.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 149.03 | 98.40 | 0.00 |  | 0.00 |  | 98.40 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-8/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1275386 | 1275386 | 1.00 | 108158715 | 84.80 | 1802645 | 1781389 | 98.8 | 71.6 | true | 0 | 2413 | 152.0 | 56496 / 498 / 2 | 0 | 0 |

#### Stalls: r1-base-8/pp-recv-busy-0, loop 0 on CPU 1

106883328 iterations, longest 152.0 µs; over 5 / 20 / 100 µs: 56496 / 498 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 313 | 7019.3 | 35.3 |
| mostly unexplained (+softirq NET_RX) | 87 | 1951.1 | 31.2 |
| softirq NET_RX | 75 | 1639.0 | 25.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 545.9 | 152.0 |
| mostly unexplained (+local timer tick) | 7 | 179.2 | 33.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 76.5 | 56.2 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 55.2 | 30.1 |
| softirq TIMER | 1 | 53.8 | 53.8 |
| loop work | 2 | 40.0 | 20.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 152.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 137.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 62.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 56.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 53.8 | 0 | softirq TIMER | softirq TIMER 44, local timer tick 4, timer tick_nohz_handler 1 |
| 6 | 53.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 35.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 34.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 33.1 | 1 | mostly unexplained (+local timer tick) | local timer tick 4, preempted by kworker/1:1 3, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 32.8 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2461 |
| IWI IRQ work interrupts | 2419 |
| CAL Function call interrupts | 986 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-8/pp-recv-busy-0 (median 45.74)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 280.77 | 235.03 | 0.00 |  | 152.42 | unexplained: no kernel event (hypervisor or in-kernel time) | 82.61 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 260.79 | 215.05 | 0.00 |  | 181.08 | softirq TIMER | 33.97 | server: softirq TIMER |
| 3 | 195.20 | 149.47 | 151.74 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 170.99 | 125.25 | 0.00 |  | 0.00 |  | 125.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 166.16 | 120.43 | 136.85 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 159.19 | 113.46 | 0.00 |  | 0.00 |  | 113.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 157.39 | 111.65 | 0.00 |  | 0.00 |  | 111.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 155.55 | 109.81 | 0.00 |  | 0.00 |  | 109.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 152.03 | 106.29 | 0.00 |  | 0.00 |  | 106.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 150.26 | 104.52 | 0.00 |  | 0.00 |  | 104.52 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-8/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1172016 | 1172016 | 1.00 | 225431345 | 192.34 | 1878595 | 1868828 | 99.5 | 68.3 | true | 0 | 4139 | 150.9 | 49611 / 152 / 4 | 0 | 0 |

#### Stalls: r1-base-8/pp-recv-busy-10000, loop 0 on CPU 1

224259328 iterations, longest 150.9 µs; over 5 / 20 / 100 µs: 49611 / 152 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 97 | 2155.7 | 37.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 17 | 819.9 | 147.6 |
| mostly unexplained (+softirq NET_RX) | 21 | 590.1 | 150.9 |
| softirq NET_RX | 11 | 233.1 | 22.6 |
| mostly unexplained (+local timer tick) | 5 | 109.4 | 23.4 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 27.5 | 27.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 150.9 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 147.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 132.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 123.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 63.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 56.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 37.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 35.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 27.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 11, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 27.5 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4203 |
| IWI IRQ work interrupts | 4144 |
| CAL Function call interrupts | 995 |

#### Slowest round trips: r1-base-8/pp-recv-busy-10000 (median 45.87)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 369.23 | 323.36 | 0.00 |  | 0.00 |  | 323.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 368.69 | 322.81 | 0.00 |  | 0.00 |  | 322.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 363.24 | 317.36 | 0.00 |  | 0.00 |  | 317.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 358.35 | 312.48 | 0.00 |  | 0.00 |  | 312.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 357.48 | 311.60 | 0.00 |  | 0.00 |  | 311.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 353.02 | 307.15 | 0.00 |  | 0.00 |  | 307.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 352.66 | 306.79 | 0.00 |  | 0.00 |  | 306.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 352.64 | 306.77 | 0.00 |  | 0.00 |  | 306.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 350.97 | 305.09 | 0.00 |  | 0.00 |  | 305.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 346.80 | 300.93 | 0.00 |  | 0.00 |  | 300.93 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-8/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1146678 | 1146678 | 1.00 | 3440035 | 3.00 | 57334 | 44744665 | 0.0 | 9.6 | false | 0 | 0 | 137.5 | 46168 / 27 / 2 | 0 | 0 |

#### Stalls: r1-base-8/pp-uring-spin-0, loop 0 on CPU 1

2684679872 iterations, longest 137.5 µs; over 5 / 20 / 100 µs: 46168 / 27 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 24 | 916.2 | 137.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 79.4 | 54.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 29.8 | 29.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 137.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 128.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 57.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 54.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 52.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 50.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 48.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 43.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 31.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 29.8 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 7 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-8/pp-uring-spin-0 (median 48.40)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1041.29 | 992.89 | 0.00 |  | 0.00 |  | 992.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 342.10 | 293.69 | 0.00 |  | 0.00 |  | 293.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 291.87 | 243.47 | 0.00 |  | 0.00 |  | 243.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 257.55 | 209.15 | 0.00 |  | 0.00 |  | 209.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 235.21 | 186.81 | 0.00 |  | 204.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 218.07 | 169.66 | 0.00 |  | 0.00 |  | 169.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 217.01 | 168.61 | 0.00 |  | 0.00 |  | 168.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 215.43 | 167.02 | 0.00 |  | 0.00 |  | 167.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 208.99 | 160.59 | 0.00 |  | 0.00 |  | 160.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 207.88 | 159.48 | 0.00 |  | 0.00 |  | 159.48 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-8/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1156868 | 1156868 | 1.00 | 3470605 | 3.00 | 28922 | 37065211 | 0.0 | 5.5 | false | 0 | 0 | 363.1 | 57041 / 18 / 5 | 0 | 0 |

#### Stalls: r1-base-8/pp-uring-spin-10000, loop 0 on CPU 1

4447825280 iterations, longest 363.1 µs; over 5 / 20 / 100 µs: 57041 / 18 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 14 | 898.2 | 147.3 |
| softirq TIMER | 1 | 363.1 | 363.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 90.6 | 69.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 24.1 | 24.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 363.1 | 0 | softirq TIMER | softirq TIMER 335, local timer tick 16, timer tick_nohz_handler 10, irq work (IPI/vector) 1 |
| 2 | 147.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 139.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 127.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 124.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 69.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 58.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 53.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 50.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 48.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-8/pp-uring-spin-10000 (median 51.87)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 478.31 | 426.44 | 0.00 |  | 0.00 |  | 426.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 424.99 | 373.12 | 0.00 |  | 0.00 |  | 373.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 408.81 | 356.95 | 0.00 |  | 0.00 |  | 356.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 405.16 | 353.29 | 0.00 |  | 0.00 |  | 353.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 404.37 | 352.50 | 363.05 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 6 | 403.96 | 352.09 | 0.00 |  | 0.00 |  | 352.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 401.22 | 349.35 | 0.00 |  | 0.00 |  | 349.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 395.80 | 343.93 | 0.00 |  | 0.00 |  | 343.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 392.27 | 340.41 | 0.00 |  | 0.00 |  | 340.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 392.24 | 340.38 | 0.00 |  | 0.00 |  | 340.38 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-8/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1239339 | 1239339 | 1.00 | 1 | 0.00 | 0 | 37529586 | 0.0 | 0.0 | true | 0 | 0 | 145.1 | 61 / 3 / 2 | 0 | 0 |

#### Stalls: r1-base-8/pp-uring-sqpoll-0, loop 0 on CPU 1

2251775168 iterations, longest 145.1 µs; over 5 / 20 / 100 µs: 61 / 3 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 306.2 | 145.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 145.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 136.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 25.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| LOC Local timer interrupts | 14 |
| IWI IRQ work interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-8/pp-uring-sqpoll-0 (median 47.93)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1049.67 | 1001.74 | 0.00 |  | 0.00 |  | 1001.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 326.38 | 278.45 | 0.00 |  | 284.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 248.04 | 200.11 | 0.00 |  | 0.00 |  | 200.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 188.62 | 140.69 | 0.00 |  | 0.00 |  | 140.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 179.36 | 131.43 | 0.00 |  | 0.00 |  | 131.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 171.55 | 123.62 | 135.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 166.63 | 118.70 | 0.00 |  | 0.00 |  | 118.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 164.84 | 116.91 | 0.00 |  | 0.00 |  | 116.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 163.85 | 115.92 | 0.00 |  | 0.00 |  | 115.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 161.44 | 113.51 | 144.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-8/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1184430 | 1184430 | 1.00 | 1 | 0.00 | 0 | 25043443 | 0.0 | 0.0 | true | 0 | 0 | 151.2 | 87 / 7 / 4 | 0 | 0 |

#### Stalls: r1-base-8/pp-uring-sqpoll-10000, loop 0 on CPU 1

3005213184 iterations, longest 151.2 µs; over 5 / 20 / 100 µs: 87 / 7 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 564.5 | 151.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 30.9 | 30.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 151.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 132.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 122.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 113.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 30.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 23.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 20.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1008 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: r1-base-8/pp-uring-sqpoll-10000 (median 46.04)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 433.71 | 387.67 | 0.00 |  | 0.00 |  | 387.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 370.62 | 324.58 | 0.00 |  | 0.00 |  | 324.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 364.75 | 318.72 | 0.00 |  | 0.00 |  | 318.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 358.12 | 312.08 | 0.00 |  | 0.00 |  | 312.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 348.96 | 302.92 | 0.00 |  | 0.00 |  | 302.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 348.58 | 302.54 | 0.00 |  | 0.00 |  | 302.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 346.27 | 300.24 | 0.00 |  | 0.00 |  | 300.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 344.87 | 298.83 | 0.00 |  | 0.00 |  | 298.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 344.18 | 298.14 | 0.00 |  | 0.00 |  | 298.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 343.15 | 297.12 | 0.00 |  | 0.00 |  | 297.12 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-busypoll50/http-recv-busy-10000, loop 0 on CPU 1

1825271587 iterations, longest 157.1 µs; over 5 / 20 / 100 µs: 82912 / 1362 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 920 | 22059.7 | 41.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 157 | 4087.8 | 157.1 |
| softirq NET_RX | 137 | 3180.6 | 31.2 |
| mostly unexplained (+softirq NET_RX) | 113 | 2768.3 | 64.7 |
| mostly unexplained (+irq work (IPI/vector)) | 33 | 807.0 | 35.5 |
| loop work | 1 | 21.8 | 21.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.3 | 20.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 157.1 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 151.1 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 129.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 121.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 64.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 55.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 53.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 41.5 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 40.9 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 40.6 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3902 |
| IWI IRQ work interrupts | 3895 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-busypoll50/http-recv-busy-10000 (median 46.72)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 193.17 | 146.45 | 156.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 187.75 | 141.03 | 150.57 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 147.00 | 100.28 | 0.00 |  | 0.00 |  | 100.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 146.43 | 99.70 | 21.31 | softirq NET_RX | 0.00 |  | 78.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 143.17 | 96.44 | 0.00 |  | 0.00 |  | 96.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 142.28 | 95.56 | 0.00 |  | 0.00 |  | 95.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 139.95 | 93.23 | 0.00 |  | 0.00 |  | 93.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 138.68 | 91.96 | 129.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 138.26 | 91.54 | 0.00 |  | 0.00 |  | 91.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 137.31 | 90.58 | 0.00 |  | 0.00 |  | 90.58 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-busypoll50/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=50 wait_us=0 mlockall=true BusyPollRxPackets during run=574532 clocksource=tsc

### Loop counters (r1-busypoll50/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002747 | 640582 | 4.69 | 108734592 | 36.21 | 1812243 | 1812243 | 99.4 | 65.5 | true | 0 | 700 | 650.8 | 99321 / 6070 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672615328 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (85901 of 2499997 in 1-msg reads) | 85901 | 2.75 | 2.47 | 3.64 | 5.10 | 14.66 | 32.44 | 41.25 |

#### Stalls: r1-busypoll50/md-recv-busy, loop 0 on CPU 1

108734592 iterations, longest 650.8 µs; over 5 / 20 / 100 µs: 99321 / 6070 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5819 | 172665.8 | 650.8 |
| mostly unexplained (+preempted by kworker/1:1) | 104 | 2750.5 | 38.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 80 | 1854.5 | 43.5 |
| softirq NET_RX | 50 | 1342.5 | 35.1 |
| mostly unexplained (+softirq NET_RX) | 16 | 436.2 | 42.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 650.8 | 2731 | loop work (2731 items) |  |
| 2 | 154.2 | 13 | loop work (13 items) |  |
| 3 | 144.3 | 30 | loop work (30 items) |  |
| 4 | 70.5 | 24 | loop work (24 items) | softirq NET_RX 23, preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 69.1 | 20 | loop work (20 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 68.0 | 17 | loop work (17 items) | softirq NET_RX 25, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 59.0 | 16 | loop work (16 items) | softirq NET_RX 1 |
| 8 | 57.7 | 28 | loop work (28 items) |  |
| 9 | 54.8 | 9 | loop work (9 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 53.8 | 27 | loop work (27 items) | softirq NET_RX 8 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 706 |
| LOC Local timer interrupts | 703 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-busypoll50/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-busypoll50/noise, loop 0 on CPU 1

9043800064 iterations, longest 255.0 µs; over 5 / 20 / 100 µs: 87 / 8 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 85 | 1232.9 | 130.8 |
| softirq TIMER | 1 | 255.0 | 255.0 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 6.5 | 6.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 255.0 | 0 | softirq TIMER | softirq TIMER 231, local timer tick 14, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 130.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 116.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 108.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 71.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 51.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 32.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 30.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 17.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 15.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| IWI IRQ work interrupts | 2 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (r1-busypoll50/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1304096 | 1304096 | 1.00 | 107003233 | 82.05 | 1783387 | 1761652 | 98.8 | 71.4 | true | 0 | 2523 | 158.0 | 45369 / 513 / 2 | 0 | 0 |

#### Stalls: r1-busypoll50/pp-recv-busy-0, loop 0 on CPU 1

105699136 iterations, longest 158.0 µs; over 5 / 20 / 100 µs: 45369 / 513 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 353 | 7897.1 | 37.3 |
| mostly unexplained (+softirq NET_RX) | 72 | 1776.5 | 158.0 |
| softirq NET_RX | 62 | 1380.3 | 35.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 11 | 423.4 | 148.1 |
| mostly unexplained (+local timer tick) | 10 | 249.7 | 31.1 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 65.0 | 35.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 51.5 | 31.0 |
| softirq TIMER | 1 | 49.4 | 49.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 158.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 148.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 49.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 49.4 | 0 | softirq TIMER | softirq TIMER 43, local timer tick 2, timer tick_nohz_handler 1 |
| 5 | 37.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 35.4 | 0 | softirq NET_RX | softirq NET_RX 20, preempted by ksoftirqd/1 9, local timer tick 6, timer tick_nohz_handler 3 |
| 7 | 35.2 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 34.5 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 14, workqueue [ena] 9, local timer tick 5, timer tick_nohz_handler 2 |
| 9 | 34.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 33.8 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2610 |
| IWI IRQ work interrupts | 2527 |
| CAL Function call interrupts | 989 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-busypoll50/pp-recv-busy-0 (median 44.70)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1049.93 | 1005.24 | 0.00 |  | 0.00 |  | 1005.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1035.17 | 990.48 | 0.00 |  | 0.00 |  | 990.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 298.36 | 253.66 | 0.00 |  | 0.00 |  | 253.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 298.05 | 253.35 | 0.00 |  | 0.00 |  | 253.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 292.03 | 247.33 | 0.00 |  | 0.00 |  | 247.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 289.59 | 244.89 | 0.00 |  | 0.00 |  | 244.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 289.14 | 244.44 | 0.00 |  | 0.00 |  | 244.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 286.07 | 241.38 | 0.00 |  | 0.00 |  | 241.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 285.92 | 241.22 | 0.00 |  | 0.00 |  | 241.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 282.31 | 237.61 | 0.00 |  | 0.00 |  | 237.61 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-busypoll50/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196861 | 1196861 | 1.00 | 225611197 | 188.50 | 1880093 | 1870119 | 99.5 | 68.2 | true | 0 | 3693 | 148.3 | 67063 / 203 / 4 | 0 | 0 |

#### Stalls: r1-busypoll50/pp-recv-busy-10000, loop 0 on CPU 1

224414336 iterations, longest 148.3 µs; over 5 / 20 / 100 µs: 67063 / 203 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 120 | 2688.4 | 33.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 22 | 1147.2 | 148.3 |
| mostly unexplained (+softirq NET_RX) | 35 | 800.6 | 30.8 |
| softirq NET_RX | 17 | 373.0 | 24.7 |
| mostly unexplained (+local timer tick) | 6 | 156.8 | 31.1 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 78.2 | 28.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 148.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 146.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 125.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 121.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 94.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 56.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 55.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 48.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 36.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 33.3 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3731 |
| IWI IRQ work interrupts | 3701 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-busypoll50/pp-recv-busy-10000 (median 47.58)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 308.13 | 260.55 | 0.00 |  | 0.00 |  | 260.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 213.94 | 166.36 | 0.00 |  | 0.00 |  | 166.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 193.85 | 146.27 | 0.00 |  | 0.00 |  | 146.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 178.87 | 131.29 | 0.00 |  | 0.00 |  | 131.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 169.76 | 122.18 | 148.03 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 165.20 | 117.61 | 0.00 |  | 27.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 90.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 157.40 | 109.82 | 0.00 |  | 0.00 |  | 109.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 150.56 | 102.97 | 0.00 |  | 0.00 |  | 102.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 149.85 | 102.27 | 0.00 |  | 0.00 |  | 102.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 147.19 | 99.60 | 0.00 |  | 0.00 |  | 99.60 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-budget64/http-recv-busy-10000, loop 0 on CPU 1

1825336938 iterations, longest 400.1 µs; over 5 / 20 / 100 µs: 81865 / 1003 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 746 | 19235.1 | 52.0 |
| mostly unexplained (+softirq NET_RX) | 95 | 2506.4 | 49.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 64 | 2066.8 | 157.3 |
| softirq NET_RX | 73 | 1769.8 | 37.1 |
| mostly unexplained (+irq work (IPI/vector)) | 21 | 496.8 | 28.5 |
| softirq TIMER | 1 | 400.1 | 400.1 |
| loop work | 3 | 64.1 | 22.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 400.1 | 1 | softirq TIMER | softirq TIMER 389, local timer tick 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 157.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 148.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 131.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 124.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 64.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 57.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 57.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 56.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 52.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 10, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2239 |
| LOC Local timer interrupts | 2236 |
| CAL Function call interrupts | 997 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-budget64/http-recv-busy-10000 (median 46.33)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 436.45 | 390.12 | 0.00 |  | 0.00 |  | 390.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 390.46 | 344.13 | 0.00 |  | 0.00 |  | 344.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 350.69 | 304.36 | 0.00 |  | 0.00 |  | 304.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 293.27 | 246.94 | 0.00 |  | 0.00 |  | 246.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 283.32 | 237.00 | 0.00 |  | 0.00 |  | 237.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 250.75 | 204.43 | 0.00 |  | 0.00 |  | 204.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 246.55 | 200.22 | 0.00 |  | 0.00 |  | 200.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 240.09 | 193.76 | 0.00 |  | 230.46 | softirq TIMER | 0.00 | server: softirq TIMER |
| 9 | 238.39 | 192.07 | 0.00 |  | 0.00 |  | 192.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 209.64 | 163.31 | 0.00 |  | 0.00 |  | 163.31 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-budget64/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=535923 clocksource=tsc

### Loop counters (r1-budget64/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002750 | 596475 | 5.03 | 108827520 | 36.24 | 1813792 | 1813792 | 99.5 | 65.4 | true | 0 | 675 | 623.9 | 108750 / 5701 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (72752 of 2499982 in 1-msg reads) | 72752 | 2.77 | 2.49 | 3.62 | 5.08 | 10.68 | 26.49 | 33.16 |

#### Stalls: r1-budget64/md-recv-busy, loop 0 on CPU 1

108827520 iterations, longest 623.9 µs; over 5 / 20 / 100 µs: 108750 / 5701 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5440 | 161858.0 | 623.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 92 | 2431.7 | 143.8 |
| mostly unexplained (+preempted by kworker/1:1) | 89 | 2358.4 | 34.9 |
| softirq NET_RX | 59 | 1508.8 | 35.5 |
| mostly unexplained (+softirq NET_RX) | 16 | 442.2 | 33.2 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 74.3 | 30.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 45.5 | 24.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 623.9 | 2750 | loop work (2750 items) |  |
| 2 | 143.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 133.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 67.9 | 15 | loop work (15 items) | softirq NET_RX 16, preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 66.7 | 14 | loop work (14 items) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 65.1 | 20 | loop work (20 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 64.6 | 19 | loop work (19 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 63.5 | 13 | loop work (13 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 59.7 | 5 | loop work (5 items) | softirq NET_RX 18, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 59.6 | 16 | loop work (16 items) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 691 |
| LOC Local timer interrupts | 681 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-budget64/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-budget64/noise, loop 0 on CPU 1

9051852800 iterations, longest 122.7 µs; over 5 / 20 / 100 µs: 62 / 8 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 56 | 886.8 | 122.7 |
| softirq TIMER | 1 | 112.0 | 112.0 |
| mostly unexplained (+local timer tick) | 4 | 29.5 | 9.2 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 5.1 | 5.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 122.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 112.0 | 0 | softirq TIMER | softirq TIMER 97, local timer tick 8, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 3 | 106.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 100.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 74.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 36.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 22.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 20.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 13.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 12.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-budget64/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1273797 | 1273797 | 1.00 | 107584966 | 84.46 | 1793083 | 1771853 | 98.8 | 71.6 | true | 0 | 2607 | 151.7 | 43604 / 594 / 3 | 0 | 0 |

#### Stalls: r1-budget64/pp-recv-busy-0, loop 0 on CPU 1

106311168 iterations, longest 151.7 µs; over 5 / 20 / 100 µs: 43604 / 594 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 415 | 9423.4 | 36.2 |
| mostly unexplained (+softirq NET_RX) | 84 | 1960.9 | 32.1 |
| softirq NET_RX | 71 | 1569.3 | 30.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 13 | 694.2 | 151.7 |
| mostly unexplained (+local timer tick) | 5 | 127.1 | 29.0 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 110.0 | 34.7 |
| mostly unexplained (+ipi_send_cpu) | 1 | 28.2 | 28.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 25.4 | 25.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 151.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 129.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 106.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 59.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 49.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 36.2 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 34.7 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 33.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 33.9 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 32.6 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2655 |
| IWI IRQ work interrupts | 2614 |
| CAL Function call interrupts | 988 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-budget64/pp-recv-busy-0 (median 45.71)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 317.46 | 271.75 | 0.00 |  | 20.93 | unexplained: no kernel event (hypervisor or in-kernel time) | 250.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 235.44 | 189.73 | 0.00 |  | 0.00 |  | 189.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 230.13 | 184.42 | 0.00 |  | 0.00 |  | 184.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 207.62 | 161.91 | 0.00 |  | 0.00 |  | 161.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 184.66 | 138.95 | 0.00 |  | 0.00 |  | 138.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 183.44 | 137.73 | 0.00 |  | 0.00 |  | 137.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 181.43 | 135.72 | 151.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 181.14 | 135.43 | 0.00 |  | 0.00 |  | 135.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 174.53 | 128.82 | 0.00 |  | 0.00 |  | 128.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 170.87 | 125.16 | 0.00 |  | 0.00 |  | 125.16 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-budget64/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196852 | 1196852 | 1.00 | 227758068 | 190.30 | 1897984 | 1888010 | 99.5 | 68.2 | true | 0 | 1913 | 146.5 | 11369 / 743 / 4 | 0 | 0 |

#### Stalls: r1-budget64/pp-recv-busy-10000, loop 0 on CPU 1

226561216 iterations, longest 146.5 µs; over 5 / 20 / 100 µs: 11369 / 743 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 461 | 11076.5 | 34.6 |
| mostly unexplained (+softirq NET_RX) | 142 | 3543.3 | 56.7 |
| softirq NET_RX | 106 | 2469.2 | 40.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 23 | 1161.9 | 146.5 |
| mostly unexplained (+local timer tick) | 7 | 184.5 | 32.2 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 97.0 | 36.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 22.3 | 22.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 146.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 140.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 128.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 118.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 65.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 61.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 56.7 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 56.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 52.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 51.6 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 25, preempted by kworker/1:1 10, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 1969 |
| IWI IRQ work interrupts | 1921 |
| CAL Function call interrupts | 998 |

#### Slowest round trips: r1-budget64/pp-recv-busy-10000 (median 46.75)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 320.50 | 273.75 | 0.00 |  | 174.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 99.25 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 268.16 | 221.41 | 0.00 |  | 0.00 |  | 221.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 236.74 | 189.99 | 0.00 |  | 0.00 |  | 189.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 183.05 | 136.30 | 0.00 |  | 0.00 |  | 136.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 182.91 | 136.16 | 0.00 |  | 0.00 |  | 136.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 180.18 | 133.43 | 0.00 |  | 0.00 |  | 133.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 170.01 | 123.26 | 0.00 |  | 175.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 167.21 | 120.46 | 0.00 |  | 0.00 |  | 120.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 165.20 | 118.45 | 0.00 |  | 0.00 |  | 118.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 165.05 | 118.30 | 0.00 |  | 0.00 |  | 118.30 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-10/http-recv-busy-10000, loop 0 on CPU 1

1821531890 iterations, longest 151.5 µs; over 5 / 20 / 100 µs: 101438 / 1283 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 907 | 23068.0 | 49.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 117 | 3178.4 | 151.5 |
| mostly unexplained (+softirq NET_RX) | 118 | 3048.6 | 44.9 |
| softirq NET_RX | 96 | 2376.8 | 43.7 |
| mostly unexplained (+irq work (IPI/vector)) | 24 | 583.3 | 33.1 |
| mostly unexplained (+local timer tick) | 18 | 539.1 | 50.6 |
| softirq TIMER | 1 | 67.5 | 67.5 |
| loop work | 2 | 44.3 | 22.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 151.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 146.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 136.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 124.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 67.5 | 1 | softirq TIMER | softirq TIMER 58, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 6 | 67.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 52.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 50.6 | 1 | mostly unexplained (+local timer tick) | local timer tick 10, preempted by kworker/1:1 9, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 9 | 49.9 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 44.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 20, preempted by ksoftirqd/1 6, local timer tick 5, timer tick_nohz_handler 3 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3770 |
| IWI IRQ work interrupts | 3687 |
| CAL Function call interrupts | 999 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-10/http-recv-busy-10000 (median 46.67)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 263.97 | 217.29 | 0.00 |  | 0.00 |  | 217.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 236.58 | 189.90 | 0.00 |  | 0.00 |  | 189.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 216.90 | 170.22 | 0.00 |  | 227.77 | softirq TIMER | 0.00 | server: softirq TIMER |
| 4 | 210.18 | 163.51 | 0.00 |  | 0.00 |  | 163.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 210.00 | 163.32 | 0.00 |  | 0.00 |  | 163.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 197.02 | 150.35 | 0.00 |  | 0.00 |  | 150.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 184.27 | 137.59 | 0.00 |  | 0.00 |  | 137.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 178.75 | 132.08 | 0.00 |  | 0.00 |  | 132.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 178.59 | 131.91 | 0.00 |  | 0.00 |  | 131.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 171.25 | 124.58 | 0.00 |  | 0.00 |  | 124.58 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-10/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=596713 clocksource=tsc

### Loop counters (r1-base-10/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002800 | 668072 | 4.49 | 109289536 | 36.40 | 1821492 | 1821492 | 99.4 | 65.3 | true | 0 | 471 | 636.3 | 78850 / 5920 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672627200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (100789 of 2499986 in 1-msg reads) | 100789 | 2.66 | 2.22 | 2.76 | 15.87 | 20.20 | 22.59 | 31.68 |

#### Stalls: r1-base-10/md-recv-busy, loop 0 on CPU 1

109289536 iterations, longest 636.3 µs; over 5 / 20 / 100 µs: 78850 / 5920 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5686 | 158605.4 | 636.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 122 | 3052.6 | 145.3 |
| softirq NET_RX | 51 | 1308.2 | 31.9 |
| mostly unexplained (+preempted by kworker/1:1) | 50 | 1241.2 | 43.6 |
| mostly unexplained (+softirq NET_RX) | 8 | 187.6 | 27.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 32.4 | 32.4 |
| mostly unexplained (+local timer tick) | 1 | 29.6 | 29.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 22.7 | 22.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 636.3 | 2800 | loop work (2800 items) |  |
| 2 | 145.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 131.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 57.8 | 10 | loop work (10 items) | local timer tick 4, preempted by kworker/1:1 3, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 5 | 57.3 | 10 | loop work (10 items) | softirq NET_RX 21, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 56.6 | 44 | loop work (44 items) | local timer tick 4, preempted by kworker/1:1 1, softirq RCU 1, timer tick_nohz_handler 1 |
| 7 | 56.6 | 8 | loop work (8 items) | local timer tick 4, preempted by kworker/1:1 3, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 8 | 55.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 55.0 | 9 | loop work (9 items) | local timer tick 5, preempted by kworker/1:1 2, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 54.6 | 9 | loop work (9 items) | local timer tick 3, preempted by kworker/1:1 2, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| LOC Local timer interrupts | 479 |
| IWI IRQ work interrupts | 478 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-10/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-10/noise, loop 0 on CPU 1

9042952192 iterations, longest 122.0 µs; over 5 / 20 / 100 µs: 63 / 7 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 55 | 919.7 | 122.0 |
| softirq TIMER | 1 | 110.8 | 110.8 |
| local timer tick | 2 | 22.7 | 16.9 |
| mostly unexplained (+local timer tick) | 3 | 21.7 | 9.1 |
| mostly unexplained (+call function (IPI/vector)) | 2 | 11.8 | 6.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 122.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 111.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 110.8 | 0 | softirq TIMER | softirq TIMER 90, local timer tick 11, timer tick_nohz_handler 6 |
| 4 | 98.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 70.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 50.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 35.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 17.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 17.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 16.9 | 0 | local timer tick | local timer tick 9, timer tick_nohz_handler 5 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 1 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-base-10/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1140401 | 1140401 | 1.00 | 280944035 | 246.36 | 4682401 | 4644387 | 0.0 | 40.7 | false | 0 | 0 | 132.5 | 47319 / 24 / 2 | 0 | 0 |

#### Stalls: r1-base-10/pp-epoll-spin-0, loop 0 on CPU 1

278663232 iterations, longest 132.5 µs; over 5 / 20 / 100 µs: 47319 / 24 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 24 | 885.3 | 132.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 132.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 114.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 57.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 56.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 45.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 36.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 33.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 31.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 30.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 30.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 14 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-epoll-spin-0 (median 50.16)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 218.94 | 168.78 | 0.00 |  | 0.00 |  | 168.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 199.00 | 148.84 | 0.00 |  | 0.00 |  | 148.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 197.44 | 147.28 | 0.00 |  | 0.00 |  | 147.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 194.33 | 144.17 | 0.00 |  | 0.00 |  | 144.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 194.11 | 143.95 | 0.00 |  | 0.00 |  | 143.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 191.77 | 141.61 | 0.00 |  | 0.00 |  | 141.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 190.25 | 140.09 | 0.00 |  | 0.00 |  | 140.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 189.62 | 139.47 | 0.00 |  | 0.00 |  | 139.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 189.61 | 139.45 | 0.00 |  | 0.00 |  | 139.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 188.87 | 138.71 | 0.00 |  | 0.00 |  | 138.71 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1192856 | 1192856 | 1.00 | 558996656 | 468.62 | 4658305 | 4638425 | 0.0 | 37.4 | false | 0 | 0 | 141.6 | 36031 / 17 / 4 | 0 | 0 |

#### Stalls: r1-base-10/pp-epoll-spin-10000, loop 0 on CPU 1

556610944 iterations, longest 141.6 µs; over 5 / 20 / 100 µs: 36031 / 17 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 17 | 878.4 | 141.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 141.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 134.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 119.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 116.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 58.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 40.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 31.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 29.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 27.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 24.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 4 |

#### Slowest round trips: r1-base-10/pp-epoll-spin-10000 (median 51.75)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 978.73 | 926.98 | 0.00 |  | 0.00 |  | 926.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 242.24 | 190.49 | 0.00 |  | 0.00 |  | 190.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 240.42 | 188.67 | 0.00 |  | 0.00 |  | 188.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 239.21 | 187.46 | 0.00 |  | 0.00 |  | 187.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 231.28 | 179.54 | 0.00 |  | 0.00 |  | 179.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 223.45 | 171.70 | 0.00 |  | 0.00 |  | 171.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 223.10 | 171.35 | 0.00 |  | 0.00 |  | 171.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 221.51 | 169.76 | 0.00 |  | 0.00 |  | 169.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 211.05 | 159.30 | 0.00 |  | 0.00 |  | 159.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 210.68 | 158.93 | 0.00 |  | 0.00 |  | 158.93 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1264229 | 1264229 | 1.00 | 108339110 | 85.70 | 1805652 | 1784581 | 98.8 | 71.3 | true | 0 | 2203 | 144.1 | 37616 / 413 / 2 | 0 | 0 |

#### Stalls: r1-base-10/pp-recv-busy-0, loop 0 on CPU 1

107074880 iterations, longest 144.1 µs; over 5 / 20 / 100 µs: 37616 / 413 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 288 | 6557.1 | 42.7 |
| mostly unexplained (+softirq NET_RX) | 64 | 1518.8 | 45.4 |
| softirq NET_RX | 37 | 829.5 | 31.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 10 | 563.6 | 144.1 |
| mostly unexplained (+local timer tick) | 10 | 237.4 | 32.4 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 90.6 | 37.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 24.4 | 24.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 144.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 133.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 90.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 45.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 22, preempted by kworker/1:1 9, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 42.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 40.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 39.7 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 37.8 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 36.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 16, preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 36.6 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2259 |
| IWI IRQ work interrupts | 2209 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-recv-busy-0 (median 47.32)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 205.42 | 158.09 | 0.00 |  | 27.31 | unexplained: no kernel event (hypervisor or in-kernel time) | 130.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 192.83 | 145.51 | 0.00 |  | 0.00 |  | 145.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 192.47 | 145.15 | 144.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 1.03 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 185.82 | 138.50 | 0.00 |  | 0.00 |  | 138.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 185.54 | 138.22 | 0.00 |  | 0.00 |  | 138.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 183.24 | 135.91 | 0.00 |  | 0.00 |  | 135.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 182.46 | 135.14 | 0.00 |  | 0.00 |  | 135.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 181.79 | 134.47 | 0.00 |  | 0.00 |  | 134.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 173.25 | 125.93 | 0.00 |  | 0.00 |  | 125.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 169.28 | 121.95 | 0.00 |  | 0.00 |  | 121.95 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196849 | 1196849 | 1.00 | 224020017 | 187.17 | 1866833 | 1856860 | 99.5 | 68.3 | true | 0 | 4038 | 155.4 | 51922 / 931 / 4 | 0 | 0 |

#### Stalls: r1-base-10/pp-recv-busy-10000, loop 0 on CPU 1

222823168 iterations, longest 155.4 µs; over 5 / 20 / 100 µs: 51922 / 931 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 673 | 15435.0 | 34.3 |
| softirq NET_RX | 126 | 2874.2 | 31.0 |
| mostly unexplained (+softirq NET_RX) | 94 | 2240.2 | 43.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 19 | 971.8 | 155.4 |
| mostly unexplained (+local timer tick) | 15 | 375.6 | 31.5 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 64.8 | 35.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 60.7 | 40.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 155.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 148.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 130.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 124.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 59.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 43.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 41.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 40.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 35.1 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 34.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4112 |
| IWI IRQ work interrupts | 4045 |
| CAL Function call interrupts | 998 |

#### Slowest round trips: r1-base-10/pp-recv-busy-10000 (median 45.56)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1056.92 | 1011.35 | 0.00 |  | 0.00 |  | 1011.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 279.61 | 234.05 | 0.00 |  | 0.00 |  | 234.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 251.24 | 205.68 | 0.00 |  | 0.00 |  | 205.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 249.80 | 204.24 | 0.00 |  | 120.93 | unexplained: no kernel event (hypervisor or in-kernel time) | 83.31 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 249.74 | 204.18 | 0.00 |  | 0.00 |  | 204.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 203.74 | 158.18 | 0.00 |  | 0.00 |  | 158.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 197.94 | 152.37 | 0.00 |  | 0.00 |  | 152.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 190.29 | 144.73 | 147.76 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 181.54 | 135.98 | 0.00 |  | 0.00 |  | 135.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 181.31 | 135.75 | 0.00 |  | 0.00 |  | 135.75 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1121899 | 1121899 | 1.00 | 3365698 | 3.00 | 56095 | 44817138 | 0.0 | 9.3 | false | 0 | 0 | 203.5 | 27848 / 18 / 3 | 0 | 0 |

#### Stalls: r1-base-10/pp-uring-spin-0, loop 0 on CPU 1

2689028288 iterations, longest 203.5 µs; over 5 / 20 / 100 µs: 27848 / 18 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 15 | 666.1 | 147.8 |
| softirq TIMER | 1 | 203.5 | 203.5 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 35.5 | 35.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.2 | 21.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 203.5 | 0 | softirq TIMER | softirq TIMER 178, local timer tick 14, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 147.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 132.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 59.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 57.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 37.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 35.5 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 27.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 27.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 25.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 8 |
| IWI IRQ work interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-uring-spin-0 (median 49.01)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 514.72 | 465.71 | 0.00 |  | 0.00 |  | 465.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 334.54 | 285.53 | 203.47 | softirq TIMER | 0.00 |  | 82.06 | client: softirq TIMER |
| 3 | 310.05 | 261.04 | 0.00 |  | 0.00 |  | 261.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 285.17 | 236.16 | 0.00 |  | 0.00 |  | 236.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 246.57 | 197.57 | 0.00 |  | 0.00 |  | 197.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 230.21 | 181.20 | 0.00 |  | 0.00 |  | 181.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 222.29 | 173.28 | 0.00 |  | 0.00 |  | 173.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 222.19 | 173.18 | 0.00 |  | 0.00 |  | 173.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 219.53 | 170.52 | 0.00 |  | 0.00 |  | 170.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 213.48 | 164.47 | 0.00 |  | 0.00 |  | 164.47 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1198807 | 1198807 | 1.00 | 3596422 | 3.00 | 29970 | 35448228 | 0.0 | 5.8 | false | 0 | 0 | 155.1 | 34931 / 27 / 4 | 0 | 0 |

#### Stalls: r1-base-10/pp-uring-spin-10000, loop 0 on CPU 1

4253787328 iterations, longest 155.1 µs; over 5 / 20 / 100 µs: 34931 / 27 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 26 | 1169.7 | 155.1 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 28.7 | 28.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 155.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 137.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 124.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 111.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 60.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 53.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 50.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 45.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 36.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 28.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-uring-spin-10000 (median 52.02)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 269.39 | 217.36 | 0.00 |  | 21.43 | unexplained: no kernel event (hypervisor or in-kernel time) | 195.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 222.15 | 170.13 | 0.00 |  | 0.00 |  | 170.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 214.69 | 162.67 | 0.00 |  | 0.00 |  | 162.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 203.73 | 151.71 | 0.00 |  | 0.00 |  | 151.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 201.68 | 149.65 | 0.00 |  | 0.00 |  | 149.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 201.54 | 149.52 | 0.00 |  | 0.00 |  | 149.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 201.16 | 149.14 | 0.00 |  | 0.00 |  | 149.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 198.02 | 146.00 | 155.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 193.78 | 141.76 | 0.00 |  | 0.00 |  | 141.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 192.54 | 140.52 | 0.00 |  | 0.00 |  | 140.52 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1250227 | 1250227 | 1.00 | 1 | 0.00 | 0 | 35574151 | 0.0 | 0.0 | true | 0 | 0 | 140.1 | 61 / 6 / 2 | 0 | 0 |

#### Stalls: r1-base-10/pp-uring-sqpoll-0, loop 0 on CPU 1

2134449088 iterations, longest 140.1 µs; over 5 / 20 / 100 µs: 61 / 6 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 465.3 | 140.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 140.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 137.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 57.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 53.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 44.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 33.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-uring-sqpoll-0 (median 47.61)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1044.55 | 996.94 | 0.00 |  | 0.00 |  | 996.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 300.37 | 252.76 | 0.00 |  | 0.00 |  | 252.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 184.96 | 137.35 | 0.00 |  | 0.00 |  | 137.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 180.65 | 133.04 | 0.00 |  | 0.00 |  | 133.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 179.33 | 131.72 | 136.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 176.48 | 128.87 | 0.00 |  | 0.00 |  | 128.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 173.65 | 126.04 | 0.00 |  | 0.00 |  | 126.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 173.48 | 125.87 | 0.00 |  | 0.00 |  | 125.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 172.89 | 125.28 | 0.00 |  | 0.00 |  | 125.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 171.56 | 123.95 | 140.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-10/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199504 | 1199504 | 1.00 | 1 | 0.00 | 0 | 26710708 | 0.0 | 0.0 | true | 0 | 0 | 273.4 | 137 / 11 / 5 | 0 | 0 |

#### Stalls: r1-base-10/pp-uring-sqpoll-10000, loop 0 on CPU 1

3205284992 iterations, longest 273.4 µs; over 5 / 20 / 100 µs: 137 / 11 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 767.7 | 150.7 |
| softirq TIMER | 1 | 273.4 | 273.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 25.0 | 25.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 273.4 | 0 | softirq TIMER | softirq TIMER 252, local timer tick 12, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 150.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 136.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 130.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 119.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 58.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 48.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 47.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 42.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 34.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1005 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-uring-sqpoll-10000 (median 47.46)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1037.29 | 989.83 | 0.00 |  | 236.63 | softirq TIMER | 753.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 278.79 | 231.33 | 273.41 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 175.90 | 128.44 | 0.00 |  | 0.00 |  | 128.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 174.49 | 127.03 | 0.00 |  | 0.00 |  | 127.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 170.77 | 123.31 | 0.00 |  | 0.00 |  | 123.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 167.02 | 119.56 | 0.00 |  | 0.00 |  | 119.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 165.47 | 118.01 | 0.00 |  | 0.00 |  | 118.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 162.09 | 114.64 | 0.00 |  | 0.00 |  | 114.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 157.81 | 110.35 | 0.00 |  | 0.00 |  | 110.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 156.20 | 108.74 | 0.00 |  | 28.79 | unexplained: no kernel event (hypervisor or in-kernel time) | 79.95 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-cstate/http-recv-busy-10000, loop 0 on CPU 1

1932559262 iterations, longest 148.9 µs; over 5 / 20 / 100 µs: 60863 / 1122 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 639 | 14610.5 | 37.7 |
| softirq NET_RX | 182 | 4073.8 | 29.7 |
| mostly unexplained (+softirq NET_RX) | 159 | 3803.5 | 41.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 117 | 3335.7 | 148.9 |
| mostly unexplained (+irq work (IPI/vector)) | 23 | 568.7 | 32.5 |
| softirq TIMER | 1 | 69.2 | 69.2 |
| loop work | 1 | 38.1 | 38.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 148.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 147.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 134.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 129.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 128.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 69.2 | 1 | softirq TIMER | softirq TIMER 60, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 7 | 68.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 67.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 60.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 55.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 4302 |
| LOC Local timer interrupts | 4298 |
| CAL Function call interrupts | 999 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-cstate/http-recv-busy-10000 (median 42.78)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 243.46 | 200.68 | 0.00 |  | 0.00 |  | 200.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 236.77 | 193.99 | 0.00 |  | 229.99 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 222.69 | 179.91 | 0.00 |  | 214.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 180.71 | 137.93 | 0.00 |  | 70.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 67.56 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 171.08 | 128.30 | 0.00 |  | 0.00 |  | 128.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 170.74 | 127.96 | 0.00 |  | 0.00 |  | 127.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 161.66 | 118.88 | 148.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 161.15 | 118.37 | 146.57 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 128.64 | 85.86 | 0.00 |  | 0.00 |  | 85.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 126.42 | 83.64 | 0.00 |  | 0.00 |  | 83.64 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-cstate/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=624058 clocksource=tsc

### Loop counters (r1-cstate/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002800 | 691192 | 4.34 | 107325568 | 35.74 | 1788759 | 1788759 | 99.4 | 65.3 | true | 0 | 832 | 597.6 | 105955 / 6597 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672627200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (104603 of 2500000 in 1-msg reads) | 104603 | 2.70 | 2.42 | 3.49 | 5.22 | 18.70 | 37.25 | 39.77 |

#### Stalls: r1-cstate/md-recv-busy, loop 0 on CPU 1

107325568 iterations, longest 597.6 µs; over 5 / 20 / 100 µs: 105955 / 6597 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 6290 | 176704.5 | 597.6 |
| mostly unexplained (+preempted by kworker/1:1) | 111 | 3133.3 | 49.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 96 | 2575.6 | 148.1 |
| softirq NET_RX | 60 | 1563.9 | 48.4 |
| mostly unexplained (+softirq NET_RX) | 24 | 697.1 | 38.4 |
| mostly unexplained (+local timer tick) | 13 | 421.6 | 37.5 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 89.0 | 30.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 597.6 | 2800 | loop work (2800 items) |  |
| 2 | 148.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 130.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 104.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 61.9 | 6 | loop work (6 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 56.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 53.9 | 8 | loop work (8 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 53.2 | 12 | loop work (12 items) | softirq NET_RX 16, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 52.9 | 41 | loop work (41 items) | preempted by kworker/1:1 1 |
| 10 | 52.8 | 16 | loop work (16 items) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| LOC Local timer interrupts | 966 |
| IWI IRQ work interrupts | 847 |
| TLB TLB shootdowns | 2 |
| RES Rescheduling interrupts | 1 |

## r1-cstate/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-cstate/noise, loop 0 on CPU 1

8902217728 iterations, longest 304.1 µs; over 5 / 20 / 100 µs: 72 / 8 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 59 | 921.0 | 125.8 |
| softirq TIMER | 1 | 304.1 | 304.1 |
| mostly unexplained (+call function (IPI/vector)) | 8 | 42.9 | 5.9 |
| mostly unexplained (+local timer tick) | 4 | 35.9 | 12.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 304.1 | 0 | softirq TIMER | softirq TIMER 282, local timer tick 13, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 125.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 100.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 94.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 80.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 52.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 28.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 24.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 17.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 16.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-cstate/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1158565 | 1158565 | 1.00 | 280295307 | 241.93 | 4671588 | 4632970 | 0.0 | 40.4 | false | 0 | 0 | 143.2 | 3298 / 8 / 2 | 0 | 0 |

#### Stalls: r1-cstate/pp-epoll-spin-0, loop 0 on CPU 1

277978176 iterations, longest 143.2 µs; over 5 / 20 / 100 µs: 3298 / 8 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 452.4 | 143.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 50.4 | 50.4 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 24.8 | 24.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 143.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 126.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 69.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 57.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 50.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 36.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 24.8 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 20.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 16 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-cstate/pp-epoll-spin-0 (median 49.01)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 304.29 | 255.28 | 0.00 |  | 23.65 | unexplained: no kernel event (hypervisor or in-kernel time) | 231.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 219.25 | 170.24 | 0.00 |  | 0.00 |  | 170.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 213.45 | 164.43 | 0.00 |  | 0.00 |  | 164.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 196.24 | 147.23 | 0.00 |  | 0.00 |  | 147.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 194.54 | 145.53 | 0.00 |  | 0.00 |  | 145.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 192.69 | 143.67 | 0.00 |  | 0.00 |  | 143.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 192.14 | 143.13 | 0.00 |  | 0.00 |  | 143.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 190.72 | 141.71 | 0.00 |  | 0.00 |  | 141.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 189.85 | 140.83 | 0.00 |  | 0.00 |  | 140.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 189.56 | 140.54 | 0.00 |  | 0.00 |  | 140.54 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-cstate/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198513 | 1198513 | 1.00 | 554676386 | 462.80 | 4622303 | 4602328 | 0.0 | 36.5 | false | 0 | 0 | 138.8 | 8499 / 19 / 4 | 0 | 0 |

#### Stalls: r1-cstate/pp-epoll-spin-10000, loop 0 on CPU 1

552279360 iterations, longest 138.8 µs; over 5 / 20 / 100 µs: 8499 / 19 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 18 | 1035.5 | 138.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 37.3 | 37.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 138.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 135.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 122.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 121.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 79.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 56.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 55.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 51.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 42.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 40.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-cstate/pp-epoll-spin-10000 (median 48.16)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 204.20 | 156.04 | 0.00 |  | 0.00 |  | 156.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 198.29 | 150.13 | 0.00 |  | 49.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 100.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 194.65 | 146.49 | 0.00 |  | 0.00 |  | 146.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 192.18 | 144.02 | 0.00 |  | 0.00 |  | 144.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 190.93 | 142.76 | 0.00 |  | 0.00 |  | 142.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 188.96 | 140.79 | 0.00 |  | 0.00 |  | 140.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 177.98 | 129.82 | 0.00 |  | 0.00 |  | 129.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 175.82 | 127.66 | 0.00 |  | 0.00 |  | 127.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 169.79 | 121.63 | 0.00 |  | 0.00 |  | 121.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 168.03 | 119.87 | 0.00 |  | 0.00 |  | 119.87 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-cstate/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1243508 | 1243508 | 1.00 | 108830389 | 87.52 | 1813840 | 1793115 | 98.8 | 70.7 | true | 0 | 2476 | 144.3 | 14886 / 223 / 2 | 0 | 0 |

#### Stalls: r1-cstate/pp-recv-busy-0, loop 0 on CPU 1

107586880 iterations, longest 144.3 µs; over 5 / 20 / 100 µs: 14886 / 223 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 135 | 2936.5 | 33.6 |
| softirq NET_RX | 45 | 1003.9 | 41.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 13 | 656.2 | 144.3 |
| mostly unexplained (+softirq NET_RX) | 18 | 441.9 | 33.9 |
| mostly unexplained (+local timer tick) | 9 | 219.2 | 27.5 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 56.8 | 31.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.7 | 35.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 144.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 136.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 61.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 57.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 54.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 41.7 | 0 | softirq NET_RX | softirq NET_RX 23, preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 36.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 35.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 33.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 4, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 33.6 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2529 |
| IWI IRQ work interrupts | 2481 |
| CAL Function call interrupts | 989 |

#### Slowest round trips: r1-cstate/pp-recv-busy-0 (median 48.65)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 482.76 | 434.11 | 0.00 |  | 0.00 |  | 434.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 278.42 | 229.77 | 0.00 |  | 0.00 |  | 229.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 192.70 | 144.05 | 0.00 |  | 0.00 |  | 144.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 191.36 | 142.71 | 0.00 |  | 0.00 |  | 142.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 181.92 | 133.28 | 0.00 |  | 0.00 |  | 133.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 178.94 | 130.29 | 0.00 |  | 0.00 |  | 130.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 175.96 | 127.31 | 0.00 |  | 0.00 |  | 127.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 174.56 | 125.91 | 0.00 |  | 0.00 |  | 125.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 171.63 | 122.98 | 0.00 |  | 0.00 |  | 122.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 169.93 | 121.28 | 0.00 |  | 0.00 |  | 121.28 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-cstate/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196852 | 1196852 | 1.00 | 222363380 | 185.79 | 1853028 | 1843054 | 99.5 | 67.9 | true | 0 | 4954 | 155.3 | 32827 / 737 / 4 | 0 | 0 |

#### Stalls: r1-cstate/pp-recv-busy-10000, loop 0 on CPU 1

221166528 iterations, longest 155.3 µs; over 5 / 20 / 100 µs: 32827 / 737 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 478 | 10412.5 | 31.2 |
| softirq NET_RX | 120 | 2614.5 | 28.0 |
| mostly unexplained (+softirq NET_RX) | 100 | 2415.9 | 155.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 20 | 907.4 | 144.6 |
| mostly unexplained (+local timer tick) | 15 | 353.4 | 30.1 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 81.2 | 28.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 22.8 | 22.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 155.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 144.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 127.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 126.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 88.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 67.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 35.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 31.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 31.2 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 30.1 | 1 | mostly unexplained (+local timer tick) | local timer tick 7, timer tick_nohz_handler 4, preempted by kworker/1:1 3, irq work (IPI/vector) 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 5059 |
| IWI IRQ work interrupts | 4962 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-cstate/pp-recv-busy-10000 (median 42.91)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 187.88 | 144.97 | 0.00 |  | 0.00 |  | 144.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 178.87 | 135.96 | 144.61 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 177.45 | 134.54 | 155.00 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 4 | 172.87 | 129.96 | 0.00 |  | 0.00 |  | 129.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 171.99 | 129.08 | 0.00 |  | 0.00 |  | 129.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 171.35 | 128.44 | 0.00 |  | 0.00 |  | 128.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 170.74 | 127.82 | 0.00 |  | 0.00 |  | 127.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 170.45 | 127.54 | 0.00 |  | 0.00 |  | 127.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 168.65 | 125.74 | 126.74 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 167.14 | 124.23 | 0.00 |  | 0.00 |  | 124.23 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-cstate/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1132811 | 1132811 | 1.00 | 3398434 | 3.00 | 56641 | 44511458 | 0.0 | 9.3 | false | 0 | 0 | 388.3 | 7910 / 11 / 3 | 0 | 0 |

#### Stalls: r1-cstate/pp-uring-spin-0, loop 0 on CPU 1

2670687488 iterations, longest 388.3 µs; over 5 / 20 / 100 µs: 7910 / 11 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 541.0 | 147.3 |
| softirq TIMER | 1 | 388.3 | 388.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 32.8 | 32.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 388.3 | 0 | softirq TIMER | softirq TIMER 364, local timer tick 14, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 147.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 129.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 93.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 56.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 32.8 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 31.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 21.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 20.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 20.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 14 |
| IWI IRQ work interrupts | 6 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-cstate/pp-uring-spin-0 (median 48.87)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 445.55 | 396.68 | 388.33 | softirq TIMER | 0.00 |  | 8.34 | client: softirq TIMER |
| 2 | 314.12 | 265.25 | 0.00 |  | 0.00 |  | 265.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 221.81 | 172.94 | 0.00 |  | 132.29 | softirq TIMER | 40.65 | server: softirq TIMER |
| 4 | 218.33 | 169.45 | 0.00 |  | 0.00 |  | 169.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 210.40 | 161.53 | 0.00 |  | 185.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 182.79 | 133.92 | 0.00 |  | 0.00 |  | 133.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 179.64 | 130.76 | 0.00 |  | 0.00 |  | 130.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 178.37 | 129.50 | 0.00 |  | 0.00 |  | 129.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 176.15 | 127.27 | 0.00 |  | 0.00 |  | 127.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 174.22 | 125.34 | 147.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-cstate/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199487 | 1199487 | 1.00 | 3598461 | 3.00 | 29987 | 36100810 | 0.0 | 5.0 | false | 0 | 0 | 141.3 | 2764 / 17 / 4 | 0 | 0 |

#### Stalls: r1-cstate/pp-uring-spin-10000, loop 0 on CPU 1

4332097152 iterations, longest 141.3 µs; over 5 / 20 / 100 µs: 2764 / 17 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 16 | 979.8 | 141.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 26.9 | 26.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 141.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 136.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 129.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 123.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 69.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 56.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 51.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 47.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 31.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 4 |

#### Slowest round trips: r1-cstate/pp-uring-spin-10000 (median 48.40)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 237.17 | 188.77 | 0.00 |  | 0.00 |  | 188.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 233.52 | 185.13 | 0.00 |  | 0.00 |  | 185.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 176.93 | 128.53 | 0.00 |  | 0.00 |  | 128.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 172.38 | 123.98 | 0.00 |  | 0.00 |  | 123.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 172.12 | 123.73 | 0.00 |  | 0.00 |  | 123.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 170.44 | 122.05 | 0.00 |  | 0.00 |  | 122.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 168.68 | 120.28 | 0.00 |  | 0.00 |  | 120.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 163.62 | 115.22 | 0.00 |  | 0.00 |  | 115.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 162.72 | 114.32 | 0.00 |  | 0.00 |  | 114.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 162.51 | 114.11 | 0.00 |  | 0.00 |  | 114.11 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-cstate/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1211785 | 1211785 | 1.00 | 1 | 0.00 | 0 | 33250855 | 0.0 | 0.0 | true | 0 | 0 | 151.1 | 67 / 4 / 2 | 0 | 0 |

#### Stalls: r1-cstate/pp-uring-sqpoll-0, loop 0 on CPU 1

1995051328 iterations, longest 151.1 µs; over 5 / 20 / 100 µs: 67 / 4 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 385.0 | 151.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 151.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 126.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 72.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 34.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| LOC Local timer interrupts | 10 |
| IWI IRQ work interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-cstate/pp-uring-sqpoll-0 (median 48.81)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 196.93 | 148.12 | 151.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 185.66 | 136.84 | 0.00 |  | 0.00 |  | 136.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 175.02 | 126.21 | 0.00 |  | 0.00 |  | 126.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 163.63 | 114.82 | 0.00 |  | 154.12 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 162.82 | 114.01 | 126.93 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 159.76 | 110.95 | 0.00 |  | 0.00 |  | 110.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 156.75 | 107.94 | 0.00 |  | 0.00 |  | 107.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 152.41 | 103.60 | 0.00 |  | 0.00 |  | 103.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 150.39 | 101.58 | 0.00 |  | 0.00 |  | 101.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 141.57 | 92.76 | 0.00 |  | 0.00 |  | 92.76 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-cstate/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199302 | 1199302 | 1.00 | 1 | 0.00 | 0 | 21485981 | 0.0 | 0.0 | true | 0 | 0 | 201.1 | 130 / 13 / 5 | 0 | 0 |

#### Stalls: r1-cstate/pp-uring-sqpoll-10000, loop 0 on CPU 1

2578317760 iterations, longest 201.1 µs; over 5 / 20 / 100 µs: 130 / 13 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 12 | 837.7 | 144.6 |
| softirq TIMER | 1 | 201.1 | 201.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 201.1 | 0 | softirq TIMER | softirq TIMER 177, local timer tick 12, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 144.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 130.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 127.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 114.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 55.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 50.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 50.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 40.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1006 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-cstate/pp-uring-sqpoll-10000 (median 45.47)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1055.23 | 1009.75 | 0.00 |  | 0.00 |  | 1009.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 260.67 | 215.20 | 0.00 |  | 220.62 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 204.74 | 159.27 | 0.00 |  | 0.00 |  | 159.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 204.64 | 159.17 | 0.00 |  | 49.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 109.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 182.74 | 137.26 | 0.00 |  | 0.00 |  | 137.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 163.70 | 118.22 | 0.00 |  | 0.00 |  | 118.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 156.78 | 111.30 | 0.00 |  | 0.00 |  | 111.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 153.30 | 107.82 | 0.00 |  | 0.00 |  | 107.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 148.58 | 103.11 | 0.00 |  | 0.00 |  | 103.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 146.92 | 101.44 | 0.00 |  | 0.00 |  | 101.44 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-11/http-recv-busy-10000, loop 0 on CPU 1

1837082142 iterations, longest 151.3 µs; over 5 / 20 / 100 µs: 74171 / 1009 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 649 | 15707.3 | 43.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 121 | 3221.5 | 151.3 |
| mostly unexplained (+softirq NET_RX) | 99 | 2500.3 | 37.9 |
| softirq NET_RX | 74 | 1656.2 | 28.0 |
| mostly unexplained (+local timer tick) | 32 | 947.2 | 41.8 |
| mostly unexplained (+irq work (IPI/vector)) | 29 | 680.3 | 27.0 |
| loop work | 3 | 90.7 | 36.8 |
| softirq TIMER | 1 | 68.2 | 68.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.8 | 21.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 151.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 145.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 126.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 121.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 68.2 | 1 | softirq TIMER | softirq TIMER 58, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 6 | 48.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 43.9 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 41.8 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, preempted by kworker/1:1 3, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 9 | 41.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 41.6 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, preempted by kworker/1:1 4, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3891 |
| IWI IRQ work interrupts | 3841 |
| CAL Function call interrupts | 999 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-11/http-recv-busy-10000 (median 46.11)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1011.07 | 964.96 | 0.00 |  | 0.00 |  | 964.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 347.70 | 301.59 | 0.00 |  | 0.00 |  | 301.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 347.59 | 301.48 | 0.00 |  | 0.00 |  | 301.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 334.28 | 288.17 | 0.00 |  | 0.00 |  | 288.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 334.10 | 287.99 | 0.00 |  | 0.00 |  | 287.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 327.07 | 280.96 | 0.00 |  | 0.00 |  | 280.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 325.67 | 279.56 | 0.00 |  | 0.00 |  | 279.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 321.70 | 275.59 | 0.00 |  | 0.00 |  | 275.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 318.38 | 272.27 | 0.00 |  | 0.00 |  | 272.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 317.27 | 271.16 | 0.00 |  | 0.00 |  | 271.16 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-11/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=530691 clocksource=tsc

### Loop counters (r1-base-11/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3003100 | 589279 | 5.10 | 108955264 | 36.28 | 1815921 | 1815921 | 99.5 | 65.4 | true | 0 | 819 | 694.8 | 113018 / 5866 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672694400 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (72934 of 2500000 in 1-msg reads) | 72934 | 2.79 | 2.44 | 3.76 | 5.84 | 15.68 | 32.81 | 38.46 |

#### Stalls: r1-base-11/md-recv-busy, loop 0 on CPU 1

108955264 iterations, longest 694.8 µs; over 5 / 20 / 100 µs: 113018 / 5866 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5569 | 170087.8 | 694.8 |
| mostly unexplained (+preempted by kworker/1:1) | 119 | 3326.9 | 51.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 96 | 2701.7 | 149.2 |
| softirq NET_RX | 58 | 1533.4 | 43.8 |
| mostly unexplained (+softirq NET_RX) | 16 | 503.8 | 41.2 |
| mostly unexplained (+local timer tick) | 5 | 173.2 | 49.0 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 58.5 | 36.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 21.2 | 21.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 694.8 | 3100 | loop work (3100 items) |  |
| 2 | 149.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 130.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 68.3 | 16 | loop work (16 items) | softirq NET_RX 17, preempted by kworker/1:1 10, workqueue [ena] 8, local timer tick 4 |
| 5 | 65.5 | 25 | loop work (25 items) |  |
| 6 | 64.0 | 12 | loop work (12 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 61.6 | 20 | loop work (20 items) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 60.9 | 18 | loop work (18 items) | softirq NET_RX 20, preempted by kworker/1:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 55.9 | 8 | loop work (8 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 55.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 861 |
| IWI IRQ work interrupts | 833 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-11/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-11/noise, loop 0 on CPU 1

9043292160 iterations, longest 347.6 µs; over 5 / 20 / 100 µs: 75 / 9 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 68 | 1086.8 | 127.7 |
| softirq TIMER | 1 | 347.6 | 347.6 |
| mostly unexplained (+local timer tick) | 3 | 24.5 | 10.5 |
| mostly unexplained (+call function (IPI/vector)) | 3 | 15.9 | 5.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 347.6 | 0 | softirq TIMER | softirq TIMER 332, local timer tick 10, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 127.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 103.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 101.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 99.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 48.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 35.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 29.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 26.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 18.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-base-11/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1143565 | 1143565 | 1.00 | 280890651 | 245.63 | 4681511 | 4643392 | 0.0 | 40.6 | false | 0 | 0 | 142.7 | 55374 / 13 / 2 | 0 | 0 |

#### Stalls: r1-base-11/pp-epoll-spin-0, loop 0 on CPU 1

278603520 iterations, longest 142.7 µs; over 5 / 20 / 100 µs: 55374 / 13 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 12 | 518.2 | 142.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.3 | 23.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 142.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 111.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 40.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 34.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 32.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 27.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 23.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 22.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 22.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 12 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-11/pp-epoll-spin-0 (median 49.04)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1041.06 | 992.02 | 0.00 |  | 0.00 |  | 992.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 340.68 | 291.64 | 0.00 |  | 0.00 |  | 291.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 300.36 | 251.32 | 0.00 |  | 0.00 |  | 251.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 241.37 | 192.33 | 0.00 |  | 0.00 |  | 192.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 233.14 | 184.10 | 0.00 |  | 0.00 |  | 184.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 222.78 | 173.74 | 0.00 |  | 0.00 |  | 173.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 214.31 | 165.27 | 0.00 |  | 0.00 |  | 165.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 203.55 | 154.51 | 0.00 |  | 0.00 |  | 154.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 199.42 | 150.38 | 0.00 |  | 0.00 |  | 150.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 196.69 | 147.65 | 0.00 |  | 0.00 |  | 147.65 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-11/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1196760 | 1196760 | 1.00 | 556570545 | 465.06 | 4638088 | 4618142 | 0.0 | 36.8 | false | 0 | 0 | 149.6 | 6459 / 16 / 4 | 0 | 0 |

#### Stalls: r1-base-11/pp-epoll-spin-10000, loop 0 on CPU 1

554177024 iterations, longest 149.6 µs; over 5 / 20 / 100 µs: 6459 / 16 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 16 | 967.3 | 149.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 149.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 137.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 123.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 118.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 58.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 47.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 45.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 44.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 42.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 42.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 3 |

#### Slowest round trips: r1-base-11/pp-epoll-spin-10000 (median 50.84)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 999.45 | 948.62 | 0.00 |  | 0.00 |  | 948.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 220.92 | 170.08 | 0.00 |  | 0.00 |  | 170.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 176.55 | 125.71 | 0.00 |  | 0.00 |  | 125.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 167.24 | 116.41 | 0.00 |  | 0.00 |  | 116.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 155.54 | 104.71 | 0.00 |  | 0.00 |  | 104.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 152.85 | 102.02 | 0.00 |  | 0.00 |  | 102.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 152.37 | 101.53 | 0.00 |  | 0.00 |  | 101.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 152.28 | 101.44 | 0.00 |  | 0.00 |  | 101.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 151.41 | 100.57 | 0.00 |  | 0.00 |  | 100.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 150.83 | 99.99 | 0.00 |  | 0.00 |  | 99.99 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-11/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1281161 | 1281161 | 1.00 | 108431242 | 84.64 | 1807187 | 1785835 | 98.8 | 71.2 | true | 0 | 2519 | 152.7 | 37343 / 574 / 2 | 0 | 0 |

#### Stalls: r1-base-11/pp-recv-busy-0, loop 0 on CPU 1

107150080 iterations, longest 152.7 µs; over 5 / 20 / 100 µs: 37343 / 574 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 388 | 8795.5 | 35.5 |
| softirq NET_RX | 84 | 1844.7 | 29.5 |
| mostly unexplained (+softirq NET_RX) | 78 | 1807.3 | 30.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 14 | 713.5 | 152.7 |
| mostly unexplained (+irq work (IPI/vector)) | 5 | 142.9 | 31.0 |
| mostly unexplained (+local timer tick) | 5 | 115.6 | 27.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 152.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 130.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 65.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 61.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 61.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 43.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 35.5 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 33.5 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 13, workqueue [ena] 8, local timer tick 4, timer tick_nohz_handler 2 |
| 9 | 33.3 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 33.2 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2561 |
| IWI IRQ work interrupts | 2528 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-11/pp-recv-busy-0 (median 46.78)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 275.21 | 228.43 | 0.00 |  | 0.00 |  | 228.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 207.76 | 160.98 | 0.00 |  | 0.00 |  | 160.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 200.23 | 153.45 | 0.00 |  | 0.00 |  | 153.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 184.92 | 138.14 | 152.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 167.03 | 120.25 | 0.00 |  | 0.00 |  | 120.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 163.67 | 116.89 | 130.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 153.64 | 106.86 | 0.00 |  | 0.00 |  | 106.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 144.67 | 97.89 | 0.00 |  | 26.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 71.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 142.36 | 95.58 | 0.00 |  | 0.00 |  | 95.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 133.92 | 87.14 | 0.00 |  | 0.00 |  | 87.14 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-11/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196890 | 1196890 | 1.00 | 222936474 | 186.26 | 1857804 | 1847830 | 99.5 | 68.1 | true | 0 | 3144 | 143.4 | 52780 / 853 / 4 | 0 | 0 |

#### Stalls: r1-base-11/pp-recv-busy-10000, loop 0 on CPU 1

221739584 iterations, longest 143.4 µs; over 5 / 20 / 100 µs: 52780 / 853 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 639 | 14776.0 | 32.6 |
| softirq NET_RX | 106 | 2423.1 | 37.9 |
| mostly unexplained (+softirq NET_RX) | 78 | 2030.9 | 129.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 15 | 832.6 | 143.4 |
| mostly unexplained (+local timer tick) | 14 | 343.6 | 32.6 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 34.1 | 34.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 143.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 142.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 137.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 129.1 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 63.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 61.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 43.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 41.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 39.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.9 | 1 | softirq NET_RX | softirq NET_RX 20, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3252 |
| IWI IRQ work interrupts | 3151 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-base-11/pp-recv-busy-10000 (median 43.83)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 996.67 | 952.85 | 0.00 |  | 0.00 |  | 952.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 204.35 | 160.52 | 0.00 |  | 0.00 |  | 160.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 187.78 | 143.95 | 0.00 |  | 198.78 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 158.75 | 114.93 | 141.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 147.91 | 104.08 | 0.00 |  | 0.00 |  | 104.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 143.81 | 99.99 | 0.00 |  | 0.00 |  | 99.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 141.08 | 97.26 | 0.00 |  | 0.00 |  | 97.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 137.04 | 93.22 | 0.00 |  | 0.00 |  | 93.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 135.56 | 91.73 | 0.00 |  | 0.00 |  | 91.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 134.00 | 90.17 | 0.00 |  | 0.00 |  | 90.17 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-11/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1148416 | 1148416 | 1.00 | 3445249 | 3.00 | 57421 | 44729361 | 0.0 | 9.5 | false | 0 | 0 | 392.9 | 40540 / 25 / 3 | 0 | 0 |

#### Stalls: r1-base-11/pp-uring-spin-0, loop 0 on CPU 1

2683761664 iterations, longest 392.9 µs; over 5 / 20 / 100 µs: 40540 / 25 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 23 | 871.3 | 152.0 |
| softirq TIMER | 1 | 392.9 | 392.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 35.6 | 35.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 392.9 | 0 | softirq TIMER | softirq TIMER 366, local timer tick 15, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 152.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 131.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 91.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 56.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 40.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 35.6 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 27.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 26.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 25.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 15 |
| IWI IRQ work interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-11/pp-uring-spin-0 (median 48.35)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1062.24 | 1013.89 | 0.00 |  | 0.00 |  | 1013.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 412.12 | 363.77 | 0.00 |  | 0.00 |  | 363.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 411.59 | 363.24 | 392.88 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 4 | 407.36 | 359.01 | 0.00 |  | 0.00 |  | 359.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 400.95 | 352.60 | 0.00 |  | 0.00 |  | 352.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 389.33 | 340.98 | 0.00 |  | 0.00 |  | 340.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 382.60 | 334.25 | 0.00 |  | 0.00 |  | 334.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 381.56 | 333.21 | 0.00 |  | 0.00 |  | 333.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 377.95 | 329.60 | 0.00 |  | 0.00 |  | 329.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 376.74 | 328.39 | 0.00 |  | 0.00 |  | 328.39 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-11/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1193987 | 1193987 | 1.00 | 3581962 | 3.00 | 29850 | 37598732 | 0.0 | 5.3 | false | 0 | 0 | 145.9 | 7312 / 19 / 4 | 0 | 0 |

#### Stalls: r1-base-11/pp-uring-spin-10000, loop 0 on CPU 1

4511847808 iterations, longest 145.9 µs; over 5 / 20 / 100 µs: 7312 / 19 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 18 | 908.5 | 145.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 28.3 | 28.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 145.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 132.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 124.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 113.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 56.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 41.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 40.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 28.3 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 28.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 26.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-11/pp-uring-spin-10000 (median 54.80)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 377.08 | 322.28 | 0.00 |  | 0.00 |  | 322.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 369.86 | 315.06 | 0.00 |  | 0.00 |  | 315.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 358.24 | 303.44 | 0.00 |  | 0.00 |  | 303.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 353.13 | 298.33 | 0.00 |  | 0.00 |  | 298.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 345.62 | 290.81 | 0.00 |  | 0.00 |  | 290.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 341.67 | 286.87 | 0.00 |  | 0.00 |  | 286.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 339.24 | 284.44 | 0.00 |  | 0.00 |  | 284.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 339.13 | 284.32 | 0.00 |  | 0.00 |  | 284.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 339.05 | 284.24 | 0.00 |  | 0.00 |  | 284.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 338.86 | 284.05 | 0.00 |  | 0.00 |  | 284.05 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-11/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1243219 | 1243219 | 1.00 | 1 | 0.00 | 0 | 31395207 | 0.0 | 0.0 | true | 0 | 0 | 149.9 | 65 / 7 / 2 | 0 | 0 |

#### Stalls: r1-base-11/pp-uring-sqpoll-0, loop 0 on CPU 1

1883712448 iterations, longest 149.9 µs; over 5 / 20 / 100 µs: 65 / 7 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 7 | 494.8 | 149.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 149.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 133.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 56.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 47.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 45.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 37.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 26.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-11/pp-uring-sqpoll-0 (median 47.95)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 323.85 | 275.90 | 0.00 |  | 0.00 |  | 275.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 255.62 | 207.67 | 0.00 |  | 0.00 |  | 207.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 224.12 | 176.17 | 0.00 |  | 0.00 |  | 176.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 205.05 | 157.10 | 0.00 |  | 0.00 |  | 157.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 180.82 | 132.87 | 149.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 172.59 | 124.64 | 131.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 165.15 | 117.20 | 0.00 |  | 27.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 89.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 162.71 | 114.76 | 0.00 |  | 0.00 |  | 114.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 146.40 | 98.45 | 0.00 |  | 0.00 |  | 98.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 145.35 | 97.40 | 0.00 |  | 0.00 |  | 97.40 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-11/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1185257 | 1185257 | 1.00 | 1 | 0.00 | 0 | 25935725 | 0.0 | 0.0 | true | 0 | 0 | 255.8 | 120 / 10 / 5 | 0 | 0 |

#### Stalls: r1-base-11/pp-uring-sqpoll-10000, loop 0 on CPU 1

3112287040 iterations, longest 255.8 µs; over 5 / 20 / 100 µs: 120 / 10 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 735.2 | 145.6 |
| softirq TIMER | 1 | 255.8 | 255.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 255.8 | 0 | softirq TIMER | softirq TIMER 232, local timer tick 14, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 145.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 135.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 134.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 127.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 57.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 49.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 32.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 20.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1005 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-11/pp-uring-sqpoll-10000 (median 46.55)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 369.08 | 322.53 | 0.00 |  | 0.00 |  | 322.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 357.85 | 311.30 | 0.00 |  | 0.00 |  | 311.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 351.22 | 304.67 | 0.00 |  | 0.00 |  | 304.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 348.99 | 302.44 | 0.00 |  | 0.00 |  | 302.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 346.37 | 299.82 | 0.00 |  | 0.00 |  | 299.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 343.62 | 297.07 | 0.00 |  | 0.00 |  | 297.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 341.88 | 295.33 | 0.00 |  | 0.00 |  | 295.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 337.40 | 290.85 | 0.00 |  | 0.00 |  | 290.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 336.77 | 290.22 | 0.00 |  | 0.00 |  | 290.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 336.14 | 289.60 | 0.00 |  | 0.00 |  | 289.60 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: boot-stock/http-recv-busy-10000, loop 0 on CPU 1

1918734428 iterations, longest 59.1 µs; over 5 / 20 / 100 µs: 138088 / 2499 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2036 | 49611.0 | 59.1 |
| mostly unexplained (+softirq NET_RX) | 397 | 9852.6 | 39.2 |
| mostly unexplained (+preempted by kworker/1:1) | 40 | 1000.5 | 36.3 |
| mostly unexplained (+local timer tick) | 10 | 219.9 | 26.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 8 | 187.9 | 32.8 |
| softirq NET_RX | 8 | 175.6 | 25.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 59.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 39.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 3 | 38.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 37.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 37.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 37.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 37.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 36.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 36.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 3, softirq NET_RX 3, workqueue [ena] 1 |
| 10 | 36.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120378 |
| CAL Function call interrupts | 1004 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 95 |
| RES Rescheduling interrupts | 2 |
| TLB TLB shootdowns | 2 |

#### Slowest round trips: boot-stock/http-recv-busy-10000 (median 44.29)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 255.53 | 211.24 | 0.00 |  | 0.00 |  | 211.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 154.94 | 110.65 | 0.00 |  | 0.00 |  | 110.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 146.11 | 101.82 | 0.00 |  | 0.00 |  | 101.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 145.97 | 101.68 | 0.00 |  | 0.00 |  | 101.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 140.90 | 96.61 | 0.00 |  | 0.00 |  | 96.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 139.29 | 95.01 | 0.00 |  | 0.00 |  | 95.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 138.57 | 94.28 | 0.00 |  | 0.00 |  | 94.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 138.30 | 94.01 | 0.00 |  | 0.00 |  | 94.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 138.25 | 93.97 | 0.00 |  | 0.00 |  | 93.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 138.14 | 93.85 | 0.00 |  | 0.00 |  | 93.85 | neither loop stalled long enough: network, hypervisor or untraced |

## boot-stock/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=353671 clocksource=tsc

### Loop counters (boot-stock/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002850 | 419002 | 7.17 | 133631296 | 44.50 | 2227188 | 2227188 | 99.7 | 72.7 | true | 0 | 681 | 545.5 | 133525 / 4213 / 8 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672638400 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (60804 of 2500000 in 1-msg reads) | 60804 | 2.69 | 2.37 | 3.83 | 4.53 | 5.69 | 23.90 | 31.22 |

#### Stalls: boot-stock/md-recv-busy, loop 0 on CPU 1

133631296 iterations, longest 545.5 µs; over 5 / 20 / 100 µs: 133525 / 4213 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 3960 | 112779.2 | 545.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 103 | 2620.4 | 65.1 |
| mostly unexplained (+preempted by kworker/1:1) | 94 | 2310.9 | 44.5 |
| softirq TIMER | 9 | 1151.1 | 301.1 |
| mostly unexplained (+local timer tick) | 20 | 436.4 | 25.2 |
| preempted by kworker/1:1 | 8 | 336.1 | 63.0 |
| mostly unexplained (+softirq NET_RX) | 9 | 237.2 | 38.3 |
| softirq NET_RX | 5 | 121.3 | 27.5 |
| mostly unexplained (+preempted by kworker/1:1H) | 3 | 71.5 | 25.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.8 | 35.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 545.5 | 2850 | loop work (2850 items) | local timer tick 7, timer tick_nohz_handler 4, softirq SCHED 2 |
| 2 | 301.1 | 0 | softirq TIMER | softirq TIMER 291, local timer tick 7, timer tick_nohz_handler 4 |
| 3 | 146.8 | 0 | softirq TIMER | softirq TIMER 126, preempted by kworker/1:1 8, local timer tick 6, timer tick_nohz_handler 4 |
| 4 | 133.9 | 0 | softirq TIMER | softirq TIMER 124, local timer tick 6, timer tick_nohz_handler 4 |
| 5 | 131.8 | 0 | softirq TIMER | softirq TIMER 122, local timer tick 6, timer tick_nohz_handler 4 |
| 6 | 131.0 | 0 | softirq TIMER | softirq TIMER 123, local timer tick 5, timer tick_nohz_handler 3 |
| 7 | 130.9 | 0 | softirq TIMER | softirq TIMER 123, local timer tick 5, timer tick_nohz_handler 3 |
| 8 | 130.4 | 0 | softirq TIMER | softirq TIMER 121, local timer tick 6, timer tick_nohz_handler 3 |
| 9 | 99.3 | 45 | loop work (45 items) |  |
| 10 | 92.0 | 30 | loop work (30 items) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60602 |
| CAL Function call interrupts | 1020 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 6 |
| TLB TLB shootdowns | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## boot-stock/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: boot-stock/noise, loop 0 on CPU 1

8995639296 iterations, longest 147.0 µs; over 5 / 20 / 100 µs: 120240 / 240 / 69

| cause | stalls | total µs | max µs |
|---|---|---|---|
| local timer tick | 119074 | 873050.6 | 28.0 |
| softirq TIMER | 153 | 13372.5 | 147.0 |
| mostly unexplained (+local timer tick) | 623 | 7490.0 | 20.0 |
| mostly unexplained (+preempted by kworker/1:1) | 174 | 3389.1 | 32.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 105 | 1208.4 | 61.1 |
| preempted by kworker/1:1 | 20 | 624.5 | 38.1 |
| mostly unexplained (+irq ena-mgmnt@pci:0000:27:00.0) | 79 | 578.5 | 12.3 |
| mostly unexplained (+preempted by kworker/1:1H) | 4 | 94.9 | 27.2 |
| mostly unexplained (+softirq TIMER) | 5 | 94.1 | 25.8 |
| mostly unexplained (+preempted by migration/1) | 3 | 31.6 | 11.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 147.0 | 0 | softirq TIMER | softirq TIMER 125, preempted by kworker/1:1 10, local timer tick 5, workqueue psi_avgs_work 4 |
| 2 | 146.1 | 0 | softirq TIMER | softirq TIMER 127, preempted by kworker/1:1 8, local timer tick 5, workqueue psi_avgs_work 4 |
| 3 | 144.3 | 0 | softirq TIMER | softirq TIMER 120, preempted by kworker/1:1 13, workqueue psi_avgs_work 7, local timer tick 5 |
| 4 | 143.0 | 0 | softirq TIMER | softirq TIMER 124, preempted by kworker/1:1 8, local timer tick 5, timer tick_nohz_handler 3 |
| 5 | 141.4 | 0 | softirq TIMER | softirq TIMER 120, preempted by kworker/1:1 9, local timer tick 5, workqueue psi_avgs_work 4 |
| 6 | 139.0 | 0 | softirq TIMER | softirq TIMER 124, local timer tick 5, preempted by kworker/1:1 5, timer tick_nohz_handler 3 |
| 7 | 138.8 | 0 | softirq TIMER | softirq TIMER 122, preempted by kworker/1:1 7, local timer tick 5, timer tick_nohz_handler 3 |
| 8 | 137.8 | 0 | softirq TIMER | softirq TIMER 125, local timer tick 5, preempted by kworker/1:1 3, timer tick_nohz_handler 3 |
| 9 | 137.0 | 0 | softirq TIMER | softirq TIMER 120, preempted by kworker/1:1 6, local timer tick 5, timer tick_nohz_handler 3 |
| 10 | 136.7 | 0 | softirq TIMER | softirq TIMER 124, local timer tick 5, preempted by kworker/1:1 4, timer tick_nohz_handler 3 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120224 |
| CAL Function call interrupts | 511 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 120 |
| IWI IRQ work interrupts | 10 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

### Loop counters (boot-stock/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 830478 | 830478 | 1.00 | 486096221 | 585.32 | 8101604 | 8073921 | 0.0 | 45.8 | false | 0 | 352 | 153.9 | 61167 / 102 / 7 | 0 | 0 |

#### Stalls: boot-stock/pp-epoll-spin-0, loop 0 on CPU 1

484435264 iterations, longest 153.9 µs; over 5 / 20 / 100 µs: 61167 / 102 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 47 | 1133.7 | 30.4 |
| softirq TIMER | 7 | 955.1 | 153.9 |
| mostly unexplained (+local timer tick) | 23 | 501.4 | 27.4 |
| preempted by kworker/1:1 | 9 | 352.9 | 54.0 |
| softirq NET_RX | 6 | 184.0 | 45.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 6 | 140.1 | 29.5 |
| mostly unexplained (+preempted by kworker/1:1H) | 3 | 66.5 | 23.6 |
| local timer tick | 1 | 21.2 | 21.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 153.9 | 1 | softirq TIMER | softirq TIMER 124, preempted by kworker/1:1 20, workqueue vmstat_update 18, local timer tick 5 |
| 2 | 153.8 | 1 | softirq TIMER | softirq TIMER 136, local timer tick 6, preempted by kworker/1:1 5, timer tick_nohz_handler 4 |
| 3 | 136.6 | 1 | softirq TIMER | softirq TIMER 125, local timer tick 6, timer tick_nohz_handler 4 |
| 4 | 129.5 | 0 | softirq TIMER | softirq TIMER 121, local timer tick 6, timer tick_nohz_handler 4 |
| 5 | 128.5 | 1 | softirq TIMER | softirq TIMER 117, local timer tick 6, timer tick_nohz_handler 4 |
| 6 | 126.8 | 0 | softirq TIMER | softirq TIMER 118, local timer tick 6, timer tick_nohz_handler 4 |
| 7 | 126.1 | 1 | softirq TIMER | softirq TIMER 115, local timer tick 5, timer tick_nohz_handler 4 |
| 8 | 54.0 | 0 | preempted by kworker/1:1 | preempted by kworker/1:1 42, workqueue vmstat_update 40, local timer tick 6, timer tick_nohz_handler 4 |
| 9 | 46.6 | 0 | preempted by kworker/1:1 | preempted by kworker/1:1 35, workqueue vmstat_update 34, local timer tick 5, timer tick_nohz_handler 4 |
| 10 | 45.2 | 1 | softirq NET_RX | softirq NET_RX 34, irq enp39s0-Tx-Rx-0 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 61034 |
| CAL Function call interrupts | 1008 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 33 |
| TLB TLB shootdowns | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-stock/pp-epoll-spin-0 (median 49.24)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 549.86 | 500.62 | 0.00 |  | 0.00 |  | 500.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 503.01 | 453.78 | 0.00 |  | 0.00 |  | 453.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 500.87 | 451.63 | 0.00 |  | 0.00 |  | 451.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 498.85 | 449.61 | 0.00 |  | 0.00 |  | 449.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 493.62 | 444.38 | 0.00 |  | 0.00 |  | 444.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 493.02 | 443.78 | 0.00 |  | 0.00 |  | 443.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 490.87 | 441.63 | 0.00 |  | 0.00 |  | 441.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 490.51 | 441.27 | 0.00 |  | 0.00 |  | 441.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 486.07 | 436.83 | 0.00 |  | 0.00 |  | 436.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 485.16 | 435.93 | 0.00 |  | 0.00 |  | 435.93 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 498220 | 498220 | 1.00 | 1008894553 | 2025.00 | 8407455 | 8399151 | 0.0 | 44.8 | false | 0 | 128 | 52.5 | 121356 / 44 / 0 | 0 | 0 |

#### Stalls: boot-stock/pp-epoll-spin-10000, loop 0 on CPU 1

1007898112 iterations, longest 52.5 µs; over 5 / 20 / 100 µs: 121356 / 44 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 18 | 398.3 | 25.6 |
| softirq NET_RX | 12 | 318.0 | 34.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 6 | 148.7 | 33.2 |
| softirq TIMER | 2 | 74.3 | 52.5 |
| mostly unexplained (+local timer tick) | 3 | 65.2 | 24.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 50.9 | 50.9 |
| preempted by kworker/1:1 | 1 | 28.9 | 28.9 |
| mostly unexplained (+softirq NET_RX) | 1 | 26.0 | 26.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 52.5 | 0 | softirq TIMER | softirq TIMER 44, local timer tick 6, timer tick_nohz_handler 4 |
| 2 | 50.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 34.8 | 0 | softirq NET_RX | softirq NET_RX 27, irq enp39s0-Tx-Rx-0 1 |
| 4 | 34.6 | 0 | softirq NET_RX | softirq NET_RX 26, irq enp39s0-Tx-Rx-0 1 |
| 5 | 33.9 | 0 | softirq NET_RX | softirq NET_RX 27, irq enp39s0-Tx-Rx-0 1 |
| 6 | 33.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 30.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 28.9 | 0 | preempted by kworker/1:1 | preempted by kworker/1:1 16, workqueue vmstat_update 14, local timer tick 5, timer tick_nohz_handler 4 |
| 9 | 27.2 | 0 | softirq NET_RX | softirq NET_RX 21, irq enp39s0-Tx-Rx-0 1 |
| 10 | 26.8 | 0 | softirq NET_RX | softirq NET_RX 21, irq enp39s0-Tx-Rx-0 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120331 |
| CAL Function call interrupts | 1010 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 93 |
| TLB TLB shootdowns | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-stock/pp-epoll-spin-10000 (median 256.32)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 662.28 | 405.96 | 0.00 |  | 0.00 |  | 405.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 656.64 | 400.32 | 0.00 |  | 0.00 |  | 400.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 636.72 | 380.40 | 0.00 |  | 0.00 |  | 380.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 603.59 | 347.27 | 0.00 |  | 0.00 |  | 347.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 601.76 | 345.43 | 0.00 |  | 0.00 |  | 345.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 599.91 | 343.58 | 0.00 |  | 0.00 |  | 343.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 588.78 | 332.45 | 0.00 |  | 0.00 |  | 332.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 587.63 | 331.31 | 0.00 |  | 0.00 |  | 331.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 577.85 | 321.53 | 0.00 |  | 0.00 |  | 321.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 577.28 | 320.96 | 0.00 |  | 0.00 |  | 320.96 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1304713 | 1304713 | 1.00 | 123451338 | 94.62 | 2057522 | 2035777 | 98.9 | 79.0 | true | 0 | 4974 | 151.1 | 317652 / 166 / 15 | 0 | 0 |

#### Stalls: boot-stock/pp-recv-busy-0, loop 0 on CPU 1

122146624 iterations, longest 151.1 µs; over 5 / 20 / 100 µs: 317652 / 166 / 15

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 119 | 4653.1 | 151.1 |
| mostly unexplained (+preempted by kworker/1:1) | 24 | 599.7 | 44.5 |
| softirq TIMER | 2 | 278.6 | 139.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 8 | 235.2 | 46.6 |
| mostly unexplained (+local timer tick) | 8 | 183.1 | 28.1 |
| preempted by kworker/1:1 | 3 | 94.6 | 40.6 |
| softirq NET_RX | 1 | 22.8 | 22.8 |
| local timer tick | 1 | 20.3 | 20.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 151.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 148.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 147.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 147.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 144.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 142.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 142.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 141.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 140.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 140.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 1231691 |
| LOC Local timer interrupts | 60541 |
| CAL Function call interrupts | 507 |
| TLB TLB shootdowns | 7 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-stock/pp-recv-busy-0 (median 45.19)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 188.57 | 143.38 | 142.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.98 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 181.06 | 135.87 | 136.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 180.36 | 135.17 | 150.85 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 177.69 | 132.50 | 138.78 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 5 | 175.30 | 130.11 | 147.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 166.93 | 121.74 | 141.56 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 166.71 | 121.52 | 140.42 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 165.43 | 120.24 | 140.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 157.18 | 111.98 | 136.24 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 10 | 153.54 | 108.35 | 139.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (boot-stock/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1197056 | 1197056 | 1.00 | 278314177 | 232.50 | 2319285 | 2309309 | 99.6 | 75.9 | true | 0 | 2340 | 437.1 | 126559 / 125 / 12 | 0 | 0 |

#### Stalls: boot-stock/pp-recv-busy-10000, loop 0 on CPU 1

277117120 iterations, longest 437.1 µs; over 5 / 20 / 100 µs: 126559 / 125 / 12

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 11 | 1376.0 | 169.9 |
| mostly unexplained (+local timer tick) | 21 | 921.6 | 437.1 |
| mostly unexplained (+preempted by kworker/1:1) | 38 | 878.5 | 37.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 12 | 553.3 | 84.8 |
| preempted by kworker/1:1 | 15 | 463.3 | 39.7 |
| softirq NET_RX | 14 | 383.4 | 39.3 |
| mostly unexplained (+irq enp39s0-Tx-Rx-0) | 1 | 261.4 | 261.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 9 | 219.9 | 38.7 |
| mostly unexplained (+preempted by kworker/1:1H) | 3 | 71.6 | 26.9 |
| mostly unexplained (+softirq NET_RX) | 1 | 20.3 | 20.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 437.1 | 0 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 3, irq ena-mgmnt@pci:0000:27:00.0 1, vector_alloc 0 |
| 2 | 261.4 | 0 | mostly unexplained (+irq enp39s0-Tx-Rx-0) | irq enp39s0-Tx-Rx-0 1, ipi_send_cpu 0 |
| 3 | 169.9 | 0 | softirq TIMER | softirq TIMER 137, preempted by kworker/1:1 22, workqueue psi_avgs_work 19, local timer tick 5 |
| 4 | 137.8 | 0 | softirq TIMER | softirq TIMER 128, local timer tick 6, timer tick_nohz_handler 3, ipi_send_cpu 0 |
| 5 | 137.2 | 0 | softirq TIMER | softirq TIMER 126, local timer tick 8, timer tick_nohz_handler 4, ipi_send_cpu 0 |
| 6 | 136.9 | 0 | softirq TIMER | softirq TIMER 126, local timer tick 6, timer tick_nohz_handler 4, ipi_send_cpu 0 |
| 7 | 135.1 | 0 | softirq TIMER | softirq TIMER 126, local timer tick 6, timer tick_nohz_handler 4, ipi_send_cpu 0 |
| 8 | 132.7 | 0 | softirq TIMER | softirq TIMER 125, local timer tick 5, timer tick_nohz_handler 4 |
| 9 | 129.8 | 0 | softirq TIMER | softirq TIMER 120, local timer tick 6, timer tick_nohz_handler 4 |
| 10 | 127.2 | 0 | softirq TIMER | softirq TIMER 118, local timer tick 6, timer tick_nohz_handler 4, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120545 |
| CAL Function call interrupts | 967 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 206 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 40 |
| IWI IRQ work interrupts | 10 |
| TLB TLB shootdowns | 5 |

#### Slowest round trips: boot-stock/pp-recv-busy-10000 (median 42.09)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1047.96 | 1005.88 | 0.00 |  | 0.00 |  | 1005.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 858.50 | 816.41 | 0.00 |  | 0.00 |  | 816.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 771.41 | 729.33 | 0.00 |  | 0.00 |  | 729.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 744.89 | 702.80 | 0.00 |  | 0.00 |  | 702.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 714.68 | 672.59 | 0.00 |  | 0.00 |  | 672.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 696.26 | 654.18 | 0.00 |  | 0.00 |  | 654.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 626.39 | 584.30 | 0.00 |  | 0.00 |  | 584.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 589.79 | 547.71 | 0.00 |  | 0.00 |  | 547.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 575.66 | 533.57 | 0.00 |  | 0.00 |  | 533.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 572.43 | 530.34 | 0.00 |  | 0.00 |  | 530.34 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1052710 | 1052710 | 1.00 | 3158131 | 3.00 | 52636 | 45988666 | 0.0 | 3.0 | false | 0 | 41 | 81.8 | 61022 / 32 / 0 | 0 | 0 |

#### Stalls: boot-stock/pp-uring-spin-0, loop 0 on CPU 1

2759319936 iterations, longest 81.8 µs; over 5 / 20 / 100 µs: 61022 / 32 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| preempted by kworker/1:1 | 7 | 334.1 | 73.8 |
| mostly unexplained (+local timer tick) | 12 | 267.4 | 26.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 6 | 144.4 | 34.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 117.9 | 81.8 |
| mostly unexplained (+preempted by kworker/1:1) | 3 | 67.8 | 23.3 |
| softirq NET_RX | 1 | 29.5 | 29.5 |
| mostly unexplained (+preempted by migration/1) | 1 | 24.9 | 24.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 81.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 73.8 | 0 | preempted by kworker/1:1 | preempted by kworker/1:1 55, workqueue vmstat_update 51, local timer tick 6, softirq TIMER 4 |
| 3 | 54.5 | 0 | preempted by kworker/1:1 | preempted by kworker/1:1 35, workqueue vmstat_update 28, local timer tick 6, softirq TIMER 5 |
| 4 | 50.5 | 0 | preempted by kworker/1:1 | preempted by kworker/1:1 33, workqueue vmstat_update 30, local timer tick 5, softirq TIMER 4 |
| 5 | 48.4 | 0 | preempted by kworker/1:1 | preempted by kworker/1:1 31, workqueue vmstat_update 27, local timer tick 6, softirq TIMER 5 |
| 6 | 41.3 | 0 | preempted by kworker/1:1 | preempted by kworker/1:1 26, workqueue vmstat_update 23, local timer tick 6, timer tick_nohz_handler 4 |
| 7 | 36.6 | 0 | preempted by kworker/1:1 | preempted by kworker/1:1 23, workqueue vmstat_update 20, local timer tick 6, timer tick_nohz_handler 4 |
| 8 | 36.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 34.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 29.5 | 0 | softirq NET_RX | softirq NET_RX 20, irq enp39s0-Tx-Rx-0 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60360 |
| CAL Function call interrupts | 1014 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 27 |
| TLB TLB shootdowns | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-stock/pp-uring-spin-0 (median 48.73)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 494.21 | 445.47 | 0.00 |  | 0.00 |  | 445.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 488.83 | 440.10 | 0.00 |  | 0.00 |  | 440.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 483.37 | 434.64 | 0.00 |  | 0.00 |  | 434.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 470.47 | 421.74 | 0.00 |  | 0.00 |  | 421.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 465.14 | 416.41 | 0.00 |  | 0.00 |  | 416.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 462.02 | 413.29 | 0.00 |  | 0.00 |  | 413.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 460.85 | 412.11 | 0.00 |  | 0.00 |  | 412.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 460.20 | 411.47 | 0.00 |  | 0.00 |  | 411.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 455.42 | 406.68 | 0.00 |  | 0.00 |  | 406.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 454.79 | 406.05 | 0.00 |  | 0.00 |  | 406.05 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 756793 | 756793 | 1.00 | 2270380 | 3.00 | 18920 | 42649649 | 0.0 | 3.2 | false | 0 | 94 | 107.6 | 122730 / 59 / 1 | 0 | 0 |

#### Stalls: boot-stock/pp-uring-spin-10000, loop 0 on CPU 1

5117957824 iterations, longest 107.6 µs; over 5 / 20 / 100 µs: 122730 / 59 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 31 | 1075.9 | 107.6 |
| mostly unexplained (+preempted by kworker/1:1) | 10 | 230.1 | 27.0 |
| softirq NET_RX | 8 | 214.1 | 33.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 5 | 132.5 | 38.8 |
| mostly unexplained (+local timer tick) | 4 | 87.0 | 23.7 |
| mostly unexplained (+irq enp39s0-Tx-Rx-0) | 1 | 20.3 | 20.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 107.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 62.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 59.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 58.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 52.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 47.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 47.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 38.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 38.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 38.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 757005 |
| LOC Local timer interrupts | 120644 |
| CAL Function call interrupts | 1055 |
| TLB TLB shootdowns | 8 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-stock/pp-uring-spin-10000 (median 51.56)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 5116.32 | 5064.76 | 0.00 |  | 0.00 |  | 5064.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 533.87 | 482.31 | 0.00 |  | 0.00 |  | 482.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 527.80 | 476.24 | 0.00 |  | 0.00 |  | 476.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 521.19 | 469.63 | 0.00 |  | 0.00 |  | 469.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 519.86 | 468.30 | 0.00 |  | 0.00 |  | 468.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 518.42 | 466.86 | 0.00 |  | 0.00 |  | 466.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 518.26 | 466.70 | 0.00 |  | 0.00 |  | 466.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 513.33 | 461.77 | 0.00 |  | 0.00 |  | 461.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 512.29 | 460.73 | 0.00 |  | 0.00 |  | 460.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 511.10 | 459.53 | 0.00 |  | 0.00 |  | 459.53 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1295811 | 1295811 | 1.00 | 1 | 0.00 | 0 | 18263322 | 0.0 | 2.6 | true | 0 | 4479 | 62.8 | 84994 / 34 / 0 | 0 | 0 |

#### Stalls: boot-stock/pp-uring-sqpoll-0, loop 0 on CPU 1

1095799296 iterations, longest 62.8 µs; over 5 / 20 / 100 µs: 84994 / 34 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 25 | 731.9 | 62.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 79.7 | 33.3 |
| mostly unexplained (+softirq NET_RX) | 2 | 58.1 | 31.6 |
| mostly unexplained (+local timer tick) | 2 | 44.4 | 23.2 |
| mostly unexplained (+preempted by kworker/1:1) | 1 | 22.2 | 22.2 |
| local timer tick | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 62.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 61.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 61.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 58.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 43.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 33.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 31.6 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, local timer tick 10, timer tick_nohz_handler 6, irq enp39s0-Tx-Rx-0 3 |
| 8 | 26.5 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 11, local timer tick 9, timer tick_nohz_handler 5, irq enp39s0-Tx-Rx-0 1 |
| 9 | 25.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 25.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 1263435 |
| LOC Local timer interrupts | 60488 |
| CAL Function call interrupts | 996 |
| TLB TLB shootdowns | 5 |
| RES Rescheduling interrupts | 2 |
| IWI IRQ work interrupts | 1 |

#### Slowest round trips: boot-stock/pp-uring-sqpoll-0 (median 45.41)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 342.18 | 296.77 | 0.00 |  | 26.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 270.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 151.01 | 105.60 | 0.00 |  | 0.00 |  | 105.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 149.42 | 104.01 | 0.00 |  | 0.00 |  | 104.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 136.31 | 90.90 | 0.00 |  | 0.00 |  | 90.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 134.97 | 89.56 | 0.00 |  | 0.00 |  | 89.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 133.42 | 88.01 | 0.00 |  | 0.00 |  | 88.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 132.46 | 87.05 | 0.00 |  | 0.00 |  | 87.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 131.88 | 86.47 | 0.00 |  | 0.00 |  | 86.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 130.67 | 85.26 | 0.00 |  | 0.00 |  | 85.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 129.74 | 84.33 | 0.00 |  | 0.00 |  | 84.33 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1194136 | 1194136 | 1.00 | 1 | 0.00 | 0 | 13496090 | 0.0 | 0.0 | true | 0 | 163 | 64.7 | 120334 / 85 / 0 | 0 | 0 |

#### Stalls: boot-stock/pp-uring-sqpoll-10000, loop 0 on CPU 1

1619530752 iterations, longest 64.7 µs; over 5 / 20 / 100 µs: 120334 / 85 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 35 | 820.0 | 31.9 |
| softirq NET_RX | 16 | 500.7 | 47.6 |
| mostly unexplained (+local timer tick) | 16 | 356.9 | 25.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 354.7 | 64.7 |
| preempted by kworker/1:1 | 4 | 159.3 | 49.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 64.2 | 32.1 |
| mostly unexplained (+softirq RCU) | 1 | 26.4 | 26.4 |
| mostly unexplained (+preempted by kworker/1:1H) | 1 | 23.5 | 23.5 |
| local timer tick | 1 | 20.9 | 20.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 64.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 62.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 58.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 58.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 49.1 | 0 | preempted by kworker/1:1 | preempted by kworker/1:1 37, workqueue vmstat_update 33, local timer tick 6, timer tick_nohz_handler 4 |
| 6 | 47.6 | 0 | softirq NET_RX | softirq NET_RX 41, local timer tick 7, timer tick_nohz_handler 4, irq enp39s0-Tx-Rx-0 1 |
| 7 | 47.5 | 0 | preempted by kworker/1:1 | preempted by kworker/1:1 30, workqueue vmstat_update 26, local timer tick 7, softirq TIMER 4 |
| 8 | 44.4 | 0 | softirq NET_RX | softirq NET_RX 31, local timer tick 8, timer tick_nohz_handler 7, irq enp39s0-Tx-Rx-0 1 |
| 9 | 40.2 | 0 | softirq NET_RX | softirq NET_RX 34, irq enp39s0-Tx-Rx-0 1 |
| 10 | 39.6 | 1 | softirq NET_RX | softirq NET_RX 32, irq enp39s0-Tx-Rx-0 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120399 |
| CAL Function call interrupts | 1004 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 158 |
| TLB TLB shootdowns | 3 |

#### Slowest round trips: boot-stock/pp-uring-sqpoll-10000 (median 44.59)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 353.90 | 309.31 | 0.00 |  | 0.00 |  | 309.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 348.45 | 303.85 | 0.00 |  | 0.00 |  | 303.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 342.78 | 298.19 | 0.00 |  | 0.00 |  | 298.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 340.66 | 296.07 | 0.00 |  | 0.00 |  | 296.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 331.07 | 286.47 | 0.00 |  | 0.00 |  | 286.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 329.80 | 285.21 | 0.00 |  | 0.00 |  | 285.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 329.45 | 284.86 | 0.00 |  | 0.00 |  | 284.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 329.45 | 284.86 | 0.00 |  | 0.00 |  | 284.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 323.90 | 279.31 | 0.00 |  | 0.00 |  | 279.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 322.95 | 278.36 | 0.00 |  | 0.00 |  | 278.36 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: boot-isolcpus/http-recv-busy-10000, loop 0 on CPU 1

1858393699 iterations, longest 62.3 µs; over 5 / 20 / 100 µs: 206259 / 3596 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2429 | 57814.7 | 62.3 |
| mostly unexplained (+local timer tick) | 534 | 14067.2 | 42.1 |
| mostly unexplained (+softirq NET_RX) | 574 | 13836.4 | 37.7 |
| mostly unexplained (+preempted by kworker/1:1) | 44 | 1176.5 | 45.9 |
| softirq NET_RX | 9 | 230.3 | 35.3 |
| mostly unexplained (+preempted by migration/1) | 3 | 64.2 | 22.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 61.5 | 33.7 |
| loop work | 1 | 22.4 | 22.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 62.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 57.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 56.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 55.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 45.9 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, local timer tick 5, softirq NET_RX 5, timer tick_nohz_handler 3 |
| 6 | 42.1 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, timer tick_nohz_handler 4, softirq TIMER 1 |
| 7 | 40.5 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 4 |
| 8 | 40.5 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, timer tick_nohz_handler 3, softirq NET_RX 1 |
| 9 | 39.9 | 1 | mostly unexplained (+local timer tick) | local timer tick 1 |
| 10 | 39.5 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 3 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120111 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: boot-isolcpus/http-recv-busy-10000 (median 46.14)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 166.57 | 120.43 | 0.00 |  | 148.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 165.32 | 119.18 | 0.00 |  | 0.00 |  | 119.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 152.22 | 106.08 | 0.00 |  | 55.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 50.84 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 139.20 | 93.05 | 0.00 |  | 0.00 |  | 93.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 136.51 | 90.37 | 0.00 |  | 128.98 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 129.92 | 83.78 | 0.00 |  | 0.00 |  | 83.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 129.92 | 83.77 | 0.00 |  | 0.00 |  | 83.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 128.73 | 82.58 | 0.00 |  | 0.00 |  | 82.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 123.86 | 77.72 | 0.00 |  | 0.00 |  | 77.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 117.09 | 70.95 | 0.00 |  | 0.00 |  | 70.95 | neither loop stalled long enough: network, hypervisor or untraced |

## boot-isolcpus/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=505040 clocksource=tsc

### Loop counters (boot-isolcpus/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002750 | 547296 | 5.49 | 132506752 | 44.13 | 2208446 | 2208446 | 99.6 | 72.3 | true | 0 | 212 | 555.3 | 142261 / 5023 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (71986 of 2500000 in 1-msg reads) | 71986 | 2.41 | 2.23 | 2.66 | 4.29 | 19.50 | 23.34 | 27.78 |

#### Stalls: boot-isolcpus/md-recv-busy, loop 0 on CPU 1

132506752 iterations, longest 555.3 µs; over 5 / 20 / 100 µs: 142261 / 5023 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4869 | 133068.3 | 555.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 143 | 3302.3 | 28.8 |
| mostly unexplained (+softirq NET_RX) | 4 | 95.4 | 24.8 |
| softirq TIMER | 2 | 86.6 | 65.4 |
| mostly unexplained (+preempted by kworker/1:1) | 2 | 59.5 | 30.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.8 | 33.4 |
| mostly unexplained (+softirq TIMER) | 1 | 20.3 | 20.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 555.3 | 2750 | loop work (2750 items) | local timer tick 7, timer tick_nohz_handler 5 |
| 2 | 65.4 | 0 | softirq TIMER | softirq TIMER 53, local timer tick 7, timer tick_nohz_handler 5 |
| 3 | 52.0 | 15 | loop work (15 items) |  |
| 4 | 46.1 | 22 | loop work (22 items) |  |
| 5 | 44.0 | 35 | loop work (35 items) |  |
| 6 | 43.0 | 12 | loop work (12 items) |  |
| 7 | 42.1 | 35 | loop work (35 items) | softirq NET_RX 7 |
| 8 | 41.8 | 21 | loop work (21 items) |  |
| 9 | 41.2 | 11 | loop work (11 items) |  |
| 10 | 40.9 | 24 | loop work (24 items) | preempted by kworker/1:1 3, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60106 |
| CAL Function call interrupts | 989 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## boot-isolcpus/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: boot-isolcpus/noise, loop 0 on CPU 1

8993116160 iterations, longest 52.3 µs; over 5 / 20 / 100 µs: 120127 / 8 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| local timer tick | 119984 | 878466.9 | 12.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 96 | 1036.8 | 52.3 |
| mostly unexplained (+local timer tick) | 46 | 470.1 | 16.6 |
| mostly unexplained (+preempted by migration/1) | 1 | 12.4 | 12.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 52.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 41.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 38.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 34.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 30.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 26.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 25.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 21.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 16.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 16.6 | 0 | mostly unexplained (+local timer tick) | local timer tick 5, preempted by migration/1 5, timer watchdog_timer_fn 3 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120078 |
| CAL Function call interrupts | 998 |

### Loop counters (boot-isolcpus/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1119146 | 1119146 | 1.00 | 468886229 | 418.97 | 7814770 | 7777466 | 0.0 | 47.7 | false | 0 | 15 | 55.9 | 103867 / 26 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-epoll-spin-0, loop 0 on CPU 1

466647936 iterations, longest 55.9 µs; over 5 / 20 / 100 µs: 103867 / 26 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+local timer tick) | 12 | 263.8 | 29.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 7 | 199.2 | 55.9 |
| mostly unexplained (+preempted by migration/1) | 4 | 88.5 | 24.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 83.6 | 38.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 55.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 38.5 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 36.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 29.3 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, timer tick_nohz_handler 4, softirq TIMER 1 |
| 5 | 24.2 | 0 | mostly unexplained (+local timer tick) | local timer tick 10, timer watchdog_timer_fn 8, preempted by migration/1 5 |
| 6 | 24.0 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 24.0 | 1 | mostly unexplained (+preempted by migration/1) | preempted by migration/1 7, local timer tick 4, timer watchdog_timer_fn 2 |
| 8 | 22.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 22.2 | 0 | mostly unexplained (+preempted by migration/1) | preempted by migration/1 7, local timer tick 6, timer watchdog_timer_fn 5 |
| 10 | 22.1 | 0 | mostly unexplained (+preempted by migration/1) | preempted by migration/1 7, local timer tick 6, timer watchdog_timer_fn 4 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60231 |
| CAL Function call interrupts | 989 |

#### Slowest round trips: boot-isolcpus/pp-epoll-spin-0 (median 50.57)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 510.03 | 459.46 | 0.00 |  | 0.00 |  | 459.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 468.54 | 417.97 | 0.00 |  | 0.00 |  | 417.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 406.44 | 355.88 | 0.00 |  | 0.00 |  | 355.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 399.25 | 348.69 | 0.00 |  | 0.00 |  | 348.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 386.85 | 336.28 | 0.00 |  | 0.00 |  | 336.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 384.96 | 334.40 | 0.00 |  | 0.00 |  | 334.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 383.62 | 333.06 | 0.00 |  | 0.00 |  | 333.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 376.75 | 326.18 | 0.00 |  | 0.00 |  | 326.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 359.81 | 309.25 | 0.00 |  | 0.00 |  | 309.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 355.83 | 305.26 | 0.00 |  | 0.00 |  | 305.26 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1112519 | 1112519 | 1.00 | 922510287 | 829.21 | 7687586 | 7669044 | 0.0 | 44.2 | false | 0 | 30 | 65.0 | 178924 / 27 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-epoll-spin-10000, loop 0 on CPU 1

920285248 iterations, longest 65.0 µs; over 5 / 20 / 100 µs: 178924 / 27 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+local timer tick) | 15 | 319.6 | 24.0 |
| mostly unexplained (+preempted by migration/1) | 6 | 140.6 | 31.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 82.3 | 56.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 75.6 | 30.1 |
| softirq TIMER | 1 | 65.0 | 65.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 65.0 | 0 | softirq TIMER | softirq TIMER 53, local timer tick 7, timer tick_nohz_handler 5 |
| 2 | 56.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 31.5 | 0 | mostly unexplained (+preempted by migration/1) | preempted by migration/1 11, local timer tick 10, timer watchdog_timer_fn 8 |
| 4 | 30.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 26.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 25.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 24.1 | 0 | mostly unexplained (+preempted by migration/1) | preempted by migration/1 7, local timer tick 5, timer watchdog_timer_fn 4 |
| 8 | 24.0 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 4 |
| 9 | 23.1 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, timer tick_nohz_handler 5, softirq TIMER 1 |
| 10 | 22.9 | 0 | mostly unexplained (+preempted by migration/1) | preempted by migration/1 7, local timer tick 6, timer watchdog_timer_fn 4 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120184 |
| CAL Function call interrupts | 1002 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-isolcpus/pp-epoll-spin-10000 (median 52.47)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 538.73 | 486.25 | 0.00 |  | 0.00 |  | 486.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 513.73 | 461.26 | 0.00 |  | 0.00 |  | 461.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 506.48 | 454.01 | 0.00 |  | 0.00 |  | 454.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 505.71 | 453.23 | 0.00 |  | 0.00 |  | 453.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 504.74 | 452.26 | 0.00 |  | 0.00 |  | 452.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 504.56 | 452.08 | 0.00 |  | 0.00 |  | 452.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 502.49 | 450.02 | 0.00 |  | 0.00 |  | 450.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 497.91 | 445.43 | 0.00 |  | 0.00 |  | 445.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 496.11 | 443.64 | 0.00 |  | 0.00 |  | 443.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 495.99 | 443.52 | 0.00 |  | 0.00 |  | 443.52 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1298471 | 1298471 | 1.00 | 130397544 | 100.42 | 2173292 | 2151651 | 99.0 | 79.0 | true | 0 | 2851 | 68.5 | 90377 / 22 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-recv-busy-0, loop 0 on CPU 1

129099072 iterations, longest 68.5 µs; over 5 / 20 / 100 µs: 90377 / 22 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 245.8 | 58.8 |
| mostly unexplained (+local timer tick) | 7 | 154.9 | 24.8 |
| softirq TIMER | 1 | 68.5 | 68.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 61.8 | 40.0 |
| mostly unexplained (+preempted by kworker/1:1) | 2 | 45.7 | 23.9 |
| mostly unexplained (+preempted by migration/1) | 1 | 20.2 | 20.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 68.5 | 1 | softirq TIMER | softirq TIMER 53, local timer tick 8, timer tick_nohz_handler 6 |
| 2 | 58.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 40.0 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 31.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 26.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 24.8 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, preempted by kworker/1:1 5, timer tick_nohz_handler 5, workqueue [ena] 1 |
| 7 | 23.9 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, local timer tick 5, timer tick_nohz_handler 4, softirq NET_RX 3 |
| 8 | 23.6 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, preempted by kworker/1:1 5, timer tick_nohz_handler 4, softirq NET_RX 3 |
| 9 | 23.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 23.1 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 4 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60236 |
| CAL Function call interrupts | 990 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-isolcpus/pp-recv-busy-0 (median 45.35)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 984.62 | 939.26 | 0.00 |  | 0.00 |  | 939.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 265.43 | 220.07 | 0.00 |  | 0.00 |  | 220.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 265.39 | 220.04 | 0.00 |  | 221.37 | softirq TIMER | 0.00 | server: softirq TIMER |
| 4 | 217.85 | 172.50 | 0.00 |  | 0.00 |  | 172.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 207.47 | 162.12 | 0.00 |  | 0.00 |  | 162.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 199.40 | 154.05 | 0.00 |  | 0.00 |  | 154.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 197.86 | 152.51 | 0.00 |  | 0.00 |  | 152.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 197.24 | 151.89 | 0.00 |  | 0.00 |  | 151.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 188.19 | 142.84 | 0.00 |  | 0.00 |  | 142.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 172.93 | 127.58 | 0.00 |  | 0.00 |  | 127.58 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1184768 | 1184768 | 1.00 | 275298432 | 232.36 | 2294154 | 2284281 | 99.6 | 76.3 | true | 0 | 2670 | 36.8 | 141340 / 27 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-recv-busy-10000, loop 0 on CPU 1

274113664 iterations, longest 36.8 µs; over 5 / 20 / 100 µs: 141340 / 27 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+local timer tick) | 16 | 360.7 | 28.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 7 | 170.4 | 35.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 64.2 | 36.8 |
| mostly unexplained (+softirq NET_RX) | 2 | 44.2 | 23.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 36.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 35.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 28.5 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 3, softirq TIMER 1 |
| 4 | 27.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 26.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 24.4 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 3, softirq RCU 1 |
| 7 | 24.2 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, timer tick_nohz_handler 4 |
| 8 | 23.9 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, timer tick_nohz_handler 3 |
| 9 | 23.6 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 3 |
| 10 | 23.4 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, timer tick_nohz_handler 4, softirq RCU 1, softirq TIMER 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120179 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: boot-isolcpus/pp-recv-busy-10000 (median 45.37)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 359.96 | 314.58 | 0.00 |  | 0.00 |  | 314.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 359.62 | 314.25 | 0.00 |  | 0.00 |  | 314.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 358.61 | 313.23 | 0.00 |  | 0.00 |  | 313.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 357.33 | 311.95 | 0.00 |  | 0.00 |  | 311.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 354.05 | 308.67 | 0.00 |  | 0.00 |  | 308.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 351.31 | 305.93 | 0.00 |  | 0.00 |  | 305.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 350.06 | 304.68 | 0.00 |  | 0.00 |  | 304.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 349.14 | 303.77 | 0.00 |  | 0.00 |  | 303.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 348.68 | 303.31 | 0.00 |  | 0.00 |  | 303.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 346.62 | 301.24 | 0.00 |  | 0.00 |  | 301.24 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1126571 | 1126571 | 1.00 | 3379714 | 3.00 | 56329 | 44795543 | 0.0 | 7.2 | false | 0 | 15 | 44.4 | 91827 / 25 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-uring-spin-0, loop 0 on CPU 1

2687732608 iterations, longest 44.4 µs; over 5 / 20 / 100 µs: 91827 / 25 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 12 | 317.0 | 44.4 |
| mostly unexplained (+local timer tick) | 10 | 226.4 | 26.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 63.5 | 43.1 |
| local timer tick | 1 | 21.9 | 21.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 44.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 43.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 29.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 28.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 28.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 26.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 26.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 26.5 | 0 | mostly unexplained (+local timer tick) | local timer tick 9, preempted by migration/1 8, timer watchdog_timer_fn 7 |
| 9 | 25.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 7, preempted by migration/1 6, timer watchdog_timer_fn 4 |
| 10 | 24.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60235 |
| CAL Function call interrupts | 992 |

#### Slowest round trips: boot-isolcpus/pp-uring-spin-0 (median 48.58)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1061.57 | 1012.99 | 0.00 |  | 0.00 |  | 1012.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 638.34 | 589.75 | 0.00 |  | 0.00 |  | 589.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 570.99 | 522.41 | 0.00 |  | 0.00 |  | 522.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 529.51 | 480.93 | 0.00 |  | 0.00 |  | 480.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 522.47 | 473.88 | 0.00 |  | 0.00 |  | 473.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 520.50 | 471.92 | 0.00 |  | 0.00 |  | 471.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 514.72 | 466.14 | 0.00 |  | 0.00 |  | 466.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 511.11 | 462.53 | 0.00 |  | 0.00 |  | 462.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 510.33 | 461.75 | 0.00 |  | 0.00 |  | 461.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 510.31 | 461.73 | 0.00 |  | 0.00 |  | 461.73 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 880177 | 880177 | 1.00 | 2640532 | 3.00 | 22004 | 40914805 | 0.0 | 3.8 | false | 0 | 30 | 65.6 | 290310 / 25 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-uring-spin-10000, loop 0 on CPU 1

4909776640 iterations, longest 65.6 µs; over 5 / 20 / 100 µs: 290310 / 25 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 283.5 | 57.7 |
| mostly unexplained (+local timer tick) | 12 | 265.4 | 26.1 |
| softirq TIMER | 1 | 65.6 | 65.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 58.2 | 33.9 |
| mostly unexplained (+preempted by migration/1) | 2 | 44.7 | 23.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 65.6 | 0 | softirq TIMER | softirq TIMER 50, local timer tick 6, timer tick_nohz_handler 4 |
| 2 | 57.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 49.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 37.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 35.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 33.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 33.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 26.1 | 0 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 4 |
| 9 | 25.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 24.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 6, preempted by migration/1 6, timer watchdog_timer_fn 4 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120171 |
| CAL Function call interrupts | 1002 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-isolcpus/pp-uring-spin-10000 (median 55.79)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1101.91 | 1046.12 | 0.00 |  | 0.00 |  | 1046.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1044.99 | 989.20 | 0.00 |  | 0.00 |  | 989.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 626.22 | 570.42 | 0.00 |  | 0.00 |  | 570.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 620.45 | 564.65 | 0.00 |  | 0.00 |  | 564.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 600.54 | 544.75 | 0.00 |  | 0.00 |  | 544.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 596.27 | 540.48 | 0.00 |  | 0.00 |  | 540.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 593.02 | 537.23 | 0.00 |  | 0.00 |  | 537.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 583.51 | 527.71 | 0.00 |  | 0.00 |  | 527.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 582.93 | 527.14 | 0.00 |  | 0.00 |  | 527.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 579.14 | 523.35 | 0.00 |  | 0.00 |  | 523.35 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1273199 | 1273199 | 1.00 | 1 | 0.00 | 0 | 20650866 | 0.0 | 0.0 | true | 0 | 15 | 35.6 | 60054 / 3 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-uring-sqpoll-0, loop 0 on CPU 1

1239051968 iterations, longest 35.6 µs; over 5 / 20 / 100 µs: 60054 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 56.1 | 35.6 |
| mostly unexplained (+local timer tick) | 1 | 25.8 | 25.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 35.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 25.8 | 0 | mostly unexplained (+local timer tick) | local timer tick 9, preempted by migration/1 8, timer watchdog_timer_fn 7 |
| 3 | 20.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60237 |
| CAL Function call interrupts | 992 |

#### Slowest round trips: boot-isolcpus/pp-uring-sqpoll-0 (median 46.80)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 181.53 | 134.72 | 0.00 |  | 144.74 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 151.61 | 104.81 | 0.00 |  | 0.00 |  | 104.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 150.50 | 103.70 | 0.00 |  | 0.00 |  | 103.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 147.14 | 100.34 | 0.00 |  | 0.00 |  | 100.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 146.38 | 99.58 | 0.00 |  | 0.00 |  | 99.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 146.35 | 99.55 | 0.00 |  | 0.00 |  | 99.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 145.58 | 98.78 | 0.00 |  | 0.00 |  | 98.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 141.63 | 94.82 | 0.00 |  | 0.00 |  | 94.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 140.10 | 93.29 | 0.00 |  | 0.00 |  | 93.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 136.87 | 90.07 | 0.00 |  | 0.00 |  | 90.07 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199036 | 1199036 | 1.00 | 1 | 0.00 | 0 | 13935983 | 0.0 | 0.0 | true | 0 | 30 | 91.1 | 120120 / 6 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-uring-sqpoll-10000, loop 0 on CPU 1

1672318016 iterations, longest 91.1 µs; over 5 / 20 / 100 µs: 120120 / 6 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 248.2 | 91.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 65.1 | 36.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 91.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 54.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 53.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 48.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 36.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 28.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120178 |
| CAL Function call interrupts | 1002 |

#### Slowest round trips: boot-isolcpus/pp-uring-sqpoll-10000 (median 45.61)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 339.15 | 293.55 | 0.00 |  | 0.00 |  | 293.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 286.11 | 240.50 | 0.00 |  | 0.00 |  | 240.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 271.88 | 226.27 | 0.00 |  | 0.00 |  | 226.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 244.18 | 198.57 | 0.00 |  | 0.00 |  | 198.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 233.29 | 187.69 | 0.00 |  | 0.00 |  | 187.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 229.76 | 184.16 | 0.00 |  | 0.00 |  | 184.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 227.18 | 181.57 | 0.00 |  | 0.00 |  | 181.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 216.35 | 170.74 | 0.00 |  | 0.00 |  | 170.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 216.34 | 170.73 | 0.00 |  | 0.00 |  | 170.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 191.03 | 145.43 | 0.00 |  | 0.00 |  | 145.43 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: boot-full-nomitig/http-recv-busy-10000, loop 0 on CPU 1

1851470368 iterations, longest 58.0 µs; over 5 / 20 / 100 µs: 49816 / 2240 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1194 | 28114.4 | 58.0 |
| mostly unexplained (+preempted by kworker/1:1) | 642 | 15404.2 | 44.7 |
| mostly unexplained (+softirq NET_RX) | 292 | 6997.2 | 41.0 |
| softirq NET_RX | 81 | 1849.6 | 27.9 |
| mostly unexplained (+irq work (IPI/vector)) | 26 | 624.0 | 32.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 91.7 | 40.6 |
| loop work | 2 | 52.9 | 32.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 58.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 51.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 44.7 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 44.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 41.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 41.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 40.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 40.2 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 39.9 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 39.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3348 |
| IWI IRQ work interrupts | 3340 |
| CAL Function call interrupts | 978 |

#### Slowest round trips: boot-full-nomitig/http-recv-busy-10000 (median 46.10)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1045.66 | 999.56 | 0.00 |  | 0.00 |  | 999.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 251.98 | 205.88 | 0.00 |  | 0.00 |  | 205.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 205.58 | 159.48 | 0.00 |  | 0.00 |  | 159.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 195.44 | 149.34 | 0.00 |  | 0.00 |  | 149.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 193.78 | 147.68 | 0.00 |  | 0.00 |  | 147.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 179.61 | 133.51 | 0.00 |  | 0.00 |  | 133.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 174.91 | 128.81 | 0.00 |  | 0.00 |  | 128.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 157.95 | 111.85 | 0.00 |  | 27.93 | unexplained: no kernel event (hypervisor or in-kernel time) | 83.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 154.15 | 108.05 | 0.00 |  | 22.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 85.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 153.26 | 107.16 | 0.00 |  | 0.00 |  | 107.16 | neither loop stalled long enough: network, hypervisor or untraced |

## boot-full-nomitig/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=624333 clocksource=tsc

### Loop counters (boot-full-nomitig/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002700 | 696999 | 4.31 | 111972160 | 37.29 | 1866203 | 1866203 | 99.4 | 67.3 | true | 0 | 585 | 580.5 | 69645 / 6433 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672604800 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (113010 of 2499950 in 1-msg reads) | 113010 | 2.29 | 2.01 | 3.14 | 5.45 | 12.73 | 30.73 | 38.20 |

#### Stalls: boot-full-nomitig/md-recv-busy, loop 0 on CPU 1

111972160 iterations, longest 580.5 µs; over 5 / 20 / 100 µs: 69645 / 6433 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 6098 | 176589.0 | 580.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 150 | 3502.4 | 57.6 |
| mostly unexplained (+preempted by kworker/1:1) | 100 | 2684.5 | 52.2 |
| softirq NET_RX | 74 | 2030.3 | 37.8 |
| softirq TIMER | 1 | 247.9 | 247.9 |
| mostly unexplained (+softirq NET_RX) | 8 | 207.4 | 32.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 58.0 | 36.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 580.5 | 2700 | loop work (2700 items) |  |
| 2 | 247.9 | 0 | softirq TIMER | softirq TIMER 229, local timer tick 12, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 3 | 61.4 | 10 | loop work (10 items) | softirq NET_RX 20, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 59.4 | 18 | loop work (18 items) | softirq NET_RX 20, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 59.0 | 17 | loop work (17 items) | softirq NET_RX 22, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 58.7 | 19 | loop work (19 items) | softirq NET_RX 19, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 58.3 | 24 | loop work (24 items) | preempted by kworker/1:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 58.2 | 16 | loop work (16 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 58.1 | 21 | loop work (21 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 58.1 | 18 | loop work (18 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 980 |
| IWI IRQ work interrupts | 591 |
| LOC Local timer interrupts | 588 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## boot-full-nomitig/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: boot-full-nomitig/noise, loop 0 on CPU 1

9025150976 iterations, longest 57.7 µs; over 5 / 20 / 100 µs: 78 / 4 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 78 | 821.8 | 57.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 57.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 33.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 32.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 31.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 18.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 17.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 15.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 12.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 12.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 11.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 978 |

### Loop counters (boot-full-nomitig/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1133528 | 1133528 | 1.00 | 282134705 | 248.90 | 4702245 | 4664461 | 0.0 | 40.4 | false | 0 | 0 | 61.9 | 26842 / 12 / 0 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-epoll-spin-0, loop 0 on CPU 1

279867648 iterations, longest 61.9 µs; over 5 / 20 / 100 µs: 26842 / 12 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 10 | 336.7 | 61.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 39.1 | 39.1 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 25.6 | 25.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 61.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 51.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 46.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 45.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 39.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 25.6 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 24.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 22.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 22.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 21.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 978 |
| LOC Local timer interrupts | 12 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: boot-full-nomitig/pp-epoll-spin-0 (median 49.09)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 245.79 | 196.70 | 0.00 |  | 0.00 |  | 196.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 243.85 | 194.76 | 0.00 |  | 0.00 |  | 194.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 232.84 | 183.75 | 0.00 |  | 0.00 |  | 183.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 224.64 | 175.55 | 0.00 |  | 0.00 |  | 175.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 209.55 | 160.46 | 0.00 |  | 27.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 132.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 208.41 | 159.33 | 0.00 |  | 0.00 |  | 159.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 197.16 | 148.07 | 0.00 |  | 0.00 |  | 148.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 189.34 | 140.25 | 0.00 |  | 0.00 |  | 140.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 187.41 | 138.32 | 0.00 |  | 0.00 |  | 138.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 187.05 | 137.96 | 0.00 |  | 0.00 |  | 137.96 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198618 | 1198618 | 1.00 | 553918901 | 462.13 | 4615991 | 4596014 | 0.0 | 36.7 | false | 0 | 0 | 195.1 | 5820 / 14 / 2 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-epoll-spin-10000, loop 0 on CPU 1

551521664 iterations, longest 195.1 µs; over 5 / 20 / 100 µs: 5820 / 14 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 9 | 347.1 | 100.2 |
| softirq TIMER | 1 | 195.1 | 195.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 148.8 | 68.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 195.1 | 0 | softirq TIMER | softirq TIMER 165, local timer tick 14, timer tick_nohz_handler 8 |
| 2 | 100.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 68.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 54.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 47.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 30.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 27.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 26.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 26.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 25.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 978 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-epoll-spin-10000 (median 50.02)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 166.61 | 116.59 | 0.00 |  | 136.28 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 162.01 | 112.00 | 0.00 |  | 0.00 |  | 112.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 161.25 | 111.23 | 0.00 |  | 0.00 |  | 111.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 153.75 | 103.73 | 99.65 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 4.08 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 147.44 | 97.42 | 0.00 |  | 0.00 |  | 97.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 136.72 | 86.70 | 0.00 |  | 0.00 |  | 86.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 134.90 | 84.89 | 0.00 |  | 0.00 |  | 84.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 134.48 | 84.46 | 0.00 |  | 0.00 |  | 84.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 133.82 | 83.80 | 0.00 |  | 0.00 |  | 83.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 133.18 | 83.17 | 0.00 |  | 0.00 |  | 83.17 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1332849 | 1332849 | 1.00 | 112672562 | 84.54 | 1877876 | 1855662 | 98.8 | 73.1 | true | 0 | 2415 | 256.2 | 22054 / 563 / 1 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-recv-busy-0, loop 0 on CPU 1

111339712 iterations, longest 256.2 µs; over 5 / 20 / 100 µs: 22054 / 563 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 353 | 7962.7 | 36.1 |
| softirq NET_RX | 107 | 2383.7 | 29.8 |
| mostly unexplained (+softirq NET_RX) | 88 | 2074.9 | 71.4 |
| softirq TIMER | 1 | 256.2 | 256.2 |
| mostly unexplained (+local timer tick) | 7 | 186.1 | 29.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 5 | 136.7 | 42.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 62.5 | 37.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 256.2 | 1 | softirq TIMER | softirq TIMER 235, local timer tick 6, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 2 | 71.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 3 | 42.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 37.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 36.1 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 34.5 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 34.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 33.3 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 9 | 32.1 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 31.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2452 |
| IWI IRQ work interrupts | 2420 |
| CAL Function call interrupts | 979 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-full-nomitig/pp-recv-busy-0 (median 43.72)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 416.89 | 373.16 | 0.00 |  | 0.00 |  | 373.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 299.42 | 255.70 | 256.00 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 287.71 | 243.99 | 0.00 |  | 0.00 |  | 243.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 246.42 | 202.70 | 0.00 |  | 0.00 |  | 202.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 217.65 | 173.92 | 0.00 |  | 0.00 |  | 173.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 206.16 | 162.44 | 0.00 |  | 0.00 |  | 162.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 195.41 | 151.69 | 0.00 |  | 0.00 |  | 151.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 171.87 | 128.15 | 0.00 |  | 0.00 |  | 128.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 167.49 | 123.77 | 0.00 |  | 0.00 |  | 123.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 158.14 | 114.42 | 0.00 |  | 0.00 |  | 114.42 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1189588 | 1189588 | 1.00 | 236221908 | 198.57 | 1968516 | 1958603 | 99.5 | 70.4 | true | 0 | 5614 | 60.4 | 28007 / 1230 / 0 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-recv-busy-10000, loop 0 on CPU 1

235032320 iterations, longest 60.4 µs; over 5 / 20 / 100 µs: 28007 / 1230 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 838 | 18947.7 | 34.5 |
| softirq NET_RX | 197 | 4419.4 | 34.1 |
| mostly unexplained (+softirq NET_RX) | 155 | 3688.3 | 32.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 17 | 459.5 | 60.4 |
| mostly unexplained (+local timer tick) | 15 | 378.0 | 33.4 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 116.9 | 46.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 94.5 | 36.4 |
| loop work | 1 | 20.0 | 20.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 60.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 46.3 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 36.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 35.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 34.5 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 9, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 6 | 34.1 | 0 | softirq NET_RX | softirq NET_RX 19, preempted by kworker/1:1 10, local timer tick 6, timer tick_nohz_handler 3 |
| 7 | 34.0 | 0 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 33.4 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, preempted by kworker/1:1 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 9 | 32.9 | 1 | softirq NET_RX | softirq NET_RX 20, local timer tick 6, preempted by kworker/1:1 5, timer tick_nohz_handler 4 |
| 10 | 32.5 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/1:1 7, local timer tick 3, timer tick_nohz_handler 2 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 5745 |
| IWI IRQ work interrupts | 5621 |
| CAL Function call interrupts | 978 |

#### Slowest round trips: boot-full-nomitig/pp-recv-busy-10000 (median 46.77)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 730.42 | 683.65 | 0.00 |  | 0.00 |  | 683.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 712.73 | 665.96 | 0.00 |  | 0.00 |  | 665.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 638.33 | 591.55 | 0.00 |  | 0.00 |  | 591.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 596.93 | 550.16 | 0.00 |  | 0.00 |  | 550.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 546.76 | 499.99 | 0.00 |  | 0.00 |  | 499.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 437.62 | 390.85 | 0.00 |  | 0.00 |  | 390.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 394.72 | 347.95 | 0.00 |  | 0.00 |  | 347.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 362.96 | 316.19 | 0.00 |  | 0.00 |  | 316.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 359.07 | 312.30 | 0.00 |  | 0.00 |  | 312.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 358.37 | 311.60 | 0.00 |  | 0.00 |  | 311.60 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1106856 | 1106856 | 1.00 | 3320569 | 3.00 | 55343 | 45475146 | 0.0 | 7.9 | false | 0 | 0 | 98.5 | 15750 / 13 / 0 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-uring-spin-0, loop 0 on CPU 1

2728508736 iterations, longest 98.5 µs; over 5 / 20 / 100 µs: 15750 / 13 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 10 | 350.4 | 98.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.9 | 33.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 33.7 | 33.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 98.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 55.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 33.7 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 33.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 29.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 29.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 26.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 24.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 23.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 22.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 981 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-uring-spin-0 (median 49.15)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1107.96 | 1058.81 | 0.00 |  | 0.00 |  | 1058.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 480.57 | 431.42 | 0.00 |  | 0.00 |  | 431.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 478.94 | 429.79 | 0.00 |  | 0.00 |  | 429.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 438.92 | 389.77 | 0.00 |  | 0.00 |  | 389.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 391.51 | 342.36 | 0.00 |  | 0.00 |  | 342.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 319.63 | 270.48 | 0.00 |  | 0.00 |  | 270.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 278.27 | 229.12 | 0.00 |  | 0.00 |  | 229.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 251.37 | 202.22 | 0.00 |  | 0.00 |  | 202.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 222.47 | 173.32 | 0.00 |  | 0.00 |  | 173.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 202.01 | 152.86 | 0.00 |  | 0.00 |  | 152.86 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199115 | 1199115 | 1.00 | 3597346 | 3.00 | 29978 | 35269451 | 0.0 | 5.1 | false | 0 | 0 | 119.7 | 22530 / 8 / 1 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-uring-spin-10000, loop 0 on CPU 1

4232334080 iterations, longest 119.7 µs; over 5 / 20 / 100 µs: 22530 / 8 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 119.7 | 119.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 111.8 | 57.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 103.0 | 32.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 29.1 | 29.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 119.7 | 0 | softirq TIMER | softirq TIMER 90, local timer tick 17, timer tick_nohz_handler 9, irq work (IPI/vector) 1 |
| 2 | 57.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 54.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 32.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 29.1 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 27.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 21.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 20.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 981 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-full-nomitig/pp-uring-spin-10000 (median 50.27)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 210.34 | 160.07 | 0.00 |  | 21.99 | unexplained: no kernel event (hypervisor or in-kernel time) | 138.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 205.32 | 155.05 | 0.00 |  | 0.00 |  | 155.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 204.10 | 153.83 | 0.00 |  | 0.00 |  | 153.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 201.84 | 151.57 | 0.00 |  | 0.00 |  | 151.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 201.51 | 151.24 | 0.00 |  | 0.00 |  | 151.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 200.73 | 150.46 | 0.00 |  | 0.00 |  | 150.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 199.74 | 149.47 | 0.00 |  | 0.00 |  | 149.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 197.07 | 146.79 | 0.00 |  | 0.00 |  | 146.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 194.46 | 144.19 | 0.00 |  | 0.00 |  | 144.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 192.86 | 142.58 | 0.00 |  | 0.00 |  | 142.58 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1227701 | 1227701 | 1.00 | 1 | 0.00 | 0 | 35143050 | 0.0 | 0.0 | true | 0 | 0 | 55.1 | 49 / 2 / 0 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-uring-sqpoll-0, loop 0 on CPU 1

2108582976 iterations, longest 55.1 µs; over 5 / 20 / 100 µs: 49 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 55.1 | 55.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 38.4 | 38.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 55.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 38.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 985 |
| LOC Local timer interrupts | 13 |
| IWI IRQ work interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-uring-sqpoll-0 (median 47.34)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 305.72 | 258.39 | 0.00 |  | 260.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 229.07 | 181.73 | 0.00 |  | 0.00 |  | 181.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 222.98 | 175.65 | 0.00 |  | 0.00 |  | 175.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 209.53 | 162.20 | 0.00 |  | 0.00 |  | 162.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 202.37 | 155.03 | 0.00 |  | 0.00 |  | 155.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 199.26 | 151.93 | 0.00 |  | 0.00 |  | 151.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 198.40 | 151.06 | 0.00 |  | 0.00 |  | 151.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 195.82 | 148.48 | 0.00 |  | 0.00 |  | 148.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 188.46 | 141.12 | 0.00 |  | 0.00 |  | 141.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 183.51 | 136.18 | 0.00 |  | 0.00 |  | 136.18 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1186966 | 1186966 | 1.00 | 1 | 0.00 | 0 | 22308719 | 0.0 | 0.0 | true | 0 | 0 | 55.6 | 88 / 6 / 0 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-uring-sqpoll-10000, loop 0 on CPU 1

2677046336 iterations, longest 55.6 µs; over 5 / 20 / 100 µs: 88 / 6 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 185.4 | 55.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 58.9 | 35.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 55.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 46.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 44.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 39.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 35.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 23.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 988 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-uring-sqpoll-10000 (median 47.27)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1003.40 | 956.13 | 0.00 |  | 0.00 |  | 956.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 368.83 | 321.55 | 0.00 |  | 0.00 |  | 321.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 361.85 | 314.58 | 0.00 |  | 0.00 |  | 314.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 356.15 | 308.88 | 0.00 |  | 0.00 |  | 308.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 354.18 | 306.90 | 0.00 |  | 0.00 |  | 306.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 351.40 | 304.12 | 0.00 |  | 0.00 |  | 304.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 351.35 | 304.08 | 0.00 |  | 0.00 |  | 304.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 348.13 | 300.86 | 0.00 |  | 0.00 |  | 300.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 347.68 | 300.40 | 0.00 |  | 0.00 |  | 300.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 346.79 | 299.52 | 0.00 |  | 0.00 |  | 299.52 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: boot-full/http-recv-busy-10000, loop 0 on CPU 1

1828608372 iterations, longest 156.0 µs; over 5 / 20 / 100 µs: 174020 / 2319 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1010 | 23558.3 | 156.0 |
| mostly unexplained (+preempted by kworker/1:1) | 720 | 19220.9 | 49.2 |
| mostly unexplained (+softirq NET_RX) | 428 | 10273.1 | 40.6 |
| softirq NET_RX | 104 | 2612.8 | 47.8 |
| mostly unexplained (+local timer tick) | 40 | 1196.9 | 43.9 |
| mostly unexplained (+irq work (IPI/vector)) | 12 | 311.0 | 35.5 |
| loop work | 5 | 122.9 | 32.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 156.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 154.2 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 125.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 124.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 59.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 56.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 55.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 55.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 55.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 49.2 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2214 |
| IWI IRQ work interrupts | 2132 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: boot-full/http-recv-busy-10000 (median 46.31)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 269.07 | 222.76 | 0.00 |  | 0.00 |  | 222.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 178.37 | 132.06 | 0.00 |  | 0.00 |  | 132.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 177.93 | 131.62 | 0.00 |  | 0.00 |  | 131.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 165.11 | 118.80 | 155.42 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 164.30 | 118.00 | 153.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 158.46 | 112.15 | 0.00 |  | 0.00 |  | 112.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 155.87 | 109.57 | 25.31 | loop work (2 items) | 0.00 |  | 84.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 150.06 | 103.75 | 0.00 |  | 0.00 |  | 103.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 148.04 | 101.73 | 0.00 |  | 0.00 |  | 101.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 145.85 | 99.54 | 0.00 |  | 0.00 |  | 99.54 | neither loop stalled long enough: network, hypervisor or untraced |

## boot-full/md-recv-busy: lab, 60 s (first 10 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=396458 clocksource=tsc

### Loop counters (boot-full/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002700 | 439873 | 6.83 | 109134080 | 36.35 | 1818901 | 1818901 | 99.6 | 65.1 | true | 0 | 666 | 532.3 | 127148 / 4683 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672604800 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (49704 of 2500000 in 1-msg reads) | 49704 | 2.98 | 2.80 | 3.82 | 6.58 | 12.31 | 28.39 | 36.89 |

#### Stalls: boot-full/md-recv-busy, loop 0 on CPU 1

109134080 iterations, longest 532.3 µs; over 5 / 20 / 100 µs: 127148 / 4683 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4409 | 128216.7 | 532.3 |
| mostly unexplained (+preempted by kworker/1:1) | 104 | 2670.4 | 45.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 79 | 2135.2 | 152.3 |
| softirq NET_RX | 75 | 1885.9 | 34.4 |
| softirq TIMER | 1 | 258.0 | 258.0 |
| mostly unexplained (+softirq NET_RX) | 10 | 245.4 | 30.6 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 70.7 | 28.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.2 | 32.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 532.3 | 2700 | loop work (2700 items) |  |
| 2 | 258.0 | 0 | softirq TIMER | softirq TIMER 240, local timer tick 12, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 3 | 152.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 112.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 65.0 | 20 | loop work (20 items) | softirq NET_RX 23, preempted by kworker/1:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 64.2 | 12 | loop work (12 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 63.6 | 20 | loop work (20 items) | softirq NET_RX 23, preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 63.3 | 17 | loop work (17 items) | preempted by kworker/1:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 63.1 | 20 | loop work (20 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 60.3 | 18 | loop work (18 items) | preempted by kworker/1:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 677 |
| LOC Local timer interrupts | 670 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## boot-full/noise: platform noise on CPU 1 (120 s, a loop that only reads the counter)

#### Stalls: boot-full/noise, loop 0 on CPU 1

9035653120 iterations, longest 133.9 µs; over 5 / 20 / 100 µs: 100 / 9 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 90 | 1227.9 | 133.9 |
| mostly unexplained (+call function (IPI/vector)) | 10 | 56.4 | 7.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 133.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 108.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 80.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 57.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 40.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 29.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 26.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 26.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 24.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 15.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |

### Loop counters (boot-full/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1163286 | 1163286 | 1.00 | 281253037 | 241.77 | 4687551 | 4648774 | 0.0 | 40.5 | false | 0 | 0 | 150.8 | 34556 / 8 / 2 | 0 | 0 |

#### Stalls: boot-full/pp-epoll-spin-0, loop 0 on CPU 1

278926464 iterations, longest 150.8 µs; over 5 / 20 / 100 µs: 34556 / 8 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 7 | 394.8 | 150.8 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 23.5 | 23.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 150.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 126.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 26.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 25.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 23.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 23.5 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 21.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 21.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 13 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full/pp-epoll-spin-0 (median 48.89)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1035.94 | 987.05 | 0.00 |  | 0.00 |  | 987.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 571.10 | 522.21 | 0.00 |  | 0.00 |  | 522.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 535.87 | 486.98 | 0.00 |  | 0.00 |  | 486.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 519.18 | 470.29 | 0.00 |  | 0.00 |  | 470.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 515.06 | 466.18 | 0.00 |  | 0.00 |  | 466.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 505.95 | 457.06 | 0.00 |  | 0.00 |  | 457.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 503.49 | 454.61 | 0.00 |  | 0.00 |  | 454.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 502.56 | 453.67 | 0.00 |  | 0.00 |  | 453.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 477.87 | 428.98 | 0.00 |  | 0.00 |  | 428.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 472.63 | 423.74 | 0.00 |  | 0.00 |  | 423.74 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1190823 | 1190823 | 1.00 | 564096527 | 473.70 | 4700804 | 4680957 | 0.0 | 37.9 | false | 0 | 0 | 254.2 | 23693 / 15 / 4 | 0 | 0 |

#### Stalls: boot-full/pp-epoll-spin-10000, loop 0 on CPU 1

561714880 iterations, longest 254.2 µs; over 5 / 20 / 100 µs: 23693 / 15 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 12 | 723.9 | 144.9 |
| softirq TIMER | 1 | 254.2 | 254.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.1 | 32.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 254.2 | 0 | softirq TIMER | softirq TIMER 230, local timer tick 12, timer tick_nohz_handler 7 |
| 2 | 144.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 141.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 105.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 59.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 48.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 47.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 47.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 32.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1001 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full/pp-epoll-spin-10000 (median 74.04)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 534.91 | 460.87 | 0.00 |  | 0.00 |  | 460.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 453.49 | 379.45 | 0.00 |  | 0.00 |  | 379.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 391.64 | 317.60 | 0.00 |  | 0.00 |  | 317.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 381.90 | 307.86 | 0.00 |  | 0.00 |  | 307.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 379.83 | 305.79 | 0.00 |  | 0.00 |  | 305.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 357.08 | 283.04 | 0.00 |  | 0.00 |  | 283.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 352.99 | 278.95 | 0.00 |  | 0.00 |  | 278.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 352.80 | 278.76 | 0.00 |  | 0.00 |  | 278.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 351.17 | 277.13 | 0.00 |  | 0.00 |  | 277.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 349.94 | 275.90 | 0.00 |  | 0.00 |  | 275.90 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1296865 | 1296865 | 1.00 | 106747042 | 82.31 | 1779117 | 1757503 | 98.8 | 71.5 | true | 0 | 2498 | 163.3 | 47807 / 745 / 2 | 0 | 0 |

#### Stalls: boot-full/pp-recv-busy-0, loop 0 on CPU 1

105450176 iterations, longest 163.3 µs; over 5 / 20 / 100 µs: 47807 / 745 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 489 | 11204.4 | 44.0 |
| softirq NET_RX | 135 | 3094.4 | 28.9 |
| mostly unexplained (+softirq NET_RX) | 92 | 2199.3 | 33.8 |
| mostly unexplained (+local timer tick) | 19 | 470.2 | 30.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 6 | 311.0 | 152.1 |
| softirq TIMER | 1 | 163.3 | 163.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 56.6 | 36.2 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 26.6 | 26.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 163.3 | 0 | softirq TIMER | softirq TIMER 144, local timer tick 9, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 152.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 44.0 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 37.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 36.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 35.2 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 7, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 7 | 34.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 33.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/1:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 33.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 31.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2565 |
| IWI IRQ work interrupts | 2504 |
| CAL Function call interrupts | 990 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full/pp-recv-busy-0 (median 45.23)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 694.49 | 649.27 | 0.00 |  | 0.00 |  | 649.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 335.02 | 289.79 | 163.29 | softirq TIMER | 0.00 |  | 126.50 | client: softirq TIMER |
| 3 | 311.31 | 266.08 | 0.00 |  | 0.00 |  | 266.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 176.88 | 131.65 | 0.00 |  | 0.00 |  | 131.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 164.11 | 118.88 | 152.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 146.83 | 101.61 | 0.00 |  | 0.00 |  | 101.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 140.87 | 95.65 | 0.00 |  | 0.00 |  | 95.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 129.70 | 84.48 | 0.00 |  | 0.00 |  | 84.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 128.97 | 83.75 | 0.00 |  | 0.00 |  | 83.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 128.04 | 82.82 | 0.00 |  | 0.00 |  | 82.82 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1196803 | 1196803 | 1.00 | 222693188 | 186.07 | 1855777 | 1845803 | 99.5 | 68.2 | true | 0 | 3952 | 148.1 | 55067 / 1116 / 4 | 0 | 0 |

#### Stalls: boot-full/pp-recv-busy-10000, loop 0 on CPU 1

221496384 iterations, longest 148.1 µs; over 5 / 20 / 100 µs: 55067 / 1116 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/1:1) | 819 | 19807.1 | 38.6 |
| softirq NET_RX | 148 | 3453.9 | 31.4 |
| mostly unexplained (+softirq NET_RX) | 107 | 2746.2 | 64.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 19 | 1023.7 | 148.1 |
| mostly unexplained (+local timer tick) | 22 | 552.2 | 35.4 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 43.2 | 43.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 148.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 145.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 130.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 124.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 75.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 64.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 54.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 51.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 43.2 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 38.6 | 1 | mostly unexplained (+preempted by kworker/1:1) | preempted by kworker/1:1 10, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 4104 |
| IWI IRQ work interrupts | 3956 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: boot-full/pp-recv-busy-10000 (median 46.80)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 311.56 | 264.75 | 0.00 |  | 22.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 242.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 267.88 | 221.08 | 0.00 |  | 0.00 |  | 221.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 239.77 | 192.97 | 0.00 |  | 0.00 |  | 192.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 232.66 | 185.85 | 0.00 |  | 230.56 | softirq TIMER | 0.00 | server: softirq TIMER |
| 5 | 208.21 | 161.40 | 0.00 |  | 0.00 |  | 161.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 207.45 | 160.64 | 0.00 |  | 0.00 |  | 160.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 205.61 | 158.81 | 0.00 |  | 0.00 |  | 158.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 198.67 | 151.87 | 0.00 |  | 0.00 |  | 151.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 198.39 | 151.59 | 0.00 |  | 0.00 |  | 151.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 197.72 | 150.91 | 0.00 |  | 0.00 |  | 150.91 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1176710 | 1176710 | 1.00 | 3530131 | 3.00 | 58836 | 44853844 | 0.0 | 9.2 | false | 0 | 0 | 149.7 | 26150 / 24 / 1 | 0 | 0 |

#### Stalls: boot-full/pp-uring-spin-0, loop 0 on CPU 1

2691230656 iterations, longest 149.7 µs; over 5 / 20 / 100 µs: 26150 / 24 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 21 | 778.3 | 149.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.1 | 33.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 34.1 | 34.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 149.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 59.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 50.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 48.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 42.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 34.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 34.1 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 33.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 31.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 31.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full/pp-uring-spin-0 (median 47.90)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 338.92 | 291.02 | 0.00 |  | 0.00 |  | 291.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 337.21 | 289.31 | 0.00 |  | 0.00 |  | 289.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 333.65 | 285.75 | 0.00 |  | 0.00 |  | 285.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 302.54 | 254.64 | 0.00 |  | 0.00 |  | 254.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 295.88 | 247.97 | 0.00 |  | 0.00 |  | 247.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 293.81 | 245.90 | 0.00 |  | 0.00 |  | 245.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 293.25 | 245.35 | 0.00 |  | 0.00 |  | 245.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 291.41 | 243.51 | 0.00 |  | 0.00 |  | 243.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 291.33 | 243.43 | 0.00 |  | 0.00 |  | 243.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 291.09 | 243.19 | 0.00 |  | 0.00 |  | 243.19 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1186914 | 1186914 | 1.00 | 3560742 | 3.00 | 29673 | 35620791 | 0.0 | 5.3 | false | 0 | 0 | 216.2 | 7557 / 15 / 5 | 0 | 0 |

#### Stalls: boot-full/pp-uring-spin-10000, loop 0 on CPU 1

4274494912 iterations, longest 216.2 µs; over 5 / 20 / 100 µs: 7557 / 15 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 13 | 874.3 | 142.5 |
| softirq TIMER | 1 | 216.2 | 216.2 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 43.9 | 43.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 216.2 | 0 | softirq TIMER | softirq TIMER 184, local timer tick 14, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 142.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 141.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 131.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 114.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 84.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 66.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 53.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 43.9 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 27.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-full/pp-uring-spin-10000 (median 51.48)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 380.39 | 328.92 | 0.00 |  | 0.00 |  | 328.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 372.53 | 321.06 | 0.00 |  | 0.00 |  | 321.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 371.30 | 319.83 | 0.00 |  | 0.00 |  | 319.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 368.36 | 316.88 | 0.00 |  | 0.00 |  | 316.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 364.17 | 312.69 | 0.00 |  | 0.00 |  | 312.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 360.21 | 308.74 | 0.00 |  | 0.00 |  | 308.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 358.58 | 307.10 | 0.00 |  | 0.00 |  | 307.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 357.33 | 305.86 | 0.00 |  | 0.00 |  | 305.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 357.03 | 305.56 | 0.00 |  | 0.00 |  | 305.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 354.61 | 303.13 | 0.00 |  | 0.00 |  | 303.13 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1248223 | 1248223 | 1.00 | 1 | 0.00 | 0 | 34983947 | 0.0 | 0.0 | true | 0 | 0 | 144.4 | 63 / 4 / 2 | 0 | 0 |

#### Stalls: boot-full/pp-uring-sqpoll-0, loop 0 on CPU 1

2099036800 iterations, longest 144.4 µs; over 5 / 20 / 100 µs: 63 / 4 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 292.4 | 144.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 65.0 | 65.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 144.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 127.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 65.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 20.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 995 |
| LOC Local timer interrupts | 5 |
| IWI IRQ work interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full/pp-uring-sqpoll-0 (median 47.62)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1052.82 | 1005.20 | 0.00 |  | 0.00 |  | 1005.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 277.53 | 229.90 | 0.00 |  | 225.42 | softirq TIMER | 4.49 | server: softirq TIMER |
| 3 | 180.18 | 132.55 | 143.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 158.39 | 110.77 | 0.00 |  | 0.00 |  | 110.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 150.31 | 102.69 | 0.00 |  | 0.00 |  | 102.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 149.47 | 101.85 | 126.99 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 145.06 | 97.43 | 0.00 |  | 0.00 |  | 97.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 144.84 | 97.22 | 0.00 |  | 0.00 |  | 97.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 143.17 | 95.55 | 0.00 |  | 0.00 |  | 95.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 143.14 | 95.52 | 0.00 |  | 0.00 |  | 95.52 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199460 | 1199460 | 1.00 | 1 | 0.00 | 0 | 25014543 | 0.0 | 0.0 | true | 0 | 0 | 144.7 | 121 / 12 / 4 | 0 | 0 |

#### Stalls: boot-full/pp-uring-sqpoll-10000, loop 0 on CPU 1

3001745216 iterations, longest 144.7 µs; over 5 / 20 / 100 µs: 121 / 12 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 11 | 819.4 | 144.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.6 | 35.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 144.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 133.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 106.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 103.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 58.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 50.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 49.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 47.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 44.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 1 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1008 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: boot-full/pp-uring-sqpoll-10000 (median 47.86)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 302.19 | 254.33 | 0.00 |  | 0.00 |  | 254.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 281.91 | 234.05 | 0.00 |  | 0.00 |  | 234.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 239.01 | 191.15 | 0.00 |  | 0.00 |  | 191.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 208.78 | 160.91 | 0.00 |  | 0.00 |  | 160.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 177.60 | 129.74 | 0.00 |  | 0.00 |  | 129.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 172.79 | 124.93 | 133.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 172.39 | 124.53 | 0.00 |  | 0.00 |  | 124.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 165.07 | 117.20 | 0.00 |  | 0.00 |  | 117.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 151.44 | 103.57 | 0.00 |  | 0.00 |  | 103.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 151.41 | 103.55 | 103.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.14 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

## Ping-pong round trip (client send → echo decoded; "from when due": from the slot on the paced schedule, including any wait behind a stall)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189938 | 47.04 | 46.37 | 50.29 | 61.96 | 75.61 | 110.51 | 338.88 |
| r1-base-0/pp-epoll-spin-0: epoll-spin @ 0/s | 1137716 | 51.76 | 48.94 | 59.57 | 97.37 | 108.04 | 286.57 | 438.11 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1146636 | 62.17 | 51.56 | 91.15 | 232.20 | 295.26 | 342.17 | 411.31 |
| r1-base-0/pp-recv-busy-0: recv-busy @ 0/s | 1270218 | 46.35 | 45.25 | 51.91 | 65.56 | 93.96 | 101.94 | 199.64 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s | 1186907 | 46.06 | 45.46 | 49.42 | 57.91 | 70.69 | 85.60 | 331.27 |
| r1-base-0/pp-uring-spin-0: uring-spin @ 0/s | 1132952 | 51.29 | 48.31 | 59.41 | 96.52 | 103.05 | 134.84 | 247.40 |
| r1-base-0/pp-uring-spin-10000: uring-spin @ 10000/s | 1187447 | 56.13 | 51.18 | 75.36 | 100.97 | 134.51 | 141.55 | 233.45 |
| r1-base-0/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1239942 | 47.27 | 46.43 | 53.07 | 84.38 | 96.70 | 106.70 | 1048.50 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189549 | 46.76 | 46.11 | 50.36 | 58.53 | 74.41 | 107.36 | 292.05 |
| r1-housekeeping/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189937 | 48.82 | 48.11 | 53.56 | 62.60 | 78.88 | 121.61 | 1051.57 |
| r1-housekeeping/pp-recv-busy-0: recv-busy @ 0/s | 1258068 | 46.80 | 46.04 | 52.14 | 63.37 | 95.90 | 109.95 | 260.84 |
| r1-housekeeping/pp-recv-busy-10000: recv-busy @ 10000/s | 1186892 | 47.21 | 46.55 | 50.73 | 59.63 | 75.62 | 101.07 | 994.56 |
| r1-housekeeping/pp-uring-spin-0: uring-spin @ 0/s | 1131504 | 51.33 | 48.42 | 58.42 | 96.19 | 102.78 | 141.60 | 229.31 |
| r1-housekeeping/pp-uring-spin-10000: uring-spin @ 10000/s | 1182178 | 64.22 | 55.41 | 90.11 | 124.22 | 228.34 | 288.36 | 1054.08 |
| r1-housekeeping/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1217121 | 48.22 | 47.89 | 53.07 | 87.91 | 97.80 | 106.37 | 205.90 |
| r1-housekeeping/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1172998 | 49.95 | 47.29 | 51.63 | 179.86 | 254.59 | 305.98 | 381.92 |
| r1-prefer/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189938 | 49.70 | 49.13 | 53.51 | 62.46 | 77.00 | 91.58 | 231.76 |
| r1-prefer/pp-recv-busy-0: recv-busy @ 0/s | 1085440 | 54.21 | 52.30 | 62.25 | 96.51 | 103.47 | 128.03 | 256.81 |
| r1-prefer/pp-recv-busy-10000: recv-busy @ 10000/s | 1180068 | 50.85 | 50.43 | 54.14 | 59.66 | 68.95 | 94.66 | 392.79 |
| r1-prefer/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1132025 | 51.94 | 49.48 | 55.31 | 96.23 | 101.70 | 110.76 | 277.65 |
| r1-prefer/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189617 | 55.63 | 50.83 | 77.05 | 92.95 | 98.29 | 116.45 | 998.23 |
| r1-base-2/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189938 | 46.57 | 46.04 | 49.72 | 57.97 | 68.92 | 85.43 | 160.64 |
| r1-base-2/pp-epoll-spin-0: epoll-spin @ 0/s | 1146655 | 51.36 | 49.03 | 58.44 | 96.59 | 102.48 | 202.65 | 630.86 |
| r1-base-2/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1168493 | 69.81 | 60.65 | 94.96 | 137.18 | 238.39 | 305.27 | 395.01 |
| r1-base-2/pp-recv-busy-0: recv-busy @ 0/s | 1284155 | 45.85 | 44.58 | 51.57 | 62.77 | 92.03 | 101.61 | 1042.84 |
| r1-base-2/pp-recv-busy-10000: recv-busy @ 10000/s | 1186892 | 47.28 | 46.67 | 50.59 | 59.67 | 74.55 | 93.98 | 254.20 |
| r1-base-2/pp-uring-spin-0: uring-spin @ 0/s | 1135209 | 51.24 | 48.46 | 58.67 | 96.41 | 101.90 | 134.82 | 266.89 |
| r1-base-2/pp-uring-spin-10000: uring-spin @ 10000/s | 1181232 | 62.21 | 51.87 | 96.26 | 131.16 | 141.17 | 269.32 | 376.73 |
| r1-base-2/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1245154 | 47.11 | 46.77 | 52.54 | 63.72 | 95.28 | 106.58 | 1041.15 |
| r1-base-2/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1171496 | 49.31 | 46.44 | 50.84 | 191.08 | 260.81 | 305.85 | 396.43 |
| r1-noquickack/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189939 | 84.53 | 46.91 | 227.08 | 286.27 | 321.46 | 350.64 | 394.24 |
| r1-noquickack/pp-recv-busy-0: recv-busy @ 0/s | 1309119 | 44.97 | 44.42 | 48.26 | 56.07 | 64.16 | 80.21 | 366.12 |
| r1-noquickack/pp-recv-busy-10000: recv-busy @ 10000/s | 1044377 | 65.50 | 44.51 | 176.31 | 240.28 | 294.93 | 337.47 | 401.18 |
| r1-noquickack/pp-uring-spin-0: uring-spin @ 0/s | 1141256 | 50.95 | 49.84 | 55.92 | 65.04 | 96.34 | 110.27 | 375.07 |
| r1-noquickack/pp-uring-spin-10000: uring-spin @ 10000/s | 1189587 | 54.83 | 53.17 | 59.55 | 95.94 | 102.34 | 124.16 | 207.05 |
| r1-nogro/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189938 | 54.97 | 47.80 | 79.25 | 92.50 | 228.13 | 287.31 | 363.37 |
| r1-nogro/pp-recv-busy-0: recv-busy @ 0/s | 1197015 | 49.19 | 48.91 | 52.63 | 67.90 | 96.68 | 105.58 | 415.77 |
| r1-nogro/pp-recv-busy-10000: recv-busy @ 10000/s | 1186888 | 44.96 | 44.39 | 48.72 | 56.89 | 73.47 | 99.31 | 196.23 |
| r1-nogro/pp-uring-spin-0: uring-spin @ 0/s | 1099431 | 52.89 | 48.88 | 61.56 | 97.19 | 106.39 | 139.93 | 228.53 |
| r1-nogro/pp-uring-spin-10000: uring-spin @ 10000/s | 1164937 | 57.26 | 49.36 | 85.89 | 214.53 | 320.86 | 388.87 | 507.14 |
| r1-base-4/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189938 | 48.93 | 48.05 | 53.63 | 64.75 | 83.26 | 123.14 | 984.67 |
| r1-base-4/pp-epoll-spin-0: epoll-spin @ 0/s | 1129179 | 52.15 | 49.10 | 59.09 | 97.81 | 106.41 | 142.01 | 421.13 |
| r1-base-4/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1185864 | 56.78 | 51.01 | 83.61 | 103.70 | 134.62 | 143.29 | 1053.62 |
| r1-base-4/pp-recv-busy-0: recv-busy @ 0/s | 1224046 | 48.10 | 48.26 | 52.31 | 62.98 | 95.50 | 106.21 | 192.07 |
| r1-base-4/pp-recv-busy-10000: recv-busy @ 10000/s | 1186906 | 44.63 | 44.10 | 48.42 | 55.32 | 69.16 | 97.77 | 322.58 |
| r1-base-4/pp-uring-spin-0: uring-spin @ 0/s | 1111759 | 52.30 | 48.62 | 60.82 | 96.94 | 104.19 | 145.04 | 331.27 |
| r1-base-4/pp-uring-spin-10000: uring-spin @ 10000/s | 1183429 | 62.37 | 53.95 | 88.13 | 107.98 | 229.04 | 287.55 | 352.39 |
| r1-base-4/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1166808 | 50.33 | 48.65 | 56.39 | 94.74 | 102.78 | 128.60 | 287.42 |
| r1-base-4/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189475 | 47.30 | 46.70 | 52.11 | 58.59 | 69.23 | 125.33 | 202.81 |
| r1-nocoalesce/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189939 | 52.12 | 47.39 | 72.77 | 89.96 | 95.71 | 118.65 | 191.63 |
| r1-nocoalesce/pp-recv-busy-0: recv-busy @ 0/s | 1317229 | 44.69 | 43.54 | 50.05 | 59.60 | 77.98 | 98.31 | 1049.85 |
| r1-nocoalesce/pp-recv-busy-10000: recv-busy @ 10000/s | 1175528 | 52.24 | 46.35 | 71.86 | 92.12 | 240.31 | 298.71 | 1042.13 |
| r1-nocoalesce/pp-uring-spin-0: uring-spin @ 0/s | 1088623 | 53.43 | 52.26 | 61.93 | 97.01 | 104.43 | 125.74 | 295.06 |
| r1-nocoalesce/pp-uring-spin-10000: uring-spin @ 10000/s | 1189652 | 52.52 | 50.93 | 58.94 | 71.33 | 89.33 | 128.07 | 288.47 |
| r1-irqlocal/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189936 | 56.68 | 48.93 | 57.54 | 241.11 | 297.04 | 335.89 | 371.58 |
| r1-irqlocal/pp-epoll-spin-0: epoll-spin @ 0/s | 1108612 | 53.06 | 49.08 | 58.38 | 99.15 | 135.69 | 141.66 | 275.95 |
| r1-irqlocal/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1186090 | 65.81 | 50.68 | 93.19 | 100.82 | 131.51 | 138.46 | 335.65 |
| r1-irqlocal/pp-recv-busy-0: recv-busy @ 0/s | 1215891 | 48.36 | 48.46 | 52.78 | 72.76 | 96.82 | 105.47 | 206.16 |
| r1-irqlocal/pp-recv-busy-10000: recv-busy @ 10000/s | 1174948 | 53.66 | 46.99 | 75.40 | 94.67 | 242.27 | 300.83 | 359.49 |
| r1-irqlocal/pp-uring-spin-0: uring-spin @ 0/s | 1119776 | 51.93 | 48.55 | 59.26 | 97.64 | 131.76 | 137.59 | 201.62 |
| r1-irqlocal/pp-uring-spin-10000: uring-spin @ 10000/s | 1157892 | 71.21 | 57.29 | 97.66 | 137.81 | 245.13 | 308.88 | 379.99 |
| r1-base-6/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189937 | 50.24 | 46.19 | 67.17 | 88.47 | 94.99 | 111.64 | 510.88 |
| r1-base-6/pp-epoll-spin-0: epoll-spin @ 0/s | 1100305 | 53.52 | 49.41 | 62.64 | 99.48 | 241.04 | 343.35 | 490.06 |
| r1-base-6/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188213 | 54.64 | 50.33 | 62.02 | 98.06 | 109.32 | 138.27 | 1011.40 |
| r1-base-6/pp-recv-busy-0: recv-busy @ 0/s | 1301906 | 45.22 | 43.96 | 50.91 | 61.37 | 91.17 | 104.91 | 226.75 |
| r1-base-6/pp-recv-busy-10000: recv-busy @ 10000/s | 1186924 | 48.32 | 46.00 | 54.12 | 85.65 | 92.60 | 102.40 | 160.35 |
| r1-base-6/pp-uring-spin-0: uring-spin @ 0/s | 1149513 | 50.55 | 48.02 | 57.92 | 96.06 | 101.70 | 135.28 | 474.38 |
| r1-base-6/pp-uring-spin-10000: uring-spin @ 10000/s | 1163799 | 57.92 | 50.10 | 83.32 | 221.06 | 323.06 | 388.35 | 999.60 |
| r1-base-6/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1211750 | 48.35 | 47.60 | 53.48 | 89.92 | 99.03 | 109.58 | 332.90 |
| r1-base-6/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1173366 | 49.06 | 46.43 | 50.77 | 181.11 | 252.26 | 303.73 | 993.19 |
| r1-rt/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189938 | 45.89 | 45.42 | 49.02 | 55.17 | 65.54 | 103.19 | 231.15 |
| r1-rt/pp-recv-busy-0: recv-busy @ 0/s | 1303582 | 45.16 | 43.87 | 51.08 | 62.19 | 85.77 | 100.41 | 184.61 |
| r1-rt/pp-recv-busy-10000: recv-busy @ 10000/s | 1186854 | 43.87 | 43.42 | 47.54 | 53.47 | 63.62 | 103.14 | 287.06 |
| r1-rt/pp-uring-spin-0: uring-spin @ 0/s | 1102745 | 52.74 | 48.46 | 61.21 | 100.85 | 137.51 | 146.87 | 380.33 |
| r1-rt/pp-uring-spin-10000: uring-spin @ 10000/s | 1108556 | 65.82 | 50.92 | 92.84 | 289.00 | 396.41 | 486.31 | 654.04 |
| r1-sibling/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189917 | 48.62 | 47.92 | 52.57 | 61.01 | 75.91 | 227.03 | 1054.43 |
| r1-sibling/pp-recv-busy-0: recv-busy @ 0/s | 1226617 | 47.97 | 47.92 | 52.26 | 65.10 | 96.12 | 104.90 | 241.07 |
| r1-sibling/pp-recv-busy-10000: recv-busy @ 10000/s | 1179036 | 55.22 | 48.28 | 78.32 | 91.72 | 231.72 | 290.97 | 1050.71 |
| r1-sibling/pp-uring-spin-0: uring-spin @ 0/s | 1033966 | 56.06 | 53.57 | 65.17 | 99.54 | 137.81 | 148.81 | 325.31 |
| r1-sibling/pp-uring-spin-10000: uring-spin @ 10000/s | 1189125 | 55.61 | 51.37 | 62.90 | 99.83 | 107.07 | 124.89 | 331.94 |
| r1-sibling/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1196858 | 49.01 | 48.63 | 52.86 | 89.77 | 100.11 | 117.06 | 260.90 |
| r1-sibling/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189297 | 47.87 | 47.41 | 50.98 | 56.65 | 66.84 | 96.32 | 454.83 |
| r1-base-8/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189938 | 46.61 | 46.10 | 49.84 | 56.68 | 67.90 | 92.60 | 155.22 |
| r1-base-8/pp-epoll-spin-0: epoll-spin @ 0/s | 1127171 | 52.24 | 49.05 | 59.98 | 97.56 | 107.44 | 276.32 | 894.63 |
| r1-base-8/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1187801 | 53.15 | 50.64 | 57.90 | 94.30 | 128.81 | 135.71 | 186.72 |
| r1-base-8/pp-recv-busy-0: recv-busy @ 0/s | 1253980 | 46.95 | 45.74 | 53.45 | 63.86 | 91.86 | 120.68 | 280.77 |
| r1-base-8/pp-recv-busy-10000: recv-busy @ 10000/s | 1162328 | 49.70 | 45.87 | 50.60 | 212.81 | 273.04 | 307.27 | 369.23 |
| r1-base-8/pp-uring-spin-0: uring-spin @ 0/s | 1127438 | 51.56 | 48.40 | 59.01 | 96.65 | 104.15 | 142.59 | 1041.29 |
| r1-base-8/pp-uring-spin-10000: uring-spin @ 10000/s | 1147118 | 62.93 | 51.87 | 92.85 | 232.23 | 296.28 | 344.49 | 478.31 |
| r1-base-8/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1219432 | 48.14 | 47.93 | 52.69 | 86.02 | 96.96 | 106.00 | 1049.67 |
| r1-base-8/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1174494 | 48.50 | 46.04 | 50.16 | 170.43 | 248.43 | 301.37 | 433.71 |
| r1-busypoll50/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189936 | 47.23 | 46.72 | 50.48 | 58.23 | 69.65 | 115.71 | 193.17 |
| r1-busypoll50/pp-recv-busy-0: recv-busy @ 0/s | 1281699 | 45.94 | 44.70 | 51.58 | 63.11 | 92.93 | 150.55 | 1049.93 |
| r1-busypoll50/pp-recv-busy-10000: recv-busy @ 10000/s | 1186885 | 48.34 | 47.58 | 53.01 | 61.30 | 71.27 | 117.12 | 308.13 |
| r1-budget64/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189934 | 46.99 | 46.33 | 50.47 | 59.10 | 76.62 | 110.33 | 436.45 |
| r1-budget64/pp-recv-busy-0: recv-busy @ 0/s | 1251157 | 47.05 | 45.71 | 53.06 | 78.21 | 99.48 | 120.35 | 317.46 |
| r1-budget64/pp-recv-busy-10000: recv-busy @ 10000/s | 1186876 | 47.34 | 46.75 | 51.63 | 57.98 | 70.62 | 122.05 | 320.50 |
| r1-base-10/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189937 | 47.33 | 46.67 | 50.61 | 60.09 | 77.07 | 106.84 | 263.97 |
| r1-base-10/pp-epoll-spin-0: epoll-spin @ 0/s | 1121246 | 52.51 | 50.16 | 60.36 | 95.27 | 104.51 | 143.09 | 218.94 |
| r1-base-10/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1182868 | 62.42 | 51.75 | 96.23 | 129.63 | 140.54 | 149.21 | 978.73 |
| r1-base-10/pp-recv-busy-0: recv-busy @ 0/s | 1242073 | 47.40 | 47.32 | 52.18 | 80.00 | 95.11 | 106.42 | 205.42 |
| r1-base-10/pp-recv-busy-10000: recv-busy @ 10000/s | 1186875 | 46.17 | 45.56 | 49.44 | 57.77 | 75.40 | 110.11 | 1056.92 |
| r1-base-10/pp-uring-spin-0: uring-spin @ 0/s | 1103197 | 52.70 | 49.01 | 61.49 | 97.01 | 107.45 | 145.11 | 514.72 |
| r1-base-10/pp-uring-spin-10000: uring-spin @ 10000/s | 1189133 | 55.03 | 52.02 | 61.05 | 99.53 | 109.47 | 140.82 | 269.39 |
| r1-base-10/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1228848 | 47.76 | 47.61 | 52.75 | 72.78 | 97.51 | 110.09 | 1044.55 |
| r1-base-10/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189507 | 48.35 | 47.46 | 53.09 | 63.72 | 82.12 | 125.03 | 1037.29 |
| r1-cstate/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189937 | 43.27 | 42.78 | 46.20 | 53.38 | 62.61 | 80.25 | 243.46 |
| r1-cstate/pp-epoll-spin-0: epoll-spin @ 0/s | 1138830 | 51.71 | 49.01 | 55.89 | 123.67 | 132.78 | 140.54 | 304.29 |
| r1-cstate/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188524 | 53.91 | 48.16 | 88.66 | 95.67 | 102.91 | 126.58 | 204.20 |
| r1-cstate/pp-recv-busy-0: recv-busy @ 0/s | 1220516 | 48.24 | 48.65 | 51.93 | 60.57 | 94.21 | 106.27 | 482.76 |
| r1-cstate/pp-recv-busy-10000: recv-busy @ 10000/s | 1186877 | 43.41 | 42.91 | 46.18 | 53.85 | 70.01 | 99.38 | 187.88 |
| r1-cstate/pp-uring-spin-0: uring-spin @ 0/s | 1114740 | 52.19 | 48.87 | 57.82 | 97.69 | 133.78 | 140.94 | 445.55 |
| r1-cstate/pp-uring-spin-10000: uring-spin @ 10000/s | 1189654 | 57.10 | 48.40 | 88.91 | 93.42 | 99.95 | 114.23 | 237.17 |
| r1-cstate/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1190968 | 49.27 | 48.81 | 52.22 | 86.59 | 96.79 | 106.51 | 196.93 |
| r1-cstate/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189306 | 46.09 | 45.47 | 50.42 | 56.45 | 65.89 | 116.28 | 1055.23 |
| r1-base-11/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189936 | 46.85 | 46.11 | 49.84 | 58.90 | 89.93 | 272.13 | 1011.07 |
| r1-base-11/pp-epoll-spin-0: epoll-spin @ 0/s | 1124260 | 52.38 | 49.04 | 59.19 | 100.43 | 108.24 | 140.04 | 1041.06 |
| r1-base-11/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1186876 | 55.16 | 50.84 | 70.85 | 97.17 | 133.77 | 138.94 | 999.45 |
| r1-base-11/pp-recv-busy-0: recv-busy @ 0/s | 1258423 | 46.77 | 46.78 | 51.73 | 60.36 | 92.56 | 101.03 | 275.21 |
| r1-base-11/pp-recv-busy-10000: recv-busy @ 10000/s | 1186916 | 44.31 | 43.83 | 47.88 | 54.79 | 68.85 | 86.82 | 996.67 |
| r1-base-11/pp-uring-spin-0: uring-spin @ 0/s | 1129328 | 51.48 | 48.35 | 59.12 | 96.65 | 106.80 | 296.19 | 1062.24 |
| r1-base-11/pp-uring-spin-10000: uring-spin @ 10000/s | 1184099 | 63.03 | 54.80 | 88.23 | 101.66 | 231.46 | 292.06 | 377.08 |
| r1-base-11/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1221351 | 48.00 | 47.95 | 52.58 | 82.96 | 96.83 | 104.84 | 323.85 |
| r1-base-11/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1175543 | 48.88 | 46.55 | 50.53 | 157.06 | 248.07 | 299.95 | 369.08 |
| boot-stock/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189936 | 44.93 | 44.29 | 49.38 | 58.55 | 70.70 | 109.43 | 255.53 |
| boot-stock/pp-epoll-spin-0: epoll-spin @ 0/s | 819759 | 71.88 | 49.24 | 96.93 | 328.19 | 403.64 | 456.11 | 549.86 |
| boot-stock/pp-epoll-spin-10000: epoll-spin @ 10000/s | 493856 | 231.48 | 256.32 | 328.09 | 404.35 | 464.62 | 531.64 | 662.28 |
| boot-stock/pp-recv-busy-0: recv-busy @ 0/s | 1282803 | 45.85 | 45.19 | 49.56 | 57.19 | 70.43 | 95.57 | 188.57 |
| boot-stock/pp-recv-busy-10000: recv-busy @ 10000/s | 1187077 | 42.68 | 42.09 | 46.13 | 53.02 | 68.41 | 119.56 | 1047.96 |
| boot-stock/pp-uring-spin-0: uring-spin @ 0/s | 1035989 | 56.31 | 48.73 | 87.97 | 207.41 | 327.47 | 407.09 | 494.21 |
| boot-stock/pp-uring-spin-10000: uring-spin @ 10000/s | 749532 | 128.07 | 51.56 | 294.56 | 360.42 | 421.34 | 469.88 | 5116.32 |
| boot-stock/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1274231 | 45.96 | 45.41 | 49.50 | 57.38 | 66.14 | 91.19 | 342.18 |
| boot-stock/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1184423 | 45.76 | 44.59 | 48.57 | 58.21 | 228.37 | 286.56 | 353.90 |
| boot-isolcpus/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189937 | 46.60 | 46.14 | 49.86 | 57.60 | 69.77 | 92.50 | 166.57 |
| boot-isolcpus/pp-epoll-spin-0: epoll-spin @ 0/s | 1100902 | 53.48 | 50.57 | 62.03 | 97.14 | 104.61 | 235.01 | 510.03 |
| boot-isolcpus/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1103391 | 71.39 | 52.47 | 94.69 | 297.06 | 368.99 | 432.55 | 538.73 |
| boot-isolcpus/pp-recv-busy-0: recv-busy @ 0/s | 1275992 | 46.13 | 45.35 | 51.67 | 60.13 | 94.32 | 109.35 | 984.62 |
| boot-isolcpus/pp-recv-busy-10000: recv-busy @ 10000/s | 1174859 | 51.55 | 45.37 | 71.28 | 94.89 | 243.32 | 300.67 | 359.96 |
| boot-isolcpus/pp-uring-spin-0: uring-spin @ 0/s | 1108845 | 52.56 | 48.58 | 59.10 | 97.56 | 306.99 | 417.09 | 1061.57 |
| boot-isolcpus/pp-uring-spin-10000: uring-spin @ 10000/s | 874736 | 100.95 | 55.79 | 285.72 | 372.17 | 444.00 | 511.76 | 1101.91 |
| boot-isolcpus/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1251583 | 46.84 | 46.80 | 52.09 | 62.08 | 94.32 | 104.55 | 181.53 |
| boot-isolcpus/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189043 | 46.17 | 45.61 | 49.50 | 56.33 | 70.91 | 107.14 | 339.15 |
| boot-full-nomitig/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189938 | 46.69 | 46.10 | 50.23 | 58.65 | 70.45 | 88.99 | 1045.66 |
| boot-full-nomitig/pp-epoll-spin-0: epoll-spin @ 0/s | 1114043 | 52.87 | 49.09 | 58.88 | 100.81 | 134.97 | 143.46 | 245.79 |
| boot-full-nomitig/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188628 | 51.48 | 50.02 | 55.07 | 93.77 | 99.67 | 109.31 | 166.61 |
| boot-full-nomitig/pp-recv-busy-0: recv-busy @ 0/s | 1310151 | 44.95 | 43.72 | 50.70 | 60.06 | 90.44 | 102.83 | 416.89 |
| boot-full-nomitig/pp-recv-busy-10000: recv-busy @ 10000/s | 1179773 | 53.17 | 46.77 | 75.36 | 91.20 | 233.37 | 291.76 | 730.42 |
| boot-full-nomitig/pp-uring-spin-0: uring-spin @ 0/s | 1087910 | 53.51 | 49.15 | 62.77 | 98.91 | 111.86 | 144.16 | 1107.96 |
| boot-full-nomitig/pp-uring-spin-10000: uring-spin @ 10000/s | 1189221 | 53.78 | 50.27 | 59.71 | 97.48 | 115.57 | 135.74 | 210.34 |
| boot-full-nomitig/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1208416 | 48.57 | 47.34 | 54.67 | 91.08 | 100.58 | 124.22 | 305.72 |
| boot-full-nomitig/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1177394 | 53.31 | 47.27 | 73.29 | 95.64 | 242.10 | 298.82 | 1003.40 |
| boot-full/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189937 | 46.89 | 46.31 | 51.61 | 58.80 | 72.65 | 119.69 | 269.07 |
| boot-full/pp-epoll-spin-0: epoll-spin @ 0/s | 1144070 | 51.47 | 48.89 | 58.57 | 97.47 | 146.58 | 320.70 | 1035.94 |
| boot-full/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1181101 | 73.87 | 74.04 | 94.40 | 113.56 | 233.68 | 293.07 | 534.91 |
| boot-full/pp-recv-busy-0: recv-busy @ 0/s | 1274731 | 46.19 | 45.23 | 51.76 | 61.51 | 92.67 | 101.01 | 694.49 |
| boot-full/pp-recv-busy-10000: recv-busy @ 10000/s | 1186828 | 51.80 | 46.80 | 72.91 | 88.76 | 95.20 | 117.26 | 311.56 |
| boot-full/pp-uring-spin-0: uring-spin @ 0/s | 1156769 | 50.28 | 47.90 | 57.24 | 94.81 | 100.76 | 224.70 | 338.92 |
| boot-full/pp-uring-spin-10000: uring-spin @ 10000/s | 1177205 | 55.73 | 51.48 | 60.98 | 130.46 | 261.03 | 314.97 | 380.39 |
| boot-full/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1227519 | 47.80 | 47.62 | 52.71 | 76.05 | 98.42 | 107.09 | 1052.82 |
| boot-full/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189463 | 52.57 | 47.86 | 72.84 | 89.92 | 96.32 | 105.44 | 302.19 |

