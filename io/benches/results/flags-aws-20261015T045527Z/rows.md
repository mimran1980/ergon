# Rows quoted by the docs

Verbatim sections of this run's generated tables, recovered from git (`git show 128747b9^:io/benches/results/flags-aws-20261015T045527Z/tables-local.md`).
Only the sections `io/benches/BENCHMARK.md` and the book quote; nothing is recomputed. All values µs unless a column says otherwise.

## WebSocket round trip, pp-epoll-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1190258 | 48.71 | 63.83 | 98.02 | 132.00 | 343.74 | 305.1 | 0.0 | 0.0 | 0.0 | 42.22 | 10.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1181942 | 49.03 | 97.80 | 101.06 | 135.10 | 258.29 | 6325.5 | 0.0 | 0.0 | 0.0 | 35.54 | 4.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1207389 | 48.97 | 56.50 | 66.46 | 245.40 | 368.77 | 667.1 | 0.0 | 0.0 | 0.0 | 347.29 | 3.0 | client: softirq TIMER |
| r1-base-12 | 1241664 | 47.12 | 55.41 | 61.33 | 81.38 | 187.80 | 94.6 | 0.0 | 0.0 | 0.0 | 30.58 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1271385 | 46.02 (−2.95) | 53.80 (−10.04) | 60.42 (−37.60) | 78.47 (−56.62) | 239.55 (−104.19) | 78.3 | 0.0 | 0.0 | 0.0 | 65.79 | 86.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1253595 | 46.63 (−2.34) | 57.65 (−6.18) | 65.27 (−32.75) | 79.83 (−55.27) | 389.90 (+46.16) | 32.5 | 0.0 | 0.0 | 0.0 | 28.50 | 1.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1234200 | 47.48 (−1.49) | 57.20 (−6.64) | 65.44 (−32.58) | 81.75 (−53.34) | 212.53 (−131.21) | 75.3 | 0.0 | 0.0 | 0.0 | 34.74 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1230369 | 47.84 (−1.14) | 56.20 (−7.63) | 64.24 (−33.78) | 85.56 (−49.54) | 1057.36 (+713.62) | 87.5 | 1.0 | 1.0 | 0.0 | 28.95 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-recv-busy-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1475161 | 39.51 | 46.68 | 57.17 | 69.95 | 1038.29 | 62.0 | 1.0 | 1.0 | 0.0 | 41.20 | 1158.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1442319 | 40.48 | 47.45 | 57.83 | 71.20 | 129.50 | 32.5 | 0.0 | 0.0 | 0.0 | 46.10 | 1149.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1387073 | 41.98 | 49.65 | 56.49 | 88.94 | 221.01 | 120.0 | 0.0 | 0.0 | 0.0 | 114.80 | 589.0 | client: softirq TIMER |
| r1-base-12 | 1450604 | 40.35 | 45.52 | 54.81 | 67.75 | 161.84 | 21.4 | 0.0 | 0.0 | 0.0 | 34.54 | 524.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-stock | 1423663 | 40.68 (+0.19) | 50.59 (+3.14) | 57.70 (+0.53) | 77.06 (+5.85) | 5225.32 (+5004.31) | 104.7 | 1.0 | 1.0 | 0.0 | 164.91 | 395.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1475589 | 39.58 (−0.91) | 46.47 (−0.98) | 53.66 (−3.51) | 67.61 (−3.59) | 220.51 (−0.50) | 64.1 | 0.0 | 0.0 | 0.0 | 59.66 | 19.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1504438 | 38.86 (−1.63) | 44.49 (−2.96) | 51.70 (−5.47) | 64.34 (−6.86) | 1031.97 (+810.96) | 20.3 | 1.0 | 1.0 | 0.0 | 247.63 | 383.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1428501 | 40.96 (+0.47) | 46.33 (−1.12) | 55.51 (−1.67) | 71.77 (+0.56) | 1029.52 (+808.51) | 69.2 | 1.0 | 1.0 | 0.0 | 163.96 | 294.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-sqpoll-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1455907 | 40.00 | 47.03 | 54.53 | 66.07 | 176.29 | 28.5 | 0.0 | 0.0 | 0.0 | 34.12 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1428151 | 40.84 | 45.49 | 52.09 | 65.59 | 195.38 | 22.4 | 0.0 | 0.0 | 0.0 | 30.68 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 1421401 | 41.02 | 45.84 | 54.28 | 68.53 | 861.59 | 16.3 | 1.0 | 0.0 | 0.0 | 390.61 | 4.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1395311 | 41.72 (+0.88) | 48.05 (+2.21) | 53.49 (−0.79) | 64.24 (−1.83) | 130.21 (−65.17) | 15.3 | 0.0 | 0.0 | 0.0 | 42.19 | 47.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1445894 | 40.27 (−0.57) | 47.12 (+1.29) | 55.20 (+0.92) | 72.15 (+6.08) | 184.26 (−11.12) | 55.9 | 0.0 | 0.0 | 0.0 | 32.27 | 1.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1346928 | 42.89 (+2.05) | 52.62 (+6.79) | 58.45 (+4.17) | 91.44 (+25.37) | 196.52 (+1.14) | 119.0 | 0.0 | 0.0 | 0.0 | 30.40 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1401188 | 41.60 (+0.76) | 47.29 (+1.45) | 56.39 (+2.10) | 71.10 (+5.03) | 331.52 (+136.14) | 22.4 | 0.0 | 0.0 | 0.0 | 29.89 | 1.0 | neither loop stalled long enough: network, hypervisor or untraced |
