# Rows quoted by the docs

Verbatim sections of this run's generated tables, recovered from git (`git show 128747b9^:io/benches/results/flags-aws-20261015T062428Z/tables.md`).
Only the sections `io/benches/BENCHMARK.md` and the book quote; nothing is recomputed. All values µs unless a column says otherwise.

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
