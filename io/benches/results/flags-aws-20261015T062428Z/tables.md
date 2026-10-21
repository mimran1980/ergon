# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|
| c1-base | 407.71 | 152.6 | 1.1 | 0.8 | 407.71 µs stall: softirq TIMER |
| c1-irqlocal | 73.70 | 546.8 | 37.1 | 0.0 | 73.70 µs stall: softirq TIMER |
| c1-onequeue | 29.63 | 147.8 | 0.8 | 0.0 | 29.63 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| c1-cstate | 255.71 | 259.1 | 2.6 | 0.4 | 255.71 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| c1-noautocork | 193.68 | 158.6 | 2.2 | 0.4 | 193.68 µs stall: softirq TIMER |
| c2-base | 135.45 | 125.6 | 1.9 | 0.8 | 135.45 µs stall: softirq TIMER |
| c2-irqlocal | 40.67 | 462.4 | 22.1 | 0.0 | 40.67 µs stall: softirq NET_RX |
| c2-onequeue | 24.64 | 123.0 | 1.5 | 0.0 | 24.64 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| c2-cstate | 2581.60 | 313.1 | 3.8 | 1.9 | 2581.60 µs stall: mostly unexplained (+softirq NET_RX) |
| c2-noautocork | 172.61 | 153.0 | 3.4 | 1.5 | 172.61 µs stall: softirq TIMER |

## HTTP round trip, http-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| c1-base | 1989935 | 37.65 | 46.20 | 61.28 | 112.84 | 1009.61 | 94.4 | 0.6 | 0.3 | 0.0 | 284.12 | 430.4 | neither loop stalled long enough: network, hypervisor or untraced |
| c1-irqlocal | 1989934 | 38.52 (+0.87) | 52.36 (+6.16) | 76.42 (+9.12) | 122.57 (+9.72) | 556.09 (−493.69) | 113.4 | 0.3 | 0.0 | 0.0 | 132.56 | 4100.4 | neither loop stalled long enough: network, hypervisor or untraced |
| c1-onequeue | 1989935 | 38.76 (+1.11) | 51.85 (+5.65) | 76.81 (+9.52) | 132.59 (+19.74) | 967.59 (−82.20) | 172.2 | 0.3 | 0.0 | 0.0 | 220.65 | 318.1 | neither loop stalled long enough: network, hypervisor or untraced |
| c1-cstate | 1989933 | 38.30 (+0.65) | 53.48 (+7.28) | 75.71 (+8.42) | 106.05 (−6.79) | 492.76 (−557.03) | 81.7 | 0.0 | 0.0 | 0.0 | 220.13 | 1366.5 | neither loop stalled long enough: network, hypervisor or untraced |
| c1-noautocork | 1989934 | 38.42 (+0.77) | 50.03 (+3.83) | 72.48 (+5.19) | 128.50 (+15.65) | 484.50 (−565.29) | 136.0 | 0.0 | 0.0 | 0.0 | 148.67 | 1371.8 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-base | 1989933 | 37.56 | 46.05 | 67.29 | 106.21 | 1049.79 | 74.5 | 0.9 | 0.3 | 0.0 | 210.79 | 518.6 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-irqlocal | 1989936 | 38.55 (+0.90) | 49.75 (+3.55) | 69.58 (+2.29) | 109.05 (−3.79) | 449.35 (−600.44) | 92.9 | 0.0 | 0.0 | 0.0 | 107.82 | 1873.9 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-onequeue | 1989936 | 37.64 (−0.01) | 46.50 (+0.30) | 68.14 (+0.85) | 111.01 (−1.83) | 395.33 (−654.46) | 87.4 | 0.0 | 0.0 | 0.0 | 258.28 | 234.6 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-cstate | 1989935 | 38.62 (+0.96) | 51.04 (+4.84) | 69.31 (+2.02) | 90.25 (−22.59) | 227.01 (−822.78) | 18.1 | 0.0 | 0.0 | 0.0 | 344.25 | 888.0 | server: softirq TIMER |
| c2-noautocork | 1989935 | 37.98 (+0.34) | 49.62 (+3.42) | 62.43 (−4.86) | 96.40 (−16.44) | 354.37 (−695.42) | 48.2 | 0.0 | 0.0 | 0.0 | 162.44 | 1167.2 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| c1-base | 1989935 | 37.83 | 46.41 | 62.06 | 406.10 | 1009.78 | 294.0 | 34.1 | 0.3 | 0.0 |
| c1-irqlocal | 1989934 | 38.79 (+0.96) | 52.67 (+6.25) | 77.90 (+9.70) | 472.24 (+44.01) | 1109.28 (−340.29) | 351.0 | 54.3 | 4.2 | 0.0 |
| c1-onequeue | 1989935 | 38.94 (+1.11) | 52.11 (+5.69) | 78.99 (+10.79) | 276.44 (−151.79) | 967.75 (−481.82) | 308.8 | 25.0 | 0.0 | 0.0 |
| c1-cstate | 1989933 | 38.48 (+0.65) | 53.71 (+7.30) | 76.67 (+8.47) | 157.74 (−270.49) | 4251.46 (+2801.89) | 166.4 | 23.2 | 19.9 | 0.0 |
| c1-noautocork | 1989934 | 38.62 (+0.79) | 50.31 (+3.90) | 75.11 (+6.90) | 437.06 (+8.83) | 879.42 (−570.15) | 354.0 | 42.8 | 0.0 | 0.0 |
| c2-base | 1989933 | 37.73 | 46.23 | 68.20 | 428.23 | 1449.57 | 226.1 | 52.2 | 11.8 | 0.0 |
| c2-irqlocal | 1989936 | 38.79 (+0.96) | 50.03 (+3.62) | 70.76 (+2.55) | 435.63 (+7.40) | 1020.44 (−429.13) | 260.2 | 46.7 | 1.2 | 0.0 |
| c2-onequeue | 1989936 | 37.81 (−0.02) | 46.69 (+0.28) | 68.77 (+0.57) | 271.94 (−156.29) | 722.22 (−727.35) | 227.9 | 21.7 | 0.0 | 0.0 |
| c2-cstate | 1989935 | 38.78 (+0.95) | 51.21 (+4.80) | 69.57 (+1.37) | 90.97 (−337.26) | 348.12 (−1101.45) | 21.1 | 0.0 | 0.0 | 0.0 |
| c2-noautocork | 1989935 | 38.15 (+0.32) | 49.80 (+3.39) | 62.77 (−5.43) | 124.09 (−304.14) | 791.83 (−657.74) | 101.9 | 11.2 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| c1-base | 3897100 | 3.42 | 7.87 | 16.24 | 24.33 | 36.31 | 0.0 | 0.0 | 0.0 | 0.0 | 616.88 | 4474.5 | 616.88 µs stall: loop work (2900 items) |
| c1-irqlocal | 3896850 | 6.49 (+3.06) | 26.09 (+18.03) | 46.40 (+30.16) | 65.58 (+32.37) | 98.61 (+50.65) | 0.0 | 0.0 | 0.0 | 0.0 | 880.97 | 32802.0 | 880.97 µs stall: loop work (3150 items) |
| c1-onequeue | 3897100 | 3.13 (−0.30) | 9.40 (+1.35) | 18.29 (+2.05) | 34.47 (+1.26) | 63.63 (+15.67) | 0.0 | 0.0 | 0.0 | 0.0 | 680.79 | 4565.6 | 680.79 µs stall: loop work (2900 items) |
| c1-cstate | 3897150 | 2.84 (−0.58) | 6.45 (−1.60) | 14.83 (−1.41) | 24.11 (−9.10) | 36.34 (−11.62) | 0.0 | 0.0 | 0.0 | 0.0 | 161594.79 | 4944.0 | 161594.79 µs stall: VM paused: the idle spare core stalled too (hypervisor) |
| c1-noautocork | 3897100 | 3.22 (−0.20) | 7.61 (−0.44) | 15.26 (−0.98) | 25.28 (−7.93) | 36.62 (−11.34) | 0.0 | 0.0 | 0.0 | 0.0 | 651.62 | 4242.0 | 651.62 µs stall: loop work (2870 items) |
| c2-base | 3897200 | 2.86 | 8.05 | 15.31 | 33.21 | 47.96 | 0.0 | 0.0 | 0.0 | 0.0 | 621.95 | 5268.4 | 621.95 µs stall: loop work (2753 items) |
| c2-irqlocal | 3897041 | 4.68 (+1.25) | 20.11 (+12.06) | 35.72 (+19.48) | 50.16 (+16.96) | 73.64 (+25.68) | 0.0 | 0.0 | 0.0 | 0.0 | 755.66 | 10880.2 | 755.66 µs stall: loop work (2950 items) |
| c2-onequeue | 3897250 | 2.52 (−0.90) | 5.91 (−2.15) | 10.46 (−5.78) | 27.50 (−5.70) | 44.48 (−3.48) | 0.0 | 0.0 | 0.0 | 0.0 | 711.87 | 4315.5 | 711.87 µs stall: loop work (2750 items) |
| c2-cstate | 3897234 | 2.94 (−0.48) | 7.49 (−0.56) | 16.42 (+0.18) | 29.57 (−3.64) | 41.21 (−6.75) | 0.0 | 0.0 | 0.0 | 0.0 | 590.90 | 5074.5 | 590.90 µs stall: loop work (2750 items) |
| c2-noautocork | 3897200 | 3.31 (−0.11) | 8.84 (+0.79) | 19.16 (+2.92) | 34.32 (+1.11) | 44.04 (−3.92) | 0.0 | 0.0 | 0.0 | 0.0 | 594.43 | 4786.9 | 594.43 µs stall: loop work (2800 items) |

## WebSocket round trip, pp-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| c1-base | 4789999 | 36.92 | 49.38 | 58.75 | 115.88 | 454.17 | 98.6 | 0.0 | 0.0 | 0.0 | 83.48 | 38.4 | neither loop stalled long enough: network, hypervisor or untraced |
| c1-irqlocal | 4789999 | 38.36 (+0.68) | 50.89 (+1.52) | 77.80 (+12.06) | 117.15 (+1.27) | 1017.76 (+49.86) | 109.7 | 0.1 | 0.1 | 0.0 | 200.63 | 2556.5 | neither loop stalled long enough: network, hypervisor or untraced |
| c1-onequeue | 4789999 | 35.92 (−1.76) | 47.45 (−1.93) | 68.36 (+2.62) | 100.72 (−15.16) | 960.71 (−7.20) | 63.8 | 0.1 | 0.0 | 0.0 | 127.55 | 217.4 | neither loop stalled long enough: network, hypervisor or untraced |
| c1-cstate | 4789999 | 39.86 (+2.18) | 51.30 (+1.92) | 75.59 (+9.85) | 133.30 (+17.42) | 464.79 (−503.12) | 181.1 | 0.0 | 0.0 | 0.0 | 380.06 | 695.2 | neither loop stalled long enough: network, hypervisor or untraced |
| c1-noautocork | 4789999 | 38.17 (+0.48) | 44.70 (−4.68) | 65.92 (+0.18) | 96.24 (−19.65) | 966.49 (−1.41) | 45.8 | 0.1 | 0.0 | 0.0 | 161.46 | 559.5 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-base | 4789999 | 37.69 | 44.80 | 65.74 | 104.24 | 967.90 | 74.4 | 0.3 | 0.0 | 0.0 | 170.25 | 269.6 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-irqlocal | 4789999 | 37.52 (−0.17) | 47.33 (−2.05) | 60.90 (−4.84) | 95.86 (−20.02) | 350.42 (−617.49) | 44.2 | 0.0 | 0.0 | 0.0 | 214.45 | 2411.4 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-onequeue | 4789999 | 38.93 (+1.25) | 48.46 (−0.92) | 71.59 (+5.85) | 130.84 (+14.96) | 966.89 (−1.01) | 173.7 | 0.3 | 0.0 | 0.0 | 254.58 | 291.1 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-cstate | 4789999 | 39.78 (+2.10) | 50.47 (+1.09) | 73.02 (+7.28) | 128.17 (+12.28) | 1035.66 (+67.76) | 156.3 | 0.3 | 0.1 | 0.0 | 145.94 | 347.6 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-noautocork | 4789999 | 37.06 (−0.63) | 49.55 (+0.18) | 64.37 (−1.38) | 102.36 (−13.53) | 950.55 (−17.35) | 70.8 | 0.5 | 0.0 | 0.0 | 147.83 | 182.5 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| c1-base | 4789999 | 37.27 | 49.74 | 59.93 | 416.93 | 1345.66 | 286.7 | 46.8 | 3.8 | 0.0 |
| c1-irqlocal | 4789999 | 38.74 (+0.69) | 51.33 (+1.59) | 79.46 (+12.66) | 456.89 (+39.97) | 1263.88 (−81.78) | 271.7 | 53.2 | 5.6 | 0.0 |
| c1-onequeue | 4789999 | 36.26 (−1.79) | 47.80 (−1.94) | 69.11 (+2.31) | 221.90 (−195.03) | 960.81 (−384.85) | 180.4 | 10.6 | 0.0 | 0.0 |
| c1-cstate | 4789999 | 40.22 (+2.17) | 51.75 (+2.01) | 77.91 (+11.11) | 317.52 (−99.41) | 896.62 (−449.04) | 312.7 | 28.4 | 0.0 | 0.0 |
| c1-noautocork | 4789999 | 38.51 (+0.46) | 45.05 (−4.69) | 66.56 (−0.24) | 154.64 (−262.29) | 966.95 (−378.71) | 110.6 | 8.9 | 0.0 | 0.0 |
| c2-base | 4789999 | 38.05 | 45.17 | 66.80 | 329.75 | 1004.97 | 212.9 | 27.2 | 0.1 | 0.0 |
| c2-irqlocal | 4789999 | 37.84 (−0.21) | 47.68 (−2.06) | 61.55 (−5.25) | 139.98 (−276.94) | 814.21 (−531.45) | 107.3 | 10.1 | 0.0 | 0.0 |
| c2-onequeue | 4789999 | 39.26 (+1.21) | 48.84 (−0.91) | 74.16 (+7.36) | 372.56 (−44.37) | 967.03 (−378.63) | 321.9 | 27.8 | 0.0 | 0.0 |
| c2-cstate | 4789999 | 40.10 (+2.05) | 50.85 (+1.11) | 74.94 (+8.14) | 304.62 (−112.31) | 1100.92 (−244.74) | 277.0 | 29.4 | 2.4 | 0.0 |
| c2-noautocork | 4789999 | 37.37 (−0.68) | 49.90 (+0.16) | 65.11 (−1.69) | 179.43 (−237.50) | 950.71 (−394.95) | 156.8 | 9.9 | 0.0 | 0.0 |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs. The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | receiving CPU | LOC | IWI | RES | CAL |
|---|---|---|---|---|---|---|---|---|---|---|---|
| http-recv-busy-10000 | c1-base | 2 | 16 | 16 | 0 | 2 | | | | | | |
| http-recv-busy-10000 | c1-irqlocal | 2 | 41 | 137 | 0 | 2 | | | | | | |
| http-recv-busy-10000 | c1-onequeue | 2 | 8 | 8 | 0 | 2 | | | | | | |
| http-recv-busy-10000 | c1-cstate | 2 | 45 | 44 | 0 | 2 | | | | | | |
| http-recv-busy-10000 | c1-noautocork | 2 | 43 | 42 | 0 | 2 | | | | | | |
| http-recv-busy-10000 | c2-base | 2 | 11 | 10 | 0 | 2 | | | | | | |
| http-recv-busy-10000 | c2-irqlocal | 2 | 45 | 139 | 0 | 2 | | | | | | |
| http-recv-busy-10000 | c2-onequeue | 2 | 6 | 6 | 0 | 2 | | | | | | |
| http-recv-busy-10000 | c2-cstate | 2 | 44 | 44 | 0 | 2 | | | | | | |
| http-recv-busy-10000 | c2-noautocork | 2 | 49 | 45 | 0 | 2 | | | | | | |
| md-recv-busy | c1-base | 2 | 13 | 13 | 0 | 6 | | | | | | |
| md-recv-busy | c1-irqlocal | 2 | 21 | 43 | 0 | 6 | | | | | | |
| md-recv-busy | c1-onequeue | 2 | 9 | 10 | 0 | 6 | | | | | | |
| md-recv-busy | c1-cstate | 2 | 13 | 14 | 0 | 6 | | | | | | |
| md-recv-busy | c1-noautocork | 2 | 12 | 13 | 0 | 6 | 4 | 150 | 0 | 0 | 2 |
| md-recv-busy | c2-base | 2 | 12 | 12 | 0 | 6 | 0 | 1054 | 0 | 0 | 5 |
| md-recv-busy | c2-irqlocal | 2 | 18 | 44 | 0 | 6 | | | | | | |
| md-recv-busy | c2-onequeue | 2 | 10 | 11 | 0 | 6 | | | | | | |
| md-recv-busy | c2-cstate | 2 | 14 | 15 | 0 | 6 | | | | | | |
| md-recv-busy | c2-noautocork | 2 | 16 | 16 | 0 | 6 | | | | | | |
| noise | c1-base | 2 | 0 | 0 | 0 | 6 | | | | | | |
| noise | c1-irqlocal | 2 | 1 | 1 | 0 | 6 | | | | | | |
| noise | c1-onequeue | 2 | 0 | 0 | 0 | 6 | | | | | | |
| noise | c1-cstate | 2 | 0 | 0 | 0 | 6 | | | | | | |
| noise | c1-noautocork | 2 | 0 | 0 | 0 | 6 | | | | | | |
| noise | c2-base | 2 | 0 | 0 | 0 | 6 | | | | | | |
| noise | c2-irqlocal | 2 | 1 | 2 | 0 | 6 | | | | | | |
| noise | c2-onequeue | 2 | 0 | 0 | 0 | 6 | | | | | | |
| noise | c2-cstate | 2 | 0 | 0 | 0 | 6 | | | | | | |
| noise | c2-noautocork | 2 | 0 | 0 | 0 | 6 | | | | | | |
| pp-recv-busy-10000 | c1-base | 2 | 33 | 33 | 0 | 2 | | | | | | |
| pp-recv-busy-10000 | c1-irqlocal | 2 | 41 | 131 | 0 | 2 | | | | | | |
| pp-recv-busy-10000 | c1-onequeue | 2 | 24 | 23 | 0 | 2 | | | | | | |
| pp-recv-busy-10000 | c1-cstate | 2 | 63 | 57 | 0 | 2 | | | | | | |
| pp-recv-busy-10000 | c1-noautocork | 2 | 24 | 22 | 0 | 2 | | | | | | |
| pp-recv-busy-10000 | c2-base | 2 | 18 | 18 | 0 | 2 | | | | | | |
| pp-recv-busy-10000 | c2-irqlocal | 2 | 52 | 128 | 0 | 2 | | | | | | |
| pp-recv-busy-10000 | c2-onequeue | 2 | 21 | 20 | 0 | 2 | | | | | | |
| pp-recv-busy-10000 | c2-cstate | 2 | 64 | 61 | 0 | 2 | | | | | | |
| pp-recv-busy-10000 | c2-noautocork | 2 | 38 | 38 | 0 | 2 | | | | | | |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### http-recv-busy-10000

| condition | worst events |
|---|---|
| c1-base | 1009.61 at 16.3 s: neither loop stalled long enough: network, hypervisor or untraced; 561.01 at 113.2 s: neither loop stalled long enough: network, hypervisor or untraced; 350.24 at 113.2 s: neither loop stalled long enough: network, hypervisor or untraced; 335.18 at 29.9 s: neither loop stalled long enough: network, hypervisor or untraced; 332.59 at 104.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-irqlocal | 556.09 at 57.9 s: neither loop stalled long enough: network, hypervisor or untraced; 451.70 at 49.7 s: neither loop stalled long enough: network, hypervisor or untraced; 418.62 at 57.9 s: neither loop stalled long enough: network, hypervisor or untraced; 358.18 at 57.9 s: neither loop stalled long enough: network, hypervisor or untraced; 333.57 at 49.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-onequeue | 967.59 at 183.3 s: neither loop stalled long enough: network, hypervisor or untraced; 348.54 at 78.9 s: neither loop stalled long enough: network, hypervisor or untraced; 348.43 at 78.9 s: neither loop stalled long enough: network, hypervisor or untraced; 326.88 at 199.0 s: neither loop stalled long enough: network, hypervisor or untraced; 309.79 at 108.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-cstate | 492.76 at 125.3 s: neither loop stalled long enough: network, hypervisor or untraced; 470.31 at 125.3 s: neither loop stalled long enough: network, hypervisor or untraced; 450.88 at 125.3 s: neither loop stalled long enough: network, hypervisor or untraced; 449.69 at 125.3 s: neither loop stalled long enough: network, hypervisor or untraced; 427.31 at 125.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-noautocork | 484.50 at 113.9 s: neither loop stalled long enough: network, hypervisor or untraced; 446.28 at 77.4 s: neither loop stalled long enough: network, hypervisor or untraced; 373.18 at 159.0 s: neither loop stalled long enough: network, hypervisor or untraced; 367.61 at 62.1 s: neither loop stalled long enough: network, hypervisor or untraced; 361.26 at 8.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-base | 1049.79 at 5.9 s: neither loop stalled long enough: network, hypervisor or untraced; 560.04 at 191.0 s: neither loop stalled long enough: network, hypervisor or untraced; 512.93 at 152.8 s: neither loop stalled long enough: network, hypervisor or untraced; 435.83 at 191.0 s: neither loop stalled long enough: network, hypervisor or untraced; 395.93 at 191.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-irqlocal | 449.35 at 91.4 s: neither loop stalled long enough: network, hypervisor or untraced; 439.97 at 84.7 s: neither loop stalled long enough: network, hypervisor or untraced; 377.45 at 136.4 s: neither loop stalled long enough: network, hypervisor or untraced; 334.89 at 31.5 s: neither loop stalled long enough: network, hypervisor or untraced; 328.44 at 31.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-onequeue | 395.33 at 160.6 s: neither loop stalled long enough: network, hypervisor or untraced; 383.01 at 160.6 s: neither loop stalled long enough: network, hypervisor or untraced; 338.44 at 160.6 s: neither loop stalled long enough: network, hypervisor or untraced; 321.41 at 109.0 s: neither loop stalled long enough: network, hypervisor or untraced; 310.82 at 25.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-cstate | 227.01 at 108.8 s: server: softirq TIMER; 153.21 at 74.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 152.93 at 134.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 150.81 at 14.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 145.67 at 194.0 s: client: mostly unexplained (+softirq NET_RX) |
| c2-noautocork | 354.37 at 78.9 s: neither loop stalled long enough: network, hypervisor or untraced; 342.46 at 78.9 s: neither loop stalled long enough: network, hypervisor or untraced; 323.40 at 55.8 s: neither loop stalled long enough: network, hypervisor or untraced; 303.51 at 78.9 s: neither loop stalled long enough: network, hypervisor or untraced; 284.18 at 78.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### md-recv-busy

| condition | worst events |
|---|---|
| c1-base | 36.31 at 56.2 s: client: loop work (3 items); 36.13 at 56.2 s: client: loop work (3 items); 35.92 at 56.2 s: client: loop work (3 items); 34.68 at 56.0 s: client: loop work (3 items); 34.42 at 56.0 s: client: loop work (3 items) |
| c1-irqlocal | 98.61 at 64.9 s: client: loop work (12 items); 98.38 at 64.9 s: client: loop work (12 items); 98.18 at 64.9 s: client: loop work (12 items); 98.01 at 64.9 s: client: loop work (12 items); 97.83 at 64.9 s: client: loop work (12 items) |
| c1-onequeue | 63.63 at 31.8 s: client: loop work (313 items); 63.46 at 31.8 s: client: loop work (313 items); 63.30 at 31.8 s: client: loop work (313 items); 63.14 at 31.8 s: client: loop work (313 items); 62.98 at 31.8 s: client: loop work (313 items) |
| c1-cstate | 36.34 at 71.3 s: client: loop work (14 items); 36.14 at 71.3 s: client: loop work (14 items); 35.94 at 71.3 s: client: loop work (14 items); 35.75 at 71.3 s: client: loop work (14 items); 35.55 at 71.3 s: client: loop work (14 items) |
| c1-noautocork | 36.62 at 62.0 s: client: loop work (19 items); 36.45 at 62.0 s: client: loop work (19 items); 36.29 at 62.0 s: client: loop work (19 items); 36.27 at 50.1 s: client: loop work (40 items); 36.13 at 62.0 s: client: loop work (19 items) |
| c2-base | 47.96 at 14.7 s: client: loop work (4 items); 47.81 at 14.7 s: client: loop work (4 items); 47.67 at 14.7 s: client: loop work (4 items); 47.49 at 14.7 s: client: loop work (4 items); 46.83 at 19.0 s: client: loop work (15 items) |
| c2-irqlocal | 73.64 at 52.8 s: client: loop work (9 items); 73.48 at 52.8 s: client: loop work (9 items); 73.34 at 52.8 s: client: loop work (9 items); 73.19 at 52.8 s: client: loop work (9 items); 72.92 at 52.8 s: client: loop work (9 items) |
| c2-onequeue | 44.48 at 54.8 s: client: loop work (12 items); 44.33 at 54.8 s: client: loop work (12 items); 44.15 at 54.8 s: client: loop work (12 items); 43.98 at 54.8 s: client: loop work (12 items); 43.80 at 54.8 s: client: loop work (12 items) |
| c2-cstate | 41.21 at 25.7 s: client: loop work (50 items); 41.07 at 25.7 s: client: loop work (50 items); 40.93 at 25.7 s: client: loop work (50 items); 40.79 at 25.7 s: client: loop work (50 items); 40.65 at 25.7 s: client: loop work (50 items) |
| c2-noautocork | 44.04 at 30.8 s: client: loop work (3 items); 43.88 at 30.8 s: client: loop work (3 items); 43.71 at 30.8 s: client: loop work (3 items); 41.32 at 39.4 s: client: loop work (21 items); 41.17 at 39.4 s: client: loop work (21 items) |

### noise

| condition | worst events |
|---|---|
| c1-base | 407.71 at 101.2 s: softirq TIMER; 109.23 at 130.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 31.96 at 6.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 19.56 at 159.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 19.21 at 1.9 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c1-irqlocal | 73.70 at 25.8 s: softirq TIMER; 50.28 at 4.6 s: softirq NET_RX; 39.16 at 16.2 s: softirq NET_RX; 36.62 at 143.4 s: softirq NET_RX; 35.96 at 3.3 s: softirq NET_RX |
| c1-onequeue | 29.63 at 42.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 23.20 at 2.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.76 at 102.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 13.30 at 87.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 13.05 at 115.0 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c1-cstate | 255.71 at 70.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 60.19 at 8.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 50.75 at 4.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 44.92 at 21.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 31.68 at 29.1 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c1-noautocork | 193.68 at 105.4 s: softirq TIMER; 39.94 at 87.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 25.27 at 44.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 24.43 at 22.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 22.71 at 62.7 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c2-base | 135.45 at 23.4 s: softirq TIMER; 108.84 at 46.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 24.56 at 9.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 24.35 at 107.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 24.31 at 25.9 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c2-irqlocal | 40.67 at 95.9 s: softirq NET_RX; 37.48 at 71.9 s: softirq NET_RX; 36.16 at 146.6 s: softirq NET_RX; 34.47 at 24.3 s: softirq NET_RX; 33.97 at 3.1 s: softirq NET_RX |
| c2-onequeue | 24.64 at 8.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 23.62 at 3.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 23.33 at 44.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 20.88 at 76.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.70 at 2.1 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c2-cstate | 2581.60 at 9.2 s: mostly unexplained (+softirq NET_RX); 128.68 at 33.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 116.07 at 129.0 s: softirq TIMER; 111.08 at 153.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 109.16 at 93.4 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c2-noautocork | 172.61 at 43.8 s: softirq TIMER; 138.10 at 17.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 111.98 at 77.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 108.97 at 137.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 89.36 at 39.5 s: unexplained: no kernel event (hypervisor or in-kernel time) |

### pp-recv-busy-10000

| condition | worst events |
|---|---|
| c1-base | 454.17 at 369.7 s: neither loop stalled long enough: network, hypervisor or untraced; 409.24 at 144.7 s: neither loop stalled long enough: network, hypervisor or untraced; 399.20 at 334.4 s: neither loop stalled long enough: network, hypervisor or untraced; 385.94 at 333.7 s: neither loop stalled long enough: network, hypervisor or untraced; 380.36 at 272.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-irqlocal | 1017.76 at 245.4 s: neither loop stalled long enough: network, hypervisor or untraced; 452.14 at 49.2 s: neither loop stalled long enough: network, hypervisor or untraced; 417.11 at 199.3 s: neither loop stalled long enough: network, hypervisor or untraced; 411.34 at 199.2 s: neither loop stalled long enough: network, hypervisor or untraced; 397.91 at 252.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-onequeue | 960.71 at 260.2 s: neither loop stalled long enough: network, hypervisor or untraced; 375.00 at 309.1 s: neither loop stalled long enough: network, hypervisor or untraced; 364.59 at 420.1 s: neither loop stalled long enough: network, hypervisor or untraced; 362.48 at 39.0 s: neither loop stalled long enough: network, hypervisor or untraced; 358.33 at 24.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-cstate | 464.79 at 407.8 s: neither loop stalled long enough: network, hypervisor or untraced; 428.53 at 296.1 s: neither loop stalled long enough: network, hypervisor or untraced; 382.52 at 167.9 s: neither loop stalled long enough: network, hypervisor or untraced; 371.26 at 407.8 s: neither loop stalled long enough: network, hypervisor or untraced; 364.52 at 401.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-noautocork | 966.49 at 181.3 s: neither loop stalled long enough: network, hypervisor or untraced; 360.02 at 16.8 s: neither loop stalled long enough: network, hypervisor or untraced; 344.69 at 338.6 s: neither loop stalled long enough: network, hypervisor or untraced; 324.80 at 338.6 s: neither loop stalled long enough: network, hypervisor or untraced; 315.65 at 338.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-base | 967.90 at 326.3 s: neither loop stalled long enough: network, hypervisor or untraced; 507.55 at 410.8 s: neither loop stalled long enough: network, hypervisor or untraced; 462.13 at 387.7 s: neither loop stalled long enough: network, hypervisor or untraced; 455.62 at 35.9 s: neither loop stalled long enough: network, hypervisor or untraced; 436.54 at 140.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-irqlocal | 350.42 at 296.3 s: neither loop stalled long enough: network, hypervisor or untraced; 323.08 at 439.4 s: neither loop stalled long enough: network, hypervisor or untraced; 319.99 at 191.2 s: neither loop stalled long enough: network, hypervisor or untraced; 308.06 at 191.2 s: neither loop stalled long enough: network, hypervisor or untraced; 302.95 at 26.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-onequeue | 966.89 at 73.3 s: neither loop stalled long enough: network, hypervisor or untraced; 692.85 at 297.8 s: neither loop stalled long enough: network, hypervisor or untraced; 445.51 at 154.4 s: neither loop stalled long enough: network, hypervisor or untraced; 443.28 at 402.6 s: neither loop stalled long enough: network, hypervisor or untraced; 421.58 at 132.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-cstate | 1035.66 at 283.6 s: neither loop stalled long enough: network, hypervisor or untraced; 928.72 at 287.5 s: neither loop stalled long enough: network, hypervisor or untraced; 419.95 at 110.6 s: neither loop stalled long enough: network, hypervisor or untraced; 397.06 at 102.5 s: neither loop stalled long enough: network, hypervisor or untraced; 385.31 at 102.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-noautocork | 950.55 at 365.1 s: neither loop stalled long enough: network, hypervisor or untraced; 915.31 at 307.5 s: neither loop stalled long enough: network, hypervisor or untraced; 612.41 at 369.0 s: neither loop stalled long enough: network, hypervisor or untraced; 611.97 at 14.2 s: neither loop stalled long enough: network, hypervisor or untraced; 345.74 at 89.1 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| c1-base | loop work 316003.31; mostly unexplained (+preempted by kworker/2:1) 60048.36; unexplained: no kernel event (hypervisor or in-kernel time) 12263.57; mostly unexplained (+softirq NET_RX) 11023.08; softirq NET_RX 10424.66 |
| c1-irqlocal | loop work 2447642.69; unexplained: no kernel event (hypervisor or in-kernel time) 1261158.46; softirq NET_RX 59148.26; mostly unexplained (+softirq NET_RX) 12249.39; mostly unexplained (+preempted by kworker/2:1) 7310.51 |
| c1-onequeue | loop work 334173.37; mostly unexplained (+preempted by kworker/2:1) 66228.28; unexplained: no kernel event (hypervisor or in-kernel time) 21140.66; softirq NET_RX 17995.72; mostly unexplained (+softirq NET_RX) 10825.74 |
| c1-cstate | loop work 331936.32; unexplained: no kernel event (hypervisor or in-kernel time) 189517.78; VM paused: the idle spare core stalled too (hypervisor) 173725.72; mostly unexplained (+preempted by kworker/2:1) 136705.13; mostly unexplained (+softirq NET_RX) 56470.83 |
| c1-noautocork | loop work 277149.81; mostly unexplained (+preempted by kworker/2:1) 175207.96; unexplained: no kernel event (hypervisor or in-kernel time) 76757.82; mostly unexplained (+softirq NET_RX) 66665.22; softirq NET_RX 48867.13 |
| c2-base | loop work 372900.02; mostly unexplained (+preempted by kworker/2:1) 104658.95; mostly unexplained (+softirq NET_RX) 25881.22; softirq NET_RX 21031.36; unexplained: no kernel event (hypervisor or in-kernel time) 16335.24 |
| c2-irqlocal | loop work 764880.06; unexplained: no kernel event (hypervisor or in-kernel time) 763799.10; softirq NET_RX 36102.78; mostly unexplained (+softirq NET_RX) 5561.91; mostly unexplained (+local timer tick) 4512.80 |
| c2-onequeue | loop work 279913.31; mostly unexplained (+preempted by kworker/2:1) 69339.42; softirq NET_RX 19386.97; mostly unexplained (+softirq NET_RX) 10379.50; unexplained: no kernel event (hypervisor or in-kernel time) 9694.67 |
| c2-cstate | loop work 364034.05; unexplained: no kernel event (hypervisor or in-kernel time) 96685.41; mostly unexplained (+preempted by kworker/2:1) 93447.12; mostly unexplained (+softirq NET_RX) 38825.51; softirq NET_RX 21846.09 |
| c2-noautocork | loop work 370884.63; mostly unexplained (+preempted by kworker/2:1) 128018.38; unexplained: no kernel event (hypervisor or in-kernel time) 64662.95; mostly unexplained (+softirq NET_RX) 39777.28; softirq NET_RX 37533.89 |

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
| http-recv-busy-10000 | idle spare core | 8 | :21 | 2 (25%) | 2 of 36 | 464.85 | no |
| http-recv-busy-10000 | receiver loop | 1 | :03 | 1 (100%) | 1 of 34 | 344.25 | no |
| http-recv-busy-10000 | sender loop | 17 | :46 | 17 (100%) | 1 of 33 | 167445.48 | **yes** |
| http-recv-busy-10000 | slow samples | 71 | :49 | 22 (31%) | 7 of 33 | 492.76 | **yes** |
| md-recv-busy | idle spare core | 23 | :53 | 17 (74%) | 1 of 15 | 161211.11 | **yes** |
| md-recv-busy | receiver loop | 18 | :53 | 16 (89%) | 1 of 15 | 161594.79 | **yes** |
| md-recv-busy | slow samples | 0 | | | | | |
| noise | noise probe (loop CPU) | 2 | :07 | 1 (50%) | 1 of 28 | 2581.60 | no |
| pp-recv-busy-10000 | idle spare core | 7 | :26 | 2 (29%) | 2 of 81 | 395.57 | no |
| pp-recv-busy-10000 | receiver loop | 1 | :26 | 1 (100%) | 1 of 81 | 380.06 | no |
| pp-recv-busy-10000 | sender loop | 0 | | | | | |
| pp-recv-busy-10000 | slow samples | 116 | :03 | 29 (25%) | 14 of 81 | 967.90 | **yes** |

# Every run in detail

#### Stalls: c1-base/http-recv-busy-10000, loop 0 on CPU 2

7385935341 iterations, longest 284.1 µs; over 5 / 20 / 100 µs: 757096 / 3443 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 2292 | 52862.7 | 66.4 |
| mostly unexplained (+softirq NET_RX) | 422 | 10233.0 | 45.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 306 | 7628.1 | 266.3 |
| softirq NET_RX | 299 | 6707.9 | 31.9 |
| mostly unexplained (+irq work (IPI/vector)) | 104 | 2381.5 | 40.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 15 | 520.0 | 155.6 |
| softirq TIMER | 2 | 408.3 | 284.1 |
| loop work | 2 | 41.5 | 21.5 |
| mostly unexplained (+local timer tick) | 1 | 26.1 | 26.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 284.1 | 1 | softirq TIMER | softirq TIMER 266, local timer tick 11, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 266.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 189.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 155.6 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 124.2 | 1 | softirq TIMER | softirq TIMER 113, local timer tick 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 6 | 86.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 77.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 76.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 66.4 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 8, local timer tick 5, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 10 | 57.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 7798 |
| LOC Local timer interrupts | 7792 |
| CAL Function call interrupts | 1058 |
| MCP Machine check polls | 2 |

#### Slowest round trips: c1-base/http-recv-busy-10000 (median 37.65)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1009.61 | 971.96 | 0.00 |  | 0.00 |  | 971.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 561.01 | 523.37 | 0.00 |  | 0.00 |  | 523.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 350.24 | 312.59 | 0.00 |  | 0.00 |  | 312.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 335.18 | 297.53 | 0.00 |  | 0.00 |  | 297.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 332.59 | 294.94 | 0.00 |  | 0.00 |  | 294.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 326.35 | 288.70 | 0.00 |  | 0.00 |  | 288.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 316.33 | 278.68 | 0.00 |  | 0.00 |  | 278.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 309.24 | 271.59 | 0.00 |  | 0.00 |  | 271.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 269.52 | 231.87 | 0.00 |  | 0.00 |  | 231.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 268.74 | 231.09 | 0.00 |  | 0.00 |  | 231.09 | neither loop stalled long enough: network, hypervisor or untraced |

## c1-base/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1122078 clocksource=tsc

### Loop counters (c1-base/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002900 | 1279156 | 6.26 | 256458112 | 32.05 | 1602863 | 1602863 | 99.5 | 65.7 | true | 0 | 2079 | 616.9 | 386549 / 11932 / 2 | 4002900 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792649600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (65878 of 3897100 in 1-msg reads) | 65878 | 3.08 | 2.91 | 3.73 | 7.23 | 12.29 | 21.00 | 23.12 |

#### Stalls: c1-base/md-recv-busy, loop 0 on CPU 2

256458112 iterations, longest 616.9 µs; over 5 / 20 / 100 µs: 386549 / 11932 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 11618 | 315961.8 | 616.9 |
| softirq NET_RX | 124 | 2963.8 | 35.2 |
| mostly unexplained (+preempted by kworker/2:1) | 101 | 2366.9 | 32.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 72 | 1696.5 | 37.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 6 | 290.7 | 109.9 |
| mostly unexplained (+softirq NET_RX) | 7 | 156.3 | 24.0 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 122.9 | 32.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 616.9 | 2900 | loop work (2900 items) |  |
| 2 | 109.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 68.1 | 27 | loop work (27 items) | softirq NET_RX 23, preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 60.3 | 18 | loop work (18 items) | softirq NET_RX 18, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 59.2 | 21 | loop work (21 items) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 59.2 | 12 | loop work (12 items) | preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 57.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 55.9 | 19 | loop work (19 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 55.9 | 15 | loop work (15 items) | preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 54.9 | 18 | loop work (18 items) | softirq NET_RX 16, preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2101 |
| LOC Local timer interrupts | 2082 |
| CAL Function call interrupts | 1009 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## c1-base/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c1-base/noise, loop 0 on CPU 2

10654486528 iterations, longest 407.7 µs; over 5 / 20 / 100 µs: 407 / 3 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 384 | 2614.6 | 109.2 |
| softirq TIMER | 1 | 407.7 | 407.7 |
| mostly unexplained (+call function (IPI/vector)) | 22 | 121.2 | 7.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 407.7 | 0 | softirq TIMER | softirq TIMER 387, local timer tick 11, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 109.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 32.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 19.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 19.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 19.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 19.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 16.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 15.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 15.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |
| IWI IRQ work interrupts | 1 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (c1-base/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 786254976 | 163.80 | 1638031 | 1628031 | 99.4 | 67.9 | true | 0 | 15638 | 83.5 | 233476 / 307 / 0 | 0 | 0 |

#### Stalls: c1-base/pp-recv-busy-10000, loop 0 on CPU 2

781454976 iterations, longest 83.5 µs; over 5 / 20 / 100 µs: 233476 / 307 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 209 | 4818.8 | 34.1 |
| softirq NET_RX | 33 | 753.0 | 29.0 |
| mostly unexplained (+softirq NET_RX) | 27 | 633.8 | 32.7 |
| mostly unexplained (+irq work (IPI/vector)) | 15 | 361.4 | 35.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 11 | 324.3 | 83.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 10 | 263.5 | 37.4 |
| softirq TIMER | 1 | 74.4 | 74.4 |
| mostly unexplained (+local timer tick) | 1 | 23.1 | 23.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 83.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 74.4 | 0 | softirq TIMER | softirq TIMER 62, local timer tick 5, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 3 | 37.4 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 35.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 35.5 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 34.1 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 32.9 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 32.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 32.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 32.0 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 15665 |
| IWI IRQ work interrupts | 15656 |
| CAL Function call interrupts | 1058 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: c1-base/pp-recv-busy-10000 (median 36.92)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 454.17 | 417.24 | 0.00 |  | 0.00 |  | 417.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 409.24 | 372.31 | 0.00 |  | 0.00 |  | 372.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 399.20 | 362.28 | 0.00 |  | 0.00 |  | 362.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 385.94 | 349.02 | 0.00 |  | 0.00 |  | 349.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 380.36 | 343.44 | 0.00 |  | 0.00 |  | 343.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 356.98 | 320.06 | 0.00 |  | 0.00 |  | 320.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 354.57 | 317.65 | 0.00 |  | 0.00 |  | 317.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 346.14 | 309.21 | 0.00 |  | 0.00 |  | 309.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 344.86 | 307.94 | 0.00 |  | 0.00 |  | 307.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 341.35 | 304.43 | 0.00 |  | 0.00 |  | 304.43 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c1-irqlocal/http-recv-busy-10000, loop 0 on CPU 2

6382891555 iterations, longest 132.6 µs; over 5 / 20 / 100 µs: 5077886 / 32803 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 31931 | 756508.7 | 132.6 |
| softirq NET_RX | 356 | 8250.9 | 33.0 |
| mostly unexplained (+softirq NET_RX) | 175 | 4207.0 | 32.0 |
| mostly unexplained (+preempted by kworker/2:1) | 150 | 3653.2 | 37.0 |
| mostly unexplained (+irq enp39s0-Tx-Rx-0) | 146 | 3299.0 | 36.7 |
| loop work | 34 | 748.7 | 29.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 10 | 345.9 | 41.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 20.1 | 20.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 132.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 112.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 84.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 77.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 54.5 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 51.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 49.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 49.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 48.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 48.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 9594357 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 9371899 |
| IWI IRQ work interrupts | 65730 |
| LOC Local timer interrupts | 19662 |
| CAL Function call interrupts | 1058 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 702 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 639 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 621 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 555 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 509 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 490 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 431 |

#### Slowest round trips: c1-irqlocal/http-recv-busy-10000 (median 38.52)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 556.09 | 517.58 | 0.00 |  | 0.00 |  | 517.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 451.70 | 413.19 | 0.00 |  | 0.00 |  | 413.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 418.62 | 380.10 | 0.00 |  | 0.00 |  | 380.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 358.18 | 319.66 | 0.00 |  | 0.00 |  | 319.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 333.57 | 295.05 | 0.00 |  | 0.00 |  | 295.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 331.91 | 293.39 | 0.00 |  | 0.00 |  | 293.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 315.83 | 277.31 | 0.00 |  | 0.00 |  | 277.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 300.83 | 262.31 | 0.00 |  | 0.00 |  | 262.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 290.94 | 252.42 | 0.00 |  | 0.00 |  | 252.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 268.14 | 229.62 | 0.00 |  | 0.00 |  | 229.62 | neither loop stalled long enough: network, hypervisor or untraced |

## c1-irqlocal/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1270223 clocksource=tsc

### Loop counters (c1-irqlocal/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8003150 | 1276943 | 6.27 | 242830208 | 30.34 | 1517689 | 1517689 | 99.5 | 66.2 | true | 0 | 4620 | 881.0 | 1138747 / 87472 / 6 | 4003150 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792705600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (45525 of 3896850 in 1-msg reads) | 45525 | 4.56 | 3.29 | 7.93 | 15.12 | 26.81 | 47.68 | 72.20 |

#### Stalls: c1-irqlocal/md-recv-busy, loop 0 on CPU 2

242830208 iterations, longest 881.0 µs; over 5 / 20 / 100 µs: 1138747 / 87472 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 84679 | 2446894.0 | 881.0 |
| softirq NET_RX | 1240 | 40792.6 | 95.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1425 | 38129.5 | 87.5 |
| mostly unexplained (+softirq NET_RX) | 68 | 1697.1 | 42.7 |
| mostly unexplained (+irq enp39s0-Tx-Rx-0) | 20 | 568.8 | 33.4 |
| mostly unexplained (+preempted by kworker/2:1) | 20 | 508.8 | 32.3 |
| mostly unexplained (+irq enp39s0-Tx-Rx-6) | 15 | 399.1 | 32.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 146.3 | 61.9 |
| softirq TIMER | 1 | 26.5 | 26.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 881.0 | 3150 | loop work (3150 items) |  |
| 2 | 113.4 | 14 | loop work (14 items) |  |
| 3 | 107.8 | 8 | loop work (8 items) |  |
| 4 | 105.6 | 40 | loop work (40 items) |  |
| 5 | 105.5 | 11 | loop work (11 items) |  |
| 6 | 104.7 | 14 | loop work (14 items) |  |
| 7 | 99.3 | 26 | loop work (26 items) | softirq NET_RX 40, preempted by kworker/2:1 16, irq enp39s0-Tx-Rx-0 9, irq enp39s0-Tx-Rx-6 7 |
| 8 | 99.2 | 29 | loop work (29 items) |  |
| 9 | 98.5 | 22 | loop work (22 items) |  |
| 10 | 96.9 | 29 | loop work (29 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 1631419 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 549835 |
| IWI IRQ work interrupts | 6878 |
| LOC Local timer interrupts | 3319 |
| CAL Function call interrupts | 1006 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 262 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 254 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 225 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 184 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 170 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 164 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 140 |

## c1-irqlocal/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c1-irqlocal/noise, loop 0 on CPU 2

10432794624 iterations, longest 73.7 µs; over 5 / 20 / 100 µs: 1458 / 99 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq NET_RX | 537 | 7298.0 | 50.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 393 | 2375.7 | 33.3 |
| mostly unexplained (+softirq NET_RX) | 238 | 2244.8 | 20.9 |
| mostly unexplained (+local timer tick) | 101 | 1376.0 | 23.6 |
| mostly unexplained (+irq ena-mgmnt@pci:0000:27:00.0) | 89 | 666.5 | 13.3 |
| local timer tick | 45 | 635.3 | 25.8 |
| mostly unexplained (+irq enp39s0-Tx-Rx-6) | 13 | 83.5 | 10.8 |
| softirq TIMER | 1 | 73.7 | 73.7 |
| mostly unexplained (+irq enp39s0-Tx-Rx-4) | 9 | 54.4 | 9.2 |
| mostly unexplained (+irq enp39s0-Tx-Rx-5) | 9 | 47.2 | 5.5 |
| mostly unexplained (+irq enp39s0-Tx-Rx-0) | 6 | 35.2 | 7.6 |
| mostly unexplained (+irq enp39s0-Tx-Rx-3) | 6 | 31.4 | 5.6 |
| mostly unexplained (+irq enp39s0-Tx-Rx-7) | 4 | 30.9 | 8.8 |
| mostly unexplained (+irq enp39s0-Tx-Rx-2) | 4 | 25.4 | 8.5 |
| mostly unexplained (+irq enp39s0-Tx-Rx-1) | 3 | 15.5 | 5.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 73.7 | 0 | softirq TIMER | softirq TIMER 58, local timer tick 7, timer tick_nohz_handler 3 |
| 2 | 50.3 | 0 | softirq NET_RX | softirq NET_RX 45, irq enp39s0-Tx-Rx-2 1 |
| 3 | 39.2 | 0 | softirq NET_RX | softirq NET_RX 21, preempted by kworker/2:1 7, irq enp39s0-Tx-Rx-2 1, irq work (IPI/vector) 1 |
| 4 | 36.6 | 0 | softirq NET_RX | softirq NET_RX 21, preempted by kworker/2:1 6, irq enp39s0-Tx-Rx-2 1, irq enp39s0-Tx-Rx-6 1 |
| 5 | 36.0 | 0 | softirq NET_RX | softirq NET_RX 30, irq enp39s0-Tx-Rx-2 1 |
| 6 | 34.7 | 0 | softirq NET_RX | softirq NET_RX 25, irq enp39s0-Tx-Rx-6 1, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 33.6 | 0 | softirq NET_RX | softirq NET_RX 22, irq enp39s0-Tx-Rx-0 1, irq enp39s0-Tx-Rx-6 1, irq work (IPI/vector) 1 |
| 8 | 33.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.6 | 0 | softirq NET_RX | softirq NET_RX 24, irq enp39s0-Tx-Rx-6 1, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 29.8 | 0 | softirq NET_RX | softirq NET_RX 18, irq enp39s0-Tx-Rx-0 1, irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 284 |
| IWI IRQ work interrupts | 239 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 230 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 224 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 195 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 173 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 166 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 161 |
| LOC Local timer interrupts | 148 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 120 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 92 |

### Loop counters (c1-irqlocal/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 673689344 | 140.35 | 1403519 | 1393519 | 99.3 | 69.3 | true | 0 | 44741 | 200.6 | 4867324 / 20452 / 1 | 0 | 0 |

#### Stalls: c1-irqlocal/pp-recv-busy-10000, loop 0 on CPU 2

668889344 iterations, longest 200.6 µs; over 5 / 20 / 100 µs: 4867324 / 20452 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 19941 | 464144.6 | 72.1 |
| mostly unexplained (+softirq NET_RX) | 170 | 4100.5 | 32.7 |
| mostly unexplained (+preempted by kworker/2:1) | 136 | 3148.5 | 33.9 |
| softirq NET_RX | 126 | 2806.8 | 35.6 |
| mostly unexplained (+irq enp39s0-Tx-Rx-1) | 66 | 1522.2 | 29.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 13 | 574.5 | 200.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 200.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 72.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 51.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 48.5 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 48.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 47.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 47.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 44.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 43.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 42.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 9594570 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 9245665 |
| IWI IRQ work interrupts | 62796 |
| LOC Local timer interrupts | 19514 |
| CAL Function call interrupts | 1058 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 670 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 624 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 541 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 527 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 505 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 489 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 489 |

#### Slowest round trips: c1-irqlocal/pp-recv-busy-10000 (median 38.36)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1017.76 | 979.40 | 0.00 |  | 0.00 |  | 979.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 452.14 | 413.78 | 0.00 |  | 0.00 |  | 413.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 417.11 | 378.74 | 0.00 |  | 0.00 |  | 378.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 411.34 | 372.98 | 0.00 |  | 0.00 |  | 372.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 397.91 | 359.55 | 0.00 |  | 0.00 |  | 359.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 365.23 | 326.87 | 0.00 |  | 0.00 |  | 326.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 355.55 | 317.19 | 0.00 |  | 0.00 |  | 317.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 349.87 | 311.50 | 0.00 |  | 0.00 |  | 311.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 329.73 | 291.37 | 0.00 |  | 0.00 |  | 291.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 326.41 | 288.05 | 0.00 |  | 0.00 |  | 288.05 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c1-onequeue/http-recv-busy-10000, loop 0 on CPU 2

7157865394 iterations, longest 220.7 µs; over 5 / 20 / 100 µs: 253089 / 2545 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 1520 | 39673.9 | 57.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 512 | 11912.4 | 220.7 |
| softirq NET_RX | 215 | 5400.4 | 47.8 |
| mostly unexplained (+softirq NET_RX) | 145 | 3996.4 | 55.6 |
| mostly unexplained (+irq work (IPI/vector)) | 77 | 2009.5 | 42.7 |
| mostly unexplained (+local timer tick) | 53 | 1803.0 | 51.4 |
| loop work | 11 | 285.7 | 33.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 9 | 270.5 | 34.8 |
| softirq TIMER | 1 | 179.9 | 179.9 |
| mostly unexplained (+call function single (IPI/vector)) | 2 | 40.2 | 20.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 220.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 179.9 | 1 | softirq TIMER | softirq TIMER 157, local timer tick 14, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 3 | 161.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 152.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 57.4 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 56.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 55.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 26, preempted by ksoftirqd/2 9, local timer tick 7, timer tick_nohz_handler 4 |
| 8 | 52.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 25, preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 51.4 | 1 | mostly unexplained (+local timer tick) | local timer tick 7, timer tick_nohz_handler 5, preempted by kworker/2:1 4, irq work (IPI/vector) 1 |
| 10 | 51.0 | 1 | mostly unexplained (+local timer tick) | local timer tick 7, timer tick_nohz_handler 5, preempted by kworker/2:1 4, irq work (IPI/vector) 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 3847 |
| LOC Local timer interrupts | 3828 |
| CAL Function call interrupts | 1067 |
| MCP Machine check polls | 1 |

#### Slowest round trips: c1-onequeue/http-recv-busy-10000 (median 38.76)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 967.59 | 928.83 | 0.00 |  | 0.00 |  | 928.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 348.54 | 309.78 | 0.00 |  | 0.00 |  | 309.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 348.43 | 309.67 | 0.00 |  | 0.00 |  | 309.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 326.88 | 288.12 | 0.00 |  | 0.00 |  | 288.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 309.79 | 271.03 | 0.00 |  | 0.00 |  | 271.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 309.43 | 270.67 | 0.00 |  | 0.00 |  | 270.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 292.30 | 253.54 | 0.00 |  | 0.00 |  | 253.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 290.80 | 252.04 | 0.00 |  | 0.00 |  | 252.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 275.22 | 236.46 | 0.00 |  | 0.00 |  | 236.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 259.16 | 220.40 | 0.00 |  | 0.00 |  | 220.40 | neither loop stalled long enough: network, hypervisor or untraced |

## c1-onequeue/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1089440 clocksource=tsc

### Loop counters (c1-onequeue/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002900 | 1234600 | 6.48 | 257650432 | 32.19 | 1610315 | 1610315 | 99.5 | 65.6 | true | 0 | 1438 | 680.8 | 309637 / 12175 / 2 | 4002900 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792649600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (82908 of 3897100 in 1-msg reads) | 82908 | 2.58 | 2.40 | 2.79 | 6.95 | 18.13 | 25.04 | 30.86 |

#### Stalls: c1-onequeue/md-recv-busy, loop 0 on CPU 2

257650432 iterations, longest 680.8 µs; over 5 / 20 / 100 µs: 309637 / 12175 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 11498 | 333887.7 | 680.8 |
| mostly unexplained (+preempted by kworker/2:1) | 238 | 5718.6 | 59.9 |
| softirq NET_RX | 203 | 5016.3 | 34.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 206 | 4886.1 | 61.2 |
| mostly unexplained (+irq work (IPI/vector)) | 16 | 362.3 | 30.6 |
| mostly unexplained (+softirq NET_RX) | 9 | 210.4 | 26.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 150.0 | 56.7 |
| softirq TIMER | 1 | 79.5 | 79.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 680.8 | 2900 | loop work (2900 items) |  |
| 2 | 114.8 | 313 | loop work (313 items) |  |
| 3 | 79.5 | 0 | softirq TIMER | softirq TIMER 62, local timer tick 9, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 4 | 71.1 | 47 | loop work (47 items) |  |
| 5 | 63.0 | 27 | loop work (27 items) | softirq NET_RX 23, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 63.0 | 24 | loop work (24 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 61.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 60.3 | 28 | loop work (28 items) | softirq NET_RX 17, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 59.9 | 0 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 3, irq work (IPI/vector) 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 58.2 | 23 | loop work (23 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1580 |
| LOC Local timer interrupts | 1442 |
| CAL Function call interrupts | 1010 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## c1-onequeue/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c1-onequeue/noise, loop 0 on CPU 2

10433675264 iterations, longest 29.6 µs; over 5 / 20 / 100 µs: 394 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 369 | 2340.1 | 29.6 |
| mostly unexplained (+call function (IPI/vector)) | 25 | 147.1 | 8.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 29.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 23.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 18.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 13.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 13.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 13.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 12.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 12.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 12.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 12.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |

### Loop counters (c1-onequeue/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 777689344 | 162.02 | 1620186 | 1610186 | 99.4 | 67.6 | true | 0 | 10886 | 127.5 | 179192 / 1739 / 2 | 0 | 0 |

#### Stalls: c1-onequeue/pp-recv-busy-10000, loop 0 on CPU 2

772889344 iterations, longest 127.5 µs; over 5 / 20 / 100 µs: 179192 / 1739 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 859 | 20835.8 | 34.1 |
| softirq NET_RX | 324 | 7579.0 | 34.8 |
| mostly unexplained (+softirq NET_RX) | 247 | 6618.9 | 42.2 |
| mostly unexplained (+local timer tick) | 178 | 4062.0 | 32.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 74 | 2002.1 | 127.5 |
| mostly unexplained (+irq work (IPI/vector)) | 45 | 992.1 | 31.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 10 | 342.2 | 59.7 |
| softirq TIMER | 2 | 155.5 | 77.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 127.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 124.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 77.9 | 0 | softirq TIMER | softirq TIMER 64, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 4 | 77.6 | 0 | softirq TIMER | softirq TIMER 65, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 5 | 71.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 66.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 59.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 42.2 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 38.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 34.8 | 0 | softirq NET_RX | softirq NET_RX 18, preempted by kworker/2:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 11651 |
| IWI IRQ work interrupts | 10971 |
| CAL Function call interrupts | 1058 |
| MCP Machine check polls | 2 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: c1-onequeue/pp-recv-busy-10000 (median 35.92)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 960.71 | 924.78 | 0.00 |  | 0.00 |  | 924.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 375.00 | 339.08 | 0.00 |  | 0.00 |  | 339.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 364.59 | 328.67 | 0.00 |  | 0.00 |  | 328.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 362.48 | 326.56 | 0.00 |  | 0.00 |  | 326.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 358.33 | 322.41 | 0.00 |  | 0.00 |  | 322.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 356.79 | 320.87 | 0.00 |  | 0.00 |  | 320.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 352.27 | 316.35 | 0.00 |  | 0.00 |  | 316.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 329.60 | 293.68 | 0.00 |  | 0.00 |  | 293.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 324.44 | 288.51 | 0.00 |  | 0.00 |  | 288.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 311.77 | 275.84 | 0.00 |  | 0.00 |  | 275.84 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c1-cstate/http-recv-busy-10000, loop 0 on CPU 2

7282392836 iterations, longest 220.1 µs; over 5 / 20 / 100 µs: 506654 / 10932 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4034 | 96457.2 | 220.1 |
| mostly unexplained (+preempted by kworker/2:1) | 3889 | 93340.2 | 49.1 |
| mostly unexplained (+softirq NET_RX) | 1757 | 43227.9 | 45.9 |
| softirq NET_RX | 936 | 21044.5 | 42.6 |
| mostly unexplained (+local timer tick) | 176 | 5441.3 | 50.0 |
| mostly unexplained (+irq work (IPI/vector)) | 108 | 2665.2 | 47.2 |
| loop work | 17 | 413.4 | 39.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 14 | 392.0 | 79.8 |
| softirq TIMER | 1 | 133.2 | 133.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 220.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 219.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 198.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 186.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 135.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 133.2 | 1 | softirq TIMER | softirq TIMER 116, local timer tick 10, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 7 | 101.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 79.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 51.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 50.0 | 1 | mostly unexplained (+local timer tick) | local timer tick 7, preempted by kworker/2:1 5, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 21560 |
| IWI IRQ work interrupts | 21335 |
| CAL Function call interrupts | 1058 |
| MCP Machine check polls | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c1-cstate/http-recv-busy-10000 (median 38.30)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 492.76 | 454.46 | 0.00 |  | 0.00 |  | 454.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 470.31 | 432.01 | 0.00 |  | 0.00 |  | 432.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 450.88 | 412.58 | 0.00 |  | 0.00 |  | 412.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 449.69 | 411.39 | 0.00 |  | 0.00 |  | 411.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 427.31 | 389.01 | 0.00 |  | 0.00 |  | 389.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 418.74 | 380.44 | 0.00 |  | 0.00 |  | 380.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 417.13 | 378.83 | 0.00 |  | 0.00 |  | 378.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 397.72 | 359.42 | 0.00 |  | 0.00 |  | 359.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 387.10 | 348.80 | 0.00 |  | 0.00 |  | 348.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 382.57 | 344.27 | 0.00 |  | 0.00 |  | 344.27 | neither loop stalled long enough: network, hypervisor or untraced |

## c1-cstate/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1481257 clocksource=tsc

### Loop counters (c1-cstate/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002850 | 1674297 | 4.78 | 254768128 | 31.83 | 1592301 | 1592301 | 99.3 | 65.8 | true | 0 | 2142 | 161594.8 | 314297 / 13184 / 56 | 4002850 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792638400 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (135221 of 3897150 in 1-msg reads) | 135221 | 2.52 | 2.40 | 2.88 | 4.11 | 8.37 | 21.02 | 26.70 |

#### Stalls: c1-cstate/md-recv-busy, loop 0 on CPU 2

254768128 iterations, longest 161594.8 µs; over 5 / 20 / 100 µs: 314297 / 13184 / 56

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 12532 | 331442.9 | 4395.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 77 | 172822.6 | 161594.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 170 | 46827.4 | 8128.1 |
| softirq NET_RX | 260 | 6472.5 | 44.1 |
| mostly unexplained (+preempted by kworker/2:1) | 129 | 3167.3 | 42.7 |
| mostly unexplained (+softirq NET_RX) | 11 | 280.5 | 40.8 |
| mostly unexplained (+irq work (IPI/vector)) | 5 | 126.6 | 29.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 161594.8 | 570 | VM paused: the idle spare core stalled too (hypervisor) | softirq NET_RX 127, softirq NET_TX 5 |
| 2 | 8128.1 | 400 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 8014.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 8011.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 8008.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 8007.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 5629.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 4395.4 | 247 | loop work (247 items) | softirq NET_RX 32, softirq NET_TX 3 |
| 9 | 648.5 | 2850 | loop work (2850 items) |  |
| 10 | 462.5 | 2237 | loop work (2237 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2170 |
| LOC Local timer interrupts | 2144 |
| CAL Function call interrupts | 1012 |
| TLB TLB shootdowns | 2 |
| HYP Hypervisor callback interrupts | 1 |
| RES Rescheduling interrupts | 1 |

## c1-cstate/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c1-cstate/noise, loop 0 on CPU 2

10432598016 iterations, longest 255.7 µs; over 5 / 20 / 100 µs: 691 / 7 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 665 | 4429.0 | 255.7 |
| mostly unexplained (+call function (IPI/vector)) | 24 | 134.3 | 8.2 |
| local timer tick | 1 | 27.2 | 27.2 |
| mostly unexplained (+call function single (IPI/vector)) | 1 | 9.5 | 9.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 255.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 60.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 50.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 44.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 31.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 30.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 27.2 | 0 | local timer tick | local timer tick 16, timer tick_nohz_handler 10, softirq TIMER 3, vector_free_moved 0 |
| 8 | 20.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 17.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 15.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1005 |
| LOC Local timer interrupts | 1 |

### Loop counters (c1-cstate/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 778366016 | 162.16 | 1621596 | 1611596 | 99.4 | 67.8 | true | 0 | 27542 | 380.1 | 372497 / 5562 / 3 | 0 | 0 |

#### Stalls: c1-cstate/pp-recv-busy-10000, loop 0 on CPU 2

773566016 iterations, longest 380.1 µs; over 5 / 20 / 100 µs: 372497 / 5562 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1793 | 41804.2 | 380.1 |
| mostly unexplained (+preempted by kworker/2:1) | 1793 | 40197.6 | 40.5 |
| mostly unexplained (+local timer tick) | 774 | 17986.9 | 36.6 |
| softirq NET_RX | 604 | 14163.2 | 39.5 |
| mostly unexplained (+softirq NET_RX) | 556 | 12962.5 | 32.5 |
| mostly unexplained (+irq work (IPI/vector)) | 24 | 551.7 | 33.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 13 | 511.1 | 134.2 |
| loop work | 4 | 80.0 | 20.0 |
| softirq TIMER | 1 | 75.0 | 75.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 380.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 142.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 134.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 98.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 75.0 | 0 | softirq TIMER | softirq TIMER 65, local timer tick 5, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 6 | 56.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 50.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 48.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 46.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 42.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 30357 |
| IWI IRQ work interrupts | 27558 |
| CAL Function call interrupts | 1071 |
| MCP Machine check polls | 2 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: c1-cstate/pp-recv-busy-10000 (median 39.86)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 464.79 | 424.93 | 0.00 |  | 0.00 |  | 424.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 428.53 | 388.67 | 0.00 |  | 0.00 |  | 388.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 382.52 | 342.66 | 0.00 |  | 0.00 |  | 342.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 371.26 | 331.40 | 0.00 |  | 0.00 |  | 331.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 364.52 | 324.66 | 0.00 |  | 0.00 |  | 324.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 357.33 | 317.46 | 0.00 |  | 0.00 |  | 317.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 336.95 | 297.09 | 0.00 |  | 0.00 |  | 297.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 335.68 | 295.81 | 0.00 |  | 0.00 |  | 295.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 333.91 | 294.05 | 0.00 |  | 0.00 |  | 294.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 325.89 | 286.03 | 0.00 |  | 0.00 |  | 286.03 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c1-noautocork/http-recv-busy-10000, loop 0 on CPU 2

7254993186 iterations, longest 148.7 µs; over 5 / 20 / 100 µs: 1021018 / 10974 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 4485 | 109753.1 | 53.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2985 | 67573.3 | 148.7 |
| mostly unexplained (+softirq NET_RX) | 2111 | 50592.4 | 107.1 |
| softirq NET_RX | 1256 | 29934.6 | 40.5 |
| mostly unexplained (+irq work (IPI/vector)) | 113 | 2838.6 | 35.1 |
| loop work | 16 | 378.5 | 28.6 |
| softirq TIMER | 2 | 149.9 | 76.0 |
| mostly unexplained (+local timer tick) | 4 | 88.9 | 24.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 70.8 | 47.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 148.7 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 112.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 110.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 107.1 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 102.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 76.0 | 0 | softirq TIMER | softirq TIMER 63, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 7 | 74.0 | 0 | softirq TIMER | softirq TIMER 61, local timer tick 5, timer tick_nohz_handler 3 |
| 8 | 53.5 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 51.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 49.2 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 20729 |
| IWI IRQ work interrupts | 20346 |
| CAL Function call interrupts | 1057 |
| MCP Machine check polls | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c1-noautocork/http-recv-busy-10000 (median 38.42)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 484.50 | 446.08 | 0.00 |  | 0.00 |  | 446.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 446.28 | 407.86 | 73.98 | softirq TIMER | 0.00 |  | 333.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 373.18 | 334.76 | 0.00 |  | 22.75 | mostly unexplained (+preempted by kworker/1:1) | 312.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 367.61 | 329.19 | 0.00 |  | 0.00 |  | 329.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 361.26 | 322.84 | 0.00 |  | 0.00 |  | 322.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 331.66 | 293.24 | 0.00 |  | 0.00 |  | 293.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 318.09 | 279.67 | 0.00 |  | 0.00 |  | 279.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 314.82 | 276.41 | 0.00 |  | 0.00 |  | 276.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 304.18 | 265.76 | 0.00 |  | 0.00 |  | 265.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 295.55 | 257.14 | 0.00 |  | 0.00 |  | 257.14 | neither loop stalled long enough: network, hypervisor or untraced |

## c1-noautocork/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1368661 clocksource=tsc

### Loop counters (c1-noautocork/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002850 | 1515299 | 5.28 | 255837504 | 31.97 | 1598984 | 1598984 | 99.4 | 65.9 | true | 0 | 1990 | 651.6 | 388209 / 11312 / 3 | 4002850 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792638400 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (92841 of 3897100 in 1-msg reads) | 92841 | 2.85 | 2.64 | 3.43 | 5.97 | 10.27 | 20.02 | 30.38 |

#### Stalls: c1-noautocork/md-recv-busy, loop 0 on CPU 2

255837504 iterations, longest 651.6 µs; over 5 / 20 / 100 µs: 388209 / 11312 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 11061 | 276751.3 | 651.6 |
| softirq NET_RX | 123 | 2829.5 | 42.5 |
| mostly unexplained (+preempted by kworker/2:1) | 67 | 1534.0 | 34.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 50 | 1298.7 | 106.0 |
| mostly unexplained (+softirq NET_RX) | 5 | 115.2 | 26.8 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 111.6 | 29.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 61.2 | 38.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 651.6 | 2870 | loop work (2870 items) |  |
| 2 | 106.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 101.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 59.7 | 19 | loop work (19 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 56.9 | 30 | loop work (30 items) | softirq NET_RX 1 |
| 6 | 54.0 | 20 | loop work (20 items) | preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 52.6 | 13 | loop work (13 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 52.1 | 28 | loop work (28 items) | softirq NET_RX 7 |
| 9 | 51.7 | 34 | loop work (34 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 50.4 | 40 | loop work (40 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2025 |
| LOC Local timer interrupts | 1992 |
| CAL Function call interrupts | 1012 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## c1-noautocork/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c1-noautocork/noise, loop 0 on CPU 2

10433171456 iterations, longest 193.7 µs; over 5 / 20 / 100 µs: 423 / 6 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 406 | 2648.4 | 39.9 |
| softirq TIMER | 1 | 193.7 | 193.7 |
| mostly unexplained (+call function (IPI/vector)) | 16 | 87.8 | 7.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 193.7 | 0 | softirq TIMER | softirq TIMER 175, local timer tick 12, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 39.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 25.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 24.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 22.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 21.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 18.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 16.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 14.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 13.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |
| IWI IRQ work interrupts | 1 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (c1-noautocork/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 774568320 | 161.37 | 1613684 | 1603684 | 99.4 | 68.0 | true | 0 | 10471 | 161.5 | 355958 / 4476 / 3 | 0 | 0 |

#### Stalls: c1-noautocork/pp-recv-busy-10000, loop 0 on CPU 2

769768320 iterations, longest 161.5 µs; over 5 / 20 / 100 µs: 355958 / 4476 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 2620 | 63920.9 | 35.2 |
| softirq NET_RX | 660 | 16103.1 | 43.9 |
| mostly unexplained (+softirq NET_RX) | 614 | 15957.6 | 52.2 |
| mostly unexplained (+local timer tick) | 371 | 10258.8 | 38.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 184 | 5237.4 | 161.5 |
| mostly unexplained (+irq work (IPI/vector)) | 18 | 462.2 | 32.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 6 | 139.8 | 28.6 |
| softirq TIMER | 1 | 72.8 | 72.8 |
| mostly unexplained (+ipi_send_cpu) | 1 | 26.5 | 26.5 |
| loop work | 1 | 20.0 | 20.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 161.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 133.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 105.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 98.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 72.8 | 0 | softirq TIMER | softirq TIMER 61, local timer tick 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 6 | 53.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 53.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 52.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 26, preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 43.9 | 1 | softirq NET_RX | softirq NET_RX 25, local timer tick 10, preempted by ksoftirqd/2 9, timer tick_nohz_handler 7 |
| 10 | 42.1 | 1 | softirq NET_RX | softirq NET_RX 24, local timer tick 8, preempted by ksoftirqd/2 7, timer tick_nohz_handler 5 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 11486 |
| IWI IRQ work interrupts | 10489 |
| CAL Function call interrupts | 1058 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c1-noautocork/pp-recv-busy-10000 (median 38.17)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 966.49 | 928.32 | 0.00 |  | 0.00 |  | 928.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 360.02 | 321.86 | 0.00 |  | 0.00 |  | 321.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 344.69 | 306.53 | 0.00 |  | 0.00 |  | 306.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 324.80 | 286.63 | 0.00 |  | 0.00 |  | 286.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 315.65 | 277.48 | 0.00 |  | 0.00 |  | 277.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 288.65 | 250.48 | 0.00 |  | 0.00 |  | 250.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 280.66 | 242.49 | 0.00 |  | 0.00 |  | 242.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 279.20 | 241.03 | 0.00 |  | 0.00 |  | 241.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 262.27 | 224.10 | 0.00 |  | 0.00 |  | 224.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 258.30 | 220.13 | 0.00 |  | 0.00 |  | 220.13 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c2-base/http-recv-busy-10000, loop 0 on CPU 2

8093415343 iterations, longest 210.8 µs; over 5 / 20 / 100 µs: 677442 / 4149 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 2434 | 67502.0 | 51.7 |
| mostly unexplained (+softirq NET_RX) | 787 | 20902.4 | 132.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 457 | 10408.0 | 107.0 |
| softirq NET_RX | 229 | 6186.9 | 50.9 |
| mostly unexplained (+local timer tick) | 150 | 5146.4 | 49.9 |
| mostly unexplained (+irq work (IPI/vector)) | 80 | 1864.6 | 37.5 |
| softirq TIMER | 2 | 340.5 | 210.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 8 | 211.9 | 34.2 |
| loop work | 2 | 47.5 | 27.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 210.8 | 1 | softirq TIMER | softirq TIMER 197, local timer tick 8, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 132.2 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 3 | 129.7 | 1 | softirq TIMER | softirq TIMER 114, local timer tick 10, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 4 | 107.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 95.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 52.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 20, preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 51.7 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 50.9 | 1 | softirq NET_RX | softirq NET_RX 29, preempted by ksoftirqd/2 10, local timer tick 8, timer tick_nohz_handler 4 |
| 9 | 50.9 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 50.7 | 1 | softirq NET_RX | softirq NET_RX 33, local timer tick 10, preempted by ksoftirqd/2 10, timer tick_nohz_handler 6 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 5182 |
| IWI IRQ work interrupts | 4896 |
| CAL Function call interrupts | 1145 |
| MCP Machine check polls | 2 |

#### Slowest round trips: c2-base/http-recv-busy-10000 (median 37.56)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1049.79 | 1012.23 | 0.00 |  | 108.18 | unexplained: no kernel event (hypervisor or in-kernel time) | 904.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 560.04 | 522.48 | 0.00 |  | 0.00 |  | 522.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 512.93 | 475.37 | 0.00 |  | 0.00 |  | 475.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 435.83 | 398.27 | 0.00 |  | 0.00 |  | 398.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 395.93 | 358.37 | 0.00 |  | 0.00 |  | 358.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 381.77 | 344.21 | 0.00 |  | 0.00 |  | 344.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 375.28 | 337.72 | 0.00 |  | 0.00 |  | 337.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 367.71 | 330.15 | 0.00 |  | 0.00 |  | 330.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 320.79 | 283.23 | 0.00 |  | 0.00 |  | 283.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 319.90 | 282.34 | 0.00 |  | 0.00 |  | 282.34 | neither loop stalled long enough: network, hypervisor or untraced |

## c2-base/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1604066 clocksource=tsc

### Loop counters (c2-base/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002750 | 1745503 | 4.58 | 282483456 | 35.30 | 1765522 | 1765522 | 99.4 | 65.7 | true | 0 | 1890 | 621.9 | 314263 / 14049 / 2 | 4002750 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (122395 of 3897200 in 1-msg reads) | 122395 | 2.67 | 2.41 | 3.44 | 5.56 | 14.99 | 32.26 | 41.52 |

#### Stalls: c2-base/md-recv-busy, loop 0 on CPU 2

282483456 iterations, longest 621.9 µs; over 5 / 20 / 100 µs: 314263 / 14049 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 13598 | 372852.5 | 621.9 |
| mostly unexplained (+preempted by kworker/2:1) | 156 | 4203.0 | 58.8 |
| softirq NET_RX | 144 | 3677.4 | 39.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 104 | 2766.7 | 119.9 |
| mostly unexplained (+softirq NET_RX) | 32 | 923.7 | 44.2 |
| mostly unexplained (+local timer tick) | 9 | 301.2 | 45.0 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 81.0 | 29.5 |
| softirq TIMER | 1 | 69.8 | 69.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 43.0 | 22.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 621.9 | 2753 | loop work (2753 items) |  |
| 2 | 119.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 98.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 72.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 69.8 | 0 | softirq TIMER | softirq TIMER 60, local timer tick 5, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 6 | 69.6 | 16 | loop work (16 items) | softirq NET_RX 25, preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 67.0 | 15 | loop work (15 items) | preempted by kworker/2:1 13, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 8 | 66.8 | 40 | loop work (40 items) | local timer tick 5, timer tick_nohz_handler 2 |
| 9 | 66.4 | 23 | loop work (23 items) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 64.8 | 42 | loop work (42 items) | local timer tick 3, softirq NET_TX 2, timer tick_nohz_handler 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 1996 |
| IWI IRQ work interrupts | 1911 |
| CAL Function call interrupts | 1008 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## c2-base/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c2-base/noise, loop 0 on CPU 2

10433372160 iterations, longest 135.4 µs; over 5 / 20 / 100 µs: 335 / 5 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 313 | 2104.6 | 108.8 |
| softirq TIMER | 1 | 135.4 | 135.4 |
| mostly unexplained (+call function (IPI/vector)) | 21 | 111.2 | 7.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 135.4 | 0 | softirq TIMER | softirq TIMER 116, local timer tick 12, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 108.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 24.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 24.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 24.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 18.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 16.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 14.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 12.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 12.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |
| IWI IRQ work interrupts | 1 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (c2-base/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 777294016 | 161.94 | 1619363 | 1609363 | 99.4 | 67.9 | true | 0 | 8724 | 170.3 | 216140 / 2157 / 2 | 0 | 0 |

#### Stalls: c2-base/pp-recv-busy-10000, loop 0 on CPU 2

772494016 iterations, longest 170.3 µs; over 5 / 20 / 100 µs: 216140 / 2157 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 1439 | 32954.0 | 41.4 |
| softirq NET_RX | 503 | 11167.1 | 33.8 |
| mostly unexplained (+softirq NET_RX) | 167 | 4055.1 | 41.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 24 | 1055.9 | 170.3 |
| mostly unexplained (+irq work (IPI/vector)) | 17 | 407.5 | 39.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 6 | 148.3 | 32.7 |
| softirq TIMER | 1 | 72.7 | 72.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 170.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 124.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 100.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 96.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 91.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 72.7 | 0 | softirq TIMER | softirq TIMER 62, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 7 | 41.4 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 8, local timer tick 4, irq work (IPI/vector) 1, softirq RCU 1 |
| 8 | 41.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/2:1 6, local timer tick 4, irq work (IPI/vector) 1 |
| 9 | 41.0 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 39.2 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 8839 |
| IWI IRQ work interrupts | 8739 |
| CAL Function call interrupts | 1058 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c2-base/pp-recv-busy-10000 (median 37.69)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 967.90 | 930.22 | 0.00 |  | 0.00 |  | 930.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 507.55 | 469.87 | 0.00 |  | 0.00 |  | 469.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 462.13 | 424.45 | 0.00 |  | 0.00 |  | 424.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 455.62 | 417.93 | 0.00 |  | 0.00 |  | 417.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 436.54 | 398.86 | 0.00 |  | 0.00 |  | 398.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 431.50 | 393.81 | 0.00 |  | 0.00 |  | 393.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 395.53 | 357.84 | 0.00 |  | 0.00 |  | 357.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 392.92 | 355.23 | 0.00 |  | 0.00 |  | 355.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 368.99 | 331.31 | 0.00 |  | 0.00 |  | 331.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 317.37 | 279.69 | 0.00 |  | 0.00 |  | 279.69 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c2-irqlocal/http-recv-busy-10000, loop 0 on CPU 2

7376802789 iterations, longest 107.8 µs; over 5 / 20 / 100 µs: 4802948 / 14991 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 14756 | 329619.3 | 107.8 |
| mostly unexplained (+softirq NET_RX) | 75 | 1636.6 | 29.9 |
| mostly unexplained (+irq enp39s0-Tx-Rx-3) | 36 | 774.5 | 30.2 |
| mostly unexplained (+local timer tick) | 36 | 768.6 | 23.8 |
| softirq NET_RX | 33 | 731.7 | 33.5 |
| mostly unexplained (+preempted by kworker/2:1) | 32 | 704.9 | 29.6 |
| loop work | 13 | 295.2 | 32.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 10 | 266.3 | 34.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 107.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 103.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 78.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 55.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 53.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 53.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 44.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 44.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 44.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 40.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 9597293 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 9377667 |
| IWI IRQ work interrupts | 66488 |
| LOC Local timer interrupts | 21812 |
| CAL Function call interrupts | 1057 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 668 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 562 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 529 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 504 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 489 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 393 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 357 |

#### Slowest round trips: c2-irqlocal/http-recv-busy-10000 (median 38.55)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 449.35 | 410.80 | 0.00 |  | 0.00 |  | 410.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 439.97 | 401.42 | 0.00 |  | 0.00 |  | 401.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 377.45 | 338.90 | 0.00 |  | 0.00 |  | 338.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 334.89 | 296.35 | 0.00 |  | 0.00 |  | 296.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 328.44 | 289.89 | 0.00 |  | 0.00 |  | 289.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 320.21 | 281.66 | 0.00 |  | 0.00 |  | 281.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 305.64 | 267.09 | 0.00 |  | 0.00 |  | 267.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 294.17 | 255.63 | 0.00 |  | 0.00 |  | 255.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 291.47 | 252.93 | 0.00 |  | 0.00 |  | 252.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 290.35 | 251.81 | 0.00 |  | 0.00 |  | 251.81 | neither loop stalled long enough: network, hypervisor or untraced |

## c2-irqlocal/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1339422 clocksource=tsc

### Loop counters (c2-irqlocal/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002950 | 1528588 | 5.24 | 283953280 | 35.48 | 1774708 | 1774708 | 99.5 | 65.5 | true | 0 | 5138 | 755.7 | 997216 / 29014 / 4 | 4002950 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792660800 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (62748 of 3897041 in 1-msg reads) | 62748 | 3.87 | 2.86 | 6.07 | 12.94 | 20.87 | 29.69 | 35.54 |

#### Stalls: c2-irqlocal/md-recv-busy, loop 0 on CPU 2

283953280 iterations, longest 755.7 µs; over 5 / 20 / 100 µs: 997216 / 29014 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 27277 | 764464.8 | 755.7 |
| softirq NET_RX | 1027 | 26417.7 | 71.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 590 | 14518.8 | 122.9 |
| mostly unexplained (+irq enp39s0-Tx-Rx-3) | 38 | 935.2 | 32.2 |
| mostly unexplained (+irq enp39s0-Tx-Rx-7) | 31 | 729.7 | 30.7 |
| mostly unexplained (+softirq NET_RX) | 29 | 671.0 | 33.4 |
| mostly unexplained (+preempted by kworker/2:1) | 19 | 456.5 | 32.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 36.6 | 36.6 |
| softirq TIMER | 1 | 30.5 | 30.5 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 29.8 | 29.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 755.7 | 2950 | loop work (2950 items) |  |
| 2 | 122.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 102.4 | 7 | loop work (7 items) |  |
| 4 | 101.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 82.8 | 12 | loop work (12 items) |  |
| 6 | 80.7 | 39 | loop work (39 items) |  |
| 7 | 79.6 | 16 | loop work (16 items) |  |
| 8 | 78.2 | 25 | loop work (25 items) |  |
| 9 | 75.4 | 7 | loop work (7 items) |  |
| 10 | 75.4 | 16 | loop work (16 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 1222029 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 573338 |
| IWI IRQ work interrupts | 7020 |
| LOC Local timer interrupts | 2923 |
| CAL Function call interrupts | 1006 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 265 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 250 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 191 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 173 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 169 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 164 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 163 |

## c2-irqlocal/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c2-irqlocal/noise, loop 0 on CPU 2

11844190208 iterations, longest 40.7 µs; over 5 / 20 / 100 µs: 1233 / 59 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq NET_RX | 532 | 6364.2 | 40.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 278 | 1775.8 | 24.3 |
| mostly unexplained (+softirq NET_RX) | 209 | 1751.4 | 32.6 |
| mostly unexplained (+local timer tick) | 107 | 1292.1 | 26.2 |
| local timer tick | 36 | 453.3 | 20.8 |
| mostly unexplained (+irq ena-mgmnt@pci:0000:27:00.0) | 55 | 432.4 | 12.6 |
| mostly unexplained (+irq enp39s0-Tx-Rx-1) | 6 | 35.4 | 8.1 |
| mostly unexplained (+irq enp39s0-Tx-Rx-7) | 4 | 25.0 | 7.5 |
| mostly unexplained (+irq enp39s0-Tx-Rx-6) | 2 | 13.1 | 8.1 |
| mostly unexplained (+irq enp39s0-Tx-Rx-2) | 2 | 10.2 | 5.1 |
| mostly unexplained (+irq enp39s0-Tx-Rx-3) | 1 | 8.9 | 8.9 |
| mostly unexplained (+irq enp39s0-Tx-Rx-5) | 1 | 7.9 | 7.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 40.7 | 0 | softirq NET_RX | softirq NET_RX 27, preempted by kworker/2:1 5, irq work (IPI/vector) 3, irq enp39s0-Tx-Rx-5 1 |
| 2 | 37.5 | 0 | softirq NET_RX | softirq NET_RX 25, preempted by kworker/2:1 3, irq enp39s0-Tx-Rx-0 1, irq enp39s0-Tx-Rx-5 1 |
| 3 | 36.2 | 0 | softirq NET_RX | softirq NET_RX 22, preempted by kworker/2:1 5, irq enp39s0-Tx-Rx-0 1, irq enp39s0-Tx-Rx-7 1 |
| 4 | 34.5 | 0 | softirq NET_RX | softirq NET_RX 21, preempted by kworker/2:1 6, irq enp39s0-Tx-Rx-2 1, irq enp39s0-Tx-Rx-7 1 |
| 5 | 34.0 | 0 | softirq NET_RX | softirq NET_RX 26, irq enp39s0-Tx-Rx-6 1, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 32.6 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/2:1 4, irq work (IPI/vector) 2, irq enp39s0-Tx-Rx-7 1 |
| 7 | 27.4 | 0 | softirq NET_RX | softirq NET_RX 14, irq enp39s0-Tx-Rx-2 1, irq enp39s0-Tx-Rx-3 1 |
| 8 | 27.2 | 0 | softirq NET_RX | softirq NET_RX 17, preempted by kworker/2:1 3, irq enp39s0-Tx-Rx-1 1, irq work (IPI/vector) 1 |
| 9 | 26.3 | 0 | softirq NET_RX | softirq NET_RX 17, irq work (IPI/vector) 2, irq enp39s0-Tx-Rx-6 1, ipi_send_cpu 0 |
| 10 | 26.2 | 0 | mostly unexplained (+local timer tick) | local timer tick 9, timer tick_nohz_handler 5, softirq TIMER 2, irq enp39s0-Tx-Rx-1 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 276 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 265 |
| IWI IRQ work interrupts | 257 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 252 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 197 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 160 |
| LOC Local timer interrupts | 145 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 143 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 143 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 107 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 87 |

### Loop counters (c2-irqlocal/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 778218624 | 162.13 | 1621289 | 1611289 | 99.4 | 69.2 | true | 0 | 42989 | 214.5 | 6373700 / 19291 / 1 | 0 | 0 |

#### Stalls: c2-irqlocal/pp-recv-busy-10000, loop 0 on CPU 2

773418624 iterations, longest 214.5 µs; over 5 / 20 / 100 µs: 6373700 / 19291 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 18844 | 417885.2 | 214.5 |
| softirq NET_RX | 117 | 2589.3 | 27.3 |
| mostly unexplained (+local timer tick) | 108 | 2452.1 | 26.9 |
| mostly unexplained (+preempted by kworker/2:1) | 71 | 1560.9 | 26.8 |
| mostly unexplained (+softirq NET_RX) | 66 | 1502.9 | 26.2 |
| mostly unexplained (+irq enp39s0-Tx-Rx-3) | 63 | 1387.6 | 26.3 |
| mostly unexplained (+irq enp39s0-Tx-Rx-1) | 12 | 247.6 | 21.3 |
| loop work | 6 | 120.0 | 20.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 113.0 | 31.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 214.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 97.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 90.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 68.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 42.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 39.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 39.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 38.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 38.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 38.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 9597186 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 9400169 |
| IWI IRQ work interrupts | 61382 |
| LOC Local timer interrupts | 25166 |
| CAL Function call interrupts | 1062 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 597 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 568 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 555 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 538 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 489 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 460 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 441 |

#### Slowest round trips: c2-irqlocal/pp-recv-busy-10000 (median 37.52)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 350.42 | 312.90 | 0.00 |  | 0.00 |  | 312.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 323.08 | 285.56 | 0.00 |  | 0.00 |  | 285.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 319.99 | 282.47 | 0.00 |  | 0.00 |  | 282.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 308.06 | 270.54 | 0.00 |  | 0.00 |  | 270.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 302.95 | 265.44 | 0.00 |  | 0.00 |  | 265.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 299.38 | 261.86 | 0.00 |  | 0.00 |  | 261.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 295.29 | 257.78 | 0.00 |  | 0.00 |  | 257.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 293.60 | 256.09 | 0.00 |  | 0.00 |  | 256.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 276.63 | 239.11 | 0.00 |  | 0.00 |  | 239.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 269.49 | 231.98 | 0.00 |  | 0.00 |  | 231.98 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c2-onequeue/http-recv-busy-10000, loop 0 on CPU 2

8516336843 iterations, longest 258.3 µs; over 5 / 20 / 100 µs: 160621 / 1877 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 1345 | 34704.2 | 48.4 |
| softirq NET_RX | 183 | 4770.2 | 39.9 |
| mostly unexplained (+softirq NET_RX) | 156 | 4011.1 | 60.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 125 | 3172.5 | 120.8 |
| mostly unexplained (+irq work (IPI/vector)) | 61 | 1466.0 | 31.9 |
| softirq TIMER | 1 | 258.3 | 258.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 104.2 | 32.1 |
| loop work | 1 | 36.8 | 36.8 |
| mostly unexplained (+ipi_send_cpu) | 1 | 22.8 | 22.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 258.3 | 2 | softirq TIMER | softirq TIMER 233, local timer tick 14, timer tick_nohz_handler 7, call function single (IPI/vector) 1 |
| 2 | 120.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 100.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 90.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 77.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 60.1 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 30, preempted by kworker/2:1 9, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 55.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 48.4 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 47.5 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 46.7 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2903 |
| LOC Local timer interrupts | 2894 |
| CAL Function call interrupts | 1059 |
| MCP Machine check polls | 1 |

#### Slowest round trips: c2-onequeue/http-recv-busy-10000 (median 37.64)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 395.33 | 357.69 | 0.00 |  | 0.00 |  | 357.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 383.01 | 345.37 | 0.00 |  | 0.00 |  | 345.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 338.44 | 300.80 | 0.00 |  | 0.00 |  | 300.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 321.41 | 283.77 | 0.00 |  | 0.00 |  | 283.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 310.82 | 273.18 | 0.00 |  | 0.00 |  | 273.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 287.95 | 250.31 | 0.00 |  | 0.00 |  | 250.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 285.91 | 248.27 | 0.00 |  | 0.00 |  | 248.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 282.49 | 244.85 | 0.00 |  | 0.00 |  | 244.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 269.65 | 232.01 | 257.22 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 10 | 265.52 | 227.87 | 0.00 |  | 0.00 |  | 227.87 | neither loop stalled long enough: network, hypervisor or untraced |

## c2-onequeue/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1857336 clocksource=tsc

### Loop counters (c2-onequeue/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002750 | 2014196 | 3.97 | 292051584 | 36.49 | 1825322 | 1825322 | 99.3 | 65.5 | true | 0 | 1643 | 711.9 | 259864 / 11508 / 3 | 4002750 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (175053 of 3897250 in 1-msg reads) | 175053 | 2.29 | 2.15 | 2.64 | 4.21 | 8.41 | 21.48 | 33.92 |

#### Stalls: c2-onequeue/md-recv-busy, loop 0 on CPU 2

292051584 iterations, longest 711.9 µs; over 5 / 20 / 100 µs: 259864 / 11508 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 11092 | 279876.5 | 711.9 |
| mostly unexplained (+preempted by kworker/2:1) | 173 | 4558.9 | 48.6 |
| softirq NET_RX | 129 | 3258.1 | 36.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 78 | 1978.2 | 116.8 |
| mostly unexplained (+softirq NET_RX) | 17 | 471.7 | 40.0 |
| mostly unexplained (+irq work (IPI/vector)) | 16 | 360.2 | 27.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 73.6 | 30.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 711.9 | 2750 | loop work (2750 items) |  |
| 2 | 116.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 101.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 68.1 | 12 | loop work (12 items) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 65.1 | 16 | loop work (16 items) | softirq NET_RX 25, preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 63.8 | 15 | loop work (15 items) | softirq NET_RX 24, preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 63.1 | 13 | loop work (13 items) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 62.8 | 7 | loop work (7 items) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 62.6 | 21 | loop work (21 items) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 62.5 | 21 | loop work (21 items) | softirq NET_RX 17, preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1776 |
| LOC Local timer interrupts | 1648 |
| CAL Function call interrupts | 1011 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## c2-onequeue/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c2-onequeue/noise, loop 0 on CPU 2

11937423360 iterations, longest 24.6 µs; over 5 / 20 / 100 µs: 328 / 4 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 320 | 2025.4 | 24.6 |
| mostly unexplained (+call function (IPI/vector)) | 8 | 44.3 | 7.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 24.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 23.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 23.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 20.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 18.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 18.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 17.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 17.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 15.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |

### Loop counters (c2-onequeue/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 901398848 | 187.79 | 1877914 | 1867914 | 99.5 | 67.8 | true | 0 | 9550 | 254.6 | 81196 / 2329 / 3 | 0 | 0 |

#### Stalls: c2-onequeue/pp-recv-busy-10000, loop 0 on CPU 2

896598848 iterations, longest 254.6 µs; over 5 / 20 / 100 µs: 81196 / 2329 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 1293 | 30076.4 | 39.5 |
| softirq NET_RX | 494 | 11358.6 | 36.7 |
| mostly unexplained (+softirq NET_RX) | 243 | 5896.7 | 107.5 |
| mostly unexplained (+local timer tick) | 152 | 3638.0 | 33.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 99 | 2518.7 | 114.4 |
| mostly unexplained (+irq work (IPI/vector)) | 40 | 906.9 | 31.4 |
| softirq TIMER | 2 | 325.5 | 254.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 5 | 129.2 | 31.1 |
| mostly unexplained (+ipi_send_cpu) | 1 | 21.9 | 21.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 254.6 | 0 | softirq TIMER | softirq TIMER 241, local timer tick 7, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 114.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 107.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 4 | 95.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 71.0 | 0 | softirq TIMER | softirq TIMER 56, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 6 | 39.5 | 0 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 37.2 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 36.7 | 0 | softirq NET_RX | softirq NET_RX 22, local timer tick 7, preempted by ksoftirqd/2 7, timer tick_nohz_handler 5 |
| 9 | 36.2 | 0 | softirq NET_RX | softirq NET_RX 22, local timer tick 7, preempted by ksoftirqd/2 7, timer tick_nohz_handler 4 |
| 10 | 35.5 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 10203 |
| IWI IRQ work interrupts | 9635 |
| CAL Function call interrupts | 1059 |
| MCP Machine check polls | 2 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: c2-onequeue/pp-recv-busy-10000 (median 38.93)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 966.89 | 927.96 | 0.00 |  | 0.00 |  | 927.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 692.85 | 653.92 | 0.00 |  | 0.00 |  | 653.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 445.51 | 406.58 | 0.00 |  | 0.00 |  | 406.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 443.28 | 404.35 | 0.00 |  | 0.00 |  | 404.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 421.58 | 382.65 | 0.00 |  | 0.00 |  | 382.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 408.05 | 369.12 | 0.00 |  | 0.00 |  | 369.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 322.95 | 284.01 | 0.00 |  | 0.00 |  | 284.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 312.71 | 273.78 | 0.00 |  | 0.00 |  | 273.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 309.17 | 270.24 | 0.00 |  | 0.00 |  | 270.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 307.88 | 268.95 | 0.00 |  | 0.00 |  | 268.95 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c2-cstate/http-recv-busy-10000, loop 0 on CPU 2

8298622700 iterations, longest 344.3 µs; over 5 / 20 / 100 µs: 254832 / 7104 / 15

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 3004 | 71324.1 | 147.5 |
| mostly unexplained (+preempted by kworker/2:1) | 2263 | 53046.7 | 50.6 |
| mostly unexplained (+softirq NET_RX) | 1163 | 27682.2 | 141.0 |
| softirq NET_RX | 520 | 11836.1 | 42.0 |
| mostly unexplained (+irq work (IPI/vector)) | 112 | 2663.8 | 39.1 |
| mostly unexplained (+local timer tick) | 27 | 971.1 | 48.6 |
| softirq TIMER | 2 | 497.8 | 344.3 |
| loop work | 10 | 230.6 | 35.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 78.3 | 57.8 |
| mostly unexplained (+call function (IPI/vector)) | 1 | 27.7 | 27.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 344.3 | 1 | softirq TIMER | softirq TIMER 331, local timer tick 7, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 153.6 | 0 | softirq TIMER | softirq TIMER 144, local timer tick 5, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 3 | 147.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 147.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 144.9 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 144.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 141.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 141.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 141.0 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 4 |
| 10 | 139.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 21072 |
| IWI IRQ work interrupts | 21051 |
| CAL Function call interrupts | 1057 |
| MCP Machine check polls | 2 |

#### Slowest round trips: c2-cstate/http-recv-busy-10000 (median 38.62)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 227.01 | 188.39 | 0.00 |  | 188.05 | softirq TIMER | 0.34 | server: softirq TIMER |
| 2 | 153.21 | 114.59 | 144.33 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 152.93 | 114.31 | 147.52 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 150.81 | 112.19 | 143.92 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 145.67 | 107.06 | 140.44 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 6 | 137.56 | 98.94 | 0.00 |  | 0.00 |  | 98.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 135.95 | 97.33 | 0.00 |  | 0.00 |  | 97.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 123.35 | 84.73 | 0.00 |  | 0.00 |  | 84.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 119.16 | 80.54 | 0.00 |  | 0.00 |  | 80.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 119.14 | 80.53 | 0.00 |  | 0.00 |  | 80.53 | neither loop stalled long enough: network, hypervisor or untraced |

## c2-cstate/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1377722 clocksource=tsc

### Loop counters (c2-cstate/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002750 | 1535676 | 5.21 | 287271744 | 35.90 | 1795448 | 1795448 | 99.5 | 65.3 | true | 0 | 2296 | 590.9 | 309862 / 13532 / 6 | 4002750 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (99774 of 3897234 in 1-msg reads) | 99774 | 2.69 | 2.41 | 3.40 | 6.44 | 18.36 | 30.48 | 32.70 |

#### Stalls: c2-cstate/md-recv-busy, loop 0 on CPU 2

287271744 iterations, longest 590.9 µs; over 5 / 20 / 100 µs: 309862 / 13532 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 12942 | 363763.5 | 590.9 |
| softirq NET_RX | 236 | 6121.2 | 39.5 |
| mostly unexplained (+preempted by kworker/2:1) | 217 | 5379.5 | 33.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 118 | 3109.5 | 107.2 |
| mostly unexplained (+softirq NET_RX) | 14 | 339.5 | 30.0 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 123.7 | 34.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 20.8 | 20.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 590.9 | 2750 | loop work (2750 items) |  |
| 2 | 155.7 | 39 | loop work (39 items) |  |
| 3 | 153.9 | 40 | loop work (40 items) |  |
| 4 | 151.6 | 50 | loop work (50 items) |  |
| 5 | 107.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 106.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 80.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 67.6 | 16 | loop work (16 items) | softirq NET_RX 14, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 62.6 | 24 | loop work (24 items) | softirq NET_RX 20, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 60.6 | 20 | loop work (20 items) | softirq NET_RX 23, preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2321 |
| LOC Local timer interrupts | 2299 |
| CAL Function call interrupts | 1011 |
| TLB TLB shootdowns | 2 |
| RES Rescheduling interrupts | 1 |

## c2-cstate/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c2-cstate/noise, loop 0 on CPU 2

11730305024 iterations, longest 2581.6 µs; over 5 / 20 / 100 µs: 835 / 10 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 823 | 5269.8 | 128.7 |
| mostly unexplained (+softirq NET_RX) | 1 | 2581.6 | 2581.6 |
| softirq TIMER | 1 | 116.1 | 116.1 |
| local timer tick | 2 | 42.4 | 22.8 |
| mostly unexplained (+local timer tick) | 2 | 42.0 | 25.7 |
| mostly unexplained (+call function (IPI/vector)) | 4 | 21.3 | 5.8 |
| mostly unexplained (+call function single (IPI/vector)) | 2 | 17.1 | 10.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 2581.6 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 51, irq enp39s0-Tx-Rx-5 1, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 128.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 116.1 | 0 | softirq TIMER | softirq TIMER 98, local timer tick 10, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 4 | 111.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 109.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 83.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 60.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 27.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 25.7 | 0 | mostly unexplained (+local timer tick) | local timer tick 11, timer tick_nohz_handler 5, softirq TIMER 3 |
| 10 | 22.8 | 0 | local timer tick | local timer tick 12, timer tick_nohz_handler 7, softirq TIMER 2, vector_free_moved 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1007 |
| LOC Local timer interrupts | 7 |
| IWI IRQ work interrupts | 2 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 1 |
| MCP Machine check polls | 1 |

### Loop counters (c2-cstate/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 903439680 | 188.22 | 1882166 | 1872166 | 99.5 | 68.0 | true | 0 | 29100 | 145.9 | 203423 / 2781 / 14 | 0 | 0 |

#### Stalls: c2-cstate/pp-recv-busy-10000, loop 0 on CPU 2

898639680 iterations, longest 145.9 µs; over 5 / 20 / 100 µs: 203423 / 2781 / 14

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 1556 | 35021.0 | 30.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 675 | 16982.1 | 145.9 |
| mostly unexplained (+softirq NET_RX) | 356 | 8222.2 | 30.8 |
| softirq NET_RX | 176 | 3888.8 | 29.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 5 | 231.0 | 83.6 |
| mostly unexplained (+irq work (IPI/vector)) | 8 | 212.4 | 29.3 |
| softirq TIMER | 1 | 111.4 | 111.4 |
| mostly unexplained (+ipi_send_cpu) | 2 | 41.2 | 20.8 |
| loop work | 2 | 40.0 | 20.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 145.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 143.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 142.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 142.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 140.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 138.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 136.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 131.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 117.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 113.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 30501 |
| IWI IRQ work interrupts | 29117 |
| CAL Function call interrupts | 1057 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c2-cstate/pp-recv-busy-10000 (median 39.78)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1035.66 | 995.88 | 111.41 | softirq TIMER | 0.00 |  | 884.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 928.72 | 888.93 | 0.00 |  | 0.00 |  | 888.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 419.95 | 380.17 | 0.00 |  | 0.00 |  | 380.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 397.06 | 357.28 | 0.00 |  | 0.00 |  | 357.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 385.31 | 345.53 | 0.00 |  | 0.00 |  | 345.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 376.45 | 336.66 | 0.00 |  | 0.00 |  | 336.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 349.99 | 310.21 | 0.00 |  | 0.00 |  | 310.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 336.46 | 296.68 | 0.00 |  | 0.00 |  | 296.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 307.70 | 267.92 | 0.00 |  | 0.00 |  | 267.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 293.81 | 254.03 | 0.00 |  | 0.00 |  | 254.03 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c2-noautocork/http-recv-busy-10000, loop 0 on CPU 2

8383583529 iterations, longest 162.4 µs; over 5 / 20 / 100 µs: 400727 / 9338 / 15

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 3842 | 94017.6 | 46.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2302 | 54162.1 | 145.5 |
| mostly unexplained (+softirq NET_RX) | 1416 | 34290.5 | 46.1 |
| softirq NET_RX | 1107 | 26594.0 | 38.7 |
| mostly unexplained (+local timer tick) | 576 | 13361.5 | 33.6 |
| mostly unexplained (+irq work (IPI/vector)) | 84 | 2016.2 | 36.9 |
| softirq TIMER | 2 | 225.5 | 162.4 |
| loop work | 8 | 196.8 | 33.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 26.4 | 26.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 162.4 | 1 | softirq TIMER | softirq TIMER 150, local timer tick 8, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 145.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 144.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 144.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 143.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 142.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 141.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 138.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 138.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 117.7 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 23329 |
| IWI IRQ work interrupts | 21672 |
| CAL Function call interrupts | 1048 |
| MCP Machine check polls | 2 |

#### Slowest round trips: c2-noautocork/http-recv-busy-10000 (median 37.98)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 354.37 | 316.38 | 0.00 |  | 0.00 |  | 316.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 342.46 | 304.47 | 0.00 |  | 0.00 |  | 304.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 323.40 | 285.42 | 0.00 |  | 0.00 |  | 285.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 303.51 | 265.53 | 0.00 |  | 0.00 |  | 265.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 284.18 | 246.19 | 0.00 |  | 0.00 |  | 246.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 270.62 | 232.63 | 0.00 |  | 0.00 |  | 232.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 266.53 | 228.54 | 0.00 |  | 0.00 |  | 228.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 240.27 | 202.29 | 0.00 |  | 0.00 |  | 202.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 238.63 | 200.65 | 0.00 |  | 0.00 |  | 200.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 237.75 | 199.76 | 0.00 |  | 0.00 |  | 199.76 | neither loop stalled long enough: network, hypervisor or untraced |

## c2-noautocork/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1045689 clocksource=tsc

### Loop counters (c2-noautocork/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1191770 | 6.72 | 287133248 | 35.88 | 1794583 | 1794583 | 99.6 | 65.4 | true | 0 | 2487 | 594.4 | 356407 / 12765 / 7 | 4002800 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792627200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (68665 of 3897200 in 1-msg reads) | 68665 | 3.09 | 2.68 | 4.08 | 9.50 | 25.58 | 35.21 | 37.58 |

#### Stalls: c2-noautocork/md-recv-busy, loop 0 on CPU 2

287133248 iterations, longest 594.4 µs; over 5 / 20 / 100 µs: 356407 / 12765 / 7

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 11933 | 370667.8 | 594.4 |
| mostly unexplained (+preempted by kworker/2:1) | 368 | 9705.5 | 65.0 |
| softirq NET_RX | 251 | 6930.3 | 48.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 175 | 4871.9 | 145.2 |
| mostly unexplained (+softirq NET_RX) | 34 | 945.4 | 38.5 |
| softirq TIMER | 1 | 395.4 | 395.4 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 61.7 | 37.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 23.4 | 23.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 594.4 | 2800 | loop work (2800 items) |  |
| 2 | 395.4 | 50 | softirq TIMER | softirq TIMER 371, local timer tick 7, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 3 | 145.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 144.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 143.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 120.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 105.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 91.0 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 70.9 | 21 | loop work (21 items) | softirq NET_RX 19, preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 69.6 | 20 | loop work (20 items) | softirq NET_RX 22, preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2500 |
| LOC Local timer interrupts | 2495 |
| CAL Function call interrupts | 1009 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## c2-noautocork/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c2-noautocork/noise, loop 0 on CPU 2

11847720960 iterations, longest 172.6 µs; over 5 / 20 / 100 µs: 408 / 9 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 383 | 2945.7 | 138.1 |
| softirq TIMER | 1 | 172.6 | 172.6 |
| mostly unexplained (+call function (IPI/vector)) | 19 | 103.9 | 6.2 |
| local timer tick | 3 | 28.5 | 15.6 |
| mostly unexplained (+local timer tick) | 2 | 15.3 | 8.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 172.6 | 0 | softirq TIMER | softirq TIMER 149, local timer tick 12, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 138.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 112.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 109.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 89.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 56.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 56.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 26.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 22.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 18.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| LOC Local timer interrupts | 7 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (c2-noautocork/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 883429056 | 184.05 | 1840477 | 1830477 | 99.5 | 68.0 | true | 0 | 18153 | 147.8 | 283159 / 1460 / 14 | 0 | 0 |

#### Stalls: c2-noautocork/pp-recv-busy-10000, loop 0 on CPU 2

878629056 iterations, longest 147.8 µs; over 5 / 20 / 100 µs: 283159 / 1460 / 14

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 1044 | 24295.3 | 34.5 |
| mostly unexplained (+softirq NET_RX) | 180 | 4541.3 | 142.8 |
| softirq NET_RX | 176 | 4009.6 | 32.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 46 | 2683.3 | 147.8 |
| mostly unexplained (+irq work (IPI/vector)) | 12 | 322.9 | 33.9 |
| softirq TIMER | 1 | 68.0 | 68.0 |
| loop work | 1 | 20.0 | 20.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 147.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 146.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 143.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 143.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 142.8 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 140.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 139.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 135.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 116.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 113.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 18240 |
| IWI IRQ work interrupts | 18170 |
| CAL Function call interrupts | 1053 |
| MCP Machine check polls | 1 |

#### Slowest round trips: c2-noautocork/pp-recv-busy-10000 (median 37.06)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 950.55 | 913.50 | 0.00 |  | 0.00 |  | 913.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 915.31 | 878.25 | 0.00 |  | 0.00 |  | 878.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 612.41 | 575.36 | 0.00 |  | 0.00 |  | 575.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 611.97 | 574.92 | 0.00 |  | 0.00 |  | 574.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 345.74 | 308.68 | 0.00 |  | 0.00 |  | 308.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 345.08 | 308.02 | 0.00 |  | 0.00 |  | 308.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 313.91 | 276.86 | 0.00 |  | 0.00 |  | 276.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 313.26 | 276.20 | 0.00 |  | 0.00 |  | 276.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 306.31 | 269.26 | 0.00 |  | 0.00 |  | 269.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 303.90 | 266.85 | 67.99 | softirq TIMER | 0.00 |  | 198.86 | neither loop stalled long enough: network, hypervisor or untraced |

## Ping-pong round trip (client send → echo decoded; "from when due": from the slot on the paced schedule, including any wait behind a stall)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| c1-base/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989935 | 38.07 | 37.65 | 40.63 | 46.20 | 61.28 | 112.84 | 1009.61 |
| c1-base/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989935 | 38.33 | 37.83 | 40.81 | 46.41 | 62.06 | 406.10 | 1009.78 |
| c1-base/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 37.46 | 36.92 | 40.04 | 49.38 | 58.75 | 115.88 | 454.17 |
| c1-base/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 37.89 | 37.27 | 40.40 | 49.74 | 59.93 | 416.93 | 1345.66 |
| c1-irqlocal/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989934 | 39.28 | 38.52 | 42.45 | 52.36 | 76.42 | 122.57 | 556.09 |
| c1-irqlocal/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989934 | 39.67 | 38.79 | 42.73 | 52.67 | 77.90 | 472.24 | 1109.28 |
| c1-irqlocal/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 39.02 | 38.36 | 42.01 | 50.89 | 77.80 | 117.15 | 1017.76 |
| c1-irqlocal/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 39.49 | 38.74 | 42.40 | 51.33 | 79.46 | 456.89 | 1263.88 |
| c1-onequeue/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989935 | 39.53 | 38.76 | 43.44 | 51.85 | 76.81 | 132.59 | 967.59 |
| c1-onequeue/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989935 | 39.76 | 38.94 | 43.62 | 52.11 | 78.99 | 276.44 | 967.75 |
| c1-onequeue/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 36.41 | 35.92 | 38.89 | 47.45 | 68.36 | 100.72 | 960.71 |
| c1-onequeue/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 36.79 | 36.26 | 39.24 | 47.80 | 69.11 | 221.90 | 960.81 |
| c1-cstate/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989933 | 38.89 | 38.30 | 41.31 | 53.48 | 75.71 | 106.05 | 492.76 |
| c1-cstate/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989933 | 39.17 | 38.48 | 41.49 | 53.71 | 76.67 | 157.74 | 4251.46 |
| c1-cstate/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 40.53 | 39.86 | 44.30 | 51.30 | 75.59 | 133.30 | 464.79 |
| c1-cstate/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 40.95 | 40.22 | 44.66 | 51.75 | 77.91 | 317.52 | 896.62 |
| c1-noautocork/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989934 | 38.94 | 38.42 | 41.32 | 50.03 | 72.48 | 128.50 | 484.50 |
| c1-noautocork/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989934 | 39.26 | 38.62 | 41.63 | 50.31 | 75.11 | 437.06 | 879.42 |
| c1-noautocork/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 38.53 | 38.17 | 40.84 | 44.70 | 65.92 | 96.24 | 966.49 |
| c1-noautocork/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 38.90 | 38.51 | 41.19 | 45.05 | 66.56 | 154.64 | 966.95 |
| c2-base/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989933 | 38.03 | 37.56 | 40.44 | 46.05 | 67.29 | 106.21 | 1049.79 |
| c2-base/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989933 | 38.28 | 37.73 | 40.60 | 46.23 | 68.20 | 428.23 | 1449.57 |
| c2-base/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 38.09 | 37.69 | 40.36 | 44.80 | 65.74 | 104.24 | 967.90 |
| c2-base/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 38.50 | 38.05 | 40.73 | 45.17 | 66.80 | 329.75 | 1004.97 |
| c2-irqlocal/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989936 | 39.25 | 38.55 | 42.44 | 49.75 | 69.58 | 109.05 | 449.35 |
| c2-irqlocal/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989936 | 39.58 | 38.79 | 42.69 | 50.03 | 70.76 | 435.63 | 1020.44 |
| c2-irqlocal/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 38.05 | 37.52 | 40.54 | 47.33 | 60.90 | 95.86 | 350.42 |
| c2-irqlocal/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 38.39 | 37.84 | 40.87 | 47.68 | 61.55 | 139.98 | 814.21 |
| c2-onequeue/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989936 | 38.11 | 37.64 | 40.48 | 46.50 | 68.14 | 111.01 | 395.33 |
| c2-onequeue/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989936 | 38.33 | 37.81 | 40.65 | 46.69 | 68.77 | 271.94 | 722.22 |
| c2-onequeue/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 39.55 | 38.93 | 43.31 | 48.46 | 71.59 | 130.84 | 966.89 |
| c2-onequeue/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 39.94 | 39.26 | 43.63 | 48.84 | 74.16 | 372.56 | 967.03 |
| c2-cstate/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989935 | 39.20 | 38.62 | 42.02 | 51.04 | 69.31 | 90.25 | 227.01 |
| c2-cstate/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989935 | 39.37 | 38.78 | 42.18 | 51.21 | 69.57 | 90.97 | 348.12 |
| c2-cstate/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 40.40 | 39.78 | 44.17 | 50.47 | 73.02 | 128.17 | 1035.66 |
| c2-cstate/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 40.78 | 40.10 | 44.49 | 50.85 | 74.94 | 304.62 | 1100.92 |
| c2-noautocork/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989935 | 38.46 | 37.98 | 40.79 | 49.62 | 62.43 | 96.40 | 354.37 |
| c2-noautocork/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989935 | 38.65 | 38.15 | 40.95 | 49.80 | 62.77 | 124.09 | 791.83 |
| c2-noautocork/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 37.68 | 37.06 | 40.15 | 49.55 | 64.37 | 102.36 | 950.55 |
| c2-noautocork/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 38.03 | 37.37 | 40.53 | 49.90 | 65.11 | 179.43 | 950.71 |

