# Rows quoted by the docs

Verbatim sections of this run's generated tables, recovered from git (`git show 128747b9^:io/benches/results/flags-aws-20261015T070339Z/tables.md`).
Only the sections `io/benches/BENCHMARK.md` and the book quote; nothing is recomputed. All values µs unless a column says otherwise.

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

### Loop counters (c1-base/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1593359 | 5.02 | 265609920 | 33.19 | 1660062 | 1660062 | 99.4 | 66.9 | true | 0 | 1682 | 607.7 | 379693 / 9327 / 3 | 4002800 | 0 |

### Loop counters (c1-base/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 793300096 | 165.27 | 1652709 | 1642709 | 99.4 | 68.5 | true | 0 | 8841 | 228.2 | 960002 / 1284 / 4 | 0 | 0 |

### Loop counters (c1-nodim/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1808857 | 4.42 | 271951680 | 33.98 | 1699698 | 1699698 | 99.3 | 67.2 | true | 0 | 0 | 616.5 | 240700 / 5959 / 4 | 4002800 | 0 |

### Loop counters (c1-nodim/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 807809024 | 168.29 | 1682935 | 1672935 | 99.4 | 68.2 | true | 0 | 0 | 330.1 | 292812 / 79 / 3 | 0 | 0 |

### Loop counters (c1-nocoalesce/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1907786 | 4.19 | 264269312 | 33.02 | 1651683 | 1651683 | 99.3 | 67.5 | true | 0 | 0 | 628.3 | 398875 / 12682 / 4 | 4002800 | 0 |

### Loop counters (c1-nocoalesce/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 804326400 | 167.57 | 1675680 | 1665680 | 99.4 | 68.8 | true | 0 | 0 | 279.7 | 598373 / 79 / 4 | 0 | 0 |

### Loop counters (c1-rt/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1895382 | 4.22 | 264287360 | 33.02 | 1651796 | 1651796 | 99.3 | 67.4 | true | 0 | 966 | 662.7 | 445256 / 9329 / 6 | 4002800 | 0 |

### Loop counters (c1-rt/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 807213824 | 168.17 | 1681695 | 1671695 | 99.4 | 68.7 | true | 0 | 4215 | 323.3 | 741307 / 274 / 1 | 0 | 0 |

### Loop counters (c2-base/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002850 | 1234241 | 6.48 | 269068736 | 33.62 | 1681680 | 1681680 | 99.5 | 66.9 | true | 0 | 1945 | 595.8 | 323794 / 7173 / 1 | 4002850 | 0 |

### Loop counters (c2-base/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 812871808 | 169.35 | 1693483 | 1683483 | 99.4 | 68.3 | true | 0 | 8906 | 325.8 | 76738 / 855 / 11 | 0 | 0 |

### Loop counters (c2-nodim/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002823 | 1939291 | 4.13 | 271865344 | 33.97 | 1699158 | 1699158 | 99.3 | 67.3 | true | 0 | 0 | 703.8 | 251213 / 16119 / 3 | 4002823 | 0 |

### Loop counters (c2-nodim/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 805841408 | 167.88 | 1678836 | 1668836 | 99.4 | 68.6 | true | 0 | 0 | 125.4 | 541076 / 97 / 1 | 0 | 0 |

### Loop counters (c2-nocoalesce/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1858214 | 4.31 | 264857408 | 33.10 | 1655359 | 1655359 | 99.3 | 67.4 | true | 0 | 0 | 703.4 | 363865 / 12973 / 3 | 4002800 | 0 |

### Loop counters (c2-nocoalesce/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 804331072 | 167.57 | 1675690 | 1665690 | 99.4 | 68.6 | true | 0 | 0 | 329.7 | 601653 / 92 / 3 | 0 | 0 |

### Loop counters (c2-rt/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 8002800 | 1897671 | 4.22 | 264651520 | 33.07 | 1654072 | 1654072 | 99.3 | 67.4 | true | 0 | 730 | 671.5 | 406130 / 9256 / 2 | 4002800 | 0 |

### Loop counters (c2-rt/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4800000 | 4800000 | 1.00 | 800668224 | 166.81 | 1668059 | 1658059 | 99.4 | 68.7 | true | 0 | 5008 | 111.5 | 785301 / 552 / 2 | 0 | 0 |
