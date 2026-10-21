# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

**Runs not included** (they could not be loaded):

- `r1-base-2/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-2/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-base-2/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-2/pp-uring-sqpoll-0`: records.bin: No such file or directory (os error 2)
- `r1-base-2/pp-uring-sqpoll-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-4/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-4/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-base-4/pp-epoll-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-base-4/pp-epoll-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-4/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-base-4/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-4/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-base-4/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-4/pp-uring-sqpoll-0`: records.bin: No such file or directory (os error 2)
- `r1-base-4/pp-uring-sqpoll-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-6/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-6/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-base-6/pp-epoll-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-base-6/pp-epoll-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-6/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-base-6/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-6/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-base-6/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-6/pp-uring-sqpoll-0`: records.bin: No such file or directory (os error 2)
- `r1-base-6/pp-uring-sqpoll-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-8/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-8/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-base-8/pp-epoll-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-base-8/pp-epoll-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-8/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-base-8/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-8/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-base-8/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-base-8/pp-uring-sqpoll-0`: records.bin: No such file or directory (os error 2)
- `r1-base-8/pp-uring-sqpoll-10000`: records.bin: No such file or directory (os error 2)
- `r1-budget64/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-budget64/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-budget64/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-budget64/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-busypoll50/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-busypoll50/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-busypoll50/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-busypoll50/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-cstate/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-cstate/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-cstate/pp-epoll-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-cstate/pp-epoll-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-cstate/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-cstate/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-cstate/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-cstate/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-cstate/pp-uring-sqpoll-0`: records.bin: No such file or directory (os error 2)
- `r1-cstate/pp-uring-sqpoll-10000`: records.bin: No such file or directory (os error 2)
- `r1-housekeeping/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-housekeeping/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-housekeeping/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-housekeeping/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-housekeeping/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-housekeeping/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-housekeeping/pp-uring-sqpoll-0`: records.bin: No such file or directory (os error 2)
- `r1-housekeeping/pp-uring-sqpoll-10000`: records.bin: No such file or directory (os error 2)
- `r1-irqlocal/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-irqlocal/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-irqlocal/pp-epoll-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-irqlocal/pp-epoll-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-irqlocal/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-irqlocal/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-irqlocal/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-irqlocal/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-noagent/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-noagent/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-noagent/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-noagent/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-noagent/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-noagent/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-noagent/pp-uring-sqpoll-0`: records.bin: No such file or directory (os error 2)
- `r1-noagent/pp-uring-sqpoll-10000`: records.bin: No such file or directory (os error 2)
- `r1-nocoalesce/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-nocoalesce/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-nocoalesce/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-nocoalesce/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-nocoalesce/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-nocoalesce/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-nogro/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-nogro/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-nogro/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-nogro/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-nogro/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-nogro/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-noquickack/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-noquickack/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-noquickack/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-noquickack/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-noquickack/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-noquickack/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-prefer/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-prefer/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-prefer/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-prefer/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-prefer/pp-uring-sqpoll-0`: records.bin: No such file or directory (os error 2)
- `r1-prefer/pp-uring-sqpoll-10000`: records.bin: No such file or directory (os error 2)
- `r1-rt/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-rt/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-rt/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-rt/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-rt/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-rt/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-sibling/http-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-sibling/md-recv-busy`: records.bin: No such file or directory (os error 2)
- `r1-sibling/pp-recv-busy-0`: records.bin: No such file or directory (os error 2)
- `r1-sibling/pp-recv-busy-10000`: records.bin: No such file or directory (os error 2)
- `r1-sibling/pp-uring-spin-0`: records.bin: No such file or directory (os error 2)
- `r1-sibling/pp-uring-spin-10000`: records.bin: No such file or directory (os error 2)
- `r1-sibling/pp-uring-sqpoll-0`: records.bin: No such file or directory (os error 2)
- `r1-sibling/pp-uring-sqpoll-10000`: records.bin: No such file or directory (os error 2)

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|
| r1-base-0 | 242.19 | 48.5 | 1.0 | 0.5 | 242.19 µs stall: softirq TIMER |
| r1-housekeeping | 346.00 | 48.0 | 1.0 | 0.5 | 346.00 µs stall: softirq TIMER |
| r1-prefer | 214.07 | 43.5 | 1.0 | 0.5 | 214.07 µs stall: softirq TIMER |
| r1-base-2 | 32.42 | 40.5 | 0.5 | 0.0 | 32.42 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-noquickack | 32.45 | 41.5 | 1.5 | 0.0 | 32.45 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nogro | 34.72 | 43.5 | 0.5 | 0.0 | 34.72 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-4 | 116.64 | 45.5 | 1.0 | 0.5 | 116.64 µs stall: softirq TIMER |
| r1-nocoalesce | 73.83 | 41.5 | 1.5 | 0.0 | 73.83 µs stall: softirq TIMER |
| r1-irqlocal | 43.42 | 386.5 | 17.0 | 0.0 | 43.42 µs stall: softirq NET_RX |
| r1-base-6 | 5060.68 | 47.5 | 2.5 | 1.0 | 5060.68 µs stall: mostly unexplained (+softirq NET_RX) |
| r1-rt | 36.32 | 39.0 | 0.5 | 0.0 | 36.32 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-sibling | 30.36 | 37.0 | 0.5 | 0.0 | 30.36 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-8 | 28.24 | 37.0 | 0.5 | 0.0 | 28.24 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-busypoll50 | 26.82 | 39.5 | 1.0 | 0.0 | 26.82 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-budget64 | 29.27 | 35.0 | 0.5 | 0.0 | 29.27 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 256.54 | 38.0 | 1.0 | 0.5 | 256.54 µs stall: softirq TIMER |
| r1-cstate | 372.99 | 38.0 | 1.5 | 0.5 | 372.99 µs stall: softirq TIMER |
| r1-noagent | 169.84 | 39.0 | 1.5 | 0.5 | 169.84 µs stall: softirq TIMER |
| r1-base-12 | 24.90 | 36.5 | 1.0 | 0.0 | 24.90 µs stall: mostly unexplained (+local timer tick) |
| boot-stock | 150.29 | 60130.0 | 162.0 | 34.0 | 150.29 µs stall: not traced |
| boot-isolcpus | 29.53 | 60046.0 | 0.5 | 0.0 | 29.53 µs stall: not traced |
| boot-full-nomitig | 26.59 | 30.0 | 0.5 | 0.0 | 26.59 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full | 30.48 | 21.0 | 0.5 | 0.0 | 30.48 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |

## HTTP round trip, http-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189944 | 42.01 | 63.77 | 132.29 | 298.71 | 697.59 | 1188.5 | 3.0 | 0.0 | 0.0 | 345.69 | 2748.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1189938 | 41.69 | 58.22 | 71.25 | 118.05 | 183.91 | 135.6 | 0.0 | 0.0 | 0.0 | 89.48 | 2557.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 1189934 | 41.72 | 58.38 | 65.28 | 97.37 | 257.01 | 56.5 | 0.0 | 0.0 | 0.0 | 278.26 | 2343.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1189942 | 40.85 (−0.88) | 52.75 (−5.63) | 86.90 (+15.64) | 95.19 (−22.86) | 132.54 (−124.47) | 38.3 | 0.0 | 0.0 | 0.0 | 66.84 | 1610.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1189943 | 41.20 (−0.53) | 58.65 (+0.27) | 86.31 (+15.06) | 103.71 (−14.34) | 225.76 (−31.25) | 72.6 | 0.0 | 0.0 | 0.0 | 65.18 | 1563.5 | server: softirq TIMER |
| boot-full-nomitig | 1189945 | 42.79 (+1.07) | 48.96 (−9.43) | 86.43 (+15.18) | 94.56 (−23.49) | 592.59 (+335.58) | 7.6 | 0.5 | 0.0 | 0.0 | 88.04 | 1604.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1189943 | 42.76 (+1.04) | 48.44 (−9.95) | 88.44 (+17.19) | 113.99 (−4.06) | 376.54 (+119.53) | 86.7 | 0.0 | 0.0 | 0.0 | 90.15 | 1748.5 | neither loop stalled long enough: network, hypervisor or untraced |

### HTTP round trip, http-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189944 | 42.21 | 66.12 | 336.37 | 780.20 | 1309.53 | 2622.0 | 201.2 | 30.3 | 0.0 |
| r1-base-10 | 1189938 | 41.85 | 58.50 | 71.84 | 125.76 | 257.50 | 165.4 | 0.0 | 0.0 | 0.0 |
| r1-base-12 | 1189934 | 41.88 | 58.59 | 65.74 | 104.25 | 257.17 | 63.0 | 0.0 | 0.0 | 0.0 |
| boot-stock | 1189942 | 41.02 (−0.86) | 52.96 (−5.63) | 87.07 (+15.23) | 95.36 (−30.41) | 132.69 (−124.81) | 38.3 | 0.0 | 0.0 | 0.0 |
| boot-isolcpus | 1189943 | 41.36 (−0.52) | 58.82 (+0.23) | 86.48 (+14.64) | 104.03 (−21.73) | 225.91 (−31.59) | 73.6 | 0.0 | 0.0 | 0.0 |
| boot-full-nomitig | 1189945 | 42.95 (+1.07) | 49.12 (−9.47) | 86.63 (+14.79) | 94.83 (−30.93) | 592.76 (+335.26) | 12.1 | 1.0 | 0.0 | 0.0 |
| boot-full | 1189943 | 42.91 (+1.03) | 48.64 (−9.95) | 88.62 (+16.78) | 114.60 (−11.16) | 376.68 (+119.18) | 88.7 | 0.0 | 0.0 | 0.0 |

## Market-data burst, kernel arrival → decoded (every message), md-recv-busy

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 2900000 | 3.34 | 8.51 | 24.66 | 35.12 | 40.28 | 0.0 | 0.0 | 0.0 | 0.0 | 551.50 | 5225.0 | 551.50 µs stall: loop work (2500 items) |
| r1-base-2 | 2900000 | 2.96 | 7.16 | 18.25 | 29.15 | 38.64 | 0.0 | 0.0 | 0.0 | 0.0 | 436.54 | 4411.0 | 436.54 µs stall: loop work (2450 items) |
| r1-base-10 | 2900000 | 2.05 | 5.13 | 10.26 | 16.47 | 32.46 | 0.0 | 0.0 | 0.0 | 0.0 | 613.11 | 5500.0 | 613.11 µs stall: loop work (2850 items) |
| r1-base-12 | 2900000 | 2.18 | 5.50 | 8.43 | 29.06 | 38.66 | 0.0 | 0.0 | 0.0 | 0.0 | 565.63 | 5515.0 | 565.63 µs stall: loop work (2750 items) |
| boot-stock | 2900000 | 2.33 (−0.63) | 4.85 (−2.31) | 7.14 (−11.11) | 18.40 (−10.75) | 49.34 (+10.67) | 0.0 | 0.0 | 0.0 | 0.0 | 512.62 | 5787.0 | 512.62 µs stall: loop work (2550 items) |
| boot-isolcpus | 2900000 | 2.52 (−0.45) | 7.17 (+0.00) | 15.06 (−3.19) | 24.38 (−4.77) | 29.51 (−9.16) | 0.0 | 0.0 | 0.0 | 0.0 | 530.75 | 5942.0 | 530.75 µs stall: loop work (2500 items) |
| boot-full-nomitig | 2900000 | 2.25 (−0.72) | 6.31 (−0.85) | 11.49 (−6.76) | 31.31 (+2.16) | 39.09 (+0.42) | 0.0 | 0.0 | 0.0 | 0.0 | 461.88 | 4343.0 | 461.88 µs stall: loop work (2450 items) |
| boot-full | 2900000 | 2.61 (−0.35) | 7.17 (+0.01) | 14.02 (−4.23) | 33.62 (+4.47) | 43.66 (+5.00) | 0.0 | 0.0 | 0.0 | 0.0 | 510.06 | 5645.0 | 510.06 µs stall: loop work (2450 items) |

### Market-data burst, kernel arrival → decoded (every message), md-recv-busy, sender → decoded (one way)

Every message from its send time on the sender (or the exchange's event time) to decoded: the whole path, NIC and interrupt included. Two hosts' clocks, so the values include their offset; runs made close together share it, so compare them with each other.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 2900000 | 11.01 | 37.72 | 52.33 | 65.32 | 111.34 | 12.4 | 0.0 | 0.0 | 0.0 |
| r1-base-2 | 2900000 | 26.18 | 49.65 | 64.02 | 81.21 | 153.32 | 19.7 | 0.0 | 0.0 | 0.0 |
| r1-base-10 | 2900000 | 25.23 | 42.00 | 52.08 | 68.34 | 119.42 | 49.7 | 0.0 | 0.0 | 0.0 |
| r1-base-12 | 2900000 | 24.07 | 43.14 | 54.03 | 69.53 | 109.69 | 10.3 | 0.0 | 0.0 | 0.0 |
| boot-stock | 2900000 | 24.32 (−0.91) | 42.51 (−0.63) | 51.93 (−2.10) | 66.38 (−3.15) | 103.48 (−15.94) | 5.2 | 0.0 | 0.0 | 0.0 |
| boot-isolcpus | 2900000 | 46.43 (+21.19) | 66.80 (+23.66) | 77.25 (+23.22) | 93.29 (+23.76) | 230.00 (+110.58) | 170.7 | 0.0 | 0.0 | 0.0 |
| boot-full-nomitig | 2900000 | 20.01 (−5.23) | 44.86 (+1.72) | 59.81 (+5.78) | 73.10 (+3.57) | 96.27 (−23.16) | 0.0 | 0.0 | 0.0 | 0.0 |
| boot-full | 2900000 | 22.47 (−2.77) | 43.10 (−0.04) | 55.38 (+1.35) | 73.06 (+3.53) | 115.61 (−3.81) | 27.9 | 0.0 | 0.0 | 0.0 |

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

## WebSocket round trip, pp-epoll-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189998 | 47.63 | 91.30 | 99.06 | 254.31 | 402.69 | 390.3 | 0.0 | 0.0 | 0.0 | 32.02 | 1.5 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1189999 | 47.21 | 90.47 | 102.56 | 276.07 | 389.61 | 615.6 | 0.0 | 0.0 | 0.0 | 36.24 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1189999 | 48.33 | 53.64 | 61.31 | 77.72 | 127.14 | 10.1 | 0.0 | 0.0 | 0.0 | 28.73 | 1.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 1189999 | 47.75 | 52.57 | 57.90 | 130.19 | 982.30 | 75.6 | 0.5 | 0.0 | 0.0 | 257.66 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1189998 | 47.38 (−0.37) | 255.44 (+164.97) | 326.77 (+227.71) | 386.15 (+131.84) | 4651.53 (+4248.84) | 56335.1 | 0.5 | 0.5 | 0.0 | 58.36 | 47.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1189999 | 48.86 (+1.11) | 54.23 (−36.24) | 77.82 (−21.24) | 265.73 (+11.42) | 404.94 (+2.25) | 521.3 | 0.0 | 0.0 | 0.0 | 62.55 | 1.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1189999 | 55.82 (+8.07) | 91.77 (+1.30) | 95.93 (−3.13) | 235.11 (−19.20) | 425.08 (+22.39) | 281.3 | 0.0 | 0.0 | 0.0 | 151.36 | 3.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1189999 | 49.68 (+1.93) | 54.75 (−35.73) | 63.38 (−35.69) | 85.12 (−169.19) | 1051.33 (+648.63) | 23.7 | 0.5 | 0.5 | 0.0 | 367.19 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-epoll-spin-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189998 | 47.79 | 91.79 | 864.45 | 4444.21 | 5953.20 | 989.2 | 701.9 | 562.2 | 0.0 |
| r1-base-2 | 1189999 | 47.37 | 91.18 | 7921.43 | 19620.81 | 21954.57 | 2043.0 | 1820.7 | 1617.0 | 417.0 |
| r1-base-10 | 1189999 | 48.50 | 53.81 | 61.53 | 77.86 | 127.34 | 10.6 | 0.0 | 0.0 | 0.0 |
| r1-base-12 | 1189999 | 47.90 | 52.74 | 58.62 | 133.66 | 982.42 | 87.2 | 4.5 | 0.0 | 0.0 |
| boot-stock | 1189998 | 48.01 (+0.10) | 60548.67 (+60457.49) | 138707.76 (+137843.30) | 173130.35 (+168686.14) | 177156.92 (+171203.72) | 171826.3 | 166812.0 | 160782.8 | 100831.5 |
| boot-isolcpus | 1189999 | 48.98 (+1.08) | 54.70 (−36.48) | 12777.80 (+11913.35) | 41964.15 (+37519.94) | 45739.85 (+39786.64) | 1712.3 | 1623.0 | 1549.4 | 717.5 |
| boot-full-nomitig | 1189999 | 56.03 (+8.13) | 92.05 (+0.87) | 106.84 (−757.61) | 1091.08 (−3353.14) | 1495.97 (−4457.23) | 699.3 | 383.2 | 104.9 | 0.0 |
| boot-full | 1189999 | 49.85 (+1.94) | 54.92 (−36.26) | 63.87 (−800.58) | 87.64 (−4356.57) | 1051.47 (−4901.73) | 35.3 | 5.5 | 0.5 | 0.0 |

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

## WebSocket round trip, pp-recv-busy-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 41.82 | 48.43 | 62.38 | 80.10 | 134.24 | 45.4 | 0.0 | 0.0 | 0.0 | 42.85 | 909.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 1189999 | 41.58 | 47.36 | 61.87 | 71.67 | 254.92 | 31.8 | 0.0 | 0.0 | 0.0 | 115.22 | 915.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1189999 | 41.17 | 45.73 | 53.71 | 67.85 | 128.12 | 6.1 | 0.0 | 0.0 | 0.0 | 50.12 | 429.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 1189999 | 41.04 | 45.39 | 52.04 | 65.48 | 159.90 | 11.1 | 0.0 | 0.0 | 0.0 | 271.13 | 444.0 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-stock | 1189999 | 40.00 (−1.58) | 44.30 (−3.06) | 49.13 (−12.74) | 62.74 (−8.94) | 177.47 (+17.57) | 6.6 | 0.0 | 0.0 | 0.0 | 41.73 | 23.5 | server: softirq TIMER |
| boot-isolcpus | 1189999 | 41.26 (−0.31) | 45.61 (−1.75) | 55.50 (−6.37) | 70.77 (−0.90) | 128.82 (−31.08) | 7.6 | 0.0 | 0.0 | 0.0 | 32.42 | 4.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1189999 | 41.31 (−0.26) | 45.75 (−1.61) | 53.03 (−8.84) | 77.26 (+5.58) | 972.63 (+812.73) | 10.1 | 0.5 | 0.0 | 0.0 | 43.20 | 352.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1189999 | 43.33 (+1.75) | 50.63 (+3.27) | 55.93 (−5.94) | 94.73 (+23.06) | 163.93 (+4.03) | 53.9 | 0.0 | 0.0 | 0.0 | 43.85 | 315.5 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-recv-busy-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189999 | 42.12 | 48.74 | 62.72 | 80.48 | 134.59 | 45.9 | 0.0 | 0.0 | 0.0 |
| r1-base-2 | 1189999 | 41.88 | 48.29 | 62.35 | 82.24 | 368.39 | 39.3 | 0.0 | 0.0 | 0.0 |
| r1-base-10 | 1189999 | 41.46 | 46.03 | 54.02 | 68.12 | 128.65 | 7.1 | 0.0 | 0.0 | 0.0 |
| r1-base-12 | 1189999 | 41.33 | 45.70 | 52.38 | 67.00 | 255.81 | 13.6 | 0.0 | 0.0 | 0.0 |
| boot-stock | 1189999 | 40.27 (−1.62) | 44.57 (−3.72) | 49.52 (−12.83) | 63.13 (−17.35) | 177.72 (−78.09) | 7.1 | 0.0 | 0.0 | 0.0 |
| boot-isolcpus | 1189999 | 41.54 (−0.34) | 46.00 (−2.29) | 55.78 (−6.57) | 71.21 (−9.27) | 129.09 (−126.72) | 7.6 | 0.0 | 0.0 | 0.0 |
| boot-full-nomitig | 1189999 | 41.60 (−0.28) | 46.07 (−2.23) | 53.43 (−8.91) | 81.58 (+1.10) | 972.95 (+717.14) | 19.2 | 4.0 | 0.0 | 0.0 |
| boot-full | 1189999 | 43.62 (+1.74) | 50.95 (+2.65) | 56.68 (−5.67) | 99.83 (+19.35) | 164.31 (−91.50) | 58.0 | 0.0 | 0.0 | 0.0 |

## WebSocket round trip, pp-uring-spin-0

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1179936 | 48.37 | 91.34 | 97.57 | 110.55 | 265.65 | 400.7 | 0.0 | 0.0 | 0.0 | 207.96 | 18.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1205794 | 48.38 | 55.36 | 62.99 | 204.47 | 1035.10 | 379.3 | 1.0 | 1.0 | 0.0 | 31.45 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 1215757 | 47.72 | 56.07 | 63.76 | 82.84 | 197.39 | 78.3 | 0.0 | 0.0 | 0.0 | 425.43 | 5.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1212479 | 48.33 (−0.04) | 54.20 (−1.86) | 61.25 (−2.50) | 74.26 (−36.29) | 183.16 (−82.50) | 19.3 | 0.0 | 0.0 | 0.0 | 70.66 | 43.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1236896 | 46.79 (−1.58) | 55.80 (−0.27) | 64.76 (+1.01) | 80.32 (−30.23) | 222.18 (−43.47) | 25.4 | 0.0 | 0.0 | 0.0 | 30.16 | 3.0 | server: softirq TIMER |
| boot-full-nomitig | 1210615 | 48.16 (−0.21) | 57.35 (+1.28) | 64.34 (+0.58) | 77.74 (−32.81) | 123.37 (−142.28) | 21.4 | 0.0 | 0.0 | 0.0 | 28.90 | 3.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1162806 | 49.05 (+0.67) | 60.42 (+4.36) | 68.04 (+4.29) | 95.47 (−15.08) | 170.98 (−94.67) | 108.8 | 0.0 | 0.0 | 0.0 | 39.89 | 4.0 | neither loop stalled long enough: network, hypervisor or untraced |

## WebSocket round trip, pp-uring-spin-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189998 | 46.73 | 94.27 | 266.90 | 320.74 | 431.81 | 4697.2 | 0.0 | 0.0 | 0.0 | 35.45 | 6.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1189998 | 45.43 | 50.68 | 57.99 | 120.19 | 981.86 | 68.6 | 0.5 | 0.0 | 0.0 | 289.78 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 1189998 | 47.97 | 53.00 | 63.35 | 91.73 | 971.95 | 24.7 | 0.5 | 0.0 | 0.0 | 40.25 | 1.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1189999 | 43.40 (−3.33) | 54.00 (+1.00) | 56.75 (−6.60) | 70.67 (−49.52) | 281.77 (−690.18) | 7.1 | 0.0 | 0.0 | 0.0 | 239.95 | 34.0 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-isolcpus | 1189998 | 65.94 (+19.21) | 91.72 (+38.72) | 96.22 (+32.87) | 145.34 (+25.15) | 369.03 (−602.92) | 229.4 | 0.0 | 0.0 | 0.0 | 58.43 | 2.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1189999 | 48.92 (+2.19) | 53.78 (+0.77) | 60.48 (−2.87) | 121.27 (+1.08) | 188.07 (−783.88) | 68.6 | 0.0 | 0.0 | 0.0 | 150.30 | 2.5 | server: softirq TIMER |
| boot-full | 1189998 | 49.16 (+2.43) | 54.50 (+1.50) | 61.36 (−2.00) | 130.46 (+10.27) | 403.45 (−568.50) | 75.1 | 0.0 | 0.0 | 0.0 | 396.87 | 2.5 | client: softirq TIMER |

### WebSocket round trip, pp-uring-spin-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189998 | 46.83 | 1042.22 | 4493.69 | 11646.85 | 14730.42 | 16388.6 | 11170.1 | 6303.5 | 113.9 |
| r1-base-10 | 1189998 | 45.50 | 50.76 | 59.08 | 125.86 | 981.93 | 84.2 | 4.5 | 0.0 | 0.0 |
| r1-base-12 | 1189998 | 48.04 | 53.08 | 63.54 | 94.46 | 972.03 | 35.8 | 5.0 | 0.0 | 0.0 |
| boot-stock | 1189999 | 43.48 (−3.36) | 54.08 (+1.00) | 56.85 (−6.69) | 70.94 (−54.91) | 281.86 (−700.07) | 8.6 | 0.0 | 0.0 | 0.0 |
| boot-isolcpus | 1189998 | 66.03 (+19.20) | 91.84 (+38.77) | 96.69 (+33.15) | 148.88 (+23.03) | 369.10 (−612.82) | 318.7 | 0.0 | 0.0 | 0.0 |
| boot-full-nomitig | 1189999 | 48.98 (+2.15) | 53.86 (+0.78) | 61.14 (−2.40) | 122.17 (−3.68) | 188.14 (−793.79) | 72.6 | 0.0 | 0.0 | 0.0 |
| boot-full | 1189998 | 49.22 (+2.39) | 54.60 (+1.52) | 62.40 (−1.14) | 133.12 (+7.26) | 403.51 (−578.42) | 101.8 | 0.0 | 0.0 | 0.0 |

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

## WebSocket round trip, pp-uring-sqpoll-10000

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189997 | 41.73 | 48.36 | 57.20 | 69.36 | 205.63 | 11.1 | 0.0 | 0.0 | 0.0 | 374.03 | 2.0 | server: softirq TIMER |
| r1-base-10 | 1189998 | 42.92 | 50.56 | 58.38 | 90.56 | 158.04 | 47.4 | 0.0 | 0.0 | 0.0 | 35.81 | 1.0 | neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 1189997 | 41.98 | 47.27 | 57.08 | 66.77 | 127.09 | 9.1 | 0.0 | 0.0 | 0.0 | 31.28 | 0.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1189997 | 40.72 (−1.26) | 50.79 (+2.43) | 54.51 (−2.70) | 68.52 (−0.84) | 1625.01 (+1466.98) | 8.6 | 0.5 | 0.5 | 0.0 | 52.09 | 34.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 1189998 | 40.82 (−1.17) | 49.21 (+0.85) | 54.56 (−2.64) | 78.82 (+9.46) | 168.66 (+10.62) | 18.7 | 0.0 | 0.0 | 0.0 | 44.78 | 1.5 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1189998 | 42.10 (+0.12) | 47.24 (−1.12) | 57.97 (+0.76) | 77.69 (+8.32) | 113343.70 (+113185.67) | 40.8 | 7.6 | 6.1 | 1.5 | 137458.57 | 75.0 | neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1189998 | 43.30 (+1.31) | 47.91 (−0.45) | 53.66 (−3.55) | 67.45 (−1.91) | 158.94 (+0.91) | 8.6 | 0.0 | 0.0 | 0.0 | 27.19 | 1.0 | neither loop stalled long enough: network, hypervisor or untraced |

### WebSocket round trip, pp-uring-sqpoll-10000, from when each was due

The same round trips timed from their slot on the paced schedule, which never slips: a stall delays every message due during it, and this includes that wait.

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min |
|---|---|---|---|---|---|---|---|---|---|---|
| r1-base-0 | 1189997 | 41.81 | 48.45 | 57.32 | 69.98 | 391.77 | 14.6 | 0.0 | 0.0 | 0.0 |
| r1-base-10 | 1189998 | 43.00 | 50.65 | 58.79 | 92.42 | 158.07 | 47.4 | 0.0 | 0.0 | 0.0 |
| r1-base-12 | 1189997 | 42.06 | 47.35 | 57.19 | 66.87 | 127.22 | 9.1 | 0.0 | 0.0 | 0.0 |
| boot-stock | 1189997 | 40.84 (−1.22) | 50.91 (+2.47) | 54.70 (−2.62) | 71.08 (+1.10) | 1625.10 (+1467.03) | 21.2 | 10.1 | 5.5 | 0.0 |
| boot-isolcpus | 1189998 | 40.92 (−1.14) | 49.31 (+0.87) | 54.69 (−2.63) | 80.13 (+10.15) | 168.74 (+10.66) | 19.2 | 0.0 | 0.0 | 0.0 |
| boot-full-nomitig | 1189998 | 42.19 (+0.13) | 51.30 (+2.85) | 91277.54 (+91220.22) | 130932.24 (+130862.27) | 137595.77 (+137437.70) | 3135.6 | 2976.3 | 2913.8 | 2134.8 |
| boot-full | 1189998 | 43.37 (+1.31) | 47.99 (−0.46) | 53.79 (−3.53) | 67.91 (−2.07) | 159.09 (+1.01) | 9.1 | 0.0 | 0.0 | 0.0 |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs, and system-management interrupts (SMI, bare metal only: firmware pauses). The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | SMI | receiving CPU | LOC | IWI | RES | CAL | SMI |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| http-recv-busy-10000 | r1-base-0 | 3 | 29 | 28 | 0 | 8 | 0 |  |  |  |  |  |  |
| http-recv-busy-10000 | r1-base-10 | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| http-recv-busy-10000 | r1-base-12 | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| http-recv-busy-10000 | boot-stock | 3 | 1004 | 0 | 0 | 4 | 0 |  |  |  |  |  |  |
| http-recv-busy-10000 | boot-isolcpus | 3 | 1001 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| http-recv-busy-10000 | boot-full-nomitig | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| http-recv-busy-10000 | boot-full | 3 | 3 | 3 | 0 | 9 | 0 |  |  |  |  |  |  |
| md-recv-busy | r1-base-0 | 3 | 17 | 17 | 0 | 17 | 0 |  |  |  |  |  |  |
| md-recv-busy | r1-base-2 | 3 | 6 | 6 | 0 | 17 | 0 |  |  |  |  |  |  |
| md-recv-busy | r1-base-10 | 3 | 2 | 2 | 0 | 16 | 0 |  |  |  |  |  |  |
| md-recv-busy | r1-base-12 | 3 | 9 | 9 | 0 | 17 | 0 |  |  |  |  |  |  |
| md-recv-busy | boot-stock | 3 | 1006 | 0 | 0 | 9 | 0 |  |  |  |  |  |  |
| md-recv-busy | boot-isolcpus | 3 | 1002 | 0 | 0 | 16 | 0 |  |  |  |  |  |  |
| md-recv-busy | boot-full-nomitig | 3 | 10 | 10 | 0 | 16 | 0 |  |  |  |  |  |  |
| md-recv-busy | boot-full | 3 | 14 | 14 | 0 | 17 | 0 |  |  |  |  |  |  |
| noise | r1-base-0 | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-housekeeping | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-prefer | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-base-2 | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-noquickack | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-nogro | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-base-4 | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-nocoalesce | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-irqlocal | 3 | 1 | 3 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-base-6 | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-rt | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-sibling | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-base-8 | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-busypoll50 | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-budget64 | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-base-10 | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-cstate | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-noagent | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | r1-base-12 | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | boot-stock | 3 | 1002 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | boot-isolcpus | 3 | 1001 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | boot-full-nomitig | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| noise | boot-full | 3 | 0 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-epoll-spin-0 | r1-base-0 | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1140 | 0 | 1 | 24 | 0 |
| pp-epoll-spin-0 | r1-base-2 | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1140 | 0 | 1 | 22 | 0 |
| pp-epoll-spin-0 | r1-base-10 | 3 | 0 | 0 | 0 | 16 | 0 | 4 | 1012 | 0 | 1 | 21 | 0 |
| pp-epoll-spin-0 | r1-base-12 | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 1 | 13 | 0 |
| pp-epoll-spin-0 | boot-stock | 3 | 1016 | 0 | 0 | 17 | 0 | 0 | 1005 | 0 | 0 | 21 | 0 |
| pp-epoll-spin-0 | boot-isolcpus | 3 | 1004 | 0 | 0 | 16 | 0 | 4 | 1013 | 0 | 1 | 12 | 0 |
| pp-epoll-spin-0 | boot-full-nomitig | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 0 | 21 | 0 |
| pp-epoll-spin-0 | boot-full | 3 | 0 | 0 | 0 | 16 | 0 | 0 | 1141 | 0 | 1 | 6 | 0 |
| pp-epoll-spin-10000 | r1-base-0 | 3 | 0 | 0 | 0 | 8 | 0 | 4 | 1006 | 0 | 0 | 3 | 0 |
| pp-epoll-spin-10000 | r1-base-2 | 3 | 0 | 0 | 0 | 8 | 0 | 4 | 1006 | 0 | 1 | 11 | 0 |
| pp-epoll-spin-10000 | r1-base-10 | 3 | 0 | 0 | 0 | 8 | 0 | 4 | 111 | 0 | 0 | 6 | 0 |
| pp-epoll-spin-10000 | r1-base-12 | 3 | 0 | 0 | 0 | 8 | 0 | 0 | 1072 | 0 | 0 | 5 | 0 |
| pp-epoll-spin-10000 | boot-stock | 3 | 1003 | 0 | 0 | 8 | 0 | 5 | 803 | 0 | 0 | 9 | 0 |
| pp-epoll-spin-10000 | boot-isolcpus | 3 | 1001 | 0 | 0 | 8 | 0 | 0 | 955 | 0 | 0 | 3 | 0 |
| pp-epoll-spin-10000 | boot-full-nomitig | 3 | 0 | 0 | 0 | 8 | 0 | 4 | 1005 | 0 | 0 | 7 | 0 |
| pp-epoll-spin-10000 | boot-full | 3 | 0 | 0 | 0 | 8 | 0 | 4 | 506 | 0 | 1 | 10 | 0 |
| pp-recv-busy-0 | r1-base-0 | 3 | 41 | 39 | 0 | 16 | 0 |  |  |  |  |  |  |
| pp-recv-busy-0 | r1-base-2 | 3 | 42 | 41 | 0 | 16 | 0 | 4 | 1015 | 0 | 1 | 22 | 0 |
| pp-recv-busy-0 | r1-base-10 | 3 | 33 | 33 | 0 | 16 | 0 | 0 | 1143 | 0 | 0 | 11 | 0 |
| pp-recv-busy-0 | r1-base-12 | 3 | 38 | 39 | 0 | 16 | 0 | 4 | 1011 | 0 | 1 | 4 | 0 |
| pp-recv-busy-0 | boot-stock | 3 | 1008 | 0 | 0 | 9 | 0 |  |  |  |  |  |  |
| pp-recv-busy-0 | boot-isolcpus | 3 | 1004 | 0 | 0 | 17 | 0 | 0 | 1016 | 0 | 1 | 11 | 0 |
| pp-recv-busy-0 | boot-full-nomitig | 3 | 39 | 38 | 0 | 16 | 0 | 4 | 1011 | 0 | 1 | 5 | 0 |
| pp-recv-busy-0 | boot-full | 3 | 20 | 20 | 0 | 16 | 0 | 0 | 1143 | 0 | 1 | 13 | 0 |
| pp-recv-busy-10000 | r1-base-0 | 3 | 30 | 29 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-recv-busy-10000 | r1-base-2 | 3 | 32 | 31 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-recv-busy-10000 | r1-base-10 | 3 | 18 | 18 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-recv-busy-10000 | r1-base-12 | 3 | 18 | 18 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-recv-busy-10000 | boot-stock | 3 | 1016 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-recv-busy-10000 | boot-isolcpus | 3 | 1001 | 0 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-recv-busy-10000 | boot-full-nomitig | 3 | 17 | 17 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-recv-busy-10000 | boot-full | 3 | 15 | 15 | 0 | 8 | 0 |  |  |  |  |  |  |
| pp-uring-spin-0 | r1-base-0 | 3 | 0 | 0 | 0 | 17 | 0 | 0 | 1139 | 0 | 1 | 5 | 0 |
| pp-uring-spin-0 | r1-base-10 | 3 | 0 | 0 | 0 | 17 | 0 | 4 | 1012 | 0 | 1 | 15 | 0 |
| pp-uring-spin-0 | r1-base-12 | 3 | 0 | 0 | 0 | 17 | 0 | 0 | 1141 | 0 | 1 | 11 | 0 |
| pp-uring-spin-0 | boot-stock | 3 | 1006 | 0 | 0 | 17 | 0 | 4 | 1009 | 0 | 0 | 10 | 0 |
| pp-uring-spin-0 | boot-isolcpus | 3 | 1004 | 0 | 0 | 17 | 0 | 4 | 1017 | 0 | 1 | 4 | 0 |
| pp-uring-spin-0 | boot-full-nomitig | 3 | 0 | 0 | 0 | 16 | 0 | 4 | 1015 | 0 | 1 | 23 | 0 |
| pp-uring-spin-0 | boot-full | 3 | 0 | 0 | 0 | 17 | 0 | 4 | 1012 | 0 | 1 | 14 | 0 |
| pp-uring-spin-10000 | r1-base-0 | 3 | 0 | 0 | 0 | 8 | 0 | 4 | 987 | 0 | 1 | 13 | 0 |
| pp-uring-spin-10000 | r1-base-10 | 3 | 0 | 0 | 0 | 8 | 0 | 0 | 1072 | 0 | 0 | 10 | 0 |
| pp-uring-spin-10000 | r1-base-12 | 3 | 0 | 0 | 0 | 8 | 0 | 0 | 1072 | 0 | 0 | 12 | 0 |
| pp-uring-spin-10000 | boot-stock | 3 | 1004 | 0 | 0 | 4 | 0 |  |  |  |  |  |  |
| pp-uring-spin-10000 | boot-isolcpus | 3 | 1001 | 0 | 0 | 8 | 0 | 0 | 1012 | 0 | 1 | 7 | 0 |
| pp-uring-spin-10000 | boot-full-nomitig | 3 | 0 | 0 | 0 | 8 | 0 | 0 | 1072 | 0 | 0 | 10 | 0 |
| pp-uring-spin-10000 | boot-full | 3 | 0 | 0 | 0 | 8 | 0 | 0 | 1072 | 0 | 1 | 8 | 0 |
| pp-uring-sqpoll-0 | r1-base-0 | 3 | 0 | 0 | 0 | 17 | 0 | 1 | 89 | 42 | 0 | 17 | 0 |
| pp-uring-sqpoll-0 | r1-base-10 | 3 | 0 | 0 | 0 | 17 | 0 | 1 | 11 | 3 | 3 | 17 | 0 |
| pp-uring-sqpoll-0 | r1-base-12 | 3 | 0 | 0 | 0 | 17 | 0 | 1 | 35 | 18 | 1 | 17 | 0 |
| pp-uring-sqpoll-0 | boot-stock | 3 | 1007 | 0 | 0 | 17 | 0 | 1 | 1039 | 0 | 0 | 17 | 0 |
| pp-uring-sqpoll-0 | boot-isolcpus | 3 | 1004 | 0 | 0 | 17 | 0 | 1 | 1003 | 0 | 0 | 17 | 0 |
| pp-uring-sqpoll-0 | boot-full-nomitig | 3 | 0 | 0 | 0 | 16 | 0 | 1 | 77 | 36 | 1 | 17 | 0 |
| pp-uring-sqpoll-0 | boot-full | 3 | 0 | 0 | 0 | 17 | 0 | 1 | 78 | 36 | 1 | 17 | 0 |
| pp-uring-sqpoll-10000 | r1-base-0 | 3 | 0 | 0 | 0 | 8 | 0 | 1 | 39 | 18 | 1 | 8 | 0 |
| pp-uring-sqpoll-10000 | r1-base-10 | 3 | 0 | 0 | 0 | 8 | 0 | 1 | 34 | 15 | 1 | 8 | 0 |
| pp-uring-sqpoll-10000 | r1-base-12 | 3 | 0 | 0 | 0 | 8 | 0 | 1 | 32 | 14 | 1 | 8 | 0 |
| pp-uring-sqpoll-10000 | boot-stock | 3 | 1004 | 0 | 0 | 4 | 0 | 1 | 1010 | 0 | 0 | 9 | 0 |
| pp-uring-sqpoll-10000 | boot-isolcpus | 3 | 1001 | 0 | 0 | 8 | 0 | 1 | 1002 | 0 | 0 | 8 | 0 |
| pp-uring-sqpoll-10000 | boot-full-nomitig | 3 | 0 | 0 | 0 | 8 | 0 | 1 | 39 | 18 | 1 | 8 | 0 |
| pp-uring-sqpoll-10000 | boot-full | 3 | 0 | 0 | 0 | 8 | 0 | 1 | 12 | 3 | 3 | 8 | 0 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### http-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 697.59 at 30.9 s: neither loop stalled long enough: network, hypervisor or untraced; 579.83 at 31.2 s: neither loop stalled long enough: network, hypervisor or untraced; 548.94 at 30.8 s: neither loop stalled long enough: network, hypervisor or untraced; 533.43 at 30.6 s: neither loop stalled long enough: network, hypervisor or untraced; 521.06 at 30.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 183.91 at 34.4 s: neither loop stalled long enough: network, hypervisor or untraced; 177.18 at 33.9 s: neither loop stalled long enough: network, hypervisor or untraced; 176.63 at 34.0 s: neither loop stalled long enough: network, hypervisor or untraced; 176.16 at 32.8 s: neither loop stalled long enough: network, hypervisor or untraced; 175.82 at 34.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 257.01 at 66.7 s: neither loop stalled long enough: network, hypervisor or untraced; 185.30 at 10.3 s: neither loop stalled long enough: network, hypervisor or untraced; 178.50 at 104.3 s: neither loop stalled long enough: network, hypervisor or untraced; 172.79 at 16.3 s: neither loop stalled long enough: network, hypervisor or untraced; 169.24 at 19.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 132.54 at 119.8 s: neither loop stalled long enough: network, hypervisor or untraced; 131.69 at 119.2 s: neither loop stalled long enough: network, hypervisor or untraced; 128.82 at 105.6 s: neither loop stalled long enough: network, hypervisor or untraced; 127.15 at 17.3 s: neither loop stalled long enough: network, hypervisor or untraced; 124.88 at 25.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 225.76 at 27.3 s: server: softirq TIMER; 141.71 at 2.7 s: neither loop stalled long enough: network, hypervisor or untraced; 137.85 at 65.6 s: neither loop stalled long enough: network, hypervisor or untraced; 136.80 at 70.9 s: neither loop stalled long enough: network, hypervisor or untraced; 136.08 at 4.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 592.59 at 71.4 s: neither loop stalled long enough: network, hypervisor or untraced; 134.56 at 62.8 s: neither loop stalled long enough: network, hypervisor or untraced; 129.66 at 43.8 s: neither loop stalled long enough: network, hypervisor or untraced; 126.40 at 54.6 s: client: mostly unexplained (+preempted by kworker/3:1); 124.87 at 34.4 s: server: mostly unexplained (+softirq NET_RX) |
| boot-full | 376.54 at 29.6 s: neither loop stalled long enough: network, hypervisor or untraced; 127.94 at 51.9 s: client: mostly unexplained (+preempted by kworker/3:1); 127.93 at 59.9 s: client: mostly unexplained (+preempted by kworker/3:1); 127.60 at 55.9 s: client: mostly unexplained (+preempted by kworker/3:1); 127.52 at 31.9 s: client: mostly unexplained (+preempted by kworker/3:1) |

### md-recv-busy

| condition | worst events |
|---|---|
| r1-base-0 | 40.28 at 46.2 s: client: loop work (33 items); 40.15 at 46.2 s: client: loop work (33 items); 40.01 at 46.2 s: client: loop work (33 items); 39.88 at 46.2 s: client: loop work (33 items); 39.74 at 46.2 s: client: loop work (33 items) |
| r1-base-2 | 38.64 at 3.4 s: client: loop work (24 items); 38.46 at 3.4 s: client: loop work (24 items); 38.29 at 3.4 s: client: loop work (24 items); 38.12 at 3.4 s: client: loop work (24 items); 37.95 at 3.4 s: client: loop work (24 items) |
| r1-base-10 | 32.46 at 29.0 s: client: mostly unexplained (+softirq NET_RX); 32.25 at 10.8 s: client: softirq NET_RX; 32.11 at 10.8 s: client: softirq NET_RX; 31.97 at 10.8 s: client: softirq NET_RX; 31.82 at 10.8 s: client: softirq NET_RX |
| r1-base-12 | 38.66 at 57.9 s: client: loop work (18 items); 38.49 at 57.9 s: client: loop work (18 items); 38.31 at 57.9 s: client: loop work (18 items); 38.16 at 57.9 s: client: loop work (18 items); 38.00 at 57.9 s: client: loop work (18 items) |
| boot-stock | 49.34 at 51.7 s: client: loop work (6 items); 49.14 at 51.7 s: client: loop work (6 items); 48.96 at 51.7 s: client: loop work (6 items); 48.77 at 51.7 s: client: loop work (6 items); 48.55 at 51.7 s: client: loop work (6 items) |
| boot-isolcpus | 29.51 at 20.6 s: neither loop stalled long enough: network, hypervisor or untraced; 29.37 at 20.6 s: neither loop stalled long enough: network, hypervisor or untraced; 29.30 at 23.6 s: neither loop stalled long enough: network, hypervisor or untraced; 29.23 at 20.6 s: neither loop stalled long enough: network, hypervisor or untraced; 29.14 at 23.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 39.09 at 28.2 s: client: loop work (18 items); 38.95 at 28.2 s: client: loop work (18 items); 38.81 at 28.2 s: client: loop work (18 items); 38.66 at 28.2 s: client: loop work (18 items); 38.52 at 28.2 s: client: loop work (18 items) |
| boot-full | 43.66 at 31.0 s: client: loop work (18 items); 43.49 at 31.0 s: client: loop work (18 items); 43.31 at 31.0 s: client: loop work (18 items); 43.14 at 31.0 s: client: loop work (18 items); 42.98 at 31.0 s: client: loop work (18 items) |

### noise

| condition | worst events |
|---|---|
| r1-base-0 | 242.19 at 99.7 s: softirq TIMER; 32.93 at 6.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.39 at 66.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.16 at 0.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 13.26 at 94.6 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-housekeeping | 346.00 at 108.7 s: softirq TIMER; 32.60 at 24.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.50 at 85.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.72 at 2.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 13.38 at 36.4 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-prefer | 214.07 at 8.0 s: softirq TIMER; 32.94 at 22.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.21 at 81.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.28 at 7.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 13.20 at 2.9 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-2 | 32.42 at 22.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.82 at 82.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.55 at 7.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 11.80 at 103.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 10.68 at 23.4 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-noquickack | 32.45 at 32.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 21.88 at 54.8 s: mostly unexplained (+local timer tick); 20.91 at 26.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.78 at 92.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.07 at 2.3 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-nogro | 34.72 at 25.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 18.36 at 85.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.38 at 38.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.88 at 2.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 11.49 at 0.0 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-4 | 116.64 at 106.6 s: softirq TIMER; 32.14 at 28.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.66 at 89.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.35 at 5.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 14.63 at 0.0 s: local timer tick |
| r1-nocoalesce | 73.83 at 114.3 s: softirq TIMER; 31.84 at 47.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 20.16 at 49.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.79 at 107.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.09 at 6.4 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-irqlocal | 43.42 at 72.5 s: softirq NET_RX; 43.18 at 61.9 s: mostly unexplained (+softirq NET_RX); 43.14 at 3.1 s: softirq NET_RX; 40.06 at 0.1 s: softirq NET_RX; 38.45 at 53.2 s: softirq NET_RX |
| r1-base-6 | 5060.68 at 25.0 s: mostly unexplained (+softirq NET_RX); 3909.43 at 33.7 s: mostly unexplained (+local timer tick); 30.09 at 43.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 25.99 at 91.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 23.24 at 25.0 s: local timer tick |
| r1-rt | 36.32 at 0.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 19.12 at 61.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 14.13 at 4.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 10.84 at 18.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 10.09 at 0.0 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-sibling | 30.36 at 12.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 19.11 at 72.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 13.56 at 7.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 12.65 at 0.0 s: mostly unexplained (+local timer tick); 11.26 at 102.8 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-8 | 28.24 at 16.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.36 at 2.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 15.05 at 76.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 13.23 at 16.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 11.02 at 3.1 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-busypoll50 | 26.82 at 35.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 21.40 at 6.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.79 at 5.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 15.67 at 95.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 10.36 at 21.0 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-budget64 | 29.27 at 33.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.23 at 3.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 14.16 at 93.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 10.41 at 97.5 s: unexplained: no kernel event (hypervisor or in-kernel time); 10.21 at 2.3 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-10 | 256.54 at 6.2 s: softirq TIMER; 28.88 at 23.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.64 at 1.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 15.25 at 83.3 s: unexplained: no kernel event (hypervisor or in-kernel time); 10.22 at 0.2 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-cstate | 372.99 at 13.8 s: softirq TIMER; 30.85 at 40.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 23.92 at 9.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 15.06 at 100.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 14.07 at 2.6 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-noagent | 169.84 at 10.0 s: softirq TIMER; 28.52 at 46.6 s: unexplained: no kernel event (hypervisor or in-kernel time); 21.61 at 47.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.88 at 0.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 15.22 at 107.1 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| r1-base-12 | 24.90 at 96.8 s: mostly unexplained (+local timer tick); 24.56 at 33.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.32 at 3.2 s: unexplained: no kernel event (hypervisor or in-kernel time); 14.61 at 93.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 10.68 at 0.5 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-stock | 150.29 at 62.1 s: not traced; 148.55 at 112.1 s: not traced; 148.41 at 56.1 s: not traced; 143.12 at 13.1 s: not traced; 141.27 at 44.1 s: not traced |
| boot-isolcpus | 29.53 at 12.9 s: not traced; 16.40 at 72.9 s: not traced; 16.16 at 2.6 s: not traced; 15.71 at 2.6 s: not traced; 12.23 at 8.0 s: not traced |
| boot-full-nomitig | 26.59 at 10.4 s: unexplained: no kernel event (hypervisor or in-kernel time); 17.64 at 2.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 14.52 at 70.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 12.63 at 2.7 s: unexplained: no kernel event (hypervisor or in-kernel time); 10.03 at 0.0 s: unexplained: no kernel event (hypervisor or in-kernel time) |
| boot-full | 30.48 at 11.1 s: unexplained: no kernel event (hypervisor or in-kernel time); 16.55 at 2.8 s: unexplained: no kernel event (hypervisor or in-kernel time); 14.44 at 71.9 s: unexplained: no kernel event (hypervisor or in-kernel time); 9.53 at 0.0 s: unexplained: no kernel event (hypervisor or in-kernel time); 8.85 at 38.0 s: unexplained: no kernel event (hypervisor or in-kernel time) |

### pp-epoll-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 343.74 at 48.0 s: neither loop stalled long enough: network, hypervisor or untraced; 211.31 at 8.0 s: neither loop stalled long enough: network, hypervisor or untraced; 208.30 at 4.1 s: neither loop stalled long enough: network, hypervisor or untraced; 196.24 at 6.1 s: neither loop stalled long enough: network, hypervisor or untraced; 196.20 at 26.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 258.29 at 26.6 s: neither loop stalled long enough: network, hypervisor or untraced; 225.57 at 33.7 s: neither loop stalled long enough: network, hypervisor or untraced; 203.54 at 47.0 s: neither loop stalled long enough: network, hypervisor or untraced; 198.37 at 16.0 s: neither loop stalled long enough: network, hypervisor or untraced; 196.84 at 11.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 368.77 at 19.0 s: client: softirq TIMER; 366.21 at 15.0 s: neither loop stalled long enough: network, hypervisor or untraced; 341.03 at 15.0 s: neither loop stalled long enough: network, hypervisor or untraced; 334.75 at 33.0 s: neither loop stalled long enough: network, hypervisor or untraced; 328.55 at 33.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 187.80 at 10.3 s: neither loop stalled long enough: network, hypervisor or untraced; 185.93 at 18.7 s: neither loop stalled long enough: network, hypervisor or untraced; 182.30 at 44.3 s: neither loop stalled long enough: network, hypervisor or untraced; 181.83 at 32.3 s: neither loop stalled long enough: network, hypervisor or untraced; 181.73 at 41.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 239.55 at 12.0 s: neither loop stalled long enough: network, hypervisor or untraced; 226.25 at 18.1 s: neither loop stalled long enough: network, hypervisor or untraced; 193.70 at 47.6 s: neither loop stalled long enough: network, hypervisor or untraced; 191.90 at 56.0 s: neither loop stalled long enough: network, hypervisor or untraced; 184.80 at 47.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 389.90 at 41.8 s: neither loop stalled long enough: network, hypervisor or untraced; 150.41 at 9.8 s: neither loop stalled long enough: network, hypervisor or untraced; 140.65 at 1.1 s: neither loop stalled long enough: network, hypervisor or untraced; 134.83 at 34.8 s: neither loop stalled long enough: network, hypervisor or untraced; 134.39 at 30.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 212.53 at 46.8 s: neither loop stalled long enough: network, hypervisor or untraced; 192.14 at 40.7 s: neither loop stalled long enough: network, hypervisor or untraced; 184.03 at 56.7 s: neither loop stalled long enough: network, hypervisor or untraced; 182.45 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 180.57 at 34.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1057.36 at 12.7 s: neither loop stalled long enough: network, hypervisor or untraced; 220.84 at 37.7 s: neither loop stalled long enough: network, hypervisor or untraced; 206.02 at 45.7 s: neither loop stalled long enough: network, hypervisor or untraced; 205.75 at 29.7 s: neither loop stalled long enough: network, hypervisor or untraced; 183.76 at 9.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-epoll-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 402.69 at 34.2 s: neither loop stalled long enough: network, hypervisor or untraced; 400.84 at 39.5 s: neither loop stalled long enough: network, hypervisor or untraced; 356.44 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 353.09 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 343.32 at 64.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 389.61 at 3.9 s: neither loop stalled long enough: network, hypervisor or untraced; 374.66 at 3.9 s: neither loop stalled long enough: network, hypervisor or untraced; 371.73 at 100.6 s: neither loop stalled long enough: network, hypervisor or untraced; 362.93 at 69.0 s: neither loop stalled long enough: network, hypervisor or untraced; 357.39 at 68.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 127.14 at 115.5 s: neither loop stalled long enough: network, hypervisor or untraced; 124.11 at 89.0 s: neither loop stalled long enough: network, hypervisor or untraced; 122.94 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced; 120.55 at 37.3 s: neither loop stalled long enough: network, hypervisor or untraced; 119.37 at 73.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 982.30 at 29.8 s: neither loop stalled long enough: network, hypervisor or untraced; 259.67 at 45.7 s: client: softirq TIMER; 173.91 at 92.8 s: neither loop stalled long enough: network, hypervisor or untraced; 167.34 at 50.3 s: server: softirq TIMER; 167.20 at 36.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 4651.53 at 97.5 s: neither loop stalled long enough: network, hypervisor or untraced; 446.71 at 47.0 s: neither loop stalled long enough: network, hypervisor or untraced; 445.91 at 88.4 s: neither loop stalled long enough: network, hypervisor or untraced; 445.15 at 34.3 s: neither loop stalled long enough: network, hypervisor or untraced; 443.88 at 15.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 404.94 at 68.8 s: neither loop stalled long enough: network, hypervisor or untraced; 375.17 at 54.1 s: neither loop stalled long enough: network, hypervisor or untraced; 346.74 at 68.7 s: neither loop stalled long enough: network, hypervisor or untraced; 339.90 at 16.1 s: neither loop stalled long enough: network, hypervisor or untraced; 338.55 at 68.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 425.08 at 99.6 s: neither loop stalled long enough: network, hypervisor or untraced; 395.49 at 118.4 s: neither loop stalled long enough: network, hypervisor or untraced; 387.69 at 99.6 s: neither loop stalled long enough: network, hypervisor or untraced; 352.09 at 99.6 s: neither loop stalled long enough: network, hypervisor or untraced; 349.50 at 99.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1051.33 at 112.1 s: neither loop stalled long enough: network, hypervisor or untraced; 211.79 at 17.8 s: neither loop stalled long enough: network, hypervisor or untraced; 197.44 at 95.0 s: neither loop stalled long enough: network, hypervisor or untraced; 195.75 at 79.3 s: neither loop stalled long enough: network, hypervisor or untraced; 143.43 at 68.7 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-0

| condition | worst events |
|---|---|
| r1-base-0 | 1038.29 at 55.9 s: neither loop stalled long enough: network, hypervisor or untraced; 168.72 at 33.9 s: neither loop stalled long enough: network, hypervisor or untraced; 166.60 at 15.9 s: neither loop stalled long enough: network, hypervisor or untraced; 164.64 at 31.9 s: neither loop stalled long enough: network, hypervisor or untraced; 161.85 at 4.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 129.50 at 47.6 s: neither loop stalled long enough: network, hypervisor or untraced; 126.40 at 11.1 s: neither loop stalled long enough: network, hypervisor or untraced; 125.96 at 37.1 s: neither loop stalled long enough: network, hypervisor or untraced; 124.39 at 9.6 s: neither loop stalled long enough: network, hypervisor or untraced; 122.11 at 24.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 221.01 at 53.7 s: client: softirq TIMER; 185.61 at 19.7 s: neither loop stalled long enough: network, hypervisor or untraced; 173.16 at 51.7 s: neither loop stalled long enough: network, hypervisor or untraced; 171.17 at 13.7 s: neither loop stalled long enough: network, hypervisor or untraced; 168.83 at 43.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 161.84 at 47.9 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 128.15 at 2.5 s: neither loop stalled long enough: network, hypervisor or untraced; 125.21 at 52.4 s: neither loop stalled long enough: network, hypervisor or untraced; 118.79 at 30.7 s: neither loop stalled long enough: network, hypervisor or untraced; 116.24 at 55.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 5225.32 at 13.2 s: neither loop stalled long enough: network, hypervisor or untraced; 203.46 at 9.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 186.97 at 25.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 178.15 at 51.7 s: client: softirq TIMER; 177.35 at 45.7 s: client: softirq TIMER |
| boot-isolcpus | 220.51 at 6.7 s: neither loop stalled long enough: network, hypervisor or untraced; 156.50 at 27.7 s: neither loop stalled long enough: network, hypervisor or untraced; 153.57 at 25.6 s: neither loop stalled long enough: network, hypervisor or untraced; 153.54 at 52.6 s: neither loop stalled long enough: network, hypervisor or untraced; 148.12 at 35.7 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 1031.97 at 43.1 s: neither loop stalled long enough: network, hypervisor or untraced; 257.23 at 10.2 s: client: softirq TIMER; 131.43 at 16.4 s: neither loop stalled long enough: network, hypervisor or untraced; 116.67 at 32.3 s: neither loop stalled long enough: network, hypervisor or untraced; 113.99 at 43.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 1029.52 at 5.1 s: neither loop stalled long enough: network, hypervisor or untraced; 261.68 at 56.6 s: neither loop stalled long enough: network, hypervisor or untraced; 228.34 at 8.0 s: client: softirq TIMER; 223.21 at 52.0 s: neither loop stalled long enough: network, hypervisor or untraced; 206.19 at 42.9 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-recv-busy-10000

| condition | worst events |
|---|---|
| r1-base-0 | 134.24 at 93.2 s: neither loop stalled long enough: network, hypervisor or untraced; 128.60 at 31.2 s: neither loop stalled long enough: network, hypervisor or untraced; 128.39 at 36.2 s: neither loop stalled long enough: network, hypervisor or untraced; 128.35 at 113.2 s: neither loop stalled long enough: network, hypervisor or untraced; 127.84 at 50.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-2 | 254.92 at 21.2 s: neither loop stalled long enough: network, hypervisor or untraced; 214.16 at 36.4 s: neither loop stalled long enough: network, hypervisor or untraced; 204.66 at 60.4 s: neither loop stalled long enough: network, hypervisor or untraced; 196.96 at 69.4 s: neither loop stalled long enough: network, hypervisor or untraced; 183.32 at 25.9 s: server: softirq TIMER |
| r1-base-10 | 128.12 at 105.5 s: neither loop stalled long enough: network, hypervisor or untraced; 115.36 at 83.4 s: neither loop stalled long enough: network, hypervisor or untraced; 113.14 at 66.1 s: neither loop stalled long enough: network, hypervisor or untraced; 111.86 at 27.5 s: neither loop stalled long enough: network, hypervisor or untraced; 111.23 at 29.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 159.90 at 16.7 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 159.43 at 23.5 s: neither loop stalled long enough: network, hypervisor or untraced; 153.62 at 76.7 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 137.86 at 99.9 s: server: softirq TIMER; 131.10 at 34.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 177.47 at 40.6 s: server: softirq TIMER; 118.23 at 29.8 s: neither loop stalled long enough: network, hypervisor or untraced; 112.03 at 77.0 s: neither loop stalled long enough: network, hypervisor or untraced; 111.36 at 112.6 s: neither loop stalled long enough: network, hypervisor or untraced; 110.96 at 4.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 128.82 at 105.5 s: neither loop stalled long enough: network, hypervisor or untraced; 122.88 at 118.9 s: neither loop stalled long enough: network, hypervisor or untraced; 122.16 at 8.2 s: neither loop stalled long enough: network, hypervisor or untraced; 118.13 at 5.5 s: neither loop stalled long enough: network, hypervisor or untraced; 115.58 at 47.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 972.63 at 71.9 s: neither loop stalled long enough: network, hypervisor or untraced; 269.75 at 36.8 s: neither loop stalled long enough: network, hypervisor or untraced; 216.46 at 18.7 s: neither loop stalled long enough: network, hypervisor or untraced; 127.28 at 96.3 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 118.46 at 54.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 163.93 at 7.5 s: neither loop stalled long enough: network, hypervisor or untraced; 163.79 at 95.5 s: neither loop stalled long enough: network, hypervisor or untraced; 163.01 at 25.5 s: neither loop stalled long enough: network, hypervisor or untraced; 161.53 at 62.5 s: neither loop stalled long enough: network, hypervisor or untraced; 159.42 at 76.5 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-spin-0

| condition | worst events |
|---|---|
| r1-base-0 | 265.65 at 36.9 s: neither loop stalled long enough: network, hypervisor or untraced; 240.71 at 16.3 s: client: softirq TIMER; 217.51 at 21.0 s: server: softirq TIMER; 210.01 at 25.0 s: neither loop stalled long enough: network, hypervisor or untraced; 209.50 at 33.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 1035.10 at 29.5 s: neither loop stalled long enough: network, hypervisor or untraced; 397.82 at 29.5 s: neither loop stalled long enough: network, hypervisor or untraced; 325.09 at 40.5 s: neither loop stalled long enough: network, hypervisor or untraced; 301.05 at 29.5 s: neither loop stalled long enough: network, hypervisor or untraced; 295.81 at 29.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 197.39 at 36.5 s: neither loop stalled long enough: network, hypervisor or untraced; 179.25 at 38.5 s: neither loop stalled long enough: network, hypervisor or untraced; 178.05 at 10.5 s: neither loop stalled long enough: network, hypervisor or untraced; 177.43 at 24.5 s: neither loop stalled long enough: network, hypervisor or untraced; 177.15 at 32.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 183.16 at 39.7 s: neither loop stalled long enough: network, hypervisor or untraced; 168.62 at 41.7 s: neither loop stalled long enough: network, hypervisor or untraced; 128.81 at 51.8 s: neither loop stalled long enough: network, hypervisor or untraced; 118.00 at 9.7 s: neither loop stalled long enough: network, hypervisor or untraced; 116.91 at 19.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 222.18 at 13.4 s: server: softirq TIMER; 156.71 at 1.9 s: neither loop stalled long enough: network, hypervisor or untraced; 147.54 at 14.9 s: neither loop stalled long enough: network, hypervisor or untraced; 132.78 at 31.9 s: neither loop stalled long enough: network, hypervisor or untraced; 125.87 at 48.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 123.37 at 26.6 s: neither loop stalled long enough: network, hypervisor or untraced; 122.12 at 45.9 s: neither loop stalled long enough: network, hypervisor or untraced; 120.38 at 3.4 s: neither loop stalled long enough: network, hypervisor or untraced; 117.22 at 13.7 s: neither loop stalled long enough: network, hypervisor or untraced; 115.56 at 11.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 170.98 at 28.7 s: neither loop stalled long enough: network, hypervisor or untraced; 170.37 at 15.7 s: neither loop stalled long enough: network, hypervisor or untraced; 169.24 at 51.7 s: neither loop stalled long enough: network, hypervisor or untraced; 165.97 at 47.7 s: neither loop stalled long enough: network, hypervisor or untraced; 164.92 at 17.7 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-spin-10000

| condition | worst events |
|---|---|
| r1-base-0 | 431.81 at 16.2 s: neither loop stalled long enough: network, hypervisor or untraced; 403.51 at 22.7 s: neither loop stalled long enough: network, hypervisor or untraced; 402.96 at 79.8 s: neither loop stalled long enough: network, hypervisor or untraced; 391.98 at 43.2 s: neither loop stalled long enough: network, hypervisor or untraced; 390.79 at 95.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 981.86 at 50.5 s: neither loop stalled long enough: network, hypervisor or untraced; 316.02 at 79.6 s: neither loop stalled long enough: network, hypervisor or untraced; 197.07 at 40.0 s: neither loop stalled long enough: network, hypervisor or untraced; 191.80 at 100.0 s: neither loop stalled long enough: network, hypervisor or untraced; 165.75 at 10.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 971.95 at 77.4 s: neither loop stalled long enough: network, hypervisor or untraced; 167.67 at 72.1 s: neither loop stalled long enough: network, hypervisor or untraced; 162.27 at 103.8 s: neither loop stalled long enough: network, hypervisor or untraced; 157.00 at 43.8 s: neither loop stalled long enough: network, hypervisor or untraced; 149.12 at 72.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 281.77 at 11.7 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 132.87 at 39.4 s: neither loop stalled long enough: network, hypervisor or untraced; 131.18 at 58.3 s: neither loop stalled long enough: network, hypervisor or untraced; 126.70 at 41.7 s: neither loop stalled long enough: network, hypervisor or untraced; 125.34 at 77.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 369.03 at 16.2 s: neither loop stalled long enough: network, hypervisor or untraced; 226.84 at 29.7 s: neither loop stalled long enough: network, hypervisor or untraced; 212.15 at 25.5 s: neither loop stalled long enough: network, hypervisor or untraced; 205.69 at 29.5 s: neither loop stalled long enough: network, hypervisor or untraced; 199.66 at 91.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 188.07 at 116.1 s: server: softirq TIMER; 160.14 at 21.8 s: client: softirq TIMER; 159.25 at 48.1 s: neither loop stalled long enough: network, hypervisor or untraced; 158.12 at 74.1 s: neither loop stalled long enough: network, hypervisor or untraced; 152.69 at 40.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 403.45 at 15.3 s: client: softirq TIMER; 221.36 at 101.6 s: neither loop stalled long enough: network, hypervisor or untraced; 200.46 at 111.6 s: neither loop stalled long enough: network, hypervisor or untraced; 198.06 at 38.6 s: neither loop stalled long enough: network, hypervisor or untraced; 197.80 at 109.6 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-0

| condition | worst events |
|---|---|
| r1-base-0 | 176.29 at 23.9 s: neither loop stalled long enough: network, hypervisor or untraced; 137.27 at 54.6 s: neither loop stalled long enough: network, hypervisor or untraced; 135.16 at 44.6 s: neither loop stalled long enough: network, hypervisor or untraced; 129.96 at 14.2 s: neither loop stalled long enough: network, hypervisor or untraced; 128.34 at 28.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 195.38 at 25.8 s: neither loop stalled long enough: network, hypervisor or untraced; 175.11 at 19.5 s: neither loop stalled long enough: network, hypervisor or untraced; 157.66 at 44.6 s: neither loop stalled long enough: network, hypervisor or untraced; 152.01 at 51.3 s: neither loop stalled long enough: network, hypervisor or untraced; 137.45 at 19.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 861.59 at 46.4 s: neither loop stalled long enough: network, hypervisor or untraced; 415.89 at 41.8 s: client: softirq TIMER; 191.89 at 46.4 s: server: softirq TIMER; 151.09 at 23.7 s: neither loop stalled long enough: network, hypervisor or untraced; 134.46 at 2.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 130.21 at 24.5 s: neither loop stalled long enough: network, hypervisor or untraced; 127.68 at 17.6 s: neither loop stalled long enough: network, hypervisor or untraced; 121.19 at 46.2 s: neither loop stalled long enough: network, hypervisor or untraced; 114.37 at 45.3 s: neither loop stalled long enough: network, hypervisor or untraced; 112.81 at 16.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 184.26 at 44.1 s: neither loop stalled long enough: network, hypervisor or untraced; 172.53 at 5.1 s: neither loop stalled long enough: network, hypervisor or untraced; 167.87 at 42.1 s: neither loop stalled long enough: network, hypervisor or untraced; 163.49 at 47.1 s: neither loop stalled long enough: network, hypervisor or untraced; 163.44 at 40.1 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 196.52 at 50.9 s: neither loop stalled long enough: network, hypervisor or untraced; 195.48 at 34.2 s: neither loop stalled long enough: network, hypervisor or untraced; 184.33 at 53.9 s: neither loop stalled long enough: network, hypervisor or untraced; 179.35 at 51.9 s: neither loop stalled long enough: network, hypervisor or untraced; 173.71 at 59.9 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 331.52 at 24.3 s: neither loop stalled long enough: network, hypervisor or untraced; 165.19 at 27.3 s: neither loop stalled long enough: network, hypervisor or untraced; 134.56 at 51.7 s: neither loop stalled long enough: network, hypervisor or untraced; 132.44 at 19.3 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 127.48 at 27.4 s: neither loop stalled long enough: network, hypervisor or untraced |

### pp-uring-sqpoll-10000

| condition | worst events |
|---|---|
| r1-base-0 | 205.63 at 75.3 s: server: softirq TIMER; 129.68 at 83.9 s: neither loop stalled long enough: network, hypervisor or untraced; 125.49 at 51.9 s: neither loop stalled long enough: network, hypervisor or untraced; 125.17 at 102.3 s: neither loop stalled long enough: network, hypervisor or untraced; 115.48 at 62.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-10 | 158.04 at 19.2 s: neither loop stalled long enough: network, hypervisor or untraced; 156.59 at 48.2 s: neither loop stalled long enough: network, hypervisor or untraced; 155.22 at 55.2 s: neither loop stalled long enough: network, hypervisor or untraced; 154.47 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced; 152.44 at 3.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| r1-base-12 | 127.09 at 113.1 s: neither loop stalled long enough: network, hypervisor or untraced; 127.05 at 32.5 s: neither loop stalled long enough: network, hypervisor or untraced; 121.10 at 22.7 s: neither loop stalled long enough: network, hypervisor or untraced; 120.16 at 19.9 s: server: unexplained: no kernel event (hypervisor or in-kernel time); 119.85 at 30.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-stock | 1625.01 at 4.4 s: neither loop stalled long enough: network, hypervisor or untraced; 142.00 at 68.2 s: neither loop stalled long enough: network, hypervisor or untraced; 130.37 at 116.6 s: neither loop stalled long enough: network, hypervisor or untraced; 125.77 at 77.1 s: neither loop stalled long enough: network, hypervisor or untraced; 118.81 at 71.2 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-isolcpus | 168.66 at 92.1 s: neither loop stalled long enough: network, hypervisor or untraced; 159.01 at 66.0 s: server: softirq TIMER; 153.73 at 51.3 s: neither loop stalled long enough: network, hypervisor or untraced; 132.47 at 5.2 s: neither loop stalled long enough: network, hypervisor or untraced; 119.26 at 37.3 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full-nomitig | 113343.70 at 12.4 s: neither loop stalled long enough: network, hypervisor or untraced; 15297.53 at 45.0 s: neither loop stalled long enough: network, hypervisor or untraced; 14789.34 at 45.5 s: neither loop stalled long enough: network, hypervisor or untraced; 9020.74 at 45.8 s: neither loop stalled long enough: network, hypervisor or untraced; 8159.23 at 44.6 s: neither loop stalled long enough: network, hypervisor or untraced |
| boot-full | 158.94 at 19.4 s: neither loop stalled long enough: network, hypervisor or untraced; 142.58 at 66.2 s: neither loop stalled long enough: network, hypervisor or untraced; 129.29 at 74.1 s: neither loop stalled long enough: network, hypervisor or untraced; 127.97 at 106.9 s: neither loop stalled long enough: network, hypervisor or untraced; 119.42 at 114.3 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| r1-base-0 | loop work 161283.25; mostly unexplained (+preempted by kworker/3:1) 93821.83; unexplained: no kernel event (hypervisor or in-kernel time) 75665.32; mostly unexplained (+softirq NET_RX) 36294.83; softirq NET_RX 16141.61 |
| r1-housekeeping | unexplained: no kernel event (hypervisor or in-kernel time) 698.56; softirq TIMER 346.00; mostly unexplained (+call function (IPI/vector)) 100.09 |
| r1-prefer | unexplained: no kernel event (hypervisor or in-kernel time) 620.66; softirq TIMER 214.07; mostly unexplained (+call function (IPI/vector)) 102.76 |
| r1-base-2 | loop work 117113.61; mostly unexplained (+preempted by kworker/3:1) 51936.12; mostly unexplained (+softirq NET_RX) 12494.00; softirq NET_RX 8915.04; unexplained: no kernel event (hypervisor or in-kernel time) 3252.47 |
| r1-noquickack | unexplained: no kernel event (hypervisor or in-kernel time) 583.24; mostly unexplained (+call function (IPI/vector)) 74.22; mostly unexplained (+local timer tick) 35.63; local timer tick 25.17 |
| r1-nogro | unexplained: no kernel event (hypervisor or in-kernel time) 621.39; mostly unexplained (+call function (IPI/vector)) 100.56 |
| r1-base-4 | unexplained: no kernel event (hypervisor or in-kernel time) 622.84; softirq TIMER 116.64; mostly unexplained (+call function (IPI/vector)) 87.03; local timer tick 25.84; mostly unexplained (+local timer tick) 15.66 |
| r1-nocoalesce | unexplained: no kernel event (hypervisor or in-kernel time) 577.58; mostly unexplained (+call function (IPI/vector)) 83.97; softirq TIMER 73.83; mostly unexplained (+local timer tick) 30.31 |
| r1-irqlocal | softirq NET_RX 4404.86; mostly unexplained (+softirq NET_RX) 1457.44; mostly unexplained (+local timer tick) 759.54; unexplained: no kernel event (hypervisor or in-kernel time) 634.51; local timer tick 489.06 |
| r1-base-6 | mostly unexplained (+softirq NET_RX) 5060.68; mostly unexplained (+local timer tick) 3985.93; unexplained: no kernel event (hypervisor or in-kernel time) 665.60; mostly unexplained (+call function (IPI/vector)) 54.09; mostly unexplained (+call function single (IPI/vector)) 35.71 |
| r1-rt | unexplained: no kernel event (hypervisor or in-kernel time) 582.08; mostly unexplained (+call function (IPI/vector)) 77.88 |
| r1-sibling | unexplained: no kernel event (hypervisor or in-kernel time) 566.85; mostly unexplained (+call function (IPI/vector)) 57.27; mostly unexplained (+local timer tick) 22.68; local timer tick 13.82 |
| r1-base-8 | unexplained: no kernel event (hypervisor or in-kernel time) 526.88; mostly unexplained (+call function (IPI/vector)) 81.64 |
| r1-busypoll50 | unexplained: no kernel event (hypervisor or in-kernel time) 567.41; mostly unexplained (+call function (IPI/vector)) 89.30 |
| r1-budget64 | unexplained: no kernel event (hypervisor or in-kernel time) 536.08; mostly unexplained (+call function (IPI/vector)) 46.11 |
| r1-base-10 | loop work 135221.48; unexplained: no kernel event (hypervisor or in-kernel time) 100039.07; mostly unexplained (+softirq NET_RX) 31323.81; mostly unexplained (+preempted by kworker/3:1) 24366.25; softirq NET_RX 4166.48 |
| r1-cstate | unexplained: no kernel event (hypervisor or in-kernel time) 570.33; softirq TIMER 372.99; mostly unexplained (+call function (IPI/vector)) 59.34 |
| r1-noagent | unexplained: no kernel event (hypervisor or in-kernel time) 564.36; softirq TIMER 169.84; mostly unexplained (+call function (IPI/vector)) 70.36 |
| r1-base-12 | loop work 141379.46; unexplained: no kernel event (hypervisor or in-kernel time) 92965.93; mostly unexplained (+preempted by kworker/3:1) 26137.37; mostly unexplained (+softirq NET_RX) 21412.92; softirq NET_RX 5557.15 |
| boot-stock | not traced 896542.23; loop work 148743.56; unexplained: no kernel event (hypervisor or in-kernel time) 144198.84; mostly unexplained (+softirq NET_RX) 11267.10; mostly unexplained (+local timer tick) 9586.73 |
| boot-isolcpus | not traced 864550.97; loop work 162574.95; unexplained: no kernel event (hypervisor or in-kernel time) 134424.36; mostly unexplained (+softirq NET_RX) 9103.70; mostly unexplained (+local timer tick) 1985.17 |
| boot-full-nomitig | unexplained: no kernel event (hypervisor or in-kernel time) 166052.90; VM paused: the idle spare core stalled too (hypervisor) 147034.86; loop work 111929.09; mostly unexplained (+softirq NET_RX) 24376.34; mostly unexplained (+preempted by kworker/3:1) 17773.03 |
| boot-full | loop work 152743.35; unexplained: no kernel event (hypervisor or in-kernel time) 123912.35; mostly unexplained (+preempted by kworker/3:1) 34545.56; mostly unexplained (+softirq NET_RX) 23930.20; softirq NET_RX 8594.48 |

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
| http-recv-busy-10000 | idle spare core | 1 | :31 | 1 (100%) | 1 of 48 | 392.09 | no |
| http-recv-busy-10000 | receiver loop | 1 | :45 | 1 (100%) | 1 of 48 | 345.69 | no |
| http-recv-busy-10000 | sender loop | 0 | | | | | |
| http-recv-busy-10000 | slow samples | 119 | :49 | 37 (31%) | 1 of 14 | 697.59 | **yes** |
| md-recv-busy | idle spare core | 2 | :01 | 1 (50%) | 1 of 30 | 355.37 | no |
| md-recv-busy | receiver loop | 15 | :57 | 3 (20%) | 3 of 26 | 593.45 | no |
| md-recv-busy | slow samples | 0 | | | | | |
| noise | noise probe (loop CPU) | 4 | :13 | 1 (25%) | 1 of 47 | 346.00 | no |
| pp-epoll-spin-0 | idle spare core | 0 | | | | | |
| pp-epoll-spin-0 | receiver loop | 1 | :59 | 1 (100%) | 1 of 11 | 347.29 | no |
| pp-epoll-spin-0 | sender loop | 0 | | | | | |
| pp-epoll-spin-0 | slow samples | 14 | :37 | 4 (29%) | 2 of 8 | 343.74 | no |
| pp-epoll-spin-10000 | idle spare core | 1 | :03 | 1 (100%) | 1 of 29 | 366.44 | no |
| pp-epoll-spin-10000 | receiver loop | 1 | :10 | 1 (100%) | 1 of 23 | 367.19 | no |
| pp-epoll-spin-10000 | sender loop | 0 | | | | | |
| pp-epoll-spin-10000 | slow samples | 2675 | :57 | 260 (10%) | 3 of 16 | 446.71 | no |
| pp-recv-busy-0 | idle spare core | 1 | :54 | 1 (100%) | 1 of 29 | 345.68 | no |
| pp-recv-busy-0 | receiver loop | 1 | :08 | 1 (100%) | 1 of 22 | 348.63 | no |
| pp-recv-busy-0 | sender loop | 0 | | | | | |
| pp-recv-busy-0 | slow samples | 4 | :12 | 3 (75%) | 3 of 8 | 1038.29 | no |
| pp-recv-busy-10000 | idle spare core | 1 | :08 | 1 (100%) | 1 of 47 | 345.78 | no |
| pp-recv-busy-10000 | receiver loop | 2 | :13 | 1 (50%) | 1 of 45 | 388.06 | no |
| pp-recv-busy-10000 | sender loop | 0 | | | | | |
| pp-recv-busy-10000 | slow samples | 1 | :16 | 1 (100%) | 1 of 16 | 972.63 | no |
| pp-uring-spin-0 | idle spare core | 0 | | | | | |
| pp-uring-spin-0 | receiver loop | 0 | | | | | |
| pp-uring-spin-0 | sender loop | 0 | | | | | |
| pp-uring-spin-0 | slow samples | 4 | :18 | 3 (75%) | 1 of 7 | 1035.10 | no |
| pp-uring-spin-10000 | idle spare core | 2 | :06 | 1 (50%) | 1 of 42 | 370.18 | no |
| pp-uring-spin-10000 | receiver loop | 4 | :12 | 1 (25%) | 1 of 34 | 342.55 | no |
| pp-uring-spin-10000 | sender loop | 0 | | | | | |
| pp-uring-spin-10000 | slow samples | 351 | :54 | 54 (15%) | 2 of 14 | 402.96 | no |
| pp-uring-sqpoll-0 | idle spare core | 1 | :45 | 1 (100%) | 1 of 22 | 561.19 | no |
| pp-uring-sqpoll-0 | receiver loop | 2 | :35 | 1 (50%) | 1 of 12 | 390.61 | no |
| pp-uring-sqpoll-0 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-0 | slow samples | 3 | :24 | 1 (33%) | 1 of 7 | 331.52 | no |
| pp-uring-sqpoll-10000 | idle spare core | 21 | :41 | 20 (95%) | 2 of 33 | 137451.00 | **yes** |
| pp-uring-sqpoll-10000 | receiver loop | 25 | :41 | 22 (88%) | 1 of 21 | 137458.57 | **yes** |
| pp-uring-sqpoll-10000 | sender loop | 0 | | | | | |
| pp-uring-sqpoll-10000 | slow samples | 30 | :40 | 27 (90%) | 1 of 15 | 15297.53 | **yes** |

# Every run in detail

#### Stalls: r1-base-0/http-recv-busy-10000, loop 0 on CPU 3

2017200752 iterations, longest 345.7 µs; over 5 / 20 / 100 µs: 128368 / 5497 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2817 | 72998.8 | 45.0 |
| mostly unexplained (+preempted by kworker/3:1) | 1561 | 40455.3 | 58.3 |
| mostly unexplained (+softirq NET_RX) | 882 | 23361.7 | 58.5 |
| softirq NET_RX | 188 | 4614.8 | 36.3 |
| loop work | 38 | 1021.0 | 39.1 |
| softirq TIMER | 1 | 345.7 | 345.7 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 122.0 | 42.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 117.2 | 33.3 |
| mostly unexplained (+local timer tick) | 2 | 48.7 | 26.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 345.7 | 2 | softirq TIMER | softirq TIMER 332, local timer tick 5, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 2 | 58.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 22, preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 58.3 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 57.7 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 57.1 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 54.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 18, preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 54.0 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 54.0 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 53.6 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 53.1 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3429 |
| IWI IRQ work interrupts | 3411 |
| CAL Function call interrupts | 999 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/http-recv-busy-10000 (median 42.01)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 697.59 | 655.58 | 0.00 |  | 0.00 |  | 655.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 579.83 | 537.82 | 0.00 |  | 0.00 |  | 537.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 548.94 | 506.93 | 0.00 |  | 0.00 |  | 506.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 533.43 | 491.42 | 0.00 |  | 0.00 |  | 491.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 521.06 | 479.05 | 0.00 |  | 0.00 |  | 479.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 514.33 | 472.32 | 0.00 |  | 0.00 |  | 472.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 493.92 | 451.91 | 0.00 |  | 0.00 |  | 451.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 488.21 | 446.20 | 0.00 |  | 21.47 | mostly unexplained (+softirq NET_RX) | 424.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 473.67 | 431.66 | 0.00 |  | 0.00 |  | 431.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 469.80 | 427.79 | 0.00 |  | 0.00 |  | 427.79 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-0/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=375973 clocksource=tsc

### Loop counters (r1-base-0/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002500 | 432767 | 6.94 | 112516992 | 37.47 | 1875283 | 1875283 | 99.6 | 65.3 | true | 0 | 1000 | 551.5 | 146732 / 5225 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672560000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (42308 of 2900000 in 1-msg reads) | 42308 | 3.30 | 2.72 | 4.87 | 9.12 | 17.41 | 31.46 | 37.45 |

#### Stalls: r1-base-0/md-recv-busy, loop 0 on CPU 3

112516992 iterations, longest 551.5 µs; over 5 / 20 / 100 µs: 146732 / 5225 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4925 | 160222.3 | 551.5 |
| mostly unexplained (+preempted by kworker/3:1) | 138 | 3648.9 | 51.5 |
| softirq NET_RX | 98 | 2762.1 | 42.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 46 | 1160.8 | 31.1 |
| mostly unexplained (+softirq NET_RX) | 16 | 488.2 | 49.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 53.5 | 33.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 551.5 | 2500 | loop work (2500 items) |  |
| 2 | 68.3 | 26 | loop work (26 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 67.5 | 31 | loop work (31 items) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 66.4 | 21 | loop work (21 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 66.2 | 25 | loop work (25 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 65.5 | 22 | loop work (22 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 65.4 | 18 | loop work (18 items) | softirq NET_RX 22, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 65.3 | 20 | loop work (20 items) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 64.9 | 18 | loop work (18 items) | softirq NET_RX 21, preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 64.8 | 33 | loop work (33 items) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1005 |
| LOC Local timer interrupts | 1002 |
| CAL Function call interrupts | 992 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-0/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-0/noise, loop 0 on CPU 3

9240854528 iterations, longest 242.2 µs; over 5 / 20 / 100 µs: 97 / 2 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 78 | 688.2 | 32.9 |
| softirq TIMER | 1 | 242.2 | 242.2 |
| mostly unexplained (+call function (IPI/vector)) | 18 | 101.1 | 6.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 242.2 | 0 | softirq TIMER | softirq TIMER 225, local timer tick 12, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 32.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 18.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 16.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 13.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 11.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 11.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 11.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 11.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (r1-base-0/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1210295 | 1210295 | 1.00 | 287236463 | 237.33 | 4787274 | 4746931 | 0.0 | 40.8 | false | 0 | 0 | 42.2 | 43026 / 10 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-0, loop 0 on CPU 3

284815872 iterations, longest 42.2 µs; over 5 / 20 / 100 µs: 43026 / 10 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 7 | 182.3 | 42.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 57.4 | 34.5 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 25.6 | 25.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 42.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 34.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 26.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 25.6 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 25.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 23.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 22.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 22.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 21.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 20.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 12 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-0 (median 48.71)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 343.74 | 295.03 | 0.00 |  | 0.00 |  | 295.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 211.31 | 162.61 | 0.00 |  | 0.00 |  | 162.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 208.30 | 159.60 | 0.00 |  | 0.00 |  | 159.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 196.24 | 147.53 | 0.00 |  | 0.00 |  | 147.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 196.20 | 147.49 | 0.00 |  | 0.00 |  | 147.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 190.74 | 142.03 | 0.00 |  | 0.00 |  | 142.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 188.02 | 139.31 | 0.00 |  | 0.00 |  | 139.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 185.76 | 137.06 | 0.00 |  | 0.00 |  | 137.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 182.90 | 134.20 | 0.00 |  | 0.00 |  | 134.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 182.22 | 133.51 | 0.00 |  | 0.00 |  | 133.51 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 567351104 | 472.79 | 4727926 | 4707926 | 0.0 | 37.0 | false | 0 | 0 | 32.0 | 8921 / 3 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-epoll-spin-10000, loop 0 on CPU 3

564951104 iterations, longest 32.0 µs; over 5 / 20 / 100 µs: 8921 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 79.9 | 32.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 32.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 24.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 23.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 5 |

#### Slowest round trips: r1-base-0/pp-epoll-spin-10000 (median 47.63)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 402.69 | 355.06 | 0.00 |  | 0.00 |  | 355.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 400.84 | 353.21 | 0.00 |  | 0.00 |  | 353.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 356.44 | 308.81 | 0.00 |  | 0.00 |  | 308.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 353.09 | 305.46 | 0.00 |  | 0.00 |  | 305.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 343.32 | 295.69 | 0.00 |  | 0.00 |  | 295.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 325.98 | 278.35 | 0.00 |  | 0.00 |  | 278.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 324.80 | 277.17 | 0.00 |  | 0.00 |  | 277.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 324.71 | 277.07 | 0.00 |  | 0.00 |  | 277.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 322.59 | 274.96 | 0.00 |  | 0.00 |  | 274.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 318.47 | 270.84 | 0.00 |  | 0.00 |  | 270.84 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1500248 | 1500248 | 1.00 | 110977049 | 73.97 | 1849617 | 1824613 | 98.6 | 71.0 | true | 0 | 2345 | 41.2 | 12023 / 1158 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-0, loop 0 on CPU 3

109476800 iterations, longest 41.2 µs; over 5 / 20 / 100 µs: 12023 / 1158 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 755 | 18971.9 | 41.2 |
| mostly unexplained (+softirq NET_RX) | 261 | 6836.8 | 36.1 |
| softirq NET_RX | 127 | 2958.3 | 36.7 |
| mostly unexplained (+local timer tick) | 8 | 208.3 | 33.1 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 95.5 | 39.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 81.2 | 28.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 38.5 | 38.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 41.2 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 2 | 39.0 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 38.5 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 36.7 | 0 | softirq NET_RX | softirq NET_RX 19, preempted by ksoftirqd/3 12, local timer tick 6, timer tick_nohz_handler 4 |
| 5 | 36.6 | 0 | softirq NET_RX | softirq NET_RX 21, preempted by ksoftirqd/3 10, local timer tick 7, timer tick_nohz_handler 4 |
| 6 | 36.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/3:1 10, local timer tick 4, irq work (IPI/vector) 1 |
| 7 | 34.0 | 1 | softirq NET_RX | softirq NET_RX 19, local timer tick 5, preempted by kworker/3:1 5, timer tick_nohz_handler 3 |
| 8 | 33.3 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 9, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 9 | 33.1 | 0 | mostly unexplained (+local timer tick) | local timer tick 6, preempted by kworker/3:1 6, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 10 | 32.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2435 |
| IWI IRQ work interrupts | 2350 |
| CAL Function call interrupts | 989 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-recv-busy-0 (median 39.51)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1038.29 | 998.78 | 0.00 |  | 0.00 |  | 998.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 168.72 | 129.22 | 0.00 |  | 0.00 |  | 129.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 166.60 | 127.09 | 0.00 |  | 0.00 |  | 127.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 164.64 | 125.14 | 0.00 |  | 0.00 |  | 125.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 161.85 | 122.34 | 0.00 |  | 0.00 |  | 122.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 159.88 | 120.37 | 0.00 |  | 0.00 |  | 120.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 159.59 | 120.08 | 0.00 |  | 0.00 |  | 120.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 159.06 | 119.55 | 0.00 |  | 0.00 |  | 119.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 156.72 | 117.21 | 0.00 |  | 0.00 |  | 117.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 154.90 | 115.39 | 0.00 |  | 0.00 |  | 115.39 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 228845120 | 190.70 | 1907043 | 1897043 | 99.5 | 68.2 | true | 0 | 3440 | 42.8 | 131346 / 1818 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-recv-busy-10000, loop 0 on CPU 3

227645120 iterations, longest 42.8 µs; over 5 / 20 / 100 µs: 131346 / 1818 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 1299 | 30745.6 | 39.2 |
| softirq NET_RX | 256 | 5806.4 | 42.8 |
| mostly unexplained (+softirq NET_RX) | 220 | 5608.1 | 37.9 |
| mostly unexplained (+local timer tick) | 36 | 1113.1 | 36.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 95.7 | 33.2 |
| loop work | 2 | 40.0 | 20.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 28.1 | 28.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 22.1 | 22.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 42.8 | 1 | softirq NET_RX | softirq NET_RX 25, preempted by kworker/3:1 9, local timer tick 8, timer tick_nohz_handler 5 |
| 2 | 39.2 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 11, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 3 | 39.2 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 11, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 4 | 38.4 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 12, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 5 | 38.1 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 12, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 6 | 38.0 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 14, softirq NET_RX 14, local timer tick 5, timer tick_nohz_handler 3 |
| 7 | 37.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/3:1 13, local timer tick 6, timer tick_nohz_handler 2 |
| 8 | 37.0 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 10, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 9 | 37.0 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 36.8 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, preempted by kworker/3:1 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3581 |
| IWI IRQ work interrupts | 3451 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-base-0/pp-recv-busy-10000 (median 41.82)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 134.24 | 92.42 | 0.00 |  | 0.00 |  | 92.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 128.60 | 86.78 | 0.00 |  | 0.00 |  | 86.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 128.39 | 86.57 | 0.00 |  | 0.00 |  | 86.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 128.35 | 86.53 | 0.00 |  | 0.00 |  | 86.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 127.84 | 86.02 | 0.00 |  | 0.00 |  | 86.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 127.36 | 85.53 | 0.00 |  | 0.00 |  | 85.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 122.45 | 80.63 | 0.00 |  | 0.00 |  | 80.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 121.45 | 79.63 | 0.00 |  | 0.00 |  | 79.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 121.33 | 79.51 | 0.00 |  | 0.00 |  | 79.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 120.69 | 78.87 | 0.00 |  | 0.00 |  | 78.87 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1199977 | 1199977 | 1.00 | 3599932 | 3.00 | 59999 | 45787504 | 0.0 | 9.9 | false | 0 | 0 | 208.0 | 13642 / 18 / 1 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-spin-0, loop 0 on CPU 3

2747250240 iterations, longest 208.0 µs; over 5 / 20 / 100 µs: 13642 / 18 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 14 | 328.4 | 30.9 |
| softirq TIMER | 1 | 208.0 | 208.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.5 | 31.5 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 36.4 | 36.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 208.0 | 0 | softirq TIMER | softirq TIMER 184, local timer tick 14, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 36.4 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 31.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 30.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 26.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 24.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 23.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 23.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 23.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 23.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 14 |
| IWI IRQ work interrupts | 6 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/pp-uring-spin-0 (median 48.37)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 265.65 | 217.28 | 0.00 |  | 0.00 |  | 217.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 240.71 | 192.34 | 207.96 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 217.51 | 169.14 | 0.00 |  | 206.92 | softirq TIMER | 0.00 | server: softirq TIMER |
| 4 | 210.01 | 161.64 | 0.00 |  | 0.00 |  | 161.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 209.50 | 161.12 | 0.00 |  | 0.00 |  | 161.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 209.03 | 160.66 | 0.00 |  | 0.00 |  | 160.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 196.91 | 148.53 | 0.00 |  | 21.75 | mostly unexplained (+preempted by kworker/1:1) | 126.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 188.41 | 140.03 | 0.00 |  | 0.00 |  | 140.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 186.85 | 138.48 | 0.00 |  | 0.00 |  | 138.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 185.16 | 136.78 | 0.00 |  | 0.00 |  | 136.78 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1200000 | 1200000 | 1.00 | 3600000 | 3.00 | 30000 | 34834298 | 0.0 | 5.8 | false | 0 | 0 | 35.5 | 20227 / 12 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-spin-10000, loop 0 on CPU 3

4180115712 iterations, longest 35.5 µs; over 5 / 20 / 100 µs: 20227 / 12 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 8 | 166.8 | 21.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 76.9 | 32.2 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 35.5 | 35.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 35.5 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 32.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 24.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 21.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 21.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 21.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 20.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 20.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 20.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 20.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-0/pp-uring-spin-10000 (median 46.73)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 431.81 | 385.08 | 0.00 |  | 0.00 |  | 385.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 403.51 | 356.78 | 0.00 |  | 0.00 |  | 356.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 402.96 | 356.23 | 0.00 |  | 0.00 |  | 356.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 391.98 | 345.25 | 0.00 |  | 0.00 |  | 345.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 390.79 | 344.06 | 0.00 |  | 0.00 |  | 344.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 386.50 | 339.77 | 0.00 |  | 0.00 |  | 339.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 381.40 | 334.68 | 0.00 |  | 0.00 |  | 334.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 379.66 | 332.94 | 0.00 |  | 0.00 |  | 332.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 368.06 | 321.33 | 0.00 |  | 0.00 |  | 321.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 368.01 | 321.28 | 0.00 |  | 0.00 |  | 321.28 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1480837 | 1480837 | 1.00 | 1 | 0.00 | 0 | 39641767 | 0.0 | 0.0 | true | 0 | 0 | 34.1 | 64 / 2 / 0 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-0, loop 0 on CPU 3

2378506048 iterations, longest 34.1 µs; over 5 / 20 / 100 µs: 64 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 64.0 | 34.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 34.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 29.8 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 3 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-0 (median 40.00)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 176.29 | 136.29 | 0.00 |  | 0.00 |  | 136.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 137.27 | 97.27 | 0.00 |  | 0.00 |  | 97.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 135.16 | 95.16 | 0.00 |  | 0.00 |  | 95.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 129.96 | 89.95 | 0.00 |  | 0.00 |  | 89.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 128.34 | 88.34 | 0.00 |  | 0.00 |  | 88.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 123.53 | 83.52 | 0.00 |  | 0.00 |  | 83.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 121.68 | 81.67 | 0.00 |  | 0.00 |  | 81.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 121.66 | 81.66 | 0.00 |  | 0.00 |  | 81.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 121.45 | 81.45 | 0.00 |  | 0.00 |  | 81.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 116.91 | 76.91 | 0.00 |  | 0.00 |  | 76.91 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-0/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 25078322 | 0.0 | 0.0 | true | 0 | 0 | 374.0 | 120 / 4 / 1 | 0 | 0 |

#### Stalls: r1-base-0/pp-uring-sqpoll-10000, loop 0 on CPU 3

3009398656 iterations, longest 374.0 µs; over 5 / 20 / 100 µs: 120 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 374.0 | 374.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 63.5 | 32.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 36.7 | 36.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 374.0 | 0 | softirq TIMER | softirq TIMER 358, local timer tick 10, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 36.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 32.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 30.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-0/pp-uring-sqpoll-10000 (median 41.73)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 205.63 | 163.90 | 0.00 |  | 191.05 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 129.68 | 87.95 | 0.00 |  | 0.00 |  | 87.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 125.49 | 83.76 | 0.00 |  | 0.00 |  | 83.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 125.17 | 83.44 | 0.00 |  | 0.00 |  | 83.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 115.48 | 73.75 | 0.00 |  | 0.00 |  | 73.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 114.94 | 73.21 | 0.00 |  | 0.00 |  | 73.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 114.22 | 72.49 | 0.00 |  | 0.00 |  | 72.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 112.94 | 71.21 | 0.00 |  | 0.00 |  | 71.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 111.87 | 70.14 | 0.00 |  | 0.00 |  | 70.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 108.00 | 66.27 | 0.00 |  | 0.00 |  | 66.27 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-housekeeping/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-housekeeping/noise, loop 0 on CPU 3

9241726976 iterations, longest 346.0 µs; over 5 / 20 / 100 µs: 96 / 2 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 77 | 698.6 | 32.6 |
| softirq TIMER | 1 | 346.0 | 346.0 |
| mostly unexplained (+call function (IPI/vector)) | 18 | 100.1 | 7.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 346.0 | 0 | softirq TIMER | softirq TIMER 325, local timer tick 16, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 32.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 18.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 16.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 13.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 12.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 12.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 12.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 11.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 11.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1004 |
| IWI IRQ work interrupts | 1 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

## r1-prefer/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-prefer/noise, loop 0 on CPU 3

9244602368 iterations, longest 214.1 µs; over 5 / 20 / 100 µs: 87 / 2 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 67 | 620.7 | 32.9 |
| softirq TIMER | 1 | 214.1 | 214.1 |
| mostly unexplained (+call function (IPI/vector)) | 19 | 102.8 | 8.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 214.1 | 0 | softirq TIMER | softirq TIMER 192, local timer tick 15, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 32.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 18.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 17.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 13.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 12.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 11.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 11.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 11.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| IWI IRQ work interrupts | 1 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

## r1-base-2/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=388673 clocksource=tsc

### Loop counters (r1-base-2/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002450 | 435985 | 6.89 | 113624768 | 37.84 | 1893746 | 1893746 | 99.6 | 65.1 | true | 0 | 355 | 436.5 | 96135 / 4411 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672548800 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (59514 of 2900000 in 1-msg reads) | 59514 | 2.48 | 2.24 | 2.96 | 5.39 | 19.86 | 28.06 | 35.03 |

#### Stalls: r1-base-2/md-recv-busy, loop 0 on CPU 3

113624768 iterations, longest 436.5 µs; over 5 / 20 / 100 µs: 96135 / 4411 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4148 | 117113.6 | 436.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 101 | 2351.2 | 38.4 |
| mostly unexplained (+preempted by kworker/3:1) | 90 | 2264.2 | 40.6 |
| softirq NET_RX | 57 | 1481.9 | 36.0 |
| mostly unexplained (+softirq NET_RX) | 12 | 352.9 | 36.0 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 64.6 | 33.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 32.0 | 32.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 436.5 | 2450 | loop work (2450 items) |  |
| 2 | 64.3 | 16 | loop work (16 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 62.0 | 29 | loop work (29 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 61.5 | 24 | loop work (24 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 59.5 | 28 | loop work (28 items) | softirq NET_RX 16, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 57.2 | 11 | loop work (11 items) | softirq NET_RX 18, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 52.1 | 13 | loop work (13 items) | softirq NET_RX 18, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 51.3 | 9 | loop work (9 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 49.2 | 5 | loop work (5 items) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 48.7 | 27 | loop work (27 items) | softirq NET_RX 16, preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 991 |
| IWI IRQ work interrupts | 366 |
| LOC Local timer interrupts | 360 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-2/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-2/noise, loop 0 on CPU 3

9243688960 iterations, longest 32.4 µs; over 5 / 20 / 100 µs: 81 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 66 | 593.9 | 32.4 |
| mostly unexplained (+call function (IPI/vector)) | 15 | 85.9 | 8.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 32.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 17.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 17.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 11.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 10.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 10.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 10.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 10.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 9.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 9.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |

### Loop counters (r1-base-2/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1201814 | 1201814 | 1.00 | 290502317 | 241.72 | 4841705 | 4801645 | 0.0 | 40.2 | false | 0 | 0 | 35.5 | 9042 / 4 / 0 | 0 | 0 |

#### Stalls: r1-base-2/pp-epoll-spin-0, loop 0 on CPU 3

288098688 iterations, longest 35.5 µs; over 5 / 20 / 100 µs: 9042 / 4 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 43.8 | 23.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.5 | 35.5 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 23.2 | 23.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 35.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 23.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 23.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 20.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 3 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-2/pp-epoll-spin-0 (median 49.03)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 258.29 | 209.26 | 0.00 |  | 0.00 |  | 209.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 225.57 | 176.54 | 0.00 |  | 0.00 |  | 176.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 203.54 | 154.51 | 0.00 |  | 0.00 |  | 154.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 198.37 | 149.34 | 0.00 |  | 0.00 |  | 149.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 196.84 | 147.81 | 0.00 |  | 0.00 |  | 147.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 192.82 | 143.79 | 0.00 |  | 143.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.58 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 192.29 | 143.26 | 0.00 |  | 0.00 |  | 143.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 191.38 | 142.35 | 0.00 |  | 0.00 |  | 142.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 191.15 | 142.12 | 0.00 |  | 0.00 |  | 142.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 185.42 | 136.39 | 0.00 |  | 0.00 |  | 136.39 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 568779136 | 473.98 | 4739826 | 4719826 | 0.0 | 36.9 | false | 0 | 0 | 36.2 | 10377 / 4 / 0 | 0 | 0 |

#### Stalls: r1-base-2/pp-epoll-spin-10000, loop 0 on CPU 3

566379136 iterations, longest 36.2 µs; over 5 / 20 / 100 µs: 10377 / 4 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 61.1 | 36.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 41.5 | 20.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 36.2 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 24.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 20.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 20.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: r1-base-2/pp-epoll-spin-10000 (median 47.21)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 389.61 | 342.39 | 0.00 |  | 0.00 |  | 342.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 374.66 | 327.44 | 0.00 |  | 0.00 |  | 327.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 371.73 | 324.52 | 0.00 |  | 0.00 |  | 324.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 362.93 | 315.72 | 0.00 |  | 0.00 |  | 315.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 357.39 | 310.18 | 0.00 |  | 0.00 |  | 310.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 353.99 | 306.77 | 0.00 |  | 0.00 |  | 306.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 343.99 | 296.77 | 0.00 |  | 0.00 |  | 296.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 340.32 | 293.10 | 0.00 |  | 0.00 |  | 293.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 327.35 | 280.14 | 0.00 |  | 0.00 |  | 280.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 327.24 | 280.02 | 0.00 |  | 0.00 |  | 280.02 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1467002 | 1467002 | 1.00 | 109510395 | 74.65 | 1825173 | 1800723 | 98.6 | 71.7 | true | 0 | 2471 | 46.1 | 59428 / 1149 / 0 | 0 | 0 |

#### Stalls: r1-base-2/pp-recv-busy-0, loop 0 on CPU 3

108043392 iterations, longest 46.1 µs; over 5 / 20 / 100 µs: 59428 / 1149 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 745 | 17760.9 | 35.3 |
| mostly unexplained (+softirq NET_RX) | 242 | 6022.9 | 46.1 |
| softirq NET_RX | 142 | 3248.8 | 34.5 |
| mostly unexplained (+local timer tick) | 11 | 282.3 | 30.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 126.2 | 39.7 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 88.9 | 30.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 61.2 | 34.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 46.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 22, preempted by kworker/3:1 10, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 2 | 39.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 35.3 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 4 | 34.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 34.5 | 0 | softirq NET_RX | softirq NET_RX 21, preempted by ksoftirqd/3 8, local timer tick 6, timer tick_nohz_handler 4 |
| 6 | 33.3 | 1 | softirq NET_RX | softirq NET_RX 19, local timer tick 6, preempted by ksoftirqd/3 6, timer tick_nohz_handler 4 |
| 7 | 33.2 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 13, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 8 | 33.1 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 9 | 32.9 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 12, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 32.7 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 11, local timer tick 4, irq work (IPI/vector) 1, softirq RCU 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2514 |
| IWI IRQ work interrupts | 2477 |
| CAL Function call interrupts | 989 |

#### Slowest round trips: r1-base-2/pp-recv-busy-0 (median 40.48)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 129.50 | 89.02 | 0.00 |  | 0.00 |  | 89.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 126.40 | 85.92 | 0.00 |  | 0.00 |  | 85.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 125.96 | 85.47 | 0.00 |  | 0.00 |  | 85.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 124.39 | 83.91 | 0.00 |  | 0.00 |  | 83.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 122.11 | 81.62 | 0.00 |  | 0.00 |  | 81.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 121.53 | 81.04 | 0.00 |  | 0.00 |  | 81.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 120.73 | 80.25 | 0.00 |  | 0.00 |  | 80.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 120.36 | 79.87 | 0.00 |  | 0.00 |  | 79.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 120.31 | 79.83 | 0.00 |  | 0.00 |  | 79.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 117.91 | 77.42 | 0.00 |  | 0.00 |  | 77.42 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-2/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 229756288 | 191.46 | 1914636 | 1904636 | 99.5 | 68.4 | true | 0 | 3737 | 115.2 | 86886 / 1830 / 1 | 0 | 0 |

#### Stalls: r1-base-2/pp-recv-busy-10000, loop 0 on CPU 3

228556288 iterations, longest 115.2 µs; over 5 / 20 / 100 µs: 86886 / 1830 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 1378 | 31911.1 | 38.8 |
| mostly unexplained (+softirq NET_RX) | 255 | 6118.2 | 30.7 |
| softirq NET_RX | 188 | 4184.3 | 28.7 |
| softirq TIMER | 1 | 115.2 | 115.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 96.0 | 40.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 59.5 | 32.7 |
| mostly unexplained (+local timer tick) | 2 | 56.6 | 30.1 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 28.9 | 28.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 115.2 | 0 | softirq TIMER | softirq TIMER 100, local timer tick 8, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 40.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 38.8 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 32.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 32.0 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 30.8 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 30.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 30.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 30.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 30.1 | 0 | mostly unexplained (+local timer tick) | local timer tick 7, preempted by kworker/3:1 4, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3807 |
| IWI IRQ work interrupts | 3744 |
| CAL Function call interrupts | 999 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-2/pp-recv-busy-10000 (median 41.58)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 254.92 | 213.34 | 0.00 |  | 0.00 |  | 213.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 214.16 | 172.58 | 0.00 |  | 0.00 |  | 172.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 204.66 | 163.08 | 0.00 |  | 0.00 |  | 163.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 196.96 | 155.38 | 0.00 |  | 0.00 |  | 155.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 183.32 | 141.75 | 0.00 |  | 152.53 | softirq TIMER | 0.00 | server: softirq TIMER |
| 6 | 182.05 | 140.47 | 0.00 |  | 0.00 |  | 140.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 173.97 | 132.39 | 0.00 |  | 0.00 |  | 132.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 171.54 | 129.97 | 0.00 |  | 0.00 |  | 129.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 170.39 | 128.81 | 0.00 |  | 0.00 |  | 128.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 165.31 | 123.74 | 0.00 |  | 0.00 |  | 123.74 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-noquickack/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-noquickack/noise, loop 0 on CPU 3

9248010240 iterations, longest 32.4 µs; over 5 / 20 / 100 µs: 83 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 63 | 583.2 | 32.4 |
| mostly unexplained (+call function (IPI/vector)) | 14 | 74.2 | 6.1 |
| mostly unexplained (+local timer tick) | 3 | 35.6 | 21.9 |
| local timer tick | 3 | 25.2 | 13.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 32.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 21.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 8, timer tick_nohz_handler 5, softirq TIMER 4 |
| 3 | 20.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 17.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 16.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 13.7 | 0 | local timer tick | local timer tick 9, timer tick_nohz_handler 5 |
| 7 | 12.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 10.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 10.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 1 |
| RES Rescheduling interrupts | 1 |

## r1-nogro/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-nogro/noise, loop 0 on CPU 3

9245171712 iterations, longest 34.7 µs; over 5 / 20 / 100 µs: 87 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 68 | 621.4 | 34.7 |
| mostly unexplained (+call function (IPI/vector)) | 19 | 100.6 | 6.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 34.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 18.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 17.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 16.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 11.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 10.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 10.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 10.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 10.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |

## r1-base-4/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-4/noise, loop 0 on CPU 3

9244786688 iterations, longest 116.6 µs; over 5 / 20 / 100 µs: 91 / 2 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 69 | 622.8 | 32.1 |
| softirq TIMER | 1 | 116.6 | 116.6 |
| mostly unexplained (+call function (IPI/vector)) | 16 | 87.0 | 6.3 |
| local timer tick | 3 | 25.8 | 14.6 |
| mostly unexplained (+local timer tick) | 2 | 15.7 | 9.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 116.6 | 0 | softirq TIMER | softirq TIMER 102, local timer tick 9, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 32.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 17.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 17.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 14.6 | 0 | local timer tick | local timer tick 9, timer tick_nohz_handler 6 |
| 6 | 13.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 11.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 11.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 10.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## r1-nocoalesce/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-nocoalesce/noise, loop 0 on CPU 3

9244266496 iterations, longest 73.8 µs; over 5 / 20 / 100 µs: 83 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 63 | 577.6 | 31.8 |
| mostly unexplained (+call function (IPI/vector)) | 15 | 84.0 | 8.9 |
| softirq TIMER | 1 | 73.8 | 73.8 |
| mostly unexplained (+local timer tick) | 4 | 30.3 | 9.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 73.8 | 0 | softirq TIMER | softirq TIMER 60, local timer tick 10, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 31.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 20.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 17.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 17.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 11.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 10.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 10.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 10.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 9.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| LOC Local timer interrupts | 6 |
| IWI IRQ work interrupts | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## r1-irqlocal/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-irqlocal/noise, loop 0 on CPU 3

9293152256 iterations, longest 43.4 µs; over 5 / 20 / 100 µs: 773 / 34 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq NET_RX | 373 | 4404.9 | 43.4 |
| mostly unexplained (+softirq NET_RX) | 165 | 1457.4 | 43.2 |
| mostly unexplained (+local timer tick) | 62 | 759.5 | 21.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 74 | 634.5 | 32.8 |
| local timer tick | 39 | 489.1 | 15.1 |
| mostly unexplained (+irq ena-mgmnt@pci:0000:27:00.0) | 43 | 341.6 | 12.2 |
| mostly unexplained (+irq enp39s0-Tx-Rx-6) | 6 | 37.0 | 10.5 |
| mostly unexplained (+irq enp39s0-Tx-Rx-0) | 3 | 15.7 | 5.5 |
| mostly unexplained (+irq enp39s0-Tx-Rx-4) | 2 | 11.8 | 6.7 |
| mostly unexplained (+irq enp39s0-Tx-Rx-3) | 2 | 11.7 | 6.6 |
| mostly unexplained (+irq enp39s0-Tx-Rx-7) | 1 | 8.1 | 8.1 |
| mostly unexplained (+irq enp39s0-Tx-Rx-1) | 1 | 5.5 | 5.5 |
| mostly unexplained (+irq enp39s0-Tx-Rx-2) | 1 | 5.3 | 5.3 |
| mostly unexplained (+irq enp39s0-Tx-Rx-5) | 1 | 5.1 | 5.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 43.4 | 0 | softirq NET_RX | softirq NET_RX 39, irq enp39s0-Tx-Rx-3 1 |
| 2 | 43.2 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 21, preempted by kworker/3:1 12, irq enp39s0-Tx-Rx-4 2, irq work (IPI/vector) 2 |
| 3 | 43.1 | 0 | softirq NET_RX | softirq NET_RX 32, irq enp39s0-Tx-Rx-0 1, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 40.1 | 0 | softirq NET_RX | softirq NET_RX 27, irq enp39s0-Tx-Rx-7 1, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 38.4 | 0 | softirq NET_RX | softirq NET_RX 27, local timer tick 6, irq work (IPI/vector) 2, timer tick_nohz_handler 2 |
| 6 | 36.3 | 0 | softirq NET_RX | softirq NET_RX 22, preempted by kworker/3:1 5, irq work (IPI/vector) 2, irq enp39s0-Tx-Rx-7 1 |
| 7 | 32.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 25.9 | 0 | softirq NET_RX | softirq NET_RX 19, irq work (IPI/vector) 2, irq enp39s0-Tx-Rx-0 1, ipi_send_cpu 0 |
| 9 | 25.1 | 0 | softirq NET_RX | softirq NET_RX 16, irq enp39s0-Tx-Rx-2 1, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 24.1 | 0 | softirq NET_RX | softirq NET_RX 17, irq enp39s0-Tx-Rx-5 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 325 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 251 |
| 73 PCI-MSIX-0000:27:00.0 7-edge enp39s0-Tx-Rx-6 | 178 |
| 69 PCI-MSIX-0000:27:00.0 3-edge enp39s0-Tx-Rx-2 | 133 |
| 66 PCI-MSIX-0000:27:00.0 0-edge ena-mgmnt@pci:0000:27:00.0 | 120 |
| LOC Local timer interrupts | 105 |
| 72 PCI-MSIX-0000:27:00.0 6-edge enp39s0-Tx-Rx-5 | 102 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 72 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 72 |
| 70 PCI-MSIX-0000:27:00.0 4-edge enp39s0-Tx-Rx-3 | 71 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 56 |

## r1-base-6/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-6/noise, loop 0 on CPU 3

9244770304 iterations, longest 5060.7 µs; over 5 / 20 / 100 µs: 95 / 5 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+softirq NET_RX) | 1 | 5060.7 | 5060.7 |
| mostly unexplained (+local timer tick) | 6 | 3985.9 | 3909.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 72 | 665.6 | 30.1 |
| mostly unexplained (+call function (IPI/vector)) | 10 | 54.1 | 6.3 |
| mostly unexplained (+call function single (IPI/vector)) | 5 | 35.7 | 10.5 |
| local timer tick | 1 | 23.2 | 23.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 5060.7 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 21, irq enp39s0-Tx-Rx-4 1, irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 3909.4 | 0 | mostly unexplained (+local timer tick) | local timer tick 15, timer tick_nohz_handler 9, softirq NET_RX 5, irq enp39s0-Tx-Rx-0 2 |
| 3 | 30.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 26.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 23.2 | 0 | local timer tick | local timer tick 12, timer tick_nohz_handler 8, softirq TIMER 2 |
| 6 | 19.2 | 0 | mostly unexplained (+local timer tick) | local timer tick 8, timer tick_nohz_handler 5, softirq TIMER 2, vector_free_moved 0 |
| 7 | 17.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 16.0 | 0 | mostly unexplained (+local timer tick) | local timer tick 7, timer tick_nohz_handler 5, softirq TIMER 1, vector_free_moved 0 |
| 9 | 15.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 15.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1003 |
| LOC Local timer interrupts | 7 |
| 67 PCI-MSIX-0000:27:00.0 1-edge enp39s0-Tx-Rx-0 | 1 |
| 68 PCI-MSIX-0000:27:00.0 2-edge enp39s0-Tx-Rx-1 | 1 |
| 71 PCI-MSIX-0000:27:00.0 5-edge enp39s0-Tx-Rx-4 | 1 |
| IWI IRQ work interrupts | 1 |
| RES Rescheduling interrupts | 1 |

## r1-rt/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-rt/noise, loop 0 on CPU 3

9260216320 iterations, longest 36.3 µs; over 5 / 20 / 100 µs: 78 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 64 | 582.1 | 36.3 |
| mostly unexplained (+call function (IPI/vector)) | 14 | 77.9 | 8.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 36.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 19.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 14.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 10.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 10.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 9.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 9.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 9.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 9.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 9.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |

## r1-sibling/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-sibling/noise, loop 0 on CPU 3

7686373376 iterations, longest 30.4 µs; over 5 / 20 / 100 µs: 74 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 59 | 566.9 | 30.4 |
| mostly unexplained (+call function (IPI/vector)) | 11 | 57.3 | 5.5 |
| mostly unexplained (+local timer tick) | 2 | 22.7 | 12.7 |
| local timer tick | 2 | 13.8 | 7.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 30.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 19.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 13.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 12.7 | 0 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 3 |
| 5 | 11.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 10.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 10.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 10.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 10.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 10.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| LOC Local timer interrupts | 5 |
| IWI IRQ work interrupts | 1 |
| RES Rescheduling interrupts | 1 |

## r1-base-8/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-8/noise, loop 0 on CPU 3

9246044160 iterations, longest 28.2 µs; over 5 / 20 / 100 µs: 74 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 59 | 526.9 | 28.2 |
| mostly unexplained (+call function (IPI/vector)) | 15 | 81.6 | 7.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 28.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 17.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 15.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 13.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 11.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 10.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 10.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 10.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 9.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 9.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |

## r1-busypoll50/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-busypoll50/noise, loop 0 on CPU 3

9239420928 iterations, longest 26.8 µs; over 5 / 20 / 100 µs: 79 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 63 | 567.4 | 26.8 |
| mostly unexplained (+call function (IPI/vector)) | 16 | 89.3 | 6.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 26.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 21.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 16.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 15.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 10.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 10.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 9.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 9.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 9.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 9.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |

## r1-budget64/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-budget64/noise, loop 0 on CPU 3

9237917696 iterations, longest 29.3 µs; over 5 / 20 / 100 µs: 70 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 62 | 536.1 | 29.3 |
| mostly unexplained (+call function (IPI/vector)) | 8 | 46.1 | 7.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 29.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 17.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 14.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 10.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 10.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 10.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 9.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 9.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 9.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 9.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |

#### Stalls: r1-base-10/http-recv-busy-10000, loop 0 on CPU 3

2052651699 iterations, longest 89.5 µs; over 5 / 20 / 100 µs: 9595 / 5114 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4193 | 96582.6 | 65.8 |
| mostly unexplained (+softirq NET_RX) | 866 | 21077.4 | 69.5 |
| mostly unexplained (+preempted by kworker/3:1) | 45 | 2384.0 | 89.5 |
| loop work | 6 | 144.1 | 28.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 72.9 | 29.2 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 40.9 | 40.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 89.5 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 22, local timer tick 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 2 | 69.5 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 26, preempted by kworker/3:1 9, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 67.6 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 65.8 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 64.5 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 60.5 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 59.9 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 59.5 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 58.1 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 57.6 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 56 |
| LOC Local timer interrupts | 51 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/http-recv-busy-10000 (median 41.69)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 183.91 | 142.21 | 0.00 |  | 0.00 |  | 142.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 177.18 | 135.49 | 0.00 |  | 0.00 |  | 135.49 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 176.63 | 134.94 | 0.00 |  | 0.00 |  | 134.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 176.16 | 134.47 | 0.00 |  | 0.00 |  | 134.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 175.82 | 134.13 | 0.00 |  | 0.00 |  | 134.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 174.59 | 132.90 | 0.00 |  | 0.00 |  | 132.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 171.88 | 130.19 | 0.00 |  | 0.00 |  | 130.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 169.82 | 128.13 | 0.00 |  | 0.00 |  | 128.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 163.12 | 121.42 | 0.00 |  | 0.00 |  | 121.42 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 163.00 | 121.30 | 0.00 |  | 0.00 |  | 121.30 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-10/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=657614 clocksource=tsc

### Loop counters (r1-base-10/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002850 | 734437 | 4.09 | 113501888 | 37.80 | 1891698 | 1891698 | 99.4 | 65.1 | true | 0 | 123 | 613.1 | 10554 / 5500 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672638400 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (145768 of 2900000 in 1-msg reads) | 145768 | 1.67 | 1.65 | 2.21 | 2.95 | 9.96 | 14.24 | 32.46 |

#### Stalls: r1-base-10/md-recv-busy, loop 0 on CPU 3

113501888 iterations, longest 613.1 µs; over 5 / 20 / 100 µs: 10554 / 5500 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5298 | 135077.4 | 613.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 118 | 2647.4 | 27.2 |
| mostly unexplained (+preempted by kworker/3:1) | 49 | 1219.7 | 30.8 |
| softirq NET_RX | 31 | 780.3 | 32.9 |
| mostly unexplained (+softirq NET_RX) | 3 | 83.2 | 33.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 30.5 | 30.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 613.1 | 2850 | loop work (2850 items) |  |
| 2 | 66.8 | 7 | loop work (7 items) |  |
| 3 | 57.7 | 29 | loop work (29 items) |  |
| 4 | 57.3 | 25 | loop work (25 items) |  |
| 5 | 51.5 | 19 | loop work (19 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 51.4 | 35 | loop work (35 items) |  |
| 7 | 50.3 | 7 | loop work (7 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 50.0 | 8 | loop work (8 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 49.9 | 23 | loop work (23 items) |  |
| 10 | 47.7 | 17 | loop work (17 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 144 |
| LOC Local timer interrupts | 128 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## r1-base-10/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-10/noise, loop 0 on CPU 3

9238777856 iterations, longest 256.5 µs; over 5 / 20 / 100 µs: 76 / 2 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 63 | 550.0 | 28.9 |
| softirq TIMER | 1 | 256.5 | 256.5 |
| mostly unexplained (+call function (IPI/vector)) | 12 | 67.2 | 8.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 256.5 | 0 | softirq TIMER | softirq TIMER 234, local timer tick 16, timer tick_nohz_handler 9, irq work (IPI/vector) 1 |
| 2 | 28.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 16.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 15.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 10.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 10.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 10.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 9.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 9.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 9.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| IWI IRQ work interrupts | 1 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

### Loop counters (r1-base-10/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1227903 | 1227903 | 1.00 | 294231423 | 239.62 | 4903857 | 4862927 | 0.0 | 39.4 | false | 0 | 0 | 347.3 | 1245 / 3 / 1 | 0 | 0 |

#### Stalls: r1-base-10/pp-epoll-spin-0, loop 0 on CPU 3

291775616 iterations, longest 347.3 µs; over 5 / 20 / 100 µs: 1245 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 347.3 | 347.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 29.8 | 29.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 23.4 | 23.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 347.3 | 1 | softirq TIMER | softirq TIMER 315, local timer tick 17, timer tick_nohz_handler 7, softirq RCU 2 |
| 2 | 29.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 23.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 19 |
| IWI IRQ work interrupts | 7 |
| RES Rescheduling interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-10/pp-epoll-spin-0 (median 48.97)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 368.77 | 319.80 | 346.82 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 2 | 366.21 | 317.24 | 0.00 |  | 0.00 |  | 317.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 341.03 | 292.06 | 0.00 |  | 0.00 |  | 292.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 334.75 | 285.78 | 0.00 |  | 0.00 |  | 285.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 328.55 | 279.58 | 0.00 |  | 0.00 |  | 279.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 325.44 | 276.47 | 0.00 |  | 0.00 |  | 276.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 319.67 | 270.70 | 0.00 |  | 0.00 |  | 270.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 319.58 | 270.61 | 0.00 |  | 0.00 |  | 270.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 312.18 | 263.21 | 0.00 |  | 0.00 |  | 263.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 309.02 | 260.05 | 0.00 |  | 0.00 |  | 260.05 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 576340032 | 480.28 | 4802834 | 4782834 | 0.0 | 35.6 | false | 0 | 0 | 28.7 | 130 / 2 / 0 | 0 | 0 |

#### Stalls: r1-base-10/pp-epoll-spin-10000, loop 0 on CPU 3

573940032 iterations, longest 28.7 µs; over 5 / 20 / 100 µs: 130 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 57.4 | 28.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 28.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 28.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-epoll-spin-10000 (median 48.33)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 127.14 | 78.81 | 0.00 |  | 0.00 |  | 78.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 124.11 | 75.78 | 0.00 |  | 0.00 |  | 75.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 122.94 | 74.61 | 0.00 |  | 0.00 |  | 74.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 120.55 | 72.22 | 0.00 |  | 0.00 |  | 72.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 119.37 | 71.04 | 0.00 |  | 0.00 |  | 71.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 112.36 | 64.03 | 0.00 |  | 0.00 |  | 64.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 111.69 | 63.36 | 0.00 |  | 0.00 |  | 63.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 110.43 | 62.09 | 0.00 |  | 0.00 |  | 62.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 110.19 | 61.86 | 0.00 |  | 0.00 |  | 61.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 109.86 | 61.53 | 0.00 |  | 0.00 |  | 61.53 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1410692 | 1410692 | 1.00 | 116546885 | 82.62 | 1942448 | 1918937 | 98.8 | 70.0 | true | 0 | 1944 | 114.8 | 6488 / 589 / 1 | 0 | 0 |

#### Stalls: r1-base-10/pp-recv-busy-0, loop 0 on CPU 3

115136192 iterations, longest 114.8 µs; over 5 / 20 / 100 µs: 6488 / 589 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 354 | 8257.2 | 32.1 |
| mostly unexplained (+softirq NET_RX) | 154 | 3775.9 | 31.1 |
| softirq NET_RX | 69 | 1571.7 | 29.5 |
| softirq TIMER | 1 | 114.8 | 114.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 99.5 | 30.4 |
| mostly unexplained (+local timer tick) | 3 | 75.0 | 32.6 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 56.9 | 32.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.1 | 33.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 114.8 | 0 | softirq TIMER | softirq TIMER 101, local timer tick 8, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 33.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 32.6 | 0 | mostly unexplained (+local timer tick) | local timer tick 7, preempted by kworker/3:1 6, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 4 | 32.1 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 32.1 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 31.1 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 15, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 30.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 30.3 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 29.8 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 12, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 29.5 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 11, local timer tick 4, preempted by kworker/3:1 3, timer tick_nohz_handler 2 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1986 |
| LOC Local timer interrupts | 1964 |
| CAL Function call interrupts | 989 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-10/pp-recv-busy-0 (median 41.98)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 221.01 | 179.03 | 114.80 | softirq TIMER | 0.00 |  | 64.23 | client: softirq TIMER |
| 2 | 185.61 | 143.63 | 0.00 |  | 0.00 |  | 143.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 173.16 | 131.18 | 0.00 |  | 0.00 |  | 131.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 171.17 | 129.19 | 0.00 |  | 0.00 |  | 129.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 168.83 | 126.86 | 0.00 |  | 0.00 |  | 126.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 165.43 | 123.45 | 0.00 |  | 0.00 |  | 123.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 164.96 | 122.99 | 0.00 |  | 0.00 |  | 122.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 163.27 | 121.29 | 0.00 |  | 0.00 |  | 121.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 162.90 | 120.93 | 0.00 |  | 0.00 |  | 120.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 162.88 | 120.90 | 0.00 |  | 0.00 |  | 120.90 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 235672384 | 196.39 | 1963937 | 1953937 | 99.5 | 67.2 | true | 0 | 2135 | 50.1 | 12740 / 858 / 0 | 0 | 0 |

#### Stalls: r1-base-10/pp-recv-busy-10000, loop 0 on CPU 3

234472384 iterations, longest 50.1 µs; over 5 / 20 / 100 µs: 12740 / 858 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 517 | 12505.4 | 50.1 |
| mostly unexplained (+softirq NET_RX) | 254 | 6387.2 | 29.5 |
| softirq NET_RX | 79 | 1814.5 | 30.3 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 113.1 | 34.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 57.5 | 30.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 48.1 | 25.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 50.1 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 12, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 2 | 34.4 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 30.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 30.3 | 0 | softirq NET_RX | softirq NET_RX 17, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 29.5 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 29.4 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 29.4 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 29.1 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 29.0 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 9, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 29.0 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2144 |
| LOC Local timer interrupts | 2137 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: r1-base-10/pp-recv-busy-10000 (median 41.17)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 128.12 | 86.96 | 0.00 |  | 0.00 |  | 86.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 115.36 | 74.20 | 0.00 |  | 0.00 |  | 74.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 113.14 | 71.98 | 0.00 |  | 0.00 |  | 71.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 111.86 | 70.69 | 0.00 |  | 0.00 |  | 70.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 111.23 | 70.06 | 0.00 |  | 0.00 |  | 70.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 111.03 | 69.86 | 0.00 |  | 0.00 |  | 69.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 109.66 | 68.50 | 0.00 |  | 0.00 |  | 68.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 107.13 | 65.97 | 0.00 |  | 0.00 |  | 65.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 104.47 | 63.31 | 0.00 |  | 0.00 |  | 63.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 101.14 | 59.97 | 0.00 |  | 0.00 |  | 59.97 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1226013 | 1226013 | 1.00 | 3678040 | 3.00 | 61301 | 46862464 | 0.0 | 7.8 | false | 0 | 0 | 31.5 | 985 / 2 / 0 | 0 | 0 |

#### Stalls: r1-base-10/pp-uring-spin-0, loop 0 on CPU 3

2811747840 iterations, longest 31.5 µs; over 5 / 20 / 100 µs: 985 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 31.5 | 31.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 29.6 | 29.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 31.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 29.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 11 |
| IWI IRQ work interrupts | 7 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: r1-base-10/pp-uring-spin-0 (median 48.38)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1035.10 | 986.73 | 0.00 |  | 0.00 |  | 986.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 397.82 | 349.44 | 0.00 |  | 0.00 |  | 349.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 325.09 | 276.72 | 0.00 |  | 0.00 |  | 276.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 301.05 | 252.67 | 0.00 |  | 0.00 |  | 252.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 295.81 | 247.44 | 0.00 |  | 0.00 |  | 247.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 290.66 | 242.28 | 0.00 |  | 0.00 |  | 242.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 288.90 | 240.53 | 0.00 |  | 0.00 |  | 240.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 276.56 | 228.19 | 0.00 |  | 0.00 |  | 228.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 276.25 | 227.87 | 0.00 |  | 0.00 |  | 227.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 275.70 | 227.32 | 0.00 |  | 0.00 |  | 227.32 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1200000 | 1200000 | 1.00 | 3600000 | 3.00 | 30000 | 34438958 | 0.0 | 4.6 | false | 0 | 0 | 289.8 | 248 / 4 / 1 | 0 | 0 |

#### Stalls: r1-base-10/pp-uring-spin-10000, loop 0 on CPU 3

4132675008 iterations, longest 289.8 µs; over 5 / 20 / 100 µs: 248 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 289.8 | 289.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 50.1 | 29.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 22.7 | 22.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 289.8 | 0 | softirq TIMER | softirq TIMER 265, local timer tick 14, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 29.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 22.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 20.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1003 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-10/pp-uring-spin-10000 (median 45.43)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 981.86 | 936.43 | 0.00 |  | 0.00 |  | 936.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 316.02 | 270.58 | 0.00 |  | 0.00 |  | 270.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 197.07 | 151.64 | 0.00 |  | 0.00 |  | 151.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 191.80 | 146.36 | 0.00 |  | 0.00 |  | 146.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 165.75 | 120.31 | 0.00 |  | 0.00 |  | 120.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 163.06 | 117.63 | 0.00 |  | 0.00 |  | 117.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 161.70 | 116.26 | 0.00 |  | 0.00 |  | 116.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 155.31 | 109.88 | 0.00 |  | 0.00 |  | 109.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 155.01 | 109.58 | 0.00 |  | 0.00 |  | 109.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 152.12 | 106.68 | 0.00 |  | 0.00 |  | 106.68 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1452405 | 1452405 | 1.00 | 1 | 0.00 | 0 | 37289955 | 0.0 | 0.0 | true | 0 | 0 | 30.7 | 58 / 2 / 0 | 0 | 0 |

#### Stalls: r1-base-10/pp-uring-sqpoll-0, loop 0 on CPU 3

2237397312 iterations, longest 30.7 µs; over 5 / 20 / 100 µs: 58 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 52.2 | 30.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 30.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 21.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 995 |
| LOC Local timer interrupts | 15 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-10/pp-uring-sqpoll-0 (median 40.84)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 195.38 | 154.54 | 0.00 |  | 0.00 |  | 154.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 175.11 | 134.27 | 0.00 |  | 0.00 |  | 134.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 157.66 | 116.82 | 0.00 |  | 0.00 |  | 116.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 152.01 | 111.17 | 0.00 |  | 0.00 |  | 111.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 137.45 | 96.61 | 0.00 |  | 0.00 |  | 96.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 135.40 | 94.56 | 0.00 |  | 0.00 |  | 94.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 133.05 | 92.21 | 0.00 |  | 0.00 |  | 92.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 131.74 | 90.90 | 0.00 |  | 0.00 |  | 90.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 126.96 | 86.12 | 0.00 |  | 0.00 |  | 86.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 126.76 | 85.92 | 0.00 |  | 0.00 |  | 85.92 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-10/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 24939210 | 0.0 | 0.0 | true | 0 | 0 | 35.8 | 107 / 2 / 0 | 0 | 0 |

#### Stalls: r1-base-10/pp-uring-sqpoll-10000, loop 0 on CPU 3

2992705152 iterations, longest 35.8 µs; over 5 / 20 / 100 µs: 107 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 35.8 | 35.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.8 | 28.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 35.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 28.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1007 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-base-10/pp-uring-sqpoll-10000 (median 42.92)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 158.04 | 115.11 | 0.00 |  | 0.00 |  | 115.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 156.59 | 113.67 | 0.00 |  | 0.00 |  | 113.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 155.22 | 112.30 | 0.00 |  | 0.00 |  | 112.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 154.47 | 111.55 | 0.00 |  | 0.00 |  | 111.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 152.44 | 109.52 | 0.00 |  | 0.00 |  | 109.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 150.65 | 107.73 | 0.00 |  | 0.00 |  | 107.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 150.05 | 107.13 | 0.00 |  | 0.00 |  | 107.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 149.87 | 106.95 | 0.00 |  | 0.00 |  | 106.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 149.30 | 106.37 | 0.00 |  | 0.00 |  | 106.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 149.19 | 106.27 | 0.00 |  | 0.00 |  | 106.27 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-cstate/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-cstate/noise, loop 0 on CPU 3

9242443776 iterations, longest 373.0 µs; over 5 / 20 / 100 µs: 76 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 64 | 570.3 | 30.9 |
| softirq TIMER | 1 | 373.0 | 373.0 |
| mostly unexplained (+call function (IPI/vector)) | 11 | 59.3 | 6.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 373.0 | 0 | softirq TIMER | softirq TIMER 351, local timer tick 16, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 30.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 23.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 15.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 14.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 10.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 9.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 9.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 9.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 9.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| IWI IRQ work interrupts | 1 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

## r1-noagent/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-noagent/noise, loop 0 on CPU 3

9240903680 iterations, longest 169.8 µs; over 5 / 20 / 100 µs: 78 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 64 | 564.4 | 28.5 |
| softirq TIMER | 1 | 169.8 | 169.8 |
| mostly unexplained (+call function (IPI/vector)) | 13 | 70.4 | 6.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 169.8 | 0 | softirq TIMER | softirq TIMER 150, local timer tick 15, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 28.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 21.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 16.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 15.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 10.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 10.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 9.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 9.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 9.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| IWI IRQ work interrupts | 1 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Stalls: r1-base-12/http-recv-busy-10000, loop 0 on CPU 3

2052092954 iterations, longest 278.3 µs; over 5 / 20 / 100 µs: 9626 / 4686 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 4126 | 90185.7 | 278.3 |
| mostly unexplained (+softirq NET_RX) | 515 | 11790.0 | 69.9 |
| mostly unexplained (+preempted by kworker/3:1) | 39 | 2134.2 | 81.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 86.3 | 35.3 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 70.5 | 35.8 |
| loop work | 1 | 27.1 | 27.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 278.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 81.7 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 16, local timer tick 7, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 3 | 75.2 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 9, local timer tick 5, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 4 | 74.7 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 9, local timer tick 5, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 5 | 69.9 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 27, preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 59.3 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 58.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 20, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 57.1 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 57.1 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 56.9 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 52 |
| LOC Local timer interrupts | 46 |

#### Slowest round trips: r1-base-12/http-recv-busy-10000 (median 41.72)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 257.01 | 215.29 | 0.00 |  | 0.00 |  | 215.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 185.30 | 143.58 | 0.00 |  | 0.00 |  | 143.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 178.50 | 136.78 | 0.00 |  | 0.00 |  | 136.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 172.79 | 131.07 | 0.00 |  | 0.00 |  | 131.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 169.24 | 127.52 | 0.00 |  | 0.00 |  | 127.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 166.68 | 124.95 | 0.00 |  | 0.00 |  | 124.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 166.32 | 124.60 | 0.00 |  | 0.00 |  | 124.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 163.54 | 121.81 | 0.00 |  | 0.00 |  | 121.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 163.19 | 121.47 | 0.00 |  | 0.00 |  | 121.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 162.34 | 120.61 | 0.00 |  | 0.00 |  | 120.61 | neither loop stalled long enough: network, hypervisor or untraced |

## r1-base-12/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=597269 clocksource=tsc

### Loop counters (r1-base-12/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002750 | 670909 | 4.48 | 113306496 | 37.73 | 1888442 | 1888442 | 99.4 | 65.2 | true | 0 | 501 | 565.6 | 22767 / 5515 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672616000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (113178 of 2900000 in 1-msg reads) | 113178 | 1.97 | 2.01 | 3.13 | 4.04 | 7.11 | 27.77 | 29.93 |

#### Stalls: r1-base-12/md-recv-busy, loop 0 on CPU 3

113306496 iterations, longest 565.6 µs; over 5 / 20 / 100 µs: 22767 / 5515 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5190 | 141352.3 | 565.6 |
| mostly unexplained (+preempted by kworker/3:1) | 146 | 3788.5 | 46.5 |
| softirq NET_RX | 89 | 2354.8 | 37.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 84 | 1938.2 | 35.7 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 60.4 | 32.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 50.5 | 30.3 |
| mostly unexplained (+softirq NET_RX) | 2 | 50.0 | 25.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 565.6 | 2750 | loop work (2750 items) |  |
| 2 | 63.5 | 17 | loop work (17 items) | softirq NET_RX 21, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 60.4 | 18 | loop work (18 items) | softirq NET_RX 22, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 59.8 | 13 | loop work (13 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 59.2 | 19 | loop work (19 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 58.4 | 12 | loop work (12 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 57.7 | 16 | loop work (16 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 57.6 | 20 | loop work (20 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 53.6 | 18 | loop work (18 items) | softirq NET_RX 15, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 53.3 | 16 | loop work (16 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 536 |
| LOC Local timer interrupts | 511 |
| TLB TLB shootdowns | 4 |
| RES Rescheduling interrupts | 1 |

## r1-base-12/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: r1-base-12/noise, loop 0 on CPU 3

9241268224 iterations, longest 24.9 µs; over 5 / 20 / 100 µs: 73 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 61 | 525.7 | 24.6 |
| mostly unexplained (+call function (IPI/vector)) | 11 | 60.8 | 7.9 |
| mostly unexplained (+local timer tick) | 1 | 24.9 | 24.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 24.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 12, timer tick_nohz_handler 5, softirq TIMER 4 |
| 2 | 24.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 17.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 14.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 10.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 10.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 9.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 9.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 9.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 9.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 995 |
| LOC Local timer interrupts | 1 |

### Loop counters (r1-base-12/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1262680 | 1262680 | 1.00 | 294553777 | 233.28 | 4909230 | 4867140 | 0.0 | 39.3 | false | 0 | 0 | 30.6 | 1832 / 2 / 0 | 0 | 0 |

#### Stalls: r1-base-12/pp-epoll-spin-0, loop 0 on CPU 3

292028416 iterations, longest 30.6 µs; over 5 / 20 / 100 µs: 1832 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.6 | 30.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 30.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 24.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 989 |
| LOC Local timer interrupts | 10 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-12/pp-epoll-spin-0 (median 47.12)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 187.80 | 140.68 | 0.00 |  | 0.00 |  | 140.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 185.93 | 138.81 | 0.00 |  | 0.00 |  | 138.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 182.30 | 135.19 | 0.00 |  | 0.00 |  | 135.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 181.83 | 134.71 | 0.00 |  | 0.00 |  | 134.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 181.73 | 134.61 | 0.00 |  | 0.00 |  | 134.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 181.62 | 134.50 | 0.00 |  | 0.00 |  | 134.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 180.18 | 133.07 | 0.00 |  | 0.00 |  | 133.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 179.36 | 132.24 | 0.00 |  | 0.00 |  | 132.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 179.10 | 131.99 | 0.00 |  | 0.00 |  | 131.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 176.12 | 129.00 | 0.00 |  | 0.00 |  | 129.00 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-12/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 571907136 | 476.59 | 4765893 | 4745893 | 0.0 | 36.0 | false | 0 | 0 | 257.7 | 150 / 4 / 1 | 0 | 0 |

#### Stalls: r1-base-12/pp-epoll-spin-10000, loop 0 on CPU 3

569507136 iterations, longest 257.7 µs; over 5 / 20 / 100 µs: 150 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 257.7 | 257.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.1 | 31.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 44.0 | 44.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 257.7 | 0 | softirq TIMER | softirq TIMER 216, local timer tick 13, timer tick_nohz_handler 8, irq work (IPI/vector) 2 |
| 2 | 44.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 31.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 23.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-12/pp-epoll-spin-10000 (median 47.75)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 982.30 | 934.55 | 0.00 |  | 0.00 |  | 934.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 259.67 | 211.92 | 257.66 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 173.91 | 126.16 | 0.00 |  | 0.00 |  | 126.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 167.34 | 119.59 | 0.00 |  | 120.32 | softirq TIMER | 0.00 | server: softirq TIMER |
| 5 | 167.20 | 119.45 | 0.00 |  | 0.00 |  | 119.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 164.41 | 116.66 | 0.00 |  | 0.00 |  | 116.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 160.90 | 113.15 | 0.00 |  | 0.00 |  | 113.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 150.38 | 102.64 | 0.00 |  | 0.00 |  | 102.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 149.18 | 101.43 | 0.00 |  | 0.00 |  | 101.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 148.89 | 101.14 | 0.00 |  | 0.00 |  | 101.14 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-12/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1475297 | 1475297 | 1.00 | 115737954 | 78.45 | 1928966 | 1904378 | 98.7 | 70.5 | true | 0 | 2263 | 34.5 | 7110 / 524 / 0 | 0 | 0 |

#### Stalls: r1-base-12/pp-recv-busy-0, loop 0 on CPU 3

114262656 iterations, longest 34.5 µs; over 5 / 20 / 100 µs: 7110 / 524 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 327 | 7414.2 | 34.5 |
| mostly unexplained (+softirq NET_RX) | 103 | 2406.6 | 30.5 |
| softirq NET_RX | 83 | 1795.5 | 32.5 |
| mostly unexplained (+local timer tick) | 4 | 102.0 | 27.2 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 85.4 | 33.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 73.2 | 26.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 34.0 | 34.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 34.5 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 5, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 2 | 34.0 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 33.9 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 32.5 | 0 | softirq NET_RX | softirq NET_RX 19, local timer tick 7, preempted by ksoftirqd/3 7, timer tick_nohz_handler 4 |
| 5 | 32.4 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 12, workqueue [ena] 8, local timer tick 5, timer tick_nohz_handler 2 |
| 6 | 30.5 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 12, preempted by kworker/3:1 9, local timer tick 4, irq work (IPI/vector) 1 |
| 7 | 30.0 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 8 | 28.4 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 28.2 | 0 | softirq NET_RX | softirq NET_RX 20, local timer tick 6, preempted by kworker/3:1 4, timer tick_nohz_handler 4 |
| 10 | 27.8 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2317 |
| LOC Local timer interrupts | 2308 |
| CAL Function call interrupts | 988 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-12/pp-recv-busy-0 (median 40.35)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 161.84 | 121.49 | 0.00 |  | 133.78 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 128.15 | 87.80 | 0.00 |  | 0.00 |  | 87.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 125.21 | 84.86 | 0.00 |  | 0.00 |  | 84.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 118.79 | 78.44 | 0.00 |  | 0.00 |  | 78.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 116.24 | 75.89 | 0.00 |  | 0.00 |  | 75.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 114.58 | 74.22 | 0.00 |  | 0.00 |  | 74.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 114.03 | 73.68 | 0.00 |  | 0.00 |  | 73.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 113.92 | 73.56 | 0.00 |  | 0.00 |  | 73.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 111.51 | 71.16 | 0.00 |  | 74.84 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 111.39 | 71.04 | 0.00 |  | 0.00 |  | 71.04 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-12/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 236146880 | 196.79 | 1967891 | 1957891 | 99.5 | 67.4 | true | 0 | 2110 | 271.1 | 20704 / 888 / 1 | 0 | 0 |

#### Stalls: r1-base-12/pp-recv-busy-10000, loop 0 on CPU 3

234946880 iterations, longest 271.1 µs; over 5 / 20 / 100 µs: 20704 / 888 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 526 | 12800.5 | 31.2 |
| mostly unexplained (+softirq NET_RX) | 286 | 7166.3 | 32.2 |
| softirq NET_RX | 64 | 1406.9 | 27.3 |
| softirq TIMER | 1 | 271.1 | 271.1 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 114.2 | 31.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 86.9 | 23.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 75.9 | 29.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 271.1 | 0 | softirq TIMER | softirq TIMER 256, local timer tick 11, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 32.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 5 |
| 3 | 31.9 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 31.2 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 30.6 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 29.8 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 29.2 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 29.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 29.1 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 29.1 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2120 |
| LOC Local timer interrupts | 2114 |
| CAL Function call interrupts | 999 |
| MCP Machine check polls | 1 |

#### Slowest round trips: r1-base-12/pp-recv-busy-10000 (median 41.04)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 159.90 | 118.86 | 0.00 |  | 129.99 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 159.43 | 118.40 | 0.00 |  | 0.00 |  | 118.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 153.62 | 112.59 | 0.00 |  | 112.61 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 137.86 | 96.83 | 0.00 |  | 157.45 | softirq TIMER | 0.00 | server: softirq TIMER |
| 5 | 131.10 | 90.06 | 0.00 |  | 0.00 |  | 90.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 129.03 | 88.00 | 0.00 |  | 0.00 |  | 88.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 124.80 | 83.76 | 0.00 |  | 0.00 |  | 83.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 118.18 | 77.15 | 0.00 |  | 0.00 |  | 77.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 116.08 | 75.04 | 0.00 |  | 0.00 |  | 75.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 114.70 | 73.66 | 0.00 |  | 0.00 |  | 73.66 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-12/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1236221 | 1236221 | 1.00 | 3708664 | 3.00 | 61811 | 46829012 | 0.0 | 7.9 | false | 2 | 0 | 425.4 | 1172 / 5 / 2 | 0 | 0 |

#### Stalls: r1-base-12/pp-uring-spin-0, loop 0 on CPU 3

2809740736 iterations, longest 425.4 µs; over 5 / 20 / 100 µs: 1172 / 5 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| preempted by swapper/3 | 2 | 739.3 | 425.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 67.2 | 40.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 29.8 | 29.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 425.4 | 0 | preempted by swapper/3 | preempted by swapper/3 406, page fault 0 |
| 2 | 313.8 | 0 | preempted by swapper/3 | preempted by swapper/3 293, call function (IPI/vector) 1, page fault 0 |
| 3 | 40.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 29.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 26.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |
| TLB TLB shootdowns | 6 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-12/pp-uring-spin-0 (median 47.72)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 197.39 | 149.67 | 0.00 |  | 0.00 |  | 149.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 179.25 | 131.53 | 0.00 |  | 0.00 |  | 131.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 178.05 | 130.33 | 0.00 |  | 0.00 |  | 130.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 177.43 | 129.71 | 0.00 |  | 0.00 |  | 129.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 177.15 | 129.43 | 0.00 |  | 0.00 |  | 129.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 175.27 | 127.55 | 0.00 |  | 0.00 |  | 127.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 171.47 | 123.75 | 0.00 |  | 0.00 |  | 123.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 168.67 | 120.95 | 0.00 |  | 0.00 |  | 120.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 168.29 | 120.57 | 29.79 | VM paused: the idle spare core stalled too (hypervisor) | 0.00 |  | 90.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 167.99 | 120.27 | 0.00 |  | 0.00 |  | 120.27 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-12/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1200000 | 1200000 | 1.00 | 3600000 | 3.00 | 30000 | 35056834 | 0.0 | 4.7 | false | 0 | 0 | 40.3 | 576 / 3 / 0 | 0 | 0 |

#### Stalls: r1-base-12/pp-uring-spin-10000, loop 0 on CPU 3

4206820032 iterations, longest 40.3 µs; over 5 / 20 / 100 µs: 576 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 70.2 | 40.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 22.6 | 22.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 40.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 29.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 22.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1002 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-12/pp-uring-spin-10000 (median 47.97)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 971.95 | 923.98 | 0.00 |  | 0.00 |  | 923.98 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 167.67 | 119.70 | 0.00 |  | 0.00 |  | 119.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 162.27 | 114.30 | 0.00 |  | 0.00 |  | 114.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 157.00 | 109.03 | 0.00 |  | 0.00 |  | 109.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 149.12 | 101.15 | 0.00 |  | 0.00 |  | 101.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 138.41 | 90.44 | 0.00 |  | 0.00 |  | 90.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 136.76 | 88.78 | 0.00 |  | 0.00 |  | 88.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 130.19 | 82.21 | 0.00 |  | 0.00 |  | 82.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 129.55 | 81.58 | 0.00 |  | 0.00 |  | 81.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 127.05 | 79.07 | 0.00 |  | 0.00 |  | 79.07 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-12/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1445541 | 1445541 | 1.00 | 1 | 0.00 | 0 | 35513888 | 0.0 | 0.0 | true | 0 | 0 | 390.6 | 61 / 4 / 1 | 0 | 0 |

#### Stalls: r1-base-12/pp-uring-sqpoll-0, loop 0 on CPU 3

2130833280 iterations, longest 390.6 µs; over 5 / 20 / 100 µs: 61 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 390.6 | 390.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.0 | 34.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 22.4 | 22.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 390.6 | 1 | softirq TIMER | softirq TIMER 373, local timer tick 10, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 34.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 22.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 20.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: r1-base-12/pp-uring-sqpoll-0 (median 41.02)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 861.59 | 820.57 | 0.00 |  | 23.30 | softirq TIMER | 797.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 415.89 | 374.87 | 389.93 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 191.89 | 150.87 | 0.00 |  | 154.03 | softirq TIMER | 0.00 | server: softirq TIMER |
| 4 | 151.09 | 110.07 | 0.00 |  | 0.00 |  | 110.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 134.46 | 93.44 | 0.00 |  | 0.00 |  | 93.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 120.65 | 79.63 | 0.00 |  | 0.00 |  | 79.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 118.55 | 77.53 | 0.00 |  | 0.00 |  | 77.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 117.55 | 76.53 | 0.00 |  | 0.00 |  | 76.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 114.97 | 73.95 | 0.00 |  | 0.00 |  | 73.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 111.58 | 70.56 | 0.00 |  | 0.00 |  | 70.56 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (r1-base-12/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 24030013 | 0.0 | 0.0 | true | 0 | 0 | 31.3 | 148 / 1 / 0 | 0 | 0 |

#### Stalls: r1-base-12/pp-uring-sqpoll-10000, loop 0 on CPU 3

2883601536 iterations, longest 31.3 µs; over 5 / 20 / 100 µs: 148 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 31.3 | 31.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 31.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1007 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

#### Slowest round trips: r1-base-12/pp-uring-sqpoll-10000 (median 41.98)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 127.09 | 85.11 | 0.00 |  | 0.00 |  | 85.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 127.05 | 85.06 | 0.00 |  | 0.00 |  | 85.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 121.10 | 79.12 | 0.00 |  | 0.00 |  | 79.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 120.16 | 78.17 | 0.00 |  | 144.27 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 119.85 | 77.87 | 0.00 |  | 0.00 |  | 77.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 117.10 | 75.12 | 0.00 |  | 0.00 |  | 75.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 114.03 | 72.05 | 0.00 |  | 0.00 |  | 72.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 112.83 | 70.85 | 0.00 |  | 0.00 |  | 70.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 112.19 | 70.21 | 0.00 |  | 0.00 |  | 70.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 110.73 | 68.75 | 0.00 |  | 0.00 |  | 68.75 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: boot-stock/http-recv-busy-10000, loop 0 on CPU 3

2102393772 iterations, longest 66.8 µs; over 5 / 20 / 100 µs: 146554 / 3221 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2803 | 127640.6 | 61.7 |
| mostly unexplained (+softirq NET_RX) | 235 | 10907.7 | 59.6 |
| mostly unexplained (+local timer tick) | 136 | 5670.8 | 62.9 |
| mostly unexplained (+preempted by kworker/3:1) | 18 | 708.9 | 66.8 |
| softirq NET_RX | 16 | 548.0 | 49.1 |
| mostly unexplained (+softirq SCHED) | 3 | 135.4 | 47.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 95.8 | 43.1 |
| mostly unexplained (+softirq RCU) | 1 | 48.9 | 48.9 |
| loop work | 2 | 47.7 | 24.6 |
| mostly unexplained (+preempted by kworker/3:1H) | 1 | 47.0 | 47.0 |
| local timer tick | 2 | 41.9 | 21.7 |
| preempted by kworker/3:1H | 1 | 20.4 | 20.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 66.8 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, workqueue [ena] 1 |
| 2 | 62.9 | 2 | mostly unexplained (+local timer tick) | local timer tick 1, softirq NET_RX 1 |
| 3 | 62.6 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 2, workqueue [ena] 1 |
| 4 | 61.7 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 61.4 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 5, workqueue [ena] 1 |
| 6 | 60.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 59.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 59.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 59.2 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, timer tick_nohz_handler 4, softirq SCHED 1, ipi_send_cpu 0 |
| 10 | 58.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120487 |
| CAL Function call interrupts | 513 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 220 |
| IWI IRQ work interrupts | 10 |
| TLB TLB shootdowns | 2 |

#### Slowest round trips: boot-stock/http-recv-busy-10000 (median 40.85)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 132.54 | 91.69 | 0.00 |  | 0.00 |  | 91.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 131.69 | 90.84 | 0.00 |  | 0.00 |  | 90.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 128.82 | 87.97 | 0.00 |  | 0.00 |  | 87.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 127.15 | 86.30 | 0.00 |  | 0.00 |  | 86.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 124.88 | 84.03 | 0.00 |  | 0.00 |  | 84.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 124.56 | 83.72 | 0.00 |  | 151.51 | softirq TIMER | 0.00 | server: softirq TIMER |
| 7 | 122.02 | 81.18 | 0.00 |  | 0.00 |  | 81.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 120.36 | 79.52 | 0.00 |  | 0.00 |  | 79.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 119.67 | 78.82 | 0.00 |  | 0.00 |  | 78.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 119.50 | 78.66 | 0.00 |  | 0.00 |  | 78.66 | neither loop stalled long enough: network, hypervisor or untraced |

## boot-stock/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=605444 clocksource=tsc

### Loop counters (boot-stock/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002550 | 689494 | 4.35 | 135468416 | 45.12 | 2257807 | 2257807 | 99.5 | 72.5 | true | 0 | 247 | 512.6 | 83948 / 5787 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672571200 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (120026 of 2900000 in 1-msg reads) | 120026 | 1.92 | 1.90 | 2.80 | 3.52 | 5.48 | 15.52 | 37.06 |

#### Stalls: boot-stock/md-recv-busy, loop 0 on CPU 3

135468416 iterations, longest 512.6 µs; over 5 / 20 / 100 µs: 83948 / 5787 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5557 | 148695.8 | 512.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 165 | 3922.8 | 53.4 |
| mostly unexplained (+preempted by kworker/3:1) | 33 | 783.7 | 35.0 |
| mostly unexplained (+local timer tick) | 18 | 424.2 | 26.9 |
| softirq TIMER | 1 | 246.6 | 246.6 |
| preempted by kworker/3:1 | 5 | 237.3 | 59.6 |
| mostly unexplained (+softirq NET_RX) | 6 | 156.4 | 47.0 |
| mostly unexplained (+softirq TIMER) | 1 | 32.0 | 32.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 24.1 | 24.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 512.6 | 2550 | loop work (2550 items) | local timer tick 8, timer tick_nohz_handler 5 |
| 2 | 246.6 | 0 | softirq TIMER | softirq TIMER 235, local timer tick 7, timer tick_nohz_handler 5, ipi_send_cpu 0 |
| 3 | 66.7 | 34 | loop work (34 items) |  |
| 4 | 64.7 | 18 | loop work (18 items) |  |
| 5 | 59.6 | 0 | preempted by kworker/3:1 | preempted by kworker/3:1 39, workqueue vmstat_update 36, local timer tick 6, softirq TIMER 5 |
| 6 | 59.6 | 13 | loop work (13 items) |  |
| 7 | 58.1 | 35 | loop work (35 items) |  |
| 8 | 54.3 | 19 | loop work (19 items) |  |
| 9 | 53.9 | 32 | loop work (32 items) | softirq NET_RX 6 |
| 10 | 53.4 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60338 |
| CAL Function call interrupts | 518 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 45 |
| IWI IRQ work interrupts | 10 |
| TLB TLB shootdowns | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## boot-stock/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: boot-stock/noise, loop 0 on CPU 3 (no kernel trace: causes limited to loop work)

9171525632 iterations, longest 150.3 µs; over 5 / 20 / 100 µs: 120260 / 324 / 68

| cause | stalls | total µs | max µs |
|---|---|---|---|
| not traced | 120260 | 896542.2 | 150.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 150.3 | 0 | not traced |  |
| 2 | 148.5 | 0 | not traced |  |
| 3 | 148.4 | 0 | not traced |  |
| 4 | 143.1 | 0 | not traced |  |
| 5 | 141.3 | 0 | not traced |  |
| 6 | 141.2 | 0 | not traced |  |
| 7 | 141.1 | 0 | not traced |  |
| 8 | 138.9 | 0 | not traced |  |
| 9 | 137.3 | 0 | not traced |  |
| 10 | 136.8 | 0 | not traced |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120197 |
| CAL Function call interrupts | 999 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 163 |
| TLB TLB shootdowns | 2 |

### Loop counters (boot-stock/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1292897 | 1292897 | 1.00 | 477745475 | 369.52 | 7962425 | 7919328 | 0.0 | 49.0 | false | 0 | 92 | 65.8 | 60972 / 86 / 0 | 0 | 0 |

#### Stalls: boot-stock/pp-epoll-spin-0, loop 0 on CPU 3

475159680 iterations, longest 65.8 µs; over 5 / 20 / 100 µs: 60972 / 86 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 33 | 875.8 | 33.4 |
| mostly unexplained (+local timer tick) | 35 | 827.0 | 27.6 |
| preempted by kworker/3:1 | 4 | 237.0 | 65.8 |
| softirq NET_RX | 6 | 187.5 | 42.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 110.1 | 33.0 |
| softirq SCHED | 1 | 32.5 | 32.5 |
| mostly unexplained (+preempted by kworker/3:1H) | 1 | 26.8 | 26.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 22.7 | 22.7 |
| preempted by kworker/3:1H | 1 | 21.6 | 21.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 65.8 | 1 | preempted by kworker/3:1 | preempted by kworker/3:1 43, workqueue vmstat_update 37, softirq TIMER 7, local timer tick 6 |
| 2 | 60.5 | 0 | preempted by kworker/3:1 | preempted by kworker/3:1 42, workqueue vmstat_update 36, local timer tick 7, timer tick_nohz_handler 5 |
| 3 | 57.2 | 0 | preempted by kworker/3:1 | preempted by kworker/3:1 40, workqueue vmstat_update 36, local timer tick 6, softirq TIMER 5 |
| 4 | 53.6 | 1 | preempted by kworker/3:1 | preempted by kworker/3:1 35, workqueue vmstat_update 32, local timer tick 6, timer tick_nohz_handler 4 |
| 5 | 42.7 | 1 | softirq NET_RX | softirq NET_RX 34, irq enp39s0-Tx-Rx-7 1 |
| 6 | 42.4 | 0 | softirq NET_RX | softirq NET_RX 35, irq enp39s0-Tx-Rx-7 1 |
| 7 | 33.4 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 10, local timer tick 6, softirq TIMER 6, timer tick_nohz_handler 4 |
| 8 | 33.3 | 0 | softirq NET_RX | softirq NET_RX 30, irq enp39s0-Tx-Rx-7 1 |
| 9 | 33.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 32.5 | 0 | softirq SCHED | softirq SCHED 22, local timer tick 7, timer tick_nohz_handler 5 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60960 |
| CAL Function call interrupts | 999 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 71 |
| TLB TLB shootdowns | 2 |

#### Slowest round trips: boot-stock/pp-epoll-spin-0 (median 46.02)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 239.55 | 193.53 | 0.00 |  | 0.00 |  | 193.53 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 226.25 | 180.23 | 25.61 | mostly unexplained (+preempted by kworker/3:1) | 0.00 |  | 154.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 193.70 | 147.68 | 0.00 |  | 0.00 |  | 147.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 191.90 | 145.88 | 0.00 |  | 0.00 |  | 145.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 184.80 | 138.78 | 0.00 |  | 0.00 |  | 138.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 184.40 | 138.38 | 0.00 |  | 0.00 |  | 138.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 184.03 | 138.01 | 0.00 |  | 0.00 |  | 138.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 183.09 | 137.07 | 0.00 |  | 0.00 |  | 137.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 182.16 | 136.15 | 0.00 |  | 0.00 |  | 136.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 178.87 | 132.85 | 0.00 |  | 0.00 |  | 132.85 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 954038528 | 795.03 | 7950321 | 7930321 | 0.0 | 39.4 | false | 0 | 150 | 58.4 | 120603 / 94 / 0 | 0 | 0 |

#### Stalls: boot-stock/pp-epoll-spin-10000, loop 0 on CPU 3

951638528 iterations, longest 58.4 µs; over 5 / 20 / 100 µs: 120603 / 94 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 57 | 1471.8 | 32.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 15 | 383.9 | 31.2 |
| softirq NET_RX | 10 | 347.1 | 50.2 |
| mostly unexplained (+local timer tick) | 7 | 164.6 | 25.8 |
| softirq TIMER | 2 | 81.4 | 58.4 |
| local timer tick | 3 | 74.8 | 25.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 58.4 | 0 | softirq TIMER | softirq TIMER 49, local timer tick 7, timer tick_nohz_handler 5 |
| 2 | 50.2 | 0 | softirq NET_RX | softirq NET_RX 41, irq enp39s0-Tx-Rx-7 1 |
| 3 | 39.2 | 0 | softirq NET_RX | softirq NET_RX 28, local timer tick 7, timer tick_nohz_handler 5, irq enp39s0-Tx-Rx-7 1 |
| 4 | 38.6 | 0 | softirq NET_RX | softirq NET_RX 31, irq enp39s0-Tx-Rx-7 1 |
| 5 | 37.9 | 0 | softirq NET_RX | softirq NET_RX 32, irq enp39s0-Tx-Rx-7 1 |
| 6 | 37.1 | 1 | softirq NET_RX | softirq NET_RX 25, irq enp39s0-Tx-Rx-7 1 |
| 7 | 33.7 | 0 | softirq NET_RX | softirq NET_RX 25, irq enp39s0-Tx-Rx-7 1 |
| 8 | 32.8 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 13, softirq TIMER 7, local timer tick 5, timer tick_nohz_handler 4 |
| 9 | 32.7 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 12, softirq TIMER 7, local timer tick 6, timer tick_nohz_handler 4 |
| 10 | 31.3 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 12, local timer tick 6, softirq TIMER 5, timer tick_nohz_handler 4 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120326 |
| CAL Function call interrupts | 1013 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 142 |
| TLB TLB shootdowns | 6 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-stock/pp-epoll-spin-10000 (median 47.38)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 4651.53 | 4604.15 | 0.00 |  | 0.00 |  | 4604.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 446.71 | 399.32 | 0.00 |  | 0.00 |  | 399.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 445.91 | 398.52 | 0.00 |  | 0.00 |  | 398.52 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 445.15 | 397.77 | 0.00 |  | 0.00 |  | 397.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 443.88 | 396.50 | 0.00 |  | 0.00 |  | 396.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 443.34 | 395.95 | 0.00 |  | 0.00 |  | 395.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 438.85 | 391.47 | 0.00 |  | 0.00 |  | 391.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 437.64 | 390.26 | 0.00 |  | 0.00 |  | 390.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 437.07 | 389.69 | 0.00 |  | 0.00 |  | 389.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 436.30 | 388.92 | 0.00 |  | 0.00 |  | 388.92 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1447922 | 1447922 | 1.00 | 124599923 | 86.05 | 2076665 | 2052533 | 98.8 | 80.8 | true | 0 | 9806 | 164.9 | 123787 / 395 / 26 | 0 | 0 |

#### Stalls: boot-stock/pp-recv-busy-0, loop 0 on CPU 3

123152000 iterations, longest 164.9 µs; over 5 / 20 / 100 µs: 123787 / 395 / 26

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 277 | 9215.8 | 164.9 |
| mostly unexplained (+local timer tick) | 57 | 1442.5 | 47.9 |
| softirq TIMER | 7 | 938.4 | 147.7 |
| mostly unexplained (+irq enp39s0-Tx-Rx-7) | 18 | 509.9 | 37.8 |
| local timer tick | 12 | 296.3 | 44.4 |
| mostly unexplained (+preempted by kworker/3:1) | 10 | 242.4 | 30.3 |
| preempted by kworker/3:1 | 3 | 123.4 | 45.4 |
| mostly unexplained (+softirq NET_RX) | 3 | 70.3 | 24.6 |
| irq enp39s0-Tx-Rx-7 | 2 | 60.6 | 31.2 |
| softirq NET_RX | 2 | 53.8 | 30.6 |
| mostly unexplained (+preempted by kworker/3:1H) | 1 | 25.0 | 25.0 |
| mostly unexplained (+softirq TIMER) | 1 | 23.9 | 23.9 |
| mostly unexplained (+softirq SCHED) | 1 | 23.4 | 23.4 |
| softirq SCHED | 1 | 23.3 | 23.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 164.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 150.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 149.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 148.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 147.7 | 1 | softirq TIMER | softirq TIMER 125, local timer tick 7, preempted by kworker/3:1 6, timer tick_nohz_handler 5 |
| 6 | 147.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 146.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 144.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 144.3 | 1 | softirq TIMER | softirq TIMER 125, local timer tick 7, preempted by kworker/3:1 6, timer tick_nohz_handler 5 |
| 10 | 142.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 1234545 |
| LOC Local timer interrupts | 60484 |
| CAL Function call interrupts | 514 |
| TLB TLB shootdowns | 3 |
| IWI IRQ work interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-stock/pp-recv-busy-0 (median 40.68)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 5225.32 | 5184.64 | 47.71 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 5136.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 203.46 | 162.78 | 164.91 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 3 | 186.97 | 146.29 | 149.77 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 4 | 178.15 | 137.47 | 143.84 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 5 | 177.35 | 136.67 | 141.05 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 6 | 174.13 | 133.46 | 147.41 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 7 | 172.05 | 131.37 | 149.46 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 172.00 | 131.32 | 141.57 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 168.88 | 128.21 | 141.44 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 10 | 168.09 | 127.41 | 140.74 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (boot-stock/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 289378432 | 241.15 | 2411487 | 2401487 | 99.6 | 72.5 | true | 0 | 2739 | 41.7 | 134854 / 47 / 0 | 0 | 0 |

#### Stalls: boot-stock/pp-recv-busy-10000, loop 0 on CPU 3

288178432 iterations, longest 41.7 µs; over 5 / 20 / 100 µs: 134854 / 47 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq NET_RX | 16 | 481.3 | 41.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 12 | 302.2 | 28.0 |
| mostly unexplained (+preempted by kworker/3:1) | 8 | 173.3 | 25.4 |
| mostly unexplained (+local timer tick) | 5 | 124.0 | 25.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 114.9 | 41.2 |
| local timer tick | 2 | 50.2 | 25.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 41.7 | 0 | softirq NET_RX | softirq NET_RX 26, local timer tick 9, timer tick_nohz_handler 8, irq enp39s0-Tx-Rx-7 1 |
| 2 | 41.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 37.5 | 0 | softirq NET_RX | softirq NET_RX 32, irq enp39s0-Tx-Rx-7 1 |
| 4 | 37.2 | 0 | softirq NET_RX | softirq NET_RX 24, local timer tick 5, timer tick_nohz_handler 3, irq enp39s0-Tx-Rx-7 1 |
| 5 | 34.3 | 0 | softirq NET_RX | softirq NET_RX 26, irq enp39s0-Tx-Rx-7 1 |
| 6 | 33.8 | 1 | softirq NET_RX | softirq NET_RX 25, irq enp39s0-Tx-Rx-7 1 |
| 7 | 33.0 | 0 | softirq NET_RX | softirq NET_RX 25, irq enp39s0-Tx-Rx-7 1 |
| 8 | 32.3 | 0 | softirq NET_RX | softirq NET_RX 25, irq enp39s0-Tx-Rx-7 1 |
| 9 | 31.3 | 1 | softirq NET_RX | softirq NET_RX 24, irq enp39s0-Tx-Rx-7 1 |
| 10 | 30.1 | 0 | softirq NET_RX | softirq NET_RX 24, irq enp39s0-Tx-Rx-7 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 121937 |
| CAL Function call interrupts | 1016 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 207 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: boot-stock/pp-recv-busy-10000 (median 40.00)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 177.47 | 137.48 | 0.00 |  | 165.41 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 118.23 | 78.23 | 0.00 |  | 0.00 |  | 78.23 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 112.03 | 72.03 | 0.00 |  | 0.00 |  | 72.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 111.36 | 71.36 | 0.00 |  | 0.00 |  | 71.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 110.96 | 70.96 | 0.00 |  | 0.00 |  | 70.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 107.81 | 67.81 | 0.00 |  | 0.00 |  | 67.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 107.19 | 67.19 | 0.00 |  | 0.00 |  | 67.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 107.12 | 67.12 | 0.00 |  | 0.00 |  | 67.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 106.77 | 66.77 | 0.00 |  | 0.00 |  | 66.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 105.99 | 66.00 | 0.00 |  | 0.00 |  | 66.00 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1233370 | 1233370 | 1.00 | 3700111 | 3.00 | 61669 | 43462658 | 0.0 | 6.2 | false | 0 | 46 | 70.7 | 62882 / 43 / 0 | 0 | 0 |

#### Stalls: boot-stock/pp-uring-spin-0, loop 0 on CPU 3

2607759488 iterations, longest 70.7 µs; over 5 / 20 / 100 µs: 62882 / 43 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 36 | 1062.8 | 70.7 |
| softirq NET_RX | 3 | 79.7 | 33.4 |
| local timer tick | 2 | 57.8 | 29.5 |
| mostly unexplained (+preempted by kworker/3:1) | 1 | 35.8 | 35.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 25.1 | 25.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 70.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 56.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 55.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 47.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 45.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 38.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 35.8 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 11, softirq TIMER 6, local timer tick 5, timer tick_nohz_handler 3 |
| 8 | 35.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 33.4 | 0 | softirq NET_RX | softirq NET_RX 31, irq enp39s0-Tx-Rx-7 1 |
| 10 | 32.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 1233404 |
| LOC Local timer interrupts | 60376 |
| CAL Function call interrupts | 993 |
| TLB TLB shootdowns | 2 |

#### Slowest round trips: boot-stock/pp-uring-spin-0 (median 48.33)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 183.16 | 134.82 | 0.00 |  | 0.00 |  | 134.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 168.62 | 120.29 | 25.14 | VM paused: the idle spare core stalled too (hypervisor) | 0.00 |  | 95.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 128.81 | 80.48 | 0.00 |  | 0.00 |  | 80.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 118.00 | 69.67 | 0.00 |  | 0.00 |  | 69.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 116.91 | 68.57 | 0.00 |  | 0.00 |  | 68.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 115.32 | 66.99 | 0.00 |  | 0.00 |  | 66.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 113.96 | 65.62 | 0.00 |  | 108.24 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 113.78 | 65.45 | 0.00 |  | 0.00 |  | 65.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 111.96 | 63.62 | 0.00 |  | 0.00 |  | 63.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 111.63 | 63.29 | 0.00 |  | 0.00 |  | 63.29 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1200000 | 1200000 | 1.00 | 3600000 | 3.00 | 30000 | 32354987 | 0.0 | 31.0 | false | 0 | 9465 | 239.9 | 185314 / 68 / 1 | 0 | 0 |

#### Stalls: boot-stock/pp-uring-spin-10000, loop 0 on CPU 3

3882598400 iterations, longest 239.9 µs; over 5 / 20 / 100 µs: 185314 / 68 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 46 | 1358.7 | 239.9 |
| local timer tick | 5 | 132.8 | 27.7 |
| mostly unexplained (+softirq NET_RX) | 4 | 92.0 | 26.4 |
| mostly unexplained (+preempted by kworker/3:1) | 4 | 87.4 | 22.3 |
| mostly unexplained (+local timer tick) | 4 | 81.9 | 20.8 |
| softirq NET_RX | 3 | 78.2 | 31.8 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 46.5 | 24.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 239.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 35.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 33.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 31.8 | 0 | softirq NET_RX | softirq NET_RX 29, irq enp39s0-Tx-Rx-7 1 |
| 5 | 30.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 30.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 29.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 29.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 28.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 28.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 1200122 |
| LOC Local timer interrupts | 120420 |
| CAL Function call interrupts | 516 |
| IWI IRQ work interrupts | 10 |
| TLB TLB shootdowns | 2 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-stock/pp-uring-spin-10000 (median 43.40)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 281.77 | 238.38 | 239.95 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 2 | 132.87 | 89.47 | 0.00 |  | 0.00 |  | 89.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 131.18 | 87.78 | 0.00 |  | 0.00 |  | 87.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 126.70 | 83.30 | 0.00 |  | 0.00 |  | 83.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 125.34 | 81.94 | 0.00 |  | 0.00 |  | 81.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 117.41 | 74.01 | 0.00 |  | 0.00 |  | 74.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 112.42 | 69.02 | 0.00 |  | 0.00 |  | 69.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 108.88 | 65.48 | 0.00 |  | 0.00 |  | 65.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 105.47 | 62.07 | 0.00 |  | 0.00 |  | 62.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 103.35 | 59.95 | 0.00 |  | 0.00 |  | 59.95 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1418949 | 1418949 | 1.00 | 1 | 0.00 | 0 | 18002060 | 0.0 | 3.0 | true | 0 | 4390 | 42.2 | 66707 / 47 / 0 | 0 | 0 |

#### Stalls: boot-stock/pp-uring-sqpoll-0, loop 0 on CPU 3

1080123584 iterations, longest 42.2 µs; over 5 / 20 / 100 µs: 66707 / 47 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 25 | 611.7 | 42.2 |
| softirq NET_RX | 9 | 218.8 | 27.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 8 | 209.6 | 39.6 |
| mostly unexplained (+local timer tick) | 2 | 54.8 | 28.9 |
| mostly unexplained (+softirq NET_RX) | 2 | 40.6 | 20.4 |
| local timer tick | 1 | 21.1 | 21.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 42.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 39.6 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 30.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 29.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 28.9 | 0 | mostly unexplained (+local timer tick) | local timer tick 7, timer tick_nohz_handler 6, irq enp39s0-Tx-Rx-7 1, softirq NET_RX 1 |
| 6 | 28.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 27.6 | 0 | softirq NET_RX | softirq NET_RX 18, local timer tick 6, timer tick_nohz_handler 4, irq enp39s0-Tx-Rx-7 1 |
| 8 | 27.3 | 0 | softirq NET_RX | softirq NET_RX 18, local timer tick 6, timer tick_nohz_handler 4, irq enp39s0-Tx-Rx-7 1 |
| 9 | 27.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 27.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 1419017 |
| LOC Local timer interrupts | 60393 |
| CAL Function call interrupts | 995 |
| TLB TLB shootdowns | 4 |
| IWI IRQ work interrupts | 2 |

#### Slowest round trips: boot-stock/pp-uring-sqpoll-0 (median 41.72)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 130.21 | 88.50 | 0.00 |  | 0.00 |  | 88.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 127.68 | 85.96 | 0.00 |  | 0.00 |  | 85.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 121.19 | 79.47 | 0.00 |  | 0.00 |  | 79.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 114.37 | 72.66 | 0.00 |  | 0.00 |  | 72.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 112.81 | 71.10 | 0.00 |  | 0.00 |  | 71.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 111.16 | 69.44 | 0.00 |  | 0.00 |  | 69.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 109.77 | 68.05 | 0.00 |  | 0.00 |  | 68.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 109.52 | 67.81 | 0.00 |  | 0.00 |  | 67.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 106.40 | 64.68 | 0.00 |  | 0.00 |  | 64.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 104.11 | 62.40 | 0.00 |  | 0.00 |  | 62.40 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-stock/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 12902013 | 0.0 | 0.0 | true | 0 | 86 | 52.1 | 120236 / 69 / 0 | 0 | 0 |

#### Stalls: boot-stock/pp-uring-sqpoll-10000, loop 0 on CPU 3

1548241536 iterations, longest 52.1 µs; over 5 / 20 / 100 µs: 120236 / 69 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+local timer tick) | 34 | 797.0 | 27.4 |
| mostly unexplained (+preempted by kworker/3:1) | 10 | 246.3 | 28.9 |
| local timer tick | 7 | 195.6 | 45.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 7 | 161.5 | 27.0 |
| mostly unexplained (+preempted by kworker/3:1H) | 3 | 82.2 | 30.7 |
| preempted by kworker/3:1 | 2 | 81.6 | 52.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 52.5 | 28.3 |
| softirq SCHED | 1 | 32.7 | 32.7 |
| mostly unexplained (+softirq TIMER) | 1 | 27.6 | 27.6 |
| preempted by kworker/3:1H | 1 | 25.5 | 25.5 |
| softirq NET_RX | 1 | 22.1 | 22.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 52.1 | 1 | preempted by kworker/3:1 | preempted by kworker/3:1 34, workqueue vmstat_update 31, local timer tick 6, softirq TIMER 5 |
| 2 | 45.5 | 1 | local timer tick | local timer tick 41, timer tick_nohz_handler 6, softirq RCU 1 |
| 3 | 32.7 | 0 | softirq SCHED | softirq SCHED 22, local timer tick 7, timer tick_nohz_handler 4 |
| 4 | 30.7 | 1 | mostly unexplained (+preempted by kworker/3:1H) | preempted by kworker/3:1H 15, workqueue blk_mq_run_work_fn 11, local timer tick 7, timer tick_nohz_handler 5 |
| 5 | 29.5 | 0 | preempted by kworker/3:1 | preempted by kworker/3:1 15, workqueue vmstat_update 13, local timer tick 6, timer tick_nohz_handler 4 |
| 6 | 28.9 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 11, softirq TIMER 6, workqueue psi_avgs_work 6, local timer tick 5 |
| 7 | 28.9 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 9, local timer tick 7, timer tick_nohz_handler 5, workqueue psi_avgs_work 5 |
| 8 | 28.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 9 | 27.6 | 1 | mostly unexplained (+softirq TIMER) | softirq TIMER 9, local timer tick 6, preempted by kworker/3:1 5, timer tick_nohz_handler 4 |
| 10 | 27.5 | 0 | local timer tick | local timer tick 24, timer tick_nohz_handler 23 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120440 |
| CAL Function call interrupts | 515 |
| 74 PCI-MSIX-0000:27:00.0 8-edge enp39s0-Tx-Rx-7 | 91 |
| IWI IRQ work interrupts | 10 |
| TLB TLB shootdowns | 6 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-stock/pp-uring-sqpoll-10000 (median 40.72)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1625.01 | 1584.29 | 0.00 |  | 0.00 |  | 1584.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 142.00 | 101.28 | 0.00 |  | 0.00 |  | 101.28 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 130.37 | 89.65 | 0.00 |  | 0.00 |  | 89.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 125.77 | 85.05 | 0.00 |  | 0.00 |  | 85.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 118.81 | 78.08 | 0.00 |  | 0.00 |  | 78.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 114.80 | 74.08 | 0.00 |  | 0.00 |  | 74.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 112.57 | 71.85 | 0.00 |  | 0.00 |  | 71.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 112.51 | 71.79 | 0.00 |  | 0.00 |  | 71.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 107.59 | 66.87 | 0.00 |  | 0.00 |  | 66.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 107.02 | 66.29 | 0.00 |  | 0.00 |  | 66.29 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: boot-isolcpus/http-recv-busy-10000, loop 0 on CPU 3

2052810697 iterations, longest 65.2 µs; over 5 / 20 / 100 µs: 163186 / 3127 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2892 | 130789.5 | 61.8 |
| mostly unexplained (+softirq NET_RX) | 191 | 8940.1 | 58.8 |
| mostly unexplained (+local timer tick) | 27 | 1648.0 | 65.2 |
| mostly unexplained (+preempted by kworker/3:1) | 13 | 622.8 | 60.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 65.0 | 36.0 |
| softirq NET_RX | 1 | 55.8 | 55.8 |
| loop work | 1 | 27.8 | 27.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 65.2 | 2 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 4 |
| 2 | 64.7 | 2 | mostly unexplained (+local timer tick) | local timer tick 6, timer tick_nohz_handler 4 |
| 3 | 64.6 | 2 | mostly unexplained (+local timer tick) | local timer tick 6, timer tick_nohz_handler 4 |
| 4 | 63.9 | 2 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 4 |
| 5 | 63.7 | 2 | mostly unexplained (+local timer tick) | local timer tick 6, timer tick_nohz_handler 3 |
| 6 | 63.6 | 2 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 4 |
| 7 | 63.2 | 2 | mostly unexplained (+local timer tick) | local timer tick 6, timer tick_nohz_handler 3 |
| 8 | 62.9 | 2 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 4 |
| 9 | 62.8 | 2 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 4 |
| 10 | 62.8 | 2 | mostly unexplained (+local timer tick) | local timer tick 5, timer tick_nohz_handler 4 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120109 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: boot-isolcpus/http-recv-busy-10000 (median 41.20)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 225.76 | 184.57 | 0.00 |  | 192.47 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 141.71 | 100.51 | 0.00 |  | 0.00 |  | 100.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 137.85 | 96.66 | 0.00 |  | 0.00 |  | 96.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 136.80 | 95.60 | 0.00 |  | 0.00 |  | 95.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 136.08 | 94.88 | 0.00 |  | 0.00 |  | 94.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 136.02 | 94.82 | 0.00 |  | 0.00 |  | 94.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 134.89 | 93.69 | 0.00 |  | 0.00 |  | 93.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 133.66 | 92.46 | 0.00 |  | 0.00 |  | 92.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 130.56 | 89.37 | 0.00 |  | 0.00 |  | 89.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 128.63 | 87.43 | 0.00 |  | 0.00 |  | 87.43 | neither loop stalled long enough: network, hypervisor or untraced |

## boot-isolcpus/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=622428 clocksource=tsc

### Loop counters (boot-isolcpus/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002500 | 703973 | 4.27 | 135572416 | 45.15 | 2259540 | 2259540 | 99.5 | 72.2 | true | 0 | 343 | 530.7 | 130542 / 5942 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672560000 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (107117 of 2900000 in 1-msg reads) | 107117 | 2.40 | 2.14 | 2.81 | 6.08 | 19.46 | 25.61 | 28.67 |

#### Stalls: boot-isolcpus/md-recv-busy, loop 0 on CPU 3

135572416 iterations, longest 530.7 µs; over 5 / 20 / 100 µs: 130542 / 5942 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5774 | 162547.1 | 530.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 153 | 3465.6 | 28.6 |
| mostly unexplained (+softirq NET_RX) | 7 | 163.6 | 26.4 |
| softirq TIMER | 3 | 101.3 | 60.7 |
| softirq NET_RX | 3 | 76.5 | 26.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 50.7 | 50.7 |
| mostly unexplained (+local timer tick) | 1 | 37.7 | 37.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 530.7 | 2500 | loop work (2500 items) | local timer tick 7, timer tick_nohz_handler 5 |
| 2 | 72.3 | 27 | loop work (27 items) | softirq NET_RX 1 |
| 3 | 63.4 | 38 | loop work (38 items) |  |
| 4 | 60.7 | 0 | softirq TIMER | softirq TIMER 50, local timer tick 6, timer tick_nohz_handler 4 |
| 5 | 57.6 | 22 | loop work (22 items) | softirq NET_RX 8 |
| 6 | 54.8 | 26 | loop work (26 items) |  |
| 7 | 50.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 8 | 49.4 | 41 | loop work (41 items) |  |
| 9 | 48.6 | 29 | loop work (29 items) | softirq NET_RX 9 |
| 10 | 47.8 | 14 | loop work (14 items) | softirq NET_RX 11, preempted by kworker/3:1 5, workqueue [ena] 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60090 |
| CAL Function call interrupts | 989 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## boot-isolcpus/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: boot-isolcpus/noise, loop 0 on CPU 3 (no kernel trace: causes limited to loop work)

9175154688 iterations, longest 29.5 µs; over 5 / 20 / 100 µs: 120092 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| not traced | 120092 | 864551.0 | 29.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 29.5 | 0 | not traced |  |
| 2 | 16.4 | 0 | not traced |  |
| 3 | 16.2 | 0 | not traced |  |
| 4 | 15.7 | 0 | not traced |  |
| 5 | 12.2 | 0 | not traced |  |
| 6 | 12.0 | 0 | not traced |  |
| 7 | 10.7 | 0 | not traced |  |
| 8 | 10.4 | 0 | not traced |  |
| 9 | 10.4 | 0 | not traced |  |
| 10 | 10.4 | 0 | not traced |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120077 |
| CAL Function call interrupts | 997 |

### Loop counters (boot-isolcpus/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1274489 | 1274489 | 1.00 | 488412531 | 383.22 | 8140209 | 8097726 | 0.0 | 46.5 | false | 0 | 15 | 28.5 | 61882 / 1 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-epoll-spin-0, loop 0 on CPU 3

485863552 iterations, longest 28.5 µs; over 5 / 20 / 100 µs: 61882 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.5 | 28.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 28.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60229 |
| CAL Function call interrupts | 989 |

#### Slowest round trips: boot-isolcpus/pp-epoll-spin-0 (median 46.63)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 389.90 | 343.26 | 0.00 |  | 0.00 |  | 343.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 150.41 | 103.77 | 0.00 |  | 0.00 |  | 103.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 140.65 | 94.02 | 0.00 |  | 0.00 |  | 94.02 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 134.83 | 88.19 | 0.00 |  | 0.00 |  | 88.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 134.39 | 87.75 | 0.00 |  | 0.00 |  | 87.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 128.89 | 82.26 | 0.00 |  | 0.00 |  | 82.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 123.66 | 77.03 | 0.00 |  | 0.00 |  | 77.03 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 119.20 | 72.57 | 0.00 |  | 0.00 |  | 72.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 118.42 | 71.79 | 0.00 |  | 0.00 |  | 71.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 116.23 | 69.60 | 0.00 |  | 0.00 |  | 69.60 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 918708160 | 765.59 | 7655901 | 7635901 | 0.0 | 37.3 | false | 0 | 30 | 62.6 | 120416 / 3 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-epoll-spin-10000, loop 0 on CPU 3

916308160 iterations, longest 62.6 µs; over 5 / 20 / 100 µs: 120416 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 62.6 | 62.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 47.9 | 27.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 62.6 | 0 | softirq TIMER | softirq TIMER 49, local timer tick 7, timer tick_nohz_handler 4 |
| 2 | 27.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 20.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120173 |
| CAL Function call interrupts | 1002 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-isolcpus/pp-epoll-spin-10000 (median 48.86)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 404.94 | 356.08 | 0.00 |  | 0.00 |  | 356.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 375.17 | 326.30 | 0.00 |  | 0.00 |  | 326.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 346.74 | 297.88 | 0.00 |  | 0.00 |  | 297.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 339.90 | 291.04 | 0.00 |  | 0.00 |  | 291.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 338.55 | 289.68 | 0.00 |  | 0.00 |  | 289.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 337.60 | 288.74 | 0.00 |  | 0.00 |  | 288.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 337.57 | 288.70 | 0.00 |  | 0.00 |  | 288.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 336.80 | 287.94 | 0.00 |  | 0.00 |  | 287.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 332.19 | 283.33 | 0.00 |  | 0.00 |  | 283.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 331.49 | 282.63 | 0.00 |  | 0.00 |  | 282.63 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1500763 | 1500763 | 1.00 | 133125788 | 88.71 | 2218763 | 2193750 | 98.9 | 78.4 | true | 0 | 2488 | 59.7 | 72004 / 19 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-recv-busy-0, loop 0 on CPU 3

131625024 iterations, longest 59.7 µs; over 5 / 20 / 100 µs: 72004 / 19 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+local timer tick) | 13 | 299.5 | 35.4 |
| softirq TIMER | 1 | 59.7 | 59.7 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 50.2 | 29.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 34.8 | 34.8 |
| preempted by kworker/3:1 | 1 | 22.4 | 22.4 |
| mostly unexplained (+preempted by kworker/3:1) | 1 | 20.7 | 20.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 59.7 | 0 | softirq TIMER | softirq TIMER 49, local timer tick 7, timer tick_nohz_handler 4 |
| 2 | 35.4 | 1 | mostly unexplained (+local timer tick) | local timer tick 8, timer tick_nohz_handler 6 |
| 3 | 34.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 29.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 23.7 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, preempted by kworker/3:1 6, timer tick_nohz_handler 4, workqueue [ena] 1 |
| 6 | 23.4 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, preempted by kworker/3:1 6, timer tick_nohz_handler 4, workqueue [ena] 1 |
| 7 | 22.8 | 1 | mostly unexplained (+local timer tick) | local timer tick 6, preempted by kworker/3:1 5, timer tick_nohz_handler 4, softirq NET_RX 3 |
| 8 | 22.5 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, preempted by kworker/3:1 5, timer tick_nohz_handler 4, workqueue [ena] 1 |
| 9 | 22.4 | 1 | preempted by kworker/3:1 | preempted by kworker/3:1 13, local timer tick 6, timer tick_nohz_handler 4, softirq TIMER 1 |
| 10 | 22.3 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, preempted by kworker/3:1 4, timer tick_nohz_handler 4, softirq NET_RX 3 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60235 |
| CAL Function call interrupts | 993 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-isolcpus/pp-recv-busy-0 (median 39.58)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 220.51 | 180.94 | 0.00 |  | 0.00 |  | 180.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 156.50 | 116.92 | 0.00 |  | 0.00 |  | 116.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 153.57 | 114.00 | 0.00 |  | 0.00 |  | 114.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 153.54 | 113.96 | 0.00 |  | 0.00 |  | 113.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 148.12 | 108.55 | 0.00 |  | 0.00 |  | 108.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 146.08 | 106.50 | 0.00 |  | 0.00 |  | 106.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 144.38 | 104.80 | 0.00 |  | 0.00 |  | 104.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 141.52 | 101.95 | 0.00 |  | 0.00 |  | 101.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 141.24 | 101.66 | 0.00 |  | 106.32 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 137.49 | 97.91 | 0.00 |  | 0.00 |  | 97.91 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 279497984 | 232.91 | 2329150 | 2319150 | 99.6 | 72.7 | true | 0 | 2201 | 32.4 | 126954 / 9 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-recv-busy-10000, loop 0 on CPU 3

278297984 iterations, longest 32.4 µs; over 5 / 20 / 100 µs: 126954 / 9 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 4 | 86.1 | 23.1 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 58.3 | 32.4 |
| preempted by kworker/3:1 | 2 | 40.8 | 20.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 25.3 | 25.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 32.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 25.9 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 25.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 23.1 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 4, workqueue [ena] 1 |
| 5 | 22.5 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, local timer tick 5, timer tick_nohz_handler 3, workqueue [ena] 1 |
| 6 | 20.5 | 0 | preempted by kworker/3:1 | preempted by kworker/3:1 11, local timer tick 5, timer tick_nohz_handler 3, softirq TIMER 1 |
| 7 | 20.3 | 0 | preempted by kworker/3:1 | preempted by kworker/3:1 13, local timer tick 5, softirq NET_RX 4, timer tick_nohz_handler 4 |
| 8 | 20.2 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 10, local timer tick 5, softirq NET_RX 4, timer tick_nohz_handler 3 |
| 9 | 20.2 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, local timer tick 5, softirq NET_RX 3, timer tick_nohz_handler 3 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120168 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: boot-isolcpus/pp-recv-busy-10000 (median 41.26)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 128.82 | 87.56 | 0.00 |  | 0.00 |  | 87.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 122.88 | 81.62 | 0.00 |  | 0.00 |  | 81.62 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 122.16 | 80.90 | 0.00 |  | 0.00 |  | 80.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 118.13 | 76.86 | 0.00 |  | 0.00 |  | 76.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 115.58 | 74.31 | 0.00 |  | 0.00 |  | 74.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 114.42 | 73.16 | 0.00 |  | 0.00 |  | 73.16 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 113.31 | 72.05 | 0.00 |  | 0.00 |  | 72.05 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 111.88 | 70.61 | 0.00 |  | 0.00 |  | 70.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 108.61 | 67.34 | 0.00 |  | 0.00 |  | 67.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 108.28 | 67.02 | 0.00 |  | 0.00 |  | 67.02 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1257859 | 1257859 | 1.00 | 3773578 | 3.00 | 62893 | 46365069 | 0.0 | 3.2 | false | 0 | 15 | 30.2 | 60559 / 3 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-uring-spin-0, loop 0 on CPU 3

2781904128 iterations, longest 30.2 µs; over 5 / 20 / 100 µs: 60559 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 48.8 | 28.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 30.2 | 30.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 30.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 28.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 20.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60227 |
| CAL Function call interrupts | 992 |

#### Slowest round trips: boot-isolcpus/pp-uring-spin-0 (median 46.79)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 222.18 | 175.39 | 0.00 |  | 199.84 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 156.71 | 109.92 | 0.00 |  | 0.00 |  | 109.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 147.54 | 100.75 | 0.00 |  | 0.00 |  | 100.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 132.78 | 85.99 | 0.00 |  | 0.00 |  | 85.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 125.87 | 79.08 | 0.00 |  | 0.00 |  | 79.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 125.74 | 78.95 | 0.00 |  | 0.00 |  | 78.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 125.68 | 78.89 | 0.00 |  | 0.00 |  | 78.89 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 124.22 | 77.43 | 0.00 |  | 0.00 |  | 77.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 117.44 | 70.65 | 0.00 |  | 0.00 |  | 70.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 117.22 | 70.43 | 0.00 |  | 0.00 |  | 70.43 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1200000 | 1200000 | 1.00 | 3600000 | 3.00 | 30000 | 39705508 | 0.0 | 0.0 | false | 0 | 30 | 58.4 | 128704 / 4 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-uring-spin-10000, loop 0 on CPU 3

4764660928 iterations, longest 58.4 µs; over 5 / 20 / 100 µs: 128704 / 4 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 58.4 | 58.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 52.5 | 29.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 34.2 | 34.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 58.4 | 0 | softirq TIMER | softirq TIMER 49, local timer tick 7, timer tick_nohz_handler 4 |
| 2 | 34.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 29.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 23.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120170 |
| CAL Function call interrupts | 1002 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-isolcpus/pp-uring-spin-10000 (median 65.94)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 369.03 | 303.09 | 58.43 | softirq TIMER | 0.00 |  | 244.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 226.84 | 160.90 | 0.00 |  | 0.00 |  | 160.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 212.15 | 146.21 | 0.00 |  | 0.00 |  | 146.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 205.69 | 139.75 | 0.00 |  | 0.00 |  | 139.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 199.66 | 133.72 | 0.00 |  | 0.00 |  | 133.72 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 193.44 | 127.50 | 0.00 |  | 0.00 |  | 127.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 188.98 | 123.04 | 0.00 |  | 0.00 |  | 123.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 182.78 | 116.83 | 23.43 | VM paused: the idle spare core stalled too (hypervisor) | 0.00 |  | 93.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 181.56 | 115.61 | 0.00 |  | 0.00 |  | 115.61 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 179.96 | 114.02 | 0.00 |  | 0.00 |  | 114.02 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1470562 | 1470562 | 1.00 | 1 | 0.00 | 0 | 28686707 | 0.0 | 0.0 | true | 0 | 15 | 32.3 | 60056 / 1 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-uring-sqpoll-0, loop 0 on CPU 3

1721202432 iterations, longest 32.3 µs; over 5 / 20 / 100 µs: 60056 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 32.3 | 32.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 32.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 60229 |
| CAL Function call interrupts | 992 |

#### Slowest round trips: boot-isolcpus/pp-uring-sqpoll-0 (median 40.27)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 184.26 | 143.99 | 0.00 |  | 0.00 |  | 143.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 172.53 | 132.26 | 0.00 |  | 0.00 |  | 132.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 167.87 | 127.60 | 0.00 |  | 0.00 |  | 127.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 163.49 | 123.22 | 0.00 |  | 0.00 |  | 123.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 163.44 | 123.17 | 0.00 |  | 0.00 |  | 123.17 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 162.20 | 121.93 | 0.00 |  | 0.00 |  | 121.93 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 158.66 | 118.39 | 0.00 |  | 0.00 |  | 118.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 155.41 | 115.14 | 0.00 |  | 0.00 |  | 115.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 152.41 | 112.14 | 0.00 |  | 0.00 |  | 112.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 151.92 | 111.65 | 0.00 |  | 0.00 |  | 111.65 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-isolcpus/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 14222736 | 0.0 | 0.0 | true | 0 | 30 | 44.8 | 120108 / 3 / 0 | 0 | 0 |

#### Stalls: boot-isolcpus/pp-uring-sqpoll-10000, loop 0 on CPU 3

1706728320 iterations, longest 44.8 µs; over 5 / 20 / 100 µs: 120108 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 53.6 | 31.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 44.8 | 44.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 44.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 31.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 22.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 120175 |
| CAL Function call interrupts | 1002 |

#### Slowest round trips: boot-isolcpus/pp-uring-sqpoll-10000 (median 40.82)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 168.66 | 127.84 | 0.00 |  | 0.00 |  | 127.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 159.01 | 118.19 | 0.00 |  | 183.57 | softirq TIMER | 0.00 | server: softirq TIMER |
| 3 | 153.73 | 112.91 | 0.00 |  | 0.00 |  | 112.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 132.47 | 91.65 | 0.00 |  | 0.00 |  | 91.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 119.26 | 78.45 | 0.00 |  | 0.00 |  | 78.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 117.55 | 76.74 | 0.00 |  | 0.00 |  | 76.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 114.51 | 73.69 | 0.00 |  | 0.00 |  | 73.69 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 113.41 | 72.60 | 0.00 |  | 0.00 |  | 72.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 112.82 | 72.00 | 0.00 |  | 0.00 |  | 72.00 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 112.32 | 71.50 | 0.00 |  | 0.00 |  | 71.50 | neither loop stalled long enough: network, hypervisor or untraced |

#### Stalls: boot-full-nomitig/http-recv-busy-10000, loop 0 on CPU 3

2023655795 iterations, longest 88.0 µs; over 5 / 20 / 100 µs: 9111 / 3209 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2773 | 119912.9 | 60.0 |
| mostly unexplained (+softirq NET_RX) | 422 | 18996.3 | 63.8 |
| mostly unexplained (+preempted by kworker/3:1) | 9 | 494.5 | 88.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 88.8 | 61.6 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 43.6 | 43.6 |
| loop work | 1 | 36.9 | 36.9 |
| local timer tick | 1 | 26.7 | 26.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 88.0 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 14, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 2 | 74.7 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 73.6 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 70.0 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 63.8 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 19, preempted by kworker/3:1 11, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 61.6 | 2 | VM paused: the idle spare core stalled too (hypervisor) | softirq NET_RX 1 |
| 7 | 60.0 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 59.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 59.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 10 | 58.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 978 |
| IWI IRQ work interrupts | 15 |
| LOC Local timer interrupts | 12 |

#### Slowest round trips: boot-full-nomitig/http-recv-busy-10000 (median 42.79)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 592.59 | 549.80 | 0.00 |  | 0.00 |  | 549.80 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 134.56 | 91.76 | 0.00 |  | 0.00 |  | 91.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 129.66 | 86.87 | 0.00 |  | 0.00 |  | 86.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 126.40 | 83.61 | 87.58 | mostly unexplained (+preempted by kworker/3:1) | 0.00 |  | 0.00 | client: mostly unexplained (+preempted by kworker/3:1) |
| 5 | 124.87 | 82.08 | 0.00 |  | 125.05 | mostly unexplained (+softirq NET_RX) | 0.00 | server: mostly unexplained (+softirq NET_RX) |
| 6 | 120.76 | 77.97 | 0.00 |  | 0.00 |  | 77.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 115.94 | 73.14 | 0.00 |  | 0.00 |  | 73.14 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 115.71 | 72.92 | 73.24 | mostly unexplained (+preempted by kworker/3:1) | 0.00 |  | 0.00 | client: mostly unexplained (+preempted by kworker/3:1) |
| 9 | 113.00 | 70.20 | 74.28 | mostly unexplained (+preempted by kworker/3:1) | 0.00 |  | 0.00 | client: mostly unexplained (+preempted by kworker/3:1) |
| 10 | 109.74 | 66.94 | 0.00 |  | 0.00 |  | 66.94 | neither loop stalled long enough: network, hypervisor or untraced |

## boot-full-nomitig/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=475961 clocksource=tsc

### Loop counters (boot-full-nomitig/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002450 | 539912 | 5.56 | 117412032 | 39.11 | 1956867 | 1956867 | 99.5 | 67.1 | true | 0 | 611 | 461.9 | 52350 / 4343 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672548800 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (67939 of 2900000 in 1-msg reads) | 67939 | 2.00 | 1.77 | 2.72 | 4.25 | 9.86 | 25.51 | 33.48 |

#### Stalls: boot-full-nomitig/md-recv-busy, loop 0 on CPU 3

117412032 iterations, longest 461.9 µs; over 5 / 20 / 100 µs: 52350 / 4343 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 4096 | 111892.1 | 461.9 |
| mostly unexplained (+preempted by kworker/3:1) | 99 | 2627.6 | 32.5 |
| softirq NET_RX | 92 | 2480.4 | 35.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 43 | 1003.5 | 40.7 |
| softirq TIMER | 1 | 271.9 | 271.9 |
| mostly unexplained (+softirq NET_RX) | 8 | 215.2 | 34.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.9 | 28.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 31.7 | 31.7 |
| mostly unexplained (+local timer tick) | 1 | 26.3 | 26.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 461.9 | 2450 | loop work (2450 items) |  |
| 2 | 271.9 | 0 | softirq TIMER | softirq TIMER 259, local timer tick 6, timer tick_nohz_handler 4 |
| 3 | 61.9 | 25 | loop work (25 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 61.1 | 18 | loop work (18 items) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 59.4 | 13 | loop work (13 items) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 58.1 | 10 | loop work (10 items) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 57.6 | 20 | loop work (20 items) |  |
| 8 | 55.3 | 16 | loop work (16 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 53.9 | 6 | loop work (6 items) | softirq NET_RX 17, preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 52.9 | 17 | loop work (17 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 980 |
| IWI IRQ work interrupts | 626 |
| LOC Local timer interrupts | 613 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## boot-full-nomitig/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: boot-full-nomitig/noise, loop 0 on CPU 3

9241374720 iterations, longest 26.6 µs; over 5 / 20 / 100 µs: 60 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 60 | 531.5 | 26.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 26.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 17.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 14.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 12.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 10.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 9.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 9.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 9.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 9.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 9.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 978 |

### Loop counters (boot-full-nomitig/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1255164 | 1255164 | 1.00 | 293587513 | 233.90 | 4893125 | 4851286 | 0.0 | 39.5 | false | 0 | 0 | 34.7 | 934 / 2 / 0 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-epoll-spin-0, loop 0 on CPU 3

291077184 iterations, longest 34.7 µs; over 5 / 20 / 100 µs: 934 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 56.5 | 34.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 34.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 21.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 978 |
| LOC Local timer interrupts | 10 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-epoll-spin-0 (median 47.48)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 212.53 | 165.06 | 0.00 |  | 0.00 |  | 165.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 192.14 | 144.66 | 0.00 |  | 0.00 |  | 144.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 184.03 | 136.55 | 0.00 |  | 0.00 |  | 136.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 182.45 | 134.97 | 0.00 |  | 0.00 |  | 134.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 180.57 | 133.10 | 0.00 |  | 0.00 |  | 133.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 179.19 | 131.71 | 0.00 |  | 0.00 |  | 131.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 178.22 | 130.74 | 0.00 |  | 0.00 |  | 130.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 176.84 | 129.37 | 0.00 |  | 0.00 |  | 129.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 174.85 | 127.37 | 0.00 |  | 0.00 |  | 127.37 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 174.67 | 127.19 | 0.00 |  | 0.00 |  | 127.19 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 578802688 | 482.34 | 4823356 | 4803356 | 0.0 | 36.7 | false | 0 | 0 | 151.4 | 470 / 6 / 1 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-epoll-spin-10000, loop 0 on CPU 3

576402688 iterations, longest 151.4 µs; over 5 / 20 / 100 µs: 470 / 6 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 151.4 | 151.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 99.9 | 48.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 70.9 | 49.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 151.4 | 0 | softirq TIMER | softirq TIMER 130, local timer tick 15, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 49.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 48.9 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 30.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 21.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 21.0 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 978 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-epoll-spin-10000 (median 55.82)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 425.08 | 369.26 | 0.00 |  | 0.00 |  | 369.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 395.49 | 339.66 | 151.36 | softirq TIMER | 0.00 |  | 188.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 387.69 | 331.87 | 0.00 |  | 0.00 |  | 331.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 352.09 | 296.27 | 0.00 |  | 0.00 |  | 296.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 349.50 | 293.68 | 0.00 |  | 0.00 |  | 293.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 338.48 | 282.66 | 0.00 |  | 0.00 |  | 282.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 336.55 | 280.73 | 0.00 |  | 0.00 |  | 280.73 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 333.23 | 277.41 | 0.00 |  | 0.00 |  | 277.41 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 332.81 | 276.99 | 0.00 |  | 0.00 |  | 276.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 330.90 | 275.08 | 0.00 |  | 0.00 |  | 275.08 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1529940 | 1529940 | 1.00 | 117619477 | 76.88 | 1960325 | 1934826 | 98.7 | 72.6 | true | 0 | 2287 | 247.6 | 7734 / 383 / 1 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-recv-busy-0, loop 0 on CPU 3

116089536 iterations, longest 247.6 µs; over 5 / 20 / 100 µs: 7734 / 383 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 201 | 4397.3 | 46.2 |
| softirq NET_RX | 123 | 2682.0 | 33.7 |
| mostly unexplained (+softirq NET_RX) | 49 | 1066.9 | 28.3 |
| softirq TIMER | 1 | 247.6 | 247.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 72.8 | 26.6 |
| mostly unexplained (+local timer tick) | 3 | 70.8 | 25.9 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 67.2 | 34.4 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 25.9 | 25.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 247.6 | 1 | softirq TIMER | softirq TIMER 233, local timer tick 6, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 2 | 46.2 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 34.4 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 33.7 | 0 | softirq NET_RX | softirq NET_RX 19, preempted by kworker/3:1 10, local timer tick 5, timer tick_nohz_handler 3 |
| 5 | 32.8 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 6 | 31.8 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, local timer tick 4, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 7 | 29.5 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 9, local timer tick 4, irq work (IPI/vector) 1, softirq RCU 1 |
| 8 | 28.4 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 28.3 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 11, preempted by kworker/3:1 8, local timer tick 4, irq work (IPI/vector) 1 |
| 10 | 27.0 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2330 |
| IWI IRQ work interrupts | 2295 |
| CAL Function call interrupts | 978 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-recv-busy-0 (median 38.86)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1031.97 | 993.11 | 0.00 |  | 0.00 |  | 993.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 257.23 | 218.37 | 247.17 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 131.43 | 92.57 | 0.00 |  | 0.00 |  | 92.57 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 116.67 | 77.81 | 0.00 |  | 0.00 |  | 77.81 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 113.99 | 75.13 | 0.00 |  | 0.00 |  | 75.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 113.18 | 74.33 | 0.00 |  | 0.00 |  | 74.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 113.11 | 74.25 | 0.00 |  | 0.00 |  | 74.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 111.93 | 73.07 | 0.00 |  | 0.00 |  | 73.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 111.00 | 72.15 | 0.00 |  | 0.00 |  | 72.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 109.50 | 70.65 | 0.00 |  | 0.00 |  | 70.65 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 243987776 | 203.32 | 2033231 | 2023231 | 99.5 | 69.6 | true | 0 | 2040 | 43.2 | 4816 / 704 / 0 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-recv-busy-10000, loop 0 on CPU 3

242787776 iterations, longest 43.2 µs; over 5 / 20 / 100 µs: 4816 / 704 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 440 | 10253.6 | 29.2 |
| mostly unexplained (+softirq NET_RX) | 172 | 4098.0 | 29.6 |
| softirq NET_RX | 84 | 1873.3 | 26.7 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 87.5 | 43.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 70.9 | 26.6 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 45.6 | 22.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 43.2 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 29.6 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 29.2 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 28.5 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 28.2 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 28.1 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 27.7 | 0 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 12, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 27.6 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 27.6 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 27.3 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2064 |
| IWI IRQ work interrupts | 2048 |
| CAL Function call interrupts | 978 |

#### Slowest round trips: boot-full-nomitig/pp-recv-busy-10000 (median 41.31)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 972.63 | 931.32 | 0.00 |  | 0.00 |  | 931.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 269.75 | 228.44 | 0.00 |  | 0.00 |  | 228.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 216.46 | 175.15 | 0.00 |  | 0.00 |  | 175.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 127.28 | 85.97 | 0.00 |  | 108.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 118.46 | 77.15 | 0.00 |  | 0.00 |  | 77.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 118.35 | 77.04 | 0.00 |  | 0.00 |  | 77.04 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 116.05 | 74.74 | 0.00 |  | 0.00 |  | 74.74 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 112.46 | 71.15 | 0.00 |  | 0.00 |  | 71.15 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 110.96 | 69.65 | 0.00 |  | 0.00 |  | 69.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 110.39 | 69.08 | 0.00 |  | 89.81 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |

### Loop counters (boot-full-nomitig/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1231222 | 1231222 | 1.00 | 3693667 | 3.00 | 61561 | 47310316 | 0.0 | 7.1 | false | 0 | 0 | 28.9 | 1521 / 3 / 0 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-uring-spin-0, loop 0 on CPU 3

2838618944 iterations, longest 28.9 µs; over 5 / 20 / 100 µs: 1521 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 57.0 | 28.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 28.9 | 28.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 28.9 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 28.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 28.3 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 981 |
| LOC Local timer interrupts | 13 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-uring-spin-0 (median 48.16)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 123.37 | 75.21 | 0.00 |  | 0.00 |  | 75.21 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 122.12 | 73.96 | 0.00 |  | 0.00 |  | 73.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 120.38 | 72.22 | 0.00 |  | 0.00 |  | 72.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 117.22 | 69.06 | 0.00 |  | 0.00 |  | 69.06 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 115.56 | 67.40 | 0.00 |  | 0.00 |  | 67.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 115.56 | 67.40 | 0.00 |  | 0.00 |  | 67.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 112.54 | 64.38 | 0.00 |  | 0.00 |  | 64.38 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 111.86 | 63.70 | 0.00 |  | 0.00 |  | 63.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 110.49 | 62.33 | 0.00 |  | 0.00 |  | 62.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 108.96 | 60.80 | 0.00 |  | 0.00 |  | 60.80 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1200000 | 1200000 | 1.00 | 3600000 | 3.00 | 30000 | 35499503 | 0.0 | 4.6 | false | 0 | 0 | 150.3 | 281 / 5 / 1 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-uring-spin-10000, loop 0 on CPU 3

4259940352 iterations, longest 150.3 µs; over 5 / 20 / 100 µs: 281 / 5 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 150.3 | 150.3 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 51.9 | 28.5 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 51.7 | 51.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 35.3 | 35.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 150.3 | 0 | softirq TIMER | softirq TIMER 132, local timer tick 10, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 51.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 35.3 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 4 | 28.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 23.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 981 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 5 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-uring-spin-10000 (median 48.92)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 188.07 | 139.15 | 0.00 |  | 148.92 | softirq TIMER | 0.00 | server: softirq TIMER |
| 2 | 160.14 | 111.22 | 150.30 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 3 | 159.25 | 110.33 | 0.00 |  | 0.00 |  | 110.33 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 158.12 | 109.20 | 0.00 |  | 0.00 |  | 109.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 152.69 | 103.78 | 0.00 |  | 0.00 |  | 103.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 152.11 | 103.19 | 0.00 |  | 0.00 |  | 103.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 151.89 | 102.97 | 0.00 |  | 0.00 |  | 102.97 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 151.63 | 102.71 | 0.00 |  | 0.00 |  | 102.71 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 151.00 | 102.08 | 0.00 |  | 0.00 |  | 102.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 149.60 | 100.68 | 0.00 |  | 0.00 |  | 100.68 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1369730 | 1369730 | 1.00 | 1 | 0.00 | 0 | 34026851 | 0.0 | 0.0 | true | 0 | 0 | 30.4 | 44 / 2 / 0 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-uring-sqpoll-0, loop 0 on CPU 3

2041611072 iterations, longest 30.4 µs; over 5 / 20 / 100 µs: 44 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 52.6 | 30.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 30.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 22.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 981 |
| LOC Local timer interrupts | 13 |
| IWI IRQ work interrupts | 4 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-uring-sqpoll-0 (median 42.89)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 196.52 | 153.63 | 0.00 |  | 0.00 |  | 153.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 195.48 | 152.59 | 0.00 |  | 0.00 |  | 152.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 184.33 | 141.44 | 0.00 |  | 0.00 |  | 141.44 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 179.35 | 136.46 | 0.00 |  | 0.00 |  | 136.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 173.71 | 130.82 | 0.00 |  | 0.00 |  | 130.82 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 171.36 | 128.47 | 0.00 |  | 0.00 |  | 128.47 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 170.20 | 127.31 | 0.00 |  | 0.00 |  | 127.31 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 165.47 | 122.58 | 0.00 |  | 0.00 |  | 122.58 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 162.76 | 119.87 | 0.00 |  | 0.00 |  | 119.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 162.24 | 119.34 | 0.00 |  | 0.00 |  | 119.34 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full-nomitig/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 25174555 | 0.0 | 0.0 | true | 0 | 0 | 137458.6 | 902 / 150 / 45 | 0 | 0 |

#### Stalls: boot-full-nomitig/pp-uring-sqpoll-10000, loop 0 on CPU 3

3020946624 iterations, longest 137458.6 µs; over 5 / 20 / 100 µs: 902 / 150 / 45

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 64 | 146501.8 | 137458.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 86 | 44338.6 | 8008.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 137458.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 8008.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 8002.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 7999.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 4866.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 4249.4 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 7 | 4028.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 4028.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 584.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 10 | 475.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 981 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 2 |
| HYP Hypervisor callback interrupts | 1 |

#### Slowest round trips: boot-full-nomitig/pp-uring-sqpoll-10000 (median 42.10)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 113343.70 | 113301.60 | 0.00 |  | 0.00 |  | 113301.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 15297.53 | 15255.43 | 0.00 |  | 0.00 |  | 15255.43 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 14789.34 | 14747.24 | 0.00 |  | 0.00 |  | 14747.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 9020.74 | 8978.64 | 73.64 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 8904.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 8159.23 | 8117.13 | 0.00 |  | 0.00 |  | 8117.13 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 8033.85 | 7991.75 | 7999.82 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 7 | 8024.09 | 7981.99 | 8008.59 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 8 | 8003.55 | 7961.45 | 4028.02 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 3933.43 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 9 | 5079.01 | 5036.91 | 4865.20 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 171.71 | client: unexplained: no kernel event (hypervisor or in-kernel time) |
| 10 | 4032.11 | 3990.01 | 4027.21 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 |  | 0.00 | client: unexplained: no kernel event (hypervisor or in-kernel time) |

#### Stalls: boot-full/http-recv-busy-10000, loop 0 on CPU 3

2017425305 iterations, longest 90.1 µs; over 5 / 20 / 100 µs: 12777 / 3497 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2712 | 121287.1 | 58.2 |
| mostly unexplained (+softirq NET_RX) | 432 | 20662.6 | 88.7 |
| mostly unexplained (+preempted by kworker/3:1) | 330 | 17839.2 | 90.1 |
| mostly unexplained (+local timer tick) | 6 | 484.7 | 87.0 |
| softirq NET_RX | 12 | 408.7 | 39.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 4 | 109.2 | 37.3 |
| loop work | 1 | 27.9 | 27.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 90.1 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 2 | 90.1 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 89.7 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 89.0 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 88.7 | 2 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 19, preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 88.7 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 88.6 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 87.4 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 87.0 | 2 | mostly unexplained (+local timer tick) | local timer tick 8, preempted by kworker/3:1 6, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 10 | 87.0 | 2 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1089 |
| LOC Local timer interrupts | 408 |
| IWI IRQ work interrupts | 402 |

#### Slowest round trips: boot-full/http-recv-busy-10000 (median 42.76)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 376.54 | 333.78 | 0.00 |  | 0.00 |  | 333.78 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 127.94 | 85.17 | 89.22 | mostly unexplained (+preempted by kworker/3:1) | 0.00 |  | 0.00 | client: mostly unexplained (+preempted by kworker/3:1) |
| 3 | 127.93 | 85.17 | 88.17 | mostly unexplained (+preempted by kworker/3:1) | 0.00 |  | 0.00 | client: mostly unexplained (+preempted by kworker/3:1) |
| 4 | 127.60 | 84.84 | 87.04 | mostly unexplained (+preempted by kworker/3:1) | 0.00 |  | 0.00 | client: mostly unexplained (+preempted by kworker/3:1) |
| 5 | 127.52 | 84.76 | 89.70 | mostly unexplained (+preempted by kworker/3:1) | 0.00 |  | 0.00 | client: mostly unexplained (+preempted by kworker/3:1) |
| 6 | 127.14 | 84.38 | 86.15 | mostly unexplained (+preempted by kworker/3:1) | 0.00 |  | 0.00 | client: mostly unexplained (+preempted by kworker/3:1) |
| 7 | 126.89 | 84.12 | 88.30 | mostly unexplained (+softirq NET_RX) | 0.00 |  | 0.00 | client: mostly unexplained (+softirq NET_RX) |
| 8 | 126.87 | 84.11 | 85.31 | mostly unexplained (+preempted by kworker/3:1) | 0.00 |  | 0.00 | client: mostly unexplained (+preempted by kworker/3:1) |
| 9 | 126.78 | 84.02 | 89.65 | mostly unexplained (+preempted by kworker/3:1) | 0.00 |  | 0.00 | client: mostly unexplained (+preempted by kworker/3:1) |
| 10 | 126.70 | 83.94 | 88.25 | mostly unexplained (+preempted by kworker/3:1) | 0.00 |  | 0.00 | client: mostly unexplained (+preempted by kworker/3:1) |

## boot-full/md-recv-busy: lab, 60 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=517676 clocksource=tsc

### Loop counters (boot-full/md-recv-busy)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 3002450 | 580453 | 5.17 | 113338048 | 37.75 | 1888967 | 1888967 | 99.5 | 65.7 | true | 0 | 810 | 510.1 | 81538 / 5645 / 2 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 672548800 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (88444 of 2900000 in 1-msg reads) | 88444 | 2.43 | 2.16 | 3.23 | 5.59 | 17.11 | 33.62 | 36.06 |

#### Stalls: boot-full/md-recv-busy, loop 0 on CPU 3

113338048 iterations, longest 510.1 µs; over 5 / 20 / 100 µs: 81538 / 5645 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 5294 | 152715.5 | 510.1 |
| mostly unexplained (+preempted by kworker/3:1) | 147 | 3977.9 | 49.3 |
| softirq NET_RX | 97 | 2578.9 | 35.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 91 | 2086.4 | 31.8 |
| mostly unexplained (+softirq NET_RX) | 14 | 372.6 | 36.9 |
| softirq TIMER | 1 | 111.0 | 111.0 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 30.0 | 30.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 510.1 | 2450 | loop work (2450 items) |  |
| 2 | 111.0 | 0 | softirq TIMER | softirq TIMER 97, local timer tick 8, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 3 | 71.1 | 27 | loop work (27 items) | softirq NET_RX 25, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 66.7 | 39 | loop work (39 items) | softirq NET_TX 2 |
| 5 | 66.0 | 18 | loop work (18 items) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 65.7 | 12 | loop work (12 items) |  |
| 7 | 65.5 | 23 | loop work (23 items) | softirq NET_RX 21, preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 63.9 | 24 | loop work (24 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 61.1 | 11 | loop work (11 items) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 60.2 | 20 | loop work (20 items) | softirq NET_RX 24, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| IWI IRQ work interrupts | 823 |
| LOC Local timer interrupts | 816 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## boot-full/noise: platform noise on CPU 3 (120 s, a loop that only reads the counter)

#### Stalls: boot-full/noise, loop 0 on CPU 3

9251065856 iterations, longest 30.5 µs; over 5 / 20 / 100 µs: 42 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 34 | 289.4 | 30.5 |
| mostly unexplained (+call function (IPI/vector)) | 8 | 44.4 | 6.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 30.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 16.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 14.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 9.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 8.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 8.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 8.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 8.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 8.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 8.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |

### Loop counters (boot-full/pp-epoll-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1250906 | 1250906 | 1.00 | 292654837 | 233.95 | 4877581 | 4835884 | 0.0 | 39.6 | false | 0 | 0 | 28.9 | 1769 / 2 / 0 | 0 | 0 |

#### Stalls: boot-full/pp-epoll-spin-0, loop 0 on CPU 3

290153024 iterations, longest 28.9 µs; over 5 / 20 / 100 µs: 1769 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 55.5 | 28.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 28.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 26.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 990 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 6 |
| RES Rescheduling interrupts | 2 |

#### Slowest round trips: boot-full/pp-epoll-spin-0 (median 47.84)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1057.36 | 1009.52 | 0.00 |  | 189.98 | softirq TIMER | 819.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 220.84 | 173.01 | 0.00 |  | 0.00 |  | 173.01 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 206.02 | 158.18 | 0.00 |  | 0.00 |  | 158.18 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 205.75 | 157.91 | 0.00 |  | 0.00 |  | 157.91 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 183.76 | 135.92 | 0.00 |  | 0.00 |  | 135.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 182.91 | 135.08 | 0.00 |  | 0.00 |  | 135.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 181.61 | 133.77 | 0.00 |  | 0.00 |  | 133.77 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 178.38 | 130.55 | 0.00 |  | 0.00 |  | 130.55 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 177.63 | 129.79 | 0.00 |  | 0.00 |  | 129.79 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 175.73 | 127.89 | 0.00 |  | 0.00 |  | 127.89 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-epoll-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| epoll-spin | 1200000 | 1200000 | 1.00 | 563799104 | 469.83 | 4698326 | 4678326 | 0.0 | 37.1 | false | 0 | 0 | 367.2 | 952 / 4 / 1 | 0 | 0 |

#### Stalls: boot-full/pp-epoll-spin-10000, loop 0 on CPU 3

561399104 iterations, longest 367.2 µs; over 5 / 20 / 100 µs: 952 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 367.2 | 367.2 |
| VM paused: the idle spare core stalled too (hypervisor) | 3 | 93.4 | 44.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 367.2 | 0 | softirq TIMER | softirq TIMER 347, local timer tick 12, timer tick_nohz_handler 6, irq work (IPI/vector) 1 |
| 2 | 44.5 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 3 | 26.7 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 4 | 22.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1001 |
| IWI IRQ work interrupts | 7 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full/pp-epoll-spin-10000 (median 49.68)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1051.33 | 1001.64 | 367.19 | softirq TIMER | 0.00 |  | 634.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 211.79 | 162.11 | 0.00 |  | 0.00 |  | 162.11 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 197.44 | 147.76 | 0.00 |  | 0.00 |  | 147.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 195.75 | 146.07 | 0.00 |  | 0.00 |  | 146.07 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 143.43 | 93.75 | 0.00 |  | 0.00 |  | 93.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 141.55 | 91.86 | 0.00 |  | 0.00 |  | 91.86 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 140.44 | 90.76 | 0.00 |  | 0.00 |  | 90.76 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 140.07 | 90.39 | 0.00 |  | 0.00 |  | 90.39 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 136.51 | 86.83 | 0.00 |  | 0.00 |  | 86.83 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 130.17 | 80.49 | 0.00 |  | 0.00 |  | 80.49 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-recv-busy-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1452813 | 1452813 | 1.00 | 112144270 | 77.19 | 1869071 | 1844858 | 98.7 | 71.1 | true | 0 | 1167 | 164.0 | 10733 / 294 / 1 | 0 | 0 |

#### Stalls: boot-full/pp-recv-busy-0, loop 0 on CPU 3

110691456 iterations, longest 164.0 µs; over 5 / 20 / 100 µs: 10733 / 294 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 197 | 4595.1 | 40.0 |
| mostly unexplained (+softirq NET_RX) | 51 | 1235.7 | 29.7 |
| softirq NET_RX | 40 | 913.9 | 27.8 |
| softirq TIMER | 1 | 164.0 | 164.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 71.1 | 39.0 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 37.5 | 37.5 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 35.6 | 35.6 |
| mostly unexplained (+local timer tick) | 1 | 25.7 | 25.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 164.0 | 0 | softirq TIMER | softirq TIMER 149, local timer tick 9, timer tick_nohz_handler 4, irq work (IPI/vector) 1 |
| 2 | 40.0 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 9, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 39.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 37.5 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 5 | 35.6 | 1 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 6 | 32.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 30.4 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 29.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 12, preempted by kworker/3:1 8, local timer tick 3, timer tick_nohz_handler 2 |
| 9 | 29.7 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 28.4 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 1188 |
| IWI IRQ work interrupts | 1174 |
| CAL Function call interrupts | 990 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full/pp-recv-busy-0 (median 40.96)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 1029.52 | 988.56 | 0.00 |  | 0.00 |  | 988.56 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 261.68 | 220.72 | 0.00 |  | 67.18 | softirq TIMER | 153.54 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 228.34 | 187.38 | 163.96 | softirq TIMER | 0.00 |  | 23.42 | client: softirq TIMER |
| 4 | 223.21 | 182.25 | 0.00 |  | 0.00 |  | 182.25 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 206.19 | 165.24 | 0.00 |  | 0.00 |  | 165.24 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 186.81 | 145.85 | 0.00 |  | 0.00 |  | 145.85 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 171.62 | 130.66 | 0.00 |  | 0.00 |  | 130.66 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 168.22 | 127.26 | 0.00 |  | 0.00 |  | 127.26 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 163.71 | 122.75 | 0.00 |  | 0.00 |  | 122.75 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 159.14 | 118.18 | 0.00 |  | 0.00 |  | 118.18 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-recv-busy-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 1200000 | 1200000 | 1.00 | 232712320 | 193.93 | 1939269 | 1929269 | 99.5 | 67.9 | true | 0 | 1809 | 43.8 | 12905 / 631 / 0 | 0 | 0 |

#### Stalls: boot-full/pp-recv-busy-10000, loop 0 on CPU 3

231512320 iterations, longest 43.8 µs; over 5 / 20 / 100 µs: 12905 / 631 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| mostly unexplained (+preempted by kworker/3:1) | 352 | 8133.5 | 43.8 |
| softirq NET_RX | 205 | 4692.9 | 31.4 |
| mostly unexplained (+softirq NET_RX) | 66 | 1659.3 | 34.9 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 95.4 | 40.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 74.4 | 27.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 60.0 | 33.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 43.8 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 12, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 2 | 40.2 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, softirq NET_RX 1, ipi_send_cpu 0 |
| 3 | 34.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 16, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 33.3 | 1 | VM paused: the idle spare core stalled too (hypervisor) | softirq NET_RX 1 |
| 5 | 32.0 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 31.8 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 31.6 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 31.4 | 0 | softirq NET_RX | softirq NET_RX 16, preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 30.6 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 10 | 29.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 14, preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 1848 |
| IWI IRQ work interrupts | 1817 |
| CAL Function call interrupts | 999 |

#### Slowest round trips: boot-full/pp-recv-busy-10000 (median 43.33)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 163.93 | 120.60 | 0.00 |  | 0.00 |  | 120.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 163.79 | 120.46 | 0.00 |  | 0.00 |  | 120.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 163.01 | 119.68 | 0.00 |  | 0.00 |  | 119.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 161.53 | 118.20 | 0.00 |  | 0.00 |  | 118.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 159.42 | 116.09 | 0.00 |  | 0.00 |  | 116.09 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 158.41 | 115.08 | 0.00 |  | 0.00 |  | 115.08 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 156.79 | 113.46 | 0.00 |  | 0.00 |  | 113.46 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 154.84 | 111.51 | 0.00 |  | 0.00 |  | 111.51 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 154.83 | 111.50 | 0.00 |  | 0.00 |  | 111.50 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 154.27 | 110.94 | 0.00 |  | 0.00 |  | 110.94 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-uring-spin-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1182563 | 1182563 | 1.00 | 3547690 | 3.00 | 59128 | 46947235 | 0.0 | 7.7 | false | 0 | 0 | 39.9 | 1569 / 4 / 0 | 0 | 0 |

#### Stalls: boot-full/pp-uring-spin-0, loop 0 on CPU 3

2816834112 iterations, longest 39.9 µs; over 5 / 20 / 100 µs: 1569 / 4 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 73.3 | 39.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 39.9 | 39.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 28.1 | 28.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 39.9 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 2 | 39.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 34.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 28.1 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 992 |
| LOC Local timer interrupts | 10 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full/pp-uring-spin-0 (median 49.05)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 170.98 | 121.94 | 0.00 |  | 0.00 |  | 121.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 170.37 | 121.32 | 0.00 |  | 0.00 |  | 121.32 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 169.24 | 120.19 | 0.00 |  | 0.00 |  | 120.19 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 165.97 | 116.92 | 0.00 |  | 0.00 |  | 116.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 164.92 | 115.87 | 0.00 |  | 0.00 |  | 115.87 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 163.99 | 114.95 | 0.00 |  | 0.00 |  | 114.95 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 162.50 | 113.45 | 0.00 |  | 0.00 |  | 113.45 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 162.39 | 113.34 | 0.00 |  | 0.00 |  | 113.34 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 161.67 | 112.63 | 0.00 |  | 0.00 |  | 112.63 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 161.60 | 112.55 | 0.00 |  | 0.00 |  | 112.55 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-uring-spin-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-spin | 1200000 | 1200000 | 1.00 | 3600000 | 3.00 | 30000 | 35183528 | 0.0 | 5.0 | false | 0 | 0 | 396.9 | 1010 / 5 / 1 | 0 | 0 |

#### Stalls: boot-full/pp-uring-spin-10000, loop 0 on CPU 3

4222023360 iterations, longest 396.9 µs; over 5 / 20 / 100 µs: 1010 / 5 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 396.9 | 396.9 |
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 54.8 | 29.6 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 39.0 | 39.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 30.6 | 30.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 396.9 | 0 | softirq TIMER | softirq TIMER 371, local timer tick 15, timer tick_nohz_handler 8, irq work (IPI/vector) 1 |
| 2 | 39.0 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, ipi_send_cpu 0 |
| 3 | 30.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 29.6 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 5 | 25.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1003 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |

#### Slowest round trips: boot-full/pp-uring-spin-10000 (median 49.16)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 403.45 | 354.30 | 396.87 | softirq TIMER | 0.00 |  | 0.00 | client: softirq TIMER |
| 2 | 221.36 | 172.20 | 0.00 |  | 0.00 |  | 172.20 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 200.46 | 151.30 | 0.00 |  | 0.00 |  | 151.30 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 198.06 | 148.90 | 0.00 |  | 0.00 |  | 148.90 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 197.80 | 148.64 | 0.00 |  | 0.00 |  | 148.64 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 194.52 | 145.36 | 0.00 |  | 0.00 |  | 145.36 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 182.25 | 133.10 | 0.00 |  | 0.00 |  | 133.10 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 179.83 | 130.68 | 0.00 |  | 0.00 |  | 130.68 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 178.64 | 129.48 | 0.00 |  | 0.00 |  | 129.48 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 177.24 | 128.09 | 0.00 |  | 0.00 |  | 128.09 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-uring-sqpoll-0)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1425070 | 1425070 | 1.00 | 1 | 0.00 | 0 | 35927930 | 0.0 | 0.0 | true | 0 | 0 | 29.9 | 56 / 1 / 0 | 0 | 0 |

#### Stalls: boot-full/pp-uring-sqpoll-0, loop 0 on CPU 3

2155675776 iterations, longest 29.9 µs; over 5 / 20 / 100 µs: 56 / 1 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 1 | 29.9 | 29.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 29.9 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| LOC Local timer interrupts | 11 |
| IWI IRQ work interrupts | 5 |
| RES Rescheduling interrupts | 1 |

#### Slowest round trips: boot-full/pp-uring-sqpoll-0 (median 41.60)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 331.52 | 289.92 | 0.00 |  | 0.00 |  | 289.92 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 165.19 | 123.59 | 0.00 |  | 0.00 |  | 123.59 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 134.56 | 92.96 | 0.00 |  | 0.00 |  | 92.96 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 132.44 | 90.84 | 0.00 |  | 103.98 | unexplained: no kernel event (hypervisor or in-kernel time) | 0.00 | server: unexplained: no kernel event (hypervisor or in-kernel time) |
| 5 | 127.48 | 85.88 | 0.00 |  | 0.00 |  | 85.88 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 125.54 | 83.94 | 0.00 |  | 0.00 |  | 83.94 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 122.82 | 81.22 | 0.00 |  | 0.00 |  | 81.22 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 122.20 | 80.60 | 0.00 |  | 0.00 |  | 80.60 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 118.00 | 76.40 | 0.00 |  | 0.00 |  | 76.40 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 116.70 | 75.11 | 0.00 |  | 0.00 |  | 75.11 | neither loop stalled long enough: network, hypervisor or untraced |

### Loop counters (boot-full/pp-uring-sqpoll-10000)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 1200000 | 1200000 | 1.00 | 1 | 0.00 | 0 | 28534077 | 0.0 | 0.0 | true | 0 | 0 | 27.2 | 107 / 2 / 0 | 0 | 0 |

#### Stalls: boot-full/pp-uring-sqpoll-10000, loop 0 on CPU 3

3424089280 iterations, longest 27.2 µs; over 5 / 20 / 100 µs: 107 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| VM paused: the idle spare core stalled too (hypervisor) | 2 | 49.0 | 27.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 27.2 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |
| 2 | 21.8 | 0 | VM paused: the idle spare core stalled too (hypervisor) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1001 |
| IWI IRQ work interrupts | 5 |
| LOC Local timer interrupts | 1 |

#### Slowest round trips: boot-full/pp-uring-sqpoll-10000 (median 43.30)

| # | round trip µs | over median µs | client stall µs | client cause | server stall µs | server cause | not explained by either µs | verdict |
|---|---|---|---|---|---|---|---|---|
| 1 | 158.94 | 115.65 | 0.00 |  | 0.00 |  | 115.65 | neither loop stalled long enough: network, hypervisor or untraced |
| 2 | 142.58 | 99.29 | 0.00 |  | 0.00 |  | 99.29 | neither loop stalled long enough: network, hypervisor or untraced |
| 3 | 129.29 | 85.99 | 0.00 |  | 0.00 |  | 85.99 | neither loop stalled long enough: network, hypervisor or untraced |
| 4 | 127.97 | 84.67 | 0.00 |  | 0.00 |  | 84.67 | neither loop stalled long enough: network, hypervisor or untraced |
| 5 | 119.42 | 76.12 | 0.00 |  | 0.00 |  | 76.12 | neither loop stalled long enough: network, hypervisor or untraced |
| 6 | 116.99 | 73.70 | 0.00 |  | 0.00 |  | 73.70 | neither loop stalled long enough: network, hypervisor or untraced |
| 7 | 116.64 | 73.35 | 0.00 |  | 0.00 |  | 73.35 | neither loop stalled long enough: network, hypervisor or untraced |
| 8 | 115.57 | 72.27 | 0.00 |  | 0.00 |  | 72.27 | neither loop stalled long enough: network, hypervisor or untraced |
| 9 | 115.14 | 71.84 | 0.00 |  | 0.00 |  | 71.84 | neither loop stalled long enough: network, hypervisor or untraced |
| 10 | 112.78 | 69.49 | 0.00 |  | 0.00 |  | 69.49 | neither loop stalled long enough: network, hypervisor or untraced |

## Ping-pong round trip (client send → echo decoded; "from when due": from the slot on the paced schedule, including any wait behind a stall)

| variant @ rate | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189944 | 42.95 | 42.01 | 45.04 | 63.77 | 132.29 | 298.71 | 697.59 |
| r1-base-0/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189944 | 43.83 | 42.21 | 45.27 | 66.12 | 336.37 | 780.20 | 1309.53 |
| r1-base-0/pp-epoll-spin-0: epoll-spin @ 0/s | 1190258 | 49.47 | 48.71 | 54.61 | 63.83 | 98.02 | 132.00 | 343.74 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189998 | 52.99 | 47.63 | 73.96 | 91.30 | 99.06 | 254.31 | 402.69 |
| r1-base-0/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189998 | 55.59 | 47.79 | 74.29 | 91.79 | 864.45 | 4444.21 | 5953.20 |
| r1-base-0/pp-recv-busy-0: recv-busy @ 0/s | 1475161 | 39.90 | 39.51 | 42.56 | 46.68 | 57.17 | 69.95 | 1038.29 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 42.14 | 41.82 | 44.27 | 48.43 | 62.38 | 80.10 | 134.24 |
| r1-base-0/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 42.44 | 42.12 | 44.58 | 48.74 | 62.72 | 80.48 | 134.59 |
| r1-base-0/pp-uring-spin-0: uring-spin @ 0/s | 1179936 | 49.24 | 48.37 | 49.95 | 91.34 | 97.57 | 110.55 | 265.65 |
| r1-base-0/pp-uring-spin-10000: uring-spin @ 10000/s | 1189998 | 49.50 | 46.73 | 49.40 | 94.27 | 266.90 | 320.74 | 431.81 |
| r1-base-0/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189998 | 80.61 | 46.83 | 49.90 | 1042.22 | 4493.69 | 11646.85 | 14730.42 |
| r1-base-0/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1455907 | 40.30 | 40.00 | 42.97 | 47.03 | 54.53 | 66.07 | 176.29 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189997 | 42.04 | 41.73 | 44.22 | 48.36 | 57.20 | 69.36 | 205.63 |
| r1-base-0/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189997 | 42.12 | 41.81 | 44.30 | 48.45 | 57.32 | 69.98 | 391.77 |
| r1-base-2/pp-epoll-spin-0: epoll-spin @ 0/s | 1181942 | 49.83 | 49.03 | 52.18 | 97.80 | 101.06 | 135.10 | 258.29 |
| r1-base-2/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 53.69 | 47.21 | 76.83 | 90.47 | 102.56 | 276.07 | 389.61 |
| r1-base-2/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 73.58 | 47.37 | 77.33 | 91.18 | 7921.43 | 19620.81 | 21954.57 |
| r1-base-2/pp-recv-busy-0: recv-busy @ 0/s | 1442319 | 40.81 | 40.48 | 43.36 | 47.45 | 57.83 | 71.20 | 129.50 |
| r1-base-2/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 41.87 | 41.58 | 43.92 | 47.36 | 61.87 | 71.67 | 254.92 |
| r1-base-2/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 42.19 | 41.88 | 44.26 | 48.29 | 62.35 | 82.24 | 368.39 |
| r1-base-10/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189938 | 42.09 | 41.69 | 43.74 | 58.22 | 71.25 | 118.05 | 183.91 |
| r1-base-10/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189938 | 42.26 | 41.85 | 43.90 | 58.50 | 71.84 | 125.76 | 257.50 |
| r1-base-10/pp-epoll-spin-0: epoll-spin @ 0/s | 1207389 | 48.77 | 48.97 | 50.20 | 56.50 | 66.46 | 245.40 | 368.77 |
| r1-base-10/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 48.51 | 48.33 | 50.46 | 53.64 | 61.31 | 77.72 | 127.14 |
| r1-base-10/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 48.68 | 48.50 | 50.63 | 53.81 | 61.53 | 77.86 | 127.34 |
| r1-base-10/pp-recv-busy-0: recv-busy @ 0/s | 1387073 | 42.44 | 41.98 | 45.80 | 49.65 | 56.49 | 88.94 | 221.01 |
| r1-base-10/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 41.34 | 41.17 | 43.21 | 45.73 | 53.71 | 67.85 | 128.12 |
| r1-base-10/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 41.64 | 41.46 | 43.51 | 46.03 | 54.02 | 68.12 | 128.65 |
| r1-base-10/pp-uring-spin-0: uring-spin @ 0/s | 1205794 | 48.20 | 48.38 | 49.56 | 55.36 | 62.99 | 204.47 | 1035.10 |
| r1-base-10/pp-uring-spin-10000: uring-spin @ 10000/s | 1189998 | 45.70 | 45.43 | 47.87 | 50.68 | 57.99 | 120.19 | 981.86 |
| r1-base-10/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189998 | 45.78 | 45.50 | 47.94 | 50.76 | 59.08 | 125.86 | 981.93 |
| r1-base-10/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1428151 | 41.06 | 40.84 | 42.91 | 45.49 | 52.09 | 65.59 | 195.38 |
| r1-base-10/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 43.37 | 42.92 | 46.43 | 50.56 | 58.38 | 90.56 | 158.04 |
| r1-base-10/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 43.44 | 43.00 | 46.51 | 50.65 | 58.79 | 92.42 | 158.07 |
| r1-base-12/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189934 | 42.12 | 41.72 | 43.94 | 58.38 | 65.28 | 97.37 | 257.01 |
| r1-base-12/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189934 | 42.29 | 41.88 | 44.10 | 58.59 | 65.74 | 104.25 | 257.17 |
| r1-base-12/pp-epoll-spin-0: epoll-spin @ 0/s | 1241664 | 47.42 | 47.12 | 49.53 | 55.41 | 61.33 | 81.38 | 187.80 |
| r1-base-12/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 47.63 | 47.75 | 49.90 | 52.57 | 57.90 | 130.19 | 982.30 |
| r1-base-12/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 47.80 | 47.90 | 50.06 | 52.74 | 58.62 | 133.66 | 982.42 |
| r1-base-12/pp-recv-busy-0: recv-busy @ 0/s | 1450604 | 40.58 | 40.35 | 42.57 | 45.52 | 54.81 | 67.75 | 161.84 |
| r1-base-12/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 41.20 | 41.04 | 43.03 | 45.39 | 52.04 | 65.48 | 159.90 |
| r1-base-12/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 41.50 | 41.33 | 43.33 | 45.70 | 52.38 | 67.00 | 255.81 |
| r1-base-12/pp-uring-spin-0: uring-spin @ 0/s | 1215757 | 47.79 | 47.72 | 49.76 | 56.07 | 63.76 | 82.84 | 197.39 |
| r1-base-12/pp-uring-spin-10000: uring-spin @ 10000/s | 1189998 | 48.12 | 47.97 | 50.08 | 53.00 | 63.35 | 91.73 | 971.95 |
| r1-base-12/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189998 | 48.20 | 48.04 | 50.14 | 53.08 | 63.54 | 94.46 | 972.03 |
| r1-base-12/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1421401 | 41.25 | 41.02 | 43.19 | 45.84 | 54.28 | 68.53 | 861.59 |
| r1-base-12/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189997 | 42.20 | 41.98 | 44.16 | 47.27 | 57.08 | 66.77 | 127.09 |
| r1-base-12/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189997 | 42.27 | 42.06 | 44.24 | 47.35 | 57.19 | 66.87 | 127.22 |
| boot-stock/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189942 | 41.39 | 40.85 | 43.23 | 52.75 | 86.90 | 95.19 | 132.54 |
| boot-stock/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189942 | 41.56 | 41.02 | 43.41 | 52.96 | 87.07 | 95.36 | 132.69 |
| boot-stock/pp-epoll-spin-0: epoll-spin @ 0/s | 1271385 | 46.31 | 46.02 | 48.23 | 53.80 | 60.42 | 78.47 | 239.55 |
| boot-stock/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189998 | 62.37 | 47.38 | 53.25 | 255.44 | 326.77 | 386.15 | 4651.53 |
| boot-stock/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189998 | 5912.21 | 48.01 | 23074.72 | 60548.67 | 138707.76 | 173130.35 | 177156.92 |
| boot-stock/pp-recv-busy-0: recv-busy @ 0/s | 1423663 | 41.31 | 40.68 | 45.35 | 50.59 | 57.70 | 77.06 | 5225.32 |
| boot-stock/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 40.17 | 40.00 | 42.07 | 44.30 | 49.13 | 62.74 | 177.47 |
| boot-stock/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 40.44 | 40.27 | 42.35 | 44.57 | 49.52 | 63.13 | 177.72 |
| boot-stock/pp-uring-spin-0: uring-spin @ 0/s | 1212479 | 47.96 | 48.33 | 49.28 | 54.20 | 61.25 | 74.26 | 183.16 |
| boot-stock/pp-uring-spin-10000: uring-spin @ 10000/s | 1189999 | 43.84 | 43.40 | 45.64 | 54.00 | 56.75 | 70.67 | 281.77 |
| boot-stock/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189999 | 43.92 | 43.48 | 45.72 | 54.08 | 56.85 | 70.94 | 281.86 |
| boot-stock/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1395311 | 41.97 | 41.72 | 43.95 | 48.05 | 53.49 | 64.24 | 130.21 |
| boot-stock/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189997 | 41.02 | 40.72 | 42.79 | 50.79 | 54.51 | 68.52 | 1625.01 |
| boot-stock/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189997 | 41.15 | 40.84 | 42.91 | 50.91 | 54.70 | 71.08 | 1625.10 |
| boot-isolcpus/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189943 | 41.93 | 41.20 | 43.89 | 58.65 | 86.31 | 103.71 | 225.76 |
| boot-isolcpus/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189943 | 42.10 | 41.36 | 44.06 | 58.82 | 86.48 | 104.03 | 225.91 |
| boot-isolcpus/pp-epoll-spin-0: epoll-spin @ 0/s | 1253595 | 46.97 | 46.63 | 49.44 | 57.65 | 65.27 | 79.83 | 389.90 |
| boot-isolcpus/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 49.20 | 48.86 | 51.04 | 54.23 | 77.82 | 265.73 | 404.94 |
| boot-isolcpus/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 85.72 | 48.98 | 51.18 | 54.70 | 12777.80 | 41964.15 | 45739.85 |
| boot-isolcpus/pp-recv-busy-0: recv-busy @ 0/s | 1475589 | 39.88 | 39.58 | 42.03 | 46.47 | 53.66 | 67.61 | 220.51 |
| boot-isolcpus/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 41.44 | 41.26 | 43.28 | 45.61 | 55.50 | 70.77 | 128.82 |
| boot-isolcpus/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 41.73 | 41.54 | 43.59 | 46.00 | 55.78 | 71.21 | 129.09 |
| boot-isolcpus/pp-uring-spin-0: uring-spin @ 0/s | 1236896 | 47.08 | 46.79 | 49.41 | 55.80 | 64.76 | 80.32 | 222.18 |
| boot-isolcpus/pp-uring-spin-10000: uring-spin @ 10000/s | 1189998 | 66.15 | 65.94 | 85.89 | 91.72 | 96.22 | 145.34 | 369.03 |
| boot-isolcpus/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189998 | 66.24 | 66.03 | 85.98 | 91.84 | 96.69 | 148.88 | 369.10 |
| boot-isolcpus/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1445894 | 40.58 | 40.27 | 42.83 | 47.12 | 55.20 | 72.15 | 184.26 |
| boot-isolcpus/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 41.54 | 40.82 | 46.48 | 49.21 | 54.56 | 78.82 | 168.66 |
| boot-isolcpus/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 41.64 | 40.92 | 46.58 | 49.31 | 54.69 | 80.13 | 168.74 |
| boot-full-nomitig/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189945 | 43.11 | 42.79 | 44.84 | 48.96 | 86.43 | 94.56 | 592.59 |
| boot-full-nomitig/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189945 | 43.27 | 42.95 | 45.00 | 49.12 | 86.63 | 94.83 | 592.76 |
| boot-full-nomitig/pp-epoll-spin-0: epoll-spin @ 0/s | 1234200 | 47.72 | 47.48 | 50.14 | 57.20 | 65.44 | 81.75 | 212.53 |
| boot-full-nomitig/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 61.19 | 55.82 | 84.32 | 91.77 | 95.93 | 235.11 | 425.08 |
| boot-full-nomitig/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 61.90 | 56.03 | 84.53 | 92.05 | 106.84 | 1091.08 | 1495.97 |
| boot-full-nomitig/pp-recv-busy-0: recv-busy @ 0/s | 1504438 | 39.13 | 38.86 | 41.35 | 44.49 | 51.70 | 64.34 | 1031.97 |
| boot-full-nomitig/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 41.50 | 41.31 | 43.30 | 45.75 | 53.03 | 77.26 | 972.63 |
| boot-full-nomitig/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 41.80 | 41.60 | 43.60 | 46.07 | 53.43 | 81.58 | 972.95 |
| boot-full-nomitig/pp-uring-spin-0: uring-spin @ 0/s | 1210615 | 48.06 | 48.16 | 50.21 | 57.35 | 64.34 | 77.74 | 123.37 |
| boot-full-nomitig/pp-uring-spin-10000: uring-spin @ 10000/s | 1189999 | 49.02 | 48.92 | 50.99 | 53.78 | 60.48 | 121.27 | 188.07 |
| boot-full-nomitig/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189999 | 49.09 | 48.98 | 51.05 | 53.86 | 61.14 | 122.17 | 188.14 |
| boot-full-nomitig/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1346928 | 43.53 | 42.89 | 47.70 | 52.62 | 58.45 | 91.44 | 196.52 |
| boot-full-nomitig/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 42.44 | 42.10 | 44.30 | 47.24 | 57.97 | 77.69 | 113343.70 |
| boot-full-nomitig/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 281.01 | 42.19 | 44.43 | 51.30 | 91277.54 | 130932.24 | 137595.77 |
| boot-full/http-recv-busy-10000: ergon/recv-busy @ 10000/s | 1189943 | 43.08 | 42.76 | 44.82 | 48.44 | 88.44 | 113.99 | 376.54 |
| boot-full/http-recv-busy-10000: ergon/recv-busy @ 10000/s, from when due | 1189943 | 43.24 | 42.91 | 44.98 | 48.64 | 88.62 | 114.60 | 376.68 |
| boot-full/pp-epoll-spin-0: epoll-spin @ 0/s | 1230369 | 47.85 | 47.84 | 49.77 | 56.20 | 64.24 | 85.56 | 1057.36 |
| boot-full/pp-epoll-spin-10000: epoll-spin @ 10000/s | 1189999 | 49.81 | 49.68 | 52.11 | 54.75 | 63.38 | 85.12 | 1051.33 |
| boot-full/pp-epoll-spin-10000: epoll-spin @ 10000/s, from when due | 1189999 | 49.99 | 49.85 | 52.28 | 54.92 | 63.87 | 87.64 | 1051.47 |
| boot-full/pp-recv-busy-0: recv-busy @ 0/s | 1428501 | 41.20 | 40.96 | 43.35 | 46.33 | 55.51 | 71.77 | 1029.52 |
| boot-full/pp-recv-busy-10000: recv-busy @ 10000/s | 1189999 | 43.84 | 43.33 | 46.92 | 50.63 | 55.93 | 94.73 | 163.93 |
| boot-full/pp-recv-busy-10000: recv-busy @ 10000/s, from when due | 1189999 | 44.14 | 43.62 | 47.22 | 50.95 | 56.68 | 99.83 | 164.31 |
| boot-full/pp-uring-spin-0: uring-spin @ 0/s | 1162806 | 50.03 | 49.05 | 54.35 | 60.42 | 68.04 | 95.47 | 170.98 |
| boot-full/pp-uring-spin-10000: uring-spin @ 10000/s | 1189998 | 49.34 | 49.16 | 51.20 | 54.50 | 61.36 | 130.46 | 403.45 |
| boot-full/pp-uring-spin-10000: uring-spin @ 10000/s, from when due | 1189998 | 49.41 | 49.22 | 51.26 | 54.60 | 62.40 | 133.12 | 403.51 |
| boot-full/pp-uring-sqpoll-0: uring-sqpoll @ 0/s | 1401188 | 41.85 | 41.60 | 44.09 | 47.29 | 56.39 | 71.10 | 331.52 |
| boot-full/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s | 1189998 | 43.49 | 43.30 | 45.43 | 47.91 | 53.66 | 67.45 | 158.94 |
| boot-full/pp-uring-sqpoll-10000: uring-sqpoll @ 10000/s, from when due | 1189998 | 43.56 | 43.37 | 45.50 | 47.99 | 53.79 | 67.91 | 159.09 |

