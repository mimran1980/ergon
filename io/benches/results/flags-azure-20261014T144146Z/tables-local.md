# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|
| r1-base-0 | 61.31 | 256.0 | 42.0 | 0.0 | 61.31 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-housekeeping | 596.11 | 1081.5 | 56.5 | 3.0 | 596.11 µs stall: mostly unexplained (+timer tick_nohz_handler) |
| r1-prefer | 555.86 | 690.0 | 45.5 | 3.0 | 555.86 µs stall: mostly unexplained (+timer tick_nohz_handler) |
| r1-base-2 | 577.04 | 847.0 | 54.5 | 0.5 | 577.04 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-noquickack | 357.17 | 848.5 | 53.0 | 3.0 | 357.17 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nogro | 943.88 | 988.5 | 73.0 | 3.5 | 943.88 µs stall: mostly unexplained (+timer tick_nohz_handler) |
| r1-base-4 | 822.42 | 886.0 | 50.5 | 3.0 | 822.42 µs stall: mostly unexplained (+timer tick_nohz_handler) |
| r1-nocoalesce | 737.90 | 922.0 | 60.5 | 1.0 | 737.90 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-irqlocal | 899.29 | 2819.0 | 129.5 | 4.0 | 899.29 µs stall: mostly unexplained (+timer tick_nohz_handler) |
| r1-base-6 | 1082.74 | 673.5 | 57.0 | 4.5 | 1082.74 µs stall: mostly unexplained (+timer tick_nohz_handler) |
| r1-rt | 1063.24 | 338.5 | 55.0 | 4.0 | 1063.24 µs stall: mostly unexplained (+timer tick_nohz_handler) |
| r1-sibling | 881.71 | 28155.0 | 32.5 | 4.0 | 881.71 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-8 | 714.74 | 754.5 | 53.5 | 3.0 | 714.74 µs stall: mostly unexplained (+timer tick_nohz_handler) |
| r1-busypoll50 | 1045.73 | 809.5 | 51.5 | 3.5 | 1045.73 µs stall: timer tick_nohz_handler |
| r1-budget64 | 99.01 | 842.5 | 51.5 | 0.0 | 99.01 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 425.89 | 780.5 | 61.0 | 1.5 | 425.89 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-stock | 612.94 | 17189.0 | 120.0 | 3.0 | 612.94 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-isolcpus | 58.94 | 10656.0 | 47.0 | 0.0 | 58.94 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full-nomitig | 288.38 | 277.5 | 58.5 | 2.5 | 288.38 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full | 67.82 | 239.5 | 37.0 | 0.0 | 67.82 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |

## HTTP round trip, http-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189964 | 55.20 | 66.18 | 72.65 | 89.79 | 843.84 | 36.3 | 2.0 | 0.0 | 0.0 | 772.41 | 89.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-housekeeping | 1189963 | 48.30 (+0.01) | 66.36 (+0.90) | 72.50 (−0.15) | 84.38 (−5.41) | 651.83 (−217.49) | 34.8 | 1.5 | 0.0 | 0.0 | 566.14 | 81.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 1189963 | 50.63 (+2.35) | 67.46 (+1.99) | 74.50 (+1.85) | 87.47 (−2.31) | 704.78 (−164.54) | 39.3 | 1.0 | 0.0 | 0.0 | 815.17 | 113.5 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-base-2 | 1189963 | 48.10 | 65.39 | 72.44 | 86.10 | 660.62 | 31.3 | 1.0 | 0.0 | 0.0 | 570.14 | 78.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 1189963 | 48.22 (−0.06) | 65.88 (+0.41) | 72.25 (−0.39) | 80.97 (−8.82) | 919.11 (+49.79) | 24.2 | 2.0 | 0.0 | 0.0 | 908.91 | 98.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nogro | 1189963 | 48.13 (−0.15) | 65.12 (−0.35) | 72.00 (−0.65) | 81.21 (−8.57) | 1052.02 (+182.70) | 27.7 | 2.0 | 0.5 | 0.0 | 130.61 | 108.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1189964 | 47.89 | 65.42 | 72.24 | 81.97 | 869.32 | 30.3 | 1.5 | 0.0 | 0.0 | 747.97 | 83.5 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nocoalesce | 1189963 | 48.35 (+0.07) | 65.42 (−0.05) | 72.33 (−0.32) | 89.37 (−0.42) | 1086.28 (+216.96) | 39.3 | 3.0 | 0.5 | 0.0 | 964.57 | 83.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-irqlocal | 1189963 | 48.74 (+0.46) | 66.36 (+0.89) | 72.95 (+0.30) | 81.30 (−8.49) | 821.56 (−47.76) | 27.7 | 2.5 | 0.0 | 0.0 | 738.57 | 147.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1189963 | 48.28 | 65.47 | 72.98 | 216.30 | 1193.55 | 144.2 | 2.5 | 1.0 | 0.0 | 1365.12 | 87.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 1189963 | 48.02 (−0.27) | 65.58 (+0.11) | 72.23 (−0.41) | 83.48 (−6.30) | 971.90 (+102.58) | 29.7 | 3.0 | 0.0 | 0.0 | 604.41 | 80.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1189951 | 50.11 (+1.83) | 67.39 (+1.92) | 74.32 (+1.68) | 91.23 (+1.45) | 1084.88 (+215.55) | 49.9 | 2.0 | 1.0 | 0.0 | 1034.29 | 140.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-base-8 | 1189963 | 47.97 | 64.89 | 72.04 | 83.64 | 813.67 | 31.8 | 1.5 | 0.0 | 0.0 | 767.29 | 93.0 | client: mostly unexplained (+softirq TIMER) |
| r1-busypoll50 | 1189963 | 47.97 (−0.32) | 66.05 (+0.58) | 72.09 (−0.56) | 87.39 (−2.40) | 971.33 (+102.01) | 35.8 | 3.5 | 0.0 | 0.0 | 908.00 | 107.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-budget64 | 1189963 | 48.38 (+0.10) | 65.43 (−0.04) | 72.26 (−0.39) | 120.17 (+30.39) | 689.40 (−179.92) | 77.6 | 1.0 | 0.0 | 0.0 | 647.83 | 89.5 | client: preempted by swapper/2 |
| r1-base-10 | 1189963 | 48.53 | 66.44 | 73.42 | 234.70 | 871.67 | 163.9 | 2.5 | 0.0 | 0.0 | 812.00 | 91.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1189962 | 46.72 (−1.56) | 63.81 (−1.66) | 70.64 (−2.01) | 97.70 (+7.91) | 727.34 (−141.98) | 55.5 | 1.0 | 0.0 | 0.0 | 312.74 | 657.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1189963 | 48.12 (−0.16) | 67.40 (+1.93) | 72.27 (−0.38) | 87.14 (−2.65) | 944.93 (+75.61) | 32.8 | 2.0 | 0.0 | 0.0 | 911.46 | 216.5 | client: mostly unexplained (+timer tick_nohz_handler) |
| boot-full-nomitig | 1189964 | 47.69 (−0.59) | 65.58 (+0.11) | 72.36 (−0.29) | 219.18 (+129.39) | 1080.65 (+211.32) | 125.6 | 3.0 | 0.5 | 0.0 | 951.33 | 84.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full | 1189963 | 48.25 (−0.04) | 64.98 (−0.49) | 72.49 (−0.16) | 222.43 (+132.65) | 1763.35 (+894.03) | 137.7 | 4.0 | 1.0 | 0.0 | 661.27 | 92.0 | neither loop stalled long enough: network, hypervisor or untraced |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 2900050 | 3.75 | 7.81 | 11.62 | 21.26 | 475.98 | 51.7 | 0.0 | 0.0 | 0.0 | 1016.92 | 269.0 | 1016.92 µs stall: mostly unexplained (+timer tick_nohz_handler) |
| r1-housekeeping | 2900050 | 3.70 (−0.05) | 22.39 (−0.18) | 26.70 (+0.47) | 41.45 (+8.53) | 54.52 (−346.40) | 0.0 | 0.0 | 0.0 | 0.0 | 761.89 | 5008.0 | 761.89 µs stall: mostly unexplained (+softirq NET_RX) |
| r1-prefer | 2900000 | 5.37 (+1.62) | 20.91 (−1.66) | 25.97 (−0.26) | 30.43 (−2.49) | 45.66 (−355.27) | 0.0 | 0.0 | 0.0 | 0.0 | 762.10 | 6797.0 | 762.10 µs stall: loop work (50 items) |
| r1-base-2 | 2900000 | 3.75 | 7.39 | 10.49 | 18.69 | 96.77 | 0.0 | 0.0 | 0.0 | 0.0 | 383.46 | 229.0 | 383.46 µs stall: loop work (1550 items) |
| r1-noquickack | 2900000 | 3.76 (+0.01) | 7.90 (−14.68) | 12.13 (−14.10) | 22.16 (−10.77) | 263.42 (−137.51) | 51.7 | 0.0 | 0.0 | 0.0 | 694.21 | 376.0 | 694.21 µs stall: loop work (50 items) |
| r1-nogro | 2900067 | 3.72 (−0.03) | 22.32 (−0.26) | 26.48 (+0.25) | 41.11 (+8.19) | 763.99 (+363.07) | 5.2 | 5.2 | 0.0 | 0.0 | 764.97 | 5508.0 | 764.97 µs stall: mostly unexplained (+softirq NET_RX) |
| r1-base-4 | 2900000 | 3.70 | 23.39 | 26.31 | 31.89 | 76.52 | 0.0 | 0.0 | 0.0 | 0.0 | 419.98 | 5720.0 | 419.98 µs stall: loop work (1550 items) |
| r1-nocoalesce | 2900060 | 3.77 (+0.03) | 7.56 (−15.02) | 11.84 (−14.39) | 22.16 (−10.76) | 737.69 (+336.77) | 51.7 | 51.7 | 0.0 | 0.0 | 898.13 | 278.0 | 898.13 µs stall: loop work (50 items) |
| r1-irqlocal | 2900050 | 5.24 (+1.49) | 21.70 (−0.88) | 34.64 (+8.41) | 48.45 (+15.53) | 1329.36 (+928.44) | 12.4 | 3.1 | 3.1 | 0.0 | 1342.33 | 57720.0 | 1342.33 µs stall: softirq NET_RX |
| r1-base-6 | 2900000 | 3.78 | 22.03 | 27.20 | 43.17 | 400.92 | 51.7 | 0.0 | 0.0 | 0.0 | 720.33 | 4896.0 | 720.33 µs stall: loop work (50 items) |
| r1-rt | 2900000 | 3.69 (−0.06) | 22.65 (+0.08) | 26.13 (−0.10) | 31.31 (−1.61) | 63.31 (−337.61) | 0.0 | 0.0 | 0.0 | 0.0 | 606.98 | 5153.0 | 606.98 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-sibling | 2899950 | 4.75 (+1.00) | 23.61 (+1.04) | 27.52 (+1.29) | 33.43 (+0.51) | 84.65 (−316.28) | 0.0 | 0.0 | 0.0 | 0.0 | 654.96 | 8666.0 | 654.96 µs stall: loop work (2050 items) |
| r1-base-8 | 2900031 | 3.67 | 23.16 | 26.04 | 32.92 | 567.35 | 32.1 | 32.1 | 0.0 | 0.0 | 1238.36 | 5637.0 | 1238.36 µs stall: softirq NET_RX |
| r1-busypoll50 | 2899950 | 3.71 (−0.04) | 22.76 (+0.18) | 26.78 (+0.55) | 40.98 (+8.06) | 53.94 (−346.99) | 0.0 | 0.0 | 0.0 | 0.0 | 409.00 | 5360.0 | 409.00 µs stall: loop work (1550 items) |
| r1-budget64 | 2900000 | 3.63 (−0.12) | 23.07 (+0.49) | 26.05 (−0.18) | 31.75 (−1.17) | 68.94 (−331.98) | 0.0 | 0.0 | 0.0 | 0.0 | 402.16 | 5868.0 | 402.16 µs stall: loop work (1550 items) |
| r1-base-10 | 2900050 | 3.75 | 22.57 | 26.23 | 35.90 | 179.81 | 51.7 | 0.0 | 0.0 | 0.0 | 698.25 | 5398.0 | 698.25 µs stall: loop work (50 items) |
| boot-stock | 2899950 | 3.14 (−0.61) | 5.82 (−16.75) | 10.10 (−16.13) | 18.11 (−14.82) | 110.51 (−290.41) | 51.7 | 0.0 | 0.0 | 0.0 | 428.30 | 405.0 | 428.30 µs stall: loop work (1451 items) |
| boot-isolcpus | 2900000 | 3.67 (−0.08) | 22.79 (+0.21) | 26.96 (+0.73) | 39.50 (+6.57) | 422.72 (+21.80) | 51.7 | 0.0 | 0.0 | 0.0 | 1350.61 | 5935.0 | 1350.61 µs stall: mostly unexplained (+timer tick_nohz_handler) |
| boot-full-nomitig | 2900029 | 3.37 (−0.38) | 7.65 (−14.93) | 17.79 (−8.44) | 22.66 (−10.26) | 69.72 (−331.21) | 0.0 | 0.0 | 0.0 | 0.0 | 524.31 | 219.0 | 524.31 µs stall: loop work (50 items) |
| boot-full | 2900000 | 3.73 (−0.02) | 7.34 (−15.23) | 10.52 (−15.71) | 19.89 (−13.03) | 419.91 (+18.98) | 51.7 | 0.0 | 0.0 | 0.0 | 717.03 | 212.0 | 717.03 µs stall: loop work (50 items) |

## WebSocket round trip, pp-epoll-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1148630 | 47.30 | 72.20 | 79.40 | 104.14 | 787.20 | 147.5 | 4.1 | 0.0 | 0.0 | 779.17 | 109.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-2 | 1175472 | 47.05 | 71.38 | 77.38 | 109.12 | 817.81 | 155.6 | 4.1 | 0.0 | 0.0 | 71.23 | 73.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1184461 | 47.00 | 71.15 | 76.95 | 102.12 | 710.46 | 127.1 | 2.0 | 0.0 | 0.0 | 73.81 | 93.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 1225362 | 45.40 (−1.65) | 66.45 (−4.93) | 71.16 (−6.86) | 81.18 (−27.94) | 716.72 (−105.63) | 44.7 | 3.1 | 0.0 | 0.0 | 661.89 | 177.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-6 | 1185664 | 46.07 | 71.11 | 77.04 | 118.87 | 1016.62 | 187.1 | 2.0 | 1.0 | 0.0 | 967.69 | 74.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-base-8 | 1191764 | 45.73 | 71.00 | 78.02 | 98.81 | 822.35 | 113.9 | 4.1 | 0.0 | 0.0 | 789.32 | 100.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 1153435 | 47.32 | 72.19 | 79.18 | 168.62 | 977.38 | 220.7 | 4.1 | 0.0 | 0.0 | 118.73 | 137.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1198014 | 46.67 (−0.38) | 69.63 (−1.75) | 75.72 (−2.30) | 87.81 (−21.31) | 407.49 (−414.86) | 72.2 | 0.0 | 0.0 | 0.0 | 175.07 | 1470.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1183815 | 45.86 (−1.19) | 70.84 (−0.54) | 76.48 (−1.54) | 98.28 (−10.84) | 1760.12 (+937.76) | 113.9 | 4.1 | 2.0 | 0.0 | 1077.72 | 130.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1168780 | 46.71 (−0.34) | 71.48 (+0.10) | 78.00 (−0.01) | 104.64 (−4.48) | 581.48 (−240.87) | 132.2 | 2.0 | 0.0 | 0.0 | 402.03 | 88.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1184696 | 45.89 (−1.16) | 71.11 (−0.28) | 76.70 (−1.32) | 124.82 (+15.70) | 1424.90 (+602.55) | 197.3 | 2.0 | 1.0 | 0.0 | 1370.12 | 68.0 | client: mostly unexplained (+softirq TIMER) |

## WebSocket round trip, pp-epoll-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1188593 | 47.69 | 70.94 | 80.33 | 113.54 | 1026.47 | 83.7 | 6.6 | 0.5 | 0.0 | 749.61 | 87.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1188571 | 47.21 | 70.43 | 80.51 | 115.01 | 1776.39 | 84.2 | 4.5 | 2.0 | 0.0 | 1128.48 | 83.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1188655 | 47.08 | 70.14 | 79.83 | 102.54 | 635.83 | 64.5 | 0.5 | 0.0 | 0.0 | 455.73 | 79.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 1188721 | 47.86 (+0.17) | 63.39 (−7.55) | 71.50 (−8.54) | 80.17 (−38.02) | 2013.08 (+986.61) | 22.2 | 1.5 | 0.5 | 0.0 | 739.35 | 149.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1188552 | 47.79 | 71.33 | 79.48 | 132.52 | 1015.64 | 100.3 | 2.0 | 0.5 | 0.0 | 831.92 | 79.5 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-base-8 | 1188608 | 47.53 | 70.32 | 79.05 | 118.20 | 744.41 | 93.8 | 4.0 | 0.0 | 0.0 | 599.03 | 70.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1188452 | 47.86 | 71.33 | 80.04 | 223.45 | 2863.19 | 125.0 | 3.0 | 1.0 | 0.0 | 578.43 | 71.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1188737 | 46.57 (−1.12) | 68.96 (−1.98) | 76.68 (−3.36) | 221.96 (+103.76) | 2139.72 (+1113.26) | 137.1 | 2.0 | 1.0 | 0.0 | 1516.04 | 210.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1188924 | 47.15 (−0.53) | 69.18 (−1.77) | 77.68 (−2.36) | 116.71 (−1.49) | 1090.37 (+63.90) | 83.7 | 3.0 | 0.5 | 0.0 | 741.13 | 109.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full-nomitig | 1188460 | 46.97 (−0.72) | 70.57 (−0.38) | 79.04 (−1.00) | 250.75 (+132.55) | 1057.46 (+30.99) | 125.5 | 3.5 | 0.5 | 0.0 | 716.51 | 68.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1188341 | 47.49 (−0.20) | 70.18 (−0.76) | 79.42 (−0.62) | 258.46 (+140.26) | 1665.26 (+638.79) | 172.9 | 4.5 | 1.0 | 0.0 | 1645.67 | 63.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

## WebSocket round trip, pp-recv-busy-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1229379 | 45.35 | 65.61 | 70.04 | 82.73 | 666.61 | 55.9 | 3.1 | 0.0 | 0.0 | 593.50 | 99.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1252502 | 44.62 (−0.18) | 64.75 (−0.51) | 69.40 (−0.21) | 79.34 (−1.29) | 1296.00 (+518.06) | 51.9 | 3.1 | 2.0 | 0.0 | 1264.70 | 123.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-prefer | 1232012 | 44.95 (+0.15) | 67.25 (+1.99) | 75.14 (+5.52) | 81.51 (+0.88) | 792.37 (+14.43) | 36.6 | 2.0 | 0.0 | 0.0 | 69.52 | 280.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1233329 | 45.19 | 65.81 | 70.02 | 78.43 | 554.50 | 40.7 | 1.0 | 0.0 | 0.0 | 140.16 | 132.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 1242245 | 44.84 (+0.03) | 65.24 (−0.02) | 69.83 (+0.22) | 83.28 (+2.65) | 2738.55 (+1960.61) | 47.8 | 3.1 | 1.0 | 0.0 | 708.12 | 106.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 1233236 | 45.14 (+0.34) | 65.58 (+0.32) | 69.94 (+0.32) | 80.19 (−0.45) | 1006.44 (+228.51) | 48.8 | 2.0 | 1.0 | 0.0 | 653.16 | 106.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1241848 | 44.80 | 65.26 | 69.61 | 84.24 | 792.24 | 43.7 | 2.0 | 0.0 | 0.0 | 785.30 | 135.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nocoalesce | 1233234 | 44.99 (+0.18) | 66.66 (+1.40) | 71.54 (+1.92) | 81.00 (+0.37) | 861.61 (+83.67) | 47.8 | 1.0 | 0.0 | 0.0 | 593.64 | 122.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-irqlocal | 1226239 | 44.91 (+0.10) | 66.99 (+1.73) | 71.06 (+1.44) | 80.88 (+0.25) | 490.88 (−287.06) | 42.7 | 0.0 | 0.0 | 0.0 | 348.39 | 199.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1258348 | 44.29 | 64.56 | 69.17 | 80.63 | 1992.82 | 50.8 | 5.1 | 1.0 | 0.0 | 715.50 | 117.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 1233523 | 45.22 (+0.41) | 66.06 (+0.80) | 70.70 (+1.09) | 79.50 (−1.14) | 589.14 (−188.79) | 45.8 | 1.0 | 0.0 | 0.0 | 119.50 | 98.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1213942 | 45.56 (+0.76) | 67.18 (+1.92) | 72.28 (+2.67) | 80.87 (+0.23) | 1869.68 (+1091.74) | 49.8 | 3.1 | 1.0 | 0.0 | 165.92 | 105.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1251495 | 44.73 | 64.90 | 69.44 | 79.09 | 777.94 | 43.7 | 4.1 | 0.0 | 0.0 | 544.50 | 99.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-busypoll50 | 1250397 | 44.52 (−0.28) | 64.72 (−0.54) | 69.32 (−0.29) | 78.63 (−2.00) | 1081.74 (+303.80) | 42.7 | 2.0 | 2.0 | 0.0 | 985.01 | 114.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-budget64 | 1239347 | 44.84 (+0.04) | 65.31 (+0.05) | 69.99 (+0.38) | 86.17 (+5.53) | 983.21 (+205.28) | 76.3 | 2.0 | 0.0 | 0.0 | 127.84 | 107.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1400052 | 41.34 | 56.18 | 65.20 | 72.27 | 593.66 | 25.4 | 1.0 | 0.0 | 0.0 | 558.20 | 103.0 | client: mostly unexplained (+softirq NET_RX) |
| boot-stock | 1240720 | 45.55 (+0.75) | 64.63 (−0.63) | 69.82 (+0.21) | 99.80 (+19.17) | 724.25 (−53.68) | 126.1 | 1.0 | 0.0 | 0.0 | 166.61 | 301.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1232266 | 45.15 (+0.34) | 66.44 (+1.18) | 76.32 (+6.71) | 92.36 (+11.73) | 617.35 (−160.59) | 71.2 | 2.0 | 0.0 | 0.0 | 473.85 | 179.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1235449 | 45.10 (+0.29) | 65.61 (+0.35) | 69.84 (+0.23) | 83.95 (+3.32) | 2116.75 (+1338.81) | 85.4 | 4.1 | 1.0 | 0.0 | 793.43 | 102.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1241234 | 44.86 (+0.05) | 65.25 (−0.01) | 69.78 (+0.17) | 84.75 (+4.12) | 469.00 (−308.94) | 87.5 | 0.0 | 0.0 | 0.0 | 65.85 | 106.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1184596 | 47.14 | 62.67 | 70.65 | 83.51 | 800.67 | 36.3 | 1.0 | 0.0 | 0.0 | 518.04 | 93.5 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-housekeeping | 1184592 | 46.75 (−0.19) | 63.12 (+0.87) | 71.67 (+1.61) | 84.98 (+5.10) | 1571.00 (+54.28) | 26.2 | 4.0 | 1.0 | 0.0 | 1546.12 | 95.5 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-prefer | 1173274 | 50.23 (+3.28) | 66.64 (+4.39) | 74.39 (+4.33) | 90.47 (+10.59) | 853.14 (−663.59) | 40.3 | 2.5 | 0.0 | 0.0 | 806.12 | 245.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1184624 | 46.94 | 62.15 | 70.06 | 79.85 | 1707.45 | 25.7 | 1.5 | 0.5 | 0.0 | 631.18 | 90.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 1184618 | 46.74 (−0.20) | 62.02 (−0.23) | 69.60 (−0.46) | 79.97 (+0.09) | 625.63 (−891.09) | 22.2 | 1.0 | 0.0 | 0.0 | 579.55 | 138.5 | client: preempted by swapper/2 |
| r1-nogro | 1184484 | 46.97 (+0.03) | 62.47 (+0.21) | 70.39 (+0.34) | 87.58 (+7.70) | 2423.47 (+906.75) | 46.4 | 3.0 | 1.0 | 0.0 | 1063.36 | 94.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1184619 | 46.89 | 62.01 | 69.83 | 80.34 | 675.08 | 28.7 | 1.0 | 0.0 | 0.0 | 636.76 | 91.0 | client: preempted by swapper/2 |
| r1-nocoalesce | 1184573 | 46.68 (−0.26) | 62.27 (+0.02) | 70.29 (+0.24) | 81.86 (+1.98) | 2734.75 (+1218.03) | 28.2 | 3.0 | 0.5 | 0.0 | 852.51 | 90.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 1184701 | 46.73 (−0.21) | 62.99 (+0.74) | 70.91 (+0.85) | 78.55 (−1.33) | 1017.25 (−499.48) | 23.7 | 1.5 | 0.5 | 0.0 | 623.31 | 164.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-6 | 1184629 | 46.82 | 62.02 | 69.79 | 78.95 | 799.37 | 26.2 | 2.5 | 0.0 | 0.0 | 597.25 | 94.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-rt | 1184611 | 46.87 (−0.07) | 62.32 (+0.07) | 70.08 (+0.03) | 80.29 (+0.41) | 886.10 (−630.63) | 26.7 | 2.5 | 0.0 | 0.0 | 840.33 | 106.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-sibling | 1183310 | 48.86 (+1.91) | 64.96 (+2.71) | 72.75 (+2.69) | 83.58 (+3.70) | 892.80 (−623.92) | 35.8 | 5.0 | 0.0 | 0.0 | 667.52 | 96.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1184590 | 46.94 | 62.26 | 69.99 | 79.88 | 1516.72 | 26.2 | 2.5 | 1.0 | 0.0 | 1500.26 | 95.5 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-busypoll50 | 1184519 | 46.87 (−0.08) | 62.15 (−0.10) | 69.89 (−0.17) | 80.78 (+0.90) | 1375.38 (−141.35) | 28.7 | 2.5 | 1.5 | 0.0 | 1357.90 | 91.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-budget64 | 1184370 | 46.78 (−0.16) | 62.27 (+0.02) | 70.47 (+0.41) | 221.58 (+141.70) | 651.24 (−865.49) | 133.6 | 1.0 | 0.0 | 0.0 | 539.14 | 68.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1184585 | 47.01 | 62.25 | 70.19 | 79.53 | 1870.93 | 27.2 | 1.5 | 0.5 | 0.0 | 735.05 | 100.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1184529 | 44.06 (−2.88) | 59.83 (−2.42) | 68.26 (−1.80) | 224.24 (+144.36) | 2465.00 (+948.28) | 157.8 | 1.5 | 0.5 | 0.0 | 422.36 | 364.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1184877 | 46.52 (−0.42) | 62.02 (−0.23) | 69.52 (−0.54) | 82.65 (+2.77) | 685.36 (−831.37) | 36.8 | 0.5 | 0.0 | 0.0 | 645.26 | 147.0 | client: preempted by swapper/2 |
| boot-full-nomitig | 1184857 | 45.99 (−0.96) | 61.28 (−0.97) | 69.03 (−1.03) | 81.86 (+1.98) | 873.56 (−643.17) | 32.3 | 6.6 | 0.0 | 0.0 | 836.04 | 99.5 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full | 1184389 | 46.75 (−0.19) | 62.15 (−0.10) | 70.27 (+0.21) | 222.29 (+142.41) | 1217.19 (−299.54) | 126.6 | 2.5 | 1.0 | 0.0 | 654.66 | 98.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189652 | 45.16 | 68.89 | 76.60 | 90.14 | 1053.23 | 65.1 | 3.1 | 1.0 | 0.0 | 1028.82 | 60.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-housekeeping | 1209123 | 44.54 (−1.96) | 67.95 (−3.00) | 73.21 (−4.87) | 93.89 (−9.44) | 2020.14 (+966.91) | 99.7 | 5.1 | 1.0 | 0.0 | 683.55 | 63.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1163441 | 46.51 | 71.12 | 78.08 | 98.67 | 1158.51 | 107.8 | 3.1 | 1.0 | 0.0 | 948.11 | 109.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-noquickack | 1206277 | 44.59 (−1.92) | 68.08 (−2.86) | 73.39 (−4.69) | 92.73 (−10.60) | 1662.08 (+608.85) | 96.6 | 2.0 | 1.0 | 0.0 | 1595.41 | 100.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-nogro | 1170154 | 46.52 (+0.02) | 70.92 (−0.02) | 78.25 (+0.17) | 107.87 (+4.54) | 671.78 (−381.45) | 145.4 | 4.1 | 0.0 | 0.0 | 598.18 | 104.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1169837 | 46.63 | 70.94 | 78.24 | 103.33 | 1876.16 | 141.4 | 2.0 | 1.0 | 0.0 | 125.99 | 135.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 1178835 | 47.63 (+1.12) | 71.42 (+0.47) | 79.16 (+1.08) | 96.39 (−6.94) | 1081.10 (+27.87) | 102.7 | 8.1 | 2.0 | 0.0 | 541.55 | 69.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 1217470 | 44.97 (−1.54) | 66.66 (−4.28) | 71.14 (−6.94) | 80.35 (−22.98) | 611.59 (−441.64) | 38.6 | 2.0 | 0.0 | 0.0 | 335.72 | 174.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1211540 | 46.28 | 69.84 | 75.46 | 142.15 | 672.90 | 184.1 | 4.1 | 0.0 | 0.0 | 536.40 | 60.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-rt | 1164334 | 46.57 (+0.06) | 71.09 (+0.15) | 78.06 (−0.01) | 100.50 (−2.83) | 526.02 (−527.21) | 124.1 | 1.0 | 0.0 | 0.0 | 132.45 | 92.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1195473 | 44.57 (−1.94) | 68.68 (−2.26) | 77.98 (−0.10) | 106.63 (+3.30) | 973.88 (−79.35) | 150.5 | 4.1 | 0.0 | 0.0 | 943.31 | 72.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| r1-base-8 | 1211377 | 44.52 | 68.14 | 75.86 | 91.54 | 536.43 | 69.2 | 1.0 | 0.0 | 0.0 | 306.79 | 85.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1147602 | 46.87 | 71.68 | 78.23 | 236.23 | 737.08 | 339.7 | 3.1 | 0.0 | 0.0 | 707.56 | 90.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-stock | 1194515 | 46.36 (−0.15) | 69.12 (−1.83) | 75.30 (−2.78) | 113.44 (+10.11) | 585.35 (−467.88) | 145.4 | 1.0 | 0.0 | 0.0 | 205.12 | 175.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1204396 | 44.79 (−1.72) | 68.47 (−2.47) | 73.74 (−4.34) | 90.40 (−12.93) | 723.82 (−329.41) | 79.3 | 2.0 | 0.0 | 0.0 | 695.71 | 131.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full-nomitig | 1159169 | 46.24 (−0.27) | 71.35 (+0.41) | 79.79 (+1.71) | 104.21 (+0.88) | 841.77 (−211.46) | 136.3 | 3.1 | 0.0 | 0.0 | 784.74 | 68.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| boot-full | 1176189 | 46.47 (−0.04) | 70.58 (−0.36) | 76.63 (−1.45) | 143.44 (+40.11) | 717.51 (−335.72) | 199.3 | 2.0 | 0.0 | 0.0 | 710.26 | 80.0 | client: mostly unexplained (+timer tick_nohz_handler) |

## WebSocket round trip, pp-uring-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189517 | 48.10 | 71.29 | 79.77 | 116.14 | 2942.13 | 88.2 | 2.5 | 1.5 | 0.0 | 1221.89 | 63.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1189582 | 47.74 (−0.10) | 70.94 (+0.02) | 80.39 (+0.60) | 116.31 (−5.03) | 946.29 (+77.66) | 95.8 | 1.0 | 0.0 | 0.0 | 99.94 | 83.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1189551 | 47.84 | 70.92 | 79.27 | 121.33 | 745.10 | 87.2 | 2.5 | 0.0 | 0.0 | 638.30 | 62.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 1189575 | 47.93 (+0.09) | 72.19 (+1.27) | 81.55 (+1.76) | 110.48 (−10.85) | 814.10 (−54.54) | 77.6 | 1.0 | 0.0 | 0.0 | 710.00 | 82.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 1189595 | 47.64 (−0.20) | 70.51 (−0.41) | 79.02 (−0.77) | 118.09 (−3.24) | 750.03 (−118.61) | 84.2 | 2.5 | 0.0 | 0.0 | 652.95 | 65.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1189585 | 47.72 | 70.64 | 79.79 | 116.70 | 868.63 | 98.3 | 1.0 | 0.0 | 0.0 | 679.71 | 58.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 1189542 | 49.35 (+1.51) | 73.47 (+2.55) | 81.38 (+1.59) | 124.76 (+3.43) | 1087.59 (+218.96) | 100.8 | 4.0 | 1.0 | 0.0 | 611.94 | 66.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 1189686 | 47.69 (−0.15) | 63.21 (−7.71) | 71.21 (−8.58) | 81.05 (−40.28) | 678.35 (−190.28) | 28.7 | 1.5 | 0.0 | 0.0 | 643.07 | 246.5 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-6 | 1189385 | 47.57 | 70.84 | 79.84 | 238.32 | 711.74 | 162.4 | 2.5 | 0.0 | 0.0 | 556.60 | 64.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 1189567 | 47.39 (−0.45) | 70.44 (−0.48) | 80.24 (+0.46) | 110.26 (−11.07) | 1865.76 (+997.12) | 80.7 | 3.5 | 1.0 | 0.0 | 798.51 | 70.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1189448 | 48.77 (+0.94) | 72.26 (+1.34) | 81.80 (+2.01) | 123.22 (+1.89) | 1011.41 (+142.77) | 94.3 | 3.0 | 0.5 | 0.0 | 598.16 | 75.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1189558 | 47.76 | 70.49 | 79.38 | 117.42 | 1894.53 | 85.7 | 3.5 | 1.0 | 0.0 | 1099.82 | 65.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1189240 | 48.40 | 72.77 | 82.28 | 265.25 | 867.94 | 212.8 | 3.5 | 0.0 | 0.0 | 803.09 | 76.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| boot-stock | 1189643 | 46.30 (−1.53) | 68.19 (−2.74) | 75.94 (−3.85) | 90.62 (−30.71) | 564.97 (−303.67) | 43.9 | 1.0 | 0.0 | 0.0 | 616.54 | 167.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1189557 | 47.41 (−0.43) | 69.61 (−1.31) | 77.96 (−1.83) | 112.25 (−9.08) | 1967.85 (+1099.22) | 75.1 | 3.5 | 1.0 | 0.0 | 584.09 | 86.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1189551 | 46.91 (−0.93) | 69.89 (−1.03) | 78.26 (−1.53) | 118.83 (−2.50) | 2673.12 (+1804.49) | 86.7 | 3.0 | 1.0 | 0.0 | 741.42 | 68.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1189346 | 47.70 (−0.14) | 70.64 (−0.28) | 80.59 (+0.80) | 235.43 (+114.10) | 2149.10 (+1280.47) | 169.9 | 2.0 | 0.5 | 0.0 | 753.94 | 67.5 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-sqpoll-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1314302 | 42.33 | 64.13 | 69.90 | 77.43 | 565.56 | 47.8 | 1.0 | 0.0 | 0.0 | 560.70 | 60.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-housekeeping | 1250649 | 44.62 (−0.41) | 64.83 (−0.90) | 70.25 (−1.68) | 80.03 (−2.25) | 573.64 (−33.06) | 47.8 | 1.0 | 0.0 | 0.0 | 569.74 | 60.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-prefer | 1207541 | 45.85 (+0.82) | 69.34 (+3.61) | 76.97 (+5.04) | 82.44 (+0.15) | 405.95 (−200.75) | 43.7 | 0.0 | 0.0 | 0.0 | 153.21 | 77.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1231958 | 45.03 | 66.17 | 72.24 | 80.91 | 530.64 | 58.0 | 1.0 | 0.0 | 0.0 | 115.24 | 56.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1238039 | 44.79 | 65.73 | 71.66 | 80.33 | 606.70 | 42.7 | 1.0 | 0.0 | 0.0 | 251.15 | 55.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1235196 | 45.27 | 65.11 | 71.93 | 109.85 | 495.86 | 142.4 | 0.0 | 0.0 | 0.0 | 447.17 | 55.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-sibling | 1217344 | 45.45 (+0.42) | 67.43 (+1.70) | 77.05 (+5.12) | 105.68 (+23.40) | 451.81 (−154.89) | 147.5 | 0.0 | 0.0 | 0.0 | 72.30 | 39.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1243682 | 44.62 | 65.48 | 71.62 | 82.28 | 830.44 | 56.9 | 2.0 | 0.0 | 0.0 | 816.73 | 62.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 1211168 | 45.92 | 67.16 | 73.86 | 202.67 | 948.26 | 226.8 | 2.0 | 0.0 | 0.0 | 947.02 | 82.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-stock | 1230055 | 45.74 (+0.71) | 65.22 (−0.51) | 70.79 (−1.14) | 83.75 (+1.47) | 585.21 (−21.49) | 67.1 | 3.1 | 0.0 | 0.0 | 579.66 | 415.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-isolcpus | 1238887 | 44.48 (−0.55) | 65.59 (−0.14) | 71.01 (−0.92) | 79.20 (−3.09) | 505.98 (−100.72) | 56.9 | 1.0 | 0.0 | 0.0 | 426.71 | 76.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1335972 | 42.13 (−2.91) | 63.40 (−2.33) | 68.67 (−3.26) | 76.10 (−6.18) | 478.04 (−128.66) | 43.7 | 0.0 | 0.0 | 0.0 | 398.57 | 62.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1232174 | 45.08 (+0.05) | 65.56 (−0.17) | 71.55 (−0.38) | 101.85 (+19.57) | 601.35 (−5.35) | 128.1 | 1.0 | 0.0 | 0.0 | 71.00 | 50.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-sqpoll-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189610 | 48.19 | 64.32 | 70.73 | 83.22 | 626.13 | 34.8 | 2.0 | 0.0 | 0.0 | 647.66 | 53.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-housekeeping | 1189600 | 47.32 (−0.31) | 62.51 (−0.59) | 70.47 (−0.58) | 81.33 (−1.89) | 1006.37 (+2.99) | 30.3 | 3.0 | 0.5 | 0.0 | 999.85 | 55.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-prefer | 1189659 | 50.12 (+2.49) | 66.24 (+3.14) | 73.61 (+2.56) | 82.67 (−0.55) | 1898.78 (+895.40) | 24.7 | 2.5 | 0.5 | 0.0 | 680.73 | 58.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1189623 | 47.58 | 63.00 | 71.22 | 82.31 | 700.86 | 28.2 | 1.5 | 0.0 | 0.0 | 419.37 | 50.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1189609 | 47.63 | 63.10 | 71.05 | 85.63 | 454.43 | 29.7 | 0.0 | 0.0 | 0.0 | 422.06 | 65.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1189315 | 47.72 | 63.17 | 71.84 | 222.34 | 1052.64 | 156.3 | 2.5 | 0.5 | 0.0 | 440.84 | 52.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1189393 | 47.85 (+0.22) | 63.38 (+0.27) | 71.47 (+0.42) | 160.66 (+77.43) | 681.63 (−321.75) | 79.2 | 1.5 | 0.0 | 0.0 | 931.33 | 49.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1189575 | 47.38 | 62.61 | 70.68 | 80.36 | 1612.84 | 31.3 | 3.5 | 0.5 | 0.0 | 1583.14 | 53.5 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 1189595 | 47.59 | 62.89 | 71.05 | 82.78 | 1003.38 | 35.8 | 2.0 | 0.5 | 0.0 | 635.09 | 58.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1189384 | 45.31 (−2.33) | 60.98 (−2.13) | 68.89 (−2.16) | 106.12 (+22.90) | 1138.77 (+135.39) | 64.5 | 1.0 | 0.5 | 0.0 | 1135.52 | 3020.0 | client: mostly unexplained (+timer tick_nohz_handler) |
| boot-isolcpus | 1189507 | 46.82 (−0.81) | 62.62 (−0.48) | 70.14 (−0.91) | 79.36 (−3.86) | 738.39 (−264.99) | 30.3 | 2.0 | 0.0 | 0.0 | 771.02 | 71.5 | client: mostly unexplained (+timer tick_nohz_handler) |
| boot-full-nomitig | 1189542 | 46.60 (−1.03) | 62.00 (−1.10) | 69.70 (−1.35) | 86.69 (+3.47) | 757.81 (−245.57) | 45.4 | 3.0 | 0.0 | 0.0 | 702.09 | 51.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full | 1189326 | 47.86 (+0.22) | 63.11 (+0.00) | 71.62 (+0.57) | 228.44 (+145.22) | 1818.36 (+814.98) | 142.2 | 2.5 | 0.5 | 0.0 | 125.17 | 50.0 | neither loop stalled long enough: network, hypervisor or untraced |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### http-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 843.84 at 48.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 751.58 at 108.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 746.27 at 39.4 s: client: preempted by swapper/2; 642.58 at 46.2 s: neither loop stalled long enough: network, hypervisor or untraced; 467.18 at 46.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 651.83 at 117.9 s: neither loop stalled long enough: network, hypervisor or untraced; 623.84 at 44.8 s: client: preempted by swapper/2; 585.61 at 59.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 493.07 at 57.9 s: neither loop stalled long enough: network, hypervisor or untraced; 438.49 at 117.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 704.78 at 51.0 s: client: mostly unexplained (+timer tick_nohz_handler); 604.51 at 49.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 482.95 at 49.1 s: neither loop stalled long enough: network, hypervisor or untraced; 443.61 at 49.1 s: neither loop stalled long enough: network, hypervisor or untraced; 423.54 at 108.8 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-2 | 660.62 at 93.1 s: neither loop stalled long enough: network, hypervisor or untraced; 614.47 at 47.0 s: client: preempted by swapper/2; 474.59 at 93.1 s: neither loop stalled long enough: network, hypervisor or untraced; 442.84 at 93.1 s: neither loop stalled long enough: network, hypervisor or untraced; 405.46 at 33.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 919.11 at 11.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 757.79 at 9.2 s: neither loop stalled long enough: network, hypervisor or untraced; 600.14 at 9.2 s: neither loop stalled long enough: network, hypervisor or untraced; 533.79 at 69.0 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 443.10 at 69.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 1052.02 at 107.2 s: neither loop stalled long enough: network, hypervisor or untraced; 845.59 at 47.4 s: neither loop stalled long enough: network, hypervisor or untraced; 773.14 at 47.4 s: neither loop stalled long enough: network, hypervisor or untraced; 663.53 at 107.2 s: neither loop stalled long enough: network, hypervisor or untraced; 345.10 at 107.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 869.32 at 105.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 787.20 at 53.5 s: client: preempted by swapper/2; 641.33 at 103.2 s: neither loop stalled long enough: network, hypervisor or untraced; 419.67 at 43.0 s: neither loop stalled long enough: network, hypervisor or untraced; 380.86 at 103.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 1086.28 at 29.6 s: client: mostly unexplained (+timer tick_nohz_handler); 870.58 at 29.6 s: client: mostly unexplained (+softirq NET_RX); 847.77 at 89.5 s: client: mostly unexplained (+softirq NET_RX); 654.85 at 46.2 s: client: preempted by swapper/2; 588.92 at 54.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 821.56 at 86.8 s: neither loop stalled long enough: network, hypervisor or untraced; 774.51 at 28.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 754.11 at 86.8 s: neither loop stalled long enough: network, hypervisor or untraced; 734.23 at 28.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 501.58 at 26.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1193.55 at 18.5 s: neither loop stalled long enough: network, hypervisor or untraced; 1043.31 at 18.6 s: client: mostly unexplained (+timer tick_nohz_handler); 848.10 at 76.9 s: neither loop stalled long enough: network, hypervisor or untraced; 530.02 at 76.9 s: neither loop stalled long enough: network, hypervisor or untraced; 507.26 at 78.5 s: client: mostly unexplained (+softirq NET_RX) |
| r1-rt | 971.90 at 108.7 s: neither loop stalled long enough: network, hypervisor or untraced; 619.80 at 110.6 s: client: mostly unexplained (+timer tick_nohz_handler); 607.39 at 50.7 s: client: mostly unexplained (+timer tick_nohz_handler); 574.80 at 48.8 s: neither loop stalled long enough: network, hypervisor or untraced; 568.23 at 108.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1084.88 at 66.3 s: client: mostly unexplained (+timer tick_nohz_handler); 1050.76 at 66.3 s: client: mostly unexplained (+timer tick_nohz_handler); 851.36 at 64.7 s: neither loop stalled long enough: network, hypervisor or untraced; 794.67 at 6.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 416.41 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 813.67 at 116.5 s: client: mostly unexplained (+softirq TIMER); 685.91 at 54.8 s: neither loop stalled long enough: network, hypervisor or untraced; 523.76 at 114.6 s: neither loop stalled long enough: network, hypervisor or untraced; 464.58 at 114.6 s: neither loop stalled long enough: network, hypervisor or untraced; 456.40 at 114.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-busypoll50 | 971.33 at 79.0 s: client: mostly unexplained (+timer tick_nohz_handler); 896.71 at 19.0 s: client: mostly unexplained (+timer tick_nohz_handler); 887.89 at 79.0 s: client: mostly unexplained (+timer tick_nohz_handler); 743.55 at 115.1 s: neither loop stalled long enough: network, hypervisor or untraced; 686.48 at 19.0 s: client: mostly unexplained (+timer tick_nohz_handler) |
| r1-budget64 | 689.40 at 114.0 s: client: preempted by swapper/2; 517.70 at 93.0 s: neither loop stalled long enough: network, hypervisor or untraced; 479.79 at 33.4 s: neither loop stalled long enough: network, hypervisor or untraced; 456.24 at 28.8 s: neither loop stalled long enough: network, hypervisor or untraced; 426.32 at 33.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 871.67 at 2.0 s: neither loop stalled long enough: network, hypervisor or untraced; 841.55 at 3.9 s: client: mostly unexplained (+softirq TIMER); 816.45 at 63.9 s: client: mostly unexplained (+softirq NET_RX); 687.40 at 62.0 s: neither loop stalled long enough: network, hypervisor or untraced; 628.99 at 63.9 s: client: mostly unexplained (+softirq NET_RX) |
| boot-stock | 727.34 at 93.2 s: neither loop stalled long enough: network, hypervisor or untraced; 501.07 at 33.2 s: neither loop stalled long enough: network, hypervisor or untraced; 416.39 at 33.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 415.33 at 39.0 s: neither loop stalled long enough: network, hypervisor or untraced; 391.43 at 93.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 944.93 at 36.6 s: client: mostly unexplained (+timer tick_nohz_handler); 719.95 at 96.5 s: client: mostly unexplained (+timer tick_nohz_handler); 569.61 at 35.1 s: neither loop stalled long enough: network, hypervisor or untraced; 513.94 at 36.6 s: client: mostly unexplained (+softirq NET_RX); 488.61 at 35.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1080.65 at 32.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 954.25 at 92.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 865.59 at 32.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 665.60 at 32.9 s: client: mostly unexplained (+softirq NET_RX); 586.36 at 92.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full | 1763.35 at 41.4 s: neither loop stalled long enough: network, hypervisor or untraced; 1151.37 at 82.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 874.23 at 22.4 s: neither loop stalled long enough: network, hypervisor or untraced; 692.23 at 83.9 s: client: mostly unexplained (+softirq NET_RX); 680.19 at 23.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |

### md-recv-busy

| condition | worst events |
|---|---|
| r1-base-0 | 475.98 at 13.6 s: client: mostly unexplained (+timer tick_nohz_handler); 475.82 at 13.6 s: client: mostly unexplained (+timer tick_nohz_handler); 475.66 at 13.6 s: client: mostly unexplained (+timer tick_nohz_handler); 475.50 at 13.6 s: client: mostly unexplained (+timer tick_nohz_handler); 475.35 at 13.6 s: client: mostly unexplained (+timer tick_nohz_handler) |
| r1-housekeeping | 54.52 at 14.9 s: neither loop stalled long enough: network, hypervisor or untraced; 50.06 at 16.2 s: client: mostly unexplained (+timer tick_nohz_handler); 49.98 at 4.9 s: client: loop work (3 items); 49.89 at 16.2 s: client: mostly unexplained (+timer tick_nohz_handler); 49.82 at 4.9 s: client: loop work (3 items) |
| r1-prefer | 45.66 at 35.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 45.31 at 47.7 s: client: loop work (2 items); 45.16 at 47.7 s: client: loop work (2 items); 45.00 at 47.7 s: client: loop work (2 items); 44.84 at 47.7 s: client: loop work (2 items) |
| r1-base-2 | 96.77 at 56.6 s: client: loop work (7 items); 96.57 at 56.6 s: client: loop work (7 items); 96.36 at 56.6 s: client: loop work (7 items); 96.15 at 56.6 s: client: loop work (7 items); 95.96 at 56.6 s: client: loop work (7 items) |
| r1-noquickack | 263.42 at 33.9 s: client: loop work (50 items); 263.28 at 33.9 s: client: loop work (50 items); 263.13 at 33.9 s: client: loop work (50 items); 262.99 at 33.9 s: client: loop work (50 items); 262.85 at 33.9 s: client: loop work (50 items) |
| r1-nogro | 763.99 at 14.9 s: client: mostly unexplained (+softirq NET_RX); 763.83 at 14.9 s: client: mostly unexplained (+softirq NET_RX); 763.66 at 14.9 s: client: mostly unexplained (+softirq NET_RX); 763.50 at 14.9 s: client: mostly unexplained (+softirq NET_RX); 763.28 at 14.9 s: client: mostly unexplained (+softirq NET_RX) |
| r1-base-4 | 76.52 at 18.8 s: client: loop work (8 items); 76.36 at 18.8 s: client: loop work (8 items); 76.20 at 18.8 s: client: loop work (8 items); 76.04 at 18.8 s: client: loop work (8 items); 75.88 at 18.8 s: client: loop work (8 items) |
| r1-nocoalesce | 737.69 at 56.4 s: client: loop work (50 items); 737.53 at 56.4 s: client: loop work (50 items); 737.38 at 56.4 s: client: loop work (50 items); 737.22 at 56.4 s: client: loop work (50 items); 737.06 at 56.4 s: client: loop work (50 items) |
| r1-irqlocal | 1329.36 at 54.5 s: client: softirq NET_RX; 1329.19 at 54.5 s: client: softirq NET_RX; 1328.51 at 54.5 s: client: softirq NET_RX; 113.41 at 29.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 113.24 at 29.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-6 | 400.92 at 45.8 s: client: loop work (50 items); 400.76 at 45.8 s: client: loop work (50 items); 400.61 at 45.8 s: client: loop work (50 items); 400.32 at 45.8 s: client: loop work (50 items); 400.05 at 45.8 s: client: loop work (50 items) |
| r1-rt | 63.31 at 33.7 s: client: mostly unexplained (+softirq RCU); 48.56 at 11.7 s: client: mostly unexplained (+softirq RCU); 48.45 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced; 48.28 at 40.4 s: client: loop work (2 items); 48.28 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 84.65 at 24.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 84.41 at 24.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 84.17 at 24.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 83.93 at 24.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 70.96 at 34.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-8 | 567.35 at 21.4 s: client: mostly unexplained (+timer tick_nohz_handler); 567.14 at 21.4 s: client: mostly unexplained (+timer tick_nohz_handler); 566.94 at 21.4 s: client: mostly unexplained (+timer tick_nohz_handler); 566.75 at 21.4 s: client: mostly unexplained (+timer tick_nohz_handler); 566.55 at 21.4 s: client: mostly unexplained (+timer tick_nohz_handler) |
| r1-busypoll50 | 53.94 at 23.5 s: client: mostly unexplained (+softirq RCU); 53.76 at 23.5 s: client: mostly unexplained (+softirq RCU); 48.66 at 26.3 s: neither loop stalled long enough: network, hypervisor or untraced; 48.49 at 26.3 s: neither loop stalled long enough: network, hypervisor or untraced; 48.32 at 26.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-budget64 | 68.94 at 43.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 68.66 at 43.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 68.35 at 43.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 68.03 at 43.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 67.74 at 43.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 179.81 at 31.3 s: client: loop work (50 items); 179.62 at 31.3 s: client: loop work (50 items); 179.43 at 31.3 s: client: loop work (50 items); 179.21 at 31.3 s: client: loop work (50 items); 179.01 at 31.3 s: client: loop work (50 items) |
| boot-stock | 110.51 at 9.2 s: client: irq nvme0q0; 110.22 at 9.2 s: client: irq nvme0q0; 109.91 at 9.2 s: client: irq nvme0q0; 109.56 at 9.2 s: client: irq nvme0q0; 109.30 at 9.2 s: client: irq nvme0q0 |
| boot-isolcpus | 422.72 at 7.8 s: client: loop work (50 items); 422.40 at 7.8 s: client: loop work (50 items); 422.08 at 7.8 s: client: loop work (50 items); 421.74 at 7.8 s: client: loop work (50 items); 421.43 at 7.8 s: client: loop work (50 items) |
| boot-full-nomitig | 69.72 at 41.0 s: neither loop stalled long enough: network, hypervisor or untraced; 69.56 at 41.0 s: neither loop stalled long enough: network, hypervisor or untraced; 69.40 at 41.0 s: neither loop stalled long enough: network, hypervisor or untraced; 69.24 at 41.0 s: neither loop stalled long enough: network, hypervisor or untraced; 69.07 at 41.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 419.91 at 56.4 s: client: loop work (50 items); 419.77 at 56.4 s: client: loop work (50 items); 419.62 at 56.4 s: client: loop work (50 items); 419.48 at 56.4 s: client: loop work (50 items); 419.34 at 56.4 s: client: loop work (50 items) |

### noise

| condition | worst events |
|---|---|
| r1-base-0 | 61.31 at 0.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 46.78 at 2.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 45.80 at 118.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 43.40 at 5.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 43.10 at 86.6 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-housekeeping | 596.11 at 117.0 s: mostly unexplained (+timer tick_nohz_handler); 558.45 at 57.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 482.40 at 117.0 s: mostly unexplained (+timer tick_nohz_handler); 265.11 at 57.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 172.52 at 117.0 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-prefer | 555.86 at 69.2 s: mostly unexplained (+timer tick_nohz_handler); 551.14 at 9.2 s: mostly unexplained (+timer tick_nohz_handler); 468.84 at 9.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 412.67 at 69.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 166.33 at 9.2 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-2 | 577.04 at 117.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 79.00 at 57.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 72.70 at 0.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 72.05 at 57.6 s: mostly unexplained (+timer tick_nohz_handler); 53.17 at 103.5 s: mostly unexplained (+timer tick_nohz_handler) |
| r1-noquickack | 357.17 at 41.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 215.71 at 41.8 s: mostly unexplained (+timer tick_nohz_handler); 191.77 at 101.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 165.60 at 101.8 s: mostly unexplained (+timer tick_nohz_handler); 103.99 at 101.8 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nogro | 943.88 at 74.2 s: mostly unexplained (+timer tick_nohz_handler); 804.94 at 74.2 s: mostly unexplained (+timer tick_nohz_handler); 651.98 at 14.4 s: mostly unexplained (+timer tick_nohz_handler); 147.01 at 74.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 134.64 at 74.2 s: mostly unexplained (+timer tick_nohz_handler) |
| r1-base-4 | 822.42 at 60.6 s: mostly unexplained (+timer tick_nohz_handler); 771.73 at 60.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 121.14 at 60.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 116.23 at 60.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 113.60 at 0.6 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nocoalesce | 737.90 at 58.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 115.19 at 58.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 90.93 at 118.7 s: mostly unexplained (+timer tick_nohz_handler); 70.51 at 58.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 65.80 at 58.7 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-irqlocal | 899.29 at 37.9 s: mostly unexplained (+timer tick_nohz_handler); 801.64 at 37.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 607.47 at 97.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 495.53 at 97.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 477.34 at 37.9 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-6 | 1082.74 at 101.4 s: mostly unexplained (+timer tick_nohz_handler); 932.24 at 41.5 s: mostly unexplained (+timer tick_nohz_handler); 827.39 at 41.5 s: mostly unexplained (+timer tick_nohz_handler); 244.28 at 101.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 166.27 at 41.5 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-rt | 1063.24 at 86.2 s: mostly unexplained (+timer tick_nohz_handler); 778.39 at 26.2 s: mostly unexplained (+timer tick_nohz_handler); 740.16 at 86.2 s: mostly unexplained (+timer tick_nohz_handler); 627.10 at 26.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 383.71 at 26.2 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-sibling | 881.71 at 61.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 603.68 at 1.4 s: mostly unexplained (+timer tick_nohz_handler); 536.79 at 61.4 s: mostly unexplained (+timer tick_nohz_handler); 469.21 at 1.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 456.97 at 61.4 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-8 | 714.74 at 81.4 s: mostly unexplained (+timer tick_nohz_handler); 588.26 at 81.4 s: mostly unexplained (+timer tick_nohz_handler); 585.07 at 0.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 583.10 at 21.4 s: mostly unexplained (+timer tick_nohz_handler); 444.25 at 21.4 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-busypoll50 | 1045.73 at 6.3 s: timer tick_nohz_handler; 698.94 at 6.3 s: mostly unexplained (+timer tick_nohz_handler); 501.95 at 66.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 465.28 at 66.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 233.39 at 66.3 s: mostly unexplained (+timer tick_nohz_handler) |
| r1-budget64 | 99.01 at 23.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 89.90 at 23.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 67.60 at 83.3 s: mostly unexplained (+timer tick_nohz_handler); 60.25 at 83.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 60.01 at 0.4 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 425.89 at 44.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 243.29 at 104.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 105.17 at 44.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 97.02 at 104.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 95.41 at 104.6 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-stock | 612.94 at 15.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 412.16 at 15.8 s: mostly unexplained (+timer tick_nohz_handler); 180.74 at 5.2 s: irq nvme0q0; 171.86 at 67.3 s: irq nvme0q0; 138.09 at 75.8 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-isolcpus | 58.94 at 58.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 58.67 at 74.7 s: mostly unexplained (+timer tick_nohz_handler); 58.52 at 99.9 s: mostly unexplained (+timer tick_nohz_handler); 48.61 at 118.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 46.84 at 99.9 s: mostly unexplained (+timer tick_nohz_handler) |
| boot-full-nomitig | 288.38 at 116.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 284.25 at 116.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 213.98 at 56.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 172.96 at 116.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 123.30 at 116.8 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full | 67.82 at 0.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 42.61 at 31.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 42.24 at 25.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 41.55 at 65.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 40.14 at 47.7 s: unexplained: no kernel event (hypervisor or in-kernel time) |

### pp-epoll-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 787.20 at 5.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 539.25 at 5.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 518.45 at 3.3 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 515.53 at 45.7 s: neither loop stalled long enough: network, hypervisor or untraced; 483.80 at 3.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 817.81 at 50.8 s: neither loop stalled long enough: network, hypervisor or untraced; 764.75 at 50.7 s: neither loop stalled long enough: network, hypervisor or untraced; 727.49 at 3.3 s: neither loop stalled long enough: network, hypervisor or untraced; 529.77 at 48.8 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 472.05 at 52.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 710.46 at 59.1 s: neither loop stalled long enough: network, hypervisor or untraced; 553.28 at 6.4 s: neither loop stalled long enough: network, hypervisor or untraced; 486.75 at 59.1 s: neither loop stalled long enough: network, hypervisor or untraced; 428.34 at 35.3 s: neither loop stalled long enough: network, hypervisor or untraced; 388.75 at 32.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 716.72 at 48.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 630.67 at 46.8 s: neither loop stalled long enough: network, hypervisor or untraced; 547.50 at 46.8 s: neither loop stalled long enough: network, hypervisor or untraced; 408.36 at 46.8 s: neither loop stalled long enough: network, hypervisor or untraced; 400.51 at 46.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1016.62 at 35.8 s: client: mostly unexplained (+timer tick_nohz_handler); 879.63 at 35.8 s: client: mostly unexplained (+timer tick_nohz_handler); 430.72 at 34.1 s: neither loop stalled long enough: network, hypervisor or untraced; 411.24 at 34.0 s: neither loop stalled long enough: network, hypervisor or untraced; 403.27 at 34.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 822.35 at 13.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 576.79 at 13.4 s: neither loop stalled long enough: network, hypervisor or untraced; 530.56 at 13.4 s: client: mostly unexplained (+timer tick_nohz_handler); 526.80 at 13.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 466.58 at 42.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 977.38 at 22.9 s: neither loop stalled long enough: network, hypervisor or untraced; 729.41 at 22.9 s: neither loop stalled long enough: network, hypervisor or untraced; 721.23 at 23.6 s: neither loop stalled long enough: network, hypervisor or untraced; 694.05 at 21.1 s: neither loop stalled long enough: network, hypervisor or untraced; 450.87 at 21.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-stock | 407.49 at 52.2 s: neither loop stalled long enough: network, hypervisor or untraced; 407.29 at 52.2 s: neither loop stalled long enough: network, hypervisor or untraced; 385.76 at 52.3 s: neither loop stalled long enough: network, hypervisor or untraced; 344.20 at 31.1 s: neither loop stalled long enough: network, hypervisor or untraced; 335.46 at 40.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1760.12 at 13.6 s: neither loop stalled long enough: network, hypervisor or untraced; 1117.36 at 52.1 s: client: mostly unexplained (+timer tick_nohz_handler); 871.17 at 52.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 772.32 at 50.6 s: neither loop stalled long enough: network, hypervisor or untraced; 428.81 at 50.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 581.48 at 50.5 s: neither loop stalled long enough: network, hypervisor or untraced; 506.70 at 52.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 481.09 at 50.5 s: neither loop stalled long enough: network, hypervisor or untraced; 435.65 at 50.5 s: neither loop stalled long enough: network, hypervisor or untraced; 434.42 at 20.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1424.90 at 39.9 s: client: mostly unexplained (+softirq TIMER); 506.11 at 38.5 s: neither loop stalled long enough: network, hypervisor or untraced; 493.17 at 39.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 432.84 at 38.5 s: neither loop stalled long enough: network, hypervisor or untraced; 407.18 at 26.5 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-epoll-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 1026.47 at 18.3 s: neither loop stalled long enough: network, hypervisor or untraced; 985.41 at 33.6 s: neither loop stalled long enough: network, hypervisor or untraced; 895.87 at 80.1 s: neither loop stalled long enough: network, hypervisor or untraced; 838.13 at 22.1 s: client: mostly unexplained (+timer tick_nohz_handler); 822.03 at 111.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1776.39 at 107.2 s: neither loop stalled long enough: network, hypervisor or untraced; 1517.73 at 65.3 s: client: mostly unexplained (+timer tick_nohz_handler); 1317.89 at 65.4 s: client: mostly unexplained (+timer tick_nohz_handler); 1229.44 at 5.4 s: client: mostly unexplained (+timer tick_nohz_handler); 998.96 at 46.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 635.83 at 15.1 s: neither loop stalled long enough: network, hypervisor or untraced; 480.16 at 77.3 s: client: mostly unexplained (+timer tick_nohz_handler); 450.82 at 119.3 s: neither loop stalled long enough: network, hypervisor or untraced; 441.55 at 15.1 s: neither loop stalled long enough: network, hypervisor or untraced; 432.75 at 28.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 2013.08 at 104.0 s: neither loop stalled long enough: network, hypervisor or untraced; 818.01 at 6.7 s: neither loop stalled long enough: network, hypervisor or untraced; 782.83 at 85.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 493.52 at 66.9 s: neither loop stalled long enough: network, hypervisor or untraced; 369.58 at 6.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1015.64 at 111.4 s: client: mostly unexplained (+timer tick_nohz_handler); 783.72 at 51.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 543.39 at 49.5 s: neither loop stalled long enough: network, hypervisor or untraced; 519.24 at 49.5 s: neither loop stalled long enough: network, hypervisor or untraced; 468.68 at 109.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 744.41 at 91.7 s: neither loop stalled long enough: network, hypervisor or untraced; 691.57 at 31.7 s: neither loop stalled long enough: network, hypervisor or untraced; 682.61 at 31.7 s: neither loop stalled long enough: network, hypervisor or untraced; 675.29 at 23.0 s: neither loop stalled long enough: network, hypervisor or untraced; 643.86 at 77.6 s: client: preempted by swapper/2 |
| r1-base-10 | 2863.19 at 66.1 s: neither loop stalled long enough: network, hypervisor or untraced; 1002.26 at 112.6 s: neither loop stalled long enough: network, hypervisor or untraced; 678.66 at 112.6 s: neither loop stalled long enough: network, hypervisor or untraced; 624.26 at 100.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 623.73 at 112.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 2139.72 at 96.2 s: neither loop stalled long enough: network, hypervisor or untraced; 1531.05 at 72.8 s: client: mostly unexplained (+timer tick_nohz_handler); 926.62 at 72.8 s: client: mostly unexplained (+timer tick_nohz_handler); 772.67 at 100.7 s: neither loop stalled long enough: network, hypervisor or untraced; 457.41 at 83.7 s: client: preempted by swapper/2 |
| boot-isolcpus | 1090.37 at 67.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 927.58 at 82.6 s: neither loop stalled long enough: network, hypervisor or untraced; 787.48 at 66.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 770.61 at 79.2 s: client: preempted by swapper/2; 766.74 at 7.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full-nomitig | 1057.46 at 78.6 s: neither loop stalled long enough: network, hypervisor or untraced; 967.94 at 6.9 s: client: mostly unexplained (+timer tick_nohz_handler); 727.00 at 6.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 726.28 at 67.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 704.70 at 65.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1665.26 at 113.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 1492.01 at 113.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 991.02 at 78.8 s: neither loop stalled long enough: network, hypervisor or untraced; 847.47 at 45.5 s: neither loop stalled long enough: network, hypervisor or untraced; 753.59 at 52.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-0

| condition | worst events |
|---|---|
| r1-base-0 | 666.61 at 39.5 s: neither loop stalled long enough: network, hypervisor or untraced; 628.75 at 41.6 s: client: mostly unexplained (+softirq NET_RX); 596.51 at 41.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 481.61 at 39.5 s: neither loop stalled long enough: network, hypervisor or untraced; 407.60 at 39.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1296.00 at 27.5 s: client: mostly unexplained (+timer tick_nohz_handler); 1154.80 at 27.5 s: client: mostly unexplained (+timer tick_nohz_handler); 632.58 at 25.9 s: neither loop stalled long enough: network, hypervisor or untraced; 486.92 at 25.9 s: neither loop stalled long enough: network, hypervisor or untraced; 418.18 at 25.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 792.37 at 38.4 s: neither loop stalled long enough: network, hypervisor or untraced; 567.61 at 36.9 s: neither loop stalled long enough: network, hypervisor or untraced; 397.61 at 36.9 s: neither loop stalled long enough: network, hypervisor or untraced; 388.11 at 36.9 s: neither loop stalled long enough: network, hypervisor or untraced; 366.14 at 4.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 554.50 at 28.0 s: neither loop stalled long enough: network, hypervisor or untraced; 412.11 at 27.9 s: neither loop stalled long enough: network, hypervisor or untraced; 375.36 at 27.9 s: neither loop stalled long enough: network, hypervisor or untraced; 311.96 at 27.9 s: neither loop stalled long enough: network, hypervisor or untraced; 290.37 at 28.0 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-noquickack | 2738.55 at 49.5 s: neither loop stalled long enough: network, hypervisor or untraced; 725.10 at 8.8 s: client: mostly unexplained (+timer tick_nohz_handler); 720.75 at 8.8 s: client: mostly unexplained (+timer tick_nohz_handler); 492.60 at 7.1 s: neither loop stalled long enough: network, hypervisor or untraced; 446.49 at 7.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 1006.44 at 37.4 s: neither loop stalled long enough: network, hypervisor or untraced; 666.76 at 39.3 s: client: mostly unexplained (+timer tick_nohz_handler); 480.21 at 39.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 438.25 at 37.4 s: neither loop stalled long enough: network, hypervisor or untraced; 364.72 at 37.4 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-4 | 792.24 at 33.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 521.34 at 31.9 s: neither loop stalled long enough: network, hypervisor or untraced; 378.46 at 31.9 s: neither loop stalled long enough: network, hypervisor or untraced; 354.34 at 31.9 s: neither loop stalled long enough: network, hypervisor or untraced; 329.18 at 31.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 861.61 at 25.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 487.75 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced; 402.50 at 23.4 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 385.56 at 23.4 s: neither loop stalled long enough: network, hypervisor or untraced; 363.94 at 23.4 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-irqlocal | 490.88 at 4.2 s: neither loop stalled long enough: network, hypervisor or untraced; 412.58 at 4.2 s: neither loop stalled long enough: network, hypervisor or untraced; 397.65 at 4.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 382.26 at 4.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 371.71 at 6.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-6 | 1992.82 at 45.7 s: neither loop stalled long enough: network, hypervisor or untraced; 757.38 at 10.3 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 725.33 at 11.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 608.62 at 11.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 516.32 at 10.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 589.14 at 46.1 s: neither loop stalled long enough: network, hypervisor or untraced; 483.11 at 46.1 s: neither loop stalled long enough: network, hypervisor or untraced; 389.58 at 46.1 s: neither loop stalled long enough: network, hypervisor or untraced; 314.73 at 46.1 s: neither loop stalled long enough: network, hypervisor or untraced; 303.82 at 38.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1869.68 at 3.4 s: neither loop stalled long enough: network, hypervisor or untraced; 663.55 at 30.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 557.98 at 30.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 367.16 at 30.1 s: neither loop stalled long enough: network, hypervisor or untraced; 281.67 at 34.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 777.94 at 50.7 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 586.03 at 52.5 s: client: mostly unexplained (+softirq NET_RX); 573.73 at 52.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 565.41 at 50.7 s: neither loop stalled long enough: network, hypervisor or untraced; 305.72 at 50.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-busypoll50 | 1081.74 at 36.0 s: client: mostly unexplained (+timer tick_nohz_handler); 1018.96 at 36.0 s: client: mostly unexplained (+timer tick_nohz_handler); 478.50 at 34.3 s: neither loop stalled long enough: network, hypervisor or untraced; 459.15 at 34.3 s: neither loop stalled long enough: network, hypervisor or untraced; 417.52 at 36.0 s: client: mostly unexplained (+timer tick_nohz_handler) |
| r1-budget64 | 983.21 at 54.2 s: neither loop stalled long enough: network, hypervisor or untraced; 531.77 at 54.2 s: neither loop stalled long enough: network, hypervisor or untraced; 363.08 at 7.0 s: neither loop stalled long enough: network, hypervisor or untraced; 339.14 at 54.2 s: neither loop stalled long enough: network, hypervisor or untraced; 335.23 at 29.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 593.66 at 8.0 s: client: mostly unexplained (+softirq NET_RX); 400.51 at 6.0 s: neither loop stalled long enough: network, hypervisor or untraced; 364.85 at 6.0 s: neither loop stalled long enough: network, hypervisor or untraced; 345.11 at 6.0 s: neither loop stalled long enough: network, hypervisor or untraced; 318.55 at 6.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 724.25 at 43.8 s: neither loop stalled long enough: network, hypervisor or untraced; 462.00 at 43.8 s: neither loop stalled long enough: network, hypervisor or untraced; 359.10 at 48.8 s: neither loop stalled long enough: network, hypervisor or untraced; 354.55 at 10.0 s: neither loop stalled long enough: network, hypervisor or untraced; 353.46 at 43.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 617.35 at 26.7 s: neither loop stalled long enough: network, hypervisor or untraced; 515.37 at 26.6 s: neither loop stalled long enough: network, hypervisor or untraced; 490.13 at 26.6 s: neither loop stalled long enough: network, hypervisor or untraced; 477.09 at 28.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 410.22 at 26.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 2116.75 at 51.7 s: neither loop stalled long enough: network, hypervisor or untraced; 816.09 at 31.3 s: client: mostly unexplained (+softirq NET_RX); 793.29 at 31.3 s: client: mostly unexplained (+timer tick_nohz_handler); 760.61 at 29.5 s: neither loop stalled long enough: network, hypervisor or untraced; 404.95 at 29.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 469.00 at 55.4 s: neither loop stalled long enough: network, hypervisor or untraced; 437.00 at 9.4 s: neither loop stalled long enough: network, hypervisor or untraced; 433.65 at 9.4 s: neither loop stalled long enough: network, hypervisor or untraced; 349.03 at 47.2 s: neither loop stalled long enough: network, hypervisor or untraced; 315.35 at 11.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 800.67 at 119.9 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 576.55 at 74.1 s: client: preempted by swapper/2; 463.29 at 89.0 s: neither loop stalled long enough: network, hypervisor or untraced; 462.54 at 98.2 s: neither loop stalled long enough: network, hypervisor or untraced; 462.15 at 95.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1571.00 at 106.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 1045.76 at 46.1 s: client: mostly unexplained (+timer tick_nohz_handler); 787.51 at 44.2 s: neither loop stalled long enough: network, hypervisor or untraced; 648.36 at 104.1 s: neither loop stalled long enough: network, hypervisor or untraced; 640.14 at 117.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 853.14 at 100.2 s: neither loop stalled long enough: network, hypervisor or untraced; 842.37 at 54.2 s: client: mostly unexplained (+timer tick_nohz_handler); 684.31 at 78.5 s: client: preempted by swapper/2; 598.67 at 52.1 s: neither loop stalled long enough: network, hypervisor or untraced; 506.94 at 112.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1707.45 at 86.3 s: neither loop stalled long enough: network, hypervisor or untraced; 672.27 at 76.5 s: client: preempted by swapper/2; 566.81 at 101.7 s: neither loop stalled long enough: network, hypervisor or untraced; 424.34 at 101.7 s: neither loop stalled long enough: network, hypervisor or untraced; 423.82 at 101.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 625.63 at 78.0 s: client: preempted by swapper/2; 530.54 at 25.0 s: neither loop stalled long enough: network, hypervisor or untraced; 492.99 at 25.0 s: neither loop stalled long enough: network, hypervisor or untraced; 457.11 at 84.8 s: neither loop stalled long enough: network, hypervisor or untraced; 436.71 at 25.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 2423.47 at 37.9 s: neither loop stalled long enough: network, hypervisor or untraced; 1201.87 at 58.2 s: client: mostly unexplained (+timer tick_nohz_handler); 875.17 at 58.2 s: client: mostly unexplained (+timer tick_nohz_handler); 757.61 at 13.2 s: neither loop stalled long enough: network, hypervisor or untraced; 597.24 at 56.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 675.08 at 79.9 s: client: preempted by swapper/2; 605.53 at 50.9 s: client: mostly unexplained (+timer tick_nohz_handler); 493.54 at 109.0 s: neither loop stalled long enough: network, hypervisor or untraced; 456.81 at 49.0 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 430.31 at 109.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 2734.75 at 23.3 s: neither loop stalled long enough: network, hypervisor or untraced; 872.27 at 106.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 787.11 at 104.1 s: neither loop stalled long enough: network, hypervisor or untraced; 768.80 at 44.3 s: neither loop stalled long enough: network, hypervisor or untraced; 627.75 at 46.5 s: client: mostly unexplained (+timer tick_nohz_handler) |
| r1-irqlocal | 1017.25 at 84.7 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 650.16 at 27.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 570.80 at 83.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 427.05 at 109.8 s: neither loop stalled long enough: network, hypervisor or untraced; 418.65 at 24.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 799.37 at 87.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 625.63 at 89.0 s: client: mostly unexplained (+softirq NET_RX); 585.63 at 78.8 s: client: preempted by swapper/2; 579.21 at 26.9 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 541.54 at 89.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-rt | 886.10 at 71.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 772.41 at 79.8 s: client: preempted by swapper/2; 771.46 at 69.4 s: neither loop stalled long enough: network, hypervisor or untraced; 732.62 at 11.2 s: client: mostly unexplained (+softirq NET_RX); 598.03 at 69.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 892.80 at 67.6 s: neither loop stalled long enough: network, hypervisor or untraced; 882.98 at 65.8 s: neither loop stalled long enough: network, hypervisor or untraced; 875.57 at 61.8 s: neither loop stalled long enough: network, hypervisor or untraced; 873.80 at 67.2 s: neither loop stalled long enough: network, hypervisor or untraced; 873.34 at 69.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1516.72 at 70.0 s: client: mostly unexplained (+timer tick_nohz_handler); 1026.47 at 69.9 s: client: mostly unexplained (+timer tick_nohz_handler); 588.44 at 91.1 s: client: preempted by swapper/2; 564.22 at 10.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 538.27 at 8.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-busypoll50 | 1375.38 at 53.8 s: client: mostly unexplained (+timer tick_nohz_handler); 1246.15 at 53.8 s: client: mostly unexplained (+timer tick_nohz_handler); 1188.79 at 113.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 883.05 at 52.1 s: neither loop stalled long enough: network, hypervisor or untraced; 622.18 at 83.4 s: client: preempted by swapper/2 |
| r1-budget64 | 651.24 at 9.1 s: neither loop stalled long enough: network, hypervisor or untraced; 581.56 at 73.7 s: client: preempted by swapper/2; 441.22 at 69.2 s: neither loop stalled long enough: network, hypervisor or untraced; 440.34 at 69.2 s: neither loop stalled long enough: network, hypervisor or untraced; 436.07 at 69.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1870.93 at 54.8 s: neither loop stalled long enough: network, hypervisor or untraced; 779.48 at 75.3 s: client: preempted by swapper/2; 750.35 at 86.4 s: neither loop stalled long enough: network, hypervisor or untraced; 434.49 at 67.4 s: neither loop stalled long enough: network, hypervisor or untraced; 404.03 at 26.4 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-stock | 2465.00 at 87.6 s: neither loop stalled long enough: network, hypervisor or untraced; 812.76 at 1.8 s: neither loop stalled long enough: network, hypervisor or untraced; 721.63 at 61.9 s: neither loop stalled long enough: network, hypervisor or untraced; 467.45 at 78.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 409.33 at 61.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 685.36 at 79.2 s: client: preempted by swapper/2; 487.40 at 41.2 s: neither loop stalled long enough: network, hypervisor or untraced; 448.87 at 101.0 s: neither loop stalled long enough: network, hypervisor or untraced; 415.12 at 41.2 s: neither loop stalled long enough: network, hypervisor or untraced; 413.75 at 53.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 873.56 at 45.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 810.36 at 104.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 793.80 at 104.1 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 786.76 at 45.9 s: client: mostly unexplained (+softirq NET_RX); 659.41 at 79.3 s: client: preempted by swapper/2 |
| boot-full | 1217.19 at 27.9 s: neither loop stalled long enough: network, hypervisor or untraced; 1114.50 at 27.9 s: neither loop stalled long enough: network, hypervisor or untraced; 699.12 at 77.1 s: client: preempted by swapper/2; 664.74 at 66.9 s: client: preempted by swapper/2; 534.14 at 87.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 1053.23 at 37.6 s: client: mostly unexplained (+timer tick_nohz_handler); 766.39 at 35.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 635.97 at 37.6 s: client: mostly unexplained (+timer tick_nohz_handler); 460.09 at 35.5 s: neither loop stalled long enough: network, hypervisor or untraced; 434.90 at 35.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 2020.14 at 4.6 s: neither loop stalled long enough: network, hypervisor or untraced; 810.16 at 20.0 s: client: mostly unexplained (+timer tick_nohz_handler); 770.84 at 20.0 s: neither loop stalled long enough: network, hypervisor or untraced; 597.41 at 34.6 s: neither loop stalled long enough: network, hypervisor or untraced; 566.37 at 18.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1158.51 at 21.3 s: client: mostly unexplained (+timer tick_nohz_handler); 928.15 at 21.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 716.25 at 21.3 s: client: mostly unexplained (+timer tick_nohz_handler); 442.57 at 19.3 s: neither loop stalled long enough: network, hypervisor or untraced; 418.42 at 42.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 1662.08 at 30.6 s: client: mostly unexplained (+timer tick_nohz_handler); 554.84 at 28.8 s: neither loop stalled long enough: network, hypervisor or untraced; 405.77 at 28.8 s: neither loop stalled long enough: network, hypervisor or untraced; 386.18 at 13.5 s: neither loop stalled long enough: network, hypervisor or untraced; 361.25 at 28.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 671.78 at 4.3 s: neither loop stalled long enough: network, hypervisor or untraced; 644.11 at 6.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 605.51 at 4.3 s: neither loop stalled long enough: network, hypervisor or untraced; 505.66 at 6.3 s: client: mostly unexplained (+timer tick_nohz_handler); 382.95 at 11.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 1876.16 at 8.7 s: neither loop stalled long enough: network, hypervisor or untraced; 596.74 at 54.3 s: neither loop stalled long enough: network, hypervisor or untraced; 470.49 at 28.6 s: neither loop stalled long enough: network, hypervisor or untraced; 450.68 at 28.6 s: neither loop stalled long enough: network, hypervisor or untraced; 448.28 at 53.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 1081.10 at 17.8 s: neither loop stalled long enough: network, hypervisor or untraced; 1023.75 at 5.3 s: neither loop stalled long enough: network, hypervisor or untraced; 780.84 at 59.6 s: neither loop stalled long enough: network, hypervisor or untraced; 698.26 at 29.3 s: neither loop stalled long enough: network, hypervisor or untraced; 670.40 at 29.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 611.59 at 28.3 s: neither loop stalled long enough: network, hypervisor or untraced; 503.92 at 28.4 s: neither loop stalled long enough: network, hypervisor or untraced; 467.47 at 28.3 s: neither loop stalled long enough: network, hypervisor or untraced; 352.27 at 30.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 314.29 at 28.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 672.90 at 6.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 543.78 at 6.3 s: neither loop stalled long enough: network, hypervisor or untraced; 521.83 at 35.6 s: neither loop stalled long enough: network, hypervisor or untraced; 513.83 at 49.6 s: neither loop stalled long enough: network, hypervisor or untraced; 468.98 at 4.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-rt | 526.02 at 4.3 s: neither loop stalled long enough: network, hypervisor or untraced; 490.22 at 4.2 s: neither loop stalled long enough: network, hypervisor or untraced; 420.86 at 4.3 s: neither loop stalled long enough: network, hypervisor or untraced; 410.51 at 3.4 s: neither loop stalled long enough: network, hypervisor or untraced; 407.02 at 40.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 973.88 at 25.4 s: client: mostly unexplained (+timer tick_nohz_handler); 646.26 at 23.6 s: neither loop stalled long enough: network, hypervisor or untraced; 540.86 at 25.4 s: neither loop stalled long enough: network, hypervisor or untraced; 506.40 at 34.7 s: neither loop stalled long enough: network, hypervisor or untraced; 461.45 at 25.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-8 | 536.43 at 46.6 s: neither loop stalled long enough: network, hypervisor or untraced; 418.18 at 35.6 s: neither loop stalled long enough: network, hypervisor or untraced; 379.37 at 3.3 s: neither loop stalled long enough: network, hypervisor or untraced; 366.77 at 46.6 s: neither loop stalled long enough: network, hypervisor or untraced; 365.38 at 3.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 737.08 at 58.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 587.73 at 57.0 s: neither loop stalled long enough: network, hypervisor or untraced; 500.98 at 59.3 s: neither loop stalled long enough: network, hypervisor or untraced; 460.30 at 5.7 s: neither loop stalled long enough: network, hypervisor or untraced; 454.01 at 57.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 585.35 at 30.4 s: neither loop stalled long enough: network, hypervisor or untraced; 480.24 at 30.4 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 398.86 at 42.3 s: neither loop stalled long enough: network, hypervisor or untraced; 385.22 at 27.8 s: neither loop stalled long enough: network, hypervisor or untraced; 379.37 at 53.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 723.82 at 20.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 660.03 at 20.8 s: client: mostly unexplained (+timer tick_nohz_handler); 455.55 at 20.8 s: neither loop stalled long enough: network, hypervisor or untraced; 443.20 at 19.2 s: neither loop stalled long enough: network, hypervisor or untraced; 437.04 at 40.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 841.77 at 27.9 s: client: mostly unexplained (+timer tick_nohz_handler); 572.25 at 26.1 s: neither loop stalled long enough: network, hypervisor or untraced; 504.62 at 26.1 s: neither loop stalled long enough: network, hypervisor or untraced; 456.15 at 26.1 s: neither loop stalled long enough: network, hypervisor or untraced; 444.35 at 29.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 717.51 at 8.8 s: client: mostly unexplained (+timer tick_nohz_handler); 523.83 at 8.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 493.52 at 7.2 s: neither loop stalled long enough: network, hypervisor or untraced; 450.57 at 32.3 s: neither loop stalled long enough: network, hypervisor or untraced; 425.52 at 19.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 2942.13 at 36.7 s: neither loop stalled long enough: network, hypervisor or untraced; 2325.87 at 39.7 s: neither loop stalled long enough: network, hypervisor or untraced; 1251.43 at 113.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 829.15 at 113.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 777.63 at 75.1 s: client: preempted by swapper/2 |
| r1-housekeeping | 946.29 at 97.3 s: neither loop stalled long enough: network, hypervisor or untraced; 890.43 at 37.2 s: neither loop stalled long enough: network, hypervisor or untraced; 458.62 at 35.4 s: neither loop stalled long enough: network, hypervisor or untraced; 454.93 at 95.2 s: neither loop stalled long enough: network, hypervisor or untraced; 444.55 at 95.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 745.10 at 59.7 s: neither loop stalled long enough: network, hypervisor or untraced; 732.46 at 100.1 s: neither loop stalled long enough: network, hypervisor or untraced; 688.38 at 83.1 s: client: preempted by swapper/2; 552.28 at 40.1 s: client: mostly unexplained (+timer tick_nohz_handler); 505.08 at 15.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-noquickack | 814.10 at 11.5 s: neither loop stalled long enough: network, hypervisor or untraced; 753.77 at 108.7 s: neither loop stalled long enough: network, hypervisor or untraced; 474.03 at 48.5 s: neither loop stalled long enough: network, hypervisor or untraced; 434.81 at 110.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 431.38 at 48.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nogro | 750.03 at 80.4 s: neither loop stalled long enough: network, hypervisor or untraced; 705.89 at 88.0 s: client: preempted by swapper/2; 615.95 at 82.3 s: client: mostly unexplained (+timer tick_nohz_handler); 612.41 at 31.1 s: neither loop stalled long enough: network, hypervisor or untraced; 578.78 at 20.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 868.63 at 103.6 s: neither loop stalled long enough: network, hypervisor or untraced; 723.94 at 95.2 s: client: preempted by swapper/2; 489.25 at 46.1 s: client: mostly unexplained (+timer tick_nohz_handler); 471.22 at 46.1 s: neither loop stalled long enough: network, hypervisor or untraced; 429.47 at 103.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-nocoalesce | 1087.59 at 43.1 s: neither loop stalled long enough: network, hypervisor or untraced; 1045.19 at 53.7 s: neither loop stalled long enough: network, hypervisor or untraced; 812.05 at 48.7 s: neither loop stalled long enough: network, hypervisor or untraced; 648.57 at 66.3 s: client: mostly unexplained (+timer tick_nohz_handler); 642.71 at 61.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-irqlocal | 678.35 at 22.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 552.26 at 49.8 s: neither loop stalled long enough: network, hypervisor or untraced; 546.20 at 49.8 s: neither loop stalled long enough: network, hypervisor or untraced; 465.79 at 69.3 s: neither loop stalled long enough: network, hypervisor or untraced; 414.58 at 109.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 711.74 at 22.3 s: neither loop stalled long enough: network, hypervisor or untraced; 642.50 at 82.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 632.36 at 82.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 614.69 at 20.7 s: neither loop stalled long enough: network, hypervisor or untraced; 598.68 at 82.2 s: client: preempted by swapper/2 |
| r1-rt | 1865.76 at 55.2 s: neither loop stalled long enough: network, hypervisor or untraced; 1657.15 at 58.2 s: neither loop stalled long enough: network, hypervisor or untraced; 846.17 at 86.1 s: client: mostly unexplained (+timer tick_nohz_handler); 727.25 at 86.1 s: client: mostly unexplained (+timer tick_nohz_handler); 545.66 at 26.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 1011.41 at 43.4 s: neither loop stalled long enough: network, hypervisor or untraced; 831.53 at 101.4 s: neither loop stalled long enough: network, hypervisor or untraced; 769.94 at 43.4 s: neither loop stalled long enough: network, hypervisor or untraced; 651.11 at 103.4 s: neither loop stalled long enough: network, hypervisor or untraced; 645.87 at 111.6 s: client: preempted by swapper/2 |
| r1-base-8 | 1894.53 at 31.3 s: neither loop stalled long enough: network, hypervisor or untraced; 1293.96 at 2.0 s: client: mostly unexplained (+timer tick_nohz_handler); 968.48 at 60.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 850.06 at 2.0 s: client: mostly unexplained (+timer tick_nohz_handler); 841.21 at 62.0 s: client: mostly unexplained (+timer tick_nohz_handler) |
| r1-base-10 | 867.94 at 17.3 s: client: mostly unexplained (+timer tick_nohz_handler); 713.78 at 59.6 s: neither loop stalled long enough: network, hypervisor or untraced; 700.48 at 94.5 s: client: preempted by swapper/2; 671.21 at 118.2 s: neither loop stalled long enough: network, hypervisor or untraced; 661.93 at 77.3 s: client: mostly unexplained (+timer tick_nohz_handler) |
| boot-stock | 564.97 at 105.3 s: neither loop stalled long enough: network, hypervisor or untraced; 555.12 at 72.5 s: client: preempted by swapper/2; 459.57 at 45.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 420.79 at 64.2 s: neither loop stalled long enough: network, hypervisor or untraced; 418.34 at 45.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1967.85 at 86.2 s: neither loop stalled long enough: network, hypervisor or untraced; 1033.00 at 37.8 s: neither loop stalled long enough: network, hypervisor or untraced; 791.94 at 36.3 s: neither loop stalled long enough: network, hypervisor or untraced; 742.16 at 28.6 s: neither loop stalled long enough: network, hypervisor or untraced; 642.41 at 74.4 s: client: preempted by swapper/2 |
| boot-full-nomitig | 2673.12 at 87.2 s: neither loop stalled long enough: network, hypervisor or untraced; 1005.94 at 100.0 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 885.46 at 41.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 791.30 at 75.1 s: client: preempted by swapper/2; 725.32 at 41.9 s: client: mostly unexplained (+timer tick_nohz_handler) |
| boot-full | 2149.10 at 82.0 s: neither loop stalled long enough: network, hypervisor or untraced; 649.64 at 90.5 s: client: preempted by swapper/2; 603.01 at 22.5 s: neither loop stalled long enough: network, hypervisor or untraced; 599.78 at 21.0 s: neither loop stalled long enough: network, hypervisor or untraced; 491.44 at 82.5 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-0

| condition | worst events |
|---|---|
| r1-base-0 | 565.56 at 6.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 409.30 at 6.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 394.55 at 4.7 s: neither loop stalled long enough: network, hypervisor or untraced; 378.41 at 4.8 s: neither loop stalled long enough: network, hypervisor or untraced; 372.23 at 4.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 573.64 at 52.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 462.47 at 50.6 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 460.18 at 41.8 s: neither loop stalled long enough: network, hypervisor or untraced; 413.60 at 50.6 s: neither loop stalled long enough: network, hypervisor or untraced; 377.04 at 50.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 405.95 at 4.2 s: neither loop stalled long enough: network, hypervisor or untraced; 394.06 at 4.2 s: neither loop stalled long enough: network, hypervisor or untraced; 357.71 at 4.2 s: neither loop stalled long enough: network, hypervisor or untraced; 356.00 at 32.2 s: neither loop stalled long enough: network, hypervisor or untraced; 352.36 at 4.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 530.64 at 53.7 s: neither loop stalled long enough: network, hypervisor or untraced; 473.04 at 53.7 s: neither loop stalled long enough: network, hypervisor or untraced; 446.16 at 53.7 s: neither loop stalled long enough: network, hypervisor or untraced; 396.77 at 53.7 s: neither loop stalled long enough: network, hypervisor or untraced; 324.44 at 53.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 606.70 at 1.6 s: neither loop stalled long enough: network, hypervisor or untraced; 332.77 at 1.6 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 283.62 at 1.6 s: neither loop stalled long enough: network, hypervisor or untraced; 280.13 at 38.0 s: neither loop stalled long enough: network, hypervisor or untraced; 278.53 at 1.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 495.86 at 38.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 473.74 at 37.4 s: neither loop stalled long enough: network, hypervisor or untraced; 469.26 at 37.4 s: neither loop stalled long enough: network, hypervisor or untraced; 412.95 at 50.6 s: neither loop stalled long enough: network, hypervisor or untraced; 410.62 at 37.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 451.81 at 57.2 s: neither loop stalled long enough: network, hypervisor or untraced; 446.91 at 57.1 s: neither loop stalled long enough: network, hypervisor or untraced; 406.72 at 57.1 s: neither loop stalled long enough: network, hypervisor or untraced; 374.78 at 57.2 s: neither loop stalled long enough: network, hypervisor or untraced; 358.85 at 5.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 830.44 at 15.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 561.55 at 15.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 469.14 at 53.5 s: neither loop stalled long enough: network, hypervisor or untraced; 461.99 at 15.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 448.75 at 13.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 948.26 at 34.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 929.71 at 34.8 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 494.97 at 33.0 s: neither loop stalled long enough: network, hypervisor or untraced; 476.65 at 33.0 s: neither loop stalled long enough: network, hypervisor or untraced; 445.23 at 2.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 585.21 at 6.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 544.06 at 5.0 s: neither loop stalled long enough: network, hypervisor or untraced; 532.73 at 5.0 s: neither loop stalled long enough: network, hypervisor or untraced; 483.23 at 5.0 s: neither loop stalled long enough: network, hypervisor or untraced; 459.87 at 6.3 s: client: mostly unexplained (+timer tick_nohz_handler) |
| boot-isolcpus | 505.98 at 53.5 s: neither loop stalled long enough: network, hypervisor or untraced; 436.86 at 54.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 422.99 at 53.5 s: neither loop stalled long enough: network, hypervisor or untraced; 417.88 at 53.5 s: neither loop stalled long enough: network, hypervisor or untraced; 376.30 at 53.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 478.04 at 55.6 s: neither loop stalled long enough: network, hypervisor or untraced; 470.06 at 53.8 s: neither loop stalled long enough: network, hypervisor or untraced; 415.81 at 53.8 s: neither loop stalled long enough: network, hypervisor or untraced; 406.18 at 55.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 383.72 at 53.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 601.35 at 38.3 s: neither loop stalled long enough: network, hypervisor or untraced; 438.12 at 38.3 s: neither loop stalled long enough: network, hypervisor or untraced; 405.36 at 38.3 s: neither loop stalled long enough: network, hypervisor or untraced; 380.38 at 38.3 s: neither loop stalled long enough: network, hypervisor or untraced; 367.71 at 9.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-10000

| condition | worst events |
|---|---|
| r1-base-0 | 626.13 at 82.3 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 597.63 at 89.8 s: neither loop stalled long enough: network, hypervisor or untraced; 506.09 at 20.3 s: neither loop stalled long enough: network, hypervisor or untraced; 504.35 at 80.3 s: neither loop stalled long enough: network, hypervisor or untraced; 460.61 at 80.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-housekeeping | 1006.37 at 8.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 947.30 at 68.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 846.38 at 68.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 814.87 at 8.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 638.70 at 84.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-prefer | 1898.78 at 6.4 s: neither loop stalled long enough: network, hypervisor or untraced; 860.91 at 21.1 s: neither loop stalled long enough: network, hypervisor or untraced; 725.09 at 82.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 643.37 at 82.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 534.51 at 21.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 700.86 at 12.6 s: neither loop stalled long enough: network, hypervisor or untraced; 604.51 at 12.6 s: neither loop stalled long enough: network, hypervisor or untraced; 561.46 at 72.6 s: neither loop stalled long enough: network, hypervisor or untraced; 464.05 at 74.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 405.17 at 72.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-4 | 454.43 at 77.3 s: neither loop stalled long enough: network, hypervisor or untraced; 427.63 at 17.4 s: neither loop stalled long enough: network, hypervisor or untraced; 423.09 at 19.5 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 416.90 at 77.3 s: neither loop stalled long enough: network, hypervisor or untraced; 397.29 at 77.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-6 | 1052.64 at 54.1 s: neither loop stalled long enough: network, hypervisor or untraced; 688.22 at 12.6 s: neither loop stalled long enough: network, hypervisor or untraced; 607.58 at 12.6 s: neither loop stalled long enough: network, hypervisor or untraced; 605.37 at 114.0 s: neither loop stalled long enough: network, hypervisor or untraced; 561.73 at 114.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-sibling | 681.63 at 108.3 s: neither loop stalled long enough: network, hypervisor or untraced; 602.22 at 13.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 543.39 at 73.3 s: neither loop stalled long enough: network, hypervisor or untraced; 461.35 at 95.3 s: neither loop stalled long enough: network, hypervisor or untraced; 405.12 at 73.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-8 | 1612.84 at 91.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 695.47 at 1.7 s: neither loop stalled long enough: network, hypervisor or untraced; 621.74 at 31.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 597.72 at 91.1 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 594.94 at 89.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1003.38 at 108.0 s: neither loop stalled long enough: network, hypervisor or untraced; 928.67 at 101.9 s: neither loop stalled long enough: network, hypervisor or untraced; 696.58 at 109.9 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 569.56 at 101.9 s: neither loop stalled long enough: network, hypervisor or untraced; 493.28 at 108.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1138.77 at 25.5 s: client: mostly unexplained (+timer tick_nohz_handler); 544.18 at 25.4 s: client: mostly unexplained (+timer tick_nohz_handler); 468.16 at 76.1 s: neither loop stalled long enough: network, hypervisor or untraced; 459.66 at 24.1 s: neither loop stalled long enough: network, hypervisor or untraced; 419.80 at 24.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 738.39 at 71.0 s: client: mostly unexplained (+timer tick_nohz_handler); 624.73 at 72.6 s: neither loop stalled long enough: network, hypervisor or untraced; 528.95 at 69.5 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 521.67 at 71.0 s: neither loop stalled long enough: network, hypervisor or untraced; 482.89 at 69.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 757.81 at 15.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 730.45 at 15.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 720.87 at 15.0 s: neither loop stalled long enough: network, hypervisor or untraced; 714.66 at 73.2 s: neither loop stalled long enough: network, hypervisor or untraced; 687.18 at 74.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1818.36 at 11.4 s: neither loop stalled long enough: network, hypervisor or untraced; 799.97 at 51.3 s: neither loop stalled long enough: network, hypervisor or untraced; 748.94 at 85.8 s: neither loop stalled long enough: network, hypervisor or untraced; 523.10 at 111.1 s: neither loop stalled long enough: network, hypervisor or untraced; 503.55 at 113.4 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| r1-base-0 | unexplained: no kernel event (hypervisor or in-kernel time) 48105.37; mostly unexplained (+timer tick_nohz_handler) 7182.11; loop work 5643.75; preempted by swapper/2 2650.55; mostly unexplained (+softirq NET_RX) 2277.31 |
| r1-housekeeping | loop work 82566.35; unexplained: no kernel event (hypervisor or in-kernel time) 68197.78; mostly unexplained (+timer tick_nohz_handler) 18679.29; mostly unexplained (+softirq NET_RX) 2852.95; preempted by swapper/2 1121.88 |
| r1-prefer | loop work 134985.51; unexplained: no kernel event (hypervisor or in-kernel time) 55354.36; mostly unexplained (+softirq NET_RX) 11431.08; mostly unexplained (+timer tick_nohz_handler) 9495.92; softirq NET_RX 2732.36 |
| r1-base-2 | unexplained: no kernel event (hypervisor or in-kernel time) 39410.77; mostly unexplained (+timer tick_nohz_handler) 15262.86; loop work 3745.78; mostly unexplained (+softirq NET_RX) 2610.87; preempted by swapper/2 1839.62 |
| r1-noquickack | unexplained: no kernel event (hypervisor or in-kernel time) 33042.42; mostly unexplained (+timer tick_nohz_handler) 13233.99; loop work 6415.96; mostly unexplained (+softirq NET_RX) 2263.57; softirq NET_RX 1494.18 |
| r1-nogro | loop work 84842.63; unexplained: no kernel event (hypervisor or in-kernel time) 63105.88; mostly unexplained (+timer tick_nohz_handler) 20899.62; mostly unexplained (+softirq NET_RX) 3191.26; mostly unexplained (+softirq RCU) 1080.81 |
| r1-base-4 | loop work 95090.08; unexplained: no kernel event (hypervisor or in-kernel time) 69226.83; mostly unexplained (+timer tick_nohz_handler) 13306.80; mostly unexplained (+softirq NET_RX) 2352.98; preempted by swapper/2 2063.16 |
| r1-nocoalesce | unexplained: no kernel event (hypervisor or in-kernel time) 32708.27; mostly unexplained (+timer tick_nohz_handler) 12430.38; loop work 4784.16; mostly unexplained (+softirq NET_RX) 3921.99; preempted by swapper/2 1101.65 |
| r1-irqlocal | loop work 1397908.25; softirq NET_RX 176594.79; unexplained: no kernel event (hypervisor or in-kernel time) 84491.71; softirq TASKLET 15837.21; mostly unexplained (+timer tick_nohz_handler) 5017.66 |
| r1-base-6 | loop work 76320.75; unexplained: no kernel event (hypervisor or in-kernel time) 70493.24; mostly unexplained (+timer tick_nohz_handler) 24425.94; mostly unexplained (+softirq NET_RX) 3776.83; preempted by swapper/2 1101.92 |
| r1-rt | loop work 80750.20; unexplained: no kernel event (hypervisor or in-kernel time) 56620.34; mostly unexplained (+timer tick_nohz_handler) 11142.51; mostly unexplained (+softirq NET_RX) 3644.64; mostly unexplained (+softirq TIMER) 946.32 |
| r1-sibling | mostly unexplained (+timer tick_nohz_handler) 306208.11; loop work 133744.51; unexplained: no kernel event (hypervisor or in-kernel time) 71828.82; softirq NET_RX 18714.49; mostly unexplained (+softirq NET_RX) 3598.25 |
| r1-base-8 | loop work 92380.77; unexplained: no kernel event (hypervisor or in-kernel time) 76700.26; mostly unexplained (+timer tick_nohz_handler) 17663.21; mostly unexplained (+softirq NET_RX) 2535.43; mostly unexplained (+softirq TIMER) 2289.51 |
| r1-busypoll50 | loop work 90025.26; unexplained: no kernel event (hypervisor or in-kernel time) 45000.06; mostly unexplained (+timer tick_nohz_handler) 20514.62; mostly unexplained (+softirq NET_RX) 2393.84; mostly unexplained (+softirq TIMER) 1111.11 |
| r1-budget64 | loop work 93765.73; unexplained: no kernel event (hypervisor or in-kernel time) 49294.45; mostly unexplained (+timer tick_nohz_handler) 7101.99; mostly unexplained (+softirq NET_RX) 1972.13; preempted by swapper/2 1186.97 |
| r1-base-10 | loop work 86189.82; unexplained: no kernel event (hypervisor or in-kernel time) 77454.88; mostly unexplained (+timer tick_nohz_handler) 11285.42; mostly unexplained (+softirq NET_RX) 4850.37; mostly unexplained (+softirq TIMER) 2268.95 |
| boot-stock | mostly unexplained (+timer tick_nohz_handler) 240650.97; timer tick_nohz_handler 79004.68; unexplained: no kernel event (hypervisor or in-kernel time) 71536.67; mostly unexplained (+softirq TIMER) 39951.77; softirq TIMER 36259.47 |
| boot-isolcpus | mostly unexplained (+timer tick_nohz_handler) 110497.54; loop work 97633.62; unexplained: no kernel event (hypervisor or in-kernel time) 68420.68; mostly unexplained (+softirq TIMER) 14494.56; mostly unexplained (+preempted by kworker/2:2) 12163.57 |
| boot-full-nomitig | unexplained: no kernel event (hypervisor or in-kernel time) 51333.86; mostly unexplained (+timer tick_nohz_handler) 4632.63; loop work 4454.90; mostly unexplained (+softirq NET_RX) 3933.93; preempted by swapper/2 1929.49 |
| boot-full | unexplained: no kernel event (hypervisor or in-kernel time) 40685.75; loop work 4911.00; mostly unexplained (+timer tick_nohz_handler) 2926.10; mostly unexplained (+softirq NET_RX) 2580.94; preempted by swapper/2 2516.27 |

## Once-a-minute stalls

Whether each source's events of 300 µs or more (loop stalls, or samples that slow) bunch at one second of the wall-clock minute. The busiest second is the middle of the busiest three; three seconds hold one event in twenty by chance, so a share of 25% or more over at least five events is marked periodic. Each host has its own second (its agents' timers start at boot).

| run | source | events | busiest second | events there (share) | minutes with one there | longest there (µs) | periodic |
|---|---|---|---|---|---|---|---|
| http-recv-busy-10000 | receiver loop | 55 | :51 | 49 (89%) | 29 of 40 | 1365.12 | **yes** |
| http-recv-busy-10000 | sender loop | 68 | :49 | 68 (100%) | 37 of 40 | 1022.64 | **yes** |
| http-recv-busy-10000 | slow samples | 344 | :50 | 208 (60%) | 40 of 40 | 1193.55 | **yes** |
| md-recv-busy | receiver loop | 27 | :51 | 26 (96%) | 14 of 21 | 1350.61 | **yes** |
| md-recv-busy | slow samples | 289 | :51 | 289 (100%) | 8 of 21 | 1329.36 | **yes** |
| noise | noise probe (loop CPU) | 48 | :51 | 47 (98%) | 24 of 41 | 1082.74 | **yes** |
| pp-epoll-spin-0 | receiver loop | 14 | :51 | 14 (100%) | 7 of 12 | 1370.12 | **yes** |
| pp-epoll-spin-0 | sender loop | 17 | :49 | 17 (100%) | 10 of 11 | 444.79 | **yes** |
| pp-epoll-spin-0 | slow samples | 240 | :50 | 83 (35%) | 12 of 12 | 1424.90 | **yes** |
| pp-epoll-spin-10000 | receiver loop | 39 | :50 | 32 (82%) | 18 of 22 | 1645.67 | **yes** |
| pp-epoll-spin-10000 | sender loop | 39 | :49 | 37 (95%) | 19 of 22 | 708.69 | **yes** |
| pp-epoll-spin-10000 | slow samples | 334 | :50 | 132 (40%) | 22 of 22 | 1665.26 | **yes** |
| pp-recv-busy-0 | receiver loop | 22 | :51 | 22 (100%) | 13 of 20 | 1264.70 | **yes** |
| pp-recv-busy-0 | sender loop | 30 | :49 | 30 (100%) | 17 of 20 | 702.21 | **yes** |
| pp-recv-busy-0 | slow samples | 134 | :50 | 101 (75%) | 20 of 20 | 1296.00 | **yes** |
| pp-recv-busy-10000 | receiver loop | 58 | :51 | 38 (66%) | 24 of 40 | 1546.12 | **yes** |
| pp-recv-busy-10000 | sender loop | 72 | :49 | 70 (97%) | 36 of 41 | 912.36 | **yes** |
| pp-recv-busy-10000 | slow samples | 321 | :50 | 180 (56%) | 40 of 41 | 1571.00 | **yes** |
| pp-uring-spin-0 | receiver loop | 21 | :51 | 21 (100%) | 14 of 17 | 1595.41 | **yes** |
| pp-uring-spin-0 | sender loop | 26 | :49 | 26 (100%) | 15 of 17 | 658.57 | **yes** |
| pp-uring-spin-0 | slow samples | 318 | :50 | 124 (39%) | 17 of 17 | 1662.08 | **yes** |
| pp-uring-spin-10000 | receiver loop | 51 | :51 | 39 (76%) | 22 of 34 | 1221.89 | **yes** |
| pp-uring-spin-10000 | sender loop | 70 | :49 | 70 (100%) | 33 of 35 | 895.36 | **yes** |
| pp-uring-spin-10000 | slow samples | 525 | :50 | 204 (39%) | 34 of 35 | 2149.10 | **yes** |
| pp-uring-sqpoll-0 | receiver loop | 13 | :51 | 13 (100%) | 8 of 13 | 947.02 | **yes** |
| pp-uring-sqpoll-0 | sender loop | 30 | :49 | 30 (100%) | 13 of 14 | 559.99 | **yes** |
| pp-uring-sqpoll-0 | slow samples | 119 | :50 | 69 (58%) | 13 of 13 | 948.26 | **yes** |
| pp-uring-sqpoll-10000 | receiver loop | 32 | :51 | 32 (100%) | 19 of 26 | 1583.14 | **yes** |
| pp-uring-sqpoll-10000 | sender loop | 40 | :49 | 40 (100%) | 23 of 26 | 595.51 | **yes** |
| pp-uring-sqpoll-10000 | slow samples | 231 | :50 | 129 (56%) | 26 of 26 | 1612.84 | **yes** |

# Every run in detail

#### Stalls: r1-base-0/http-recv-busy-10000, loop 0 on CPU 2

1826154985 iterations, longest 772.4 µs; over 5 / 20 / 100 µs: 106330 / 178 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 138 | 6490.3 | 772.4 |
| preempted by swapper/2 | 1 | 698.3 | 698.3 |
| mostly unexplained (+softirq NET_RX) | 19 | 668.0 | 85.3 |
| loop work | 9 | 271.7 | 38.9 |
| mostly unexplained (+reschedule (IPI/vector)) | 6 | 128.0 | 23.2 |
| mostly unexplained (+ipi_send_cpu) | 3 | 77.8 | 30.3 |
| mostly unexplained (+timer tick_nohz_handler) | 1 | 67.5 | 67.5 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 32.7 | 32.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 772.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 733.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 698.3 | 2 | preempted by swapper/2 | preempted by swapper/2 678, call function single (IPI/vector) 2, page fault 0 |
| 4 | 150.2 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 101.5 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 85.3 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 79.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 77.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 71.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 71.2 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 39442 |
| CAL Function call interrupts | 1017 |
| RES Rescheduling interrupts | 80 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-base-0/http-recv-busy-10000 (median 55.20)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 843.84 | 788.63 | 771.47 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 751.58 | 696.38 | 732.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.54 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 746.27 | 691.07 | 697.28 | preempted by swapper/2 | 35.88 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 4 | 642.58 | 587.38 | 0.00 |  | 40.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 546.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 467.18 | 411.98 | 0.00 |  | 20.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 391.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 463.11 | 407.90 | 0.00 |  | 0.00 |  | 407.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 444.93 | 389.72 | 0.00 |  | 0.00 |  | 389.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 442.55 | 387.35 | 0.00 |  | 20.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 366.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 416.34 | 361.13 | 0.00 |  | 0.00 |  | 361.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 389.00 | 333.79 | 0.00 |  | 0.00 |  | 333.79 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-0/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=664586 clocksource=tsc

### Loop counters (r1-base-0/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001600 | 950967 | 3.16 | 63097984 | 21.02 | 1051633 | 1051633 | 98.5 | 83.6 | true | 0 | 0 | 1016.9 | 369909 / 269 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8215 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672358400 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (226035 of 2900050 in 1-msg reads) | 226035 | 3.82 | 3.36 | 5.78 | 9.06 | 14.32 | 22.71 | 45.80 |

#### Stalls: r1-base-0/md-recv-busy, loop 0 on CPU 2

63097984 iterations, longest 1016.9 µs; over 5 / 20 / 100 µs: 369909 / 269 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 175 | 5372.1 | 594.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 62 | 2184.8 | 175.4 |
| mostly unexplained (+timer tick_nohz_handler) | 5 | 1130.4 | 1016.9 |
| softirq NET_RX | 25 | 556.0 | 26.3 |
| mostly unexplained (+ipi_send_cpu) | 1 | 30.4 | 30.4 |
| mostly unexplained (+softirq NET_RX) | 1 | 22.0 | 22.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1016.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 2 | 594.8 | 50 | loop work (50 items) | softirq NET_RX 41, timer tick_nohz_handler 2 |
| 3 | 399.8 | 1550 | loop work (1550 items) |  |
| 4 | 175.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 77.6 | 15 | loop work (15 items) | softirq NET_RX 3 |
| 6 | 69.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 66.1 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 62.0 | 19 | loop work (19 items) | softirq NET_RX 1 |
| 9 | 60.2 | 5 | loop work (5 items) | softirq NET_RX 1 |
| 10 | 58.7 | 15 | loop work (15 items) | softirq NET_RX 9 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60033 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

## r1-base-0/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-0/noise, loop 0 on CPU 2

12492152832 iterations, longest 61.3 µs; over 5 / 20 / 100 µs: 512 / 84 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 397 | 5487.3 | 61.3 |
| mostly unexplained (+reschedule (IPI/vector)) | 115 | 737.3 | 9.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 61.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 46.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 45.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 43.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 43.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 40.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 38.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 37.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 37.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 117 |
| IWI IRQ work interrupts | 1 |

### Loop counters (r1-base-0/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1167833 | 1167833 | 1.00 | 309404147 | 264.94 | 5156736 | 5117808 | 0.0 | 37.4 | false | 0 | 0 | 779.2 | 17386 / 109 / 3 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-0, loop 0 on CPU 2

307068480 iterations, longest 779.2 µs; over 5 / 20 / 100 µs: 17386 / 109 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 89 | 3918.0 | 779.2 |
| mostly unexplained (+timer tick_nohz_handler) | 15 | 487.6 | 98.1 |
| mostly unexplained (+softirq TIMER) | 3 | 188.9 | 145.3 |
| mostly unexplained (+softirq RCU) | 2 | 49.9 | 26.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 779.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 488.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 145.3 | 1 | mostly unexplained (+softirq TIMER) | softirq TIMER 1 |
| 4 | 98.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq TIMER 1 |
| 5 | 96.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 80.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 67.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 62.2 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 60.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 54.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60141 |
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-0 (median 47.30)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 787.20 | 739.89 | 778.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 42.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 539.25 | 491.95 | 532.61 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 518.45 | 471.15 | 0.00 |  | 327.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 144.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 515.53 | 468.23 | 0.00 |  | 20.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 447.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 483.80 | 436.50 | 0.00 |  | 41.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 395.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 479.58 | 432.27 | 0.00 |  | 43.30 | unexplained: no kernel event (hypervisor or in-kernel time) | 388.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 464.36 | 417.05 | 0.00 |  | 66.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 350.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 455.28 | 407.98 | 0.00 |  | 21.64 | unexplained: no kernel event (hypervisor or in-kernel time) | 386.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 412.16 | 364.85 | 0.00 |  | 0.00 |  | 364.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 407.73 | 360.43 | 0.00 |  | 0.00 |  | 360.43 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198582 | 1198582 | 1.00 | 619862636 | 517.16 | 5165522 | 5145546 | 0.0 | 32.5 | false | 1 | 0 | 749.6 | 14667 / 174 / 11 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-10000, loop 0 on CPU 2

617465472 iterations, longest 749.6 µs; over 5 / 20 / 100 µs: 14667 / 174 / 11

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 127 | 5540.0 | 648.3 |
| mostly unexplained (+timer tick_nohz_handler) | 35 | 2357.7 | 749.6 |
| preempted by swapper/2 | 1 | 706.0 | 706.0 |
| mostly unexplained (+softirq TIMER) | 8 | 236.6 | 47.7 |
| mostly unexplained (+reschedule (IPI/vector)) | 3 | 62.3 | 21.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 749.6 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 2 | 706.0 | 1 | preempted by swapper/2 | preempted by swapper/2 687, call function single (IPI/vector) 3, page fault 0 |
| 3 | 648.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 615.8 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 5 | 492.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 171.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 158.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 146.3 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 9 | 110.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 104.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 89401 |
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 30 |
| IWI IRQ work interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-10000 (median 47.69)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1026.47 | 978.78 | 0.00 |  | 0.00 |  | 978.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 985.41 | 937.73 | 0.00 |  | 168.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 769.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 895.87 | 848.19 | 0.00 |  | 318.64 | unexplained: no kernel event (hypervisor or in-kernel time) | 529.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 838.13 | 790.44 | 803.81 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 5 | 822.03 | 774.34 | 0.00 |  | 20.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 754.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 751.97 | 704.28 | 705.75 | preempted by swapper/2 | 40.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 7 | 724.68 | 676.99 | 0.00 |  | 581.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 95.49 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 713.43 | 665.74 | 674.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 646.47 | 598.79 | 615.39 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 10 | 567.46 | 519.78 | 523.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-0/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1250331 | 1250331 | 1.00 | 60284956 | 48.22 | 1004749 | 983910 | 97.9 | 85.6 | true | 0 | 0 | 593.5 | 157099 / 99 / 3 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-0, loop 0 on CPU 2

59034624 iterations, longest 593.5 µs; over 5 / 20 / 100 µs: 157099 / 99 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 66 | 2963.2 | 551.0 |
| mostly unexplained (+softirq NET_RX) | 13 | 1047.6 | 593.5 |
| mostly unexplained (+timer tick_nohz_handler) | 8 | 224.1 | 57.3 |
| mostly unexplained (+softirq TIMER) | 5 | 115.0 | 26.1 |
| mostly unexplained (+softirq RCU) | 2 | 65.8 | 36.8 |
| mostly unexplained (+ipi_send_cpu) | 2 | 50.6 | 27.4 |
| softirq NET_RX | 2 | 43.5 | 22.6 |
| timer tick_nohz_handler | 1 | 21.0 | 21.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 593.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 551.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 107.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 4 | 98.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 92.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 82.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 77.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 72.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 70.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 63.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60153 |
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-0/pp-recv-busy-0 (median 45.35)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 666.61 | 621.25 | 0.00 |  | 60.66 | unexplained: no kernel event (hypervisor or in-kernel time) | 560.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 628.75 | 583.39 | 593.01 | mostly unexplained (+softirq NET_RX) | 41.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 3 | 596.51 | 551.16 | 551.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 40.47 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 481.61 | 436.26 | 0.00 |  | 0.00 |  | 436.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 407.60 | 362.25 | 0.00 |  | 20.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 342.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 387.81 | 342.45 | 0.00 |  | 0.00 |  | 342.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 315.92 | 270.56 | 0.00 |  | 32.69 | unexplained: no kernel event (hypervisor or in-kernel time) | 237.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 303.88 | 258.53 | 0.00 |  | 40.43 | unexplained: no kernel event (hypervisor or in-kernel time) | 218.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 303.07 | 257.72 | 0.00 |  | 40.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 217.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 235.99 | 190.64 | 0.00 |  | 0.00 |  | 190.64 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194550 | 1194550 | 1.00 | 128732599 | 107.77 | 1072772 | 1062817 | 99.1 | 84.0 | true | 1 | 0 | 518.0 | 184209 / 187 / 2 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-10000, loop 0 on CPU 2

127538048 iterations, longest 518.0 µs; over 5 / 20 / 100 µs: 184209 / 187 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 151 | 5286.6 | 105.1 |
| mostly unexplained (+softirq NET_RX) | 18 | 539.7 | 56.6 |
| preempted by swapper/2 | 1 | 518.0 | 518.0 |
| mostly unexplained (+timer tick_nohz_handler) | 8 | 281.9 | 64.0 |
| mostly unexplained (+reschedule (IPI/vector)) | 5 | 105.3 | 22.5 |
| mostly unexplained (+ipi_send_cpu) | 2 | 56.6 | 32.2 |
| softirq TIMER | 1 | 28.0 | 28.0 |
| timer tick_nohz_handler | 1 | 26.5 | 26.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 518.0 | 1 | preempted by swapper/2 | preempted by swapper/2 501, call function single (IPI/vector) 3, page fault 0 |
| 2 | 105.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 92.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 91.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 67.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 66.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 66.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 64.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 9 | 63.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 63.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 74114 |
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 46 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/pp-recv-busy-10000 (median 47.14)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 800.67 | 753.53 | 0.00 |  | 419.63 | unexplained: no kernel event (hypervisor or in-kernel time) | 333.91 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 576.55 | 529.41 | 517.46 | preempted by swapper/2 | 20.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 3 | 463.29 | 416.15 | 0.00 |  | 0.00 |  | 416.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 462.54 | 415.40 | 0.00 |  | 0.00 |  | 415.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 462.15 | 415.01 | 0.00 |  | 0.00 |  | 415.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 461.12 | 413.98 | 0.00 |  | 0.00 |  | 413.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 455.23 | 408.09 | 0.00 |  | 0.00 |  | 408.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 453.41 | 406.27 | 0.00 |  | 0.00 |  | 406.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 452.61 | 405.47 | 0.00 |  | 0.00 |  | 405.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 451.55 | 404.41 | 0.00 |  | 0.00 |  | 404.41 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1209193 | 1209193 | 1.00 | 3627580 | 3.00 | 60460 | 50435038 | 0.0 | 11.6 | false | 0 | 0 | 1028.8 | 13778 / 60 / 5 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-spin-0, loop 0 on CPU 2

3026102272 iterations, longest 1028.8 µs; over 5 / 20 / 100 µs: 13778 / 60 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 50 | 2300.9 | 293.4 |
| mostly unexplained (+timer tick_nohz_handler) | 10 | 1859.4 | 1028.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1028.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3, softirq TIMER 1 |
| 2 | 581.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 293.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 208.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 114.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 93.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 87.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 72.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 61.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 10 | 58.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60144 |
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-uring-spin-0 (median 45.16)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1053.23 | 1008.07 | 1028.82 | mostly unexplained (+timer tick_nohz_handler) | 61.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 766.39 | 721.23 | 0.00 |  | 572.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 148.56 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 635.97 | 590.80 | 605.26 | mostly unexplained (+timer tick_nohz_handler) | 40.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 460.09 | 414.92 | 0.00 |  | 62.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 352.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 434.90 | 389.74 | 0.00 |  | 20.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 369.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 416.86 | 371.70 | 0.00 |  | 46.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 325.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 348.43 | 303.26 | 0.00 |  | 20.15 | unexplained: no kernel event (hypervisor or in-kernel time) | 283.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 338.89 | 293.73 | 293.45 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.28 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 310.73 | 265.57 | 0.00 |  | 79.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 185.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 305.12 | 259.95 | 0.00 |  | 0.00 |  | 259.95 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199513 | 1199513 | 1.00 | 3598540 | 3.00 | 29988 | 39181673 | 0.0 | 5.8 | false | 1 | 0 | 1221.9 | 9983 / 126 / 6 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-spin-10000, loop 0 on CPU 2

4701800768 iterations, longest 1221.9 µs; over 5 / 20 / 100 µs: 9983 / 126 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 114 | 6281.5 | 1221.9 |
| mostly unexplained (+timer tick_nohz_handler) | 8 | 773.5 | 541.3 |
| preempted by swapper/2 | 1 | 728.2 | 728.2 |
| mostly unexplained (+softirq TIMER) | 2 | 49.7 | 25.8 |
| mostly unexplained (+softirq RCU) | 1 | 22.5 | 22.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1221.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 826.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 728.2 | 0 | preempted by swapper/2 | preempted by swapper/2 710, call function single (IPI/vector) 3, page fault 0 |
| 4 | 541.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3, softirq TIMER 1 |
| 5 | 314.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 274.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 97.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 93.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 84.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 78.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 75164 |
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 44 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/pp-uring-spin-10000 (median 48.10)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 2942.13 | 2894.03 | 0.00 |  | 41.57 | unexplained: no kernel event (hypervisor or in-kernel time) | 2852.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 2325.87 | 2277.76 | 0.00 |  | 63.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 2214.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 1251.43 | 1203.33 | 1221.89 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 829.15 | 781.05 | 826.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 777.63 | 729.53 | 728.20 | preempted by swapper/2 | 40.69 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 6 | 472.71 | 424.61 | 0.00 |  | 57.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 367.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 439.32 | 391.22 | 0.00 |  | 0.00 |  | 391.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 439.07 | 390.97 | 0.00 |  | 20.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 370.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 413.25 | 365.14 | 0.00 |  | 0.00 |  | 365.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 385.52 | 337.42 | 0.00 |  | 0.00 |  | 337.42 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1335034 | 1335034 | 1.00 | 1 | 0.00 | 0 | 49164410 | 0.0 | 0.0 | true | 0 | 0 | 560.7 | 235 / 60 / 3 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-0, loop 0 on CPU 2

2949864576 iterations, longest 560.7 µs; over 5 / 20 / 100 µs: 235 / 60 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 60 | 2851.2 | 560.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 560.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 385.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 112.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 95.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 66.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 46.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 42.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 42.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 41.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 41.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 59 |
| IWI IRQ work interrupts | 4 |
| HVS Hyper-V stimer0 interrupts | 3 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-0 (median 42.33)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 565.56 | 523.23 | 560.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 40.36 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 409.30 | 366.97 | 385.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 394.55 | 352.22 | 0.00 |  | 20.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 331.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 378.41 | 336.08 | 0.00 |  | 20.03 | unexplained: no kernel event (hypervisor or in-kernel time) | 316.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 372.23 | 329.90 | 0.00 |  | 0.00 |  | 329.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 293.35 | 251.02 | 0.00 |  | 0.00 |  | 251.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 286.72 | 244.39 | 0.00 |  | 0.00 |  | 244.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 274.67 | 232.34 | 0.00 |  | 61.36 | unexplained: no kernel event (hypervisor or in-kernel time) | 170.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 243.98 | 201.65 | 0.00 |  | 41.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 160.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 231.22 | 188.89 | 0.00 |  | 0.00 |  | 188.89 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199603 | 1199603 | 1.00 | 1 | 0.00 | 0 | 36633874 | 0.0 | 0.0 | true | 0 | 0 | 647.7 | 495 / 106 / 3 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-10000, loop 0 on CPU 2

4396064896 iterations, longest 647.7 µs; over 5 / 20 / 100 µs: 495 / 106 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 106 | 4801.7 | 647.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 647.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 605.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 189.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 88.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 84.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 82.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 76.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 74.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 59.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 55.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1020 |
| RES Rescheduling interrupts | 117 |
| HVS Hyper-V stimer0 interrupts | 3 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-10000 (median 48.19)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 626.13 | 577.93 | 605.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 597.63 | 549.44 | 0.00 |  | 64.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 484.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 506.09 | 457.90 | 0.00 |  | 22.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 435.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 504.35 | 456.16 | 0.00 |  | 87.96 | unexplained: no kernel event (hypervisor or in-kernel time) | 368.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 460.61 | 412.42 | 0.00 |  | 63.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 348.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 451.00 | 402.81 | 0.00 |  | 21.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 380.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 446.65 | 398.45 | 0.00 |  | 42.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 355.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 435.22 | 387.03 | 0.00 |  | 20.85 | unexplained: no kernel event (hypervisor or in-kernel time) | 366.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 410.96 | 362.77 | 0.00 |  | 47.62 | unexplained: no kernel event (hypervisor or in-kernel time) | 315.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 386.33 | 338.14 | 0.00 |  | 43.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 294.96 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-housekeeping/http-recv-busy-10000, loop 0 on CPU 2

1928298728 iterations, longest 566.1 µs; over 5 / 20 / 100 µs: 145921 / 162 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 136 | 5565.4 | 524.8 |
| mostly unexplained (+softirq NET_RX) | 16 | 613.3 | 78.0 |
| preempted by swapper/2 | 1 | 566.1 | 566.1 |
| mostly unexplained (+ipi_send_cpu) | 4 | 92.0 | 27.8 |
| softirq NET_RX | 1 | 64.7 | 64.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 46.1 | 46.1 |
| mostly unexplained (+reschedule (IPI/vector)) | 2 | 43.4 | 22.6 |
| loop work | 1 | 20.5 | 20.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 566.1 | 2 | preempted by swapper/2 | preempted by swapper/2 544, call function single (IPI/vector) 3, page fault 0 |
| 2 | 524.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 480.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 164.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 98.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 78.0 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1, softirq RCU 1, timer tick_nohz_handler 1 |
| 7 | 77.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 74.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 72.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 66.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 44826 |
| CAL Function call interrupts | 1017 |
| RES Rescheduling interrupts | 73 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-housekeeping/http-recv-busy-10000 (median 48.30)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 651.83 | 603.53 | 0.00 |  | 20.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 582.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 623.84 | 575.54 | 565.23 | preempted by swapper/2 | 41.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 3 | 585.61 | 537.32 | 524.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 12.56 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 493.07 | 444.77 | 0.00 |  | 0.00 |  | 444.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 438.49 | 390.19 | 0.00 |  | 0.00 |  | 390.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 437.63 | 389.33 | 0.00 |  | 0.00 |  | 389.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 403.65 | 355.36 | 0.00 |  | 0.00 |  | 355.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 402.94 | 354.64 | 0.00 |  | 0.00 |  | 354.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 363.89 | 315.59 | 0.00 |  | 0.00 |  | 315.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 354.83 | 306.53 | 0.00 |  | 0.00 |  | 306.53 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-housekeeping/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=649119 clocksource=tsc

### Loop counters (r1-housekeeping/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001600 | 919500 | 3.26 | 62975168 | 20.98 | 1049586 | 1049586 | 98.5 | 83.5 | true | 0 | 0 | 761.9 | 360825 / 5008 / 7 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8213 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672358400 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (211340 of 2900050 in 1-msg reads) | 211340 | 4.65 | 3.30 | 7.16 | 22.82 | 29.14 | 42.91 | 54.52 |

#### Stalls: r1-housekeeping/md-recv-busy, loop 0 on CPU 2

62975168 iterations, longest 761.9 µs; over 5 / 20 / 100 µs: 360825 / 5008 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 3751 | 82545.9 | 438.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1079 | 24106.9 | 133.7 |
| mostly unexplained (+timer tick_nohz_handler) | 158 | 4549.2 | 113.7 |
| mostly unexplained (+softirq NET_RX) | 1 | 761.9 | 761.9 |
| softirq NET_RX | 14 | 317.7 | 38.2 |
| mostly unexplained (+ipi_send_cpu) | 3 | 66.0 | 22.8 |
| mostly unexplained (+softirq TIMER) | 1 | 28.5 | 28.5 |
| mostly unexplained (+softirq RCU) | 1 | 26.0 | 26.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 761.9 | 33 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13 |
| 2 | 438.6 | 1550 | loop work (1550 items) | timer tick_nohz_handler 2 |
| 3 | 200.8 | 27 | loop work (27 items) | timer tick_nohz_handler 3, softirq NET_RX 1 |
| 4 | 133.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 132.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 113.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 7 | 100.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 8 | 98.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 86.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 77.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60034 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## r1-housekeeping/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-housekeeping/noise, loop 0 on CPU 2

12393897984 iterations, longest 596.1 µs; over 5 / 20 / 100 µs: 2163 / 113 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 848 | 11845.7 | 558.4 |
| mostly unexplained (+timer tick_nohz_handler) | 1206 | 8065.2 | 596.1 |
| mostly unexplained (+softirq TIMER) | 106 | 611.1 | 7.9 |
| timer tick_nohz_handler | 2 | 12.5 | 7.1 |
| softirq TIMER | 1 | 9.2 | 9.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 596.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 2 | 558.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 482.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 4 | 265.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 172.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 143.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 78.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 66.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 62.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 10 | 55.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120024 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-housekeeping/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1273631 | 1273631 | 1.00 | 60135584 | 47.22 | 1002260 | 981033 | 97.8 | 85.6 | true | 0 | 0 | 1264.7 | 145182 / 123 / 4 | 0 | 0 |

#### Stalls: r1-housekeeping/pp-recv-busy-0, loop 0 on CPU 2

58861952 iterations, longest 1264.7 µs; over 5 / 20 / 100 µs: 145182 / 123 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 28 | 2976.4 | 1264.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 70 | 2534.8 | 124.9 |
| mostly unexplained (+softirq NET_RX) | 17 | 568.9 | 65.4 |
| mostly unexplained (+softirq TIMER) | 4 | 90.2 | 23.2 |
| mostly unexplained (+ipi_send_cpu) | 3 | 70.9 | 28.0 |
| softirq NET_RX | 1 | 22.9 | 22.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1264.7 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3, softirq NET_RX 1 |
| 2 | 1108.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 3 | 124.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 120.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 90.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 66.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 65.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 62.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 60.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 59.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60147 |
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-housekeeping/pp-recv-busy-0 (median 44.62)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1296.00 | 1251.38 | 1264.32 | mostly unexplained (+timer tick_nohz_handler) | 81.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 1154.80 | 1110.17 | 1108.82 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 1.36 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 632.58 | 587.96 | 0.00 |  | 0.00 |  | 587.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 486.92 | 442.29 | 0.00 |  | 0.00 |  | 442.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 418.18 | 373.56 | 0.00 |  | 40.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 333.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 329.79 | 285.17 | 0.00 |  | 41.15 | unexplained: no kernel event (hypervisor or in-kernel time) | 244.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 294.68 | 250.06 | 0.00 |  | 0.00 |  | 250.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 212.07 | 167.45 | 0.00 |  | 20.98 | unexplained: no kernel event (hypervisor or in-kernel time) | 146.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 205.09 | 160.47 | 0.00 |  | 0.00 |  | 160.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 199.41 | 154.78 | 0.00 |  | 0.00 |  | 154.78 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-housekeeping/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194546 | 1194546 | 1.00 | 128597171 | 107.65 | 1071643 | 1061689 | 99.1 | 84.1 | true | 1 | 1 | 1546.1 | 174790 / 191 / 8 | 0 | 0 |

#### Stalls: r1-housekeeping/pp-recv-busy-10000, loop 0 on CPU 2

127402624 iterations, longest 1546.1 µs; over 5 / 20 / 100 µs: 174790 / 191 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 144 | 7926.7 | 1546.1 |
| mostly unexplained (+timer tick_nohz_handler) | 12 | 1697.1 | 899.0 |
| mostly unexplained (+softirq NET_RX) | 23 | 908.8 | 68.4 |
| preempted by swapper/2 | 1 | 555.7 | 555.7 |
| mostly unexplained (+reschedule (IPI/vector)) | 4 | 84.9 | 22.0 |
| mostly unexplained (+ipi_send_cpu) | 3 | 67.8 | 25.8 |
| mostly unexplained (+softirq TIMER) | 3 | 66.2 | 23.3 |
| mostly unexplained (+preempted by ksoftirqd/2) | 1 | 38.9 | 38.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1546.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 1496.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 899.0 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4, softirq TIMER 1 |
| 4 | 557.7 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 5 | 555.7 | 1 | preempted by swapper/2 | preempted by swapper/2 536, call function single (IPI/vector) 2, softirq NET_RX 1, page fault 0 |
| 6 | 141.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 115.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 112.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 97.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 70.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 76791 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 43 |
| IWI IRQ work interrupts | 3 |

#### Slowest round trips: r1-housekeeping/pp-recv-busy-10000 (median 46.75)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1571.00 | 1524.25 | 1545.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 60.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 1045.76 | 999.00 | 1040.16 | mostly unexplained (+timer tick_nohz_handler) | 20.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 787.51 | 740.75 | 0.00 |  | 0.00 |  | 740.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 648.36 | 601.61 | 0.00 |  | 99.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 502.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 640.14 | 593.39 | 0.00 |  | 14.55 | unexplained: no kernel event (hypervisor or in-kernel time) | 578.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 599.21 | 552.46 | 557.35 | mostly unexplained (+timer tick_nohz_handler) | 20.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 7 | 598.84 | 552.09 | 555.38 | preempted by swapper/2 | 20.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 8 | 505.25 | 458.49 | 0.00 |  | 21.30 | unexplained: no kernel event (hypervisor or in-kernel time) | 437.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 433.14 | 386.38 | 0.00 |  | 20.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 365.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 428.13 | 381.37 | 0.00 |  | 61.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 319.62 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-housekeeping/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1228764 | 1228764 | 1.00 | 3686293 | 3.00 | 61438 | 50226203 | 0.0 | 11.6 | false | 0 | 0 | 683.6 | 11220 / 63 / 1 | 0 | 0 |

#### Stalls: r1-housekeeping/pp-uring-spin-0, loop 0 on CPU 2

3013572160 iterations, longest 683.6 µs; over 5 / 20 / 100 µs: 11220 / 63 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 55 | 1841.3 | 88.6 |
| mostly unexplained (+timer tick_nohz_handler) | 5 | 786.4 | 683.6 |
| mostly unexplained (+softirq RCU) | 2 | 45.0 | 23.8 |
| mostly unexplained (+softirq TIMER) | 1 | 24.9 | 24.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 683.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 2 | 88.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 65.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 59.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 50.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 46.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 44.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 44.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 43.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 42.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60143 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-housekeeping/pp-uring-spin-0 (median 44.54)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 2020.14 | 1975.60 | 0.00 |  | 170.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 1805.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 810.16 | 765.61 | 683.55 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 82.06 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 770.84 | 726.29 | 0.00 |  | 25.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 700.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 597.41 | 552.87 | 0.00 |  | 40.55 | unexplained: no kernel event (hypervisor or in-kernel time) | 512.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 566.37 | 521.82 | 0.00 |  | 43.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 478.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 497.10 | 452.55 | 0.00 |  | 20.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 432.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 443.89 | 399.35 | 0.00 |  | 0.00 |  | 399.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 436.51 | 391.96 | 0.00 |  | 21.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 370.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 400.32 | 355.77 | 0.00 |  | 0.00 |  | 355.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 337.46 | 292.92 | 0.00 |  | 0.00 |  | 292.92 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-housekeeping/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199574 | 1199574 | 1.00 | 3598723 | 3.00 | 29989 | 39300986 | 0.0 | 5.9 | false | 0 | 0 | 99.9 | 19213 / 166 / 0 | 0 | 0 |

#### Stalls: r1-housekeeping/pp-uring-spin-10000, loop 0 on CPU 2

4716118336 iterations, longest 99.9 µs; over 5 / 20 / 100 µs: 19213 / 166 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 135 | 4281.0 | 99.9 |
| mostly unexplained (+timer tick_nohz_handler) | 24 | 605.0 | 58.1 |
| mostly unexplained (+softirq TIMER) | 6 | 168.0 | 49.6 |
| mostly unexplained (+softirq RCU) | 1 | 22.2 | 22.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 99.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 72.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 72.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 63.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 62.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 58.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 7 | 57.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 56.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 53.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 52.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120122 |
| CAL Function call interrupts | 1019 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-housekeeping/pp-uring-spin-10000 (median 47.74)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 946.29 | 898.55 | 0.00 |  | 0.00 |  | 898.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 890.43 | 842.69 | 0.00 |  | 0.00 |  | 842.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 458.62 | 410.87 | 0.00 |  | 152.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 258.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 454.93 | 407.18 | 0.00 |  | 17.60 | unexplained: no kernel event (hypervisor or in-kernel time) | 389.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 444.55 | 396.81 | 0.00 |  | 78.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 318.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 420.78 | 373.04 | 0.00 |  | 336.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 36.72 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 410.01 | 362.27 | 0.00 |  | 0.00 |  | 362.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 405.72 | 357.98 | 0.00 |  | 20.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 337.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 400.36 | 352.62 | 0.00 |  | 368.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 397.08 | 349.34 | 22.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 306.94 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-housekeeping/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1271744 | 1271744 | 1.00 | 1 | 0.00 | 0 | 49075481 | 0.0 | 0.0 | true | 0 | 0 | 569.7 | 273 / 60 / 2 | 0 | 0 |

#### Stalls: r1-housekeeping/pp-uring-sqpoll-0, loop 0 on CPU 2

2944528832 iterations, longest 569.7 µs; over 5 / 20 / 100 µs: 273 / 60 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 60 | 2496.3 | 569.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 569.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 120.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 59.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 58.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 57.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 47.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 42.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 42.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 42.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 41.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1026 |
| RES Rescheduling interrupts | 59 |
| HVS Hyper-V stimer0 interrupts | 3 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-housekeeping/pp-uring-sqpoll-0 (median 44.62)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 573.64 | 529.02 | 569.31 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 462.47 | 417.84 | 0.00 |  | 407.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 10.36 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 460.18 | 415.56 | 0.00 |  | 21.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 394.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 413.60 | 368.98 | 0.00 |  | 0.00 |  | 368.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 377.04 | 332.41 | 0.00 |  | 54.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 277.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 346.22 | 301.60 | 0.00 |  | 331.69 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 272.56 | 227.93 | 0.00 |  | 42.99 | unexplained: no kernel event (hypervisor or in-kernel time) | 184.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 261.37 | 216.74 | 0.00 |  | 0.00 |  | 216.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 238.45 | 193.83 | 0.00 |  | 20.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 173.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 230.60 | 185.98 | 0.00 |  | 0.00 |  | 185.98 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-housekeeping/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199597 | 1199597 | 1.00 | 1 | 0.00 | 0 | 36283260 | 0.0 | 0.0 | true | 0 | 0 | 999.9 | 494 / 110 / 9 | 0 | 0 |

#### Stalls: r1-housekeeping/pp-uring-sqpoll-10000, loop 0 on CPU 2

4353991168 iterations, longest 999.9 µs; over 5 / 20 / 100 µs: 494 / 110 / 9

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 110 | 7599.6 | 999.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 999.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 901.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 836.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 813.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 157.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 157.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 153.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 115.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 110.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 96.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1034 |
| RES Rescheduling interrupts | 118 |
| HVS Hyper-V stimer0 interrupts | 3 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-housekeeping/pp-uring-sqpoll-10000 (median 47.32)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1006.37 | 959.05 | 999.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 40.60 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 947.30 | 899.98 | 900.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 111.74 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 846.38 | 799.06 | 836.06 | unexplained: no kernel event (hypervisor or in-kernel time) | 27.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 814.87 | 767.54 | 813.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 638.70 | 591.38 | 0.00 |  | 20.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 571.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 524.14 | 476.82 | 0.00 |  | 21.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 455.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 439.01 | 391.69 | 0.00 |  | 0.00 |  | 391.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 436.94 | 389.62 | 0.00 |  | 26.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 363.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 432.53 | 385.20 | 0.00 |  | 0.00 |  | 385.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 421.03 | 373.71 | 0.00 |  | 21.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 352.52 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-prefer/http-recv-busy-10000, loop 0 on CPU 2

1795686477 iterations, longest 815.2 µs; over 5 / 20 / 100 µs: 1023722 / 227 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 166 | 5679.7 | 133.6 |
| mostly unexplained (+timer tick_nohz_handler) | 6 | 1721.8 | 815.2 |
| mostly unexplained (+softirq NET_RX) | 38 | 1516.2 | 139.5 |
| mostly unexplained (+ipi_send_cpu) | 8 | 236.0 | 41.5 |
| loop work | 7 | 192.0 | 36.6 |
| softirq NET_RX | 2 | 78.8 | 50.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 815.2 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3, softirq NET_RX 1, softirq TIMER 1, timer napi_watchdog 1 |
| 2 | 642.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3, softirq NET_RX 2, timer napi_watchdog 1 |
| 3 | 155.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3, softirq RCU 2, softirq TIMER 1 |
| 4 | 139.5 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 7 |
| 5 | 133.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 118.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 102.4 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2 |
| 8 | 90.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 89.6 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2 |
| 10 | 87.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120059 |
| CAL Function call interrupts | 1016 |
| IWI IRQ work interrupts | 1 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-prefer/http-recv-busy-10000 (median 50.63)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 704.78 | 654.14 | 642.35 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 11.80 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 604.51 | 553.88 | 0.00 |  | 379.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 174.71 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 482.95 | 432.32 | 0.00 |  | 58.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 373.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 443.61 | 392.98 | 0.00 |  | 0.00 |  | 392.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 423.54 | 372.90 | 0.00 |  | 422.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 420.62 | 369.98 | 0.00 |  | 0.00 |  | 369.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 404.97 | 354.33 | 0.00 |  | 0.00 |  | 354.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 391.03 | 340.40 | 0.00 |  | 353.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 385.02 | 334.39 | 0.00 |  | 254.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 80.20 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 362.97 | 312.33 | 0.00 |  | 361.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

## r1-prefer/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=547829 clocksource=tsc

### Loop counters (r1-prefer/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001550 | 906197 | 3.31 | 27174336 | 9.05 | 452906 | 452906 | 96.7 | 91.2 | true | 0 | 0 | 762.1 | 887728 / 6797 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8211 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672347200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (155004 of 2900000 in 1-msg reads) | 155004 | 6.50 | 5.34 | 8.58 | 23.16 | 27.21 | 40.16 | 45.66 |

#### Stalls: r1-prefer/md-recv-busy, loop 0 on CPU 2

27174336 iterations, longest 762.1 µs; over 5 / 20 / 100 µs: 887728 / 6797 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5510 | 134773.5 | 762.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1163 | 25971.2 | 95.5 |
| softirq NET_RX | 102 | 2290.0 | 37.2 |
| mostly unexplained (+softirq NET_RX) | 16 | 352.6 | 23.0 |
| mostly unexplained (+timer tick_nohz_handler) | 3 | 142.8 | 72.0 |
| mostly unexplained (+ipi_send_cpu) | 3 | 70.1 | 24.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 762.1 | 50 | loop work (50 items) | softirq NET_RX 31, timer napi_watchdog 1, timer tick_nohz_handler 1 |
| 2 | 657.9 | 50 | loop work (50 items) | timer tick_nohz_handler 3 |
| 3 | 398.3 | 1550 | loop work (1550 items) | timer tick_nohz_handler 1 |
| 4 | 95.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 78.5 | 14 | loop work (14 items) | softirq NET_RX 5 |
| 6 | 72.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 14 |
| 7 | 67.0 | 6 | loop work (6 items) | softirq NET_RX 2 |
| 8 | 57.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 52.7 | 8 | loop work (8 items) |  |
| 10 | 50.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60036 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

## r1-prefer/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-prefer/noise, loop 0 on CPU 2

12405129216 iterations, longest 555.9 µs; over 5 / 20 / 100 µs: 1380 / 91 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 346 | 6190.9 | 468.8 |
| mostly unexplained (+timer tick_nohz_handler) | 930 | 6137.3 | 555.9 |
| mostly unexplained (+softirq TIMER) | 104 | 588.1 | 7.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 555.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 2 | 551.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 468.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 412.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 166.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 134.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 61.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 59.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 59.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 46.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120025 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-prefer/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1252687 | 1252687 | 1.00 | 25511631 | 20.37 | 425194 | 404316 | 94.8 | 91.8 | true | 0 | 0 | 69.5 | 1776875 / 280 / 0 | 0 | 0 |

#### Stalls: r1-prefer/pp-recv-busy-0, loop 0 on CPU 2

24258944 iterations, longest 69.5 µs; over 5 / 20 / 100 µs: 1776875 / 280 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+softirq NET_RX) | 174 | 4199.2 | 65.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 86 | 2492.6 | 69.5 |
| mostly unexplained (+timer tick_nohz_handler) | 14 | 394.6 | 61.0 |
| mostly unexplained (+softirq TIMER) | 2 | 50.7 | 30.1 |
| mostly unexplained (+ipi_send_cpu) | 2 | 43.2 | 22.0 |
| timer tick_nohz_handler | 2 | 40.8 | 20.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 69.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 65.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2 |
| 3 | 64.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 62.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2 |
| 5 | 61.0 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 6 | 59.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 52.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2 |
| 8 | 51.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 48.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 48.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60246 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-prefer/pp-recv-busy-0 (median 44.95)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 792.37 | 747.41 | 0.00 |  | 20.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 727.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 567.61 | 522.65 | 0.00 |  | 20.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 502.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 397.61 | 352.65 | 0.00 |  | 41.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 311.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 388.11 | 343.15 | 0.00 |  | 0.00 |  | 343.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 366.14 | 321.18 | 0.00 |  | 0.00 |  | 321.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 286.98 | 242.02 | 0.00 |  | 20.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 221.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 283.42 | 238.46 | 0.00 |  | 23.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 215.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 258.48 | 213.53 | 0.00 |  | 0.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 213.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 181.41 | 136.45 | 0.00 |  | 0.00 |  | 136.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 167.80 | 122.85 | 0.00 |  | 0.00 |  | 122.85 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-prefer/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1183140 | 1183140 | 1.00 | 48379621 | 40.89 | 403164 | 393304 | 97.5 | 91.7 | true | 1 | 0 | 806.1 | 2111477 / 491 / 6 | 0 | 0 |

#### Stalls: r1-prefer/pp-recv-busy-10000, loop 0 on CPU 2

47196480 iterations, longest 806.1 µs; over 5 / 20 / 100 µs: 2111477 / 491 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 229 | 6966.7 | 127.8 |
| mostly unexplained (+softirq NET_RX) | 198 | 5363.0 | 345.9 |
| mostly unexplained (+timer tick_nohz_handler) | 12 | 1099.3 | 806.1 |
| mostly unexplained (+ipi_send_cpu) | 30 | 812.3 | 38.8 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 741.5 | 741.5 |
| preempted by swapper/2 | 1 | 632.0 | 632.0 |
| softirq NET_RX | 11 | 363.6 | 73.1 |
| mostly unexplained (+softirq TIMER) | 5 | 127.2 | 43.0 |
| mostly unexplained (+reschedule (IPI/vector)) | 3 | 64.7 | 23.8 |
| loop work | 1 | 20.0 | 20.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 806.1 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 2 | 741.5 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, timer napi_watchdog 1, ipi_send_cpu 0 |
| 3 | 632.0 | 1 | preempted by swapper/2 | preempted by swapper/2 602, call function single (IPI/vector) 2, softirq TIMER 1, timer tick_nohz_handler 1 |
| 4 | 345.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2, timer napi_watchdog 1 |
| 5 | 127.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 115.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 93.3 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2 |
| 8 | 78.2 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 73.1 | 1 | softirq NET_RX | softirq NET_RX 63, ipi_send_cpu 0 |
| 10 | 73.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 78525 |
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 41 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-prefer/pp-recv-busy-10000 (median 50.23)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 853.14 | 802.91 | 0.00 |  | 155.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 647.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 842.37 | 792.15 | 805.85 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 684.31 | 634.09 | 631.56 | preempted by swapper/2 | 0.00 |  | 2.53 | client: preempted by swapper/2 |
| 4 | 598.67 | 548.44 | 0.00 |  | 0.00 |  | 548.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 506.94 | 456.71 | 0.00 |  | 15.60 | unexplained: no kernel event (hypervisor or in-kernel time) | 441.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 436.94 | 386.71 | 0.00 |  | 70.94 | unexplained: no kernel event (hypervisor or in-kernel time) | 315.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 435.86 | 385.64 | 0.00 |  | 42.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 343.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 399.43 | 349.21 | 0.00 |  | 20.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 328.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 386.69 | 336.46 | 0.00 |  | 0.00 |  | 336.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 384.57 | 334.35 | 345.63 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |

### Loop counters (r1-prefer/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1228025 | 1228025 | 1.00 | 1 | 0.00 | 0 | 54257727 | 0.0 | 0.0 | true | 0 | 0 | 153.2 | 287 / 77 / 1 | 0 | 0 |

#### Stalls: r1-prefer/pp-uring-sqpoll-0, loop 0 on CPU 2

3255463616 iterations, longest 153.2 µs; over 5 / 20 / 100 µs: 287 / 77 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 77 | 2816.4 | 153.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 153.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 95.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 94.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 86.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 80.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 75.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 57.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 56.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 51.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 51.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1020 |
| RES Rescheduling interrupts | 59 |
| HVS Hyper-V stimer0 interrupts | 3 |
| IWI IRQ work interrupts | 3 |

#### Slowest round trips: r1-prefer/pp-uring-sqpoll-0 (median 45.85)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 405.95 | 360.09 | 0.00 |  | 0.00 |  | 360.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 394.06 | 348.21 | 0.00 |  | 21.31 | unexplained: no kernel event (hypervisor or in-kernel time) | 326.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 357.71 | 311.85 | 0.00 |  | 0.00 |  | 311.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 356.00 | 310.15 | 0.00 |  | 0.00 |  | 310.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 352.36 | 306.51 | 0.00 |  | 0.00 |  | 306.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 303.54 | 257.69 | 0.00 |  | 0.00 |  | 257.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 267.71 | 221.86 | 0.00 |  | 40.42 | unexplained: no kernel event (hypervisor or in-kernel time) | 181.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 234.88 | 189.02 | 215.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.60 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 205.92 | 160.07 | 0.00 |  | 0.00 |  | 160.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 169.94 | 124.09 | 0.00 |  | 0.00 |  | 124.09 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-prefer/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199656 | 1199656 | 1.00 | 1 | 0.00 | 0 | 40435028 | 0.0 | 0.0 | true | 0 | 0 | 680.7 | 505 / 116 / 5 | 0 | 0 |

#### Stalls: r1-prefer/pp-uring-sqpoll-10000, loop 0 on CPU 2

4852203328 iterations, longest 680.7 µs; over 5 / 20 / 100 µs: 505 / 116 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 116 | 5236.9 | 680.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 680.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 640.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 267.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 130.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 102.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 73.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 67.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 65.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 63.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 58.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1022 |
| RES Rescheduling interrupts | 118 |
| IWI IRQ work interrupts | 3 |
| HVS Hyper-V stimer0 interrupts | 2 |

#### Slowest round trips: r1-prefer/pp-uring-sqpoll-10000 (median 50.12)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1898.78 | 1848.66 | 0.00 |  | 20.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 1828.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 860.91 | 810.79 | 0.00 |  | 21.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 789.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 725.09 | 674.97 | 709.61 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 643.37 | 593.24 | 640.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 534.51 | 484.38 | 0.00 |  | 41.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 442.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 466.43 | 416.30 | 0.00 |  | 0.00 |  | 416.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 439.13 | 389.01 | 0.00 |  | 43.78 | unexplained: no kernel event (hypervisor or in-kernel time) | 345.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 422.07 | 371.94 | 0.00 |  | 40.31 | unexplained: no kernel event (hypervisor or in-kernel time) | 331.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 403.45 | 353.33 | 0.00 |  | 21.61 | unexplained: no kernel event (hypervisor or in-kernel time) | 331.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 351.17 | 301.05 | 0.00 |  | 20.15 | unexplained: no kernel event (hypervisor or in-kernel time) | 280.90 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-2/http-recv-busy-10000, loop 0 on CPU 2

1937398611 iterations, longest 570.1 µs; over 5 / 20 / 100 µs: 172726 / 157 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 134 | 4299.1 | 68.0 |
| preempted by swapper/2 | 1 | 570.1 | 570.1 |
| mostly unexplained (+softirq NET_RX) | 14 | 510.9 | 58.1 |
| mostly unexplained (+timer tick_nohz_handler) | 3 | 106.3 | 54.8 |
| mostly unexplained (+ipi_send_cpu) | 3 | 77.0 | 33.2 |
| loop work | 2 | 45.1 | 24.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 570.1 | 2 | preempted by swapper/2 | preempted by swapper/2 552, call function single (IPI/vector) 3, page fault 0 |
| 2 | 68.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 66.2 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 60.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 58.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 58.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 55.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 55.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 54.8 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 10 | 51.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 47018 |
| CAL Function call interrupts | 1017 |
| RES Rescheduling interrupts | 72 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-base-2/http-recv-busy-10000 (median 48.10)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 660.62 | 612.52 | 0.00 |  | 88.73 | unexplained: no kernel event (hypervisor or in-kernel time) | 523.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 614.47 | 566.37 | 569.21 | preempted by swapper/2 | 20.12 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 3 | 474.59 | 426.50 | 0.00 |  | 20.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 406.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 442.84 | 394.74 | 0.00 |  | 20.57 | unexplained: no kernel event (hypervisor or in-kernel time) | 374.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 405.46 | 357.36 | 0.00 |  | 0.00 |  | 357.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 403.42 | 355.32 | 0.00 |  | 0.00 |  | 355.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 368.24 | 320.14 | 0.00 |  | 334.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 349.07 | 300.97 | 0.00 |  | 0.00 |  | 300.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 304.31 | 256.22 | 0.00 |  | 0.00 |  | 256.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 266.87 | 218.77 | 0.00 |  | 0.00 |  | 218.77 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-2/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=667626 clocksource=tsc

### Loop counters (r1-base-2/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001550 | 954866 | 3.14 | 63065472 | 21.01 | 1051091 | 1051091 | 98.5 | 83.5 | true | 0 | 0 | 383.5 | 366538 / 229 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8210 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672347200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (227565 of 2900000 in 1-msg reads) | 227565 | 3.74 | 3.40 | 4.94 | 8.65 | 12.46 | 21.16 | 83.62 |

#### Stalls: r1-base-2/md-recv-busy, loop 0 on CPU 2

63065472 iterations, longest 383.5 µs; over 5 / 20 / 100 µs: 366538 / 229 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 119 | 3700.7 | 383.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 66 | 2350.5 | 198.4 |
| softirq NET_RX | 34 | 752.4 | 29.0 |
| mostly unexplained (+timer tick_nohz_handler) | 7 | 256.3 | 69.9 |
| mostly unexplained (+softirq NET_RX) | 2 | 63.4 | 39.5 |
| mostly unexplained (+ipi_send_cpu) | 1 | 24.3 | 24.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 383.5 | 1550 | loop work (1550 items) |  |
| 2 | 198.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 101.4 | 32 | loop work (32 items) | softirq NET_RX 7 |
| 4 | 99.5 | 7 | loop work (7 items) |  |
| 5 | 79.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 77.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 72.2 | 15 | loop work (15 items) | softirq NET_RX 4 |
| 8 | 69.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 65.6 | 17 | loop work (17 items) | softirq NET_RX 12 |
| 10 | 65.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60033 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## r1-base-2/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-2/noise, loop 0 on CPU 2

12314628096 iterations, longest 577.0 µs; over 5 / 20 / 100 µs: 1694 / 109 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 1196 | 6506.1 | 72.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 398 | 6483.7 | 577.0 |
| mostly unexplained (+softirq TIMER) | 97 | 552.3 | 7.3 |
| timer tick_nohz_handler | 3 | 22.7 | 7.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 577.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 79.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 72.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 72.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 8 |
| 5 | 53.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 6 | 44.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 44.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 43.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 43.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 42.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120025 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-base-2/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1195415 | 1195415 | 1.00 | 306165423 | 256.12 | 5102757 | 5062910 | 0.0 | 37.1 | false | 0 | 0 | 71.2 | 11317 / 73 / 0 | 0 | 0 |

#### Stalls: r1-base-2/pp-epoll-spin-0, loop 0 on CPU 2

303774592 iterations, longest 71.2 µs; over 5 / 20 / 100 µs: 11317 / 73 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 65 | 2093.0 | 71.2 |
| mostly unexplained (+timer tick_nohz_handler) | 5 | 118.0 | 29.9 |
| mostly unexplained (+softirq RCU) | 2 | 48.7 | 28.4 |
| mostly unexplained (+softirq TIMER) | 1 | 20.1 | 20.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 71.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 69.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 60.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 47.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 46.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 45.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 45.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 45.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 43.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60148 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-2/pp-epoll-spin-0 (median 47.05)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 817.81 | 770.76 | 0.00 |  | 0.00 |  | 770.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 764.75 | 717.70 | 0.00 |  | 20.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 697.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 727.49 | 680.44 | 0.00 |  | 0.00 |  | 680.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 529.77 | 482.71 | 0.00 |  | 315.69 | unexplained: no kernel event (hypervisor or in-kernel time) | 167.03 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 472.05 | 425.00 | 0.00 |  | 60.47 | unexplained: no kernel event (hypervisor or in-kernel time) | 364.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 468.76 | 421.71 | 0.00 |  | 0.00 |  | 421.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 461.00 | 413.95 | 0.00 |  | 90.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 323.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 447.60 | 400.55 | 0.00 |  | 0.00 |  | 400.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 396.15 | 349.10 | 0.00 |  | 235.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 114.01 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 370.61 | 323.56 | 0.00 |  | 0.00 |  | 323.56 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198560 | 1198560 | 1.00 | 615241984 | 513.32 | 5127017 | 5107041 | 0.0 | 32.6 | false | 0 | 0 | 1128.5 | 17122 / 167 / 11 | 0 | 0 |

#### Stalls: r1-base-2/pp-epoll-spin-10000, loop 0 on CPU 2

612844864 iterations, longest 1128.5 µs; over 5 / 20 / 100 µs: 17122 / 167 / 11

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 127 | 4862.9 | 375.5 |
| mostly unexplained (+timer tick_nohz_handler) | 24 | 4647.2 | 1128.5 |
| mostly unexplained (+softirq TIMER) | 10 | 240.1 | 40.8 |
| mostly unexplained (+softirq RCU) | 6 | 147.5 | 37.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1128.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 2 | 1043.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 953.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq TIMER 1 |
| 4 | 881.8 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 5 | 375.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 179.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 168.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 144.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 126.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 107.4 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3, softirq TIMER 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120112 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-2/pp-epoll-spin-10000 (median 47.21)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1776.39 | 1729.18 | 0.00 |  | 168.62 | unexplained: no kernel event (hypervisor or in-kernel time) | 1560.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1517.73 | 1470.52 | 1503.77 | mostly unexplained (+timer tick_nohz_handler) | 40.56 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 1317.89 | 1270.68 | 1043.00 | mostly unexplained (+timer tick_nohz_handler) | 40.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 187.31 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 1229.44 | 1182.22 | 953.88 | mostly unexplained (+timer tick_nohz_handler) | 46.60 | unexplained: no kernel event (hypervisor or in-kernel time) | 181.74 | client: mostly unexplained (+timer tick_nohz_handler) |
| 5 | 998.96 | 951.75 | 0.00 |  | 48.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 903.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 899.79 | 852.58 | 881.51 | mostly unexplained (+timer tick_nohz_handler) | 20.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 7 | 805.74 | 758.52 | 0.00 |  | 49.55 | unexplained: no kernel event (hypervisor or in-kernel time) | 708.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 560.21 | 513.00 | 0.00 |  | 0.00 |  | 513.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 519.35 | 472.14 | 0.00 |  | 46.69 | unexplained: no kernel event (hypervisor or in-kernel time) | 425.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 443.22 | 396.01 | 0.00 |  | 478.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-2/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1254439 | 1254439 | 1.00 | 60210600 | 48.00 | 1003510 | 982603 | 97.9 | 85.7 | true | 0 | 0 | 140.2 | 137559 / 132 / 2 | 0 | 0 |

#### Stalls: r1-base-2/pp-recv-busy-0, loop 0 on CPU 2

58956160 iterations, longest 140.2 µs; over 5 / 20 / 100 µs: 137559 / 132 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 78 | 2638.3 | 140.2 |
| mostly unexplained (+softirq NET_RX) | 29 | 1070.1 | 86.8 |
| mostly unexplained (+timer tick_nohz_handler) | 18 | 547.1 | 69.1 |
| mostly unexplained (+softirq TIMER) | 3 | 70.0 | 23.6 |
| mostly unexplained (+ipi_send_cpu) | 2 | 47.8 | 26.0 |
| softirq NET_RX | 1 | 23.9 | 23.9 |
| mostly unexplained (+softirq RCU) | 1 | 22.5 | 22.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 140.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 108.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 86.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 4 | 73.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 69.1 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 6 | 65.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 65.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 18 |
| 8 | 63.2 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 59.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 12, timer tick_nohz_handler 2, softirq TIMER 1 |
| 10 | 56.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60144 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-2/pp-recv-busy-0 (median 45.19)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 554.50 | 509.31 | 0.00 |  | 35.70 | unexplained: no kernel event (hypervisor or in-kernel time) | 473.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 412.11 | 366.93 | 0.00 |  | 40.73 | unexplained: no kernel event (hypervisor or in-kernel time) | 326.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 375.36 | 330.18 | 0.00 |  | 88.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 241.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 311.96 | 266.77 | 0.00 |  | 0.00 |  | 266.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 290.37 | 245.18 | 0.00 |  | 170.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 74.32 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 283.54 | 238.35 | 0.00 |  | 40.15 | unexplained: no kernel event (hypervisor or in-kernel time) | 198.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 273.44 | 228.26 | 0.00 |  | 60.69 | unexplained: no kernel event (hypervisor or in-kernel time) | 167.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 185.62 | 140.43 | 0.00 |  | 0.00 |  | 140.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 144.76 | 99.58 | 139.93 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.30 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 142.79 | 97.60 | 0.00 |  | 72.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 24.64 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-2/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194578 | 1194578 | 1.00 | 128344595 | 107.44 | 1069538 | 1059583 | 99.1 | 84.0 | true | 1 | 0 | 631.2 | 156381 / 180 / 1 | 0 | 0 |

#### Stalls: r1-base-2/pp-recv-busy-10000, loop 0 on CPU 2

127150016 iterations, longest 631.2 µs; over 5 / 20 / 100 µs: 156381 / 180 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 134 | 4253.9 | 99.8 |
| mostly unexplained (+softirq NET_RX) | 24 | 966.5 | 86.2 |
| preempted by swapper/2 | 1 | 631.2 | 631.2 |
| mostly unexplained (+timer tick_nohz_handler) | 10 | 283.8 | 46.3 |
| mostly unexplained (+ipi_send_cpu) | 4 | 98.4 | 28.4 |
| mostly unexplained (+softirq TIMER) | 3 | 74.4 | 26.9 |
| mostly unexplained (+reschedule (IPI/vector)) | 3 | 61.5 | 20.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 34.8 | 34.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 631.2 | 1 | preempted by swapper/2 | preempted by swapper/2 602, softirq NET_RX 13, call function single (IPI/vector) 3, page fault 0 |
| 2 | 99.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 86.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 4 | 67.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 67.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 65.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 62.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 60.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 53.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 10 | 50.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 76507 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 42 |
| IWI IRQ work interrupts | 4 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-2/pp-recv-busy-10000 (median 46.94)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1707.45 | 1660.51 | 0.00 |  | 184.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 1476.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 672.27 | 625.33 | 630.75 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 3 | 566.81 | 519.87 | 0.00 |  | 0.00 |  | 519.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 424.34 | 377.40 | 0.00 |  | 159.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 218.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 423.82 | 376.88 | 0.00 |  | 21.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 355.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 419.78 | 372.84 | 0.00 |  | 0.00 |  | 372.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 410.74 | 363.80 | 0.00 |  | 0.00 |  | 363.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 405.77 | 358.83 | 0.00 |  | 0.00 |  | 358.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 398.03 | 351.09 | 0.00 |  | 24.64 | unexplained: no kernel event (hypervisor or in-kernel time) | 326.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 363.65 | 316.72 | 0.00 |  | 41.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 275.72 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1182693 | 1182693 | 1.00 | 3548080 | 3.00 | 59135 | 50555646 | 0.0 | 11.4 | false | 0 | 0 | 948.1 | 19636 / 109 / 4 | 0 | 0 |

#### Stalls: r1-base-2/pp-uring-spin-0, loop 0 on CPU 2

3033338752 iterations, longest 948.1 µs; over 5 / 20 / 100 µs: 19636 / 109 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 69 | 3045.6 | 829.7 |
| mostly unexplained (+timer tick_nohz_handler) | 27 | 2239.3 | 948.1 |
| mostly unexplained (+softirq TIMER) | 7 | 151.0 | 23.3 |
| mostly unexplained (+softirq RCU) | 6 | 146.1 | 30.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 948.1 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 2 | 829.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 671.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3, softirq TIMER 1 |
| 4 | 148.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 79.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 72.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 7 | 59.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 57.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 46.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 45.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60144 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-2/pp-uring-spin-0 (median 46.51)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1158.51 | 1112.00 | 1113.49 | mostly unexplained (+timer tick_nohz_handler) | 41.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 928.15 | 881.64 | 864.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 716.25 | 669.74 | 671.34 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 442.57 | 396.06 | 0.00 |  | 40.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 355.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 418.42 | 371.91 | 0.00 |  | 0.00 |  | 371.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 407.94 | 361.43 | 0.00 |  | 40.47 | unexplained: no kernel event (hypervisor or in-kernel time) | 320.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 404.58 | 358.07 | 0.00 |  | 0.00 |  | 358.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 402.88 | 356.38 | 0.00 |  | 41.56 | unexplained: no kernel event (hypervisor or in-kernel time) | 314.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 402.10 | 355.59 | 0.00 |  | 20.94 | unexplained: no kernel event (hypervisor or in-kernel time) | 334.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 396.28 | 349.77 | 0.00 |  | 20.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 329.27 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199549 | 1199549 | 1.00 | 3598648 | 3.00 | 29989 | 39228241 | 0.0 | 5.8 | false | 1 | 0 | 638.3 | 10381 / 124 / 3 | 0 | 0 |

#### Stalls: r1-base-2/pp-uring-spin-10000, loop 0 on CPU 2

4707388864 iterations, longest 638.3 µs; over 5 / 20 / 100 µs: 10381 / 124 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 109 | 3672.8 | 144.1 |
| preempted by swapper/2 | 1 | 638.3 | 638.3 |
| mostly unexplained (+timer tick_nohz_handler) | 5 | 558.8 | 402.5 |
| mostly unexplained (+softirq TIMER) | 4 | 150.6 | 64.3 |
| mostly unexplained (+softirq RCU) | 4 | 93.9 | 26.2 |
| mostly unexplained (+reschedule (IPI/vector)) | 1 | 22.6 | 22.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 638.3 | 0 | preempted by swapper/2 | preempted by swapper/2 613, call function single (IPI/vector) 3, page fault 0 |
| 2 | 402.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 3 | 144.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 77.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 69.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 68.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 64.3 | 0 | mostly unexplained (+softirq TIMER) | softirq TIMER 1, timer tick_nohz_handler 1 |
| 8 | 62.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 55.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 54.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 83121 |
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 37 |

#### Slowest round trips: r1-base-2/pp-uring-spin-10000 (median 47.84)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 745.10 | 697.26 | 0.00 |  | 0.00 |  | 697.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 732.46 | 684.62 | 121.77 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 562.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 688.38 | 640.55 | 638.30 | preempted by swapper/2 | 0.00 |  | 2.25 | client: preempted by swapper/2 |
| 4 | 552.28 | 504.44 | 402.47 | mostly unexplained (+timer tick_nohz_handler) | 21.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 80.91 | client: mostly unexplained (+timer tick_nohz_handler) |
| 5 | 505.08 | 457.25 | 0.00 |  | 0.00 |  | 457.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 483.98 | 436.14 | 0.00 |  | 0.00 |  | 436.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 470.79 | 422.95 | 0.00 |  | 20.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 402.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 440.40 | 392.57 | 0.00 |  | 0.00 |  | 392.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 437.25 | 389.41 | 0.00 |  | 61.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 328.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 422.01 | 374.17 | 0.00 |  | 41.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 332.79 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1252678 | 1252678 | 1.00 | 1 | 0.00 | 0 | 49116990 | 0.0 | 0.0 | true | 0 | 0 | 115.2 | 205 / 56 / 1 | 0 | 0 |

#### Stalls: r1-base-2/pp-uring-sqpoll-0, loop 0 on CPU 2

2947019392 iterations, longest 115.2 µs; over 5 / 20 / 100 µs: 205 / 56 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 55 | 1720.9 | 115.2 |
| softirq TIMER | 1 | 48.4 | 48.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 115.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 52.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 48.4 | 0 | softirq TIMER | softirq TIMER 28, timer tick_nohz_handler 4, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 45.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 45.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 44.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 44.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 41.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 40.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 39.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 59 |
| IWI IRQ work interrupts | 7 |
| HVS Hyper-V stimer0 interrupts | 6 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-2/pp-uring-sqpoll-0 (median 45.03)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 530.64 | 485.61 | 0.00 |  | 0.00 |  | 485.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 473.04 | 428.01 | 0.00 |  | 0.00 |  | 428.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 446.16 | 401.12 | 0.00 |  | 21.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 379.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 396.77 | 351.74 | 0.00 |  | 21.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 330.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 324.44 | 279.40 | 0.00 |  | 40.54 | unexplained: no kernel event (hypervisor or in-kernel time) | 238.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 287.27 | 242.24 | 0.00 |  | 20.63 | unexplained: no kernel event (hypervisor or in-kernel time) | 221.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 231.21 | 186.18 | 0.00 |  | 0.00 |  | 186.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 217.82 | 172.79 | 0.00 |  | 0.00 |  | 172.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 203.49 | 158.46 | 0.00 |  | 0.00 |  | 158.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 186.00 | 140.97 | 0.00 |  | 40.30 | unexplained: no kernel event (hypervisor or in-kernel time) | 100.67 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199620 | 1199620 | 1.00 | 1 | 0.00 | 0 | 35546984 | 0.0 | 0.0 | true | 0 | 0 | 419.4 | 448 / 101 / 5 | 0 | 0 |

#### Stalls: r1-base-2/pp-uring-sqpoll-10000, loop 0 on CPU 2

4265638080 iterations, longest 419.4 µs; over 5 / 20 / 100 µs: 448 / 101 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 101 | 3990.1 | 419.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 419.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 155.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 123.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 106.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 105.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 81.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 80.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 66.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 57.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 50.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1026 |
| RES Rescheduling interrupts | 117 |
| IWI IRQ work interrupts | 7 |
| HVS Hyper-V stimer0 interrupts | 3 |

#### Slowest round trips: r1-base-2/pp-uring-sqpoll-10000 (median 47.58)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 700.86 | 653.28 | 0.00 |  | 0.00 |  | 653.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 604.51 | 556.93 | 0.00 |  | 41.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 515.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 561.46 | 513.88 | 0.00 |  | 0.00 |  | 513.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 464.05 | 416.47 | 419.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 41.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 405.17 | 357.59 | 0.00 |  | 0.00 |  | 357.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 392.58 | 345.00 | 0.00 |  | 0.00 |  | 345.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 389.50 | 341.93 | 0.00 |  | 42.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 299.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 351.61 | 304.03 | 0.00 |  | 0.00 |  | 304.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 279.85 | 232.27 | 0.00 |  | 0.00 |  | 232.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 259.69 | 212.11 | 0.00 |  | 0.00 |  | 212.11 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-noquickack/http-recv-busy-10000, loop 0 on CPU 2

1896377965 iterations, longest 908.9 µs; over 5 / 20 / 100 µs: 164564 / 196 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 126 | 5999.8 | 908.9 |
| mostly unexplained (+softirq NET_RX) | 25 | 902.8 | 72.1 |
| mostly unexplained (+softirq RCU) | 23 | 553.6 | 33.5 |
| mostly unexplained (+timer tick_nohz_handler) | 18 | 535.3 | 76.1 |
| mostly unexplained (+ipi_send_cpu) | 3 | 70.2 | 24.0 |
| loop work | 1 | 24.3 | 24.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 908.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 588.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 186.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 177.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 155.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 126.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 108.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 76.1 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 9 | 72.1 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 10 | 67.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120053 |
| CAL Function call interrupts | 1017 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-noquickack/http-recv-busy-10000 (median 48.22)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 919.11 | 870.89 | 908.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 757.79 | 709.57 | 0.00 |  | 0.00 |  | 709.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 600.14 | 551.92 | 0.00 |  | 21.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 530.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 533.79 | 485.57 | 0.00 |  | 459.66 | unexplained: no kernel event (hypervisor or in-kernel time) | 25.91 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 443.10 | 394.88 | 0.00 |  | 0.00 |  | 394.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 372.17 | 323.95 | 0.00 |  | 0.00 |  | 323.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 367.46 | 319.24 | 0.00 |  | 29.78 | unexplained: no kernel event (hypervisor or in-kernel time) | 289.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 362.70 | 314.48 | 0.00 |  | 116.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 197.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 348.51 | 300.29 | 0.00 |  | 20.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 279.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 310.26 | 262.04 | 0.00 |  | 288.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

## r1-noquickack/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=665679 clocksource=tsc

### Loop counters (r1-noquickack/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001550 | 938093 | 3.20 | 62663040 | 20.88 | 1044384 | 1044384 | 98.5 | 83.4 | true | 0 | 0 | 694.2 | 379063 / 376 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8213 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672347200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (217464 of 2900000 in 1-msg reads) | 217464 | 3.79 | 3.36 | 5.68 | 9.00 | 15.09 | 23.35 | 46.57 |

#### Stalls: r1-noquickack/md-recv-busy, loop 0 on CPU 2

62663040 iterations, longest 694.2 µs; over 5 / 20 / 100 µs: 379063 / 376 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 222 | 6391.7 | 694.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 85 | 2740.5 | 124.2 |
| softirq NET_RX | 60 | 1398.3 | 42.7 |
| mostly unexplained (+timer tick_nohz_handler) | 4 | 136.0 | 62.1 |
| mostly unexplained (+ipi_send_cpu) | 3 | 69.5 | 27.9 |
| mostly unexplained (+softirq RCU) | 1 | 47.1 | 47.1 |
| mostly unexplained (+softirq NET_RX) | 1 | 30.1 | 30.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 694.2 | 50 | loop work (50 items) | timer tick_nohz_handler 1 |
| 2 | 442.3 | 1550 | loop work (1550 items) | timer tick_nohz_handler 2 |
| 3 | 124.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 111.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 95.3 | 25 | loop work (25 items) |  |
| 6 | 62.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 7 | 58.9 | 16 | loop work (16 items) | softirq NET_RX 8 |
| 8 | 56.2 | 17 | loop work (17 items) | softirq NET_RX 11 |
| 9 | 53.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 52.4 | 11 | loop work (11 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60037 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

## r1-noquickack/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-noquickack/noise, loop 0 on CPU 2

12366872576 iterations, longest 357.2 µs; over 5 / 20 / 100 µs: 1697 / 106 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 1204 | 6977.9 | 215.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 397 | 6679.8 | 357.2 |
| mostly unexplained (+softirq TIMER) | 95 | 529.7 | 7.0 |
| timer tick_nohz_handler | 1 | 7.2 | 7.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 357.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 215.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 191.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 165.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 5 | 104.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 102.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 84.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 74.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 69.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 67.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120024 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-noquickack/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1263258 | 1263258 | 1.00 | 60090587 | 47.57 | 1001510 | 980455 | 97.9 | 85.7 | true | 0 | 0 | 708.1 | 144578 / 106 / 2 | 0 | 0 |

#### Stalls: r1-noquickack/pp-recv-busy-0, loop 0 on CPU 2

58827328 iterations, longest 708.1 µs; over 5 / 20 / 100 µs: 144578 / 106 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 75 | 2583.8 | 78.6 |
| mostly unexplained (+timer tick_nohz_handler) | 12 | 1727.6 | 708.1 |
| mostly unexplained (+softirq NET_RX) | 11 | 315.4 | 66.3 |
| mostly unexplained (+ipi_send_cpu) | 4 | 95.5 | 26.4 |
| mostly unexplained (+softirq TIMER) | 3 | 73.7 | 26.8 |
| softirq NET_RX | 1 | 21.6 | 21.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 708.1 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 2 | 702.6 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 3 | 78.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 69.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 66.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 61.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 60.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 60.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 57.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 57.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60046 |
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-noquickack/pp-recv-busy-0 (median 44.84)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 2738.55 | 2693.71 | 0.00 |  | 122.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 2571.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 725.10 | 680.26 | 702.35 | mostly unexplained (+timer tick_nohz_handler) | 20.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 720.75 | 675.92 | 707.96 | mostly unexplained (+timer tick_nohz_handler) | 20.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 492.60 | 447.77 | 0.00 |  | 62.22 | unexplained: no kernel event (hypervisor or in-kernel time) | 385.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 446.49 | 401.66 | 0.00 |  | 60.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 340.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 381.29 | 336.45 | 0.00 |  | 40.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 295.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 359.63 | 314.80 | 0.00 |  | 40.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 274.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 305.56 | 260.72 | 0.00 |  | 0.00 |  | 260.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 286.52 | 241.69 | 0.00 |  | 40.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 201.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 232.13 | 187.29 | 0.00 |  | 20.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 167.28 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-noquickack/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194572 | 1194572 | 1.00 | 127964365 | 107.12 | 1066370 | 1056415 | 99.1 | 84.0 | true | 1 | 0 | 579.6 | 176204 / 277 / 3 | 0 | 0 |

#### Stalls: r1-noquickack/pp-recv-busy-10000, loop 0 on CPU 2

126769792 iterations, longest 579.6 µs; over 5 / 20 / 100 µs: 176204 / 277 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 233 | 6997.2 | 108.3 |
| mostly unexplained (+softirq NET_RX) | 20 | 1015.3 | 368.8 |
| preempted by swapper/2 | 1 | 579.6 | 579.6 |
| mostly unexplained (+timer tick_nohz_handler) | 7 | 233.5 | 65.4 |
| mostly unexplained (+softirq TIMER) | 3 | 157.8 | 75.9 |
| mostly unexplained (+ipi_send_cpu) | 5 | 124.8 | 28.7 |
| softirq NET_RX | 2 | 74.3 | 38.9 |
| mostly unexplained (+reschedule (IPI/vector)) | 3 | 69.7 | 25.2 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 46.0 | 46.0 |
| mostly unexplained (+softirq RCU) | 2 | 45.7 | 23.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 579.6 | 1 | preempted by swapper/2 | preempted by swapper/2 561, call function single (IPI/vector) 2, page fault 0 |
| 2 | 368.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1, timer tick_nohz_handler 1 |
| 3 | 108.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 88.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 75.9 | 1 | mostly unexplained (+softirq TIMER) | softirq TIMER 1 |
| 6 | 71.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 66.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 65.4 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 65.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 64.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 77994 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 42 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-noquickack/pp-recv-busy-10000 (median 46.74)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 625.63 | 578.89 | 579.34 | preempted by swapper/2 | 21.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 2 | 530.54 | 483.81 | 0.00 |  | 20.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 463.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 492.99 | 446.25 | 0.00 |  | 20.85 | unexplained: no kernel event (hypervisor or in-kernel time) | 425.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 457.11 | 410.37 | 0.00 |  | 5.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 405.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 436.71 | 389.97 | 0.00 |  | 42.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 347.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 375.03 | 328.29 | 368.62 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 7 | 374.55 | 327.81 | 0.00 |  | 20.89 | unexplained: no kernel event (hypervisor or in-kernel time) | 306.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 370.10 | 323.36 | 0.00 |  | 61.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 262.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 356.13 | 309.40 | 0.00 |  | 20.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 289.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 317.16 | 270.42 | 0.00 |  | 0.00 |  | 270.42 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-noquickack/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1226086 | 1226086 | 1.00 | 3678259 | 3.00 | 61304 | 50090899 | 0.0 | 11.7 | false | 0 | 0 | 1595.4 | 11489 / 100 / 2 | 0 | 0 |

#### Stalls: r1-noquickack/pp-uring-spin-0, loop 0 on CPU 2

3005453952 iterations, longest 1595.4 µs; over 5 / 20 / 100 µs: 11489 / 100 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 86 | 2800.7 | 92.7 |
| mostly unexplained (+timer tick_nohz_handler) | 10 | 2756.7 | 1595.4 |
| mostly unexplained (+softirq RCU) | 3 | 82.8 | 30.1 |
| mostly unexplained (+softirq TIMER) | 1 | 24.1 | 24.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1595.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 2 | 936.3 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4, softirq TIMER 1 |
| 3 | 92.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 90.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 79.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 70.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 67.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 63.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 60.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 10 | 49.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60115 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-noquickack/pp-uring-spin-0 (median 44.59)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1662.08 | 1617.49 | 1595.41 | mostly unexplained (+timer tick_nohz_handler) | 40.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 554.84 | 510.25 | 0.00 |  | 0.00 |  | 510.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 405.77 | 361.18 | 0.00 |  | 0.00 |  | 361.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 386.18 | 341.59 | 0.00 |  | 0.00 |  | 341.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 361.25 | 316.66 | 0.00 |  | 0.00 |  | 316.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 359.08 | 314.49 | 0.00 |  | 20.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 293.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 319.94 | 275.35 | 0.00 |  | 38.03 | unexplained: no kernel event (hypervisor or in-kernel time) | 237.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 298.58 | 253.99 | 0.00 |  | 36.62 | unexplained: no kernel event (hypervisor or in-kernel time) | 217.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 261.51 | 216.93 | 0.00 |  | 38.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 178.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 260.54 | 215.95 | 0.00 |  | 0.00 |  | 215.95 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-noquickack/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199571 | 1199571 | 1.00 | 3598714 | 3.00 | 29989 | 39281655 | 0.0 | 5.9 | false | 0 | 0 | 710.0 | 17457 / 164 / 10 | 0 | 0 |

#### Stalls: r1-noquickack/pp-uring-spin-10000, loop 0 on CPU 2

4713798592 iterations, longest 710.0 µs; over 5 / 20 / 100 µs: 17457 / 164 / 10

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 126 | 5240.6 | 710.0 |
| mostly unexplained (+timer tick_nohz_handler) | 26 | 867.0 | 157.0 |
| mostly unexplained (+softirq TIMER) | 7 | 167.3 | 29.9 |
| mostly unexplained (+softirq RCU) | 5 | 109.0 | 22.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 710.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 177.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 162.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 157.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 5 | 143.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 126.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 110.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 104.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 103.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 103.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120110 |
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-noquickack/pp-uring-spin-10000 (median 47.93)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 814.10 | 766.17 | 0.00 |  | 62.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 703.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 753.77 | 705.84 | 0.00 |  | 61.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 644.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 474.03 | 426.10 | 0.00 |  | 20.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 405.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 434.81 | 386.88 | 341.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 45.21 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 431.38 | 383.44 | 0.00 |  | 20.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 363.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 403.04 | 355.11 | 0.00 |  | 41.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 313.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 382.75 | 334.82 | 0.00 |  | 0.00 |  | 334.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 382.53 | 334.60 | 0.00 |  | 0.00 |  | 334.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 371.18 | 323.25 | 0.00 |  | 0.00 |  | 323.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 365.48 | 317.55 | 0.00 |  | 20.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 297.29 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-nogro/http-recv-busy-10000, loop 0 on CPU 2

1929141264 iterations, longest 130.6 µs; over 5 / 20 / 100 µs: 149454 / 216 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 141 | 4729.7 | 130.6 |
| mostly unexplained (+softirq RCU) | 29 | 700.5 | 40.0 |
| mostly unexplained (+timer tick_nohz_handler) | 16 | 579.7 | 76.2 |
| mostly unexplained (+softirq NET_RX) | 17 | 557.8 | 80.4 |
| mostly unexplained (+ipi_send_cpu) | 6 | 143.5 | 34.0 |
| mostly unexplained (+softirq TIMER) | 5 | 123.5 | 34.3 |
| loop work | 2 | 57.6 | 34.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 130.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 112.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 94.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 80.4 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 7 |
| 5 | 76.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 6 | 74.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 71.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 8 | 70.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 69.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 66.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120054 |
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-nogro/http-recv-busy-10000 (median 48.13)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1052.02 | 1003.89 | 0.00 |  | 20.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 983.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 845.59 | 797.46 | 0.00 |  | 0.00 |  | 797.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 773.14 | 725.01 | 0.00 |  | 0.00 |  | 725.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 663.53 | 615.40 | 0.00 |  | 0.00 |  | 615.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 345.10 | 296.97 | 0.00 |  | 0.00 |  | 296.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 282.77 | 234.64 | 0.00 |  | 228.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 6.55 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 259.35 | 211.21 | 0.00 |  | 0.00 |  | 211.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 249.73 | 201.60 | 0.00 |  | 0.00 |  | 201.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 244.06 | 195.93 | 206.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 218.78 | 170.65 | 0.00 |  | 0.00 |  | 170.65 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-nogro/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=661672 clocksource=tsc

### Loop counters (r1-nogro/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001617 | 968166 | 3.10 | 62861568 | 20.94 | 1047693 | 1047693 | 98.5 | 83.5 | true | 0 | 0 | 765.0 | 316552 / 5508 / 5 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8213 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672362208 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (251627 of 2900067 in 1-msg reads) | 251627 | 4.62 | 3.26 | 6.99 | 22.57 | 27.72 | 42.88 | 69.63 |

#### Stalls: r1-nogro/md-recv-busy, loop 0 on CPU 2

62861568 iterations, longest 765.0 µs; over 5 / 20 / 100 µs: 316552 / 5508 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 3865 | 84785.0 | 385.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1444 | 31628.4 | 425.1 |
| mostly unexplained (+timer tick_nohz_handler) | 161 | 4692.0 | 220.7 |
| mostly unexplained (+softirq NET_RX) | 4 | 844.1 | 765.0 |
| softirq NET_RX | 22 | 512.4 | 45.3 |
| mostly unexplained (+softirq RCU) | 4 | 131.3 | 51.1 |
| mostly unexplained (+softirq TIMER) | 3 | 87.9 | 35.4 |
| mostly unexplained (+ipi_send_cpu) | 3 | 75.2 | 28.1 |
| timer tick_nohz_handler | 2 | 55.0 | 28.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 765.0 | 5 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1, x86 platform ipi (IPI/vector) 1 |
| 2 | 425.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 385.7 | 1550 | loop work (1550 items) |  |
| 4 | 220.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 17 |
| 5 | 113.5 | 9 | loop work (9 items) |  |
| 6 | 92.7 | 39 | loop work (39 items) |  |
| 7 | 83.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 70.8 | 4 | loop work (4 items) |  |
| 9 | 64.7 | 10 | loop work (10 items) |  |
| 10 | 57.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60036 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## r1-nogro/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-nogro/noise, loop 0 on CPU 2

12444626944 iterations, longest 943.9 µs; over 5 / 20 / 100 µs: 1977 / 146 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 791 | 12020.6 | 147.0 |
| mostly unexplained (+timer tick_nohz_handler) | 1073 | 8544.9 | 943.9 |
| mostly unexplained (+softirq TIMER) | 110 | 624.3 | 6.8 |
| timer tick_nohz_handler | 2 | 47.8 | 26.0 |
| softirq TIMER | 1 | 9.7 | 9.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 943.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 2 | 804.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 652.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 15 |
| 4 | 147.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 134.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 6 | 134.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 110.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 98.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 75.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 73.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120024 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-nogro/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1254104 | 1254104 | 1.00 | 60493721 | 48.24 | 1008229 | 987327 | 97.9 | 85.5 | true | 0 | 0 | 653.2 | 138043 / 106 / 3 | 0 | 0 |

#### Stalls: r1-nogro/pp-recv-busy-0, loop 0 on CPU 2

59239616 iterations, longest 653.2 µs; over 5 / 20 / 100 µs: 138043 / 106 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 62 | 2619.5 | 472.0 |
| mostly unexplained (+timer tick_nohz_handler) | 15 | 1022.9 | 653.2 |
| mostly unexplained (+softirq NET_RX) | 21 | 780.7 | 67.0 |
| mostly unexplained (+softirq RCU) | 3 | 99.6 | 51.4 |
| mostly unexplained (+ipi_send_cpu) | 2 | 46.7 | 25.9 |
| mostly unexplained (+softirq TIMER) | 2 | 44.6 | 23.7 |
| softirq NET_RX | 1 | 27.7 | 27.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 653.2 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5, softirq NET_RX 1, softirq TIMER 1 |
| 2 | 472.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 166.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 67.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 64.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 63.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 62.4 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 8 | 56.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 56.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 55.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2, timer tick_nohz_handler 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60155 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-nogro/pp-recv-busy-0 (median 45.14)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1006.44 | 961.30 | 0.00 |  | 447.36 | unexplained: no kernel event (hypervisor or in-kernel time) | 513.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 666.76 | 621.61 | 652.77 | mostly unexplained (+timer tick_nohz_handler) | 20.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 480.21 | 435.07 | 471.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 438.25 | 393.11 | 0.00 |  | 162.76 | unexplained: no kernel event (hypervisor or in-kernel time) | 230.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 364.72 | 319.57 | 0.00 |  | 363.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 341.60 | 296.45 | 0.00 |  | 301.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 324.91 | 279.76 | 0.00 |  | 21.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 258.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 299.28 | 254.13 | 0.00 |  | 21.83 | unexplained: no kernel event (hypervisor or in-kernel time) | 232.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 267.97 | 222.82 | 0.00 |  | 20.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 202.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 262.56 | 217.42 | 0.00 |  | 20.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 196.71 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-nogro/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194438 | 1194438 | 1.00 | 128220231 | 107.35 | 1068502 | 1058548 | 99.1 | 83.9 | true | 0 | 0 | 1063.4 | 174785 / 188 / 9 | 0 | 0 |

#### Stalls: r1-nogro/pp-recv-busy-10000, loop 0 on CPU 2

127025792 iterations, longest 1063.4 µs; over 5 / 20 / 100 µs: 174785 / 188 / 9

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 141 | 5482.9 | 383.5 |
| mostly unexplained (+timer tick_nohz_handler) | 18 | 2865.0 | 1063.4 |
| mostly unexplained (+softirq NET_RX) | 24 | 1008.7 | 206.9 |
| mostly unexplained (+ipi_send_cpu) | 4 | 88.8 | 24.4 |
| mostly unexplained (+softirq RCU) | 1 | 26.8 | 26.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1063.4 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4, softirq NET_RX 1 |
| 2 | 865.5 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 533.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4, softirq RCU 1 |
| 4 | 383.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 206.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 136.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 119.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 119.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 117.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 98.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120112 |
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-nogro/pp-recv-busy-10000 (median 46.97)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 2423.47 | 2376.50 | 0.00 |  | 274.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 2102.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1201.87 | 1154.90 | 1199.33 | mostly unexplained (+timer tick_nohz_handler) | 21.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 875.17 | 828.20 | 865.02 | mostly unexplained (+timer tick_nohz_handler) | 61.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 757.61 | 710.64 | 0.00 |  | 21.43 | unexplained: no kernel event (hypervisor or in-kernel time) | 689.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 597.24 | 550.27 | 0.00 |  | 20.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 530.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 555.92 | 508.95 | 0.00 |  | 0.00 |  | 508.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 453.30 | 406.32 | 0.00 |  | 0.00 |  | 406.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 432.29 | 385.32 | 0.00 |  | 41.47 | unexplained: no kernel event (hypervisor or in-kernel time) | 343.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 423.43 | 376.46 | 0.00 |  | 63.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 313.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 419.45 | 372.48 | 0.00 |  | 0.00 |  | 372.48 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-nogro/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1189804 | 1189804 | 1.00 | 3569413 | 3.00 | 59490 | 50356370 | 0.0 | 11.6 | false | 0 | 0 | 598.2 | 19166 / 104 / 3 | 0 | 0 |

#### Stalls: r1-nogro/pp-uring-spin-0, loop 0 on CPU 2

3021382208 iterations, longest 598.2 µs; over 5 / 20 / 100 µs: 19166 / 104 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 56 | 2231.5 | 598.2 |
| mostly unexplained (+timer tick_nohz_handler) | 38 | 1468.6 | 450.5 |
| mostly unexplained (+softirq TIMER) | 6 | 131.3 | 22.8 |
| mostly unexplained (+softirq RCU) | 4 | 96.9 | 32.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 598.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 450.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4, softirq TIMER 1 |
| 3 | 157.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 4 | 64.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 53.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 48.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 48.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 46.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 10 | 43.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60141 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-nogro/pp-uring-spin-0 (median 46.52)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 671.78 | 625.25 | 0.00 |  | 0.00 |  | 625.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 644.11 | 597.59 | 598.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 605.51 | 558.98 | 0.00 |  | 40.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 518.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 505.66 | 459.14 | 450.51 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 8.62 | client: mostly unexplained (+timer tick_nohz_handler) |
| 5 | 382.95 | 336.42 | 0.00 |  | 0.00 |  | 336.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 377.05 | 330.52 | 0.00 |  | 20.64 | unexplained: no kernel event (hypervisor or in-kernel time) | 309.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 373.29 | 326.76 | 0.00 |  | 0.00 |  | 326.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 372.99 | 326.46 | 0.00 |  | 0.00 |  | 326.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 364.07 | 317.55 | 0.00 |  | 50.79 | unexplained: no kernel event (hypervisor or in-kernel time) | 266.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 339.67 | 293.15 | 22.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 250.32 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-nogro/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199592 | 1199592 | 1.00 | 3598777 | 3.00 | 29990 | 38054327 | 0.0 | 5.8 | false | 1 | 0 | 653.0 | 10336 / 131 / 7 | 0 | 0 |

#### Stalls: r1-nogro/pp-uring-spin-10000, loop 0 on CPU 2

4566519232 iterations, longest 653.0 µs; over 5 / 20 / 100 µs: 10336 / 131 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 112 | 4393.2 | 427.7 |
| mostly unexplained (+timer tick_nohz_handler) | 15 | 1726.5 | 602.0 |
| preempted by swapper/2 | 1 | 653.0 | 653.0 |
| softirq TIMER | 1 | 58.0 | 58.0 |
| mostly unexplained (+softirq TIMER) | 1 | 44.8 | 44.8 |
| mostly unexplained (+softirq RCU) | 1 | 25.7 | 25.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 653.0 | 0 | preempted by swapper/2 | preempted by swapper/2 637, call function single (IPI/vector) 3, page fault 0 |
| 2 | 602.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5, softirq TIMER 1 |
| 3 | 559.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 4 | 427.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 396.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 156.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 103.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 86.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 67.0 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 23 |
| 10 | 66.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 88060 |
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 32 |
| IWI IRQ work interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-nogro/pp-uring-spin-10000 (median 47.64)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 750.03 | 702.38 | 0.00 |  | 52.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 649.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 705.89 | 658.25 | 652.95 | preempted by swapper/2 | 0.00 |  | 5.30 | client: preempted by swapper/2 |
| 3 | 615.95 | 568.30 | 559.23 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 9.07 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 612.41 | 564.77 | 0.00 |  | 20.54 | unexplained: no kernel event (hypervisor or in-kernel time) | 544.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 578.78 | 531.14 | 0.00 |  | 41.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 489.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 434.68 | 387.03 | 0.00 |  | 40.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 346.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 429.12 | 381.48 | 0.00 |  | 43.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 338.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 427.51 | 379.86 | 0.00 |  | 43.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 336.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 415.56 | 367.92 | 396.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 401.91 | 354.27 | 0.00 |  | 41.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 312.36 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-4/http-recv-busy-10000, loop 0 on CPU 2

1940135352 iterations, longest 748.0 µs; over 5 / 20 / 100 µs: 131781 / 167 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 140 | 5343.9 | 748.0 |
| preempted by swapper/2 | 1 | 746.7 | 746.7 |
| mostly unexplained (+softirq NET_RX) | 13 | 345.2 | 43.5 |
| mostly unexplained (+timer tick_nohz_handler) | 1 | 343.1 | 343.1 |
| mostly unexplained (+reschedule (IPI/vector)) | 6 | 129.2 | 22.8 |
| mostly unexplained (+softirq RCU) | 1 | 49.1 | 49.1 |
| loop work | 2 | 41.8 | 21.1 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 29.5 | 29.5 |
| timer tick_nohz_handler | 1 | 22.3 | 22.3 |
| mostly unexplained (+ipi_send_cpu) | 1 | 21.0 | 21.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 748.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 746.7 | 2 | preempted by swapper/2 | preempted by swapper/2 722, call function single (IPI/vector) 4, call function (IPI/vector) 1, page fault 0 |
| 3 | 343.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 4 | 97.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 62.1 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 59.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 58.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 57.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 52.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 51.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 53554 |
| CAL Function call interrupts | 1017 |
| RES Rescheduling interrupts | 64 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-base-4/http-recv-busy-10000 (median 47.89)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 869.32 | 821.43 | 747.36 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 74.07 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 787.20 | 739.31 | 745.41 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 3 | 641.33 | 593.43 | 0.00 |  | 20.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 573.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 419.67 | 371.77 | 0.00 |  | 0.00 |  | 371.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 380.86 | 332.96 | 0.00 |  | 41.22 | unexplained: no kernel event (hypervisor or in-kernel time) | 291.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 379.32 | 331.43 | 0.00 |  | 0.00 |  | 331.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 373.52 | 325.63 | 0.00 |  | 0.00 |  | 325.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 358.72 | 310.83 | 0.00 |  | 0.00 |  | 310.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 334.74 | 286.85 | 0.00 |  | 0.00 |  | 286.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 333.43 | 285.54 | 0.00 |  | 0.00 |  | 285.54 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-4/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=652399 clocksource=tsc

### Loop counters (r1-base-4/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001550 | 922260 | 3.25 | 62696768 | 20.89 | 1044946 | 1044946 | 98.5 | 83.5 | true | 0 | 0 | 420.0 | 357365 / 5720 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8212 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672347200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (217010 of 2900000 in 1-msg reads) | 217010 | 4.83 | 3.29 | 7.85 | 23.44 | 26.44 | 38.28 | 46.33 |

#### Stalls: r1-base-4/md-recv-busy, loop 0 on CPU 2

62696768 iterations, longest 420.0 µs; over 5 / 20 / 100 µs: 357365 / 5720 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4376 | 95048.3 | 420.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1281 | 28359.5 | 250.5 |
| mostly unexplained (+timer tick_nohz_handler) | 37 | 864.7 | 40.4 |
| softirq NET_RX | 22 | 512.3 | 29.4 |
| mostly unexplained (+softirq RCU) | 2 | 48.9 | 24.6 |
| mostly unexplained (+ipi_send_cpu) | 1 | 23.9 | 23.9 |
| mostly unexplained (+softirq NET_RX) | 1 | 20.7 | 20.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 420.0 | 1550 | loop work (1550 items) | timer tick_nohz_handler 2, softirq RCU 1 |
| 2 | 250.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 128.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 85.6 | 16 | loop work (16 items) |  |
| 5 | 73.8 | 8 | loop work (8 items) |  |
| 6 | 70.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 68.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 63.3 | 7 | loop work (7 items) | softirq NET_RX 1 |
| 9 | 51.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 50.3 | 15 | loop work (15 items) | softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60035 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

## r1-base-4/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-4/noise, loop 0 on CPU 2

12311351296 iterations, longest 822.4 µs; over 5 / 20 / 100 µs: 1772 / 101 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 1283 | 7907.0 | 822.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 389 | 6854.2 | 771.7 |
| mostly unexplained (+softirq TIMER) | 95 | 539.1 | 7.0 |
| timer tick_nohz_handler | 4 | 72.1 | 38.5 |
| softirq TIMER | 1 | 10.4 | 10.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 822.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 2 | 771.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 121.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 116.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 113.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 104.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 93.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 66.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 63.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 59.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120024 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-base-4/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1204053 | 1204053 | 1.00 | 306648043 | 254.68 | 5110801 | 5070666 | 0.0 | 37.5 | false | 0 | 0 | 73.8 | 11256 / 93 / 0 | 0 | 0 |

#### Stalls: r1-base-4/pp-epoll-spin-0, loop 0 on CPU 2

304239936 iterations, longest 73.8 µs; over 5 / 20 / 100 µs: 11256 / 93 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 71 | 2208.5 | 73.8 |
| mostly unexplained (+timer tick_nohz_handler) | 14 | 359.7 | 55.3 |
| mostly unexplained (+softirq TIMER) | 6 | 159.2 | 41.6 |
| mostly unexplained (+softirq RCU) | 2 | 45.7 | 23.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 73.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 64.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 63.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 62.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 55.3 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 6 | 46.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 45.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 43.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 41.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 41.6 | 0 | mostly unexplained (+softirq TIMER) | softirq TIMER 1, timer tick_nohz_handler 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60144 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-4/pp-epoll-spin-0 (median 47.00)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 710.46 | 663.46 | 0.00 |  | 0.00 |  | 663.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 553.28 | 506.28 | 0.00 |  | 104.62 | unexplained: no kernel event (hypervisor or in-kernel time) | 401.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 486.75 | 439.74 | 0.00 |  | 0.00 |  | 439.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 428.34 | 381.34 | 0.00 |  | 0.00 |  | 381.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 388.75 | 341.75 | 0.00 |  | 20.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 321.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 387.60 | 340.60 | 0.00 |  | 0.00 |  | 340.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 374.75 | 327.75 | 0.00 |  | 0.00 |  | 327.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 373.18 | 326.18 | 0.00 |  | 0.00 |  | 326.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 372.61 | 325.61 | 0.00 |  | 0.00 |  | 325.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 348.14 | 301.14 | 0.00 |  | 0.00 |  | 301.14 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-4/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198644 | 1198644 | 1.00 | 611563368 | 510.21 | 5096361 | 5076384 | 0.0 | 32.3 | false | 0 | 0 | 455.7 | 17294 / 158 / 2 | 0 | 0 |

#### Stalls: r1-base-4/pp-epoll-spin-10000, loop 0 on CPU 2

609166080 iterations, longest 455.7 µs; over 5 / 20 / 100 µs: 17294 / 158 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 129 | 4046.0 | 65.5 |
| mostly unexplained (+timer tick_nohz_handler) | 17 | 966.5 | 455.7 |
| mostly unexplained (+softirq TIMER) | 8 | 174.2 | 25.4 |
| mostly unexplained (+softirq RCU) | 4 | 91.9 | 24.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 455.7 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 2 | 120.3 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 3 | 65.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 64.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 57.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 52.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 51.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 48.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 48.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 47.2 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120118 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-4/pp-epoll-spin-10000 (median 47.08)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 635.83 | 588.75 | 0.00 |  | 21.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 567.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 480.16 | 433.08 | 455.34 | mostly unexplained (+timer tick_nohz_handler) | 20.54 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 450.82 | 403.74 | 0.00 |  | 0.00 |  | 403.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 441.55 | 394.48 | 0.00 |  | 0.00 |  | 394.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 432.75 | 385.67 | 0.00 |  | 0.00 |  | 385.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 414.63 | 367.55 | 0.00 |  | 62.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 305.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 392.05 | 344.97 | 0.00 |  | 61.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 283.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 391.71 | 344.63 | 0.00 |  | 0.00 |  | 344.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 375.80 | 328.72 | 0.00 |  | 21.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 307.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 353.28 | 306.20 | 0.00 |  | 20.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 286.12 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-4/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1262881 | 1262881 | 1.00 | 60129954 | 47.61 | 1002166 | 981118 | 97.9 | 85.7 | true | 0 | 0 | 785.3 | 155032 / 135 / 3 | 0 | 0 |

#### Stalls: r1-base-4/pp-recv-busy-0, loop 0 on CPU 2

58867072 iterations, longest 785.3 µs; over 5 / 20 / 100 µs: 155032 / 135 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 77 | 3547.3 | 785.3 |
| mostly unexplained (+softirq NET_RX) | 34 | 1283.4 | 143.7 |
| mostly unexplained (+timer tick_nohz_handler) | 12 | 434.3 | 78.7 |
| mostly unexplained (+softirq TIMER) | 5 | 183.2 | 66.5 |
| mostly unexplained (+ipi_send_cpu) | 7 | 166.6 | 27.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 785.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 143.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 3 | 132.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 83.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 78.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 78.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 7 | 74.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 71.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 70.5 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 10 | 70.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60145 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-4/pp-recv-busy-0 (median 44.80)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 792.24 | 747.43 | 785.30 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 521.34 | 476.53 | 0.00 |  | 80.89 | unexplained: no kernel event (hypervisor or in-kernel time) | 395.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 378.46 | 333.65 | 0.00 |  | 20.06 | unexplained: no kernel event (hypervisor or in-kernel time) | 313.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 354.34 | 309.53 | 0.00 |  | 22.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 287.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 329.18 | 284.38 | 0.00 |  | 80.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 203.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 286.74 | 241.94 | 0.00 |  | 0.00 |  | 241.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 270.36 | 225.56 | 0.00 |  | 0.00 |  | 225.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 186.74 | 141.94 | 0.00 |  | 42.77 | unexplained: no kernel event (hypervisor or in-kernel time) | 99.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 183.05 | 138.25 | 132.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 5.88 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 181.70 | 136.90 | 143.17 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |

### Loop counters (r1-base-4/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194572 | 1194572 | 1.00 | 128192972 | 107.31 | 1068275 | 1058320 | 99.1 | 84.0 | true | 1 | 0 | 636.8 | 159422 / 182 / 5 | 0 | 0 |

#### Stalls: r1-base-4/pp-recv-busy-10000, loop 0 on CPU 2

126998400 iterations, longest 636.8 µs; over 5 / 20 / 100 µs: 159422 / 182 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 137 | 5021.2 | 394.8 |
| mostly unexplained (+timer tick_nohz_handler) | 12 | 918.3 | 571.8 |
| mostly unexplained (+softirq NET_RX) | 18 | 703.6 | 110.6 |
| preempted by swapper/2 | 1 | 636.8 | 636.8 |
| mostly unexplained (+reschedule (IPI/vector)) | 6 | 130.9 | 22.3 |
| mostly unexplained (+ipi_send_cpu) | 4 | 95.8 | 29.0 |
| mostly unexplained (+softirq TIMER) | 3 | 80.1 | 38.1 |
| softirq NET_RX | 1 | 21.6 | 21.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 636.8 | 1 | preempted by swapper/2 | preempted by swapper/2 610, call function single (IPI/vector) 3, softirq NET_RX 1, page fault 0 |
| 2 | 571.8 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 394.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 253.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 110.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 89.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 80.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 72.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 67.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 66.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 79885 |
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 40 |
| IWI IRQ work interrupts | 1 |

#### Slowest round trips: r1-base-4/pp-recv-busy-10000 (median 46.89)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 675.08 | 628.19 | 636.28 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 2 | 605.53 | 558.64 | 571.24 | mostly unexplained (+timer tick_nohz_handler) | 40.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 493.54 | 446.65 | 0.00 |  | 51.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 395.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 456.81 | 409.92 | 0.00 |  | 277.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 132.01 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 430.31 | 383.42 | 0.00 |  | 0.00 |  | 383.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 410.58 | 363.69 | 0.00 |  | 20.87 | unexplained: no kernel event (hypervisor or in-kernel time) | 342.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 360.39 | 313.50 | 0.00 |  | 20.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 293.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 345.08 | 298.19 | 0.00 |  | 0.00 |  | 298.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 330.83 | 283.94 | 0.00 |  | 0.00 |  | 283.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 328.96 | 282.08 | 311.77 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-4/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1189424 | 1189424 | 1.00 | 3568273 | 3.00 | 59471 | 50023506 | 0.0 | 11.6 | false | 0 | 0 | 126.0 | 20015 / 135 / 2 | 0 | 0 |

#### Stalls: r1-base-4/pp-uring-spin-0, loop 0 on CPU 2

3001410368 iterations, longest 126.0 µs; over 5 / 20 / 100 µs: 20015 / 135 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 90 | 2874.5 | 126.0 |
| mostly unexplained (+timer tick_nohz_handler) | 38 | 842.7 | 38.9 |
| mostly unexplained (+softirq TIMER) | 5 | 152.4 | 63.4 |
| mostly unexplained (+softirq RCU) | 2 | 43.2 | 22.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 126.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 100.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 81.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 63.4 | 0 | mostly unexplained (+softirq TIMER) | softirq TIMER 1, timer tick_nohz_handler 1 |
| 5 | 61.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 59.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 52.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 47.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 46.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 45.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60140 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-4/pp-uring-spin-0 (median 46.63)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1876.16 | 1829.53 | 0.00 |  | 142.05 | unexplained: no kernel event (hypervisor or in-kernel time) | 1687.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 596.74 | 550.11 | 0.00 |  | 0.00 |  | 550.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 470.49 | 423.86 | 0.00 |  | 20.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 403.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 450.68 | 404.05 | 0.00 |  | 21.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 382.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 448.28 | 401.65 | 0.00 |  | 0.00 |  | 401.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 432.41 | 385.78 | 0.00 |  | 75.89 | unexplained: no kernel event (hypervisor or in-kernel time) | 309.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 404.50 | 357.87 | 0.00 |  | 0.00 |  | 357.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 395.65 | 349.02 | 0.00 |  | 0.00 |  | 349.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 386.83 | 340.20 | 0.00 |  | 0.00 |  | 340.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 385.06 | 338.43 | 0.00 |  | 0.00 |  | 338.43 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-4/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199582 | 1199582 | 1.00 | 3598747 | 3.00 | 29990 | 39121109 | 0.0 | 5.8 | false | 1 | 0 | 679.7 | 11139 / 116 / 4 | 0 | 0 |

#### Stalls: r1-base-4/pp-uring-spin-10000, loop 0 on CPU 2

4694533056 iterations, longest 679.7 µs; over 5 / 20 / 100 µs: 11139 / 116 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 106 | 3505.3 | 144.7 |
| preempted by swapper/2 | 1 | 679.7 | 679.7 |
| mostly unexplained (+timer tick_nohz_handler) | 8 | 670.4 | 445.7 |
| mostly unexplained (+softirq TIMER) | 1 | 20.4 | 20.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 679.7 | 0 | preempted by swapper/2 | preempted by swapper/2 653, call function single (IPI/vector) 3, softirq TIMER 1, timer tick_nohz_handler 1 |
| 2 | 445.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 3 | 144.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 105.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 79.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 56.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 54.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 53.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 50.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 49.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 95227 |
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 25 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-4/pp-uring-spin-10000 (median 47.72)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 868.63 | 820.92 | 0.00 |  | 43.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 777.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 723.94 | 676.22 | 679.71 | preempted by swapper/2 | 20.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 3 | 489.25 | 441.54 | 445.74 | mostly unexplained (+timer tick_nohz_handler) | 20.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 471.22 | 423.50 | 0.00 |  | 40.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 383.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 429.47 | 381.75 | 0.00 |  | 2.73 | unexplained: no kernel event (hypervisor or in-kernel time) | 379.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 428.33 | 380.62 | 0.00 |  | 21.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 359.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 414.47 | 366.76 | 0.00 |  | 20.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 346.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 406.43 | 358.71 | 0.00 |  | 0.00 |  | 358.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 392.63 | 344.91 | 0.00 |  | 21.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 323.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 380.76 | 333.04 | 0.00 |  | 0.00 |  | 333.04 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-4/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1259139 | 1259139 | 1.00 | 1 | 0.00 | 0 | 48659461 | 0.0 | 0.0 | true | 0 | 0 | 251.1 | 261 / 55 / 1 | 0 | 0 |

#### Stalls: r1-base-4/pp-uring-sqpoll-0, loop 0 on CPU 2

2919567680 iterations, longest 251.1 µs; over 5 / 20 / 100 µs: 261 / 55 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 55 | 1960.9 | 251.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 251.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 88.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 78.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 43.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 42.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 41.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 39.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 38.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 38.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1020 |
| RES Rescheduling interrupts | 59 |
| IWI IRQ work interrupts | 7 |
| HVS Hyper-V stimer0 interrupts | 2 |

#### Slowest round trips: r1-base-4/pp-uring-sqpoll-0 (median 44.79)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 606.70 | 561.91 | 0.00 |  | 42.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 519.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 332.77 | 287.98 | 0.00 |  | 383.12 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 283.62 | 238.82 | 0.00 |  | 0.00 |  | 238.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 280.13 | 235.34 | 0.00 |  | 0.00 |  | 235.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 278.53 | 233.74 | 0.00 |  | 0.00 |  | 233.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 260.04 | 215.25 | 250.76 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 241.04 | 196.25 | 0.00 |  | 47.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 148.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 194.35 | 149.56 | 0.00 |  | 0.00 |  | 149.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 182.45 | 137.66 | 0.00 |  | 0.00 |  | 137.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 169.76 | 124.97 | 0.00 |  | 0.00 |  | 124.97 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-4/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199606 | 1199606 | 1.00 | 1 | 0.00 | 0 | 35380267 | 0.0 | 0.0 | true | 0 | 0 | 422.1 | 620 / 130 / 6 | 0 | 0 |

#### Stalls: r1-base-4/pp-uring-sqpoll-10000, loop 0 on CPU 2

4245632064 iterations, longest 422.1 µs; over 5 / 20 / 100 µs: 620 / 130 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 130 | 5505.6 | 422.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 422.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 404.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 331.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 108.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 108.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 107.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 92.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 72.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 70.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 67.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1023 |
| RES Rescheduling interrupts | 117 |
| IWI IRQ work interrupts | 5 |
| HVS Hyper-V stimer0 interrupts | 2 |

#### Slowest round trips: r1-base-4/pp-uring-sqpoll-10000 (median 47.63)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 454.43 | 406.79 | 0.00 |  | 0.00 |  | 406.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 427.63 | 380.00 | 0.00 |  | 0.00 |  | 380.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 423.09 | 375.45 | 421.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.94 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 416.90 | 369.26 | 0.00 |  | 0.00 |  | 369.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 397.29 | 349.66 | 0.00 |  | 40.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 309.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 394.20 | 346.57 | 0.00 |  | 20.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 326.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 376.74 | 329.11 | 0.00 |  | 0.00 |  | 329.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 362.89 | 315.25 | 331.36 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 360.97 | 313.34 | 0.00 |  | 20.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 292.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 297.14 | 249.50 | 0.00 |  | 20.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 229.09 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-nocoalesce/http-recv-busy-10000, loop 0 on CPU 2

1920482357 iterations, longest 964.6 µs; over 5 / 20 / 100 µs: 191133 / 166 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 142 | 5658.8 | 534.1 |
| mostly unexplained (+softirq NET_RX) | 12 | 2012.0 | 819.3 |
| mostly unexplained (+timer tick_nohz_handler) | 2 | 1005.9 | 964.6 |
| preempted by swapper/2 | 1 | 588.9 | 588.9 |
| mostly unexplained (+reschedule (IPI/vector)) | 3 | 68.5 | 26.3 |
| mostly unexplained (+ipi_send_cpu) | 3 | 65.4 | 24.9 |
| mostly unexplained (+softirq RCU) | 1 | 50.4 | 50.4 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 34.3 | 34.3 |
| loop work | 1 | 30.5 | 30.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 964.6 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 2 | 819.3 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 3 | 805.2 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 4 | 588.9 | 2 | preempted by swapper/2 | preempted by swapper/2 568, call function single (IPI/vector) 3, page fault 0 |
| 5 | 534.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 143.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 114.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 100.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 94.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 74.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 46202 |
| CAL Function call interrupts | 1017 |
| RES Rescheduling interrupts | 72 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-nocoalesce/http-recv-busy-10000 (median 48.35)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1086.28 | 1037.93 | 1077.22 | mostly unexplained (+timer tick_nohz_handler) | 20.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 870.58 | 822.23 | 818.29 | mostly unexplained (+softirq NET_RX) | 20.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 3 | 847.77 | 799.43 | 803.18 | mostly unexplained (+softirq NET_RX) | 41.12 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 4 | 654.85 | 606.50 | 587.98 | preempted by swapper/2 | 0.00 |  | 18.52 | client: preempted by swapper/2 |
| 5 | 588.92 | 540.58 | 0.00 |  | 0.00 |  | 540.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 522.16 | 473.81 | 0.00 |  | 0.00 |  | 473.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 474.48 | 426.13 | 0.00 |  | 0.00 |  | 426.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 411.95 | 363.60 | 0.00 |  | 0.00 |  | 363.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 408.17 | 359.82 | 0.00 |  | 0.00 |  | 359.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 404.92 | 356.57 | 0.00 |  | 0.00 |  | 356.57 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-nocoalesce/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=665481 clocksource=tsc

### Loop counters (r1-nocoalesce/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001610 | 949558 | 3.16 | 63096064 | 21.02 | 1051601 | 1051601 | 98.5 | 83.5 | true | 0 | 0 | 898.1 | 363726 / 278 / 5 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8216 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672360640 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (229109 of 2900060 in 1-msg reads) | 229109 | 3.76 | 3.38 | 5.01 | 8.73 | 18.14 | 23.43 | 38.61 |

#### Stalls: r1-nocoalesce/md-recv-busy, loop 0 on CPU 2

63096064 iterations, longest 898.1 µs; over 5 / 20 / 100 µs: 363726 / 278 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 148 | 4753.7 | 898.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 85 | 2920.3 | 151.6 |
| softirq NET_RX | 38 | 882.9 | 54.3 |
| mostly unexplained (+timer tick_nohz_handler) | 3 | 584.5 | 528.3 |
| mostly unexplained (+softirq RCU) | 1 | 97.8 | 97.8 |
| mostly unexplained (+ipi_send_cpu) | 2 | 42.2 | 21.5 |
| mostly unexplained (+softirq NET_RX) | 1 | 29.6 | 29.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 898.1 | 50 | loop work (50 items) | timer tick_nohz_handler 4 |
| 2 | 528.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 385.3 | 1550 | loop work (1550 items) |  |
| 4 | 151.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 150.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 97.8 | 0 | mostly unexplained (+softirq RCU) | softirq RCU 1 |
| 7 | 85.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 84.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 70.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 61.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60034 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## r1-nocoalesce/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-nocoalesce/noise, loop 0 on CPU 2

12357480448 iterations, longest 737.9 µs; over 5 / 20 / 100 µs: 1844 / 121 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 442 | 7555.3 | 737.9 |
| mostly unexplained (+timer tick_nohz_handler) | 1287 | 7111.8 | 90.9 |
| mostly unexplained (+softirq TIMER) | 112 | 637.6 | 7.5 |
| timer tick_nohz_handler | 2 | 23.4 | 18.2 |
| softirq TIMER | 1 | 8.6 | 8.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 737.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 115.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 90.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 14 |
| 4 | 70.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 65.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 61.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 54.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 8 | 48.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 48.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 46.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120025 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-nocoalesce/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1254504 | 1254504 | 1.00 | 60032105 | 47.85 | 1000535 | 979627 | 97.9 | 85.5 | true | 0 | 0 | 593.6 | 181224 / 122 / 4 | 0 | 0 |

#### Stalls: r1-nocoalesce/pp-recv-busy-0, loop 0 on CPU 2

58777600 iterations, longest 593.6 µs; over 5 / 20 / 100 µs: 181224 / 122 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 72 | 3407.5 | 593.6 |
| mostly unexplained (+timer tick_nohz_handler) | 23 | 757.2 | 203.9 |
| mostly unexplained (+softirq NET_RX) | 15 | 731.9 | 241.7 |
| mostly unexplained (+softirq TIMER) | 6 | 130.6 | 25.8 |
| mostly unexplained (+ipi_send_cpu) | 5 | 125.6 | 33.3 |
| mostly unexplained (+softirq RCU) | 1 | 24.1 | 24.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 593.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 256.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 241.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 6, timer tick_nohz_handler 4, softirq TIMER 1 |
| 4 | 203.9 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5 |
| 5 | 96.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 84.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 71.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 70.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 65.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 65.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60144 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-nocoalesce/pp-recv-busy-0 (median 44.99)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 861.61 | 816.62 | 850.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 487.75 | 442.76 | 0.00 |  | 169.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 273.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 402.50 | 357.52 | 0.00 |  | 418.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 385.56 | 340.57 | 0.00 |  | 52.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 288.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 363.94 | 318.96 | 0.00 |  | 277.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 41.86 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 312.41 | 267.42 | 0.00 |  | 61.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 205.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 286.16 | 241.17 | 0.00 |  | 63.56 | unexplained: no kernel event (hypervisor or in-kernel time) | 177.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 284.11 | 239.12 | 282.65 | mostly unexplained (+softirq NET_RX) | 40.55 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 9 | 215.23 | 170.25 | 203.71 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 10 | 192.97 | 147.99 | 25.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 102.81 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-nocoalesce/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194527 | 1194527 | 1.00 | 128198944 | 107.32 | 1068325 | 1058370 | 99.1 | 84.1 | true | 1 | 0 | 852.5 | 155181 / 181 / 6 | 0 | 0 |

#### Stalls: r1-nocoalesce/pp-recv-busy-10000, loop 0 on CPU 2

127004416 iterations, longest 852.5 µs; over 5 / 20 / 100 µs: 155181 / 181 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 142 | 5601.5 | 852.5 |
| mostly unexplained (+softirq NET_RX) | 21 | 1148.5 | 382.3 |
| mostly unexplained (+timer tick_nohz_handler) | 8 | 840.9 | 605.5 |
| preempted by swapper/2 | 1 | 512.8 | 512.8 |
| mostly unexplained (+softirq TIMER) | 4 | 117.5 | 37.3 |
| mostly unexplained (+softirq RCU) | 4 | 91.4 | 27.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 39.4 | 39.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 852.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 605.5 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4, softirq TIMER 1 |
| 3 | 512.8 | 1 | preempted by swapper/2 | preempted by swapper/2 495, call function single (IPI/vector) 2, page fault 0 |
| 4 | 382.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 123.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 100.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 99.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 72.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 65.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 65.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 83758 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 36 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-nocoalesce/pp-recv-busy-10000 (median 46.68)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 2734.75 | 2688.07 | 0.00 |  | 231.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 2457.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 872.27 | 825.59 | 852.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 60.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 787.11 | 740.42 | 0.00 |  | 0.00 |  | 740.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 768.80 | 722.12 | 0.00 |  | 307.77 | unexplained: no kernel event (hypervisor or in-kernel time) | 414.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 627.75 | 581.07 | 604.71 | mostly unexplained (+timer tick_nohz_handler) | 62.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 6 | 555.54 | 508.86 | 512.44 | preempted by swapper/2 | 81.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 7 | 442.06 | 395.37 | 0.00 |  | 268.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 126.45 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 439.25 | 392.57 | 0.00 |  | 20.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 372.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 438.75 | 392.06 | 0.00 |  | 20.93 | unexplained: no kernel event (hypervisor or in-kernel time) | 371.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 435.45 | 388.77 | 0.00 |  | 0.00 |  | 388.77 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-nocoalesce/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1198208 | 1198208 | 1.00 | 3594625 | 3.00 | 59910 | 49931973 | 0.0 | 12.0 | false | 0 | 0 | 541.5 | 13675 / 69 / 3 | 0 | 0 |

#### Stalls: r1-nocoalesce/pp-uring-spin-0, loop 0 on CPU 2

2995918400 iterations, longest 541.5 µs; over 5 / 20 / 100 µs: 13675 / 69 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 60 | 2474.5 | 464.6 |
| mostly unexplained (+timer tick_nohz_handler) | 6 | 665.5 | 541.5 |
| mostly unexplained (+softirq TIMER) | 2 | 55.9 | 35.1 |
| timer tick_nohz_handler | 1 | 21.7 | 21.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 541.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4, softirq TIMER 1 |
| 2 | 464.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 169.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 75.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 55.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 54.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 48.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 47.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 45.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 43.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60142 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-nocoalesce/pp-uring-spin-0 (median 47.63)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1081.10 | 1033.47 | 0.00 |  | 0.00 |  | 1033.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1023.75 | 976.12 | 0.00 |  | 41.12 | unexplained: no kernel event (hypervisor or in-kernel time) | 935.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 780.84 | 733.21 | 0.00 |  | 20.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 712.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 698.26 | 650.63 | 0.00 |  | 21.05 | unexplained: no kernel event (hypervisor or in-kernel time) | 629.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 670.40 | 622.77 | 0.00 |  | 0.00 |  | 622.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 633.93 | 586.29 | 0.00 |  | 0.00 |  | 586.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 591.12 | 543.49 | 541.55 | mostly unexplained (+timer tick_nohz_handler) | 20.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 8 | 518.05 | 470.42 | 464.57 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 5.84 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 480.33 | 432.69 | 0.00 |  | 42.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 389.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 430.08 | 382.45 | 0.00 |  | 103.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 279.45 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-nocoalesce/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199539 | 1199539 | 1.00 | 3598618 | 3.00 | 29988 | 39100937 | 0.0 | 6.1 | false | 0 | 0 | 611.9 | 15480 / 133 / 8 | 0 | 0 |

#### Stalls: r1-nocoalesce/pp-uring-spin-10000, loop 0 on CPU 2

4692112448 iterations, longest 611.9 µs; over 5 / 20 / 100 µs: 15480 / 133 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 120 | 5090.4 | 492.3 |
| mostly unexplained (+timer tick_nohz_handler) | 11 | 1464.6 | 611.9 |
| mostly unexplained (+softirq RCU) | 2 | 41.0 | 20.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 611.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 2 | 533.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 3 | 492.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 487.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 160.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 156.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 122.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 105.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5, softirq TIMER 1 |
| 9 | 75.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 62.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120109 |
| CAL Function call interrupts | 1019 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-nocoalesce/pp-uring-spin-10000 (median 49.35)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1087.59 | 1038.24 | 0.00 |  | 21.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 1016.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1045.19 | 995.84 | 0.00 |  | 21.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 974.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 812.05 | 762.69 | 0.00 |  | 0.00 |  | 762.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 648.57 | 599.21 | 611.94 | mostly unexplained (+timer tick_nohz_handler) | 67.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 5 | 642.71 | 593.36 | 0.00 |  | 20.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 573.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 617.95 | 568.60 | 0.00 |  | 0.00 |  | 568.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 578.97 | 529.62 | 533.16 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 8 | 514.55 | 465.20 | 492.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 497.19 | 447.84 | 487.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 464.43 | 415.07 | 0.00 |  | 0.00 |  | 415.07 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-irqlocal/http-recv-busy-10000, loop 0 on CPU 2

1789116987 iterations, longest 738.6 µs; over 5 / 20 / 100 µs: 1224105 / 295 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 277 | 10942.5 | 738.6 |
| softirq NET_RX | 8 | 201.4 | 31.7 |
| mostly unexplained (+irq mana_q4@pci:7870:00:00.0) | 3 | 111.4 | 48.2 |
| mostly unexplained (+softirq NET_RX) | 3 | 107.2 | 50.1 |
| loop work | 2 | 60.6 | 32.0 |
| mostly unexplained (+irq mana_q5@pci:7870:00:00.0) | 2 | 57.8 | 37.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 738.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 722.7 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 143.7 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 136.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 131.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 124.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 97.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 94.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 91.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 85.7 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 45 HV-PCI-MSIX-7870:00:00.0 5-edge mana_q4@pci:7870:00:00.0 | 2400164 |
| 46 HV-PCI-MSIX-7870:00:00.0 6-edge mana_q5@pci:7870:00:00.0 | 2398784 |
| HVS Hyper-V stimer0 interrupts | 120058 |
| CAL Function call interrupts | 1015 |
| 41 HV-PCI-MSIX-7870:00:00.0 1-edge mana_q0@pci:7870:00:00.0 | 313 |
| 47 HV-PCI-MSIX-7870:00:00.0 7-edge mana_q6@pci:7870:00:00.0 | 287 |
| 44 HV-PCI-MSIX-7870:00:00.0 4-edge mana_q3@pci:7870:00:00.0 | 236 |
| 48 HV-PCI-MSIX-7870:00:00.0 8-edge mana_q7@pci:7870:00:00.0 | 219 |
| 43 HV-PCI-MSIX-7870:00:00.0 3-edge mana_q2@pci:7870:00:00.0 | 215 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 204 |
| 40 HV-PCI-MSIX-7870:00:00.0 0-edge mana_hwc@pci:7870:00:00.0 | 123 |
| IWI IRQ work interrupts | 3 |

#### Slowest round trips: r1-irqlocal/http-recv-busy-10000 (median 48.74)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 821.56 | 772.81 | 0.00 |  | 20.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 751.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 774.51 | 725.77 | 737.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 754.11 | 705.36 | 0.00 |  | 0.00 |  | 705.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 734.23 | 685.48 | 722.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.43 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 501.58 | 452.84 | 0.00 |  | 0.00 |  | 452.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 446.13 | 397.39 | 0.00 |  | 0.00 |  | 397.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 419.90 | 371.15 | 0.00 |  | 0.00 |  | 371.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 402.50 | 353.76 | 0.00 |  | 0.00 |  | 353.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 392.60 | 343.85 | 0.00 |  | 0.00 |  | 343.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 383.36 | 334.61 | 0.00 |  | 0.00 |  | 334.61 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-irqlocal/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=390231 clocksource=tsc

### Loop counters (r1-irqlocal/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001800 | 711921 | 4.22 | 58938944 | 19.63 | 982316 | 982316 | 98.8 | 84.3 | true | 0 | 9 | 1342.3 | 662006 / 57720 / 15 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8212 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672403200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (33654 of 2900050 in 1-msg reads) | 33654 | 5.31 | 5.06 | 6.79 | 14.20 | 23.10 | 32.20 | 37.56 |

#### Stalls: r1-irqlocal/md-recv-busy, loop 0 on CPU 2

58938944 iterations, longest 1342.3 µs; over 5 / 20 / 100 µs: 662006 / 57720 / 15

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 52479 | 1397847.6 | 573.1 |
| softirq NET_RX | 5005 | 149636.4 | 1342.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 231 | 6650.2 | 136.9 |
| mostly unexplained (+softirq NET_RX) | 3 | 1149.0 | 1101.0 |
| mostly unexplained (+irq mana_q1@pci:7870:00:00.0) | 1 | 24.1 | 24.1 |
| mostly unexplained (+irq mana_q3@pci:7870:00:00.0) | 1 | 22.3 | 22.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1342.3 | 3 | softirq NET_RX | softirq NET_RX 1244, irq mana_q1@pci:7870:00:00.0 40, irq mana_q3@pci:7870:00:00.0 13 |
| 2 | 1101.0 | 50 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 4, irq mana_q1@pci:7870:00:00.0 2, irq mana_q3@pci:7870:00:00.0 1 |
| 3 | 573.1 | 1750 | loop work (1750 items) |  |
| 4 | 145.4 | 38 | loop work (38 items) |  |
| 5 | 139.6 | 42 | softirq NET_RX | softirq NET_RX 87, preempted by ksoftirqd/2 36, irq mana_q1@pci:7870:00:00.0 15, irq mana_q3@pci:7870:00:00.0 6 |
| 6 | 136.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 127.8 | 41 | loop work (41 items) |  |
| 8 | 113.1 | 16 | loop work (16 items) |  |
| 9 | 111.0 | 24 | loop work (24 items) |  |
| 10 | 110.4 | 30 | softirq NET_RX | softirq NET_RX 69, preempted by ksoftirqd/2 23, irq mana_q1@pci:7870:00:00.0 12, irq mana_q3@pci:7870:00:00.0 2 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 44 HV-PCI-MSIX-7870:00:00.0 4-edge mana_q3@pci:7870:00:00.0 | 955241 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 907513 |
| HVS Hyper-V stimer0 interrupts | 7526 |
| IWI IRQ work interrupts | 1688 |
| CAL Function call interrupts | 1015 |
| 43 HV-PCI-MSIX-7870:00:00.0 3-edge mana_q2@pci:7870:00:00.0 | 210 |
| 41 HV-PCI-MSIX-7870:00:00.0 1-edge mana_q0@pci:7870:00:00.0 | 133 |
| 48 HV-PCI-MSIX-7870:00:00.0 8-edge mana_q7@pci:7870:00:00.0 | 119 |
| 45 HV-PCI-MSIX-7870:00:00.0 5-edge mana_q4@pci:7870:00:00.0 | 91 |
| 47 HV-PCI-MSIX-7870:00:00.0 7-edge mana_q6@pci:7870:00:00.0 | 71 |
| 40 HV-PCI-MSIX-7870:00:00.0 0-edge mana_hwc@pci:7870:00:00.0 | 61 |
| 46 HV-PCI-MSIX-7870:00:00.0 6-edge mana_q5@pci:7870:00:00.0 | 56 |

## r1-irqlocal/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-irqlocal/noise, loop 0 on CPU 2

12335747072 iterations, longest 899.3 µs; over 5 / 20 / 100 µs: 5638 / 259 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq NET_RX | 2413 | 25763.0 | 129.8 |
| softirq TASKLET | 1554 | 15837.2 | 72.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 384 | 8788.2 | 801.6 |
| mostly unexplained (+timer tick_nohz_handler) | 683 | 5017.7 | 899.3 |
| mostly unexplained (+softirq TASKLET) | 403 | 2924.3 | 75.3 |
| mostly unexplained (+softirq NET_RX) | 38 | 495.0 | 60.5 |
| irq mana_hwc@pci:7870:00:00.0 | 61 | 364.8 | 8.7 |
| mostly unexplained (+softirq TIMER) | 58 | 327.3 | 7.2 |
| mostly unexplained (+irq mana_hwc@pci:7870:00:00.0) | 12 | 127.9 | 20.1 |
| mostly unexplained (+irq mana_q7@pci:7870:00:00.0) | 11 | 78.4 | 10.4 |
| mostly unexplained (+reschedule (IPI/vector)) | 9 | 68.8 | 10.6 |
| timer tick_nohz_handler | 3 | 19.9 | 7.7 |
| mostly unexplained (+irq mana_q5@pci:7870:00:00.0) | 3 | 16.2 | 5.9 |
| mostly unexplained (+irq mana_q1@pci:7870:00:00.0) | 1 | 8.7 | 8.7 |
| mostly unexplained (+irq mana_q0@pci:7870:00:00.0) | 1 | 7.2 | 7.2 |
| mostly unexplained (+irq mana_q2@pci:7870:00:00.0) | 1 | 7.0 | 7.0 |
| irq mana_q4@pci:7870:00:00.0 | 1 | 7.0 | 7.0 |
| mostly unexplained (+irq mana_q6@pci:7870:00:00.0) | 1 | 5.1 | 5.1 |
| mostly unexplained (+irq mana_q4@pci:7870:00:00.0) | 1 | 5.0 | 5.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 899.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 2 | 801.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 607.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 495.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 477.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 320.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 132.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 129.8 | 0 | softirq NET_RX | softirq NET_RX 128, irq mana_q0@pci:7870:00:00.0 1, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 93.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 10 | 85.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 68056 |
| 48 HV-PCI-MSIX-7870:00:00.0 8-edge mana_q7@pci:7870:00:00.0 | 7047 |
| 41 HV-PCI-MSIX-7870:00:00.0 1-edge mana_q0@pci:7870:00:00.0 | 1952 |
| CAL Function call interrupts | 1015 |
| IWI IRQ work interrupts | 755 |
| 43 HV-PCI-MSIX-7870:00:00.0 3-edge mana_q2@pci:7870:00:00.0 | 283 |
| 45 HV-PCI-MSIX-7870:00:00.0 5-edge mana_q4@pci:7870:00:00.0 | 283 |
| 46 HV-PCI-MSIX-7870:00:00.0 6-edge mana_q5@pci:7870:00:00.0 | 240 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 227 |
| 44 HV-PCI-MSIX-7870:00:00.0 4-edge mana_q3@pci:7870:00:00.0 | 145 |
| 40 HV-PCI-MSIX-7870:00:00.0 0-edge mana_hwc@pci:7870:00:00.0 | 122 |
| 47 HV-PCI-MSIX-7870:00:00.0 7-edge mana_q6@pci:7870:00:00.0 | 113 |

### Loop counters (r1-irqlocal/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1245808 | 1245808 | 1.00 | 229654689 | 184.34 | 3827578 | 3786051 | 0.0 | 48.5 | false | 0 | 0 | 661.9 | 1044589 / 177 / 3 | 0 | 0 |

#### Stalls: r1-irqlocal/pp-epoll-spin-0, loop 0 on CPU 2

227163072 iterations, longest 661.9 µs; over 5 / 20 / 100 µs: 1044589 / 177 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 169 | 6716.6 | 661.9 |
| mostly unexplained (+softirq NET_RX) | 5 | 158.3 | 46.9 |
| softirq NET_RX | 3 | 74.0 | 33.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 661.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 295.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 101.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 95.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 79.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 71.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 71.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 71.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 67.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 61.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 47 HV-PCI-MSIX-7870:00:00.0 7-edge mana_q6@pci:7870:00:00.0 | 2483972 |
| 46 HV-PCI-MSIX-7870:00:00.0 6-edge mana_q5@pci:7870:00:00.0 | 2373822 |
| HVS Hyper-V stimer0 interrupts | 60148 |
| CAL Function call interrupts | 1015 |
| 43 HV-PCI-MSIX-7870:00:00.0 3-edge mana_q2@pci:7870:00:00.0 | 189 |
| 45 HV-PCI-MSIX-7870:00:00.0 5-edge mana_q4@pci:7870:00:00.0 | 137 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 122 |
| 44 HV-PCI-MSIX-7870:00:00.0 4-edge mana_q3@pci:7870:00:00.0 | 99 |
| 48 HV-PCI-MSIX-7870:00:00.0 8-edge mana_q7@pci:7870:00:00.0 | 87 |
| 40 HV-PCI-MSIX-7870:00:00.0 0-edge mana_hwc@pci:7870:00:00.0 | 62 |
| 41 HV-PCI-MSIX-7870:00:00.0 1-edge mana_q0@pci:7870:00:00.0 | 49 |
| IWI IRQ work interrupts | 4 |

#### Slowest round trips: r1-irqlocal/pp-epoll-spin-0 (median 45.40)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 716.72 | 671.32 | 699.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 630.67 | 585.27 | 0.00 |  | 0.00 |  | 585.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 547.50 | 502.10 | 0.00 |  | 0.00 |  | 502.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 408.36 | 362.96 | 0.00 |  | 0.00 |  | 362.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 400.51 | 355.11 | 0.00 |  | 0.00 |  | 355.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 307.54 | 262.14 | 295.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 285.68 | 240.28 | 0.00 |  | 84.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 155.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 265.76 | 220.36 | 0.00 |  | 202.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 17.52 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 208.10 | 162.70 | 0.00 |  | 20.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 142.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 143.19 | 97.78 | 0.00 |  | 243.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-irqlocal/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198710 | 1198710 | 1.00 | 539398189 | 449.98 | 4494985 | 4475006 | 0.0 | 38.0 | false | 1 | 0 | 739.3 | 1038107 / 299 / 3 | 0 | 0 |

#### Stalls: r1-irqlocal/pp-epoll-spin-10000, loop 0 on CPU 2

537000768 iterations, longest 739.3 µs; over 5 / 20 / 100 µs: 1038107 / 299 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 282 | 10513.6 | 739.3 |
| mostly unexplained (+softirq NET_RX) | 9 | 237.2 | 40.1 |
| softirq NET_RX | 7 | 165.8 | 28.6 |
| mostly unexplained (+irq mana_q2@pci:7870:00:00.0) | 1 | 20.9 | 20.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 739.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 733.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 108.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 83.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 71.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 71.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 70.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 68.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 62.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 61.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 43 HV-PCI-MSIX-7870:00:00.0 3-edge mana_q2@pci:7870:00:00.0 | 2397658 |
| 47 HV-PCI-MSIX-7870:00:00.0 7-edge mana_q6@pci:7870:00:00.0 | 2288321 |
| HVS Hyper-V stimer0 interrupts | 87016 |
| IWI IRQ work interrupts | 1292 |
| CAL Function call interrupts | 1018 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 298 |
| 48 HV-PCI-MSIX-7870:00:00.0 8-edge mana_q7@pci:7870:00:00.0 | 278 |
| 41 HV-PCI-MSIX-7870:00:00.0 1-edge mana_q0@pci:7870:00:00.0 | 261 |
| 45 HV-PCI-MSIX-7870:00:00.0 5-edge mana_q4@pci:7870:00:00.0 | 217 |
| 44 HV-PCI-MSIX-7870:00:00.0 4-edge mana_q3@pci:7870:00:00.0 | 202 |
| 46 HV-PCI-MSIX-7870:00:00.0 6-edge mana_q5@pci:7870:00:00.0 | 129 |
| 40 HV-PCI-MSIX-7870:00:00.0 0-edge mana_hwc@pci:7870:00:00.0 | 122 |

#### Slowest round trips: r1-irqlocal/pp-epoll-spin-10000 (median 47.86)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 2013.08 | 1965.22 | 0.00 |  | 40.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 1925.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 818.01 | 770.15 | 0.00 |  | 277.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 493.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 782.83 | 734.97 | 733.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 1.70 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 493.52 | 445.66 | 0.00 |  | 20.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 424.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 369.58 | 321.72 | 0.00 |  | 0.00 |  | 321.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 360.52 | 312.66 | 0.00 |  | 0.00 |  | 312.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 341.53 | 293.67 | 0.00 |  | 21.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 272.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 331.72 | 283.86 | 0.00 |  | 11.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 272.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 317.00 | 269.14 | 0.00 |  | 0.00 |  | 269.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 295.58 | 247.72 | 0.00 |  | 69.66 | unexplained: no kernel event (hypervisor or in-kernel time) | 178.06 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-irqlocal/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1246773 | 1246773 | 1.00 | 49088118 | 39.37 | 818135 | 797356 | 97.4 | 87.5 | true | 0 | 0 | 348.4 | 1090759 / 199 / 2 | 0 | 0 |

#### Stalls: r1-irqlocal/pp-recv-busy-0, loop 0 on CPU 2

47841344 iterations, longest 348.4 µs; over 5 / 20 / 100 µs: 1090759 / 199 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 183 | 6596.7 | 348.4 |
| softirq NET_RX | 9 | 240.5 | 39.2 |
| mostly unexplained (+softirq NET_RX) | 5 | 189.7 | 68.5 |
| mostly unexplained (+irq mana_q4@pci:7870:00:00.0) | 2 | 84.4 | 43.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 348.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 255.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 95.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 81.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 79.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 74.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 72.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 68.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 19, irq mana_q4@pci:7870:00:00.0 1 |
| 9 | 65.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 59.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 45 HV-PCI-MSIX-7870:00:00.0 5-edge mana_q4@pci:7870:00:00.0 | 4969597 |
| HVS Hyper-V stimer0 interrupts | 60146 |
| CAL Function call interrupts | 1015 |
| 44 HV-PCI-MSIX-7870:00:00.0 4-edge mana_q3@pci:7870:00:00.0 | 188 |
| 47 HV-PCI-MSIX-7870:00:00.0 7-edge mana_q6@pci:7870:00:00.0 | 185 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 129 |
| 43 HV-PCI-MSIX-7870:00:00.0 3-edge mana_q2@pci:7870:00:00.0 | 126 |
| 48 HV-PCI-MSIX-7870:00:00.0 8-edge mana_q7@pci:7870:00:00.0 | 97 |
| 41 HV-PCI-MSIX-7870:00:00.0 1-edge mana_q0@pci:7870:00:00.0 | 69 |
| 40 HV-PCI-MSIX-7870:00:00.0 0-edge mana_hwc@pci:7870:00:00.0 | 61 |
| 46 HV-PCI-MSIX-7870:00:00.0 6-edge mana_q5@pci:7870:00:00.0 | 54 |
| IWI IRQ work interrupts | 4 |

#### Slowest round trips: r1-irqlocal/pp-recv-busy-0 (median 44.91)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 490.88 | 445.97 | 0.00 |  | 166.61 | unexplained: no kernel event (hypervisor or in-kernel time) | 279.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 412.58 | 367.67 | 0.00 |  | 91.73 | unexplained: no kernel event (hypervisor or in-kernel time) | 275.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 397.65 | 352.74 | 0.00 |  | 199.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 153.60 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 382.26 | 337.35 | 0.00 |  | 237.89 | unexplained: no kernel event (hypervisor or in-kernel time) | 99.47 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 371.71 | 326.80 | 348.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.12 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 276.76 | 231.85 | 0.00 |  | 40.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 191.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 274.65 | 229.74 | 0.00 |  | 0.00 |  | 229.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 265.19 | 220.28 | 254.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 40.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 178.90 | 133.99 | 0.00 |  | 278.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 172.58 | 127.67 | 0.00 |  | 0.00 |  | 127.67 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-irqlocal/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194657 | 1194657 | 1.00 | 117262177 | 98.16 | 977185 | 967229 | 99.0 | 84.9 | true | 1 | 0 | 623.3 | 1124518 / 328 / 4 | 0 | 0 |

#### Stalls: r1-irqlocal/pp-recv-busy-10000, loop 0 on CPU 2

116067520 iterations, longest 623.3 µs; over 5 / 20 / 100 µs: 1124518 / 328 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 308 | 11296.2 | 623.3 |
| mostly unexplained (+softirq NET_RX) | 9 | 270.7 | 58.5 |
| softirq NET_RX | 9 | 224.2 | 32.7 |
| mostly unexplained (+irq mana_q0@pci:7870:00:00.0) | 1 | 31.9 | 31.9 |
| mostly unexplained (+softirq TIMER) | 1 | 21.1 | 21.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 623.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 527.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 260.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 157.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 76.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 75.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 73.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 72.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 71.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 70.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 47 HV-PCI-MSIX-7870:00:00.0 7-edge mana_q6@pci:7870:00:00.0 | 2389544 |
| 41 HV-PCI-MSIX-7870:00:00.0 1-edge mana_q0@pci:7870:00:00.0 | 2380524 |
| HVS Hyper-V stimer0 interrupts | 84627 |
| IWI IRQ work interrupts | 1432 |
| CAL Function call interrupts | 1018 |
| 43 HV-PCI-MSIX-7870:00:00.0 3-edge mana_q2@pci:7870:00:00.0 | 345 |
| 44 HV-PCI-MSIX-7870:00:00.0 4-edge mana_q3@pci:7870:00:00.0 | 279 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 253 |
| 46 HV-PCI-MSIX-7870:00:00.0 6-edge mana_q5@pci:7870:00:00.0 | 236 |
| 45 HV-PCI-MSIX-7870:00:00.0 5-edge mana_q4@pci:7870:00:00.0 | 170 |
| 48 HV-PCI-MSIX-7870:00:00.0 8-edge mana_q7@pci:7870:00:00.0 | 151 |
| 40 HV-PCI-MSIX-7870:00:00.0 0-edge mana_hwc@pci:7870:00:00.0 | 120 |

#### Slowest round trips: r1-irqlocal/pp-recv-busy-10000 (median 46.73)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1017.25 | 970.52 | 0.00 |  | 937.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 33.36 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 650.16 | 603.43 | 622.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 21.66 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 570.80 | 524.07 | 527.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 427.05 | 380.32 | 0.00 |  | 0.00 |  | 380.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 418.65 | 371.92 | 0.00 |  | 20.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 351.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 414.50 | 367.77 | 0.00 |  | 0.00 |  | 367.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 408.73 | 362.00 | 0.00 |  | 96.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 265.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 397.72 | 350.99 | 0.00 |  | 128.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 222.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 355.95 | 309.22 | 0.00 |  | 311.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 329.84 | 283.11 | 0.00 |  | 0.00 |  | 283.11 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-irqlocal/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1238036 | 1238036 | 1.00 | 3714109 | 3.00 | 61902 | 37411355 | 0.0 | 26.6 | false | 0 | 0 | 335.7 | 1128313 / 174 / 4 | 0 | 0 |

#### Stalls: r1-irqlocal/pp-uring-spin-0, loop 0 on CPU 2

2244681280 iterations, longest 335.7 µs; over 5 / 20 / 100 µs: 1128313 / 174 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 166 | 6256.9 | 335.7 |
| softirq NET_RX | 6 | 159.1 | 35.5 |
| mostly unexplained (+irq mana_q6@pci:7870:00:00.0) | 1 | 23.9 | 23.9 |
| mostly unexplained (+softirq NET_RX) | 1 | 23.0 | 23.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 335.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 153.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 118.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 107.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 83.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 81.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 72.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 72.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 71.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 69.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 47 HV-PCI-MSIX-7870:00:00.0 7-edge mana_q6@pci:7870:00:00.0 | 2469053 |
| 41 HV-PCI-MSIX-7870:00:00.0 1-edge mana_q0@pci:7870:00:00.0 | 2359621 |
| HVS Hyper-V stimer0 interrupts | 60152 |
| CAL Function call interrupts | 1018 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 149 |
| 44 HV-PCI-MSIX-7870:00:00.0 4-edge mana_q3@pci:7870:00:00.0 | 145 |
| 45 HV-PCI-MSIX-7870:00:00.0 5-edge mana_q4@pci:7870:00:00.0 | 136 |
| 46 HV-PCI-MSIX-7870:00:00.0 6-edge mana_q5@pci:7870:00:00.0 | 125 |
| 48 HV-PCI-MSIX-7870:00:00.0 8-edge mana_q7@pci:7870:00:00.0 | 103 |
| 43 HV-PCI-MSIX-7870:00:00.0 3-edge mana_q2@pci:7870:00:00.0 | 93 |
| 40 HV-PCI-MSIX-7870:00:00.0 0-edge mana_hwc@pci:7870:00:00.0 | 70 |
| IWI IRQ work interrupts | 6 |

#### Slowest round trips: r1-irqlocal/pp-uring-spin-0 (median 44.97)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 611.59 | 566.62 | 0.00 |  | 0.00 |  | 566.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 503.92 | 458.95 | 0.00 |  | 0.00 |  | 458.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 467.47 | 422.50 | 0.00 |  | 0.00 |  | 422.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 352.27 | 307.31 | 335.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 314.29 | 269.32 | 0.00 |  | 0.00 |  | 269.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 282.68 | 237.72 | 0.00 |  | 20.64 | unexplained: no kernel event (hypervisor or in-kernel time) | 217.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 253.10 | 208.13 | 0.00 |  | 0.00 |  | 208.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 227.43 | 182.46 | 193.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 180.80 | 135.84 | 0.00 |  | 0.00 |  | 135.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 153.61 | 108.64 | 117.98 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-irqlocal/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199683 | 1199683 | 1.00 | 3599050 | 3.00 | 29992 | 32824226 | 0.0 | 13.4 | false | 0 | 8 | 643.1 | 1133625 / 493 / 11 | 0 | 0 |

#### Stalls: r1-irqlocal/pp-uring-spin-10000, loop 0 on CPU 2

3938907136 iterations, longest 643.1 µs; over 5 / 20 / 100 µs: 1133625 / 493 / 11

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 474 | 16730.8 | 643.1 |
| mostly unexplained (+softirq NET_RX) | 12 | 366.8 | 56.1 |
| softirq NET_RX | 5 | 130.5 | 35.5 |
| mostly unexplained (+irq mana_q1@pci:7870:00:00.0) | 1 | 47.0 | 47.0 |
| irq mana_q4@pci:7870:00:00.0 | 1 | 43.8 | 43.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 643.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 331.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 326.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 299.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 163.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 127.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 125.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 120.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 111.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 111.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 2399579 |
| 45 HV-PCI-MSIX-7870:00:00.0 5-edge mana_q4@pci:7870:00:00.0 | 2266540 |
| HVS Hyper-V stimer0 interrupts | 41232 |
| 46 HV-PCI-MSIX-7870:00:00.0 6-edge mana_q5@pci:7870:00:00.0 | 4191 |
| IWI IRQ work interrupts | 3084 |
| CAL Function call interrupts | 1018 |
| 48 HV-PCI-MSIX-7870:00:00.0 8-edge mana_q7@pci:7870:00:00.0 | 321 |
| 41 HV-PCI-MSIX-7870:00:00.0 1-edge mana_q0@pci:7870:00:00.0 | 275 |
| 43 HV-PCI-MSIX-7870:00:00.0 3-edge mana_q2@pci:7870:00:00.0 | 239 |
| 44 HV-PCI-MSIX-7870:00:00.0 4-edge mana_q3@pci:7870:00:00.0 | 239 |
| 40 HV-PCI-MSIX-7870:00:00.0 0-edge mana_hwc@pci:7870:00:00.0 | 122 |
| 47 HV-PCI-MSIX-7870:00:00.0 7-edge mana_q6@pci:7870:00:00.0 | 90 |

#### Slowest round trips: r1-irqlocal/pp-uring-spin-10000 (median 47.69)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 678.35 | 630.67 | 643.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 552.26 | 504.57 | 0.00 |  | 21.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 483.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 546.20 | 498.51 | 0.00 |  | 0.00 |  | 498.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 465.79 | 418.11 | 0.00 |  | 20.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 397.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 414.58 | 366.89 | 0.00 |  | 0.00 |  | 366.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 405.89 | 358.20 | 0.00 |  | 0.00 |  | 358.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 401.08 | 353.40 | 0.00 |  | 0.00 |  | 353.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 396.23 | 348.54 | 0.00 |  | 20.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 328.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 379.39 | 331.71 | 0.00 |  | 22.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 309.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 377.10 | 329.41 | 0.00 |  | 20.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 308.66 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-6/http-recv-busy-10000, loop 0 on CPU 2

1898546619 iterations, longest 1365.1 µs; over 5 / 20 / 100 µs: 172184 / 174 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 138 | 4930.7 | 161.6 |
| mostly unexplained (+timer tick_nohz_handler) | 12 | 2906.6 | 1365.1 |
| mostly unexplained (+softirq NET_RX) | 16 | 1096.2 | 448.4 |
| loop work | 3 | 84.4 | 35.4 |
| mostly unexplained (+ipi_send_cpu) | 3 | 68.8 | 24.5 |
| mostly unexplained (+softirq TIMER) | 2 | 68.4 | 46.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1365.1 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 2 | 1009.3 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 448.4 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 6 |
| 4 | 243.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 5 | 161.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 129.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 107.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 106.9 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 80.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 76.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120060 |
| CAL Function call interrupts | 1017 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-6/http-recv-busy-10000 (median 48.28)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1193.55 | 1145.26 | 243.21 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 902.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1043.31 | 995.02 | 1029.20 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 848.10 | 799.82 | 0.00 |  | 0.00 |  | 799.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 530.02 | 481.74 | 0.00 |  | 21.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 460.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 507.26 | 458.98 | 474.20 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 6 | 447.47 | 399.19 | 0.00 |  | 0.00 |  | 399.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 404.99 | 356.71 | 0.00 |  | 0.00 |  | 356.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 391.21 | 342.93 | 0.00 |  | 0.00 |  | 342.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 384.37 | 336.09 | 0.00 |  | 21.31 | unexplained: no kernel event (hypervisor or in-kernel time) | 314.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 378.04 | 329.76 | 0.00 |  | 0.00 |  | 329.76 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-6/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=624823 clocksource=tsc

### Loop counters (r1-base-6/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001550 | 891139 | 3.37 | 62836288 | 20.93 | 1047271 | 1047271 | 98.6 | 83.5 | true | 0 | 0 | 720.3 | 356107 / 4896 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8215 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672347200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (215592 of 2900000 in 1-msg reads) | 215592 | 4.64 | 3.28 | 6.91 | 22.63 | 34.69 | 43.43 | 55.98 |

#### Stalls: r1-base-6/md-recv-busy, loop 0 on CPU 2

62836288 iterations, longest 720.3 µs; over 5 / 20 / 100 µs: 356107 / 4896 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 3372 | 76236.3 | 720.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1229 | 27293.7 | 100.0 |
| mostly unexplained (+timer tick_nohz_handler) | 275 | 7889.8 | 46.5 |
| softirq NET_RX | 18 | 478.9 | 64.6 |
| mostly unexplained (+softirq TIMER) | 1 | 30.2 | 30.2 |
| mostly unexplained (+softirq RCU) | 1 | 29.2 | 29.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 720.3 | 50 | loop work (50 items) | timer tick_nohz_handler 2 |
| 2 | 428.5 | 1550 | loop work (1550 items) |  |
| 3 | 100.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 94.2 | 20 | loop work (20 items) |  |
| 5 | 89.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 71.0 | 9 | loop work (9 items) | softirq NET_RX 1 |
| 7 | 68.8 | 12 | loop work (12 items) | softirq NET_RX 11 |
| 8 | 64.6 | 18 | softirq NET_RX | softirq NET_RX 42 |
| 9 | 61.4 | 5 | loop work (5 items) |  |
| 10 | 60.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60035 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

## r1-base-6/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-6/noise, loop 0 on CPU 2

12346937344 iterations, longest 1082.7 µs; over 5 / 20 / 100 µs: 1347 / 114 / 9

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 876 | 7801.5 | 1082.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 367 | 6519.9 | 244.3 |
| mostly unexplained (+softirq TIMER) | 99 | 566.6 | 8.3 |
| mostly unexplained (+softirq NET_RX) | 1 | 28.5 | 28.5 |
| timer tick_nohz_handler | 3 | 17.5 | 7.2 |
| softirq TIMER | 1 | 8.9 | 8.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1082.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 2 | 932.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 827.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 4 | 244.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 166.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 165.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 143.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 8 | 113.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 104.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 95.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120025 |
| CAL Function call interrupts | 1016 |
| 46 HV-PCI-MSIX-7870:00:00.0 6-edge mana_q5@pci:7870:00:00.0 | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-base-6/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1205658 | 1205658 | 1.00 | 303152565 | 251.44 | 5052543 | 5012354 | 0.0 | 37.6 | false | 0 | 0 | 967.7 | 12316 / 74 / 4 | 0 | 0 |

#### Stalls: r1-base-6/pp-epoll-spin-0, loop 0 on CPU 2

300741248 iterations, longest 967.7 µs; over 5 / 20 / 100 µs: 12316 / 74 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 15 | 2544.6 | 967.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 57 | 2136.7 | 143.2 |
| mostly unexplained (+softirq RCU) | 2 | 49.5 | 27.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 967.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 2 | 868.5 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4, softirq TIMER 1 |
| 3 | 383.1 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 4 | 143.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 90.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 88.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 79.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 63.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 61.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 56.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60145 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-6/pp-epoll-spin-0 (median 46.07)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1016.62 | 970.55 | 967.69 | mostly unexplained (+timer tick_nohz_handler) | 20.45 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 879.63 | 833.56 | 868.18 | mostly unexplained (+timer tick_nohz_handler) | 20.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 430.72 | 384.65 | 0.00 |  | 40.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 343.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 411.24 | 365.17 | 0.00 |  | 60.64 | unexplained: no kernel event (hypervisor or in-kernel time) | 304.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 403.27 | 357.20 | 0.00 |  | 20.65 | unexplained: no kernel event (hypervisor or in-kernel time) | 336.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 396.46 | 350.39 | 0.00 |  | 0.00 |  | 350.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 396.19 | 350.12 | 0.00 |  | 40.74 | unexplained: no kernel event (hypervisor or in-kernel time) | 309.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 395.68 | 349.61 | 0.00 |  | 0.00 |  | 349.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 392.12 | 346.06 | 382.88 | mostly unexplained (+timer tick_nohz_handler) | 20.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 10 | 390.60 | 344.53 | 0.00 |  | 20.54 | unexplained: no kernel event (hypervisor or in-kernel time) | 323.99 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198541 | 1198541 | 1.00 | 610146523 | 509.07 | 5084554 | 5064579 | 0.0 | 32.6 | false | 0 | 0 | 831.9 | 15413 / 159 / 10 | 0 | 0 |

#### Stalls: r1-base-6/pp-epoll-spin-10000, loop 0 on CPU 2

607749440 iterations, longest 831.9 µs; over 5 / 20 / 100 µs: 15413 / 159 / 10

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 140 | 6764.0 | 750.4 |
| mostly unexplained (+timer tick_nohz_handler) | 15 | 1740.1 | 831.9 |
| mostly unexplained (+softirq TIMER) | 2 | 50.2 | 25.8 |
| mostly unexplained (+softirq RCU) | 2 | 42.6 | 21.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 831.9 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 2 | 750.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 654.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 495.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 333.5 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq TIMER 1 |
| 6 | 199.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 184.9 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 8 | 172.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 161.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 124.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120114 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-6/pp-epoll-spin-10000 (median 47.79)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1015.64 | 967.85 | 1004.25 | mostly unexplained (+timer tick_nohz_handler) | 20.54 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 783.72 | 735.93 | 750.31 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 543.39 | 495.60 | 0.00 |  | 41.47 | unexplained: no kernel event (hypervisor or in-kernel time) | 454.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 519.24 | 471.45 | 0.00 |  | 21.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 450.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 468.68 | 420.89 | 0.00 |  | 0.00 |  | 420.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 429.17 | 381.38 | 0.00 |  | 42.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 339.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 426.98 | 379.19 | 0.00 |  | 21.57 | unexplained: no kernel event (hypervisor or in-kernel time) | 357.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 409.26 | 361.47 | 0.00 |  | 27.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 334.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 403.28 | 355.49 | 0.00 |  | 41.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 314.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 399.43 | 351.64 | 0.00 |  | 0.00 |  | 351.64 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1279424 | 1279424 | 1.00 | 59868993 | 46.79 | 997817 | 976493 | 97.8 | 85.4 | true | 0 | 0 | 715.5 | 138375 / 117 / 4 | 0 | 0 |

#### Stalls: r1-base-6/pp-recv-busy-0, loop 0 on CPU 2

58589568 iterations, longest 715.5 µs; over 5 / 20 / 100 µs: 138375 / 117 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 74 | 3756.0 | 715.5 |
| mostly unexplained (+softirq NET_RX) | 22 | 828.3 | 107.9 |
| mostly unexplained (+timer tick_nohz_handler) | 14 | 465.2 | 125.8 |
| mostly unexplained (+ipi_send_cpu) | 4 | 97.3 | 27.6 |
| mostly unexplained (+softirq RCU) | 1 | 23.9 | 23.9 |
| timer tick_nohz_handler | 1 | 22.7 | 22.7 |
| softirq NET_RX | 1 | 20.2 | 20.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 715.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 575.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 125.8 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq TIMER 1 |
| 4 | 107.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 72.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 66.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 64.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 63.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 61.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 57.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60147 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-6/pp-recv-busy-0 (median 44.29)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1992.82 | 1948.54 | 0.00 |  | 185.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 1763.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 757.38 | 713.09 | 0.00 |  | 426.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 286.89 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 725.33 | 681.04 | 715.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 60.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 608.62 | 564.33 | 574.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 516.32 | 472.03 | 0.00 |  | 114.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 357.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 440.24 | 395.95 | 0.00 |  | 0.00 |  | 395.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 308.25 | 263.96 | 0.00 |  | 21.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 242.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 283.78 | 239.50 | 0.00 |  | 0.00 |  | 239.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 282.52 | 238.24 | 0.00 |  | 40.06 | unexplained: no kernel event (hypervisor or in-kernel time) | 198.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 193.59 | 149.31 | 0.00 |  | 20.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 129.18 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194583 | 1194583 | 1.00 | 128051351 | 107.19 | 1067095 | 1057140 | 99.1 | 84.1 | true | 1 | 0 | 597.2 | 182543 / 188 / 7 | 0 | 0 |

#### Stalls: r1-base-6/pp-recv-busy-10000, loop 0 on CPU 2

126856768 iterations, longest 597.2 µs; over 5 / 20 / 100 µs: 182543 / 188 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 125 | 5030.9 | 485.3 |
| mostly unexplained (+softirq NET_RX) | 30 | 1823.9 | 597.2 |
| mostly unexplained (+timer tick_nohz_handler) | 17 | 558.6 | 50.5 |
| preempted by swapper/2 | 1 | 545.3 | 545.3 |
| mostly unexplained (+ipi_send_cpu) | 4 | 92.3 | 29.1 |
| mostly unexplained (+reschedule (IPI/vector)) | 4 | 87.4 | 23.3 |
| mostly unexplained (+softirq RCU) | 2 | 55.3 | 30.2 |
| timer tick_nohz_handler | 2 | 48.3 | 28.0 |
| mostly unexplained (+softirq TIMER) | 1 | 30.3 | 30.3 |
| mostly unexplained (+call function single (IPI/vector)) | 1 | 23.6 | 23.6 |
| softirq NET_RX | 1 | 22.0 | 22.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 597.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 545.3 | 1 | preempted by swapper/2 | preempted by swapper/2 523, call function single (IPI/vector) 3, page fault 0 |
| 3 | 485.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 172.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 166.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 137.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 116.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 75.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 67.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 64.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 78867 |
| CAL Function call interrupts | 1023 |
| RES Rescheduling interrupts | 42 |
| IWI IRQ work interrupts | 3 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-6/pp-recv-busy-10000 (median 46.82)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 799.37 | 752.55 | 0.00 |  | 475.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 276.71 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 625.63 | 578.81 | 596.80 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 3 | 585.63 | 538.81 | 544.74 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 4 | 579.21 | 532.39 | 0.00 |  | 346.76 | unexplained: no kernel event (hypervisor or in-kernel time) | 185.63 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 541.54 | 494.72 | 485.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 9.43 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 406.47 | 359.65 | 0.00 |  | 79.05 | unexplained: no kernel event (hypervisor or in-kernel time) | 280.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 392.08 | 345.27 | 0.00 |  | 438.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 365.83 | 319.01 | 0.00 |  | 199.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 119.20 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 299.61 | 252.79 | 0.00 |  | 37.88 | unexplained: no kernel event (hypervisor or in-kernel time) | 214.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 284.80 | 237.99 | 0.00 |  | 0.00 |  | 237.99 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1231240 | 1231240 | 1.00 | 3693721 | 3.00 | 61562 | 49326283 | 0.0 | 11.9 | false | 0 | 0 | 536.4 | 11649 / 60 / 2 | 0 | 0 |

#### Stalls: r1-base-6/pp-uring-spin-0, loop 0 on CPU 2

2959576960 iterations, longest 536.4 µs; over 5 / 20 / 100 µs: 11649 / 60 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 53 | 2251.3 | 536.4 |
| mostly unexplained (+timer tick_nohz_handler) | 6 | 174.6 | 55.9 |
| mostly unexplained (+softirq TIMER) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 536.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 137.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 60.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 60.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 55.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 6 | 51.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 48.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 48.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 47.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 44.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60146 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-6/pp-uring-spin-0 (median 46.28)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 672.90 | 626.62 | 536.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 90.21 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 543.78 | 497.50 | 0.00 |  | 60.54 | unexplained: no kernel event (hypervisor or in-kernel time) | 436.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 521.83 | 475.55 | 0.00 |  | 20.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 454.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 513.83 | 467.55 | 0.00 |  | 0.00 |  | 467.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 468.98 | 422.70 | 0.00 |  | 51.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 371.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 465.52 | 419.24 | 0.00 |  | 20.36 | unexplained: no kernel event (hypervisor or in-kernel time) | 398.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 460.04 | 413.76 | 0.00 |  | 0.00 |  | 413.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 406.44 | 360.16 | 0.00 |  | 21.35 | unexplained: no kernel event (hypervisor or in-kernel time) | 338.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 405.05 | 358.77 | 0.00 |  | 0.00 |  | 358.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 386.30 | 340.02 | 0.00 |  | 0.00 |  | 340.02 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199382 | 1199382 | 1.00 | 3598147 | 3.00 | 29985 | 38675697 | 0.0 | 5.8 | false | 1 | 0 | 556.6 | 11229 / 128 / 8 | 0 | 0 |

#### Stalls: r1-base-6/pp-uring-spin-10000, loop 0 on CPU 2

4641083584 iterations, longest 556.6 µs; over 5 / 20 / 100 µs: 11229 / 128 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 113 | 5485.7 | 552.5 |
| preempted by swapper/2 | 1 | 556.6 | 556.6 |
| mostly unexplained (+timer tick_nohz_handler) | 11 | 345.0 | 66.2 |
| mostly unexplained (+softirq RCU) | 2 | 50.8 | 29.7 |
| mostly unexplained (+softirq TIMER) | 1 | 21.3 | 21.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 556.6 | 0 | preempted by swapper/2 | preempted by swapper/2 530, call function single (IPI/vector) 3, page fault 0 |
| 2 | 552.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 481.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 323.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 209.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 175.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 162.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 151.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 93.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 89.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 82186 |
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 37 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-6/pp-uring-spin-10000 (median 47.57)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 711.74 | 664.16 | 0.00 |  | 20.35 | unexplained: no kernel event (hypervisor or in-kernel time) | 643.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 642.50 | 594.92 | 572.66 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 2.13 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 632.36 | 584.78 | 481.54 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 83.12 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 614.69 | 567.12 | 0.00 |  | 146.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 421.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 598.68 | 551.11 | 556.60 | preempted by swapper/2 | 20.56 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 6 | 460.78 | 413.21 | 0.00 |  | 29.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 383.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 391.43 | 343.86 | 0.00 |  | 0.00 |  | 343.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 386.42 | 338.85 | 0.00 |  | 20.98 | unexplained: no kernel event (hypervisor or in-kernel time) | 317.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 386.31 | 338.74 | 0.00 |  | 20.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 318.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 381.71 | 334.13 | 0.00 |  | 0.00 |  | 334.13 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1256045 | 1256045 | 1.00 | 1 | 0.00 | 0 | 47983989 | 0.0 | 0.0 | true | 0 | 0 | 447.2 | 243 / 55 / 2 | 0 | 0 |

#### Stalls: r1-base-6/pp-uring-sqpoll-0, loop 0 on CPU 2

2879039360 iterations, longest 447.2 µs; over 5 / 20 / 100 µs: 243 / 55 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 55 | 2219.5 | 447.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 447.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 108.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 72.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 46.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 45.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 42.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 42.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 41.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 40.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 39.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 59 |
| HVS Hyper-V stimer0 interrupts | 3 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-base-6/pp-uring-sqpoll-0 (median 45.27)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 495.86 | 450.59 | 447.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 3.42 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 473.74 | 428.47 | 0.00 |  | 20.65 | unexplained: no kernel event (hypervisor or in-kernel time) | 407.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 469.26 | 423.99 | 0.00 |  | 21.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 402.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 412.95 | 367.68 | 0.00 |  | 40.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 326.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 410.62 | 365.35 | 0.00 |  | 61.47 | unexplained: no kernel event (hypervisor or in-kernel time) | 303.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 362.38 | 317.11 | 0.00 |  | 0.00 |  | 317.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 353.13 | 307.87 | 0.00 |  | 0.00 |  | 307.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 334.22 | 288.95 | 0.00 |  | 41.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 247.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 325.18 | 279.90 | 0.00 |  | 0.00 |  | 279.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 308.67 | 263.40 | 0.00 |  | 21.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 241.55 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-6/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199312 | 1199312 | 1.00 | 1 | 0.00 | 0 | 35638895 | 0.0 | 0.0 | true | 0 | 0 | 440.8 | 485 / 105 / 3 | 0 | 0 |

#### Stalls: r1-base-6/pp-uring-sqpoll-10000, loop 0 on CPU 2

4276667392 iterations, longest 440.8 µs; over 5 / 20 / 100 µs: 485 / 105 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 105 | 4104.8 | 440.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 440.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 288.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 209.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 80.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 55.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 48.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 47.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 44.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 43.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1022 |
| RES Rescheduling interrupts | 117 |
| IWI IRQ work interrupts | 4 |
| HVS Hyper-V stimer0 interrupts | 3 |

#### Slowest round trips: r1-base-6/pp-uring-sqpoll-10000 (median 47.72)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1052.64 | 1004.92 | 0.00 |  | 0.00 |  | 1004.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 688.22 | 640.50 | 0.00 |  | 112.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 527.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 607.58 | 559.86 | 0.00 |  | 38.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 521.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 605.37 | 557.64 | 0.00 |  | 29.70 | unexplained: no kernel event (hypervisor or in-kernel time) | 527.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 561.73 | 514.01 | 0.00 |  | 104.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 409.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 428.53 | 380.80 | 0.00 |  | 40.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 340.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 405.69 | 357.96 | 0.00 |  | 39.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 318.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 403.23 | 355.50 | 0.00 |  | 0.00 |  | 355.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 396.60 | 348.87 | 0.00 |  | 20.83 | unexplained: no kernel event (hypervisor or in-kernel time) | 328.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 387.98 | 340.26 | 0.00 |  | 273.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 66.98 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

#### Stalls: r1-rt/http-recv-busy-10000, loop 0 on CPU 2

1926546825 iterations, longest 604.4 µs; over 5 / 20 / 100 µs: 136718 / 160 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 130 | 5246.4 | 517.5 |
| mostly unexplained (+timer tick_nohz_handler) | 6 | 1327.9 | 604.4 |
| mostly unexplained (+softirq NET_RX) | 17 | 971.7 | 499.2 |
| mostly unexplained (+softirq RCU) | 3 | 135.1 | 53.0 |
| mostly unexplained (+ipi_send_cpu) | 3 | 69.8 | 25.1 |
| mostly unexplained (+softirq TIMER) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 604.4 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq TIMER 1 |
| 2 | 586.0 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 517.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 499.2 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 169.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 112.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 109.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 65.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 60.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 60.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120054 |
| CAL Function call interrupts | 1017 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-rt/http-recv-busy-10000 (median 48.02)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 971.90 | 923.88 | 0.00 |  | 0.00 |  | 923.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 619.80 | 571.78 | 584.00 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 607.39 | 559.37 | 603.38 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 574.80 | 526.78 | 0.00 |  | 20.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 506.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 568.23 | 520.21 | 0.00 |  | 0.00 |  | 520.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 518.35 | 470.33 | 497.93 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 7 | 430.39 | 382.37 | 0.00 |  | 20.43 | unexplained: no kernel event (hypervisor or in-kernel time) | 361.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 400.45 | 352.43 | 0.00 |  | 0.00 |  | 352.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 322.07 | 274.05 | 0.00 |  | 0.00 |  | 274.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 319.48 | 271.47 | 0.00 |  | 0.00 |  | 271.47 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-rt/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=670444 clocksource=tsc

### Loop counters (r1-rt/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001550 | 975037 | 3.08 | 63223680 | 21.06 | 1053728 | 1053728 | 98.5 | 83.5 | true | 0 | 0 | 607.0 | 296991 / 5153 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8212 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672347200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (257483 of 2900000 in 1-msg reads) | 257483 | 4.60 | 3.22 | 6.97 | 22.60 | 26.11 | 32.28 | 63.31 |

#### Stalls: r1-rt/md-recv-busy, loop 0 on CPU 2

63223680 iterations, longest 607.0 µs; over 5 / 20 / 100 µs: 296991 / 5153 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 3665 | 80750.2 | 581.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1293 | 28565.9 | 607.0 |
| mostly unexplained (+timer tick_nohz_handler) | 154 | 3628.1 | 69.1 |
| softirq NET_RX | 20 | 507.2 | 38.5 |
| mostly unexplained (+softirq RCU) | 12 | 381.5 | 47.2 |
| mostly unexplained (+ipi_send_cpu) | 4 | 100.0 | 30.0 |
| mostly unexplained (+softirq NET_RX) | 4 | 88.2 | 25.3 |
| timer tick_nohz_handler | 1 | 21.1 | 21.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 607.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 581.5 | 50 | loop work (50 items) | softirq NET_RX 1, softirq RCU 1, timer tick_nohz_handler 1 |
| 3 | 414.6 | 1550 | loop work (1550 items) |  |
| 4 | 141.5 | 9 | loop work (9 items) | softirq NET_RX 5 |
| 5 | 73.0 | 18 | loop work (18 items) | softirq NET_RX 4 |
| 6 | 69.3 | 17 | loop work (17 items) |  |
| 7 | 69.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 8 | 67.6 | 14 | loop work (14 items) | softirq NET_RX 3 |
| 9 | 63.7 | 16 | loop work (16 items) | softirq NET_RX 5 |
| 10 | 59.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60037 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

## r1-rt/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-rt/noise, loop 0 on CPU 2

12371267584 iterations, longest 1063.2 µs; over 5 / 20 / 100 µs: 677 / 110 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 407 | 7852.2 | 627.1 |
| mostly unexplained (+timer tick_nohz_handler) | 176 | 3592.3 | 1063.2 |
| mostly unexplained (+softirq TIMER) | 94 | 547.6 | 24.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1063.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 2 | 778.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 3 | 740.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 4 | 627.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 383.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 374.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 326.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 117.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 65.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 61.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120025 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-rt/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1254621 | 1254621 | 1.00 | 60297182 | 48.06 | 1004953 | 984043 | 97.9 | 85.5 | true | 0 | 0 | 119.5 | 153845 / 98 / 1 | 0 | 0 |

#### Stalls: r1-rt/pp-recv-busy-0, loop 0 on CPU 2

59042560 iterations, longest 119.5 µs; over 5 / 20 / 100 µs: 153845 / 98 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 72 | 2381.7 | 68.3 |
| mostly unexplained (+softirq NET_RX) | 13 | 548.7 | 119.5 |
| mostly unexplained (+timer tick_nohz_handler) | 6 | 232.7 | 67.6 |
| mostly unexplained (+softirq TIMER) | 3 | 82.2 | 29.9 |
| softirq NET_RX | 2 | 71.6 | 49.4 |
| mostly unexplained (+ipi_send_cpu) | 2 | 40.8 | 20.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 119.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 68.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 67.6 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 4 | 67.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 66.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 66.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 58.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 54.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 50.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 50.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60147 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-rt/pp-recv-busy-0 (median 45.22)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 589.14 | 543.93 | 0.00 |  | 0.00 |  | 543.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 483.11 | 437.89 | 0.00 |  | 20.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 417.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 389.58 | 344.37 | 0.00 |  | 0.00 |  | 344.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 314.73 | 269.52 | 0.00 |  | 0.00 |  | 269.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 303.82 | 258.60 | 0.00 |  | 21.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 237.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 277.66 | 232.44 | 0.00 |  | 0.00 |  | 232.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 217.09 | 171.88 | 0.00 |  | 0.00 |  | 171.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 187.25 | 142.04 | 0.00 |  | 0.00 |  | 142.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 154.76 | 109.55 | 119.35 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 10 | 153.54 | 108.32 | 0.00 |  | 40.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 67.81 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-rt/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194565 | 1194565 | 1.00 | 128212550 | 107.33 | 1068438 | 1058483 | 99.1 | 84.1 | true | 1 | 0 | 840.3 | 165729 / 212 / 7 | 0 | 0 |

#### Stalls: r1-rt/pp-recv-busy-10000, loop 0 on CPU 2

127017984 iterations, longest 840.3 µs; over 5 / 20 / 100 µs: 165729 / 212 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 162 | 6033.6 | 840.3 |
| mostly unexplained (+softirq NET_RX) | 34 | 2036.0 | 720.2 |
| preempted by swapper/2 | 1 | 724.1 | 724.1 |
| mostly unexplained (+timer tick_nohz_handler) | 6 | 479.3 | 181.1 |
| mostly unexplained (+ipi_send_cpu) | 5 | 112.6 | 30.3 |
| softirq NET_RX | 2 | 54.6 | 34.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 43.9 | 43.9 |
| mostly unexplained (+softirq TIMER) | 1 | 29.0 | 29.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 840.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 724.1 | 1 | preempted by swapper/2 | preempted by swapper/2 701, call function single (IPI/vector) 2, page fault 0 |
| 3 | 720.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1, softirq RCU 1, softirq TIMER 1, timer tick_nohz_handler 1 |
| 4 | 181.1 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq TIMER 1 |
| 5 | 162.6 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 6 | 119.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 106.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 89.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 86.8 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 10 | 75.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 79804 |
| CAL Function call interrupts | 1019 |
| IWI IRQ work interrupts | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-rt/pp-recv-busy-10000 (median 46.87)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 886.10 | 839.22 | 840.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 83.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 772.41 | 725.54 | 723.50 | preempted by swapper/2 | 0.00 |  | 2.04 | client: preempted by swapper/2 |
| 3 | 771.46 | 724.59 | 0.00 |  | 20.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 704.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 732.62 | 685.75 | 719.86 | mostly unexplained (+softirq NET_RX) | 40.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 5 | 598.03 | 551.16 | 0.00 |  | 21.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 529.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 411.27 | 364.40 | 0.00 |  | 0.00 |  | 364.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 400.40 | 353.53 | 0.00 |  | 0.00 |  | 353.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 363.72 | 316.85 | 0.00 |  | 0.00 |  | 316.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 339.16 | 292.29 | 0.00 |  | 0.00 |  | 292.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 319.40 | 272.53 | 0.00 |  | 20.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 252.42 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-rt/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1184158 | 1184158 | 1.00 | 3552475 | 3.00 | 59208 | 50318187 | 0.0 | 11.4 | false | 0 | 0 | 132.4 | 18580 / 92 / 1 | 0 | 0 |

#### Stalls: r1-rt/pp-uring-spin-0, loop 0 on CPU 2

3019091200 iterations, longest 132.4 µs; over 5 / 20 / 100 µs: 18580 / 92 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 74 | 2308.7 | 132.4 |
| mostly unexplained (+timer tick_nohz_handler) | 6 | 204.9 | 59.5 |
| mostly unexplained (+softirq TIMER) | 7 | 154.4 | 26.3 |
| mostly unexplained (+softirq RCU) | 5 | 107.5 | 23.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 132.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 82.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 60.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 59.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 59.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 6 | 59.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 56.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 8 | 49.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 46.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 46.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60145 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-rt/pp-uring-spin-0 (median 46.57)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 526.02 | 479.46 | 0.00 |  | 85.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 394.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 490.22 | 443.65 | 0.00 |  | 20.43 | unexplained: no kernel event (hypervisor or in-kernel time) | 423.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 420.86 | 374.29 | 0.00 |  | 82.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 291.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 410.51 | 363.94 | 0.00 |  | 0.00 |  | 363.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 407.02 | 360.46 | 0.00 |  | 20.55 | unexplained: no kernel event (hypervisor or in-kernel time) | 339.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 401.58 | 355.01 | 0.00 |  | 0.00 |  | 355.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 399.09 | 352.52 | 0.00 |  | 0.00 |  | 352.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 356.60 | 310.03 | 0.00 |  | 20.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 289.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 346.33 | 299.76 | 0.00 |  | 0.00 |  | 299.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 333.13 | 286.57 | 0.00 |  | 22.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 264.28 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-rt/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199564 | 1199564 | 1.00 | 3598692 | 3.00 | 29989 | 39056725 | 0.0 | 5.8 | false | 0 | 0 | 798.5 | 13495 / 140 / 3 | 0 | 0 |

#### Stalls: r1-rt/pp-uring-spin-10000, loop 0 on CPU 2

4686807040 iterations, longest 798.5 µs; over 5 / 20 / 100 µs: 13495 / 140 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 128 | 4231.8 | 140.1 |
| mostly unexplained (+timer tick_nohz_handler) | 9 | 1677.2 | 798.5 |
| mostly unexplained (+softirq TIMER) | 2 | 112.5 | 81.4 |
| mostly unexplained (+softirq RCU) | 1 | 51.8 | 51.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 798.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 2 | 663.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 3 | 140.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 89.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 85.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 84.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 81.4 | 0 | mostly unexplained (+softirq TIMER) | softirq TIMER 1, timer tick_nohz_handler 1 |
| 8 | 63.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 62.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 59.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120109 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-rt/pp-uring-spin-10000 (median 47.39)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1865.76 | 1818.36 | 30.30 | unexplained: no kernel event (hypervisor or in-kernel time) | 21.42 | unexplained: no kernel event (hypervisor or in-kernel time) | 1766.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1657.15 | 1609.76 | 0.00 |  | 126.96 | unexplained: no kernel event (hypervisor or in-kernel time) | 1482.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 846.17 | 798.77 | 798.51 | mostly unexplained (+timer tick_nohz_handler) | 65.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 727.25 | 679.86 | 663.60 | mostly unexplained (+timer tick_nohz_handler) | 41.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 5 | 545.66 | 498.27 | 0.00 |  | 21.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 476.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 527.97 | 480.58 | 0.00 |  | 86.99 | unexplained: no kernel event (hypervisor or in-kernel time) | 393.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 507.24 | 459.84 | 0.00 |  | 0.00 |  | 459.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 489.41 | 442.02 | 0.00 |  | 347.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 94.61 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 480.63 | 433.23 | 0.00 |  | 21.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 411.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 472.59 | 425.19 | 0.00 |  | 0.00 |  | 425.19 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-sibling/http-recv-busy-10000, loop 0 on CPU 2

1490356084 iterations, longest 1034.3 µs; over 5 / 20 / 100 µs: 1223589 / 280 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 132 | 5547.8 | 791.8 |
| mostly unexplained (+timer tick_nohz_handler) | 28 | 3042.2 | 1034.3 |
| mostly unexplained (+softirq NET_RX) | 74 | 1916.5 | 58.9 |
| softirq NET_RX | 19 | 490.7 | 77.6 |
| mostly unexplained (+ipi_send_cpu) | 18 | 451.7 | 43.7 |
| loop work | 4 | 101.7 | 30.2 |
| mostly unexplained (+softirq TIMER) | 4 | 91.9 | 26.3 |
| mostly unexplained (+softirq RCU) | 1 | 25.6 | 25.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1034.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 2 | 998.1 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq TIMER 1 |
| 3 | 791.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 297.3 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 5 | 112.5 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 106.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 97.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 77.6 | 2 | softirq NET_RX | softirq NET_RX 58 |
| 9 | 76.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 76.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120069 |
| CAL Function call interrupts | 1016 |
| IWI IRQ work interrupts | 1 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-sibling/http-recv-busy-10000 (median 50.11)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1084.88 | 1034.76 | 1034.29 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.47 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 1050.76 | 1000.65 | 997.09 | mostly unexplained (+timer tick_nohz_handler) | 20.06 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 851.36 | 801.25 | 0.00 |  | 0.00 |  | 801.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 794.67 | 744.56 | 790.74 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 416.41 | 366.30 | 0.00 |  | 20.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 345.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 400.86 | 350.75 | 0.00 |  | 0.00 |  | 350.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 399.60 | 349.49 | 0.00 |  | 0.00 |  | 349.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 386.71 | 336.60 | 0.00 |  | 0.00 |  | 336.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 377.33 | 327.21 | 0.00 |  | 0.00 |  | 327.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 355.76 | 305.65 | 0.00 |  | 0.00 |  | 305.65 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-sibling/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=606488 clocksource=tsc

### Loop counters (r1-sibling/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002050 | 847530 | 3.54 | 51725184 | 17.23 | 862086 | 862086 | 98.4 | 82.3 | true | 0 | 0 | 655.0 | 772926 / 8666 / 5 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8211 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672459200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (139851 of 2899950 in 1-msg reads) | 139851 | 6.26 | 4.29 | 12.46 | 24.38 | 28.95 | 42.14 | 51.45 |

#### Stalls: r1-sibling/md-recv-busy, loop 0 on CPU 2

51725184 iterations, longest 655.0 µs; over 5 / 20 / 100 µs: 772926 / 8666 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5943 | 133642.8 | 655.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1802 | 40169.4 | 424.0 |
| softirq NET_RX | 723 | 18057.5 | 62.2 |
| mostly unexplained (+timer tick_nohz_handler) | 187 | 5044.3 | 548.8 |
| mostly unexplained (+ipi_send_cpu) | 5 | 128.3 | 34.9 |
| mostly unexplained (+softirq RCU) | 3 | 88.9 | 32.6 |
| mostly unexplained (+softirq NET_RX) | 2 | 44.5 | 24.1 |
| timer tick_nohz_handler | 1 | 28.0 | 28.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 655.0 | 2050 | loop work (2050 items) | timer tick_nohz_handler 4 |
| 2 | 548.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 424.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 152.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 105.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 82.1 | 4 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 69.2 | 12 | loop work (12 items) |  |
| 8 | 62.2 | 19 | softirq NET_RX | softirq NET_RX 48 |
| 9 | 61.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 60.2 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60052 |
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 1 |

## r1-sibling/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-sibling/noise, loop 0 on CPU 2

8703746048 iterations, longest 881.7 µs; over 5 / 20 / 100 µs: 56310 / 65 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 55914 | 294532.7 | 603.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 273 | 6304.3 | 881.7 |
| mostly unexplained (+softirq TIMER) | 116 | 788.9 | 8.3 |
| timer tick_nohz_handler | 7 | 55.0 | 8.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 881.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 603.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 536.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 4 | 469.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 457.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 119.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 103.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 100.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 60.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 46.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120034 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-sibling/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1234545 | 1234545 | 1.00 | 47157298 | 38.20 | 785955 | 765379 | 97.3 | 84.6 | true | 0 | 0 | 165.9 | 1090834 / 105 / 3 | 0 | 0 |

#### Stalls: r1-sibling/pp-recv-busy-0, loop 0 on CPU 2

45922752 iterations, longest 165.9 µs; over 5 / 20 / 100 µs: 1090834 / 105 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 63 | 2277.5 | 124.4 |
| mostly unexplained (+softirq NET_RX) | 19 | 792.0 | 165.9 |
| mostly unexplained (+timer tick_nohz_handler) | 8 | 239.3 | 58.4 |
| mostly unexplained (+ipi_send_cpu) | 7 | 180.1 | 38.6 |
| mostly unexplained (+softirq RCU) | 3 | 70.2 | 25.4 |
| softirq NET_RX | 2 | 58.8 | 38.0 |
| mostly unexplained (+softirq TIMER) | 2 | 46.7 | 23.6 |
| timer tick_nohz_handler | 1 | 22.1 | 22.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 165.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 21 |
| 2 | 124.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 124.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 4 | 60.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 60.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 59.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 58.4 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 8 | 57.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 57.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 55.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60190 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-sibling/pp-recv-busy-0 (median 45.56)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1869.68 | 1824.12 | 0.00 |  | 183.77 | unexplained: no kernel event (hypervisor or in-kernel time) | 1640.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 663.55 | 617.99 | 0.00 |  | 574.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 43.90 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 557.98 | 512.42 | 0.00 |  | 505.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 6.68 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 367.16 | 321.60 | 0.00 |  | 0.00 |  | 321.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 281.67 | 236.10 | 0.00 |  | 0.00 |  | 236.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 242.79 | 197.23 | 0.00 |  | 342.79 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 242.40 | 196.83 | 0.00 |  | 0.00 |  | 196.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 210.52 | 164.96 | 0.00 |  | 0.00 |  | 164.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 204.98 | 159.42 | 0.00 |  | 0.00 |  | 159.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 200.64 | 155.07 | 0.00 |  | 71.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 83.41 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-sibling/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1193253 | 1193253 | 1.00 | 103209957 | 86.49 | 860083 | 850139 | 98.8 | 82.6 | true | 1 | 0 | 667.5 | 970446 / 192 / 6 | 0 | 0 |

#### Stalls: r1-sibling/pp-recv-busy-10000, loop 0 on CPU 2

102016704 iterations, longest 667.5 µs; over 5 / 20 / 100 µs: 970446 / 192 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 105 | 5337.9 | 567.5 |
| mostly unexplained (+timer tick_nohz_handler) | 40 | 959.4 | 39.5 |
| mostly unexplained (+softirq NET_RX) | 27 | 845.3 | 64.9 |
| preempted by swapper/2 | 1 | 667.5 | 667.5 |
| mostly unexplained (+ipi_send_cpu) | 8 | 194.4 | 33.4 |
| mostly unexplained (+softirq TIMER) | 5 | 137.6 | 30.7 |
| softirq NET_RX | 4 | 107.5 | 27.6 |
| softirq TIMER | 1 | 56.1 | 56.1 |
| softirq RCU | 1 | 25.4 | 25.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 667.5 | 1 | preempted by swapper/2 | preempted by swapper/2 642, call function single (IPI/vector) 4, page fault 0 |
| 2 | 567.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 541.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 456.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 159.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 151.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 69.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 69.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 64.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 10 | 60.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 119323 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 2 |
| IWI IRQ work interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-sibling/pp-recv-busy-10000 (median 48.86)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 892.80 | 843.95 | 0.00 |  | 0.00 |  | 843.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 882.98 | 834.12 | 0.00 |  | 0.00 |  | 834.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 875.57 | 826.71 | 0.00 |  | 0.00 |  | 826.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 873.80 | 824.94 | 0.00 |  | 0.00 |  | 824.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 873.34 | 824.49 | 0.00 |  | 0.00 |  | 824.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 869.73 | 820.87 | 0.00 |  | 0.00 |  | 820.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 862.43 | 813.57 | 0.00 |  | 0.00 |  | 813.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 708.79 | 659.93 | 667.27 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 9 | 606.38 | 557.52 | 567.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 584.48 | 535.62 | 540.96 | unexplained: no kernel event (hypervisor or in-kernel time) | 62.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-sibling/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1215185 | 1215185 | 1.00 | 3645556 | 3.00 | 60759 | 32222474 | 0.0 | 13.6 | false | 0 | 0 | 943.3 | 49830 / 72 / 3 | 0 | 0 |

#### Stalls: r1-sibling/pp-uring-spin-0, loop 0 on CPU 2

1933348416 iterations, longest 943.3 µs; over 5 / 20 / 100 µs: 49830 / 72 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 61 | 2281.0 | 123.6 |
| mostly unexplained (+timer tick_nohz_handler) | 9 | 1134.3 | 943.3 |
| mostly unexplained (+softirq RCU) | 1 | 21.8 | 21.8 |
| mostly unexplained (+softirq TIMER) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 943.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 2 | 123.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 112.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 78.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 62.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 59.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 56.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 55.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 54.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 52.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60184 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-sibling/pp-uring-spin-0 (median 44.57)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 973.88 | 929.31 | 943.31 | mostly unexplained (+timer tick_nohz_handler) | 60.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 646.26 | 601.69 | 0.00 |  | 0.00 |  | 601.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 540.86 | 496.29 | 62.13 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 434.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 506.40 | 461.83 | 0.00 |  | 0.00 |  | 461.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 461.45 | 416.88 | 333.73 | unexplained: no kernel event (hypervisor or in-kernel time) | 21.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 61.75 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 426.44 | 381.88 | 0.00 |  | 60.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 321.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 389.86 | 345.29 | 0.00 |  | 20.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 325.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 360.17 | 315.60 | 0.00 |  | 0.00 |  | 315.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 356.97 | 312.40 | 0.00 |  | 0.00 |  | 312.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 356.79 | 312.22 | 0.00 |  | 0.00 |  | 312.22 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-sibling/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199445 | 1199445 | 1.00 | 3598336 | 3.00 | 29986 | 26121739 | 0.0 | 6.9 | false | 1 | 0 | 598.2 | 95381 / 150 / 3 | 0 | 0 |

#### Stalls: r1-sibling/pp-uring-spin-10000, loop 0 on CPU 2

3134608640 iterations, longest 598.2 µs; over 5 / 20 / 100 µs: 95381 / 150 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 90 | 3139.4 | 143.9 |
| mostly unexplained (+timer tick_nohz_handler) | 47 | 1255.9 | 173.2 |
| preempted by swapper/2 | 1 | 598.2 | 598.2 |
| mostly unexplained (+softirq TIMER) | 9 | 199.4 | 24.7 |
| mostly unexplained (+softirq RCU) | 2 | 49.2 | 25.8 |
| timer tick_nohz_handler | 1 | 27.1 | 27.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 598.2 | 0 | preempted by swapper/2 | preempted by swapper/2 581, call function single (IPI/vector) 3, page fault 0 |
| 2 | 173.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 3 | 143.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 70.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 66.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 56.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 54.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 53.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 51.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 51.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 111659 |
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 9 |

#### Slowest round trips: r1-sibling/pp-uring-spin-10000 (median 48.77)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1011.41 | 962.63 | 0.00 |  | 0.00 |  | 962.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 831.53 | 782.76 | 0.00 |  | 256.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 525.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 769.94 | 721.17 | 0.00 |  | 0.00 |  | 721.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 651.11 | 602.33 | 0.00 |  | 20.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 582.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 645.87 | 597.10 | 598.16 | preempted by swapper/2 | 46.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 6 | 529.85 | 481.08 | 0.00 |  | 179.78 | unexplained: no kernel event (hypervisor or in-kernel time) | 301.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 496.77 | 447.99 | 0.00 |  | 20.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 427.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 484.27 | 435.49 | 0.00 |  | 0.00 |  | 435.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 459.89 | 411.12 | 0.00 |  | 20.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 390.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 444.37 | 395.59 | 0.00 |  | 62.62 | unexplained: no kernel event (hypervisor or in-kernel time) | 332.97 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-sibling/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1237928 | 1237928 | 1.00 | 1 | 0.00 | 0 | 30791460 | 0.0 | 0.0 | true | 0 | 0 | 72.3 | 217 / 39 / 0 | 0 | 0 |

#### Stalls: r1-sibling/pp-uring-sqpoll-0, loop 0 on CPU 2

1847487616 iterations, longest 72.3 µs; over 5 / 20 / 100 µs: 217 / 39 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 38 | 1452.4 | 72.3 |
| softirq TIMER | 1 | 20.2 | 20.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 72.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 65.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 53.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 50.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 49.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 48.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 48.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 47.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 47.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 46.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1020 |
| RES Rescheduling interrupts | 59 |
| IWI IRQ work interrupts | 9 |
| HVS Hyper-V stimer0 interrupts | 5 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-sibling/pp-uring-sqpoll-0 (median 45.45)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 451.81 | 406.36 | 0.00 |  | 21.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 384.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 446.91 | 401.46 | 0.00 |  | 20.73 | unexplained: no kernel event (hypervisor or in-kernel time) | 380.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 406.72 | 361.27 | 0.00 |  | 0.00 |  | 361.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 374.78 | 329.33 | 0.00 |  | 0.00 |  | 329.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 358.85 | 313.40 | 0.00 |  | 0.00 |  | 313.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 358.68 | 313.23 | 0.00 |  | 20.05 | unexplained: no kernel event (hypervisor or in-kernel time) | 293.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 350.29 | 304.84 | 0.00 |  | 20.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 284.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 348.53 | 303.08 | 0.00 |  | 40.59 | unexplained: no kernel event (hypervisor or in-kernel time) | 262.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 347.17 | 301.72 | 0.00 |  | 30.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 271.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 338.40 | 292.96 | 0.00 |  | 0.00 |  | 292.96 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-sibling/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199389 | 1199389 | 1.00 | 1 | 0.00 | 0 | 24089016 | 0.0 | 0.0 | true | 0 | 0 | 931.3 | 404 / 98 / 3 | 0 | 0 |

#### Stalls: r1-sibling/pp-uring-sqpoll-10000, loop 0 on CPU 2

2890681920 iterations, longest 931.3 µs; over 5 / 20 / 100 µs: 404 / 98 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 98 | 5319.1 | 931.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 931.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 818.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 117.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 98.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 69.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 61.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 61.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 60.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 56.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 56.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1023 |
| RES Rescheduling interrupts | 117 |
| IWI IRQ work interrupts | 4 |
| HVS Hyper-V stimer0 interrupts | 2 |

#### Slowest round trips: r1-sibling/pp-uring-sqpoll-10000 (median 47.85)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 681.63 | 633.78 | 0.00 |  | 0.00 |  | 633.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 602.22 | 554.37 | 0.00 |  | 383.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 171.17 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 543.39 | 495.54 | 0.00 |  | 20.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 475.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 461.35 | 413.50 | 0.00 |  | 42.42 | unexplained: no kernel event (hypervisor or in-kernel time) | 371.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 405.12 | 357.27 | 0.00 |  | 20.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 337.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 404.00 | 356.15 | 0.00 |  | 0.00 |  | 356.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 400.00 | 352.15 | 0.00 |  | 20.60 | unexplained: no kernel event (hypervisor or in-kernel time) | 331.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 383.73 | 335.88 | 0.00 |  | 22.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 313.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 382.03 | 334.18 | 0.00 |  | 0.00 |  | 334.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 380.79 | 332.93 | 0.00 |  | 11.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 321.81 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-8/http-recv-busy-10000, loop 0 on CPU 2

1925445806 iterations, longest 767.3 µs; over 5 / 20 / 100 µs: 141014 / 186 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 162 | 5422.6 | 103.0 |
| mostly unexplained (+softirq TIMER) | 4 | 861.4 | 767.3 |
| mostly unexplained (+softirq NET_RX) | 10 | 440.7 | 125.5 |
| mostly unexplained (+timer tick_nohz_handler) | 8 | 264.5 | 59.8 |
| mostly unexplained (+ipi_send_cpu) | 2 | 47.6 | 24.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 767.3 | 0 | mostly unexplained (+softirq TIMER) | softirq TIMER 1, timer tick_nohz_handler 1 |
| 2 | 125.5 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1, softirq RCU 1, timer tick_nohz_handler 1 |
| 3 | 103.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 82.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 72.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 71.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 69.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 66.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 65.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 62.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120056 |
| CAL Function call interrupts | 1016 |
| IWI IRQ work interrupts | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-8/http-recv-busy-10000 (median 47.97)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 813.67 | 765.70 | 767.29 | mostly unexplained (+softirq TIMER) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq TIMER) |
| 2 | 685.91 | 637.94 | 0.00 |  | 0.00 |  | 637.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 523.76 | 475.79 | 0.00 |  | 0.00 |  | 475.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 464.58 | 416.61 | 0.00 |  | 0.00 |  | 416.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 456.40 | 408.43 | 0.00 |  | 0.00 |  | 408.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 444.05 | 396.08 | 0.00 |  | 20.60 | unexplained: no kernel event (hypervisor or in-kernel time) | 375.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 432.74 | 384.77 | 0.00 |  | 0.00 |  | 384.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 426.70 | 378.72 | 0.00 |  | 0.00 |  | 378.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 365.06 | 317.09 | 0.00 |  | 0.00 |  | 317.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 333.87 | 285.90 | 0.00 |  | 0.00 |  | 285.90 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-8/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=657463 clocksource=tsc

### Loop counters (r1-base-8/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001581 | 934124 | 3.21 | 62584896 | 20.85 | 1043082 | 1043082 | 98.5 | 83.5 | true | 0 | 0 | 1238.4 | 355290 / 5637 / 3 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8212 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672354144 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (219412 of 2900031 in 1-msg reads) | 219412 | 4.79 | 3.27 | 7.62 | 23.27 | 26.34 | 41.76 | 45.83 |

#### Stalls: r1-base-8/md-recv-busy, loop 0 on CPU 2

62584896 iterations, longest 1238.4 µs; over 5 / 20 / 100 µs: 355290 / 5637 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4271 | 92380.8 | 386.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1349 | 29425.4 | 87.4 |
| softirq NET_RX | 13 | 1530.5 | 1238.4 |
| mostly unexplained (+timer tick_nohz_handler) | 1 | 668.8 | 668.8 |
| mostly unexplained (+softirq NET_RX) | 2 | 46.6 | 26.2 |
| mostly unexplained (+softirq RCU) | 1 | 40.2 | 40.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1238.4 | 66 | softirq NET_RX | softirq NET_RX 1174, timer tick_nohz_handler 4 |
| 2 | 668.8 | 31 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 3 | 386.5 | 1550 | loop work (1550 items) |  |
| 4 | 92.7 | 8 | loop work (8 items) |  |
| 5 | 87.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 79.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 73.4 | 18 | loop work (18 items) | softirq NET_RX 3 |
| 8 | 72.6 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 62.6 | 20 | loop work (20 items) | softirq NET_RX 7 |
| 10 | 52.9 | 12 | loop work (12 items) | softirq NET_RX 23 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60037 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## r1-base-8/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-8/noise, loop 0 on CPU 2

12340576256 iterations, longest 714.7 µs; over 5 / 20 / 100 µs: 1509 / 107 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 1012 | 7411.2 | 714.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 379 | 6824.6 | 585.1 |
| mostly unexplained (+softirq TIMER) | 115 | 667.4 | 7.5 |
| timer tick_nohz_handler | 3 | 26.0 | 14.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 714.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 2 | 588.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 3 | 585.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 583.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5 |
| 5 | 444.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 138.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 62.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 54.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 47.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 10 | 45.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120042 |
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-base-8/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1212195 | 1212195 | 1.00 | 307827079 | 253.94 | 5130451 | 5090045 | 0.0 | 37.5 | false | 0 | 0 | 789.3 | 15366 / 100 / 3 | 0 | 0 |

#### Stalls: r1-base-8/pp-epoll-spin-0, loop 0 on CPU 2

305402688 iterations, longest 789.3 µs; over 5 / 20 / 100 µs: 15366 / 100 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 68 | 3244.2 | 789.3 |
| mostly unexplained (+timer tick_nohz_handler) | 19 | 1001.5 | 498.5 |
| mostly unexplained (+softirq TIMER) | 9 | 196.7 | 29.3 |
| mostly unexplained (+softirq RCU) | 4 | 88.0 | 25.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 789.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 498.5 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 3 | 459.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 62.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 60.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 51.4 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 7 | 51.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 13 |
| 8 | 49.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 47.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 46.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60142 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-8/pp-epoll-spin-0 (median 45.73)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 822.35 | 776.62 | 789.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 576.79 | 531.07 | 21.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 509.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 530.56 | 484.83 | 498.37 | mostly unexplained (+timer tick_nohz_handler) | 17.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 526.80 | 481.07 | 458.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 2.03 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 466.58 | 420.85 | 0.00 |  | 0.00 |  | 420.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 445.72 | 400.00 | 0.00 |  | 41.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 358.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 426.53 | 380.80 | 0.00 |  | 41.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 339.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 417.37 | 371.64 | 0.00 |  | 0.00 |  | 371.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 397.66 | 351.93 | 0.00 |  | 60.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 291.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 391.39 | 345.66 | 0.00 |  | 20.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 325.18 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-8/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198597 | 1198597 | 1.00 | 616432843 | 514.30 | 5136940 | 5116964 | 0.0 | 32.6 | false | 1 | 0 | 599.0 | 10660 / 140 / 8 | 0 | 0 |

#### Stalls: r1-base-8/pp-epoll-spin-10000, loop 0 on CPU 2

614035648 iterations, longest 599.0 µs; over 5 / 20 / 100 µs: 10660 / 140 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 126 | 5360.2 | 449.7 |
| preempted by swapper/2 | 1 | 599.0 | 599.0 |
| mostly unexplained (+timer tick_nohz_handler) | 5 | 453.6 | 341.3 |
| mostly unexplained (+softirq TIMER) | 4 | 117.8 | 41.3 |
| mostly unexplained (+softirq RCU) | 2 | 47.7 | 27.5 |
| mostly unexplained (+reschedule (IPI/vector)) | 2 | 42.1 | 21.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 599.0 | 1 | preempted by swapper/2 | preempted by swapper/2 572, call function single (IPI/vector) 2, page fault 0 |
| 2 | 449.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 418.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 341.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 5 | 146.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 122.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 112.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 100.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 97.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 85.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 77612 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 42 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-8/pp-epoll-spin-10000 (median 47.53)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 744.41 | 696.88 | 0.00 |  | 61.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 635.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 691.57 | 644.04 | 0.00 |  | 0.00 |  | 644.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 682.61 | 635.08 | 0.00 |  | 75.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 559.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 675.29 | 627.76 | 0.00 |  | 20.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 607.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 643.86 | 596.34 | 598.66 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 6 | 544.62 | 497.09 | 0.00 |  | 41.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 455.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 521.98 | 474.45 | 449.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 61.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 514.66 | 467.13 | 0.00 |  | 0.00 |  | 467.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 496.85 | 449.32 | 418.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.73 | unexplained: no kernel event (hypervisor or in-kernel time) | 10.40 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 455.57 | 408.04 | 0.00 |  | 20.06 | unexplained: no kernel event (hypervisor or in-kernel time) | 387.98 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-8/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1272634 | 1272634 | 1.00 | 60192571 | 47.30 | 1003210 | 981999 | 97.8 | 85.6 | true | 0 | 0 | 544.5 | 162223 / 99 / 3 | 0 | 0 |

#### Stalls: r1-base-8/pp-recv-busy-0, loop 0 on CPU 2

58919936 iterations, longest 544.5 µs; over 5 / 20 / 100 µs: 162223 / 99 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 72 | 3081.4 | 540.2 |
| mostly unexplained (+softirq NET_RX) | 15 | 1067.1 | 544.5 |
| mostly unexplained (+timer tick_nohz_handler) | 8 | 260.2 | 48.9 |
| mostly unexplained (+softirq RCU) | 1 | 64.8 | 64.8 |
| softirq NET_RX | 1 | 51.7 | 51.7 |
| mostly unexplained (+ipi_send_cpu) | 2 | 43.2 | 21.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 544.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 540.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 145.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 90.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 80.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 67.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 64.8 | 0 | mostly unexplained (+softirq RCU) | softirq RCU 1, timer tick_nohz_handler 1 |
| 8 | 57.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 55.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 53.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60149 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-8/pp-recv-busy-0 (median 44.73)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 777.94 | 733.20 | 0.00 |  | 525.45 | unexplained: no kernel event (hypervisor or in-kernel time) | 207.75 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 586.03 | 541.30 | 544.07 | mostly unexplained (+softirq NET_RX) | 21.22 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 3 | 573.73 | 529.00 | 539.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 565.41 | 520.67 | 0.00 |  | 95.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 425.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 305.72 | 260.99 | 0.00 |  | 0.00 |  | 260.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 287.11 | 242.38 | 0.00 |  | 172.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 70.17 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 268.12 | 223.39 | 0.00 |  | 20.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 203.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 172.28 | 127.55 | 0.00 |  | 126.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 1.13 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 165.51 | 120.77 | 145.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 154.99 | 110.26 | 0.00 |  | 0.00 |  | 110.26 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-8/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194544 | 1194544 | 1.00 | 128235248 | 107.35 | 1068627 | 1058673 | 99.1 | 84.1 | true | 1 | 0 | 1500.3 | 167794 / 191 / 8 | 0 | 0 |

#### Stalls: r1-base-8/pp-recv-busy-10000, loop 0 on CPU 2

127040704 iterations, longest 1500.3 µs; over 5 / 20 / 100 µs: 167794 / 191 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 124 | 4810.9 | 529.8 |
| mostly unexplained (+timer tick_nohz_handler) | 14 | 2968.1 | 1500.3 |
| mostly unexplained (+softirq NET_RX) | 28 | 981.1 | 68.6 |
| preempted by swapper/2 | 1 | 547.5 | 547.5 |
| mostly unexplained (+softirq TIMER) | 9 | 297.7 | 64.9 |
| mostly unexplained (+ipi_send_cpu) | 6 | 142.4 | 30.3 |
| mostly unexplained (+reschedule (IPI/vector)) | 6 | 126.3 | 23.2 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 59.8 | 39.5 |
| softirq NET_RX | 1 | 26.7 | 26.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1500.3 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 2 | 1022.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 3 | 547.5 | 1 | preempted by swapper/2 | preempted by swapper/2 529, call function single (IPI/vector) 2, softirq NET_RX 2, page fault 0 |
| 4 | 529.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 136.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 125.5 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 7 | 106.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 101.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 83.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 78.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 91165 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 29 |
| IWI IRQ work interrupts | 3 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-8/pp-recv-busy-10000 (median 46.94)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1516.72 | 1469.78 | 1499.92 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 1026.47 | 979.53 | 1022.74 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 588.44 | 541.50 | 546.93 | preempted by swapper/2 | 20.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 4 | 564.22 | 517.28 | 529.77 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 538.27 | 491.32 | 0.00 |  | 20.87 | unexplained: no kernel event (hypervisor or in-kernel time) | 470.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 480.60 | 433.66 | 0.00 |  | 0.00 |  | 433.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 439.91 | 392.97 | 0.00 |  | 63.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 329.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 414.95 | 368.00 | 0.00 |  | 20.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 347.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 404.68 | 357.73 | 0.00 |  | 83.43 | unexplained: no kernel event (hypervisor or in-kernel time) | 274.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 395.00 | 348.06 | 0.00 |  | 204.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 143.26 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-8/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1231256 | 1231256 | 1.00 | 3693769 | 3.00 | 61563 | 49811742 | 0.0 | 11.8 | false | 0 | 0 | 306.8 | 14672 / 85 / 1 | 0 | 0 |

#### Stalls: r1-base-8/pp-uring-spin-0, loop 0 on CPU 2

2988704512 iterations, longest 306.8 µs; over 5 / 20 / 100 µs: 14672 / 85 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 76 | 2628.2 | 306.8 |
| mostly unexplained (+softirq TIMER) | 2 | 100.7 | 78.1 |
| mostly unexplained (+timer tick_nohz_handler) | 4 | 95.6 | 30.5 |
| mostly unexplained (+softirq RCU) | 3 | 74.6 | 30.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 306.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 78.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 78.1 | 0 | mostly unexplained (+softirq TIMER) | softirq TIMER 1, timer tick_nohz_handler 1 |
| 4 | 55.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 51.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 50.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 48.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 46.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 45.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 44.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60143 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-8/pp-uring-spin-0 (median 44.52)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 536.43 | 491.91 | 0.00 |  | 20.03 | unexplained: no kernel event (hypervisor or in-kernel time) | 471.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 418.18 | 373.65 | 0.00 |  | 0.00 |  | 373.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 379.37 | 334.84 | 0.00 |  | 0.00 |  | 334.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 366.77 | 322.25 | 0.00 |  | 0.00 |  | 322.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 365.38 | 320.86 | 0.00 |  | 0.00 |  | 320.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 364.73 | 320.21 | 0.00 |  | 0.00 |  | 320.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 333.74 | 289.22 | 306.79 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 323.38 | 278.86 | 0.00 |  | 0.00 |  | 278.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 294.86 | 250.34 | 0.00 |  | 0.00 |  | 250.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 288.45 | 243.93 | 0.00 |  | 20.73 | unexplained: no kernel event (hypervisor or in-kernel time) | 223.21 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-8/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199540 | 1199540 | 1.00 | 3598620 | 3.00 | 29988 | 38941270 | 0.0 | 5.9 | false | 1 | 0 | 1099.8 | 12454 / 130 / 9 | 0 | 0 |

#### Stalls: r1-base-8/pp-uring-spin-10000, loop 0 on CPU 2

4672952384 iterations, longest 1099.8 µs; over 5 / 20 / 100 µs: 12454 / 130 / 9

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 9 | 4539.8 | 1099.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 115 | 3974.5 | 180.6 |
| preempted by swapper/2 | 1 | 648.3 | 648.3 |
| mostly unexplained (+softirq RCU) | 3 | 123.5 | 63.4 |
| mostly unexplained (+softirq TIMER) | 2 | 47.9 | 26.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1099.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5, softirq TIMER 1 |
| 2 | 820.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq TIMER 1 |
| 3 | 802.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 4 | 800.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 5 | 664.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq TIMER 1 |
| 6 | 648.3 | 0 | preempted by swapper/2 | preempted by swapper/2 629, call function single (IPI/vector) 3, page fault 0 |
| 7 | 217.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 8 | 180.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 116.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 98.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 103601 |
| CAL Function call interrupts | 1022 |
| RES Rescheduling interrupts | 17 |
| IWI IRQ work interrupts | 1 |

#### Slowest round trips: r1-base-8/pp-uring-spin-10000 (median 47.76)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1894.53 | 1846.77 | 0.00 |  | 174.78 | unexplained: no kernel event (hypervisor or in-kernel time) | 1671.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1293.96 | 1246.20 | 1280.37 | mostly unexplained (+timer tick_nohz_handler) | 41.30 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 968.48 | 920.72 | 0.00 |  | 776.65 | unexplained: no kernel event (hypervisor or in-kernel time) | 144.07 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 850.06 | 802.30 | 800.23 | mostly unexplained (+timer tick_nohz_handler) | 19.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 5 | 841.21 | 793.45 | 820.16 | mostly unexplained (+timer tick_nohz_handler) | 20.76 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 6 | 731.08 | 683.32 | 685.78 | mostly unexplained (+timer tick_nohz_handler) | 20.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 7 | 695.46 | 647.70 | 648.29 | preempted by swapper/2 | 20.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 8 | 433.50 | 385.74 | 0.00 |  | 149.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 236.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 385.82 | 338.06 | 0.00 |  | 0.00 |  | 338.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 382.22 | 334.46 | 0.00 |  | 41.03 | unexplained: no kernel event (hypervisor or in-kernel time) | 293.43 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-8/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1264680 | 1264680 | 1.00 | 1 | 0.00 | 0 | 48416111 | 0.0 | 0.0 | true | 0 | 0 | 816.7 | 242 / 62 / 5 | 0 | 0 |

#### Stalls: r1-base-8/pp-uring-sqpoll-0, loop 0 on CPU 2

2904966656 iterations, longest 816.7 µs; over 5 / 20 / 100 µs: 242 / 62 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 61 | 4063.6 | 816.7 |
| softirq TIMER | 1 | 38.8 | 38.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 816.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 558.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 412.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 123.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 106.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 95.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 86.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 86.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 72.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 67.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 59 |
| HVS Hyper-V stimer0 interrupts | 6 |
| IWI IRQ work interrupts | 3 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-8/pp-uring-sqpoll-0 (median 44.62)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 830.44 | 785.82 | 816.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 21.05 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 561.55 | 516.93 | 558.31 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 469.14 | 424.52 | 0.00 |  | 0.00 |  | 424.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 461.99 | 417.38 | 412.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 448.75 | 404.13 | 0.00 |  | 60.61 | unexplained: no kernel event (hypervisor or in-kernel time) | 343.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 409.39 | 364.77 | 0.00 |  | 0.00 |  | 364.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 359.57 | 314.95 | 0.00 |  | 0.00 |  | 314.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 347.36 | 302.74 | 0.00 |  | 0.00 |  | 302.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 292.20 | 247.58 | 0.00 |  | 61.57 | unexplained: no kernel event (hypervisor or in-kernel time) | 186.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 291.18 | 246.56 | 0.00 |  | 0.00 |  | 246.56 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-8/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199572 | 1199572 | 1.00 | 1 | 0.00 | 0 | 35434209 | 0.0 | 0.0 | true | 0 | 0 | 1583.1 | 475 / 107 / 7 | 0 | 0 |

#### Stalls: r1-base-8/pp-uring-sqpoll-10000, loop 0 on CPU 2

4252105088 iterations, longest 1583.1 µs; over 5 / 20 / 100 µs: 475 / 107 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 107 | 7864.7 | 1583.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1583.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 806.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 588.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 549.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 540.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 503.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 155.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 70.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 67.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 63.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1024 |
| RES Rescheduling interrupts | 117 |
| IWI IRQ work interrupts | 7 |
| HVS Hyper-V stimer0 interrupts | 3 |

#### Slowest round trips: r1-base-8/pp-uring-sqpoll-10000 (median 47.38)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1612.84 | 1565.46 | 1582.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 40.94 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 695.47 | 648.10 | 0.00 |  | 41.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 606.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 621.74 | 574.36 | 587.83 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 597.72 | 550.34 | 549.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.62 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 594.94 | 547.56 | 0.00 |  | 0.00 |  | 547.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 587.77 | 540.39 | 540.54 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 545.45 | 498.07 | 503.12 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 468.64 | 421.27 | 0.00 |  | 421.83 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 434.50 | 387.12 | 0.00 |  | 20.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 366.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 422.85 | 375.47 | 0.00 |  | 0.00 |  | 375.47 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-busypoll50/http-recv-busy-10000, loop 0 on CPU 2

1937959074 iterations, longest 908.0 µs; over 5 / 20 / 100 µs: 154328 / 214 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 132 | 4459.7 | 87.2 |
| mostly unexplained (+timer tick_nohz_handler) | 18 | 4033.3 | 908.0 |
| mostly unexplained (+softirq NET_RX) | 20 | 925.4 | 373.4 |
| mostly unexplained (+softirq RCU) | 29 | 677.8 | 27.4 |
| mostly unexplained (+softirq TIMER) | 9 | 224.5 | 36.5 |
| mostly unexplained (+ipi_send_cpu) | 5 | 140.5 | 36.9 |
| softirq RCU | 1 | 20.5 | 20.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 908.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4, softirq TIMER 1 |
| 2 | 867.5 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq NET_RX 1 |
| 3 | 866.5 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 4 | 648.5 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 5 | 373.4 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 176 |
| 6 | 228.4 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 7 | 125.7 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq TIMER 1 |
| 8 | 87.2 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 86.7 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 85.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120053 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-busypoll50/http-recv-busy-10000 (median 47.97)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 971.33 | 923.37 | 908.00 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 15.36 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 896.71 | 848.74 | 866.08 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 887.89 | 839.92 | 865.36 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 743.55 | 695.59 | 0.00 |  | 103.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 592.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 686.48 | 638.51 | 647.69 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 6 | 576.92 | 528.95 | 0.00 |  | 0.00 |  | 528.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 538.91 | 490.95 | 0.00 |  | 72.70 | unexplained: no kernel event (hypervisor or in-kernel time) | 418.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 422.25 | 374.29 | 0.00 |  | 389.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 420.90 | 372.93 | 0.00 |  | 70.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 302.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 420.59 | 372.62 | 0.00 |  | 33.63 | unexplained: no kernel event (hypervisor or in-kernel time) | 338.99 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-busypoll50/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=50 wait_us=0 mlockall=true BusyPollRxPackets during run=651655 clocksource=tsc

### Loop counters (r1-busypoll50/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001500 | 940879 | 3.19 | 62881792 | 20.95 | 1048030 | 1048030 | 98.5 | 83.5 | true | 0 | 0 | 409.0 | 359719 / 5360 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8213 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672336000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (225933 of 2899950 in 1-msg reads) | 225933 | 4.67 | 3.30 | 7.23 | 22.93 | 28.61 | 42.43 | 48.29 |

#### Stalls: r1-busypoll50/md-recv-busy, loop 0 on CPU 2

62881792 iterations, longest 409.0 µs; over 5 / 20 / 100 µs: 359719 / 5360 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4113 | 90025.3 | 409.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1075 | 23301.5 | 65.0 |
| mostly unexplained (+timer tick_nohz_handler) | 155 | 4364.4 | 53.9 |
| softirq NET_RX | 11 | 256.8 | 27.4 |
| mostly unexplained (+softirq RCU) | 4 | 128.4 | 47.6 |
| timer tick_nohz_handler | 1 | 21.7 | 21.7 |
| mostly unexplained (+ipi_send_cpu) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 409.0 | 1550 | loop work (1550 items) | timer tick_nohz_handler 2 |
| 2 | 89.8 | 23 | loop work (23 items) | softirq NET_RX 1 |
| 3 | 72.4 | 19 | loop work (19 items) | timer tick_nohz_handler 1 |
| 4 | 65.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 59.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 53.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 7 | 52.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 8 | 50.6 | 15 | loop work (15 items) |  |
| 9 | 50.4 | 4 | loop work (4 items) |  |
| 10 | 47.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60037 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

## r1-busypoll50/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-busypoll50/noise, loop 0 on CPU 2

12280602624 iterations, longest 1045.7 µs; over 5 / 20 / 100 µs: 1619 / 103 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 390 | 7309.3 | 501.9 |
| mostly unexplained (+timer tick_nohz_handler) | 1114 | 6838.9 | 698.9 |
| timer tick_nohz_handler | 2 | 1051.7 | 1045.7 |
| mostly unexplained (+softirq TIMER) | 113 | 662.1 | 8.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1045.7 | 0 | timer tick_nohz_handler | timer tick_nohz_handler 816 |
| 2 | 698.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 3 | 501.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 465.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 233.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 6 | 169.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 154.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 87.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 85.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 83.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120025 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-busypoll50/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1271537 | 1271537 | 1.00 | 60131058 | 47.29 | 1002184 | 980992 | 97.8 | 85.7 | true | 0 | 0 | 985.0 | 147299 / 114 / 5 | 0 | 0 |

#### Stalls: r1-busypoll50/pp-recv-busy-0, loop 0 on CPU 2

58859520 iterations, longest 985.0 µs; over 5 / 20 / 100 µs: 147299 / 114 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 81 | 2899.2 | 165.2 |
| mostly unexplained (+timer tick_nohz_handler) | 9 | 2440.2 | 985.0 |
| mostly unexplained (+softirq NET_RX) | 16 | 714.6 | 173.1 |
| mostly unexplained (+softirq TIMER) | 3 | 110.6 | 50.2 |
| mostly unexplained (+ipi_send_cpu) | 4 | 92.8 | 25.6 |
| mostly unexplained (+softirq RCU) | 1 | 21.9 | 21.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 985.0 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 2 | 909.4 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4, softirq NET_RX 1, softirq TIMER 1 |
| 3 | 388.5 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 4 | 173.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 165.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 87.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 78.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 77.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 66.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 64.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60154 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-busypoll50/pp-recv-busy-0 (median 44.52)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1081.74 | 1037.22 | 1074.26 | mostly unexplained (+timer tick_nohz_handler) | 102.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 1018.96 | 974.44 | 984.66 | mostly unexplained (+timer tick_nohz_handler) | 20.42 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 478.50 | 433.98 | 0.00 |  | 0.00 |  | 433.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 459.15 | 414.63 | 0.00 |  | 0.00 |  | 414.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 417.52 | 373.00 | 388.11 | mostly unexplained (+timer tick_nohz_handler) | 20.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 6 | 320.52 | 276.00 | 0.00 |  | 0.00 |  | 276.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 287.84 | 243.32 | 0.00 |  | 60.69 | unexplained: no kernel event (hypervisor or in-kernel time) | 182.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 260.22 | 215.70 | 0.00 |  | 0.00 |  | 215.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 245.72 | 201.20 | 0.00 |  | 211.22 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 210.56 | 166.04 | 172.86 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |

### Loop counters (r1-busypoll50/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194473 | 1194473 | 1.00 | 127081065 | 106.39 | 1059009 | 1049055 | 99.1 | 84.0 | true | 1 | 0 | 1357.9 | 214914 / 182 / 5 | 0 | 0 |

#### Stalls: r1-busypoll50/pp-recv-busy-10000, loop 0 on CPU 2

125886592 iterations, longest 1357.9 µs; over 5 / 20 / 100 µs: 214914 / 182 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 131 | 7030.4 | 1357.9 |
| mostly unexplained (+timer tick_nohz_handler) | 11 | 2837.8 | 1350.7 |
| mostly unexplained (+softirq NET_RX) | 20 | 753.8 | 85.2 |
| preempted by swapper/2 | 1 | 583.5 | 583.5 |
| mostly unexplained (+ipi_send_cpu) | 7 | 163.2 | 25.9 |
| mostly unexplained (+softirq TIMER) | 5 | 114.0 | 28.9 |
| mostly unexplained (+reschedule (IPI/vector)) | 3 | 63.0 | 21.3 |
| mostly unexplained (+softirq RCU) | 2 | 54.2 | 28.8 |
| softirq NET_RX | 2 | 41.4 | 21.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1357.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 1350.7 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4, softirq TIMER 1 |
| 3 | 1213.2 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4, softirq NET_RX 1 |
| 4 | 1176.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 583.5 | 1 | preempted by swapper/2 | preempted by swapper/2 558, call function (IPI/vector) 3, call function single (IPI/vector) 3, page fault 0 |
| 6 | 91.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 85.2 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 66.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 63.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 58.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 83370 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 37 |
| IWI IRQ work interrupts | 1 |

#### Slowest round trips: r1-busypoll50/pp-recv-busy-10000 (median 46.87)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1375.38 | 1328.51 | 1349.48 | mostly unexplained (+timer tick_nohz_handler) | 20.66 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 1246.15 | 1199.28 | 1212.74 | mostly unexplained (+timer tick_nohz_handler) | 20.88 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 1188.79 | 1141.92 | 1175.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 883.05 | 836.18 | 0.00 |  | 278.99 | unexplained: no kernel event (hypervisor or in-kernel time) | 557.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 622.18 | 575.31 | 582.69 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 6 | 445.85 | 398.98 | 420.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 42.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 425.86 | 378.99 | 0.00 |  | 41.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 337.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 425.22 | 378.35 | 0.00 |  | 0.00 |  | 378.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 409.05 | 362.19 | 0.00 |  | 21.73 | unexplained: no kernel event (hypervisor or in-kernel time) | 340.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 407.75 | 360.88 | 0.00 |  | 127.06 | unexplained: no kernel event (hypervisor or in-kernel time) | 233.82 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-budget64/http-recv-busy-10000, loop 0 on CPU 2

1909607383 iterations, longest 647.8 µs; over 5 / 20 / 100 µs: 117488 / 179 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 150 | 4833.0 | 108.9 |
| mostly unexplained (+softirq NET_RX) | 19 | 757.4 | 70.4 |
| preempted by swapper/2 | 1 | 647.8 | 647.8 |
| mostly unexplained (+timer tick_nohz_handler) | 5 | 124.5 | 35.4 |
| mostly unexplained (+ipi_send_cpu) | 2 | 46.6 | 23.5 |
| loop work | 1 | 34.8 | 34.8 |
| mostly unexplained (+softirq RCU) | 1 | 25.0 | 25.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 647.8 | 2 | preempted by swapper/2 | preempted by swapper/2 618, call function single (IPI/vector) 3, call function (IPI/vector) 1, softirq NET_RX 1 |
| 2 | 108.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 82.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 70.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 67.2 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 19 |
| 6 | 64.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 63.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 62.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 61.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 60.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 114030 |
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 6 |

#### Slowest round trips: r1-budget64/http-recv-busy-10000 (median 48.38)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 689.40 | 641.02 | 646.78 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 2 | 517.70 | 469.32 | 0.00 |  | 0.00 |  | 469.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 479.79 | 431.41 | 0.00 |  | 20.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 411.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 456.24 | 407.87 | 0.00 |  | 0.00 |  | 407.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 426.32 | 377.94 | 0.00 |  | 0.00 |  | 377.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 400.48 | 352.10 | 0.00 |  | 0.00 |  | 352.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 392.90 | 344.52 | 0.00 |  | 43.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 301.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 392.19 | 343.81 | 0.00 |  | 0.00 |  | 343.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 368.74 | 320.36 | 0.00 |  | 20.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 299.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 345.57 | 297.19 | 0.00 |  | 0.00 |  | 297.19 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-budget64/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=699559 clocksource=tsc

### Loop counters (r1-budget64/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001550 | 1022659 | 2.94 | 62871360 | 20.95 | 1047856 | 1047856 | 98.4 | 83.6 | true | 0 | 0 | 402.2 | 329104 / 5868 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8214 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672347200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (261098 of 2900000 in 1-msg reads) | 261098 | 4.64 | 3.24 | 6.86 | 22.89 | 25.80 | 39.31 | 60.03 |

#### Stalls: r1-budget64/md-recv-busy, loop 0 on CPU 2

62871360 iterations, longest 402.2 µs; over 5 / 20 / 100 µs: 329104 / 5868 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4337 | 93730.9 | 402.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1492 | 32380.4 | 59.8 |
| softirq NET_RX | 18 | 425.2 | 30.5 |
| mostly unexplained (+softirq NET_RX) | 9 | 241.0 | 35.2 |
| mostly unexplained (+ipi_send_cpu) | 8 | 229.3 | 46.2 |
| mostly unexplained (+timer tick_nohz_handler) | 4 | 137.3 | 57.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 402.2 | 1550 | loop work (1550 items) | timer tick_nohz_handler 2 |
| 2 | 76.3 | 19 | loop work (19 items) | softirq NET_RX 5 |
| 3 | 61.8 | 4 | loop work (4 items) |  |
| 4 | 59.9 | 9 | loop work (9 items) | softirq NET_RX 1 |
| 5 | 59.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 59.5 | 15 | loop work (15 items) | softirq NET_RX 19, ipi_send_cpu 0 |
| 7 | 59.0 | 17 | loop work (17 items) |  |
| 8 | 57.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 55.7 | 10 | loop work (10 items) | softirq NET_RX 1 |
| 10 | 53.6 | 11 | loop work (11 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60041 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

## r1-budget64/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-budget64/noise, loop 0 on CPU 2

12392660992 iterations, longest 99.0 µs; over 5 / 20 / 100 µs: 1685 / 103 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 433 | 6376.4 | 99.0 |
| mostly unexplained (+timer tick_nohz_handler) | 1148 | 6162.7 | 67.6 |
| mostly unexplained (+softirq TIMER) | 103 | 583.5 | 8.2 |
| timer tick_nohz_handler | 1 | 26.1 | 26.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 99.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 89.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 67.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 4 | 60.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 60.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 54.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 48.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 45.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 43.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120024 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-budget64/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1260487 | 1260487 | 1.00 | 60272840 | 47.82 | 1004547 | 983539 | 97.9 | 85.7 | true | 0 | 0 | 127.8 | 132796 / 107 / 2 | 0 | 0 |

#### Stalls: r1-budget64/pp-recv-busy-0, loop 0 on CPU 2

59012352 iterations, longest 127.8 µs; over 5 / 20 / 100 µs: 132796 / 107 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 62 | 2124.6 | 127.8 |
| mostly unexplained (+softirq NET_RX) | 20 | 708.5 | 114.2 |
| mostly unexplained (+timer tick_nohz_handler) | 18 | 435.1 | 44.0 |
| mostly unexplained (+ipi_send_cpu) | 4 | 88.8 | 24.0 |
| mostly unexplained (+softirq RCU) | 1 | 24.5 | 24.5 |
| softirq NET_RX | 1 | 22.2 | 22.2 |
| mostly unexplained (+softirq TIMER) | 1 | 21.4 | 21.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 127.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 114.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 3 | 69.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 63.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 62.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 54.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 53.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 52.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 50.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 47.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60146 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-budget64/pp-recv-busy-0 (median 44.84)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 983.21 | 938.37 | 0.00 |  | 101.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 837.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 531.77 | 486.93 | 0.00 |  | 50.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 436.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 363.08 | 318.25 | 0.00 |  | 20.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 297.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 339.14 | 294.30 | 0.00 |  | 20.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 273.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 335.23 | 290.39 | 0.00 |  | 0.00 |  | 290.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 327.92 | 283.08 | 0.00 |  | 0.00 |  | 283.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 326.87 | 282.03 | 0.00 |  | 21.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 260.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 302.09 | 257.25 | 0.00 |  | 0.00 |  | 257.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 286.00 | 241.16 | 0.00 |  | 0.00 |  | 241.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 285.63 | 240.79 | 0.00 |  | 0.00 |  | 240.79 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-budget64/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194324 | 1194324 | 1.00 | 128042517 | 107.21 | 1067021 | 1057068 | 99.1 | 84.2 | true | 1 | 0 | 539.1 | 175198 / 136 / 1 | 0 | 0 |

#### Stalls: r1-budget64/pp-recv-busy-10000, loop 0 on CPU 2

126848192 iterations, longest 539.1 µs; over 5 / 20 / 100 µs: 175198 / 136 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 111 | 3580.1 | 65.7 |
| preempted by swapper/2 | 1 | 539.1 | 539.1 |
| mostly unexplained (+softirq NET_RX) | 10 | 265.3 | 53.8 |
| mostly unexplained (+timer tick_nohz_handler) | 7 | 242.4 | 46.3 |
| mostly unexplained (+ipi_send_cpu) | 2 | 47.8 | 26.5 |
| mostly unexplained (+reschedule (IPI/vector)) | 2 | 42.1 | 21.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 40.9 | 40.9 |
| mostly unexplained (+softirq RCU) | 1 | 22.6 | 22.6 |
| softirq NET_RX | 1 | 21.8 | 21.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 539.1 | 1 | preempted by swapper/2 | preempted by swapper/2 523, call function single (IPI/vector) 2, softirq NET_RX 1, page fault 0 |
| 2 | 65.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 64.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 62.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 55.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 53.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 53.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 52.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 51.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 47.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 73693 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 47 |
| IWI IRQ work interrupts | 1 |

#### Slowest round trips: r1-budget64/pp-recv-busy-10000 (median 46.78)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 651.24 | 604.46 | 0.00 |  | 20.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 584.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 581.56 | 534.78 | 538.70 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 3 | 441.22 | 394.44 | 0.00 |  | 13.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 380.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 440.34 | 393.56 | 0.00 |  | 0.00 |  | 393.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 436.07 | 389.29 | 0.00 |  | 4.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 384.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 420.19 | 373.41 | 0.00 |  | 0.00 |  | 373.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 379.44 | 332.66 | 0.00 |  | 42.45 | unexplained: no kernel event (hypervisor or in-kernel time) | 290.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 371.62 | 324.84 | 0.00 |  | 20.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 304.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 361.70 | 314.92 | 0.00 |  | 0.00 |  | 314.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 359.90 | 313.12 | 0.00 |  | 20.22 | unexplained: no kernel event (hypervisor or in-kernel time) | 292.90 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: r1-base-10/http-recv-busy-10000, loop 0 on CPU 2

1880353901 iterations, longest 812.0 µs; over 5 / 20 / 100 µs: 208548 / 183 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 142 | 4909.5 | 132.6 |
| mostly unexplained (+softirq NET_RX) | 33 | 2502.3 | 812.0 |
| mostly unexplained (+softirq TIMER) | 2 | 831.3 | 799.2 |
| mostly unexplained (+timer tick_nohz_handler) | 4 | 143.8 | 48.4 |
| softirq NET_RX | 1 | 39.4 | 39.4 |
| mostly unexplained (+ipi_send_cpu) | 1 | 25.6 | 25.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 812.0 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 799.2 | 2 | mostly unexplained (+softirq TIMER) | softirq TIMER 1, timer tick_nohz_handler 1 |
| 3 | 609.8 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 4 | 132.6 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 110.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 109.2 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 91.7 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 83.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 77.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 70.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120052 |
| CAL Function call interrupts | 1016 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/http-recv-busy-10000 (median 48.53)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 871.67 | 823.14 | 0.00 |  | 380.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 442.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 841.55 | 793.02 | 798.41 | mostly unexplained (+softirq TIMER) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq TIMER) |
| 3 | 816.45 | 767.92 | 810.85 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 4 | 687.40 | 638.87 | 0.00 |  | 0.00 |  | 638.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 628.99 | 580.46 | 608.31 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 6 | 427.29 | 378.75 | 0.00 |  | 20.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 358.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 426.87 | 378.34 | 0.00 |  | 20.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 358.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 386.70 | 338.16 | 0.00 |  | 0.00 |  | 338.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 365.63 | 317.09 | 0.00 |  | 0.00 |  | 317.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 363.66 | 315.12 | 0.00 |  | 0.00 |  | 315.12 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-10/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=680827 clocksource=tsc

### Loop counters (r1-base-10/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001600 | 964858 | 3.11 | 61970048 | 20.65 | 1032834 | 1032834 | 98.4 | 83.5 | true | 0 | 0 | 698.2 | 389011 / 5398 / 4 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8216 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672358400 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (225679 of 2900050 in 1-msg reads) | 225679 | 4.76 | 3.31 | 7.59 | 22.94 | 26.50 | 41.96 | 76.92 |

#### Stalls: r1-base-10/md-recv-busy, loop 0 on CPU 2

61970048 iterations, longest 698.2 µs; over 5 / 20 / 100 µs: 389011 / 5398 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 3973 | 86189.8 | 698.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1392 | 30362.7 | 76.4 |
| mostly unexplained (+timer tick_nohz_handler) | 9 | 977.4 | 507.5 |
| softirq NET_RX | 18 | 413.2 | 29.5 |
| mostly unexplained (+softirq NET_RX) | 3 | 87.2 | 29.7 |
| mostly unexplained (+ipi_send_cpu) | 3 | 65.5 | 23.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 698.2 | 50 | loop work (50 items) |  |
| 2 | 507.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 398.6 | 1550 | loop work (1550 items) |  |
| 4 | 200.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 5 | 76.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 74.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 70.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 61.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 60.3 | 10 | loop work (10 items) |  |
| 10 | 57.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60037 |
| CAL Function call interrupts | 1015 |
| IWI IRQ work interrupts | 1 |
| RES Rescheduling interrupts | 1 |

## r1-base-10/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-10/noise, loop 0 on CPU 2

12420575232 iterations, longest 425.9 µs; over 5 / 20 / 100 µs: 1561 / 122 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 409 | 7052.5 | 425.9 |
| mostly unexplained (+timer tick_nohz_handler) | 1052 | 5834.7 | 71.2 |
| mostly unexplained (+softirq TIMER) | 96 | 536.8 | 7.3 |
| timer tick_nohz_handler | 3 | 25.3 | 12.0 |
| softirq TIMER | 1 | 10.1 | 10.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 425.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 243.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 105.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 97.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 95.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 80.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 71.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 8 | 55.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 55.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 54.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120024 |
| CAL Function call interrupts | 1015 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (r1-base-10/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1172923 | 1172923 | 1.00 | 303865335 | 259.07 | 5064422 | 5025325 | 0.0 | 37.4 | false | 0 | 0 | 118.7 | 18989 / 137 / 1 | 0 | 0 |

#### Stalls: r1-base-10/pp-epoll-spin-0, loop 0 on CPU 2

301519488 iterations, longest 118.7 µs; over 5 / 20 / 100 µs: 18989 / 137 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 101 | 3173.4 | 118.7 |
| mostly unexplained (+timer tick_nohz_handler) | 25 | 620.9 | 62.0 |
| mostly unexplained (+softirq TIMER) | 8 | 191.0 | 32.9 |
| mostly unexplained (+softirq RCU) | 3 | 64.6 | 22.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 118.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 69.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 69.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 68.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 64.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 62.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 18, softirq TIMER 1 |
| 7 | 59.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 59.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 47.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 47.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60149 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-epoll-spin-0 (median 47.32)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 977.38 | 930.05 | 34.65 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.83 | unexplained: no kernel event (hypervisor or in-kernel time) | 874.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 729.41 | 682.09 | 21.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 24.88 | unexplained: no kernel event (hypervisor or in-kernel time) | 636.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 721.23 | 673.91 | 0.00 |  | 0.00 |  | 673.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 694.05 | 646.73 | 0.00 |  | 40.62 | unexplained: no kernel event (hypervisor or in-kernel time) | 606.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 450.87 | 403.55 | 0.00 |  | 480.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 443.76 | 396.44 | 0.00 |  | 20.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 376.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 436.45 | 389.12 | 0.00 |  | 0.00 |  | 389.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 430.24 | 382.92 | 0.00 |  | 0.00 |  | 382.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 406.87 | 359.55 | 0.00 |  | 40.59 | unexplained: no kernel event (hypervisor or in-kernel time) | 318.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 397.44 | 350.12 | 0.00 |  | 0.00 |  | 350.12 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198441 | 1198441 | 1.00 | 607396947 | 506.82 | 5061641 | 5041667 | 0.0 | 32.6 | false | 0 | 0 | 578.4 | 15856 / 143 / 7 | 0 | 0 |

#### Stalls: r1-base-10/pp-epoll-spin-10000, loop 0 on CPU 2

605000064 iterations, longest 578.4 µs; over 5 / 20 / 100 µs: 15856 / 143 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 119 | 5121.5 | 578.4 |
| mostly unexplained (+timer tick_nohz_handler) | 13 | 912.5 | 516.3 |
| mostly unexplained (+softirq TIMER) | 6 | 210.6 | 59.2 |
| mostly unexplained (+softirq RCU) | 5 | 132.8 | 29.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 578.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 534.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 516.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 4 | 161.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 117.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 108.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 104.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 78.0 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 70.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 67.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120111 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-epoll-spin-10000 (median 47.86)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 2863.19 | 2815.33 | 0.00 |  | 186.96 | unexplained: no kernel event (hypervisor or in-kernel time) | 2628.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1002.26 | 954.40 | 0.00 |  | 41.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 912.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 678.66 | 630.80 | 0.00 |  | 19.70 | unexplained: no kernel event (hypervisor or in-kernel time) | 611.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 624.26 | 576.40 | 578.43 | unexplained: no kernel event (hypervisor or in-kernel time) | 59.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 623.73 | 575.87 | 0.00 |  | 0.00 |  | 575.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 560.12 | 512.26 | 534.63 | unexplained: no kernel event (hypervisor or in-kernel time) | 21.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 490.02 | 442.16 | 0.00 |  | 76.22 | unexplained: no kernel event (hypervisor or in-kernel time) | 365.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 485.19 | 437.33 | 0.00 |  | 20.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 417.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 465.09 | 417.23 | 0.00 |  | 0.00 |  | 417.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 434.26 | 386.40 | 0.00 |  | 58.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 328.06 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1420863 | 1420863 | 1.00 | 59495616 | 41.87 | 991594 | 967913 | 97.6 | 85.7 | true | 0 | 0 | 558.2 | 185832 / 103 / 2 | 0 | 0 |

#### Stalls: r1-base-10/pp-recv-busy-0, loop 0 on CPU 2

58074752 iterations, longest 558.2 µs; over 5 / 20 / 100 µs: 185832 / 103 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 70 | 2408.0 | 123.5 |
| mostly unexplained (+softirq NET_RX) | 19 | 1129.5 | 558.2 |
| mostly unexplained (+timer tick_nohz_handler) | 6 | 229.3 | 65.3 |
| softirq NET_RX | 4 | 150.6 | 53.9 |
| mostly unexplained (+ipi_send_cpu) | 2 | 59.1 | 34.1 |
| mostly unexplained (+softirq TIMER) | 2 | 53.5 | 33.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 558.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 123.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 65.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 4 | 64.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 54.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 53.9 | 1 | softirq NET_RX | softirq NET_RX 47 |
| 7 | 52.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 51.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 50.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 50.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60155 |
| CAL Function call interrupts | 1015 |
| IWI IRQ work interrupts | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-recv-busy-0 (median 41.34)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 593.66 | 552.32 | 557.97 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 2 | 400.51 | 359.17 | 0.00 |  | 31.63 | unexplained: no kernel event (hypervisor or in-kernel time) | 327.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 364.85 | 323.50 | 0.00 |  | 0.00 |  | 323.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 345.11 | 303.77 | 0.00 |  | 85.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 218.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 318.55 | 277.21 | 0.00 |  | 0.00 |  | 277.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 268.62 | 227.28 | 0.00 |  | 204.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 23.03 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 180.89 | 139.55 | 0.00 |  | 0.00 |  | 139.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 172.80 | 131.46 | 0.00 |  | 0.00 |  | 131.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 158.09 | 116.75 | 0.00 |  | 0.00 |  | 116.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 145.48 | 104.14 | 0.00 |  | 0.00 |  | 104.14 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194539 | 1194539 | 1.00 | 128116780 | 107.25 | 1067640 | 1057685 | 99.1 | 83.9 | true | 1 | 0 | 735.1 | 153271 / 201 / 9 | 0 | 0 |

#### Stalls: r1-base-10/pp-recv-busy-10000, loop 0 on CPU 2

126922240 iterations, longest 735.1 µs; over 5 / 20 / 100 µs: 153271 / 201 / 9

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 137 | 6362.6 | 583.2 |
| mostly unexplained (+softirq NET_RX) | 29 | 1131.4 | 78.2 |
| preempted by swapper/2 | 1 | 735.1 | 735.1 |
| mostly unexplained (+timer tick_nohz_handler) | 22 | 613.9 | 111.4 |
| mostly unexplained (+softirq TIMER) | 2 | 201.0 | 139.3 |
| mostly unexplained (+reschedule (IPI/vector)) | 5 | 111.4 | 24.7 |
| mostly unexplained (+ipi_send_cpu) | 2 | 47.9 | 26.9 |
| softirq NET_RX | 2 | 43.2 | 22.7 |
| timer tick_nohz_handler | 1 | 36.6 | 36.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 735.1 | 1 | preempted by swapper/2 | preempted by swapper/2 716, call function single (IPI/vector) 2, page fault 0 |
| 2 | 583.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 448.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 432.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 313.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 161.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 139.3 | 0 | mostly unexplained (+softirq TIMER) | softirq TIMER 1, timer tick_nohz_handler 1 |
| 8 | 111.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5 |
| 9 | 103.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 83.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 75293 |
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 45 |

#### Slowest round trips: r1-base-10/pp-recv-busy-10000 (median 47.01)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1870.93 | 1823.92 | 0.00 |  | 271.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 1552.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 779.48 | 732.47 | 734.68 | preempted by swapper/2 | 40.47 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 3 | 750.35 | 703.34 | 0.00 |  | 20.98 | unexplained: no kernel event (hypervisor or in-kernel time) | 682.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 434.49 | 387.48 | 0.00 |  | 0.00 |  | 387.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 404.03 | 357.02 | 0.00 |  | 387.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 392.92 | 345.91 | 0.00 |  | 203.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 142.83 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 386.99 | 339.98 | 0.00 |  | 20.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 319.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 383.65 | 336.64 | 0.00 |  | 0.00 |  | 336.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 372.43 | 325.42 | 0.00 |  | 192.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 133.05 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 345.32 | 298.31 | 0.00 |  | 281.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 17.04 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (r1-base-10/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1167010 | 1167010 | 1.00 | 3501031 | 3.00 | 58351 | 49488450 | 0.0 | 11.2 | false | 0 | 0 | 707.6 | 11086 / 90 / 2 | 0 | 0 |

#### Stalls: r1-base-10/pp-uring-spin-0, loop 0 on CPU 2

2969307008 iterations, longest 707.6 µs; over 5 / 20 / 100 µs: 11086 / 90 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 73 | 3165.0 | 707.6 |
| mostly unexplained (+timer tick_nohz_handler) | 6 | 175.8 | 65.5 |
| mostly unexplained (+softirq RCU) | 5 | 135.4 | 29.9 |
| mostly unexplained (+softirq TIMER) | 6 | 122.2 | 20.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 707.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 136.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 76.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 69.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 65.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 6 | 63.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 56.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 55.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 47.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 44.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60144 |
| CAL Function call interrupts | 1018 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-uring-spin-0 (median 46.87)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 737.08 | 690.21 | 707.56 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 587.73 | 540.85 | 0.00 |  | 60.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 480.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 500.98 | 454.11 | 0.00 |  | 126.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 328.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 460.30 | 413.43 | 0.00 |  | 21.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 392.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 454.01 | 407.14 | 0.00 |  | 0.00 |  | 407.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 438.23 | 391.36 | 0.00 |  | 0.00 |  | 391.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 418.08 | 371.20 | 0.00 |  | 0.00 |  | 371.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 413.05 | 366.18 | 0.00 |  | 77.99 | unexplained: no kernel event (hypervisor or in-kernel time) | 288.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 404.65 | 357.78 | 0.00 |  | 20.59 | unexplained: no kernel event (hypervisor or in-kernel time) | 337.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 400.92 | 354.05 | 0.00 |  | 0.00 |  | 354.05 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199228 | 1199228 | 1.00 | 3597684 | 3.00 | 29981 | 38585388 | 0.0 | 6.0 | false | 1 | 0 | 803.1 | 15799 / 152 / 7 | 0 | 0 |

#### Stalls: r1-base-10/pp-uring-spin-10000, loop 0 on CPU 2

4630246592 iterations, longest 803.1 µs; over 5 / 20 / 100 µs: 15799 / 152 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 130 | 5021.0 | 635.4 |
| mostly unexplained (+timer tick_nohz_handler) | 16 | 1777.2 | 803.1 |
| preempted by swapper/2 | 1 | 645.1 | 645.1 |
| mostly unexplained (+softirq TIMER) | 4 | 86.4 | 22.8 |
| mostly unexplained (+softirq RCU) | 1 | 21.7 | 21.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 803.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 2 | 645.1 | 0 | preempted by swapper/2 | preempted by swapper/2 628, call function single (IPI/vector) 3, page fault 0 |
| 3 | 635.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 616.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 5 | 310.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 123.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 100.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 92.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 83.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 69.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 24 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 94561 |
| CAL Function call interrupts | 1022 |
| RES Rescheduling interrupts | 25 |
| IWI IRQ work interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-uring-spin-10000 (median 48.40)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 867.94 | 819.54 | 802.98 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 16.56 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 713.78 | 665.38 | 0.00 |  | 0.00 |  | 665.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 700.48 | 652.08 | 645.14 | preempted by swapper/2 | 26.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 4 | 671.21 | 622.81 | 0.00 |  | 13.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 609.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 661.93 | 613.53 | 616.38 | mostly unexplained (+timer tick_nohz_handler) | 40.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 6 | 582.67 | 534.27 | 0.00 |  | 20.88 | unexplained: no kernel event (hypervisor or in-kernel time) | 513.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 507.45 | 459.05 | 0.00 |  | 42.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 416.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 491.13 | 442.73 | 0.00 |  | 81.64 | unexplained: no kernel event (hypervisor or in-kernel time) | 361.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 480.26 | 431.87 | 0.00 |  | 0.00 |  | 431.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 470.34 | 421.94 | 0.00 |  | 0.00 |  | 421.94 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1231861 | 1231861 | 1.00 | 1 | 0.00 | 0 | 48306080 | 0.0 | 0.0 | true | 0 | 0 | 947.0 | 455 / 82 / 5 | 0 | 0 |

#### Stalls: r1-base-10/pp-uring-sqpoll-0, loop 0 on CPU 2

2898364800 iterations, longest 947.0 µs; over 5 / 20 / 100 µs: 455 / 82 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 82 | 4621.6 | 947.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 947.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 910.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 146.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 115.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 109.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 88.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 83.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 71.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 58.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 47.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 59 |
| HVS Hyper-V stimer0 interrupts | 3 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: r1-base-10/pp-uring-sqpoll-0 (median 45.92)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 948.26 | 902.34 | 946.62 | unexplained: no kernel event (hypervisor or in-kernel time) | 141.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 929.71 | 883.78 | 910.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 41.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 494.97 | 449.05 | 0.00 |  | 0.00 |  | 449.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 476.65 | 430.73 | 0.00 |  | 98.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 332.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 445.23 | 399.31 | 0.00 |  | 0.00 |  | 399.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 406.80 | 360.88 | 0.00 |  | 40.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 320.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 391.18 | 345.26 | 0.00 |  | 0.00 |  | 345.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 346.93 | 301.01 | 0.00 |  | 80.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 220.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 343.99 | 298.07 | 0.00 |  | 61.89 | unexplained: no kernel event (hypervisor or in-kernel time) | 236.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 338.70 | 292.78 | 0.00 |  | 0.00 |  | 292.78 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199592 | 1199592 | 1.00 | 1 | 0.00 | 0 | 35664778 | 0.0 | 0.0 | true | 0 | 0 | 635.1 | 457 / 116 / 5 | 0 | 0 |

#### Stalls: r1-base-10/pp-uring-sqpoll-10000, loop 0 on CPU 2

4279773312 iterations, longest 635.1 µs; over 5 / 20 / 100 µs: 457 / 116 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 115 | 5257.1 | 635.1 |
| mostly unexplained (+softirq TIMER) | 1 | 36.2 | 36.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 635.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 631.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 107.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 101.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 101.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 99.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 97.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 86.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 84.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 72.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 118 |
| HVS Hyper-V stimer0 interrupts | 6 |
| IWI IRQ work interrupts | 5 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-10/pp-uring-sqpoll-10000 (median 47.59)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1003.38 | 955.79 | 0.00 |  | 40.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 915.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 928.67 | 881.08 | 0.00 |  | 20.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 860.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 696.58 | 648.99 | 630.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 40.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 569.56 | 521.97 | 0.00 |  | 162.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 359.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 493.28 | 445.69 | 0.00 |  | 20.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 425.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 422.01 | 374.43 | 0.00 |  | 20.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 354.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 403.81 | 356.22 | 0.00 |  | 2.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 353.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 383.38 | 335.79 | 0.00 |  | 20.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 315.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 345.42 | 297.83 | 0.00 |  | 62.66 | unexplained: no kernel event (hypervisor or in-kernel time) | 235.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 320.04 | 272.45 | 0.00 |  | 20.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 252.19 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: boot-stock/http-recv-busy-10000, loop 0 on CPU 2

1969066115 iterations, longest 312.7 µs; over 5 / 20 / 100 µs: 130169 / 1315 / 9

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 563 | 14475.2 | 172.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 196 | 7225.8 | 312.7 |
| mostly unexplained (+preempted by kworker/2:1) | 160 | 3923.3 | 49.2 |
| mostly unexplained (+softirq TIMER) | 107 | 2586.5 | 42.4 |
| mostly unexplained (+softirq NET_RX) | 83 | 2364.0 | 107.2 |
| mostly unexplained (+preempted by ksoftirqd/2) | 80 | 2003.5 | 37.3 |
| softirq NET_RX | 32 | 940.8 | 38.0 |
| mostly unexplained (+softirq RCU) | 20 | 452.5 | 29.4 |
| mostly unexplained (+preempted by kworker/2:1H) | 17 | 447.7 | 30.1 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 14 | 330.1 | 28.3 |
| irq nvme0q0 | 2 | 293.3 | 168.1 |
| preempted by kworker/2:1 | 6 | 255.3 | 87.8 |
| softirq RCU | 8 | 228.9 | 33.3 |
| softirq TIMER | 8 | 226.1 | 33.7 |
| timer tick_nohz_handler | 6 | 170.9 | 35.7 |
| mostly unexplained (+softirq SCHED) | 5 | 118.7 | 29.4 |
| loop work | 2 | 59.6 | 32.6 |
| mostly unexplained (+irq mana_q1@pci:7870:00:00.0) | 2 | 42.1 | 21.8 |
| preempted by kworker/2:1H | 1 | 40.6 | 40.6 |
| irq mana_q1@pci:7870:00:00.0 | 1 | 21.7 | 21.7 |
| workqueue mana_gf_stats_work_handler | 1 | 21.6 | 21.6 |
| mostly unexplained (+softirq TASKLET) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 312.7 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 276.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 183.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 176.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 172.5 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 7, ipi_send_cpu 0 |
| 6 | 168.1 | 0 | irq nvme0q0 | irq nvme0q0 158, softirq BLOCK 6, ipi_send_cpu 0 |
| 7 | 125.1 | 1 | irq nvme0q0 | irq nvme0q0 110, softirq BLOCK 7, timer tick_nohz_handler 3, ipi_send_cpu 0 |
| 8 | 112.1 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3, softirq NET_RX 1 |
| 9 | 107.2 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 10 | 92.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120455 |
| CAL Function call interrupts | 1094 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 231 |
| RES Rescheduling interrupts | 58 |
| HYP Hypervisor callback interrupts | 38 |
| 24 HV-PCI-MSIX-c05b:00:00.0 0-edge nvme0q0 | 2 |
| 28 HV-PCI-MSIX-c05b:00:00.0 3-edge nvme0q3 | 1 |

#### Slowest round trips: boot-stock/http-recv-busy-10000 (median 46.72)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 727.34 | 680.62 | 0.00 |  | 279.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 400.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 501.07 | 454.35 | 0.00 |  | 100.70 | unexplained: no kernel event (hypervisor or in-kernel time) | 353.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 416.39 | 369.67 | 0.00 |  | 376.30 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 415.33 | 368.61 | 0.00 |  | 0.00 |  | 368.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 391.43 | 344.71 | 0.00 |  | 0.00 |  | 344.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 389.57 | 342.85 | 0.00 |  | 414.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 387.77 | 341.05 | 347.89 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 378.84 | 332.12 | 346.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 359.59 | 312.87 | 0.00 |  | 0.00 |  | 312.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 353.99 | 307.27 | 0.00 |  | 0.00 |  | 307.27 | neither loop stalled long enough: network, hypervisor or untraced |

## boot-stock/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=718169 clocksource=tsc

### Loop counters (boot-stock/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001500 | 1028855 | 2.92 | 68644288 | 22.87 | 1144071 | 1144071 | 98.5 | 89.0 | true | 0 | 394 | 428.3 | 175750 / 405 / 8 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8212 | 6 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672336000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (271710 of 2899950 in 1-msg reads) | 271710 | 2.84 | 2.95 | 3.83 | 6.11 | 9.64 | 16.19 | 32.51 |

#### Stalls: boot-stock/md-recv-busy, loop 0 on CPU 2

68644288 iterations, longest 428.3 µs; over 5 / 20 / 100 µs: 175750 / 405 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 161 | 4668.8 | 428.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 108 | 3935.6 | 165.8 |
| mostly unexplained (+preempted by kworker/2:1) | 36 | 873.7 | 34.4 |
| irq nvme0q0 | 3 | 663.4 | 403.6 |
| mostly unexplained (+timer tick_nohz_handler) | 22 | 613.9 | 72.1 |
| mostly unexplained (+softirq TIMER) | 25 | 600.7 | 33.2 |
| softirq RCU | 15 | 396.6 | 30.6 |
| preempted by kworker/2:1 | 11 | 319.9 | 40.6 |
| softirq NET_RX | 5 | 155.5 | 54.9 |
| mostly unexplained (+preempted by kworker/2:1H) | 6 | 154.7 | 29.4 |
| softirq TIMER | 4 | 102.0 | 31.5 |
| timer tick_nohz_handler | 3 | 91.2 | 43.8 |
| mostly unexplained (+softirq RCU) | 3 | 74.7 | 27.2 |
| mostly unexplained (+softirq NET_RX) | 1 | 49.7 | 49.7 |
| mostly unexplained (+ipi_send_cpu) | 1 | 23.9 | 23.9 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 1 | 23.0 | 23.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 428.3 | 1451 | loop work (1451 items) |  |
| 2 | 403.6 | 50 | irq nvme0q0 | irq nvme0q0 378, softirq BLOCK 6, ipi_send_cpu 0 |
| 3 | 165.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 132.4 | 0 | irq nvme0q0 | irq nvme0q0 122, softirq BLOCK 7, ipi_send_cpu 0 |
| 5 | 127.4 | 0 | irq nvme0q0 | irq nvme0q0 117, softirq BLOCK 3, timer tick_nohz_handler 3, ipi_send_cpu 0 |
| 6 | 114.0 | 9 | loop work (9 items) | softirq NET_RX 1 |
| 7 | 112.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 110.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 92.0 | 5 | loop work (5 items) |  |
| 10 | 81.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60134 |
| CAL Function call interrupts | 1030 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 144 |
| RES Rescheduling interrupts | 31 |
| HYP Hypervisor callback interrupts | 18 |
| 24 HV-PCI-MSIX-c05b:00:00.0 0-edge nvme0q0 | 3 |

## boot-stock/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: boot-stock/noise, loop 0 on CPU 2

12529037312 iterations, longest 612.9 µs; over 5 / 20 / 100 µs: 34378 / 240 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 24520 | 136128.6 | 412.2 |
| timer tick_nohz_handler | 5505 | 38091.5 | 22.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1444 | 16563.9 | 612.9 |
| mostly unexplained (+softirq TIMER) | 1742 | 16174.4 | 28.8 |
| mostly unexplained (+preempted by kworker/2:1) | 436 | 4876.0 | 27.4 |
| softirq TIMER | 226 | 3241.5 | 35.6 |
| mostly unexplained (+softirq SCHED) | 250 | 1372.2 | 8.4 |
| preempted by kworker/2:1 | 40 | 832.6 | 36.8 |
| workqueue mana_gf_stats_work_handler | 70 | 501.8 | 19.2 |
| softirq NET_RX | 31 | 482.3 | 39.5 |
| mostly unexplained (+preempted by kworker/2:1H) | 21 | 381.6 | 24.8 |
| irq nvme0q0 | 2 | 352.6 | 180.7 |
| mostly unexplained (+preempted by migration/2) | 30 | 289.9 | 10.9 |
| softirq TASKLET | 19 | 231.9 | 13.8 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 17 | 188.0 | 17.3 |
| mostly unexplained (+softirq RCU) | 4 | 61.9 | 16.6 |
| mostly unexplained (+irq mana_q1@pci:7870:00:00.0) | 8 | 54.8 | 12.9 |
| preempted by kworker/2:1H | 2 | 43.6 | 24.3 |
| irq mana_q1@pci:7870:00:00.0 | 2 | 29.9 | 17.1 |
| mostly unexplained (+timer dl_task_timer) | 2 | 14.7 | 8.1 |
| workqueue psi_avgs_work | 1 | 13.9 | 13.9 |
| timer hrtimer_wakeup | 2 | 13.0 | 7.1 |
| mostly unexplained (+timer hrtimer_wakeup) | 1 | 9.4 | 9.4 |
| timer timerfd_tmrproc | 1 | 8.1 | 8.1 |
| mostly unexplained (+softirq NET_RX) | 1 | 7.1 | 7.1 |
| timer dl_task_timer | 1 | 5.9 | 5.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 612.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 412.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 3 | 180.7 | 0 | irq nvme0q0 | irq nvme0q0 173, softirq BLOCK 5, ipi_send_cpu 0 |
| 4 | 171.9 | 0 | irq nvme0q0 | irq nvme0q0 163, softirq BLOCK 6, ipi_send_cpu 0 |
| 5 | 138.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 115.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 82.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 79.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 76.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 67.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120098 |
| CAL Function call interrupts | 1046 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 128 |
| RES Rescheduling interrupts | 60 |
| HYP Hypervisor callback interrupts | 19 |
| 24 HV-PCI-MSIX-c05b:00:00.0 0-edge nvme0q0 | 2 |

### Loop counters (boot-stock/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1218287 | 1218287 | 1.00 | 397115295 | 325.96 | 6618588 | 6577979 | 0.0 | 50.1 | false | 0 | 361 | 175.1 | 426943 / 1470 / 2 | 0 | 0 |

#### Stalls: boot-stock/pp-epoll-spin-0, loop 0 on CPU 2

394678720 iterations, longest 175.1 µs; over 5 / 20 / 100 µs: 426943 / 1470 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| timer tick_nohz_handler | 659 | 13917.9 | 33.9 |
| mostly unexplained (+timer tick_nohz_handler) | 461 | 10230.1 | 56.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 83 | 2644.9 | 110.3 |
| mostly unexplained (+softirq TIMER) | 86 | 2052.2 | 43.5 |
| mostly unexplained (+preempted by kworker/2:1) | 61 | 1591.5 | 44.9 |
| softirq TIMER | 56 | 1467.0 | 42.1 |
| preempted by kworker/2:1 | 18 | 561.4 | 48.0 |
| softirq NET_RX | 13 | 405.7 | 47.4 |
| mostly unexplained (+preempted by kworker/2:1H) | 7 | 227.9 | 43.6 |
| irq nvme0q0 | 1 | 175.1 | 175.1 |
| mostly unexplained (+softirq RCU) | 4 | 123.0 | 37.9 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 5 | 122.2 | 29.4 |
| softirq RCU | 4 | 96.5 | 27.2 |
| softirq SCHED | 4 | 92.8 | 27.9 |
| mostly unexplained (+softirq NET_RX) | 3 | 68.5 | 24.2 |
| workqueue mana_gf_stats_work_handler | 2 | 50.4 | 26.5 |
| mostly unexplained (+preempted by migration/2) | 2 | 44.1 | 23.5 |
| mostly unexplained (+softirq TASKLET) | 1 | 22.9 | 22.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 175.1 | 1 | irq nvme0q0 | irq nvme0q0 164, softirq BLOCK 6, ipi_send_cpu 0 |
| 2 | 110.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 76.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 76.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 67.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 67.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 63.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 56.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 16 |
| 9 | 50.1 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 17 |
| 10 | 49.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60443 |
| CAL Function call interrupts | 1030 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 106 |
| RES Rescheduling interrupts | 31 |
| HYP Hypervisor callback interrupts | 7 |
| 24 HV-PCI-MSIX-c05b:00:00.0 0-edge nvme0q0 | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-stock/pp-epoll-spin-0 (median 46.67)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 407.49 | 360.82 | 0.00 |  | 21.85 | unexplained: no kernel event (hypervisor or in-kernel time) | 338.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 407.29 | 360.61 | 0.00 |  | 0.00 |  | 360.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 385.76 | 339.09 | 0.00 |  | 0.00 |  | 339.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 344.20 | 297.53 | 0.00 |  | 20.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 277.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 335.46 | 288.78 | 0.00 |  | 0.00 |  | 288.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 306.94 | 260.27 | 0.00 |  | 0.00 |  | 260.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 305.62 | 258.95 | 0.00 |  | 0.00 |  | 258.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 302.18 | 255.50 | 0.00 |  | 20.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 234.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 298.93 | 252.25 | 0.00 |  | 0.00 |  | 252.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 287.24 | 240.57 | 0.00 |  | 21.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 219.38 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198718 | 1198718 | 1.00 | 884807164 | 738.13 | 7373393 | 7353414 | 0.0 | 46.0 | false | 1 | 716 | 1516.0 | 123212 / 420 / 8 | 0 | 0 |

#### Stalls: boot-stock/pp-epoll-spin-10000, loop 0 on CPU 2

882409728 iterations, longest 1516.0 µs; over 5 / 20 / 100 µs: 123212 / 420 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 164 | 5501.1 | 198.3 |
| mostly unexplained (+timer tick_nohz_handler) | 48 | 3690.1 | 1516.0 |
| preempted by kworker/2:1 | 57 | 1540.5 | 107.9 |
| mostly unexplained (+preempted by kworker/2:1) | 41 | 967.2 | 32.5 |
| softirq TIMER | 33 | 867.6 | 30.5 |
| mostly unexplained (+softirq TIMER) | 28 | 641.9 | 32.1 |
| softirq NET_RX | 21 | 589.1 | 38.6 |
| preempted by swapper/2 | 1 | 411.1 | 411.1 |
| mostly unexplained (+preempted by kworker/2:1H) | 10 | 248.8 | 36.1 |
| irq nvme0q0 | 2 | 239.9 | 123.4 |
| softirq RCU | 6 | 164.3 | 30.3 |
| timer tick_nohz_handler | 4 | 86.2 | 22.6 |
| mostly unexplained (+softirq RCU) | 3 | 75.3 | 28.1 |
| preempted by kworker/2:1H | 2 | 65.6 | 37.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1516.0 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 9, ipi_send_cpu 0 |
| 2 | 898.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 6 |
| 3 | 411.1 | 1 | preempted by swapper/2 | preempted by swapper/2 399, call function single (IPI/vector) 2, page fault 0 |
| 4 | 198.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 123.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 123.4 | 0 | irq nvme0q0 | irq nvme0q0 113, softirq BLOCK 7, ipi_send_cpu 0 |
| 7 | 116.5 | 0 | irq nvme0q0 | irq nvme0q0 108, softirq BLOCK 5, ipi_send_cpu 0 |
| 8 | 107.9 | 1 | preempted by kworker/2:1 | preempted by kworker/2:1 93, workqueue key_garbage_collector 92, softirq TIMER 5, timer tick_nohz_handler 2 |
| 9 | 65.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 61.3 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 25 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120572 |
| CAL Function call interrupts | 539 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 234 |
| HYP Hypervisor callback interrupts | 38 |
| RES Rescheduling interrupts | 15 |
| IWI IRQ work interrupts | 9 |
| 24 HV-PCI-MSIX-c05b:00:00.0 0-edge nvme0q0 | 2 |

#### Slowest round trips: boot-stock/pp-epoll-spin-10000 (median 46.57)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 2139.72 | 2093.16 | 0.00 |  | 183.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 1909.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1531.05 | 1484.49 | 1515.57 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 926.62 | 880.05 | 898.68 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 772.67 | 726.10 | 0.00 |  | 20.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 706.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 457.41 | 410.85 | 410.77 | preempted by swapper/2 | 81.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 6 | 444.91 | 398.34 | 0.00 |  | 21.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 377.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 390.41 | 343.84 | 0.00 |  | 0.00 |  | 343.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 386.04 | 339.47 | 0.00 |  | 0.00 |  | 339.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 382.12 | 335.55 | 0.00 |  | 81.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 254.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 365.08 | 318.51 | 0.00 |  | 0.00 |  | 318.51 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1261674 | 1261674 | 1.00 | 66858603 | 52.99 | 1114310 | 1093282 | 98.1 | 90.6 | true | 0 | 362 | 166.6 | 67823 / 301 / 2 | 0 | 0 |

#### Stalls: boot-stock/pp-recv-busy-0, loop 0 on CPU 2

65596928 iterations, longest 166.6 µs; over 5 / 20 / 100 µs: 67823 / 301 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 82 | 2610.6 | 77.3 |
| mostly unexplained (+softirq NET_RX) | 41 | 1517.9 | 82.7 |
| mostly unexplained (+timer tick_nohz_handler) | 46 | 1296.1 | 68.9 |
| mostly unexplained (+preempted by kworker/2:1) | 51 | 1236.6 | 47.1 |
| mostly unexplained (+softirq TIMER) | 33 | 791.0 | 38.9 |
| preempted by kworker/2:1 | 12 | 481.6 | 166.6 |
| softirq NET_RX | 11 | 273.4 | 30.1 |
| mostly unexplained (+preempted by ksoftirqd/2) | 8 | 231.6 | 48.1 |
| softirq RCU | 6 | 166.5 | 32.4 |
| mostly unexplained (+preempted by kworker/2:1H) | 5 | 160.4 | 47.8 |
| irq nvme0q0 | 1 | 155.8 | 155.8 |
| mostly unexplained (+softirq RCU) | 3 | 80.7 | 28.2 |
| mostly unexplained (+softirq TASKLET) | 1 | 22.2 | 22.2 |
| mostly unexplained (+ipi_send_cpu) | 1 | 20.1 | 20.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 166.6 | 1 | preempted by kworker/2:1 | preempted by kworker/2:1 148, workqueue key_garbage_collector 145, softirq RCU 4, softirq TIMER 4 |
| 2 | 155.8 | 0 | irq nvme0q0 | irq nvme0q0 143, softirq BLOCK 5, timer tick_nohz_handler 2, ipi_send_cpu 0 |
| 3 | 82.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 4 | 79.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 78.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 77.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 70.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2 |
| 8 | 68.9 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 12 |
| 9 | 68.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 63.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60609 |
| CAL Function call interrupts | 1037 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 89 |
| RES Rescheduling interrupts | 29 |
| HYP Hypervisor callback interrupts | 18 |
| 24 HV-PCI-MSIX-c05b:00:00.0 0-edge nvme0q0 | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-stock/pp-recv-busy-0 (median 45.55)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 724.25 | 678.70 | 0.00 |  | 42.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 636.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 462.00 | 416.45 | 0.00 |  | 20.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 395.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 359.10 | 313.55 | 0.00 |  | 7.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 306.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 354.55 | 308.99 | 0.00 |  | 40.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 268.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 353.46 | 307.90 | 0.00 |  | 20.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 287.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 352.81 | 307.25 | 0.00 |  | 0.00 |  | 307.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 329.75 | 284.19 | 0.00 |  | 41.77 | unexplained: no kernel event (hypervisor or in-kernel time) | 242.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 314.30 | 268.74 | 0.00 |  | 20.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 248.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 309.87 | 264.31 | 0.00 |  | 36.93 | unexplained: no kernel event (hypervisor or in-kernel time) | 227.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 307.14 | 261.58 | 0.00 |  | 0.00 |  | 261.58 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194471 | 1194471 | 1.00 | 136824488 | 114.55 | 1140204 | 1130250 | 99.1 | 89.7 | true | 1 | 761 | 422.4 | 134077 / 728 / 2 | 0 | 0 |

#### Stalls: boot-stock/pp-recv-busy-10000, loop 0 on CPU 2

135630016 iterations, longest 422.4 µs; over 5 / 20 / 100 µs: 134077 / 728 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 551 | 15722.8 | 422.4 |
| softirq TASKLET | 49 | 1490.1 | 60.3 |
| mostly unexplained (+timer tick_nohz_handler) | 35 | 860.3 | 31.0 |
| mostly unexplained (+irq mana_q1@pci:7870:00:00.0) | 13 | 388.7 | 68.8 |
| mostly unexplained (+preempted by kworker/2:1) | 16 | 380.7 | 30.2 |
| mostly unexplained (+softirq NET_RX) | 14 | 325.2 | 31.4 |
| softirq NET_RX | 11 | 246.2 | 26.1 |
| softirq TIMER | 7 | 190.7 | 34.6 |
| mostly unexplained (+softirq TIMER) | 7 | 154.5 | 25.8 |
| mostly unexplained (+softirq TASKLET) | 3 | 97.8 | 45.1 |
| mostly unexplained (+preempted by ksoftirqd/2) | 4 | 91.8 | 25.5 |
| preempted by kworker/2:1 | 3 | 89.0 | 33.9 |
| timer tick_nohz_handler | 4 | 85.8 | 22.8 |
| softirq RCU | 3 | 85.1 | 31.9 |
| mostly unexplained (+preempted by kworker/2:1H) | 3 | 83.3 | 32.0 |
| mostly unexplained (+softirq RCU) | 1 | 27.3 | 27.3 |
| preempted by kworker/2:1H | 1 | 27.2 | 27.2 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 1 | 27.1 | 27.1 |
| mostly unexplained (+ipi_send_cpu) | 1 | 24.3 | 24.3 |
| irq mana_q1@pci:7870:00:00.0 | 1 | 23.8 | 23.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 422.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 185.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 73.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 68.8 | 1 | mostly unexplained (+irq mana_q1@pci:7870:00:00.0) | irq mana_q1@pci:7870:00:00.0 1, softirq NET_RX 1 |
| 5 | 67.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 66.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 64.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 63.9 | 1 | mostly unexplained (+irq mana_q1@pci:7870:00:00.0) | irq mana_q1@pci:7870:00:00.0 1, softirq NET_RX 1 |
| 9 | 63.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 62.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 1196495 |
| HVS Hyper-V stimer0 interrupts | 120525 |
| CAL Function call interrupts | 531 |
| RES Rescheduling interrupts | 62 |
| HYP Hypervisor callback interrupts | 40 |
| IWI IRQ work interrupts | 9 |
| 24 HV-PCI-MSIX-c05b:00:00.0 0-edge nvme0q0 | 1 |
| 28 HV-PCI-MSIX-c05b:00:00.0 3-edge nvme0q3 | 1 |

#### Slowest round trips: boot-stock/pp-recv-busy-10000 (median 44.06)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 2465.00 | 2420.93 | 0.00 |  | 151.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 2269.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 812.76 | 768.70 | 0.00 |  | 88.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 680.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 721.63 | 677.56 | 0.00 |  | 0.00 |  | 677.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 467.45 | 423.39 | 422.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 1.38 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 409.33 | 365.26 | 0.00 |  | 40.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 324.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 402.32 | 358.25 | 0.00 |  | 42.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 316.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 385.58 | 341.52 | 0.00 |  | 21.36 | unexplained: no kernel event (hypervisor or in-kernel time) | 320.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 345.31 | 301.25 | 0.00 |  | 20.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 281.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 337.20 | 293.13 | 0.00 |  | 31.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 261.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 337.07 | 293.01 | 0.00 |  | 0.00 |  | 293.01 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1214775 | 1214775 | 1.00 | 3644326 | 3.00 | 60739 | 51452109 | 0.0 | 8.5 | false | 0 | 357 | 205.1 | 60514 / 175 / 1 | 0 | 0 |

#### Stalls: boot-stock/pp-uring-spin-0, loop 0 on CPU 2

3087126528 iterations, longest 205.1 µs; over 5 / 20 / 100 µs: 60514 / 175 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 82 | 2706.7 | 66.7 |
| mostly unexplained (+timer tick_nohz_handler) | 17 | 482.4 | 69.3 |
| preempted by kworker/2:1 | 16 | 440.0 | 40.9 |
| mostly unexplained (+preempted by kworker/2:1) | 19 | 434.5 | 27.6 |
| softirq NET_RX | 10 | 326.0 | 54.4 |
| softirq TIMER | 10 | 255.7 | 28.5 |
| irq nvme0q0 | 1 | 205.1 | 205.1 |
| mostly unexplained (+softirq TIMER) | 9 | 199.7 | 24.6 |
| mostly unexplained (+preempted by kworker/2:1H) | 5 | 128.1 | 28.6 |
| timer tick_nohz_handler | 3 | 89.7 | 47.8 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 2 | 40.8 | 20.6 |
| mostly unexplained (+softirq SCHED) | 1 | 24.4 | 24.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 205.1 | 0 | irq nvme0q0 | irq nvme0q0 196, softirq BLOCK 6, ipi_send_cpu 0 |
| 2 | 69.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5 |
| 3 | 66.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 61.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 57.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 56.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 56.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 54.4 | 0 | softirq NET_RX | softirq NET_RX 43, timer tick_nohz_handler 5, softirq TIMER 4, irq mana_q1@pci:7870:00:00.0 2 |
| 9 | 53.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 52.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60472 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 88 |
| CAL Function call interrupts | 35 |
| RES Rescheduling interrupts | 29 |
| HYP Hypervisor callback interrupts | 16 |
| IWI IRQ work interrupts | 9 |
| 24 HV-PCI-MSIX-c05b:00:00.0 0-edge nvme0q0 | 1 |

#### Slowest round trips: boot-stock/pp-uring-spin-0 (median 46.36)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 585.35 | 538.99 | 0.00 |  | 129.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 409.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 480.24 | 433.88 | 0.00 |  | 462.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 398.86 | 352.50 | 0.00 |  | 0.00 |  | 352.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 385.22 | 338.86 | 0.00 |  | 0.00 |  | 338.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 379.37 | 333.01 | 0.00 |  | 20.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 312.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 362.92 | 316.56 | 0.00 |  | 142.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 174.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 360.46 | 314.10 | 0.00 |  | 0.00 |  | 314.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 357.07 | 310.72 | 0.00 |  | 60.56 | unexplained: no kernel event (hypervisor or in-kernel time) | 250.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 346.04 | 299.69 | 0.00 |  | 0.00 |  | 299.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 335.82 | 289.46 | 0.00 |  | 20.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 269.44 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199640 | 1199640 | 1.00 | 3598920 | 3.00 | 29991 | 38991350 | 0.0 | 4.5 | false | 1 | 760 | 616.5 | 118572 / 335 / 7 | 0 | 0 |

#### Stalls: boot-stock/pp-uring-spin-10000, loop 0 on CPU 2

4678961984 iterations, longest 616.5 µs; over 5 / 20 / 100 µs: 118572 / 335 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 174 | 6137.5 | 504.8 |
| mostly unexplained (+timer tick_nohz_handler) | 24 | 1397.1 | 616.5 |
| mostly unexplained (+preempted by kworker/2:1) | 46 | 1116.2 | 39.2 |
| preempted by kworker/2:1 | 25 | 851.5 | 114.7 |
| softirq NET_RX | 18 | 523.4 | 42.9 |
| preempted by swapper/2 | 1 | 466.7 | 466.7 |
| softirq TIMER | 17 | 428.5 | 29.5 |
| mostly unexplained (+softirq TIMER) | 19 | 419.3 | 25.8 |
| irq nvme0q0 | 1 | 190.3 | 190.3 |
| mostly unexplained (+preempted by kworker/2:1H) | 7 | 166.5 | 26.1 |
| softirq RCU | 1 | 26.2 | 26.2 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 1 | 22.1 | 22.1 |
| mostly unexplained (+softirq RCU) | 1 | 20.4 | 20.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 616.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 7 |
| 2 | 504.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 466.7 | 0 | preempted by swapper/2 | preempted by swapper/2 454, call function single (IPI/vector) 1, page fault 0 |
| 4 | 190.3 | 0 | irq nvme0q0 | irq nvme0q0 180, softirq BLOCK 6, ipi_send_cpu 0 |
| 5 | 170.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 117.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 7 | 114.7 | 0 | preempted by kworker/2:1 | preempted by kworker/2:1 98, workqueue key_garbage_collector 93, softirq TIMER 7, workqueue psi_avgs_work 3 |
| 8 | 72.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 68.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 6 |
| 10 | 67.5 | 0 | preempted by kworker/2:1 | preempted by kworker/2:1 55, workqueue vmstat_update 53, softirq TIMER 2, timer tick_nohz_handler 2 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120433 |
| CAL Function call interrupts | 537 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 258 |
| RES Rescheduling interrupts | 63 |
| HYP Hypervisor callback interrupts | 21 |
| 24 HV-PCI-MSIX-c05b:00:00.0 0-edge nvme0q0 | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-stock/pp-uring-spin-10000 (median 46.30)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 564.97 | 518.66 | 0.00 |  | 20.89 | unexplained: no kernel event (hypervisor or in-kernel time) | 497.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 555.12 | 508.81 | 466.65 | preempted by swapper/2 | 0.00 |  | 42.16 | client: preempted by swapper/2 |
| 3 | 459.57 | 413.27 | 0.00 |  | 474.15 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 420.79 | 374.49 | 0.00 |  | 0.00 |  | 374.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 418.34 | 372.04 | 0.00 |  | 20.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 351.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 409.17 | 362.86 | 0.00 |  | 316.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 46.86 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 403.62 | 357.31 | 0.00 |  | 0.00 |  | 357.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 372.80 | 326.50 | 0.00 |  | 238.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 88.29 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 346.10 | 299.79 | 0.00 |  | 20.03 | unexplained: no kernel event (hypervisor or in-kernel time) | 279.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 343.93 | 297.63 | 0.00 |  | 0.00 |  | 297.63 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1250871 | 1250871 | 1.00 | 1 | 0.00 | 0 | 46771232 | 0.0 | 0.0 | true | 0 | 385 | 579.7 | 66343 / 415 / 4 | 0 | 0 |

#### Stalls: boot-stock/pp-uring-sqpoll-0, loop 0 on CPU 2

2806273920 iterations, longest 579.7 µs; over 5 / 20 / 100 µs: 66343 / 415 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 185 | 4562.6 | 38.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 86 | 3268.0 | 579.7 |
| mostly unexplained (+timer tick_nohz_handler) | 34 | 1447.5 | 444.4 |
| mostly unexplained (+softirq TIMER) | 41 | 1026.8 | 37.5 |
| mostly unexplained (+preempted by kworker/2:1) | 35 | 885.2 | 44.4 |
| preempted by kworker/2:1 | 17 | 494.4 | 37.2 |
| irq nvme0q0 | 1 | 161.8 | 161.8 |
| mostly unexplained (+preempted by kworker/2:1H) | 4 | 97.5 | 26.9 |
| softirq NET_RX | 2 | 73.5 | 39.1 |
| preempted by kworker/2:1H | 2 | 67.7 | 37.7 |
| softirq RCU | 2 | 53.1 | 27.3 |
| mostly unexplained (+softirq SCHED) | 2 | 51.1 | 26.0 |
| workqueue mana_gf_stats_work_handler | 2 | 49.7 | 29.6 |
| timer tick_nohz_handler | 2 | 44.8 | 24.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 579.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 444.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5 |
| 3 | 161.8 | 0 | irq nvme0q0 | irq nvme0q0 152, softirq BLOCK 7, ipi_send_cpu 0 |
| 4 | 121.8 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 5 | 65.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 14 |
| 6 | 65.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 7 | 60.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 57.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 51.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 51.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60280 |
| CAL Function call interrupts | 1131 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 79 |
| RES Rescheduling interrupts | 35 |
| HYP Hypervisor callback interrupts | 23 |
| 24 HV-PCI-MSIX-c05b:00:00.0 0-edge nvme0q0 | 1 |

#### Slowest round trips: boot-stock/pp-uring-sqpoll-0 (median 45.74)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 585.21 | 539.46 | 578.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.12 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 544.06 | 498.32 | 0.00 |  | 43.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 455.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 532.73 | 486.99 | 0.00 |  | 0.00 |  | 486.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 483.23 | 437.49 | 0.00 |  | 20.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 417.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 459.87 | 414.12 | 444.40 | mostly unexplained (+timer tick_nohz_handler) | 40.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 6 | 348.66 | 302.91 | 0.00 |  | 51.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 251.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 344.54 | 298.80 | 0.00 |  | 0.00 |  | 298.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 299.40 | 253.66 | 0.00 |  | 0.00 |  | 253.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 294.55 | 248.81 | 0.00 |  | 20.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 228.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 293.72 | 247.98 | 0.00 |  | 60.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 187.79 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199380 | 1199380 | 1.00 | 1 | 0.00 | 0 | 27213495 | 0.0 | 0.0 | true | 0 | 570 | 1135.5 | 886764 / 6040 / 9 | 0 | 0 |

#### Stalls: boot-stock/pp-uring-sqpoll-10000, loop 0 on CPU 2

3265619392 iterations, longest 1135.5 µs; over 5 / 20 / 100 µs: 886764 / 6040 / 9

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 2913 | 70029.8 | 1135.5 |
| timer tick_nohz_handler | 1249 | 26426.8 | 36.6 |
| softirq TIMER | 983 | 24917.7 | 38.0 |
| mostly unexplained (+softirq TIMER) | 547 | 15304.9 | 45.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 141 | 5220.0 | 258.4 |
| mostly unexplained (+preempted by kworker/2:1) | 109 | 3065.0 | 59.8 |
| preempted by kworker/2:1 | 24 | 744.7 | 46.1 |
| softirq NET_RX | 21 | 738.2 | 53.8 |
| irq nvme0q0 | 2 | 427.2 | 250.1 |
| softirq SCHED | 10 | 237.9 | 27.6 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 9 | 232.6 | 32.3 |
| mostly unexplained (+softirq SCHED) | 7 | 187.7 | 34.4 |
| mostly unexplained (+preempted by migration/2) | 5 | 116.8 | 25.2 |
| mostly unexplained (+preempted by kworker/2:1H) | 4 | 104.7 | 28.2 |
| mostly unexplained (+softirq NET_RX) | 4 | 98.0 | 26.2 |
| softirq TASKLET | 3 | 80.4 | 29.9 |
| mostly unexplained (+softirq TASKLET) | 3 | 65.6 | 22.5 |
| workqueue mana_gf_stats_work_handler | 2 | 46.9 | 26.6 |
| preempted by kworker/2:1H | 1 | 34.4 | 34.4 |
| preempted by migration/2 | 1 | 28.1 | 28.1 |
| mostly unexplained (+irq mana_q1@pci:7870:00:00.0) | 1 | 21.9 | 21.9 |
| mostly unexplained (+softirq RCU) | 1 | 21.6 | 21.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1135.5 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5 |
| 2 | 767.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5 |
| 3 | 725.4 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 4 | 540.1 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 8, ipi_send_cpu 0 |
| 5 | 258.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 250.1 | 1 | irq nvme0q0 | irq nvme0q0 240, softirq BLOCK 6, ipi_send_cpu 0 |
| 7 | 177.0 | 0 | irq nvme0q0 | irq nvme0q0 168, softirq BLOCK 5, ipi_send_cpu 0 |
| 8 | 152.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 126.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 87.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120561 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 204 |
| RES Rescheduling interrupts | 60 |
| HYP Hypervisor callback interrupts | 30 |
| CAL Function call interrupts | 26 |
| IWI IRQ work interrupts | 9 |
| 24 HV-PCI-MSIX-c05b:00:00.0 0-edge nvme0q0 | 2 |
| 28 HV-PCI-MSIX-c05b:00:00.0 3-edge nvme0q3 | 2 |

#### Slowest round trips: boot-stock/pp-uring-sqpoll-10000 (median 45.31)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1138.77 | 1093.46 | 1134.93 | mostly unexplained (+timer tick_nohz_handler) | 104.59 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 544.18 | 498.88 | 539.76 | mostly unexplained (+timer tick_nohz_handler) | 20.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 468.16 | 422.86 | 0.00 |  | 20.45 | unexplained: no kernel event (hypervisor or in-kernel time) | 402.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 459.66 | 414.35 | 0.00 |  | 0.00 |  | 414.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 419.80 | 374.50 | 0.00 |  | 41.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 333.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 405.29 | 359.98 | 0.00 |  | 42.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 317.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 403.83 | 358.52 | 0.00 |  | 0.00 |  | 358.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 393.34 | 348.03 | 0.00 |  | 244.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 103.53 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 376.90 | 331.59 | 0.00 |  | 20.55 | unexplained: no kernel event (hypervisor or in-kernel time) | 311.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 366.59 | 321.28 | 0.00 |  | 0.00 |  | 321.28 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: boot-isolcpus/http-recv-busy-10000, loop 0 on CPU 2

1895194319 iterations, longest 911.5 µs; over 5 / 20 / 100 µs: 234201 / 433 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 191 | 6161.2 | 183.0 |
| mostly unexplained (+timer tick_nohz_handler) | 51 | 3570.5 | 911.5 |
| mostly unexplained (+preempted by kworker/2:2) | 89 | 2095.0 | 38.1 |
| mostly unexplained (+softirq NET_RX) | 30 | 1641.0 | 484.1 |
| mostly unexplained (+softirq TIMER) | 30 | 695.6 | 30.5 |
| preempted by kworker/2:2 | 15 | 515.6 | 83.7 |
| mostly unexplained (+preempted by kworker/2:1H) | 9 | 231.1 | 31.9 |
| timer tick_nohz_handler | 5 | 162.5 | 36.7 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 4 | 85.1 | 23.2 |
| softirq NET_RX | 1 | 65.6 | 65.6 |
| preempted by kworker/2:1H | 2 | 57.8 | 36.4 |
| softirq TIMER | 2 | 47.5 | 26.7 |
| mostly unexplained (+ipi_send_cpu) | 2 | 47.1 | 25.0 |
| loop work | 1 | 29.3 | 29.3 |
| workqueue mana_gf_stats_work_handler | 1 | 22.0 | 22.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 911.5 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 9, softirq NET_RX 1 |
| 2 | 687.3 | 2 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 7 |
| 3 | 502.9 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 7 |
| 4 | 484.1 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 183.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 107.9 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 83.7 | 2 | preempted by kworker/2:2 | preempted by kworker/2:2 71, workqueue key_garbage_collector 68, softirq TIMER 2, timer tick_nohz_handler 2 |
| 8 | 78.8 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5 |
| 9 | 77.3 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 16, softirq NET_RX 1 |
| 10 | 72.8 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120090 |
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 54 |
| 28 HV-PCI-MSIX-c05b:00:00.0 3-edge nvme1q3 | 1 |

#### Slowest round trips: boot-isolcpus/http-recv-busy-10000 (median 48.12)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 944.93 | 896.81 | 909.56 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 719.95 | 671.83 | 685.94 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 569.61 | 521.49 | 0.00 |  | 0.00 |  | 521.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 513.94 | 465.81 | 482.83 | mostly unexplained (+softirq NET_RX) | 20.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 5 | 488.61 | 440.49 | 0.00 |  | 0.00 |  | 440.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 451.74 | 403.61 | 0.00 |  | 20.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 383.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 443.21 | 395.08 | 0.00 |  | 0.00 |  | 395.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 423.39 | 375.27 | 0.00 |  | 0.00 |  | 375.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 418.27 | 370.15 | 0.00 |  | 0.00 |  | 370.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 398.11 | 349.99 | 0.00 |  | 0.00 |  | 349.99 | neither loop stalled long enough: network, hypervisor or untraced |

## boot-isolcpus/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=696903 clocksource=tsc

### Loop counters (boot-isolcpus/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001550 | 1027485 | 2.92 | 67211200 | 22.39 | 1120187 | 1120187 | 98.5 | 88.7 | true | 0 | 406 | 1350.6 | 365758 / 5935 / 5 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8215 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672347200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (260362 of 2900000 in 1-msg reads) | 260362 | 4.58 | 3.26 | 6.73 | 22.88 | 27.16 | 40.91 | 66.11 |

#### Stalls: boot-isolcpus/md-recv-busy, loop 0 on CPU 2

67211200 iterations, longest 1350.6 µs; over 5 / 20 / 100 µs: 365758 / 5935 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4379 | 97604.3 | 692.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1278 | 28046.2 | 174.6 |
| mostly unexplained (+timer tick_nohz_handler) | 168 | 5441.6 | 1350.6 |
| softirq NET_RX | 61 | 1450.1 | 35.9 |
| preempted by kworker/2:2 | 12 | 363.7 | 38.0 |
| mostly unexplained (+preempted by kworker/2:2) | 15 | 338.4 | 25.6 |
| mostly unexplained (+softirq TIMER) | 9 | 221.1 | 44.0 |
| timer tick_nohz_handler | 3 | 89.2 | 31.9 |
| mostly unexplained (+preempted by kworker/2:1H) | 3 | 84.4 | 37.8 |
| workqueue mana_gf_stats_work_handler | 3 | 66.6 | 24.3 |
| mostly unexplained (+softirq NET_RX) | 1 | 32.1 | 32.1 |
| preempted by kworker/2:0 | 1 | 28.4 | 28.4 |
| mostly unexplained (+softirq RCU) | 1 | 27.1 | 27.1 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 1 | 20.3 | 20.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1350.6 | 50 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 567, softirq NET_RX 1 |
| 2 | 692.7 | 50 | loop work (50 items) | timer tick_nohz_handler 7 |
| 3 | 544.4 | 48 | loop work (48 items) |  |
| 4 | 391.3 | 1550 | loop work (1550 items) |  |
| 5 | 174.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 84.1 | 21 | loop work (21 items) |  |
| 7 | 81.6 | 11 | loop work (11 items) | timer tick_nohz_handler 3 |
| 8 | 74.2 | 17 | loop work (17 items) | softirq NET_RX 10, timer tick_nohz_handler 4 |
| 9 | 72.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 67.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60103 |
| CAL Function call interrupts | 1014 |
| RES Rescheduling interrupts | 30 |

## boot-isolcpus/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: boot-isolcpus/noise, loop 0 on CPU 2

12578037760 iterations, longest 58.9 µs; over 5 / 20 / 100 µs: 21312 / 94 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+timer tick_nohz_handler) | 18138 | 95757.9 | 58.7 |
| mostly unexplained (+softirq TIMER) | 1448 | 12231.6 | 20.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 438 | 5626.4 | 58.9 |
| mostly unexplained (+preempted by kworker/2:2) | 472 | 4417.8 | 24.7 |
| timer tick_nohz_handler | 473 | 2583.0 | 18.1 |
| softirq TIMER | 134 | 1141.1 | 15.3 |
| mostly unexplained (+preempted by kworker/2:1) | 76 | 740.3 | 19.3 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 28 | 330.8 | 13.5 |
| preempted by kworker/2:2 | 15 | 317.5 | 27.4 |
| mostly unexplained (+preempted by kworker/2:1H) | 17 | 258.4 | 22.1 |
| mostly unexplained (+preempted by migration/2) | 30 | 220.4 | 9.9 |
| workqueue mana_gf_stats_work_handler | 34 | 202.4 | 23.7 |
| preempted by kworker/2:1 | 3 | 59.5 | 22.9 |
| mostly unexplained (+softirq RCU) | 5 | 25.7 | 5.4 |
| preempted by kworker/2:0 | 1 | 9.7 | 9.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 58.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 58.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 16 |
| 3 | 58.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 15 |
| 4 | 48.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 46.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 16 |
| 6 | 46.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 45.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 45.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 45.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 43.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120064 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 56 |
| 42 HV-PCI-MSIX-7870:00:00.0 2-edge mana_q1@pci:7870:00:00.0 | 1 |

### Loop counters (boot-isolcpus/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1203572 | 1203572 | 1.00 | 457902313 | 380.45 | 7631705 | 7591586 | 0.0 | 52.2 | false | 0 | 406 | 1077.7 | 60351 / 130 / 4 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-epoll-spin-0, loop 0 on CPU 2

455495168 iterations, longest 1077.7 µs; over 5 / 20 / 100 µs: 60351 / 130 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 66 | 3213.5 | 843.9 |
| mostly unexplained (+timer tick_nohz_handler) | 9 | 1584.7 | 1077.7 |
| mostly unexplained (+preempted by kworker/2:2) | 33 | 734.2 | 28.9 |
| preempted by kworker/2:2 | 8 | 254.3 | 52.6 |
| mostly unexplained (+softirq TIMER) | 7 | 168.5 | 29.6 |
| mostly unexplained (+preempted by kworker/2:1H) | 6 | 137.1 | 27.0 |
| preempted by kworker/2:1H | 1 | 28.0 | 28.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1077.7 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 7 |
| 2 | 843.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 258.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 238.2 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5, softirq TIMER 1 |
| 5 | 87.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 6 | 66.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 66.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 53.0 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 15 |
| 9 | 52.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 52.6 | 0 | preempted by kworker/2:2 | preempted by kworker/2:2 34, workqueue psi_avgs_work 20, softirq TIMER 11, timer tick_nohz_handler 2 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60164 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 10 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-isolcpus/pp-epoll-spin-0 (median 45.86)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1760.12 | 1714.25 | 0.00 |  | 185.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 1529.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1117.36 | 1071.50 | 1077.16 | mostly unexplained (+timer tick_nohz_handler) | 20.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 871.17 | 825.31 | 870.56 | unexplained: no kernel event (hypervisor or in-kernel time) | 102.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 772.32 | 726.46 | 0.00 |  | 20.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 706.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 428.81 | 382.94 | 0.00 |  | 20.31 | unexplained: no kernel event (hypervisor or in-kernel time) | 362.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 399.96 | 354.10 | 0.00 |  | 61.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 292.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 387.50 | 341.63 | 0.00 |  | 60.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 280.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 380.74 | 334.88 | 0.00 |  | 20.98 | unexplained: no kernel event (hypervisor or in-kernel time) | 313.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 339.32 | 293.46 | 0.00 |  | 0.00 |  | 293.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 329.11 | 283.25 | 0.00 |  | 20.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 262.34 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198916 | 1198916 | 1.00 | 887890120 | 740.58 | 7399084 | 7379102 | 0.0 | 46.8 | false | 1 | 820 | 741.1 | 120670 / 218 / 8 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-epoll-spin-10000, loop 0 on CPU 2

885492288 iterations, longest 741.1 µs; over 5 / 20 / 100 µs: 120670 / 218 / 8

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 135 | 6217.0 | 721.2 |
| mostly unexplained (+timer tick_nohz_handler) | 10 | 1162.1 | 741.1 |
| preempted by kworker/2:2 | 23 | 727.9 | 81.9 |
| preempted by swapper/2 | 1 | 724.5 | 724.5 |
| mostly unexplained (+preempted by kworker/2:2) | 22 | 519.8 | 40.3 |
| workqueue mana_gf_stats_work_handler | 12 | 248.2 | 22.0 |
| mostly unexplained (+preempted by kworker/2:1H) | 8 | 197.6 | 27.6 |
| mostly unexplained (+softirq TIMER) | 4 | 87.4 | 24.8 |
| timer tick_nohz_handler | 1 | 43.8 | 43.8 |
| mostly unexplained (+softirq RCU) | 1 | 26.3 | 26.3 |
| preempted by kworker/2:1H | 1 | 26.2 | 26.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 741.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5 |
| 2 | 724.5 | 1 | preempted by swapper/2 | preempted by swapper/2 702, call function single (IPI/vector) 3, call function (IPI/vector) 2, page fault 0 |
| 3 | 721.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 706.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 137.8 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5 |
| 6 | 124.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 121.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 117.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 98.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 92.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120150 |
| CAL Function call interrupts | 1022 |
| RES Rescheduling interrupts | 57 |

#### Slowest round trips: boot-isolcpus/pp-epoll-spin-10000 (median 47.15)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1090.37 | 1043.21 | 794.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 249.04 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 927.58 | 880.42 | 97.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 41.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 741.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 787.48 | 740.33 | 0.00 |  | 695.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 45.23 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 770.61 | 723.46 | 724.20 | preempted by swapper/2 | 61.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 5 | 766.74 | 719.58 | 721.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 61.35 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 749.27 | 702.12 | 741.13 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 7 | 464.39 | 417.23 | 0.00 |  | 20.15 | unexplained: no kernel event (hypervisor or in-kernel time) | 397.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 447.35 | 400.20 | 0.00 |  | 42.12 | unexplained: no kernel event (hypervisor or in-kernel time) | 358.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 445.69 | 398.54 | 0.00 |  | 94.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 303.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 438.71 | 391.55 | 0.00 |  | 20.78 | unexplained: no kernel event (hypervisor or in-kernel time) | 370.77 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1252855 | 1252855 | 1.00 | 65282104 | 52.11 | 1088035 | 1067154 | 98.0 | 90.8 | true | 0 | 408 | 473.9 | 200232 / 179 / 1 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-recv-busy-0, loop 0 on CPU 2

64029248 iterations, longest 473.9 µs; over 5 / 20 / 100 µs: 200232 / 179 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 67 | 2757.0 | 473.9 |
| mostly unexplained (+softirq NET_RX) | 29 | 1113.3 | 84.4 |
| mostly unexplained (+preempted by kworker/2:2) | 32 | 727.6 | 36.6 |
| mostly unexplained (+preempted by kworker/2:1H) | 10 | 277.3 | 41.9 |
| mostly unexplained (+softirq TIMER) | 12 | 262.5 | 25.2 |
| preempted by kworker/2:2 | 8 | 235.3 | 36.7 |
| mostly unexplained (+timer tick_nohz_handler) | 6 | 166.5 | 53.2 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 5 | 114.0 | 25.9 |
| mostly unexplained (+preempted by ksoftirqd/2) | 4 | 109.0 | 31.1 |
| softirq NET_RX | 2 | 78.8 | 57.0 |
| mostly unexplained (+ipi_send_cpu) | 2 | 51.5 | 28.7 |
| mostly unexplained (+softirq RCU) | 1 | 25.3 | 25.3 |
| workqueue mana_gf_stats_work_handler | 1 | 21.2 | 21.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 473.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 84.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 3 | 82.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 81.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 77.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 77.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 65.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 64.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 62.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 60.6 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60178 |
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 29 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-isolcpus/pp-recv-busy-0 (median 45.15)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 617.35 | 572.20 | 0.00 |  | 0.00 |  | 572.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 515.37 | 470.22 | 0.00 |  | 43.74 | unexplained: no kernel event (hypervisor or in-kernel time) | 426.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 490.13 | 444.99 | 0.00 |  | 40.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 404.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 477.09 | 431.94 | 473.73 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.05 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 410.22 | 365.07 | 0.00 |  | 0.00 |  | 365.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 365.76 | 320.61 | 0.00 |  | 25.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 295.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 356.50 | 311.35 | 0.00 |  | 0.00 |  | 311.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 339.77 | 294.63 | 0.00 |  | 40.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 254.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 293.11 | 247.96 | 0.00 |  | 0.00 |  | 247.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 281.99 | 236.84 | 0.00 |  | 20.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 216.84 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194835 | 1194835 | 1.00 | 138326995 | 115.77 | 1152725 | 1142768 | 99.1 | 89.6 | true | 1 | 838 | 645.3 | 216166 / 294 / 1 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-recv-busy-10000, loop 0 on CPU 2

137132160 iterations, longest 645.3 µs; over 5 / 20 / 100 µs: 216166 / 294 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 125 | 3918.3 | 83.8 |
| mostly unexplained (+preempted by kworker/2:2) | 68 | 1527.8 | 29.2 |
| mostly unexplained (+softirq NET_RX) | 22 | 721.9 | 79.8 |
| preempted by swapper/2 | 1 | 645.3 | 645.3 |
| preempted by kworker/2:2 | 16 | 464.3 | 36.5 |
| mostly unexplained (+softirq TIMER) | 15 | 343.8 | 34.8 |
| mostly unexplained (+preempted by kworker/2:1H) | 13 | 314.5 | 31.4 |
| mostly unexplained (+timer tick_nohz_handler) | 8 | 290.8 | 73.6 |
| workqueue mana_gf_stats_work_handler | 10 | 218.7 | 24.6 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 6 | 126.7 | 23.4 |
| mostly unexplained (+preempted by ksoftirqd/2) | 3 | 84.9 | 31.9 |
| softirq TIMER | 3 | 77.3 | 31.1 |
| softirq NET_RX | 3 | 70.8 | 29.3 |
| mostly unexplained (+ipi_send_cpu) | 1 | 20.4 | 20.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 645.3 | 1 | preempted by swapper/2 | preempted by swapper/2 611, call function single (IPI/vector) 2, softirq NET_RX 1, page fault 0 |
| 2 | 83.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 79.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 4 | 73.6 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 5 |
| 5 | 73.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 65.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 61.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 60.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 59.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 58.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120520 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 54 |
| 28 HV-PCI-MSIX-c05b:00:00.0 3-edge nvme1q3 | 1 |

#### Slowest round trips: boot-isolcpus/pp-recv-busy-10000 (median 46.52)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 685.36 | 638.84 | 644.85 | preempted by swapper/2 | 0.00 |  | 0.00 | client: preempted by swapper/2 |
| 2 | 487.40 | 440.88 | 0.00 |  | 20.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 420.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 448.87 | 402.35 | 0.00 |  | 0.00 |  | 402.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 415.12 | 368.60 | 0.00 |  | 20.22 | unexplained: no kernel event (hypervisor or in-kernel time) | 348.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 413.75 | 367.23 | 0.00 |  | 20.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 347.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 399.86 | 353.33 | 0.00 |  | 0.00 |  | 353.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 393.76 | 347.24 | 0.00 |  | 20.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 326.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 368.34 | 321.82 | 0.00 |  | 60.87 | unexplained: no kernel event (hypervisor or in-kernel time) | 260.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 321.52 | 275.00 | 0.00 |  | 20.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 254.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 317.52 | 271.00 | 0.00 |  | 0.00 |  | 271.00 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1224145 | 1224145 | 1.00 | 3672436 | 3.00 | 61207 | 50806284 | 0.0 | 11.3 | false | 0 | 378 | 695.7 | 59558 / 131 / 3 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-uring-spin-0, loop 0 on CPU 2

3048377024 iterations, longest 695.7 µs; over 5 / 20 / 100 µs: 59558 / 131 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 74 | 3224.9 | 695.7 |
| mostly unexplained (+timer tick_nohz_handler) | 4 | 722.3 | 614.0 |
| mostly unexplained (+preempted by kworker/2:2) | 31 | 705.6 | 26.5 |
| preempted by kworker/2:2 | 7 | 201.5 | 42.3 |
| mostly unexplained (+softirq TIMER) | 7 | 158.2 | 25.4 |
| mostly unexplained (+preempted by kworker/2:1H) | 6 | 139.7 | 28.9 |
| mostly unexplained (+workqueue mana_gf_stats_work_handler) | 1 | 23.6 | 23.6 |
| timer tick_nohz_handler | 1 | 20.3 | 20.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 695.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 614.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 3 | 113.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 82.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 79.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 70.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 64.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 52.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 49.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 48.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 15 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60167 |
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 10 |

#### Slowest round trips: boot-isolcpus/pp-uring-spin-0 (median 44.79)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 723.82 | 679.04 | 695.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 46.61 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 660.03 | 615.24 | 613.97 | mostly unexplained (+timer tick_nohz_handler) | 60.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 455.55 | 410.76 | 133.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 41.66 | unexplained: no kernel event (hypervisor or in-kernel time) | 236.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 443.20 | 398.41 | 0.00 |  | 20.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 378.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 437.04 | 392.25 | 0.00 |  | 0.00 |  | 392.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 399.40 | 354.61 | 0.00 |  | 41.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 313.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 391.43 | 346.65 | 0.00 |  | 0.00 |  | 346.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 321.27 | 276.49 | 0.00 |  | 0.00 |  | 276.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 319.33 | 274.54 | 0.00 |  | 20.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 254.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 296.36 | 251.57 | 0.00 |  | 0.00 |  | 251.57 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199554 | 1199554 | 1.00 | 3598663 | 3.00 | 29989 | 39236363 | 0.0 | 6.0 | false | 1 | 799 | 584.1 | 119821 / 173 / 7 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-uring-spin-10000, loop 0 on CPU 2

4708363520 iterations, longest 584.1 µs; over 5 / 20 / 100 µs: 119821 / 173 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 97 | 3555.1 | 192.2 |
| preempted by swapper/2 | 1 | 584.1 | 584.1 |
| mostly unexplained (+preempted by kworker/2:2) | 25 | 574.0 | 29.9 |
| preempted by kworker/2:2 | 17 | 553.6 | 90.5 |
| mostly unexplained (+timer tick_nohz_handler) | 5 | 490.2 | 324.0 |
| mostly unexplained (+preempted by kworker/2:1H) | 14 | 359.4 | 31.5 |
| mostly unexplained (+softirq TIMER) | 9 | 213.7 | 31.2 |
| softirq TIMER | 2 | 68.4 | 46.6 |
| mostly unexplained (+softirq RCU) | 2 | 46.6 | 26.3 |
| preempted by kworker/2:1H | 1 | 33.2 | 33.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 584.1 | 0 | preempted by swapper/2 | preempted by swapper/2 572, call function single (IPI/vector) 2, page fault 0 |
| 2 | 324.0 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 3 | 192.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 167.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 138.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 125.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 100.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 90.5 | 0 | preempted by kworker/2:2 | preempted by kworker/2:2 71, workqueue key_garbage_collector 68, softirq TIMER 9, timer tick_nohz_handler 2 |
| 9 | 84.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 10 | 80.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120150 |
| CAL Function call interrupts | 1020 |
| RES Rescheduling interrupts | 55 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-isolcpus/pp-uring-spin-10000 (median 47.41)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1967.85 | 1920.44 | 144.43 | mostly unexplained (+timer tick_nohz_handler) | 40.30 | unexplained: no kernel event (hypervisor or in-kernel time) | 1735.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1033.00 | 985.59 | 0.00 |  | 0.00 |  | 985.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 791.94 | 744.53 | 0.00 |  | 21.47 | unexplained: no kernel event (hypervisor or in-kernel time) | 723.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 742.16 | 694.75 | 0.00 |  | 0.00 |  | 694.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 642.41 | 595.00 | 584.09 | preempted by swapper/2 | 21.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 6 | 558.10 | 510.69 | 0.00 |  | 0.00 |  | 510.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 524.84 | 477.43 | 0.00 |  | 0.00 |  | 477.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 451.71 | 404.30 | 0.00 |  | 20.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 383.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 442.28 | 394.87 | 324.04 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 70.83 | client: mostly unexplained (+timer tick_nohz_handler) |
| 10 | 428.07 | 380.66 | 0.00 |  | 0.00 |  | 380.66 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1259601 | 1259601 | 1.00 | 1 | 0.00 | 0 | 47082620 | 0.0 | 0.0 | true | 0 | 387 | 426.7 | 59498 / 76 / 2 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-uring-sqpoll-0, loop 0 on CPU 2

2824957184 iterations, longest 426.7 µs; over 5 / 20 / 100 µs: 59498 / 76 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 52 | 1883.4 | 426.7 |
| mostly unexplained (+preempted by kworker/2:2) | 12 | 263.7 | 26.1 |
| preempted by kworker/2:2 | 5 | 135.2 | 31.1 |
| mostly unexplained (+timer tick_nohz_handler) | 1 | 113.4 | 113.4 |
| mostly unexplained (+softirq TIMER) | 4 | 89.4 | 26.5 |
| mostly unexplained (+preempted by kworker/2:1H) | 2 | 47.4 | 24.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 426.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 113.4 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 51.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 48.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 48.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 46.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 37.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 35.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 33.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 33.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60173 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 29 |

#### Slowest round trips: boot-isolcpus/pp-uring-sqpoll-0 (median 44.48)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 505.98 | 461.49 | 0.00 |  | 99.56 | unexplained: no kernel event (hypervisor or in-kernel time) | 361.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 436.86 | 392.37 | 426.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 21.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 422.99 | 378.50 | 0.00 |  | 40.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 337.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 417.88 | 373.39 | 0.00 |  | 20.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 353.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 376.30 | 331.81 | 0.00 |  | 20.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 311.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 323.14 | 278.65 | 0.00 |  | 0.00 |  | 278.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 307.70 | 263.22 | 0.00 |  | 0.00 |  | 263.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 301.02 | 256.53 | 0.00 |  | 40.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 216.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 277.38 | 232.89 | 0.00 |  | 40.51 | unexplained: no kernel event (hypervisor or in-kernel time) | 192.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 243.04 | 198.56 | 0.00 |  | 0.00 |  | 198.56 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199503 | 1199503 | 1.00 | 1 | 0.00 | 0 | 32784922 | 0.0 | 0.0 | true | 0 | 663 | 771.0 | 118946 / 143 / 5 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-uring-sqpoll-10000, loop 0 on CPU 2

3934190656 iterations, longest 771.0 µs; over 5 / 20 / 100 µs: 118946 / 143 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 98 | 3817.7 | 771.0 |
| mostly unexplained (+timer tick_nohz_handler) | 5 | 1197.5 | 706.7 |
| preempted by kworker/2:2 | 20 | 533.8 | 34.8 |
| mostly unexplained (+preempted by kworker/2:2) | 12 | 259.7 | 25.6 |
| mostly unexplained (+preempted by kworker/2:1H) | 6 | 147.6 | 30.4 |
| mostly unexplained (+softirq TIMER) | 1 | 22.7 | 22.7 |
| timer tick_nohz_handler | 1 | 20.2 | 20.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 771.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 706.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 3 | 348.7 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 4 | 165.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 129.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 86.5 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 3 |
| 7 | 66.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 53.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 49.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 47.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 120146 |
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-isolcpus/pp-uring-sqpoll-10000 (median 46.82)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 738.39 | 691.57 | 706.75 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 624.73 | 577.90 | 0.00 |  | 0.00 |  | 577.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 528.95 | 482.13 | 0.00 |  | 543.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 521.67 | 474.85 | 0.00 |  | 20.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 454.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 482.89 | 436.06 | 0.00 |  | 157.63 | unexplained: no kernel event (hypervisor or in-kernel time) | 278.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 452.58 | 405.75 | 0.00 |  | 409.26 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 434.63 | 387.81 | 0.00 |  | 143.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 243.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 433.21 | 386.38 | 0.00 |  | 0.00 |  | 386.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 422.43 | 375.61 | 393.47 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 10 | 354.65 | 307.83 | 0.00 |  | 61.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 246.12 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: boot-full-nomitig/http-recv-busy-10000, loop 0 on CPU 2

1919765427 iterations, longest 951.3 µs; over 5 / 20 / 100 µs: 67801 / 168 / 10

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 145 | 8259.8 | 951.3 |
| mostly unexplained (+softirq NET_RX) | 15 | 1269.2 | 654.9 |
| loop work | 5 | 131.1 | 38.6 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 82.4 | 42.2 |
| mostly unexplained (+timer tick_nohz_handler) | 1 | 21.2 | 21.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 951.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 946.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 816.5 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 654.9 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 543.2 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 191.1 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 159.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 152.4 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 117.7 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 104.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 116 |
| HVS Hyper-V stimer0 interrupts | 8 |
| IWI IRQ work interrupts | 6 |
| 28 HV-PCI-MSIX-c05b:00:00.0 3-edge nvme0q3 | 1 |

#### Slowest round trips: boot-full-nomitig/http-recv-busy-10000 (median 47.69)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1080.65 | 1032.95 | 1055.72 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 954.25 | 906.56 | 944.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 865.59 | 817.89 | 814.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 3.09 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 665.60 | 617.91 | 653.89 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 5 | 586.36 | 538.66 | 541.66 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 574.16 | 526.47 | 0.00 |  | 50.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 475.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 476.41 | 428.72 | 0.00 |  | 0.00 |  | 428.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 475.79 | 428.10 | 0.00 |  | 49.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 378.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 468.39 | 420.70 | 0.00 |  | 42.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 378.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 435.02 | 387.32 | 0.00 |  | 0.00 |  | 387.32 | neither loop stalled long enough: network, hypervisor or untraced |

## boot-full-nomitig/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=697695 clocksource=tsc

### Loop counters (boot-full-nomitig/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001579 | 987856 | 3.04 | 66083520 | 22.02 | 1101392 | 1101392 | 98.5 | 82.6 | true | 0 | 0 | 524.3 | 233949 / 219 / 7 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8216 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672353696 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (264984 of 2900029 in 1-msg reads) | 264984 | 3.46 | 2.99 | 5.35 | 8.96 | 19.47 | 23.38 | 47.14 |

#### Stalls: boot-full-nomitig/md-recv-busy, loop 0 on CPU 2

66083520 iterations, longest 524.3 µs; over 5 / 20 / 100 µs: 233949 / 219 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 123 | 4323.8 | 524.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 75 | 2272.2 | 126.4 |
| softirq NET_RX | 13 | 289.4 | 26.0 |
| mostly unexplained (+timer tick_nohz_handler) | 4 | 164.1 | 78.8 |
| mostly unexplained (+softirq RCU) | 2 | 41.3 | 20.8 |
| softirq TIMER | 1 | 21.0 | 21.0 |
| mostly unexplained (+softirq NET_RX) | 1 | 20.6 | 20.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 524.3 | 50 | loop work (50 items) | timer tick_nohz_handler 5 |
| 2 | 436.4 | 1550 | loop work (1550 items) | timer tick_nohz_handler 2, ipi_send_cpu 0 |
| 3 | 126.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 123.7 | 22 | loop work (22 items) | softirq NET_RX 19 |
| 5 | 108.6 | 15 | loop work (15 items) |  |
| 6 | 107.0 | 26 | loop work (26 items) |  |
| 7 | 101.7 | 9 | loop work (9 items) |  |
| 8 | 81.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 78.8 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 10 | 71.7 | 9 | loop work (9 items) | timer tick_nohz_handler 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60039 |
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 1 |

## boot-full-nomitig/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: boot-full-nomitig/noise, loop 0 on CPU 2

12275113984 iterations, longest 288.4 µs; over 5 / 20 / 100 µs: 555 / 117 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 440 | 7495.2 | 288.4 |
| mostly unexplained (+reschedule (IPI/vector)) | 115 | 719.5 | 14.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 288.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 284.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 214.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 173.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 123.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 65.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 63.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 51.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 44.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1015 |
| RES Rescheduling interrupts | 118 |
| IWI IRQ work interrupts | 1 |

### Loop counters (boot-full-nomitig/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1188528 | 1188528 | 1.00 | 306087585 | 257.54 | 5101460 | 5061842 | 0.0 | 36.5 | false | 0 | 0 | 402.0 | 13825 / 88 / 2 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-epoll-spin-0, loop 0 on CPU 2

303710528 iterations, longest 402.0 µs; over 5 / 20 / 100 µs: 13825 / 88 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 75 | 3029.8 | 402.0 |
| mostly unexplained (+timer tick_nohz_handler) | 11 | 299.0 | 61.1 |
| mostly unexplained (+softirq TIMER) | 2 | 128.4 | 94.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 402.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 105.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 97.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 94.4 | 1 | mostly unexplained (+softirq TIMER) | softirq TIMER 1, timer tick_nohz_handler 1 |
| 5 | 90.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 82.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 78.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 66.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 64.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 61.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60146 |
| CAL Function call interrupts | 1016 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-epoll-spin-0 (median 46.71)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 581.48 | 534.77 | 0.00 |  | 121.77 | unexplained: no kernel event (hypervisor or in-kernel time) | 413.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 506.70 | 459.98 | 373.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 86.85 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 481.09 | 434.38 | 0.00 |  | 21.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 413.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 435.65 | 388.94 | 0.00 |  | 40.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 348.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 434.42 | 387.70 | 0.00 |  | 0.00 |  | 387.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 423.00 | 376.29 | 402.03 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 420.58 | 373.87 | 0.00 |  | 85.56 | unexplained: no kernel event (hypervisor or in-kernel time) | 288.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 391.96 | 345.25 | 0.00 |  | 20.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 324.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 387.31 | 340.60 | 0.00 |  | 123.79 | unexplained: no kernel event (hypervisor or in-kernel time) | 216.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 372.81 | 326.09 | 0.00 |  | 40.68 | unexplained: no kernel event (hypervisor or in-kernel time) | 285.42 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198448 | 1198448 | 1.00 | 613159264 | 511.63 | 5109661 | 5089686 | 0.0 | 32.1 | false | 1 | 0 | 716.5 | 8765 / 136 / 9 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-epoll-spin-10000, loop 0 on CPU 2

610762368 iterations, longest 716.5 µs; over 5 / 20 / 100 µs: 8765 / 136 / 9

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 128 | 6260.8 | 714.4 |
| mostly unexplained (+timer tick_nohz_handler) | 5 | 806.1 | 716.5 |
| preempted by swapper/2 | 1 | 569.4 | 569.4 |
| mostly unexplained (+softirq RCU) | 2 | 62.5 | 41.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 716.5 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq TIMER 1 |
| 2 | 714.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 677.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 569.4 | 1 | preempted by swapper/2 | preempted by swapper/2 553, call function single (IPI/vector) 2, page fault 0 |
| 5 | 183.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 159.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 127.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 119.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 101.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 82.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 65083 |
| CAL Function call interrupts | 1018 |
| RES Rescheduling interrupts | 53 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: boot-full-nomitig/pp-epoll-spin-10000 (median 46.97)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1057.46 | 1010.49 | 126.66 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 883.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 967.94 | 920.98 | 823.47 | mostly unexplained (+timer tick_nohz_handler) | 20.05 | unexplained: no kernel event (hypervisor or in-kernel time) | 77.45 | client: mostly unexplained (+timer tick_nohz_handler) |
| 3 | 727.00 | 680.04 | 676.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 3.18 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 726.28 | 679.31 | 714.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 704.70 | 657.74 | 0.00 |  | 0.00 |  | 657.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 618.12 | 571.15 | 569.10 | preempted by swapper/2 | 20.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 7 | 505.95 | 458.99 | 134.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 324.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 461.91 | 414.94 | 293.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 121.03 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 427.65 | 380.68 | 0.00 |  | 40.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 340.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 399.42 | 352.45 | 0.00 |  | 20.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 332.21 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1256402 | 1256402 | 1.00 | 63548755 | 50.58 | 1059146 | 1038206 | 98.0 | 84.7 | true | 0 | 0 | 793.4 | 75164 / 102 / 3 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-recv-busy-0, loop 0 on CPU 2

62292352 iterations, longest 793.4 µs; over 5 / 20 / 100 µs: 75164 / 102 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 74 | 2526.7 | 150.4 |
| mostly unexplained (+softirq NET_RX) | 14 | 1310.1 | 793.4 |
| mostly unexplained (+timer tick_nohz_handler) | 13 | 1165.9 | 771.7 |
| mostly unexplained (+ipi_send_cpu) | 1 | 31.3 | 31.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 793.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 771.7 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 3 | 150.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 83.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 67.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 67.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 66.5 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 8 | 64.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 63.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 10 | 63.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60144 |
| CAL Function call interrupts | 1016 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-recv-busy-0 (median 45.10)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 2116.75 | 2071.65 | 0.00 |  | 162.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 1909.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 816.09 | 770.99 | 793.11 | mostly unexplained (+softirq NET_RX) | 20.88 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 3 | 793.29 | 748.19 | 771.45 | mostly unexplained (+timer tick_nohz_handler) | 40.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 4 | 760.61 | 715.52 | 0.00 |  | 50.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 665.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 404.95 | 359.86 | 0.00 |  | 4.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 355.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 384.25 | 339.15 | 0.00 |  | 0.00 |  | 339.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 335.79 | 290.69 | 0.00 |  | 0.00 |  | 290.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 319.91 | 274.81 | 0.00 |  | 20.85 | unexplained: no kernel event (hypervisor or in-kernel time) | 253.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 310.11 | 265.01 | 0.00 |  | 0.00 |  | 265.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 305.83 | 260.73 | 0.00 |  | 0.00 |  | 260.73 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194807 | 1194807 | 1.00 | 135159159 | 113.12 | 1126326 | 1116370 | 99.1 | 83.1 | true | 1 | 0 | 836.0 | 42903 / 199 / 5 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-recv-busy-10000, loop 0 on CPU 2

133964352 iterations, longest 836.0 µs; over 5 / 20 / 100 µs: 42903 / 199 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 165 | 6345.5 | 836.0 |
| mostly unexplained (+softirq NET_RX) | 16 | 1334.1 | 767.5 |
| preempted by swapper/2 | 1 | 618.6 | 618.6 |
| mostly unexplained (+reschedule (IPI/vector)) | 11 | 231.9 | 23.4 |
| mostly unexplained (+ipi_send_cpu) | 5 | 117.5 | 29.4 |
| mostly unexplained (+timer tick_nohz_handler) | 1 | 20.8 | 20.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 836.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 767.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 3 | 618.6 | 1 | preempted by swapper/2 | preempted by swapper/2 599, call function single (IPI/vector) 3, page fault 0 |
| 4 | 132.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 117.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 83.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 78.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 73.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 71.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 70.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 118 |
| HVS Hyper-V stimer0 interrupts | 9 |
| IWI IRQ work interrupts | 9 |
| 28 HV-PCI-MSIX-c05b:00:00.0 3-edge nvme0q3 | 1 |

#### Slowest round trips: boot-full-nomitig/pp-recv-busy-10000 (median 45.99)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 873.56 | 827.57 | 857.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 37.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 810.36 | 764.37 | 0.00 |  | 399.99 | unexplained: no kernel event (hypervisor or in-kernel time) | 364.38 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 793.80 | 747.81 | 0.00 |  | 636.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 111.15 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 786.76 | 740.77 | 767.33 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 5 | 659.41 | 613.43 | 618.24 | preempted by swapper/2 | 21.62 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 6 | 614.71 | 568.73 | 0.00 |  | 0.00 |  | 568.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 596.62 | 550.63 | 0.00 |  | 0.00 |  | 550.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 582.14 | 536.16 | 0.00 |  | 20.45 | unexplained: no kernel event (hypervisor or in-kernel time) | 515.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 580.43 | 534.44 | 0.00 |  | 21.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 513.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 579.06 | 533.07 | 0.00 |  | 20.43 | unexplained: no kernel event (hypervisor or in-kernel time) | 512.64 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1178495 | 1178495 | 1.00 | 3535486 | 3.00 | 58925 | 50214871 | 0.0 | 10.1 | false | 0 | 0 | 784.7 | 9327 / 68 / 5 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-uring-spin-0, loop 0 on CPU 2

3012892288 iterations, longest 784.7 µs; over 5 / 20 / 100 µs: 9327 / 68 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 60 | 2490.4 | 223.6 |
| mostly unexplained (+timer tick_nohz_handler) | 6 | 1159.1 | 784.7 |
| mostly unexplained (+softirq RCU) | 1 | 23.9 | 23.9 |
| mostly unexplained (+softirq TIMER) | 1 | 20.2 | 20.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 784.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 2 | 223.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 188.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2, softirq TIMER 1 |
| 4 | 124.7 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 5 | 114.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 91.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 88.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 86.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 83.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 79.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60146 |
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-uring-spin-0 (median 46.24)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 841.77 | 795.53 | 807.00 | mostly unexplained (+timer tick_nohz_handler) | 81.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 572.25 | 526.01 | 0.00 |  | 0.00 |  | 526.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 504.62 | 458.37 | 0.00 |  | 83.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 374.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 456.15 | 409.91 | 0.00 |  | 0.00 |  | 409.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 444.35 | 398.11 | 0.00 |  | 81.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 316.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 429.16 | 382.92 | 0.00 |  | 0.00 |  | 382.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 404.33 | 358.09 | 0.00 |  | 20.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 338.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 373.20 | 326.95 | 0.00 |  | 40.39 | unexplained: no kernel event (hypervisor or in-kernel time) | 286.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 369.95 | 323.71 | 0.00 |  | 0.00 |  | 323.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 368.06 | 321.82 | 0.00 |  | 42.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 279.81 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199548 | 1199548 | 1.00 | 3598645 | 3.00 | 29989 | 38706346 | 0.0 | 5.2 | false | 1 | 0 | 741.4 | 8727 / 136 / 5 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-uring-spin-10000, loop 0 on CPU 2

4644761472 iterations, longest 741.4 µs; over 5 / 20 / 100 µs: 8727 / 136 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 123 | 5581.8 | 729.3 |
| mostly unexplained (+timer tick_nohz_handler) | 10 | 996.4 | 698.6 |
| preempted by swapper/2 | 1 | 741.4 | 741.4 |
| mostly unexplained (+softirq TIMER) | 1 | 28.7 | 28.7 |
| mostly unexplained (+softirq RCU) | 1 | 26.2 | 26.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 741.4 | 0 | preempted by swapper/2 | preempted by swapper/2 718, call function single (IPI/vector) 3, page fault 0 |
| 2 | 729.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 698.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 4 | 658.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 113.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 87.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 84.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 74.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 72.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 65.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 75137 |
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 44 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-full-nomitig/pp-uring-spin-10000 (median 46.91)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 2673.12 | 2626.21 | 101.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 2525.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1005.94 | 959.03 | 0.00 |  | 658.36 | unexplained: no kernel event (hypervisor or in-kernel time) | 300.67 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 885.46 | 838.55 | 842.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.11 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 791.30 | 744.40 | 741.42 | preempted by swapper/2 | 22.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 5 | 725.32 | 678.41 | 698.64 | mostly unexplained (+timer tick_nohz_handler) | 0.00 |  | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 6 | 664.21 | 617.30 | 658.36 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.23 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 475.67 | 428.76 | 141.54 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 287.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 472.98 | 426.07 | 0.00 |  | 228.57 | unexplained: no kernel event (hypervisor or in-kernel time) | 197.50 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 450.57 | 403.66 | 174.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 229.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 419.28 | 372.37 | 0.00 |  | 0.00 |  | 372.37 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1356731 | 1356731 | 1.00 | 1 | 0.00 | 0 | 47305691 | 0.0 | 0.0 | true | 0 | 0 | 398.6 | 259 / 62 / 3 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-uring-sqpoll-0, loop 0 on CPU 2

2838341440 iterations, longest 398.6 µs; over 5 / 20 / 100 µs: 259 / 62 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 62 | 2573.2 | 398.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 398.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 176.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 118.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 85.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 73.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 55.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 47.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 46.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 45.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 44.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 58 |
| HVS Hyper-V stimer0 interrupts | 4 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: boot-full-nomitig/pp-uring-sqpoll-0 (median 42.13)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 478.04 | 435.91 | 0.00 |  | 0.00 |  | 435.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 470.06 | 427.93 | 0.00 |  | 0.00 |  | 427.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 415.81 | 373.69 | 0.00 |  | 0.00 |  | 373.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 406.18 | 364.06 | 398.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 383.72 | 341.59 | 0.00 |  | 0.00 |  | 341.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 353.55 | 311.43 | 0.00 |  | 0.00 |  | 311.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 349.16 | 307.03 | 0.00 |  | 0.00 |  | 307.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 346.21 | 304.08 | 0.00 |  | 0.00 |  | 304.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 288.50 | 246.38 | 0.00 |  | 0.00 |  | 246.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 243.26 | 201.13 | 197.06 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 4.08 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (boot-full-nomitig/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199539 | 1199539 | 1.00 | 1 | 0.00 | 0 | 34056820 | 0.0 | 0.0 | true | 0 | 0 | 702.1 | 474 / 102 / 2 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-uring-sqpoll-10000, loop 0 on CPU 2

4086818368 iterations, longest 702.1 µs; over 5 / 20 / 100 µs: 474 / 102 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 102 | 4498.5 | 702.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 702.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 701.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 93.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 50.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 49.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 46.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 46.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 46.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 44.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1020 |
| RES Rescheduling interrupts | 118 |
| HVS Hyper-V stimer0 interrupts | 3 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: boot-full-nomitig/pp-uring-sqpoll-10000 (median 46.60)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 757.81 | 711.21 | 701.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 40.57 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 730.45 | 683.85 | 702.09 | unexplained: no kernel event (hypervisor or in-kernel time) | 27.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 720.87 | 674.26 | 0.00 |  | 20.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 654.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 714.66 | 668.06 | 0.00 |  | 42.03 | unexplained: no kernel event (hypervisor or in-kernel time) | 626.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 687.18 | 640.57 | 0.00 |  | 40.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 599.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 604.43 | 557.83 | 0.00 |  | 20.38 | unexplained: no kernel event (hypervisor or in-kernel time) | 537.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 413.58 | 366.98 | 0.00 |  | 0.00 |  | 366.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 413.00 | 366.39 | 0.00 |  | 0.00 |  | 366.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 395.04 | 348.43 | 0.00 |  | 0.00 |  | 348.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 386.57 | 339.97 | 0.00 |  | 40.90 | unexplained: no kernel event (hypervisor or in-kernel time) | 299.07 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: boot-full/http-recv-busy-10000, loop 0 on CPU 2

1903604548 iterations, longest 661.3 µs; over 5 / 20 / 100 µs: 130707 / 184 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 147 | 6466.1 | 639.0 |
| mostly unexplained (+softirq NET_RX) | 23 | 1525.0 | 661.3 |
| mostly unexplained (+reschedule (IPI/vector)) | 6 | 135.2 | 25.3 |
| mostly unexplained (+ipi_send_cpu) | 4 | 98.8 | 29.1 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 79.1 | 42.1 |
| mostly unexplained (+preempted by ksoftirqd/2) | 1 | 44.1 | 44.1 |
| loop work | 1 | 34.3 | 34.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 661.3 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 11 |
| 2 | 639.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 565.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 469.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 167.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 152.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 88.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 70.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 65.9 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 10 | 60.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 115 |
| IWI IRQ work interrupts | 11 |
| HVS Hyper-V stimer0 interrupts | 10 |
| 28 HV-PCI-MSIX-c05b:00:00.0 3-edge nvme0q3 | 1 |

#### Slowest round trips: boot-full/http-recv-busy-10000 (median 48.25)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1763.35 | 1715.10 | 0.00 |  | 282.60 | unexplained: no kernel event (hypervisor or in-kernel time) | 1432.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1151.37 | 1103.13 | 0.00 |  | 983.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 119.84 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 874.23 | 825.99 | 0.00 |  | 324.41 | unexplained: no kernel event (hypervisor or in-kernel time) | 501.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 692.23 | 643.98 | 659.67 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 5 | 680.19 | 631.94 | 637.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 647.08 | 598.84 | 0.00 |  | 0.00 |  | 598.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 603.46 | 555.21 | 565.45 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 502.73 | 454.48 | 0.00 |  | 420.19 | unexplained: no kernel event (hypervisor or in-kernel time) | 34.29 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 429.30 | 381.06 | 0.00 |  | 0.00 |  | 381.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 418.11 | 369.86 | 0.00 |  | 0.00 |  | 369.86 | neither loop stalled long enough: network, hypervisor or untraced |

## boot-full/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 6.17.0-1022-azure

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=696264 clocksource=tsc

### Loop counters (boot-full/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3001550 | 979242 | 3.07 | 62701440 | 20.89 | 1045024 | 1045024 | 98.4 | 83.5 | true | 0 | 0 | 717.0 | 349742 / 212 / 5 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8216 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672347200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (248745 of 2900000 in 1-msg reads) | 248745 | 3.67 | 3.31 | 5.35 | 8.24 | 12.23 | 20.72 | 71.81 |

#### Stalls: boot-full/md-recv-busy, loop 0 on CPU 2

62701440 iterations, longest 717.0 µs; over 5 / 20 / 100 µs: 349742 / 212 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 119 | 4876.7 | 717.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 66 | 2111.8 | 70.8 |
| softirq NET_RX | 24 | 557.7 | 29.5 |
| mostly unexplained (+ipi_send_cpu) | 2 | 47.9 | 26.3 |
| mostly unexplained (+timer tick_nohz_handler) | 1 | 32.9 | 32.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 717.0 | 50 | loop work (50 items) | timer tick_nohz_handler 1 |
| 2 | 680.1 | 50 | loop work (50 items) | timer tick_nohz_handler 1 |
| 3 | 377.1 | 1550 | loop work (1550 items) |  |
| 4 | 151.2 | 23 | loop work (23 items) | softirq NET_RX 29 |
| 5 | 111.8 | 13 | loop work (13 items) | softirq NET_RX 1 |
| 6 | 74.0 | 11 | loop work (11 items) | softirq NET_RX 6 |
| 7 | 70.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 67.8 | 21 | loop work (21 items) | softirq NET_RX 9 |
| 9 | 62.8 | 15 | loop work (15 items) |  |
| 10 | 62.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60033 |
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 1 |

## boot-full/noise: platform noise on CPU 2 (120 s, a loop that only reads the counter)

#### Stalls: boot-full/noise, loop 0 on CPU 2

12193583104 iterations, longest 67.8 µs; over 5 / 20 / 100 µs: 479 / 74 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 362 | 5090.3 | 67.8 |
| mostly unexplained (+reschedule (IPI/vector)) | 116 | 758.0 | 9.5 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 8.4 | 8.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 67.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 42.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 42.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 41.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 40.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 40.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 39.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 38.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 37.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 37.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 117 |
| IWI IRQ work interrupts | 1 |

### Loop counters (boot-full/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1204903 | 1204903 | 1.00 | 303005007 | 251.48 | 5050083 | 5009920 | 0.0 | 37.5 | false | 0 | 0 | 1370.1 | 11750 / 68 / 5 | 0 | 0 |

#### Stalls: boot-full/pp-epoll-spin-0, loop 0 on CPU 2

300595200 iterations, longest 1370.1 µs; over 5 / 20 / 100 µs: 11750 / 68 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 57 | 2568.3 | 429.2 |
| mostly unexplained (+softirq TIMER) | 3 | 1418.0 | 1370.1 |
| mostly unexplained (+timer tick_nohz_handler) | 6 | 146.0 | 26.6 |
| mostly unexplained (+softirq RCU) | 2 | 45.2 | 24.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1370.1 | 0 | mostly unexplained (+softirq TIMER) | softirq TIMER 373, timer tick_nohz_handler 5 |
| 2 | 429.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 128.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 115.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 107.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 79.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 71.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 70.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 66.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 55.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60147 |
| CAL Function call interrupts | 1016 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full/pp-epoll-spin-0 (median 45.89)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1424.90 | 1379.01 | 1370.12 | mostly unexplained (+softirq TIMER) | 20.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+softirq TIMER) |
| 2 | 506.11 | 460.22 | 0.00 |  | 42.47 | unexplained: no kernel event (hypervisor or in-kernel time) | 417.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 493.17 | 447.28 | 429.22 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.10 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 432.84 | 386.95 | 0.00 |  | 0.00 |  | 386.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 407.18 | 361.29 | 0.00 |  | 0.00 |  | 361.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 403.52 | 357.62 | 0.00 |  | 20.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 337.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 362.71 | 316.81 | 0.00 |  | 20.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 296.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 362.35 | 316.45 | 0.00 |  | 40.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 276.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 361.37 | 315.48 | 0.00 |  | 40.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 275.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 343.05 | 297.16 | 0.00 |  | 0.00 |  | 297.16 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1198328 | 1198328 | 1.00 | 610319024 | 509.31 | 5085992 | 5066020 | 0.0 | 32.5 | false | 1 | 0 | 1645.7 | 10941 / 126 / 4 | 0 | 0 |

#### Stalls: boot-full/pp-epoll-spin-10000, loop 0 on CPU 2

607922368 iterations, longest 1645.7 µs; over 5 / 20 / 100 µs: 10941 / 126 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 115 | 6688.0 | 1645.7 |
| preempted by swapper/2 | 1 | 653.9 | 653.9 |
| mostly unexplained (+softirq TIMER) | 6 | 296.3 | 188.0 |
| mostly unexplained (+timer tick_nohz_handler) | 2 | 56.9 | 31.1 |
| mostly unexplained (+softirq RCU) | 2 | 43.8 | 22.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 1645.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 1487.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 653.9 | 1 | preempted by swapper/2 | preempted by swapper/2 637, call function single (IPI/vector) 3, page fault 0 |
| 4 | 188.0 | 1 | mostly unexplained (+softirq TIMER) | softirq TIMER 1, timer tick_nohz_handler 1 |
| 5 | 90.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 81.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 67.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 55.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 52.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 50.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 75467 |
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 43 |
| IWI IRQ work interrupts | 1 |

#### Slowest round trips: boot-full/pp-epoll-spin-10000 (median 47.49)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1665.26 | 1617.77 | 1645.67 | unexplained: no kernel event (hypervisor or in-kernel time) | 20.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 1492.01 | 1444.52 | 1487.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 41.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 991.02 | 943.53 | 90.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 853.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 847.47 | 799.98 | 0.00 |  | 40.70 | unexplained: no kernel event (hypervisor or in-kernel time) | 759.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 753.59 | 706.10 | 0.00 |  | 20.25 | unexplained: no kernel event (hypervisor or in-kernel time) | 685.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 714.68 | 667.19 | 653.43 | preempted by swapper/2 | 55.49 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 7 | 691.18 | 643.69 | 0.00 |  | 42.13 | unexplained: no kernel event (hypervisor or in-kernel time) | 601.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 632.46 | 584.97 | 0.00 |  | 40.94 | unexplained: no kernel event (hypervisor or in-kernel time) | 544.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 500.69 | 453.20 | 0.00 |  | 40.15 | unexplained: no kernel event (hypervisor or in-kernel time) | 413.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 456.20 | 408.71 | 0.00 |  | 46.70 | unexplained: no kernel event (hypervisor or in-kernel time) | 362.00 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1262230 | 1262230 | 1.00 | 59867671 | 47.43 | 997795 | 976757 | 97.8 | 85.7 | true | 0 | 0 | 65.9 | 154197 / 106 / 0 | 0 | 0 |

#### Stalls: boot-full/pp-recv-busy-0, loop 0 on CPU 2

58605440 iterations, longest 65.9 µs; over 5 / 20 / 100 µs: 154197 / 106 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 64 | 1975.0 | 64.0 |
| mostly unexplained (+softirq NET_RX) | 20 | 575.2 | 64.0 |
| mostly unexplained (+timer tick_nohz_handler) | 13 | 381.4 | 59.5 |
| mostly unexplained (+softirq TIMER) | 3 | 115.2 | 65.9 |
| mostly unexplained (+ipi_send_cpu) | 4 | 103.0 | 29.4 |
| mostly unexplained (+softirq RCU) | 1 | 22.9 | 22.9 |
| softirq TIMER | 1 | 20.1 | 20.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 65.9 | 1 | mostly unexplained (+softirq TIMER) | softirq TIMER 1, timer tick_nohz_handler 1 |
| 2 | 64.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 64.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 4 | 59.5 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 5 | 58.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 55.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 55.2 | 1 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 8 | 54.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 50.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 46.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60143 |
| CAL Function call interrupts | 1016 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-full/pp-recv-busy-0 (median 44.86)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 469.00 | 424.14 | 0.00 |  | 20.22 | unexplained: no kernel event (hypervisor or in-kernel time) | 403.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 437.00 | 392.15 | 0.00 |  | 0.00 |  | 392.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 433.65 | 388.79 | 0.00 |  | 0.00 |  | 388.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 349.03 | 304.18 | 0.00 |  | 40.63 | unexplained: no kernel event (hypervisor or in-kernel time) | 263.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 315.35 | 270.50 | 0.00 |  | 0.86 | unexplained: no kernel event (hypervisor or in-kernel time) | 269.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 313.92 | 269.06 | 0.00 |  | 0.00 |  | 269.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 310.90 | 266.05 | 0.00 |  | 22.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 243.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 303.07 | 258.21 | 0.00 |  | 20.89 | unexplained: no kernel event (hypervisor or in-kernel time) | 237.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 298.07 | 253.22 | 0.00 |  | 20.34 | unexplained: no kernel event (hypervisor or in-kernel time) | 232.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 297.68 | 252.82 | 0.00 |  | 40.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 212.50 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1194342 | 1194342 | 1.00 | 127571175 | 106.81 | 1063093 | 1053140 | 99.1 | 84.0 | true | 2 | 0 | 654.7 | 139612 / 196 / 3 | 0 | 0 |

#### Stalls: boot-full/pp-recv-busy-10000, loop 0 on CPU 2

126376832 iterations, longest 654.7 µs; over 5 / 20 / 100 µs: 139612 / 196 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 161 | 5124.0 | 100.8 |
| preempted by swapper/2 | 2 | 1269.6 | 654.7 |
| mostly unexplained (+softirq NET_RX) | 16 | 480.8 | 67.1 |
| mostly unexplained (+reschedule (IPI/vector)) | 10 | 212.7 | 24.3 |
| mostly unexplained (+ipi_send_cpu) | 5 | 127.0 | 29.9 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 46.1 | 46.1 |
| mostly unexplained (+timer tick_nohz_handler) | 1 | 20.9 | 20.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 654.7 | 0 | preempted by swapper/2 | preempted by swapper/2 632, call function (IPI/vector) 3, call function single (IPI/vector) 3, page fault 0 |
| 2 | 614.9 | 1 | preempted by swapper/2 | preempted by swapper/2 589, call function single (IPI/vector) 3, call function (IPI/vector) 2, page fault 0 |
| 3 | 100.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 82.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 69.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 68.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 67.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 61.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 57.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 52.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1020 |
| RES Rescheduling interrupts | 117 |
| IWI IRQ work interrupts | 14 |
| HVS Hyper-V stimer0 interrupts | 4 |
| 28 HV-PCI-MSIX-c05b:00:00.0 3-edge nvme0q3 | 1 |

#### Slowest round trips: boot-full/pp-recv-busy-10000 (median 46.75)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1217.19 | 1170.43 | 0.00 |  | 279.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 891.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 1114.50 | 1067.75 | 0.00 |  | 41.99 | unexplained: no kernel event (hypervisor or in-kernel time) | 1025.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 699.12 | 652.36 | 654.66 | preempted by swapper/2 | 20.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 4 | 664.74 | 617.99 | 614.54 | preempted by swapper/2 | 20.29 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 5 | 534.14 | 487.38 | 0.00 |  | 82.57 | unexplained: no kernel event (hypervisor or in-kernel time) | 404.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 389.27 | 342.51 | 0.00 |  | 41.79 | unexplained: no kernel event (hypervisor or in-kernel time) | 300.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 382.50 | 335.74 | 0.00 |  | 20.88 | unexplained: no kernel event (hypervisor or in-kernel time) | 314.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 375.74 | 328.98 | 0.00 |  | 20.16 | unexplained: no kernel event (hypervisor or in-kernel time) | 308.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 369.12 | 322.37 | 0.00 |  | 79.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 242.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 367.53 | 320.78 | 0.00 |  | 0.00 |  | 320.78 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1196127 | 1196127 | 1.00 | 3588382 | 3.00 | 59806 | 49445030 | 0.0 | 11.6 | false | 0 | 0 | 710.3 | 10929 / 80 / 3 | 0 | 0 |

#### Stalls: boot-full/pp-uring-spin-0, loop 0 on CPU 2

2966701824 iterations, longest 710.3 µs; over 5 / 20 / 100 µs: 10929 / 80 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 64 | 2120.6 | 218.0 |
| mostly unexplained (+timer tick_nohz_handler) | 13 | 1121.6 | 710.3 |
| mostly unexplained (+softirq RCU) | 2 | 59.3 | 31.6 |
| mostly unexplained (+softirq TIMER) | 1 | 21.3 | 21.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 710.3 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 4 |
| 2 | 218.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 117.2 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 4 | 71.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 52.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 51.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 50.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 50.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 1 |
| 9 | 48.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 44.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 60148 |
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full/pp-uring-spin-0 (median 46.47)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 717.51 | 671.04 | 710.26 | mostly unexplained (+timer tick_nohz_handler) | 20.31 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: mostly unexplained (+timer tick_nohz_handler) |
| 2 | 523.83 | 477.35 | 257.04 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 220.31 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 493.52 | 447.05 | 0.00 |  | 46.80 | unexplained: no kernel event (hypervisor or in-kernel time) | 400.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 450.57 | 404.10 | 0.00 |  | 40.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 363.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 425.52 | 379.05 | 0.00 |  | 0.00 |  | 379.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 425.38 | 378.91 | 0.00 |  | 0.00 |  | 378.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 422.97 | 376.49 | 0.00 |  | 60.65 | unexplained: no kernel event (hypervisor or in-kernel time) | 315.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 421.07 | 374.60 | 0.00 |  | 40.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 334.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 387.23 | 340.75 | 0.00 |  | 60.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 279.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 384.16 | 337.69 | 0.00 |  | 0.00 |  | 337.69 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199340 | 1199340 | 1.00 | 3598021 | 3.00 | 29984 | 38192244 | 0.0 | 5.9 | false | 1 | 0 | 753.9 | 14999 / 135 / 5 | 0 | 0 |

#### Stalls: boot-full/pp-uring-spin-10000, loop 0 on CPU 2

4583069312 iterations, longest 753.9 µs; over 5 / 20 / 100 µs: 14999 / 135 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 121 | 3857.8 | 103.0 |
| mostly unexplained (+timer tick_nohz_handler) | 7 | 1166.4 | 753.9 |
| preempted by swapper/2 | 1 | 592.8 | 592.8 |
| mostly unexplained (+softirq TIMER) | 3 | 85.2 | 43.4 |
| mostly unexplained (+softirq RCU) | 3 | 73.7 | 30.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 753.9 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 2 |
| 2 | 592.8 | 0 | preempted by swapper/2 | preempted by swapper/2 577, call function single (IPI/vector) 3, page fault 0 |
| 3 | 141.6 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 23 |
| 4 | 141.1 | 0 | mostly unexplained (+timer tick_nohz_handler) | timer tick_nohz_handler 46 |
| 5 | 103.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 85.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 83.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 66.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 63.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 59.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| HVS Hyper-V stimer0 interrupts | 90495 |
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 29 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-full/pp-uring-spin-10000 (median 47.70)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 2149.10 | 2101.40 | 75.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 43.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 1983.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 649.64 | 601.93 | 592.85 | preempted by swapper/2 | 61.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | client: preempted by swapper/2 |
| 3 | 603.01 | 555.31 | 0.00 |  | 20.07 | unexplained: no kernel event (hypervisor or in-kernel time) | 535.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 599.78 | 552.08 | 0.00 |  | 81.89 | unexplained: no kernel event (hypervisor or in-kernel time) | 470.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 491.44 | 443.73 | 198.39 | mostly unexplained (+timer tick_nohz_handler) | 21.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 224.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 448.81 | 401.11 | 0.00 |  | 20.40 | unexplained: no kernel event (hypervisor or in-kernel time) | 380.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 425.27 | 377.56 | 0.00 |  | 43.37 | unexplained: no kernel event (hypervisor or in-kernel time) | 334.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 417.58 | 369.88 | 0.00 |  | 0.00 |  | 369.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 410.65 | 362.95 | 0.00 |  | 0.00 |  | 362.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 401.69 | 353.99 | 0.00 |  | 64.44 | unexplained: no kernel event (hypervisor or in-kernel time) | 289.55 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1252865 | 1252865 | 1.00 | 1 | 0.00 | 0 | 47664868 | 0.0 | 0.0 | true | 0 | 0 | 71.0 | 239 / 50 / 0 | 0 | 0 |

#### Stalls: boot-full/pp-uring-sqpoll-0, loop 0 on CPU 2

2859892096 iterations, longest 71.0 µs; over 5 / 20 / 100 µs: 239 / 50 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 50 | 1609.0 | 71.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 71.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 47.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 46.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 45.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 45.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 44.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 44.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 44.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 42.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 41.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1019 |
| RES Rescheduling interrupts | 59 |
| HVS Hyper-V stimer0 interrupts | 3 |
| IWI IRQ work interrupts | 3 |

#### Slowest round trips: boot-full/pp-uring-sqpoll-0 (median 45.08)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 601.35 | 556.27 | 0.00 |  | 20.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 536.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 438.12 | 393.04 | 0.00 |  | 20.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 372.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 405.36 | 360.28 | 0.00 |  | 0.00 |  | 360.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 380.38 | 335.30 | 0.00 |  | 0.00 |  | 335.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 367.71 | 322.63 | 0.00 |  | 0.00 |  | 322.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 365.83 | 320.75 | 0.00 |  | 0.00 |  | 320.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 343.70 | 298.62 | 0.00 |  | 0.00 |  | 298.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 321.97 | 276.89 | 0.00 |  | 0.00 |  | 276.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 317.72 | 272.64 | 0.00 |  | 268.97 | unexplained: no kernel event (hypervisor or in-kernel time) | 3.67 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 315.02 | 269.94 | 0.00 |  | 40.48 | unexplained: no kernel event (hypervisor or in-kernel time) | 229.45 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1199319 | 1199319 | 1.00 | 1 | 0.00 | 0 | 35007199 | 0.0 | 0.0 | true | 0 | 0 | 125.2 | 497 / 100 / 1 | 0 | 0 |

#### Stalls: boot-full/pp-uring-sqpoll-10000, loop 0 on CPU 2

4200863872 iterations, longest 125.2 µs; over 5 / 20 / 100 µs: 497 / 100 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 100 | 3074.8 | 125.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 125.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 59.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 54.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 48.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 45.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 45.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 44.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 41.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 41.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 40.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1021 |
| RES Rescheduling interrupts | 117 |
| HVS Hyper-V stimer0 interrupts | 3 |
| IWI IRQ work interrupts | 3 |

#### Slowest round trips: boot-full/pp-uring-sqpoll-10000 (median 47.86)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1818.36 | 1770.50 | 0.00 |  | 335.47 | unexplained: no kernel event (hypervisor or in-kernel time) | 1435.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 799.97 | 752.11 | 0.00 |  | 42.17 | unexplained: no kernel event (hypervisor or in-kernel time) | 709.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 748.94 | 701.08 | 0.00 |  | 20.43 | unexplained: no kernel event (hypervisor or in-kernel time) | 680.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 523.10 | 475.24 | 0.00 |  | 20.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 454.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 503.55 | 455.69 | 0.00 |  | 42.08 | unexplained: no kernel event (hypervisor or in-kernel time) | 413.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 460.80 | 412.94 | 0.00 |  | 0.00 |  | 412.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 428.12 | 380.26 | 0.00 |  | 41.73 | unexplained: no kernel event (hypervisor or in-kernel time) | 338.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 414.19 | 366.33 | 0.00 |  | 20.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 346.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 413.17 | 365.31 | 0.00 |  | 0.00 |  | 365.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 407.75 | 359.89 | 0.00 |  | 0.00 |  | 359.89 | neither loop stalled long enough: network, hypervisor or untraced |

## Ping-pong round trip (client send → echo decoded; "from when due": from the slot on the paced schedule, including any wait behind a stall)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189964 | 54.43 | 55.20 | 61.81 | 66.18 | 72.65 | 89.79 | 843.84 |
| r1-base-0/pp-epoll-spin-0: epoll-spin @ 0/s | 1148630 | 51.28 | 47.30 | 66.80 | 72.20 | 79.40 | 104.14 | 787.20 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188593 | 50.59 | 47.69 | 64.40 | 70.94 | 80.33 | 113.54 | 1026.47 |
| r1-base-0/pp-recv-busy-0: recv-busy @ 0/s | 1229379 | 47.88 | 45.35 | 57.72 | 65.61 | 70.04 | 82.73 | 666.61 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s | 1184596 | 48.63 | 47.14 | 57.85 | 62.67 | 70.65 | 83.51 | 800.67 |
| r1-base-0/pp-uring-spin-0: uring-spin @ 0/s | 1189652 | 48.83 | 45.16 | 61.35 | 68.89 | 76.60 | 90.14 | 1053.23 |
| r1-base-0/pp-uring-spin-10000: uring-spin @ 10000/s | 1189517 | 50.90 | 48.10 | 64.56 | 71.29 | 79.77 | 116.14 | 2942.13 |
| r1-base-0/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1314302 | 44.71 | 42.33 | 55.07 | 64.13 | 69.90 | 77.43 | 565.56 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189610 | 50.88 | 48.19 | 60.74 | 64.32 | 70.73 | 83.22 | 626.13 |
| r1-housekeeping/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.58 | 48.30 | 60.54 | 66.36 | 72.50 | 84.38 | 651.83 |
| r1-housekeeping/pp-recv-busy-0: recv-busy @ 0/s | 1252502 | 47.01 | 44.62 | 56.59 | 64.75 | 69.40 | 79.34 | 1296.00 |
| r1-housekeeping/pp-recv-busy-10000: recv-busy @ 10000/s | 1184592 | 48.36 | 46.75 | 57.82 | 63.12 | 71.67 | 84.98 | 1571.00 |
| r1-housekeeping/pp-uring-spin-0: uring-spin @ 0/s | 1209123 | 48.04 | 44.54 | 60.44 | 67.95 | 73.21 | 93.89 | 2020.14 |
| r1-housekeeping/pp-uring-spin-10000: uring-spin @ 10000/s | 1189582 | 50.50 | 47.74 | 63.68 | 70.94 | 80.39 | 116.31 | 946.29 |
| r1-housekeeping/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1250649 | 46.99 | 44.62 | 56.52 | 64.83 | 70.25 | 80.03 | 573.64 |
| r1-housekeeping/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189600 | 48.65 | 47.32 | 56.00 | 62.51 | 70.47 | 81.33 | 1006.37 |
| r1-prefer/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 53.38 | 50.63 | 62.67 | 67.46 | 74.50 | 87.47 | 704.78 |
| r1-prefer/pp-recv-busy-0: recv-busy @ 0/s | 1232012 | 47.77 | 44.95 | 58.08 | 67.25 | 75.14 | 81.51 | 792.37 |
| r1-prefer/pp-recv-busy-10000: recv-busy @ 10000/s | 1173274 | 51.72 | 50.23 | 60.39 | 66.64 | 74.39 | 90.47 | 853.14 |
| r1-prefer/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1207541 | 48.68 | 45.85 | 59.84 | 69.34 | 76.97 | 82.44 | 405.95 |
| r1-prefer/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189659 | 51.51 | 50.12 | 60.31 | 66.24 | 73.61 | 82.67 | 1898.78 |
| r1-base-2/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.34 | 48.10 | 60.43 | 65.39 | 72.44 | 86.10 | 660.62 |
| r1-base-2/pp-epoll-spin-0: epoll-spin @ 0/s | 1175472 | 50.10 | 47.05 | 65.54 | 71.38 | 77.38 | 109.12 | 817.81 |
| r1-base-2/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188571 | 50.23 | 47.21 | 64.51 | 70.43 | 80.51 | 115.01 | 1776.39 |
| r1-base-2/pp-recv-busy-0: recv-busy @ 0/s | 1233329 | 47.74 | 45.19 | 57.70 | 65.81 | 70.02 | 78.43 | 554.50 |
| r1-base-2/pp-recv-busy-10000: recv-busy @ 10000/s | 1184624 | 48.34 | 46.94 | 57.44 | 62.15 | 70.06 | 79.85 | 1707.45 |
| r1-base-2/pp-uring-spin-0: uring-spin @ 0/s | 1163441 | 49.93 | 46.51 | 65.52 | 71.12 | 78.08 | 98.67 | 1158.51 |
| r1-base-2/pp-uring-spin-10000: uring-spin @ 10000/s | 1189551 | 50.71 | 47.84 | 64.50 | 70.92 | 79.27 | 121.33 | 745.10 |
| r1-base-2/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1231958 | 47.70 | 45.03 | 57.82 | 66.17 | 72.24 | 80.91 | 530.64 |
| r1-base-2/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189623 | 49.06 | 47.58 | 58.41 | 63.00 | 71.22 | 82.31 | 700.86 |
| r1-noquickack/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.59 | 48.22 | 60.80 | 65.88 | 72.25 | 80.97 | 919.11 |
| r1-noquickack/pp-recv-busy-0: recv-busy @ 0/s | 1242245 | 47.40 | 44.84 | 57.23 | 65.24 | 69.83 | 83.28 | 2738.55 |
| r1-noquickack/pp-recv-busy-10000: recv-busy @ 10000/s | 1184618 | 48.21 | 46.74 | 57.27 | 62.02 | 69.60 | 79.97 | 625.63 |
| r1-noquickack/pp-uring-spin-0: uring-spin @ 0/s | 1206277 | 48.16 | 44.59 | 60.57 | 68.08 | 73.39 | 92.73 | 1662.08 |
| r1-noquickack/pp-uring-spin-10000: uring-spin @ 10000/s | 1189575 | 51.07 | 47.93 | 64.89 | 72.19 | 81.55 | 110.48 | 814.10 |
| r1-nogro/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.47 | 48.13 | 60.79 | 65.12 | 72.00 | 81.21 | 1052.02 |
| r1-nogro/pp-recv-busy-0: recv-busy @ 0/s | 1233236 | 47.74 | 45.14 | 57.60 | 65.58 | 69.94 | 80.19 | 1006.44 |
| r1-nogro/pp-recv-busy-10000: recv-busy @ 10000/s | 1184484 | 48.52 | 46.97 | 57.82 | 62.47 | 70.39 | 87.58 | 2423.47 |
| r1-nogro/pp-uring-spin-0: uring-spin @ 0/s | 1170154 | 49.66 | 46.52 | 64.98 | 70.92 | 78.25 | 107.87 | 671.78 |
| r1-nogro/pp-uring-spin-10000: uring-spin @ 10000/s | 1189595 | 50.76 | 47.64 | 65.04 | 70.51 | 79.02 | 118.09 | 750.03 |
| r1-base-4/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189964 | 51.11 | 47.89 | 60.22 | 65.42 | 72.24 | 81.97 | 869.32 |
| r1-base-4/pp-epoll-spin-0: epoll-spin @ 0/s | 1184461 | 49.73 | 47.00 | 65.34 | 71.15 | 76.95 | 102.12 | 710.46 |
| r1-base-4/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188655 | 50.29 | 47.08 | 64.45 | 70.14 | 79.83 | 102.54 | 635.83 |
| r1-base-4/pp-recv-busy-0: recv-busy @ 0/s | 1241848 | 47.41 | 44.80 | 57.27 | 65.26 | 69.61 | 84.24 | 792.24 |
| r1-base-4/pp-recv-busy-10000: recv-busy @ 10000/s | 1184619 | 48.33 | 46.89 | 57.53 | 62.01 | 69.83 | 80.34 | 675.08 |
| r1-base-4/pp-uring-spin-0: uring-spin @ 0/s | 1169837 | 49.66 | 46.63 | 65.06 | 70.94 | 78.24 | 103.33 | 1876.16 |
| r1-base-4/pp-uring-spin-10000: uring-spin @ 10000/s | 1189585 | 50.54 | 47.72 | 64.26 | 70.64 | 79.79 | 116.70 | 868.63 |
| r1-base-4/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1238039 | 47.47 | 44.79 | 57.45 | 65.73 | 71.66 | 80.33 | 606.70 |
| r1-base-4/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189609 | 49.23 | 47.63 | 58.71 | 63.10 | 71.05 | 85.63 | 454.43 |
| r1-nocoalesce/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.51 | 48.35 | 60.45 | 65.42 | 72.33 | 89.37 | 1086.28 |
| r1-nocoalesce/pp-recv-busy-0: recv-busy @ 0/s | 1233234 | 47.72 | 44.99 | 57.92 | 66.66 | 71.54 | 81.00 | 861.61 |
| r1-nocoalesce/pp-recv-busy-10000: recv-busy @ 10000/s | 1184573 | 48.31 | 46.68 | 57.93 | 62.27 | 70.29 | 81.86 | 2734.75 |
| r1-nocoalesce/pp-uring-spin-0: uring-spin @ 0/s | 1178835 | 49.30 | 47.63 | 61.10 | 71.42 | 79.16 | 96.39 | 1081.10 |
| r1-nocoalesce/pp-uring-spin-10000: uring-spin @ 10000/s | 1189542 | 51.28 | 49.35 | 62.04 | 73.47 | 81.38 | 124.76 | 1087.59 |
| r1-irqlocal/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 52.00 | 48.74 | 61.17 | 66.36 | 72.95 | 81.30 | 821.56 |
| r1-irqlocal/pp-epoll-spin-0: epoll-spin @ 0/s | 1225362 | 48.05 | 45.40 | 57.96 | 66.45 | 71.16 | 81.18 | 716.72 |
| r1-irqlocal/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188721 | 49.29 | 47.86 | 58.86 | 63.39 | 71.50 | 80.17 | 2013.08 |
| r1-irqlocal/pp-recv-busy-0: recv-busy @ 0/s | 1226239 | 48.01 | 44.91 | 58.30 | 66.99 | 71.06 | 80.88 | 490.88 |
| r1-irqlocal/pp-recv-busy-10000: recv-busy @ 10000/s | 1184701 | 48.57 | 46.73 | 58.59 | 62.99 | 70.91 | 78.55 | 1017.25 |
| r1-irqlocal/pp-uring-spin-0: uring-spin @ 0/s | 1217470 | 47.76 | 44.97 | 57.75 | 66.66 | 71.14 | 80.35 | 611.59 |
| r1-irqlocal/pp-uring-spin-10000: uring-spin @ 10000/s | 1189686 | 49.07 | 47.69 | 58.82 | 63.21 | 71.21 | 81.05 | 678.35 |
| r1-base-6/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.63 | 48.28 | 60.53 | 65.47 | 72.98 | 216.30 | 1193.55 |
| r1-base-6/pp-epoll-spin-0: epoll-spin @ 0/s | 1185664 | 49.67 | 46.07 | 64.80 | 71.11 | 77.04 | 118.87 | 1016.62 |
| r1-base-6/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188552 | 50.99 | 47.79 | 65.36 | 71.33 | 79.48 | 132.52 | 1015.64 |
| r1-base-6/pp-recv-busy-0: recv-busy @ 0/s | 1258348 | 46.79 | 44.29 | 56.45 | 64.56 | 69.17 | 80.63 | 1992.82 |
| r1-base-6/pp-recv-busy-10000: recv-busy @ 10000/s | 1184629 | 48.26 | 46.82 | 57.38 | 62.02 | 69.79 | 78.95 | 799.37 |
| r1-base-6/pp-uring-spin-0: uring-spin @ 0/s | 1211540 | 47.93 | 46.28 | 56.93 | 69.84 | 75.46 | 142.15 | 672.90 |
| r1-base-6/pp-uring-spin-10000: uring-spin @ 10000/s | 1189385 | 50.52 | 47.57 | 64.30 | 70.84 | 79.84 | 238.32 | 711.74 |
| r1-base-6/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1235196 | 47.57 | 45.27 | 57.04 | 65.11 | 71.93 | 109.85 | 495.86 |
| r1-base-6/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189315 | 49.39 | 47.72 | 59.03 | 63.17 | 71.84 | 222.34 | 1052.64 |
| r1-rt/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.24 | 48.02 | 60.44 | 65.58 | 72.23 | 83.48 | 971.90 |
| r1-rt/pp-recv-busy-0: recv-busy @ 0/s | 1233523 | 47.73 | 45.22 | 57.79 | 66.06 | 70.70 | 79.50 | 589.14 |
| r1-rt/pp-recv-busy-10000: recv-busy @ 10000/s | 1184611 | 48.35 | 46.87 | 57.68 | 62.32 | 70.08 | 80.29 | 886.10 |
| r1-rt/pp-uring-spin-0: uring-spin @ 0/s | 1164334 | 49.92 | 46.57 | 65.60 | 71.09 | 78.06 | 100.50 | 526.02 |
| r1-rt/pp-uring-spin-10000: uring-spin @ 10000/s | 1189567 | 50.47 | 47.39 | 64.55 | 70.44 | 80.24 | 110.26 | 1865.76 |
| r1-sibling/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189951 | 53.43 | 50.11 | 62.42 | 67.39 | 74.32 | 91.23 | 1084.88 |
| r1-sibling/pp-recv-busy-0: recv-busy @ 0/s | 1213942 | 48.48 | 45.56 | 58.60 | 67.18 | 72.28 | 80.87 | 1869.68 |
| r1-sibling/pp-recv-busy-10000: recv-busy @ 10000/s | 1183310 | 50.45 | 48.86 | 60.14 | 64.96 | 72.75 | 83.58 | 892.80 |
| r1-sibling/pp-uring-spin-0: uring-spin @ 0/s | 1195473 | 48.27 | 44.57 | 60.75 | 68.68 | 77.98 | 106.63 | 973.88 |
| r1-sibling/pp-uring-spin-10000: uring-spin @ 10000/s | 1189448 | 51.80 | 48.77 | 65.80 | 72.26 | 81.80 | 123.22 | 1011.41 |
| r1-sibling/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1217344 | 48.24 | 45.45 | 58.66 | 67.43 | 77.05 | 105.68 | 451.81 |
| r1-sibling/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189393 | 49.50 | 47.85 | 59.19 | 63.38 | 71.47 | 160.66 | 681.63 |
| r1-base-8/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.09 | 47.97 | 60.20 | 64.89 | 72.04 | 83.64 | 813.67 |
| r1-base-8/pp-epoll-spin-0: epoll-spin @ 0/s | 1191764 | 49.41 | 45.73 | 64.92 | 71.00 | 78.02 | 98.81 | 822.35 |
| r1-base-8/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188608 | 50.30 | 47.53 | 63.94 | 70.32 | 79.05 | 118.20 | 744.41 |
| r1-base-8/pp-recv-busy-0: recv-busy @ 0/s | 1251495 | 47.04 | 44.73 | 56.64 | 64.90 | 69.44 | 79.09 | 777.94 |
| r1-base-8/pp-recv-busy-10000: recv-busy @ 10000/s | 1184590 | 48.31 | 46.94 | 57.14 | 62.26 | 69.99 | 79.88 | 1516.72 |
| r1-base-8/pp-uring-spin-0: uring-spin @ 0/s | 1211377 | 47.91 | 44.52 | 60.17 | 68.14 | 75.86 | 91.54 | 536.43 |
| r1-base-8/pp-uring-spin-10000: uring-spin @ 10000/s | 1189558 | 50.49 | 47.76 | 64.04 | 70.49 | 79.38 | 117.42 | 1894.53 |
| r1-base-8/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1243682 | 47.25 | 44.62 | 57.16 | 65.48 | 71.62 | 82.28 | 830.44 |
| r1-base-8/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189575 | 48.77 | 47.38 | 57.50 | 62.61 | 70.68 | 80.36 | 1612.84 |
| r1-busypoll50/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.20 | 47.97 | 60.42 | 66.05 | 72.09 | 87.39 | 971.33 |
| r1-busypoll50/pp-recv-busy-0: recv-busy @ 0/s | 1250397 | 47.08 | 44.52 | 56.81 | 64.72 | 69.32 | 78.63 | 1081.74 |
| r1-busypoll50/pp-recv-busy-10000: recv-busy @ 10000/s | 1184519 | 48.31 | 46.87 | 57.36 | 62.15 | 69.89 | 80.78 | 1375.38 |
| r1-budget64/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.74 | 48.38 | 60.46 | 65.43 | 72.26 | 120.17 | 689.40 |
| r1-budget64/pp-recv-busy-0: recv-busy @ 0/s | 1239347 | 47.51 | 44.84 | 57.43 | 65.31 | 69.99 | 86.17 | 983.21 |
| r1-budget64/pp-recv-busy-10000: recv-busy @ 10000/s | 1184370 | 48.39 | 46.78 | 57.79 | 62.27 | 70.47 | 221.58 | 651.24 |
| r1-base-10/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.99 | 48.53 | 60.91 | 66.44 | 73.42 | 234.70 | 871.67 |
| r1-base-10/pp-epoll-spin-0: epoll-spin @ 0/s | 1153435 | 51.06 | 47.32 | 66.91 | 72.19 | 79.18 | 168.62 | 977.38 |
| r1-base-10/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188452 | 51.00 | 47.86 | 65.22 | 71.33 | 80.04 | 223.45 | 2863.19 |
| r1-base-10/pp-recv-busy-0: recv-busy @ 0/s | 1400052 | 42.04 | 41.34 | 44.16 | 56.18 | 65.20 | 72.27 | 593.66 |
| r1-base-10/pp-recv-busy-10000: recv-busy @ 10000/s | 1184585 | 48.51 | 47.01 | 57.97 | 62.25 | 70.19 | 79.53 | 1870.93 |
| r1-base-10/pp-uring-spin-0: uring-spin @ 0/s | 1147602 | 50.64 | 46.87 | 66.39 | 71.68 | 78.23 | 236.23 | 737.08 |
| r1-base-10/pp-uring-spin-10000: uring-spin @ 10000/s | 1189240 | 51.66 | 48.40 | 65.52 | 72.77 | 82.28 | 265.25 | 867.94 |
| r1-base-10/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1211168 | 48.51 | 45.92 | 58.62 | 67.16 | 73.86 | 202.67 | 948.26 |
| r1-base-10/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189595 | 49.09 | 47.59 | 58.57 | 62.89 | 71.05 | 82.78 | 1003.38 |
| boot-stock/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189962 | 50.01 | 46.72 | 59.02 | 63.81 | 70.64 | 97.70 | 727.34 |
| boot-stock/pp-epoll-spin-0: epoll-spin @ 0/s | 1198014 | 49.14 | 46.67 | 62.49 | 69.63 | 75.72 | 87.81 | 407.49 |
| boot-stock/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188737 | 48.85 | 46.57 | 60.14 | 68.96 | 76.68 | 221.96 | 2139.72 |
| boot-stock/pp-recv-busy-0: recv-busy @ 0/s | 1240720 | 47.45 | 45.55 | 56.80 | 64.63 | 69.82 | 99.80 | 724.25 |
| boot-stock/pp-recv-busy-10000: recv-busy @ 10000/s | 1184529 | 45.49 | 44.06 | 51.57 | 59.83 | 68.26 | 224.24 | 2465.00 |
| boot-stock/pp-uring-spin-0: uring-spin @ 0/s | 1194515 | 48.68 | 46.36 | 61.16 | 69.12 | 75.30 | 113.44 | 585.35 |
| boot-stock/pp-uring-spin-10000: uring-spin @ 10000/s | 1189643 | 48.41 | 46.30 | 59.47 | 68.19 | 75.94 | 90.62 | 564.97 |
| boot-stock/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1230055 | 47.77 | 45.74 | 57.27 | 65.22 | 70.79 | 83.75 | 585.21 |
| boot-stock/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189384 | 46.82 | 45.31 | 53.08 | 60.98 | 68.89 | 106.12 | 1138.77 |
| boot-isolcpus/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.49 | 48.12 | 60.51 | 67.40 | 72.27 | 87.14 | 944.93 |
| boot-isolcpus/pp-epoll-spin-0: epoll-spin @ 0/s | 1183815 | 49.76 | 45.86 | 65.22 | 70.84 | 76.48 | 98.28 | 1760.12 |
| boot-isolcpus/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188924 | 49.98 | 47.15 | 64.12 | 69.18 | 77.68 | 116.71 | 1090.37 |
| boot-isolcpus/pp-recv-busy-0: recv-busy @ 0/s | 1232266 | 47.77 | 45.15 | 57.72 | 66.44 | 76.32 | 92.36 | 617.35 |
| boot-isolcpus/pp-recv-busy-10000: recv-busy @ 10000/s | 1184877 | 48.14 | 46.52 | 57.54 | 62.02 | 69.52 | 82.65 | 685.36 |
| boot-isolcpus/pp-uring-spin-0: uring-spin @ 0/s | 1204396 | 48.29 | 44.79 | 61.10 | 68.47 | 73.74 | 90.40 | 723.82 |
| boot-isolcpus/pp-uring-spin-10000: uring-spin @ 10000/s | 1189557 | 50.42 | 47.41 | 65.02 | 69.61 | 77.96 | 112.25 | 1967.85 |
| boot-isolcpus/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1238887 | 47.41 | 44.48 | 57.64 | 65.59 | 71.01 | 79.20 | 505.98 |
| boot-isolcpus/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189507 | 48.53 | 46.82 | 58.08 | 62.62 | 70.14 | 79.36 | 738.39 |
| boot-full-nomitig/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189964 | 51.08 | 47.69 | 60.11 | 65.58 | 72.36 | 219.18 | 1080.65 |
| boot-full-nomitig/pp-epoll-spin-0: epoll-spin @ 0/s | 1168780 | 50.39 | 46.71 | 66.19 | 71.48 | 78.00 | 104.64 | 581.48 |
| boot-full-nomitig/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188460 | 49.96 | 46.97 | 63.78 | 70.57 | 79.04 | 250.75 | 1057.46 |
| boot-full-nomitig/pp-recv-busy-0: recv-busy @ 0/s | 1235449 | 47.66 | 45.10 | 57.80 | 65.61 | 69.84 | 83.95 | 2116.75 |
| boot-full-nomitig/pp-recv-busy-10000: recv-busy @ 10000/s | 1184857 | 47.47 | 45.99 | 56.97 | 61.28 | 69.03 | 81.86 | 873.56 |
| boot-full-nomitig/pp-uring-spin-0: uring-spin @ 0/s | 1159169 | 50.16 | 46.24 | 65.76 | 71.35 | 79.79 | 104.21 | 841.77 |
| boot-full-nomitig/pp-uring-spin-10000: uring-spin @ 10000/s | 1189551 | 50.13 | 46.91 | 64.48 | 69.89 | 78.26 | 118.83 | 2673.12 |
| boot-full-nomitig/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1335972 | 43.98 | 42.13 | 52.05 | 63.40 | 68.67 | 76.10 | 478.04 |
| boot-full-nomitig/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189542 | 48.31 | 46.60 | 58.08 | 62.00 | 69.70 | 86.69 | 757.81 |
| boot-full/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189963 | 51.54 | 48.25 | 60.54 | 64.98 | 72.49 | 222.43 | 1763.35 |
| boot-full/pp-epoll-spin-0: epoll-spin @ 0/s | 1184696 | 49.69 | 45.89 | 65.09 | 71.11 | 76.70 | 124.82 | 1424.90 |
| boot-full/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1188341 | 50.33 | 47.49 | 64.17 | 70.18 | 79.42 | 258.46 | 1665.26 |
| boot-full/pp-recv-busy-0: recv-busy @ 0/s | 1241234 | 47.43 | 44.86 | 57.29 | 65.25 | 69.78 | 84.75 | 469.00 |
| boot-full/pp-recv-busy-10000: recv-busy @ 10000/s | 1184389 | 48.33 | 46.75 | 57.71 | 62.15 | 70.27 | 222.29 | 1217.19 |
| boot-full/pp-uring-spin-0: uring-spin @ 0/s | 1176189 | 49.40 | 46.47 | 64.99 | 70.58 | 76.63 | 143.44 | 717.51 |
| boot-full/pp-uring-spin-10000: uring-spin @ 10000/s | 1189346 | 50.80 | 47.70 | 64.76 | 70.64 | 80.59 | 235.43 | 2149.10 |
| boot-full/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1232174 | 47.67 | 45.08 | 57.47 | 65.56 | 71.55 | 101.85 | 601.35 |
| boot-full/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189326 | 49.40 | 47.86 | 58.76 | 63.11 | 71.62 | 228.44 | 1818.36 |

