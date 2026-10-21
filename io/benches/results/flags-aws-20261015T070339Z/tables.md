# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|
| c1-base | 78.88 | 151.1 | 1.1 | 0.0 | 78.88 µs stall: softirq TIMER |
| c1-nodim | 80.29 | 151.5 | 1.1 | 0.0 | 80.29 µs stall: softirq TIMER |
| c1-nocoalesce | 29.12 | 155.6 | 0.8 | 0.0 | 29.12 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| c1-rt | 30.91 | 151.1 | 0.8 | 0.0 | 30.91 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| c2-base | 128.78 | 145.9 | 2.2 | 1.1 | 128.78 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| c2-nodim | 79.16 | 208.1 | 1.1 | 0.0 | 79.16 µs stall: softirq TIMER |
| c2-nocoalesce | 29.38 | 159.0 | 0.8 | 0.0 | 29.38 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| c2-rt | 30.95 | 144.8 | 0.8 | 0.0 | 30.95 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |

## HTTP round trip, http-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| c1-base | 1989935 | 43.35 | 56.92 | 67.48 | 118.88 | 445.88 | 123.6 | 0.0 | 0.0 | 0.0 | 429.41 | 210.0 | client: softirq TIMER |
| c1-nodim | 1989935 | 44.06 (+0.71) | 54.42 (−2.50) | 65.98 (−1.50) | 116.06 (−2.82) | 980.65 (+534.76) | 130.9 | 0.3 | 0.0 | 0.0 | 328.16 | 35.5 | neither loop stalled long enough: network, hypervisor or untraced |
| c1-nocoalesce | 1989936 | 41.69 (−1.66) | 50.10 (−6.82) | 59.60 (−7.88) | 80.99 (−37.88) | 201.65 (−244.24) | 37.1 | 0.0 | 0.0 | 0.0 | 395.82 | 37.0 | neither loop stalled long enough: network, hypervisor or untraced |
| c1-rt | 1989936 | 41.32 (−2.03) | 49.66 (−7.26) | 59.90 (−7.58) | 75.62 (−43.26) | 982.27 (+536.39) | 20.2 | 0.3 | 0.0 | 0.0 | 235.60 | 36.1 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-base | 1989936 | 41.77 | 50.62 | 61.53 | 77.06 | 432.70 | 24.1 | 0.0 | 0.0 | 0.0 | 445.27 | 215.6 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-nodim | 1989935 | 41.52 (−1.83) | 51.33 (−5.60) | 60.39 (−7.09) | 83.44 (−35.44) | 201.50 (−244.38) | 41.9 | 0.0 | 0.0 | 0.0 | 221.03 | 91.6 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-nocoalesce | 1989936 | 40.71 (−2.64) | 53.93 (−3.00) | 61.22 (−6.26) | 73.78 (−45.10) | 226.79 (−219.09) | 9.3 | 0.0 | 0.0 | 0.0 | 243.51 | 98.4 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-rt | 1989936 | 41.58 (−1.77) | 50.88 (−6.04) | 59.53 (−7.95) | 75.47 (−43.41) | 229.95 (−215.93) | 19.3 | 0.0 | 0.0 | 0.0 | 224.00 | 88.1 | client: mostly unexplained (+softirq NET_RX) |

### HTTP round trip, http-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| c1-base | 1989935 | 43.53 | 57.17 | 68.28 | 121.62 | 446.09 | 140.8 | 0.0 | 0.0 | 0.0 |
| c1-nodim | 1989935 | 44.24 (+0.70) | 54.64 (−2.53) | 66.92 (−1.36) | 120.30 (−1.32) | 980.82 (+534.73) | 148.0 | 2.7 | 0.0 | 0.0 |
| c1-nocoalesce | 1989936 | 41.87 (−1.67) | 50.30 (−6.87) | 60.10 (−8.17) | 88.62 (−32.99) | 201.83 (−244.27) | 47.0 | 0.0 | 0.0 | 0.0 |
| c1-rt | 1989936 | 41.51 (−2.03) | 49.87 (−7.30) | 60.23 (−8.04) | 77.12 (−44.50) | 982.45 (+536.36) | 25.0 | 2.7 | 0.0 | 0.0 |
| c2-base | 1989936 | 41.94 | 50.81 | 61.77 | 78.06 | 432.85 | 25.9 | 0.0 | 0.0 | 0.0 |
| c2-nodim | 1989935 | 41.69 (−1.84) | 51.52 (−5.65) | 60.87 (−7.41) | 89.10 (−32.52) | 201.68 (−244.41) | 45.2 | 0.0 | 0.0 | 0.0 |
| c2-nocoalesce | 1989936 | 40.87 (−2.67) | 54.10 (−3.07) | 61.45 (−6.83) | 74.21 (−47.41) | 226.96 (−219.13) | 10.6 | 0.0 | 0.0 | 0.0 |
| c2-rt | 1989936 | 41.76 (−1.78) | 51.08 (−6.09) | 59.82 (−8.45) | 77.13 (−44.49) | 230.18 (−215.92) | 21.7 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| c1-base | 3897200 | 3.06 | 6.50 | 13.32 | 21.59 | 33.62 | 0.0 | 0.0 | 0.0 | 0.0 | 607.66 | 3497.6 | 607.66 µs stall: loop work (2800 items) |
| c1-nodim | 3897150 | 2.68 (−0.39) | 12.87 (+6.33) | 17.70 (+3.38) | 20.49 (−1.18) | 25.19 (−8.61) | 0.0 | 0.0 | 0.0 | 0.0 | 616.50 | 2234.6 | 616.50 µs stall: loop work (2800 items) |
| c1-nocoalesce | 3897150 | 3.22 (+0.15) | 7.90 (+1.36) | 10.64 (−3.67) | 19.30 (−2.37) | 34.06 (+0.27) | 0.0 | 0.0 | 0.0 | 0.0 | 628.28 | 4755.8 | 628.28 µs stall: loop work (2800 items) |
| c1-rt | 3897150 | 2.87 (−0.19) | 6.92 (+0.38) | 10.92 (−3.40) | 16.95 (−4.72) | 88.86 (+55.07) | 0.0 | 0.0 | 0.0 | 0.0 | 662.66 | 3498.4 | 662.66 µs stall: loop work (2800 items) |
| c2-base | 3897150 | 3.07 | 6.54 | 14.32 | 21.67 | 33.80 | 0.0 | 0.0 | 0.0 | 0.0 | 595.78 | 2689.9 | 595.78 µs stall: loop work (2850 items) |
| c2-nodim | 3897150 | 2.63 (−0.44) | 9.29 (+2.75) | 17.25 (+2.94) | 19.98 (−1.69) | 28.57 (−5.23) | 0.0 | 0.0 | 0.0 | 0.0 | 703.79 | 6044.6 | 703.79 µs stall: loop work (2800 items) |
| c2-nocoalesce | 3897200 | 3.14 (+0.07) | 8.60 (+2.07) | 12.97 (−1.34) | 20.67 (−1.00) | 41.20 (+7.41) | 0.0 | 0.0 | 0.0 | 0.0 | 703.37 | 4864.9 | 703.37 µs stall: loop work (2800 items) |
| c2-rt | 3897150 | 2.85 (−0.22) | 5.86 (−0.68) | 8.71 (−5.61) | 15.20 (−6.47) | 29.95 (−3.85) | 0.0 | 0.0 | 0.0 | 0.0 | 671.52 | 3471.0 | 671.52 µs stall: loop work (2800 items) |

## WebSocket round trip, pp-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| c1-base | 4789999 | 43.90 | 53.62 | 64.19 | 118.85 | 1023.80 | 117.1 | 0.3 | 0.1 | 0.0 | 228.21 | 160.5 | neither loop stalled long enough: network, hypervisor or untraced |
| c1-nodim | 4789999 | 40.88 (−3.02) | 47.16 (−6.45) | 56.94 (−7.25) | 71.89 (−46.96) | 354.05 (−669.76) | 14.7 | 0.0 | 0.0 | 0.0 | 330.10 | 9.9 | client: softirq TIMER |
| c1-nocoalesce | 4789999 | 41.90 (−2.00) | 48.59 (−5.03) | 59.76 (−4.43) | 78.50 (−40.35) | 233.02 (−790.79) | 35.2 | 0.0 | 0.0 | 0.0 | 279.74 | 9.9 | server: softirq TIMER |
| c1-rt | 4789999 | 44.39 (+0.49) | 54.00 (+0.38) | 64.28 (+0.09) | 119.81 (+0.96) | 579.59 (−444.21) | 129.8 | 0.1 | 0.0 | 0.0 | 323.32 | 34.2 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-base | 4789999 | 40.65 | 47.56 | 59.40 | 116.53 | 259.23 | 73.4 | 0.0 | 0.0 | 0.0 | 325.75 | 106.9 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-nodim | 4789999 | 40.93 (−2.97) | 48.43 (−5.18) | 57.93 (−6.26) | 76.02 (−42.83) | 970.07 (−53.73) | 32.3 | 0.1 | 0.0 | 0.0 | 125.36 | 12.1 | neither loop stalled long enough: network, hypervisor or untraced |
| c2-nocoalesce | 4789999 | 41.55 (−2.35) | 48.69 (−4.93) | 59.60 (−4.59) | 80.41 (−38.44) | 343.68 (−680.12) | 31.9 | 0.0 | 0.0 | 0.0 | 329.72 | 11.5 | client: softirq TIMER |
| c2-rt | 4789999 | 42.12 (−1.78) | 50.59 (−3.03) | 59.59 (−4.60) | 73.87 (−44.98) | 1008.92 (−14.88) | 19.4 | 0.1 | 0.1 | 0.0 | 111.46 | 69.0 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| c1-base | 4789999 | 44.24 | 54.00 | 65.19 | 121.85 | 1024.38 | 136.4 | 2.4 | 0.1 | 0.0 |
| c1-nodim | 4789999 | 41.22 (−3.03) | 47.52 (−6.47) | 57.34 (−7.84) | 72.78 (−49.07) | 354.61 (−669.76) | 15.8 | 0.0 | 0.0 | 0.0 |
| c1-nocoalesce | 4789999 | 42.23 (−2.01) | 48.94 (−5.06) | 60.32 (−4.87) | 80.83 (−41.02) | 311.38 (−713.00) | 37.1 | 0.0 | 0.0 | 0.0 |
| c1-rt | 4789999 | 44.72 (+0.48) | 54.37 (+0.37) | 65.61 (+0.42) | 123.05 (+1.20) | 579.82 (−444.55) | 149.6 | 0.3 | 0.0 | 0.0 |
| c2-base | 4789999 | 40.98 | 47.92 | 60.29 | 120.46 | 350.87 | 82.5 | 0.0 | 0.0 | 0.0 |
| c2-nodim | 4789999 | 41.26 (−2.98) | 48.78 (−5.22) | 58.54 (−6.64) | 83.00 (−38.84) | 970.40 (−53.98) | 37.1 | 1.0 | 0.0 | 0.0 |
| c2-nocoalesce | 4789999 | 41.89 (−2.35) | 49.05 (−4.95) | 60.16 (−5.02) | 86.70 (−35.15) | 343.76 (−680.61) | 39.6 | 0.0 | 0.0 | 0.0 |
| c2-rt | 4789999 | 42.45 (−1.79) | 50.96 (−3.03) | 60.00 (−5.18) | 74.70 (−47.15) | 1009.41 (−14.97) | 22.2 | 1.1 | 0.1 | 0.0 |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs, and system-management interrupts (SMI, bare metal only: firmware pauses). The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | SMI | receiving CPU | LOC | IWI | RES | CAL | SMI |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| http-recv-busy-10000 | c1-base | 2 | 23 | 23 | 0 | 2 | 0 | | | | | | | |
| http-recv-busy-10000 | c1-nodim | 2 | 0 | 0 | 0 | 2 | 0 | | | | | | | |
| http-recv-busy-10000 | c1-nocoalesce | 2 | 0 | 0 | 0 | 2 | 0 | | | | | | | |
| http-recv-busy-10000 | c1-rt | 2 | 947 | 0 | 0 | 2 | 0 | | | | | | | |
| http-recv-busy-10000 | c2-base | 2 | 12 | 11 | 0 | 2 | 0 | | | | | | | |
| http-recv-busy-10000 | c2-nodim | 2 | 0 | 0 | 0 | 2 | 0 | | | | | | | |
| http-recv-busy-10000 | c2-nocoalesce | 2 | 0 | 0 | 0 | 2 | 0 | | | | | | | |
| http-recv-busy-10000 | c2-rt | 2 | 549 | 0 | 0 | 2 | 0 | | | | | | | |
| md-recv-busy | c1-base | 2 | 11 | 11 | 0 | 6 | 0 | | | | | | | |
| md-recv-busy | c1-nodim | 2 | 0 | 0 | 0 | 6 | 0 | | | | | | | |
| md-recv-busy | c1-nocoalesce | 2 | 0 | 0 | 0 | 6 | 0 | 4 | 961 | 0 | 0 | 8 | 0 |
| md-recv-busy | c1-rt | 2 | 554 | 1 | 0 | 6 | 0 | | | | | | | |
| md-recv-busy | c2-base | 2 | 12 | 12 | 0 | 6 | 0 | | | | | | | |
| md-recv-busy | c2-nodim | 2 | 0 | 0 | 0 | 6 | 0 | | | | | | | |
| md-recv-busy | c2-nocoalesce | 2 | 0 | 0 | 0 | 6 | 0 | | | | | | | |
| md-recv-busy | c2-rt | 2 | 548 | 1 | 0 | 6 | 0 | | | | | | | |
| noise | c1-base | 2 | 0 | 0 | 0 | 6 | 0 | | | | | | | |
| noise | c1-nodim | 2 | 0 | 0 | 0 | 6 | 0 | | | | | | | |
| noise | c1-nocoalesce | 2 | 0 | 0 | 0 | 6 | 0 | | | | | | | |
| noise | c1-rt | 2 | 0 | 0 | 0 | 6 | 0 | | | | | | | |
| noise | c2-base | 2 | 0 | 0 | 0 | 6 | 0 | | | | | | | |
| noise | c2-nodim | 2 | 0 | 0 | 0 | 6 | 0 | | | | | | | |
| noise | c2-nocoalesce | 2 | 0 | 0 | 0 | 6 | 0 | | | | | | | |
| noise | c2-rt | 2 | 0 | 0 | 0 | 6 | 0 | | | | | | | |
| pp-recv-busy-10000 | c1-base | 2 | 19 | 18 | 0 | 2 | 0 | | | | | | | |
| pp-recv-busy-10000 | c1-nodim | 2 | 0 | 0 | 0 | 2 | 0 | | | | | | | |
| pp-recv-busy-10000 | c1-nocoalesce | 2 | 0 | 0 | 0 | 2 | 0 | | | | | | | |
| pp-recv-busy-10000 | c1-rt | 2 | 533 | 1 | 1 | 2 | 0 | | | | | | | |
| pp-recv-busy-10000 | c2-base | 2 | 19 | 19 | 0 | 2 | 0 | | | | | | | |
| pp-recv-busy-10000 | c2-nodim | 2 | 0 | 0 | 0 | 2 | 0 | | | | | | | |
| pp-recv-busy-10000 | c2-nocoalesce | 2 | 0 | 0 | 0 | 2 | 0 | | | | | | | |
| pp-recv-busy-10000 | c2-rt | 2 | 543 | 1 | 0 | 2 | 0 | | | | | | | |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### http-recv-busy-10000

| condition | worst events |
|---|---|
| c1-base | 445.88 at 74.1 s: client: softirq TIMER; 189.95 at 107.6 s: neither loop stalled long enough: network, hypervisor or untraced; 178.57 at 195.1 s: neither loop stalled long enough: network, hypervisor or untraced; 177.81 at 96.1 s: neither loop stalled long enough: network, hypervisor or untraced; 174.49 at 160.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-nodim | 980.65 at 100.3 s: neither loop stalled long enough: network, hypervisor or untraced; 212.59 at 1.2 s: server: softirq TIMER; 180.65 at 74.2 s: neither loop stalled long enough: network, hypervisor or untraced; 180.53 at 90.2 s: neither loop stalled long enough: network, hypervisor or untraced; 176.87 at 145.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-nocoalesce | 201.65 at 20.7 s: neither loop stalled long enough: network, hypervisor or untraced; 174.42 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 174.32 at 182.8 s: neither loop stalled long enough: network, hypervisor or untraced; 172.99 at 85.8 s: neither loop stalled long enough: network, hypervisor or untraced; 171.83 at 198.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-rt | 982.27 at 22.5 s: neither loop stalled long enough: network, hypervisor or untraced; 193.72 at 169.2 s: server: softirq TIMER; 131.24 at 63.2 s: neither loop stalled long enough: network, hypervisor or untraced; 130.01 at 31.2 s: neither loop stalled long enough: network, hypervisor or untraced; 129.62 at 165.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-base | 432.70 at 89.7 s: neither loop stalled long enough: network, hypervisor or untraced; 187.57 at 93.1 s: server: softirq TIMER; 140.17 at 84.6 s: neither loop stalled long enough: network, hypervisor or untraced; 132.48 at 55.1 s: neither loop stalled long enough: network, hypervisor or untraced; 130.89 at 113.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-nodim | 201.50 at 68.2 s: neither loop stalled long enough: network, hypervisor or untraced; 188.48 at 18.3 s: server: softirq TIMER; 185.92 at 121.2 s: neither loop stalled long enough: network, hypervisor or untraced; 171.89 at 28.9 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 164.51 at 129.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-nocoalesce | 226.79 at 25.6 s: neither loop stalled long enough: network, hypervisor or untraced; 144.31 at 16.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 143.71 at 136.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 132.91 at 76.2 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 131.94 at 88.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-rt | 229.95 at 140.3 s: client: mostly unexplained (+softirq NET_RX); 225.37 at 175.1 s: server: softirq TIMER; 171.56 at 100.1 s: neither loop stalled long enough: network, hypervisor or untraced; 165.04 at 56.6 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 161.77 at 176.6 s: server: unexplained: no kernel event (hypervisor or in-kernel time) |

### md-recv-busy

| condition | worst events |
|---|---|
| c1-base | 33.62 at 17.2 s: client: loop work (5 items); 33.45 at 17.2 s: client: loop work (5 items); 33.27 at 17.2 s: client: loop work (5 items); 33.10 at 17.2 s: client: loop work (5 items); 32.93 at 17.2 s: client: loop work (5 items) |
| c1-nodim | 25.19 at 44.4 s: neither loop stalled long enough: network, hypervisor or untraced; 25.02 at 44.4 s: neither loop stalled long enough: network, hypervisor or untraced; 24.99 at 41.7 s: neither loop stalled long enough: network, hypervisor or untraced; 24.86 at 44.4 s: neither loop stalled long enough: network, hypervisor or untraced; 24.83 at 41.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-nocoalesce | 34.06 at 71.8 s: neither loop stalled long enough: network, hypervisor or untraced; 33.90 at 71.8 s: neither loop stalled long enough: network, hypervisor or untraced; 33.85 at 47.7 s: client: loop work (4 items); 33.73 at 71.8 s: neither loop stalled long enough: network, hypervisor or untraced; 33.67 at 72.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| c1-rt | 88.86 at 22.8 s: client: VM paused: the idle spare core stalled too (hypervisor); 88.67 at 22.8 s: client: VM paused: the idle spare core stalled too (hypervisor); 29.02 at 30.3 s: client: loop work (7 items); 28.85 at 30.3 s: client: loop work (7 items); 28.74 at 9.2 s: client: loop work (6 items) |
| c2-base | 33.80 at 35.3 s: client: loop work (2 items); 33.60 at 35.3 s: client: loop work (2 items); 31.04 at 38.7 s: client: loop work (2 items); 30.85 at 38.7 s: client: loop work (2 items); 30.70 at 36.6 s: client: loop work (8 items) |
| c2-nodim | 28.57 at 16.8 s: neither loop stalled long enough: network, hypervisor or untraced; 28.41 at 16.8 s: neither loop stalled long enough: network, hypervisor or untraced; 28.26 at 16.8 s: neither loop stalled long enough: network, hypervisor or untraced; 28.08 at 16.8 s: neither loop stalled long enough: network, hypervisor or untraced; 27.93 at 16.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-nocoalesce | 41.20 at 44.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 41.02 at 44.2 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 34.31 at 59.9 s: client: loop work (4 items); 34.11 at 59.9 s: client: loop work (4 items); 34.09 at 42.7 s: client: loop work (9 items) |
| c2-rt | 29.95 at 9.4 s: client: loop work (8 items); 29.78 at 9.4 s: client: loop work (8 items); 29.61 at 9.4 s: client: loop work (8 items); 29.44 at 9.4 s: client: loop work (8 items); 29.27 at 9.4 s: client: loop work (8 items) |

### noise

| condition | worst events |
|---|---|
| c1-base | 78.88 at 101.4 s: softirq TIMER; 28.89 at 7.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 25.57 at 8.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 19.21 at 0.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.30 at 158.3 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c1-nodim | 80.29 at 24.2 s: softirq TIMER; 34.34 at 0.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 26.24 at 60.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.52 at 120.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.06 at 140.3 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c1-nocoalesce | 29.12 at 49.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 21.09 at 9.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.67 at 0.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 15.27 at 29.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 14.38 at 89.6 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c1-rt | 30.91 at 1.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 29.68 at 41.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 15.51 at 11.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 15.38 at 101.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 14.87 at 111.9 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c2-base | 128.78 at 39.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 105.33 at 99.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 101.69 at 159.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 83.73 at 116.0 s: softirq TIMER; 38.85 at 2.3 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c2-nodim | 79.16 at 37.9 s: softirq TIMER; 28.81 at 25.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 27.03 at 3.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.96 at 0.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.18 at 84.8 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c2-nocoalesce | 29.38 at 16.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 25.34 at 4.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.59 at 3.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.89 at 76.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 15.62 at 114.5 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| c2-rt | 30.95 at 0.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 27.13 at 2.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.19 at 62.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 15.00 at 90.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 14.67 at 40.9 s: unexplained: no kernel event (hypervisor or in-kernel time) |

### pp-recv-busy-10000

| condition | worst events |
|---|---|
| c1-base | 1023.80 at 459.9 s: neither loop stalled long enough: network, hypervisor or untraced; 980.04 at 340.3 s: neither loop stalled long enough: network, hypervisor or untraced; 325.51 at 194.7 s: neither loop stalled long enough: network, hypervisor or untraced; 261.78 at 305.6 s: client: VM paused: the idle spare core stalled too (hypervisor); 253.54 at 258.4 s: server: softirq TIMER |
| c1-nodim | 354.05 at 178.7 s: client: softirq TIMER; 154.26 at 182.1 s: server: softirq TIMER; 153.22 at 188.0 s: neither loop stalled long enough: network, hypervisor or untraced; 134.97 at 47.0 s: neither loop stalled long enough: network, hypervisor or untraced; 133.80 at 181.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| c1-nocoalesce | 233.02 at 429.0 s: server: softirq TIMER; 214.46 at 101.4 s: server: softirq TIMER; 203.26 at 347.9 s: neither loop stalled long enough: network, hypervisor or untraced; 198.94 at 107.6 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 174.57 at 169.0 s: client: unexplained: no kernel event (hypervisor or in-kernel time) |
| c1-rt | 579.59 at 420.2 s: neither loop stalled long enough: network, hypervisor or untraced; 354.52 at 347.7 s: client: softirq TIMER; 326.08 at 159.2 s: neither loop stalled long enough: network, hypervisor or untraced; 309.67 at 130.8 s: neither loop stalled long enough: network, hypervisor or untraced; 297.85 at 231.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-base | 259.23 at 49.4 s: neither loop stalled long enough: network, hypervisor or untraced; 240.80 at 273.0 s: server: softirq TIMER; 217.10 at 231.4 s: neither loop stalled long enough: network, hypervisor or untraced; 204.99 at 475.4 s: neither loop stalled long enough: network, hypervisor or untraced; 204.39 at 291.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-nodim | 970.07 at 30.4 s: neither loop stalled long enough: network, hypervisor or untraced; 226.50 at 194.3 s: server: softirq TIMER; 210.60 at 135.9 s: neither loop stalled long enough: network, hypervisor or untraced; 203.50 at 310.8 s: neither loop stalled long enough: network, hypervisor or untraced; 202.39 at 379.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-nocoalesce | 343.68 at 441.1 s: client: softirq TIMER; 257.02 at 113.4 s: client: softirq TIMER; 187.90 at 51.7 s: neither loop stalled long enough: network, hypervisor or untraced; 185.13 at 40.7 s: neither loop stalled long enough: network, hypervisor or untraced; 184.73 at 104.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| c2-rt | 1008.92 at 255.9 s: neither loop stalled long enough: network, hypervisor or untraced; 224.96 at 30.6 s: neither loop stalled long enough: network, hypervisor or untraced; 177.08 at 354.9 s: neither loop stalled long enough: network, hypervisor or untraced; 142.12 at 171.9 s: neither loop stalled long enough: network, hypervisor or untraced; 141.11 at 194.1 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| c1-base | loop work 212724.53; mostly unexplained (+preempted by kworker/2:1) 30889.69; unexplained: no kernel event (hypervisor or in-kernel time) 18003.27; mostly unexplained (+softirq NET_RX) 11091.69; softirq NET_RX 11045.76 |
| c1-nodim | loop work 132916.52; unexplained: no kernel event (hypervisor or in-kernel time) 12363.00; softirq TIMER 1080.29; mostly unexplained (+softirq NET_RX) 685.88; VM paused: the idle spare core stalled too (hypervisor) 575.78 |
| c1-nocoalesce | loop work 308736.24; unexplained: no kernel event (hypervisor or in-kernel time) 12318.07; mostly unexplained (+softirq NET_RX) 1851.70; softirq NET_RX 1589.01; softirq TIMER 1021.40 |
| c1-rt | loop work 214935.91; unexplained: no kernel event (hypervisor or in-kernel time) 13551.03; preempted by kworker/2:1 3314.30; mostly unexplained (+preempted by kworker/2:1) 1437.59; VM paused: the idle spare core stalled too (hypervisor) 883.27 |
| c2-base | loop work 165378.40; mostly unexplained (+preempted by kworker/2:1) 35916.19; unexplained: no kernel event (hypervisor or in-kernel time) 10007.78; softirq NET_RX 10006.44; mostly unexplained (+softirq NET_RX) 8108.41 |
| c2-nodim | loop work 434749.14; unexplained: no kernel event (hypervisor or in-kernel time) 25631.08; mostly unexplained (+softirq NET_RX) 6780.73; VM paused: the idle spare core stalled too (hypervisor) 910.03; softirq TIMER 801.30 |
| c2-nocoalesce | loop work 340650.90; unexplained: no kernel event (hypervisor or in-kernel time) 26082.45; softirq NET_RX 1650.88; mostly unexplained (+softirq NET_RX) 1154.37; softirq TIMER 894.10 |
| c2-rt | loop work 210608.04; unexplained: no kernel event (hypervisor or in-kernel time) 14572.47; mostly unexplained (+preempted by kworker/2:1) 12327.75; preempted by kworker/2:1 5506.14; mostly unexplained (+local timer tick) 1848.69 |

## Once-a-minute stalls

Whether each source's events of 300 µs or more (loop stalls, or samples that slow) bunch at one second of the wall-clock minute. The busiest second is the middle of the busiest three; three seconds hold one event in twenty by chance, so a share of 25% or more over at least five events is marked periodic. Each host has its own second (its agents' timers start at boot).

| run | source | events | busiest second | events there (share) | minutes with one there | longest there (µs) | periodic |
|---|---|---|---|---|---|---|---|
| cpumap-receiver/cpu1 | counter-only spinner | 5 | :48 | 5 (100%) | 2 of 2 | 16952.20 | **yes** |
| cpumap-receiver/cpu2 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu3 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu5 | counter-only spinner | 7 | :48 | 7 (100%) | 2 of 2 | 13893.15 | **yes** |
| cpumap-receiver/cpu6 | counter-only spinner | 0 | | | | | |
| cpumap-receiver/cpu7 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu1 | counter-only spinner | 0 | | | | | |
| cpumap-sender/cpu3 | counter-only spinner | 0 | | | | | |
| http-recv-busy-10000 | idle spare core | 193 | :48 | 184 (95%) | 63 of 28 | 17177.97 | **yes** |
| http-recv-busy-10000 | receiver loop | 4 | :00 | 2 (50%) | 2 of 27 | 445.27 | no |
| http-recv-busy-10000 | sender loop | 0 | | | | | |
| http-recv-busy-10000 | slow samples | 4 | :15 | 1 (25%) | 1 of 27 | 980.65 | no |
| md-recv-busy | idle spare core | 57 | :48 | 52 (91%) | 23 of 11 | 17130.56 | **yes** |
| md-recv-busy | receiver loop | 1 | :28 | 1 (100%) | 1 of 12 | 341.75 | no |
| md-recv-busy | slow samples | 0 | | | | | |
| noise | noise probe (loop CPU) | 0 | | | | | |
| pp-recv-busy-10000 | idle spare core | 191 | :48 | 175 (92%) | 65 of 64 | 17049.54 | **yes** |
| pp-recv-busy-10000 | receiver loop | 4 | :05 | 2 (50%) | 2 of 64 | 330.10 | no |
| pp-recv-busy-10000 | sender loop | 0 | | | | | |
| pp-recv-busy-10000 | slow samples | 11 | :14 | 4 (36%) | 4 of 64 | 970.07 | no |

# Every run in detail

#### Stalls: c1-base/http-recv-busy-10000, loop 0 on CPU 2

6886366749 iterations, longest 429.4 µs; over 5 / 20 / 100 µs: 887360 / 1680 / 5

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 660 | 14670.8 | 37.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 478 | 11566.8 | 151.5 |
| mostly unexplained (+softirq NET_RX) | 283 | 6414.9 | 41.0 |
| softirq NET_RX | 159 | 3384.2 | 29.4 |
| mostly unexplained (+irq work (IPI/vector)) | 79 | 1811.2 | 32.2 |
| softirq TIMER | 2 | 499.3 | 429.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 13 | 310.1 | 30.7 |
| loop work | 4 | 96.1 | 29.2 |
| mostly unexplained (+local timer tick) | 2 | 45.7 | 23.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 429.4 | 2 | softirq TIMER | softirq TIMER 416, local timer tick 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 151.5 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 146.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 118.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 113.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 82.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 69.9 | 0 | softirq TIMER | softirq TIMER 59, local timer tick 5, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 8 | 45.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 42.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 41.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 9, preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 11010 |
| IWI IRQ work interrupts | 10974 |
| CAL Function call interrupts | 1057 |
| MCP Machine check polls | 2 |

#### Slowest round trips: c1-base/http-recv-busy-10000 (median 43.35)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 445.88 | 402.54 | 428.48 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 2 | 189.95 | 146.60 | 0.00 |  | 0.00 |  | 146.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 178.57 | 135.22 | 0.00 |  | 0.00 |  | 135.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 177.81 | 134.46 | 0.00 |  | 0.00 |  | 134.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 174.49 | 131.14 | 0.00 |  | 0.00 |  | 131.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 174.22 | 130.87 | 0.00 |  | 0.00 |  | 130.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 173.39 | 130.04 | 0.00 |  | 0.00 |  | 130.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 172.96 | 129.61 | 0.00 |  | 0.00 |  | 129.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 171.58 | 128.23 | 0.00 |  | 0.00 |  | 128.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 171.28 | 127.93 | 0.00 |  | 0.00 |  | 127.93 | neither loop stalled long enough: network, hypervisor or untraced |

## c1-base/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1487413 clocksource=tsc

### Loop counters (c1-base/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1593359 | 5.02 | 265609920 | 33.19 | 1660062 | 1660062 | 99.4 | 66.9 | true | 0 | 1682 | 607.7 | 379693 / 9327 / 3 | 4002800 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792627200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (93344 of 3897200 in 1-msg reads) | 93344 | 2.74 | 2.61 | 3.08 | 5.89 | 12.76 | 19.89 | 30.13 |

#### Stalls: c1-base/md-recv-busy, loop 0 on CPU 2

265609920 iterations, longest 607.7 µs; over 5 / 20 / 100 µs: 379693 / 9327 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 9197 | 212628.5 | 607.7 |
| softirq NET_RX | 80 | 1786.3 | 33.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 21 | 783.2 | 139.1 |
| mostly unexplained (+preempted by kworker/2:1) | 23 | 508.5 | 34.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 77.4 | 27.9 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 73.4 | 43.3 |
| softirq TIMER | 1 | 22.6 | 22.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 607.7 | 2800 | loop work (2800 items) |  |
| 2 | 139.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 105.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 52.2 | 23 | loop work (23 items) | softirq NET_RX 9 |
| 5 | 48.0 | 43 | loop work (43 items) |  |
| 6 | 46.1 | 20 | loop work (20 items) | softirq NET_RX 11, preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 45.3 | 18 | loop work (18 items) | preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 45.1 | 16 | loop work (16 items) | softirq NET_RX 18, irq work (IPI/vector) 1, preempted by kworker/2:1 1, workqueue [ena] 1 |
| 9 | 44.4 | 19 | loop work (19 items) | softirq NET_RX 16, preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 43.9 | 18 | loop work (18 items) | preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1699 |
| LOC Local timer interrupts | 1685 |
| CAL Function call interrupts | 1010 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## c1-base/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c1-base/noise, loop 0 on CPU 2

11196993536 iterations, longest 78.9 µs; over 5 / 20 / 100 µs: 403 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 379 | 3394.6 | 28.9 |
| mostly unexplained (+call function (IPI/vector)) | 20 | 105.6 | 5.7 |
| softirq TIMER | 1 | 78.9 | 78.9 |
| mostly unexplained (+local timer tick) | 3 | 26.3 | 9.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 78.9 | 0 | softirq TIMER | softirq TIMER 63, local timer tick 8, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 28.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 25.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 19.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 16.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 15.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 15.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 14.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 14.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 14.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (c1-base/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 793300096 | 165.27 | 1652709 | 1642709 | 99.4 | 68.5 | true | 0 | 8841 | 228.2 | 960002 / 1284 / 4 | 0 | 0 |

#### Stalls: c1-base/pp-recv-busy-10000, loop 0 on CPU 2

788500096 iterations, longest 228.2 µs; over 5 / 20 / 100 µs: 960002 / 1284 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 710 | 15710.4 | 33.7 |
| softirq NET_RX | 271 | 5875.3 | 28.8 |
| mostly unexplained (+softirq NET_RX) | 200 | 4676.8 | 33.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 74 | 2258.7 | 146.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 10 | 455.4 | 228.2 |
| mostly unexplained (+irq work (IPI/vector)) | 15 | 366.2 | 39.3 |
| softirq TIMER | 1 | 221.6 | 221.6 |
| mostly unexplained (+local timer tick) | 3 | 65.8 | 23.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 228.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 221.6 | 0 | softirq TIMER | softirq TIMER 208, local timer tick 7, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 3 | 146.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 146.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 42.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 42.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 39.3 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 34.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 34.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 33.7 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 8967 |
| IWI IRQ work interrupts | 8858 |
| CAL Function call interrupts | 1058 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: c1-base/pp-recv-busy-10000 (median 43.90)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1023.80 | 979.90 | 0.00 |  | 0.00 |  | 979.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 980.04 | 936.14 | 0.00 |  | 0.00 |  | 936.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 325.51 | 281.61 | 0.00 |  | 0.00 |  | 281.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 261.78 | 217.88 | 228.21 | VM paused: the idle spare core stalled too (hypervisor) | 0.00 |  | 0.00 | client: VM paused: the idle spare core stalled too (hypervisor) |
| 5 | 253.54 | 209.64 | 0.00 |  | 157.96 | softirq TIMER | 51.68 | server: softirq TIMER |
| 6 | 224.55 | 180.65 | 0.00 |  | 0.00 |  | 180.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 215.00 | 171.10 | 0.00 |  | 0.00 |  | 171.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 196.23 | 152.33 | 0.00 |  | 0.00 |  | 152.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 189.48 | 145.59 | 146.01 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 188.74 | 144.84 | 146.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

#### Stalls: c1-nodim/http-recv-busy-10000, loop 0 on CPU 2

6775408768 iterations, longest 328.2 µs; over 5 / 20 / 100 µs: 1799975 / 284 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 246 | 6484.5 | 99.4 |
| mostly unexplained (+softirq NET_RX) | 22 | 515.3 | 41.2 |
| softirq TIMER | 1 | 328.2 | 328.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 11 | 324.5 | 61.0 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 112.6 | 39.3 |
| loop work | 1 | 31.0 | 31.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 328.2 | 1 | softirq TIMER | softirq TIMER 302, local timer tick 15, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 99.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 61.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 53.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 41.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 4, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 39.3 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 37.7 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 35.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 35.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 35.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1058 |
| IWI IRQ work interrupts | 10 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c1-nodim/http-recv-busy-10000 (median 44.06)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 980.65 | 936.59 | 0.00 |  | 0.00 |  | 936.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 212.59 | 168.53 | 0.00 |  | 205.16 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 180.65 | 136.59 | 0.00 |  | 0.00 |  | 136.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 180.53 | 136.47 | 0.00 |  | 0.00 |  | 136.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 176.87 | 132.81 | 0.00 |  | 0.00 |  | 132.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 175.35 | 131.29 | 0.00 |  | 0.00 |  | 131.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 172.87 | 128.81 | 0.00 |  | 0.00 |  | 128.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 170.77 | 126.71 | 0.00 |  | 0.00 |  | 126.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 169.89 | 125.83 | 0.00 |  | 0.00 |  | 125.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 169.47 | 125.41 | 0.00 |  | 0.00 |  | 125.41 | neither loop stalled long enough: network, hypervisor or untraced |

## c1-nodim/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1626427 clocksource=tsc

### Loop counters (c1-nodim/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1808857 | 4.42 | 271951680 | 33.98 | 1699698 | 1699698 | 99.3 | 67.2 | true | 0 | 0 | 616.5 | 240700 / 5959 / 4 | 4002800 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8200 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792627200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (146495 of 3897150 in 1-msg reads) | 146495 | 2.42 | 2.25 | 2.63 | 5.20 | 18.02 | 20.40 | 23.66 |

#### Stalls: c1-nodim/md-recv-busy, loop 0 on CPU 2

271951680 iterations, longest 616.5 µs; over 5 / 20 / 100 µs: 240700 / 5959 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5926 | 132885.5 | 616.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 19 | 520.0 | 41.3 |
| softirq TIMER | 1 | 341.7 | 341.7 |
| mostly unexplained (+irq work (IPI/vector)) | 7 | 167.5 | 28.9 |
| softirq NET_RX | 4 | 89.5 | 25.0 |
| mostly unexplained (+softirq NET_RX) | 2 | 43.1 | 22.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 616.5 | 2800 | loop work (2800 items) |  |
| 2 | 341.7 | 0 | softirq TIMER | softirq TIMER 319, local timer tick 13, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 3 | 145.9 | 28 | loop work (28 items) | softirq NET_RX 1 |
| 4 | 135.2 | 20 | loop work (20 items) |  |
| 5 | 51.0 | 28 | loop work (28 items) |  |
| 6 | 49.7 | 25 | loop work (25 items) |  |
| 7 | 45.3 | 21 | loop work (21 items) |  |
| 8 | 41.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 39.4 | 25 | loop work (25 items) |  |
| 10 | 37.9 | 11 | loop work (11 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1010 |
| IWI IRQ work interrupts | 42 |
| LOC Local timer interrupts | 11 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## c1-nodim/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c1-nodim/noise, loop 0 on CPU 2

11202064384 iterations, longest 80.3 µs; over 5 / 20 / 100 µs: 404 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 394 | 3473.6 | 34.3 |
| softirq TIMER | 1 | 80.3 | 80.3 |
| mostly unexplained (+call function (IPI/vector)) | 9 | 48.4 | 6.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 80.3 | 0 | softirq TIMER | softirq TIMER 56, local timer tick 12, timer tick_nohz_handler 5 |
| 2 | 34.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 26.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 17.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 16.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 16.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 15.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 14.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 14.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 14.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (c1-nodim/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 807809024 | 168.29 | 1682935 | 1672935 | 99.4 | 68.2 | true | 0 | 0 | 330.1 | 292812 / 79 / 3 | 0 | 0 |

#### Stalls: c1-nodim/pp-recv-busy-10000, loop 0 on CPU 2

803009024 iterations, longest 330.1 µs; over 5 / 20 / 100 µs: 292812 / 79 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 55 | 1884.9 | 210.0 |
| softirq TIMER | 1 | 330.1 | 330.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 9 | 251.3 | 33.7 |
| mostly unexplained (+irq work (IPI/vector)) | 8 | 236.8 | 37.4 |
| mostly unexplained (+softirq NET_RX) | 5 | 127.4 | 32.6 |
| mostly unexplained (+local timer tick) | 1 | 34.5 | 34.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 330.1 | 1 | softirq TIMER | softirq TIMER 305, local timer tick 15, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 210.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 123.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 89.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 81.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 42.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 37.4 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 34.5 | 1 | mostly unexplained (+local timer tick) | local timer tick 15, timer tick_nohz_handler 8, softirq TIMER 3, softirq RCU 1 |
| 9 | 33.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 32.7 | 1 | VM paused: the idle spare core stalled too (hypervisor) | softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1058 |
| IWI IRQ work interrupts | 15 |
| LOC Local timer interrupts | 9 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c1-nodim/pp-recv-busy-10000 (median 40.88)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 354.05 | 313.17 | 329.62 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 2 | 154.26 | 113.38 | 0.00 |  | 183.64 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 153.22 | 112.34 | 0.00 |  | 0.00 |  | 112.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 134.97 | 94.09 | 0.00 |  | 0.00 |  | 94.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 133.80 | 92.92 | 0.00 |  | 0.00 |  | 92.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 131.90 | 91.02 | 0.00 |  | 0.00 |  | 91.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 131.00 | 90.13 | 0.00 |  | 0.00 |  | 90.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 130.78 | 89.91 | 0.00 |  | 0.00 |  | 89.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 130.67 | 89.79 | 0.00 |  | 0.00 |  | 89.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 130.06 | 89.18 | 0.00 |  | 0.00 |  | 89.18 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c1-nocoalesce/http-recv-busy-10000, loop 0 on CPU 2

7080449024 iterations, longest 395.8 µs; over 5 / 20 / 100 µs: 1253356 / 296 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 209 | 5248.7 | 161.7 |
| mostly unexplained (+softirq NET_RX) | 73 | 1607.7 | 33.8 |
| softirq TIMER | 1 | 395.8 | 395.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 9 | 303.2 | 78.0 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 107.4 | 38.9 |
| loop work | 1 | 23.7 | 23.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 395.8 | 0 | softirq TIMER | softirq TIMER 369, local timer tick 21, timer tick_nohz_handler 10, irq work (IPI/vector) 1 |
| 2 | 161.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 143.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 111.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 78.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 44.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 38.9 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 2, ipi_send_cpu 0 |
| 8 | 38.6 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 35.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 34.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1058 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: c1-nocoalesce/http-recv-busy-10000 (median 41.69)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 201.65 | 159.96 | 0.00 |  | 0.00 |  | 159.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 174.42 | 132.73 | 0.00 |  | 0.00 |  | 132.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 174.32 | 132.63 | 0.00 |  | 0.00 |  | 132.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 172.99 | 131.30 | 0.00 |  | 0.00 |  | 131.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 171.83 | 130.14 | 0.00 |  | 0.00 |  | 130.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 169.86 | 128.17 | 0.00 |  | 0.00 |  | 128.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 169.04 | 127.35 | 0.00 |  | 0.00 |  | 127.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 168.15 | 126.46 | 0.00 |  | 0.00 |  | 126.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 168.09 | 126.39 | 0.00 |  | 0.00 |  | 126.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 167.39 | 125.70 | 0.00 |  | 0.00 |  | 125.70 | neither loop stalled long enough: network, hypervisor or untraced |

## c1-nocoalesce/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1563160 clocksource=tsc

### Loop counters (c1-nocoalesce/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1907786 | 4.19 | 264269312 | 33.02 | 1651683 | 1651683 | 99.3 | 67.5 | true | 0 | 0 | 628.3 | 398875 / 12682 / 4 | 4002800 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792627200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (152724 of 3897150 in 1-msg reads) | 152724 | 2.96 | 2.71 | 3.83 | 7.25 | 10.66 | 21.51 | 33.58 |

#### Stalls: c1-nocoalesce/md-recv-busy, loop 0 on CPU 2

264269312 iterations, longest 628.3 µs; over 5 / 20 / 100 µs: 398875 / 12682 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 12560 | 308712.5 | 628.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 51 | 1595.1 | 200.5 |
| softirq NET_RX | 63 | 1589.0 | 36.0 |
| softirq TIMER | 1 | 167.6 | 167.6 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 91.7 | 31.5 |
| mostly unexplained (+softirq NET_RX) | 3 | 61.5 | 20.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 27.3 | 27.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 628.3 | 2800 | loop work (2800 items) |  |
| 2 | 200.5 | 4 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 167.6 | 0 | softirq TIMER | softirq TIMER 142, local timer tick 12, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 4 | 141.0 | 3 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 49.5 | 18 | loop work (18 items) | softirq NET_RX 17 |
| 6 | 46.9 | 19 | loop work (19 items) |  |
| 7 | 46.0 | 19 | loop work (19 items) |  |
| 8 | 45.7 | 13 | loop work (13 items) | softirq NET_RX 12, softirq NET_TX 1 |
| 9 | 45.5 | 36 | loop work (36 items) |  |
| 10 | 45.4 | 20 | loop work (20 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1010 |
| IWI IRQ work interrupts | 21 |
| LOC Local timer interrupts | 7 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## c1-nocoalesce/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c1-nocoalesce/noise, loop 0 on CPU 2

11196153856 iterations, longest 29.1 µs; over 5 / 20 / 100 µs: 415 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 404 | 3526.7 | 29.1 |
| mostly unexplained (+call function (IPI/vector)) | 11 | 58.7 | 7.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 29.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 21.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 18.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 15.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 14.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 14.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 14.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 14.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 14.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 13.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |

### Loop counters (c1-nocoalesce/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 804326400 | 167.57 | 1675680 | 1665680 | 99.4 | 68.8 | true | 0 | 0 | 279.7 | 598373 / 79 / 4 | 0 | 0 |

#### Stalls: c1-nocoalesce/pp-recv-busy-10000, loop 0 on CPU 2

799526400 iterations, longest 279.7 µs; over 5 / 20 / 100 µs: 598373 / 79 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 57 | 1947.5 | 186.5 |
| softirq TIMER | 2 | 458.0 | 279.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 8 | 219.7 | 31.3 |
| mostly unexplained (+softirq NET_RX) | 7 | 182.5 | 38.9 |
| mostly unexplained (+irq work (IPI/vector)) | 5 | 167.2 | 37.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 279.7 | 0 | softirq TIMER | softirq TIMER 252, local timer tick 18, timer tick_nohz_handler 9, irq work (IPI/vector) 1 |
| 2 | 186.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 178.2 | 0 | softirq TIMER | softirq TIMER 146, local timer tick 15, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 4 | 160.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 88.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 46.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 39.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 38.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 4, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 37.2 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 36.8 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1058 |
| IWI IRQ work interrupts | 13 |
| LOC Local timer interrupts | 13 |
| MCP Machine check polls | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c1-nocoalesce/pp-recv-busy-10000 (median 41.90)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 233.02 | 191.11 | 0.00 |  | 205.82 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 214.46 | 172.55 | 0.00 |  | 203.19 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 203.26 | 161.36 | 0.00 |  | 0.00 |  | 161.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 198.94 | 157.04 | 186.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 174.57 | 132.66 | 159.57 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 171.59 | 129.69 | 0.00 |  | 0.00 |  | 129.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 171.55 | 129.65 | 0.00 |  | 0.00 |  | 129.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 150.80 | 108.90 | 0.00 |  | 0.00 |  | 108.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 150.58 | 108.68 | 0.00 |  | 0.00 |  | 108.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 143.49 | 101.58 | 0.00 |  | 0.00 |  | 101.58 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c1-rt/http-recv-busy-10000, loop 0 on CPU 2

7053593835 iterations, longest 235.6 µs; over 5 / 20 / 100 µs: 697925 / 289 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 240 | 5846.9 | 81.0 |
| mostly unexplained (+local timer tick) | 29 | 647.1 | 29.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 10 | 485.0 | 235.6 |
| preempted by kworker/2:1 | 5 | 151.5 | 33.8 |
| mostly unexplained (+preempted by kworker/2:1) | 3 | 96.3 | 36.4 |
| mostly unexplained (+softirq NET_RX) | 1 | 33.9 | 33.9 |
| loop work | 1 | 31.1 | 31.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 235.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 81.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 53.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 40.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 39.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 38.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 36.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 36.4 | 0 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 14, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 36.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 35.5 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 17, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 454627 |
| CAL Function call interrupts | 1057 |
| IWI IRQ work interrupts | 29 |
| RES Rescheduling interrupts | 27 |
| MCP Machine check polls | 1 |

#### Slowest round trips: c1-rt/http-recv-busy-10000 (median 41.32)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 982.27 | 940.95 | 0.00 |  | 0.00 |  | 940.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 193.72 | 152.40 | 0.00 |  | 210.21 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 131.24 | 89.92 | 0.00 |  | 0.00 |  | 89.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 130.01 | 88.69 | 0.00 |  | 0.00 |  | 88.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 129.62 | 88.30 | 0.00 |  | 0.00 |  | 88.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 128.88 | 87.56 | 0.00 |  | 0.00 |  | 87.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 126.81 | 85.49 | 0.00 |  | 0.00 |  | 85.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 126.68 | 85.36 | 0.00 |  | 0.00 |  | 85.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 124.75 | 83.43 | 0.00 |  | 0.00 |  | 83.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 124.31 | 82.99 | 0.00 |  | 0.00 |  | 82.99 | neither loop stalled long enough: network, hypervisor or untraced |

## c1-rt/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1721782 clocksource=tsc

### Loop counters (c1-rt/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1895382 | 4.22 | 264287360 | 33.02 | 1651796 | 1651796 | 99.3 | 67.4 | true | 0 | 966 | 662.7 | 445256 / 9329 / 6 | 4002800 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792627200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (181606 of 3897150 in 1-msg reads) | 181606 | 2.65 | 2.42 | 3.17 | 6.43 | 10.64 | 16.42 | 27.51 |

#### Stalls: c1-rt/md-recv-busy, loop 0 on CPU 2

264287360 iterations, longest 662.7 µs; over 5 / 20 / 100 µs: 445256 / 9329 / 6

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 9217 | 214904.8 | 662.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 45 | 1529.3 | 210.9 |
| preempted by kworker/2:1 | 28 | 656.4 | 31.7 |
| mostly unexplained (+preempted by kworker/2:1) | 15 | 340.3 | 27.2 |
| softirq NET_RX | 10 | 242.0 | 32.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 174.1 | 89.5 |
| mostly unexplained (+softirq NET_RX) | 7 | 173.9 | 26.1 |
| mostly unexplained (+local timer tick) | 1 | 125.2 | 125.2 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 70.6 | 37.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 662.7 | 2800 | loop work (2800 items) |  |
| 2 | 210.9 | 4 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 145.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 139.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 133.0 | 8 | loop work (8 items) |  |
| 6 | 125.2 | 0 | mostly unexplained (+local timer tick) | local timer tick 2, timer tick_nohz_handler 1 |
| 7 | 89.5 | 2 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 52.7 | 15 | loop work (15 items) | softirq NET_RX 10 |
| 9 | 51.5 | 28 | loop work (28 items) | softirq NET_RX 15 |
| 10 | 46.8 | 19 | loop work (19 items) | softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 88598 |
| CAL Function call interrupts | 1019 |
| IWI IRQ work interrupts | 86 |
| RES Rescheduling interrupts | 77 |
| TLB TLB shootdowns | 1 |

## c1-rt/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c1-rt/noise, loop 0 on CPU 2

11201863680 iterations, longest 30.9 µs; over 5 / 20 / 100 µs: 403 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 390 | 3438.2 | 30.9 |
| mostly unexplained (+call function (IPI/vector)) | 13 | 68.9 | 6.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 30.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 29.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 15.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 15.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 14.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 14.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 14.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 14.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 14.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 13.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |

### Loop counters (c1-rt/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 807213824 | 168.17 | 1681695 | 1671695 | 99.4 | 68.7 | true | 0 | 4215 | 323.3 | 741307 / 274 / 1 | 0 | 0 |

#### Stalls: c1-rt/pp-recv-busy-10000, loop 0 on CPU 2

802413824 iterations, longest 323.3 µs; over 5 / 20 / 100 µs: 741307 / 274 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 105 | 2736.7 | 53.7 |
| preempted by kworker/2:1 | 109 | 2506.4 | 28.1 |
| mostly unexplained (+preempted by kworker/2:1) | 41 | 1001.0 | 32.5 |
| softirq TIMER | 1 | 323.3 | 323.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 8 | 224.2 | 32.3 |
| mostly unexplained (+softirq NET_RX) | 4 | 97.3 | 28.2 |
| mostly unexplained (+local timer tick) | 2 | 56.9 | 34.1 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 52.2 | 26.4 |
| local timer tick | 2 | 42.8 | 22.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 323.3 | 1 | softirq TIMER | softirq TIMER 310, local timer tick 6, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 2 | 53.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 36.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 35.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 34.1 | 0 | mostly unexplained (+local timer tick) | local timer tick 4, timer tick_nohz_handler 1 |
| 6 | 33.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 33.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 32.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 32.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 255616 |
| CAL Function call interrupts | 1059 |
| IWI IRQ work interrupts | 248 |
| RES Rescheduling interrupts | 241 |
| MCP Machine check polls | 2 |

#### Slowest round trips: c1-rt/pp-recv-busy-10000 (median 44.39)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 579.59 | 535.20 | 0.00 |  | 0.00 |  | 535.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 354.52 | 310.13 | 322.86 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 326.08 | 281.69 | 0.00 |  | 0.00 |  | 281.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 309.67 | 265.28 | 0.00 |  | 0.00 |  | 265.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 297.85 | 253.46 | 0.00 |  | 0.00 |  | 253.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 285.22 | 240.83 | 0.00 |  | 0.00 |  | 240.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 275.56 | 231.17 | 0.00 |  | 0.00 |  | 231.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 272.17 | 227.78 | 0.00 |  | 0.00 |  | 227.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 255.42 | 211.03 | 0.00 |  | 0.00 |  | 211.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 245.08 | 200.69 | 0.00 |  | 0.00 |  | 200.69 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c2-base/http-recv-busy-10000, loop 0 on CPU 2

7111099571 iterations, longest 445.3 µs; over 5 / 20 / 100 µs: 1055653 / 1725 / 4

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 1101 | 23967.8 | 42.4 |
| mostly unexplained (+softirq NET_RX) | 262 | 6070.7 | 35.1 |
| softirq NET_RX | 171 | 3654.7 | 30.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 81 | 2470.5 | 137.4 |
| mostly unexplained (+irq work (IPI/vector)) | 83 | 1821.2 | 39.6 |
| softirq TIMER | 2 | 511.7 | 445.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 12 | 385.8 | 68.4 |
| mostly unexplained (+local timer tick) | 11 | 245.7 | 29.3 |
| loop work | 2 | 42.3 | 22.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 445.3 | 2 | softirq TIMER | softirq TIMER 425, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 2 | 137.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 134.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 107.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 68.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 66.5 | 0 | softirq TIMER | softirq TIMER 56, local timer tick 3, softirq RCU 1, timer tick_nohz_handler 1 |
| 7 | 62.9 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 48.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 46.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 42.4 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 5562 |
| IWI IRQ work interrupts | 5471 |
| CAL Function call interrupts | 1057 |
| MCP Machine check polls | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c2-base/http-recv-busy-10000 (median 41.77)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 432.70 | 390.93 | 66.46 | softirq TIMER | 0.00 |  | 324.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 187.57 | 145.80 | 0.00 |  | 209.34 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 140.17 | 98.40 | 0.00 |  | 0.00 |  | 98.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 132.48 | 90.72 | 0.00 |  | 0.00 |  | 90.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 130.89 | 89.12 | 0.00 |  | 0.00 |  | 89.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 130.67 | 88.90 | 0.00 |  | 0.00 |  | 88.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 127.09 | 85.32 | 0.00 |  | 139.58 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 126.71 | 84.95 | 0.00 |  | 0.00 |  | 84.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 126.34 | 84.57 | 0.00 |  | 0.00 |  | 84.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 125.96 | 84.19 | 0.00 |  | 0.00 |  | 84.19 | neither loop stalled long enough: network, hypervisor or untraced |

## c2-base/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1073341 clocksource=tsc

### Loop counters (c2-base/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002850 | 1234241 | 6.48 | 269068736 | 33.62 | 1681680 | 1681680 | 99.5 | 66.9 | true | 0 | 1945 | 595.8 | 323794 / 7173 / 1 | 4002850 | 0 |

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
| recv-busy (85816 of 3897150 in 1-msg reads) | 85816 | 2.47 | 2.33 | 2.76 | 4.30 | 10.44 | 19.78 | 26.14 |

#### Stalls: c2-base/md-recv-busy, loop 0 on CPU 2

269068736 iterations, longest 595.8 µs; over 5 / 20 / 100 µs: 323794 / 7173 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 6981 | 165336.1 | 595.8 |
| softirq NET_RX | 113 | 2541.9 | 33.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 37 | 1020.9 | 99.0 |
| mostly unexplained (+preempted by kworker/2:1) | 31 | 708.5 | 27.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 5 | 137.6 | 30.3 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 83.7 | 30.8 |
| mostly unexplained (+softirq NET_RX) | 2 | 44.7 | 23.2 |
| softirq TIMER | 1 | 22.1 | 22.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 595.8 | 2850 | loop work (2850 items) |  |
| 2 | 99.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 48.3 | 18 | loop work (18 items) | softirq NET_RX 17, preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 47.7 | 17 | loop work (17 items) | preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 47.3 | 3 | loop work (3 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 47.2 | 12 | loop work (12 items) | preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 45.5 | 18 | loop work (18 items) | softirq NET_RX 14, preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 45.4 | 30 | loop work (30 items) | preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 45.3 | 16 | loop work (16 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 45.0 | 22 | loop work (22 items) | preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1970 |
| LOC Local timer interrupts | 1949 |
| CAL Function call interrupts | 1009 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## c2-base/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c2-base/noise, loop 0 on CPU 2

11192274944 iterations, longest 128.8 µs; over 5 / 20 / 100 µs: 389 / 6 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 380 | 3763.3 | 128.8 |
| softirq TIMER | 1 | 83.7 | 83.7 |
| mostly unexplained (+call function (IPI/vector)) | 8 | 43.3 | 7.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 128.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 105.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 101.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 83.7 | 0 | softirq TIMER | softirq TIMER 64, local timer tick 12, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 5 | 38.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 26.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 16.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 16.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 14.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 14.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

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
| recv-busy | 4800000 | 4800000 | 1.00 | 812871808 | 169.35 | 1693483 | 1683483 | 99.4 | 68.3 | true | 0 | 8906 | 325.8 | 76738 / 855 / 11 | 0 | 0 |

#### Stalls: c2-base/pp-recv-busy-10000, loop 0 on CPU 2

808071808 iterations, longest 325.8 µs; over 5 / 20 / 100 µs: 76738 / 855 / 11

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 512 | 11239.9 | 50.6 |
| softirq NET_RX | 177 | 3809.9 | 29.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 64 | 2753.1 | 149.5 |
| mostly unexplained (+softirq NET_RX) | 81 | 1993.0 | 117.0 |
| softirq TIMER | 1 | 325.8 | 325.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 8 | 309.1 | 101.4 |
| mostly unexplained (+irq work (IPI/vector)) | 10 | 285.0 | 33.8 |
| mostly unexplained (+local timer tick) | 2 | 50.2 | 29.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 325.8 | 0 | softirq TIMER | softirq TIMER 312, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 2 | 149.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 147.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 141.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 140.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 139.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 137.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 121.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 120.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 117.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 9072 |
| IWI IRQ work interrupts | 8922 |
| CAL Function call interrupts | 1058 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c2-base/pp-recv-busy-10000 (median 40.65)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 259.23 | 218.58 | 0.00 |  | 0.00 |  | 218.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 240.80 | 200.15 | 0.00 |  | 153.10 | softirq TIMER | 47.05 | server: softirq TIMER |
| 3 | 217.10 | 176.44 | 0.00 |  | 0.00 |  | 176.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 204.99 | 164.34 | 0.00 |  | 0.00 |  | 164.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 204.39 | 163.74 | 0.00 |  | 0.00 |  | 163.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 198.44 | 157.79 | 121.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 36.08 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 195.67 | 155.02 | 0.00 |  | 0.00 |  | 155.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 193.14 | 152.49 | 0.00 |  | 0.00 |  | 152.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 190.72 | 150.07 | 0.00 |  | 0.00 |  | 150.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 188.73 | 148.07 | 0.00 |  | 0.00 |  | 148.07 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c2-nodim/http-recv-busy-10000, loop 0 on CPU 2

7131748235 iterations, longest 221.0 µs; over 5 / 20 / 100 µs: 984861 / 733 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 412 | 9207.7 | 121.2 |
| mostly unexplained (+softirq NET_RX) | 303 | 6451.7 | 37.9 |
| softirq TIMER | 2 | 349.8 | 221.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 11 | 308.3 | 40.7 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 88.2 | 45.1 |
| loop work | 3 | 72.0 | 32.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 221.0 | 1 | softirq TIMER | softirq TIMER 199, local timer tick 13, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 128.8 | 1 | softirq TIMER | softirq TIMER 108, local timer tick 14, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 3 | 121.2 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 57.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 45.1 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 2, softirq NET_RX 1, ipi_send_cpu 0 |
| 6 | 43.0 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 7 | 40.7 | 1 | VM paused: the idle spare core stalled too (hypervisor) | softirq NET_RX 1 |
| 8 | 37.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 37.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 35.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1058 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c2-nodim/http-recv-busy-10000 (median 41.52)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 201.50 | 159.98 | 0.00 |  | 0.00 |  | 159.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 188.48 | 146.97 | 0.00 |  | 162.28 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 185.92 | 144.40 | 0.00 |  | 0.00 |  | 144.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 171.89 | 130.37 | 0.00 |  | 134.87 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 164.51 | 122.99 | 0.00 |  | 0.00 |  | 122.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 164.35 | 122.84 | 0.00 |  | 132.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 160.57 | 119.05 | 0.00 |  | 0.00 |  | 119.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 158.02 | 116.50 | 0.00 |  | 0.00 |  | 116.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 156.93 | 115.41 | 0.00 |  | 124.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 153.36 | 111.85 | 0.00 |  | 0.00 |  | 111.85 | neither loop stalled long enough: network, hypervisor or untraced |

## c2-nodim/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1761456 clocksource=tsc

### Loop counters (c2-nodim/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002823 | 1939291 | 4.13 | 271865344 | 33.97 | 1699158 | 1699158 | 99.3 | 67.3 | true | 0 | 0 | 703.8 | 251213 / 16119 / 3 | 4002823 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792632352 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (154694 of 3897150 in 1-msg reads) | 154694 | 2.35 | 2.25 | 2.49 | 3.39 | 17.25 | 19.89 | 21.41 |

#### Stalls: c2-nodim/md-recv-busy, loop 0 on CPU 2

271865344 iterations, longest 703.8 µs; over 5 / 20 / 100 µs: 251213 / 16119 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 15669 | 434677.2 | 703.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 423 | 9817.0 | 73.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 333.2 | 253.9 |
| softirq TIMER | 1 | 247.0 | 247.0 |
| mostly unexplained (+softirq NET_RX) | 9 | 212.8 | 27.7 |
| softirq NET_RX | 7 | 182.1 | 33.6 |
| mostly unexplained (+irq work (IPI/vector)) | 6 | 160.4 | 32.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 703.8 | 2800 | loop work (2800 items) |  |
| 2 | 253.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 247.0 | 0 | softirq TIMER | softirq TIMER 222, local timer tick 15, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 4 | 73.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 47.8 | 23 | loop work (23 items) |  |
| 6 | 47.5 | 22 | loop work (22 items) |  |
| 7 | 46.9 | 22 | loop work (22 items) |  |
| 8 | 45.7 | 28 | loop work (28 items) | softirq NET_RX 7 |
| 9 | 45.4 | 15 | loop work (15 items) |  |
| 10 | 45.1 | 23 | loop work (23 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1012 |
| IWI IRQ work interrupts | 34 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## c2-nodim/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c2-nodim/noise, loop 0 on CPU 2

11224047616 iterations, longest 79.2 µs; over 5 / 20 / 100 µs: 555 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 532 | 4648.0 | 28.8 |
| mostly unexplained (+call function (IPI/vector)) | 16 | 85.2 | 5.8 |
| softirq TIMER | 1 | 79.2 | 79.2 |
| local timer tick | 3 | 22.7 | 11.2 |
| mostly unexplained (+local timer tick) | 2 | 15.6 | 9.1 |
| call function (IPI/vector) | 1 | 10.4 | 10.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 79.2 | 0 | softirq TIMER | softirq TIMER 65, local timer tick 8, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 28.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 27.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 19.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 17.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 16.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 14.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 14.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 14.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 14.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (c2-nodim/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 805841408 | 167.88 | 1678836 | 1668836 | 99.4 | 68.6 | true | 0 | 0 | 125.4 | 541076 / 97 / 1 | 0 | 0 |

#### Stalls: c2-nodim/pp-recv-busy-10000, loop 0 on CPU 2

801041408 iterations, longest 125.4 µs; over 5 / 20 / 100 µs: 541076 / 97 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 73 | 1958.3 | 38.8 |
| mostly unexplained (+irq work (IPI/vector)) | 8 | 269.8 | 38.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 10 | 268.6 | 35.3 |
| softirq TIMER | 1 | 125.4 | 125.4 |
| mostly unexplained (+softirq NET_RX) | 5 | 116.2 | 29.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 125.4 | 0 | softirq TIMER | softirq TIMER 97, local timer tick 18, timer tick_nohz_handler 11, irq work (IPI/vector) 1 |
| 2 | 38.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 38.2 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 36.4 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 35.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 35.9 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 2, ipi_send_cpu 0 |
| 7 | 35.6 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 35.3 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 34.1 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 34.0 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1062 |
| IWI IRQ work interrupts | 15 |
| LOC Local timer interrupts | 9 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c2-nodim/pp-recv-busy-10000 (median 40.93)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 970.07 | 929.15 | 0.00 |  | 0.00 |  | 929.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 226.50 | 185.57 | 0.00 |  | 184.66 | softirq TIMER | 0.91 | server: softirq TIMER |
| 3 | 210.60 | 169.67 | 0.00 |  | 0.00 |  | 169.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 203.50 | 162.57 | 0.00 |  | 0.00 |  | 162.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 202.39 | 161.46 | 0.00 |  | 0.00 |  | 161.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 182.02 | 141.09 | 0.00 |  | 0.00 |  | 141.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 174.20 | 133.27 | 0.00 |  | 0.00 |  | 133.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 167.67 | 126.74 | 0.00 |  | 0.00 |  | 126.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 167.66 | 126.73 | 0.00 |  | 0.00 |  | 126.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 167.46 | 126.53 | 0.00 |  | 0.00 |  | 126.53 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c2-nocoalesce/http-recv-busy-10000, loop 0 on CPU 2

7202305858 iterations, longest 243.5 µs; over 5 / 20 / 100 µs: 229180 / 787 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 729 | 18356.8 | 141.9 |
| mostly unexplained (+softirq NET_RX) | 39 | 849.6 | 27.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 10 | 472.3 | 243.5 |
| mostly unexplained (+irq work (IPI/vector)) | 6 | 193.4 | 42.9 |
| softirq TIMER | 1 | 187.0 | 187.0 |
| loop work | 2 | 41.8 | 21.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 243.5 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 187.0 | 1 | softirq TIMER | softirq TIMER 161, local timer tick 16, timer tick_nohz_handler 9, irq work (IPI/vector) 1 |
| 3 | 141.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 60.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 42.9 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 40.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 39.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 38.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 36.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 36.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1057 |
| IWI IRQ work interrupts | 13 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c2-nocoalesce/http-recv-busy-10000 (median 40.71)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 226.79 | 186.08 | 0.00 |  | 0.00 |  | 186.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 144.31 | 103.60 | 0.00 |  | 139.28 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 143.71 | 103.00 | 0.00 |  | 126.60 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 132.91 | 92.20 | 0.00 |  | 125.14 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 131.94 | 91.23 | 0.00 |  | 0.00 |  | 91.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 129.86 | 89.15 | 0.00 |  | 0.00 |  | 89.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 129.65 | 88.94 | 0.00 |  | 126.75 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 127.57 | 86.86 | 0.00 |  | 0.00 |  | 86.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 123.21 | 82.50 | 0.00 |  | 0.00 |  | 82.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 122.72 | 82.01 | 0.00 |  | 0.00 |  | 82.01 | neither loop stalled long enough: network, hypervisor or untraced |

## c2-nocoalesce/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1529413 clocksource=tsc

### Loop counters (c2-nocoalesce/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1858214 | 4.31 | 264857408 | 33.10 | 1655359 | 1655359 | 99.3 | 67.4 | true | 0 | 0 | 703.4 | 363865 / 12973 / 3 | 4002800 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1792627200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (136614 of 3897200 in 1-msg reads) | 136614 | 2.93 | 2.62 | 3.83 | 7.69 | 12.06 | 19.91 | 33.97 |

#### Stalls: c2-nocoalesce/md-recv-busy, loop 0 on CPU 2

264857408 iterations, longest 703.4 µs; over 5 / 20 / 100 µs: 363865 / 12973 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 12802 | 340609.1 | 703.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 96 | 2336.2 | 40.6 |
| softirq NET_RX | 61 | 1650.9 | 44.8 |
| mostly unexplained (+softirq NET_RX) | 7 | 155.1 | 25.9 |
| softirq TIMER | 1 | 130.5 | 130.5 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 99.1 | 36.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 90.4 | 32.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 703.4 | 2800 | loop work (2800 items) |  |
| 2 | 130.5 | 0 | softirq TIMER | softirq TIMER 104, local timer tick 16, timer tick_nohz_handler 9, irq work (IPI/vector) 1 |
| 3 | 128.5 | 36 | loop work (36 items) | softirq NET_RX 1 |
| 4 | 70.6 | 32 | loop work (32 items) | softirq NET_RX 16 |
| 5 | 55.3 | 18 | loop work (18 items) | softirq NET_RX 10 |
| 6 | 54.8 | 25 | loop work (25 items) |  |
| 7 | 54.8 | 35 | loop work (35 items) |  |
| 8 | 54.6 | 26 | loop work (26 items) | softirq NET_RX 10 |
| 9 | 54.0 | 22 | loop work (22 items) | softirq NET_RX 16 |
| 10 | 52.8 | 28 | loop work (28 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1010 |
| IWI IRQ work interrupts | 31 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## c2-nocoalesce/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c2-nocoalesce/noise, loop 0 on CPU 2

11204198400 iterations, longest 29.4 µs; over 5 / 20 / 100 µs: 424 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 394 | 3522.3 | 29.4 |
| mostly unexplained (+call function (IPI/vector)) | 25 | 136.5 | 6.9 |
| mostly unexplained (+local timer tick) | 4 | 29.4 | 9.1 |
| local timer tick | 1 | 14.3 | 14.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 29.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 25.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 18.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 16.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 15.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 14.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 14.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 14.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 14.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 14.3 | 0 | local timer tick | local timer tick 8, timer tick_nohz_handler 5 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |
| LOC Local timer interrupts | 5 |
| IWI IRQ work interrupts | 1 |
| RES Rescheduling interrupts | 1 |

### Loop counters (c2-nocoalesce/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 804331072 | 167.57 | 1675690 | 1665690 | 99.4 | 68.6 | true | 0 | 0 | 329.7 | 601653 / 92 / 3 | 0 | 0 |

#### Stalls: c2-nocoalesce/pp-recv-busy-10000, loop 0 on CPU 2

799531072 iterations, longest 329.7 µs; over 5 / 20 / 100 µs: 601653 / 92 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 62 | 1867.2 | 201.6 |
| softirq TIMER | 2 | 576.6 | 329.7 |
| mostly unexplained (+irq work (IPI/vector)) | 11 | 343.1 | 35.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 11 | 326.2 | 43.7 |
| mostly unexplained (+softirq NET_RX) | 6 | 149.6 | 33.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 329.7 | 1 | softirq TIMER | softirq TIMER 303, local timer tick 14, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 246.8 | 1 | softirq TIMER | softirq TIMER 208, local timer tick 17, timer tick_nohz_handler 9, softirq NET_RX 6 |
| 3 | 201.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 43.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 42.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 36.8 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 35.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 35.7 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 9 | 35.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, softirq NET_RX 1, ipi_send_cpu 0 |
| 10 | 34.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1058 |
| IWI IRQ work interrupts | 18 |
| LOC Local timer interrupts | 12 |
| MCP Machine check polls | 2 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: c2-nocoalesce/pp-recv-busy-10000 (median 41.55)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 343.68 | 302.13 | 329.25 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 2 | 257.02 | 215.47 | 246.21 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 187.90 | 146.34 | 0.00 |  | 0.00 |  | 146.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 185.13 | 143.57 | 0.00 |  | 0.00 |  | 143.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 184.73 | 143.17 | 0.00 |  | 0.00 |  | 143.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 181.17 | 139.61 | 0.00 |  | 0.00 |  | 139.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 180.91 | 139.35 | 0.00 |  | 0.00 |  | 139.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 180.75 | 139.20 | 0.00 |  | 0.00 |  | 139.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 179.90 | 138.35 | 0.00 |  | 0.00 |  | 138.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 176.89 | 135.33 | 0.00 |  | 0.00 |  | 135.33 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: c2-rt/http-recv-busy-10000, loop 0 on CPU 2

7140596848 iterations, longest 224.0 µs; over 5 / 20 / 100 µs: 1367985 / 705 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/2:1) | 453 | 10771.5 | 47.1 |
| preempted by kworker/2:1 | 94 | 2608.9 | 32.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 92 | 2524.5 | 122.8 |
| mostly unexplained (+softirq NET_RX) | 41 | 1122.2 | 224.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 14 | 366.4 | 31.8 |
| mostly unexplained (+local timer tick) | 6 | 156.0 | 37.1 |
| softirq TIMER | 1 | 54.4 | 54.4 |
| local timer tick | 2 | 43.4 | 22.8 |
| loop work | 2 | 42.6 | 22.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 224.0 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 122.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 97.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 54.4 | 1 | softirq TIMER | softirq TIMER 48, local timer tick 3, timer tick_nohz_handler 1 |
| 5 | 47.1 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 19, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 40.0 | 0 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 10, local timer tick 6, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 7 | 39.3 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 3, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 38.7 | 0 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 12, local timer tick 5, irq work (IPI/vector) 2, timer tick_nohz_handler 2 |
| 9 | 37.9 | 0 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 8, local timer tick 6, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 37.1 | 0 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 9, local timer tick 5, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 263585 |
| CAL Function call interrupts | 1051 |
| IWI IRQ work interrupts | 235 |
| RES Rescheduling interrupts | 230 |
| MCP Machine check polls | 1 |

#### Slowest round trips: c2-rt/http-recv-busy-10000 (median 41.58)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 229.95 | 188.38 | 222.74 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 2 | 225.37 | 183.79 | 0.00 |  | 213.20 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 171.56 | 129.99 | 0.00 |  | 0.00 |  | 129.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 165.04 | 123.47 | 0.00 |  | 134.00 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 161.77 | 120.20 | 0.00 |  | 134.55 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 6 | 150.32 | 108.75 | 0.00 |  | 131.53 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 137.95 | 96.38 | 0.00 |  | 0.00 |  | 96.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 137.94 | 96.36 | 0.00 |  | 0.00 |  | 96.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 136.60 | 95.03 | 97.50 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 135.95 | 94.38 | 0.00 |  | 0.00 |  | 94.38 | neither loop stalled long enough: network, hypervisor or untraced |

## c2-rt/md-recv-busy: lab, 160 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1723004 clocksource=tsc

### Loop counters (c2-rt/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1897671 | 4.22 | 264651520 | 33.07 | 1654072 | 1654072 | 99.3 | 67.4 | true | 0 | 730 | 671.5 | 406130 / 9256 / 2 | 4002800 | 0 |

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
| recv-busy (152063 of 3897150 in 1-msg reads) | 152063 | 2.61 | 2.43 | 3.10 | 4.00 | 8.01 | 14.63 | 24.90 |

#### Stalls: c2-rt/md-recv-busy, loop 0 on CPU 2

264651520 iterations, longest 671.5 µs; over 5 / 20 / 100 µs: 406130 / 9256 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 9174 | 210565.5 | 671.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 24 | 861.2 | 171.8 |
| preempted by kworker/2:1 | 31 | 744.9 | 36.7 |
| mostly unexplained (+preempted by kworker/2:1) | 12 | 287.8 | 29.3 |
| softirq NET_RX | 10 | 215.0 | 23.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 56.5 | 30.4 |
| mostly unexplained (+local timer tick) | 1 | 34.3 | 34.3 |
| local timer tick | 1 | 31.6 | 31.6 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 31.4 | 31.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 671.5 | 2800 | loop work (2800 items) |  |
| 2 | 171.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 96.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 47.7 | 13 | loop work (13 items) |  |
| 5 | 47.1 | 47 | loop work (47 items) |  |
| 6 | 46.0 | 24 | loop work (24 items) |  |
| 7 | 42.8 | 22 | loop work (22 items) | softirq NET_RX 9 |
| 8 | 41.1 | 14 | loop work (14 items) | preempted by kworker/2:1 11, irq work (IPI/vector) 1, reschedule (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 40.4 | 18 | loop work (18 items) |  |
| 10 | 40.2 | 11 | loop work (11 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 87649 |
| CAL Function call interrupts | 1005 |
| IWI IRQ work interrupts | 87 |
| RES Rescheduling interrupts | 78 |
| TLB TLB shootdowns | 1 |

## c2-rt/noise: platform noise on CPU 2 (160 s, a loop that only reads the counter)

#### Stalls: c2-rt/noise, loop 0 on CPU 2

11197067264 iterations, longest 30.9 µs; over 5 / 20 / 100 µs: 386 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 371 | 3311.5 | 30.9 |
| mostly unexplained (+call function (IPI/vector)) | 15 | 81.5 | 7.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 30.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 27.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 17.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 15.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 14.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 14.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 14.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 14.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 14.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 13.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |

### Loop counters (c2-rt/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 800668224 | 166.81 | 1668059 | 1658059 | 99.4 | 68.7 | true | 0 | 5008 | 111.5 | 785301 / 552 / 2 | 0 | 0 |

#### Stalls: c2-rt/pp-recv-busy-10000, loop 0 on CPU 2

795868224 iterations, longest 111.5 µs; over 5 / 20 / 100 µs: 785301 / 552 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 312 | 7875.4 | 111.5 |
| preempted by kworker/2:1 | 93 | 2152.3 | 30.9 |
| mostly unexplained (+local timer tick) | 68 | 1658.4 | 35.2 |
| mostly unexplained (+preempted by kworker/2:1) | 55 | 1268.5 | 32.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 9 | 300.4 | 71.2 |
| mostly unexplained (+softirq NET_RX) | 9 | 218.8 | 35.0 |
| softirq TIMER | 1 | 75.4 | 75.4 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 58.0 | 32.8 |
| mostly unexplained (+preempted by ksoftirqd/2) | 2 | 46.0 | 24.1 |
| local timer tick | 1 | 20.0 | 20.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 111.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 100.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 75.4 | 0 | softirq TIMER | softirq TIMER 63, local timer tick 6, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 4 | 71.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 53.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 39.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 37.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 35.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 35.2 | 0 | mostly unexplained (+local timer tick) | local timer tick 4, timer tick_nohz_handler 1 |
| 10 | 35.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 260508 |
| CAL Function call interrupts | 1052 |
| IWI IRQ work interrupts | 246 |
| RES Rescheduling interrupts | 235 |
| MCP Machine check polls | 2 |

#### Slowest round trips: c2-rt/pp-recv-busy-10000 (median 42.12)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1008.92 | 966.80 | 0.00 |  | 0.00 |  | 966.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 224.96 | 182.85 | 0.00 |  | 67.60 | softirq TIMER | 115.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 177.08 | 134.97 | 0.00 |  | 0.00 |  | 134.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 142.12 | 100.01 | 0.00 |  | 0.00 |  | 100.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 141.11 | 99.00 | 0.00 |  | 0.00 |  | 99.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 138.95 | 96.83 | 0.00 |  | 0.00 |  | 96.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 137.05 | 94.94 | 0.00 |  | 0.00 |  | 94.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 134.05 | 91.94 | 0.00 |  | 0.00 |  | 91.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 133.93 | 91.81 | 0.00 |  | 0.00 |  | 91.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 132.69 | 90.57 | 0.00 |  | 0.00 |  | 90.57 | neither loop stalled long enough: network, hypervisor or untraced |

## Ping-pong round trip (client send → echo decoded; "from when due": from the slot on the paced schedule, including any wait behind a stall)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| c1-base/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989935 | 44.16 | 43.35 | 48.56 | 56.92 | 67.48 | 118.88 | 445.88 |
| c1-base/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989935 | 44.35 | 43.53 | 48.75 | 57.17 | 68.28 | 121.62 | 446.09 |
| c1-base/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 44.58 | 43.90 | 48.77 | 53.62 | 64.19 | 118.85 | 1023.80 |
| c1-base/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 44.92 | 44.24 | 49.12 | 54.00 | 65.19 | 121.85 | 1024.38 |
| c1-nodim/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989935 | 44.79 | 44.06 | 48.99 | 54.42 | 65.98 | 116.06 | 980.65 |
| c1-nodim/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989935 | 44.98 | 44.24 | 49.18 | 54.64 | 66.92 | 120.30 | 980.82 |
| c1-nodim/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 41.20 | 40.88 | 43.61 | 47.16 | 56.94 | 71.89 | 354.05 |
| c1-nodim/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 41.54 | 41.22 | 43.95 | 47.52 | 57.34 | 72.78 | 354.61 |
| c1-nocoalesce/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989936 | 42.15 | 41.69 | 44.99 | 50.10 | 59.60 | 80.99 | 201.65 |
| c1-nocoalesce/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989936 | 42.33 | 41.87 | 45.17 | 50.30 | 60.10 | 88.62 | 201.83 |
| c1-nocoalesce/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 42.26 | 41.90 | 44.84 | 48.59 | 59.76 | 78.50 | 233.02 |
| c1-nocoalesce/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 42.59 | 42.23 | 45.18 | 48.94 | 60.32 | 80.83 | 311.38 |
| c1-rt/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989936 | 41.70 | 41.32 | 44.22 | 49.66 | 59.90 | 75.62 | 982.27 |
| c1-rt/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989936 | 41.90 | 41.51 | 44.41 | 49.87 | 60.23 | 77.12 | 982.45 |
| c1-rt/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 45.09 | 44.39 | 49.28 | 54.00 | 64.28 | 119.81 | 579.59 |
| c1-rt/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 45.42 | 44.72 | 49.62 | 54.37 | 65.61 | 123.05 | 579.82 |
| c2-base/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989936 | 42.25 | 41.77 | 45.15 | 50.62 | 61.53 | 77.06 | 432.70 |
| c2-base/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989936 | 42.42 | 41.94 | 45.32 | 50.81 | 61.77 | 78.06 | 432.85 |
| c2-base/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 41.00 | 40.65 | 43.49 | 47.56 | 59.40 | 116.53 | 259.23 |
| c2-base/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 41.33 | 40.98 | 43.83 | 47.92 | 60.29 | 120.46 | 350.87 |
| c2-nodim/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989935 | 42.00 | 41.52 | 44.68 | 51.33 | 60.39 | 83.44 | 201.50 |
| c2-nodim/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989935 | 42.18 | 41.69 | 44.85 | 51.52 | 60.87 | 89.10 | 201.68 |
| c2-nodim/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 41.35 | 40.93 | 44.15 | 48.43 | 57.93 | 76.02 | 970.07 |
| c2-nodim/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 41.69 | 41.26 | 44.49 | 48.78 | 58.54 | 83.00 | 970.40 |
| c2-nocoalesce/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989936 | 41.38 | 40.71 | 44.27 | 53.93 | 61.22 | 73.78 | 226.79 |
| c2-nocoalesce/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989936 | 41.54 | 40.87 | 44.43 | 54.10 | 61.45 | 74.21 | 226.96 |
| c2-nocoalesce/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 41.94 | 41.55 | 44.42 | 48.69 | 59.60 | 80.41 | 343.68 |
| c2-nocoalesce/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 42.27 | 41.89 | 44.77 | 49.05 | 60.16 | 86.70 | 343.76 |
| c2-rt/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1989936 | 42.08 | 41.58 | 44.99 | 50.88 | 59.53 | 75.47 | 229.95 |
| c2-rt/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1989936 | 42.27 | 41.76 | 45.18 | 51.08 | 59.82 | 77.13 | 230.18 |
| c2-rt/pp-recv-busy-10000: recv-busy @ 10000/s | 4789999 | 42.51 | 42.12 | 44.90 | 50.59 | 59.59 | 73.87 | 1008.92 |
| c2-rt/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 4789999 | 42.84 | 42.45 | 45.24 | 50.96 | 60.00 | 74.70 | 1009.41 |

