# Settings matrix

Conditions in run order; baselines (`rN-base-k`) bracket every two conditions and each value shows its change against the median of the same round's baselines (boot profiles against round 1; stage-2 `cN-…` settings against all stage-2 baselines, measured A/B/A/B with them). All values in µs. Every table keeps the maximum and names the cause of the worst case, and the worst-events section lists the five slowest samples of every run with their causes. A maximum is still one event, so to choose between settings weigh p99.99 and the rates over 100 µs, 500 µs, 1 ms and 10 ms as well.

## Platform noise on the loop CPU (a loop that only reads the counter)

| condition | longest gap | gaps >5 µs /min | >20 µs /min | >100 µs /min | longest gap's cause |
|---|---|---|---|---|---|

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-vs-epoll-spin-burst50

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| md1 | 7794500 | 3.19 | 11.12 | 19.57 | 30.20 | 45.25 | 0.0 | 0.0 | 0.0 | 0.0 | 854.32 | 22974.0 | 854.32 µs stall: loop work (2750 items) |
| md2 | 7794600 | 3.86 | 12.05 | 24.01 | 35.50 | 58.02 | 0.0 | 0.0 | 0.0 | 0.0 | 617.15 | 11530.0 | 617.15 µs stall: loop work (2700 items) |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-vs-epoll-spin-single

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| md1 | 176000 | 3.74 | 13.37 | 16.09 | 30.80 | 41.87 | 0.0 | 0.0 | 0.0 | 0.0 | 69.20 | 274.0 | 69.20 µs stall: mostly unexplained (+softirq NET_RX) |
| md2 | 176000 | 3.88 | 14.38 | 31.38 | 33.21 | 40.42 | 0.0 | 0.0 | 0.0 | 0.0 | 50.53 | 394.7 | 50.53 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-vs-recv-busy-burst50

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| md1 | 7794500 | 3.64 | 11.42 | 20.39 | 35.41 | 51.46 | 0.0 | 0.0 | 0.0 | 0.0 | 729.77 | 10204.7 | 729.77 µs stall: loop work (2703 items) |
| md2 | 7794684 | 2.65 | 6.22 | 15.12 | 24.86 | 53.06 | 0.0 | 0.0 | 0.0 | 0.0 | 760.30 | 7585.3 | 760.30 µs stall: loop work (2650 items) |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-vs-recv-busy-single

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| md1 | 176000 | 3.75 | 7.21 | 8.88 | 32.87 | 65.90 | 0.0 | 0.0 | 0.0 | 0.0 | 342.28 | 257.3 | 342.28 µs stall: softirq TIMER |
| md2 | 176000 | 2.80 | 4.77 | 5.96 | 7.89 | 27.50 | 0.0 | 0.0 | 0.0 | 0.0 | 64.41 | 193.3 | 64.41 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-vs-uring-spin-burst50

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| md1 | 7794700 | 3.84 | 14.92 | 18.62 | 29.61 | 47.74 | 0.0 | 0.0 | 0.0 | 0.0 | 841.60 | 6663.3 | 841.60 µs stall: loop work (2650 items) |
| md2 | 7794600 | 3.59 | 9.85 | 16.45 | 33.27 | 52.24 | 0.0 | 0.0 | 0.0 | 0.0 | 568.40 | 14968.7 | 568.40 µs stall: loop work (2700 items) |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-vs-uring-spin-single

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| md1 | 176000 | 3.74 | 11.17 | 32.77 | 41.56 | 45.49 | 0.0 | 0.0 | 0.0 | 0.0 | 65.13 | 1115.3 | 65.13 µs stall: softirq TIMER |
| md2 | 176000 | 4.87 | 9.22 | 23.80 | 28.42 | 131.75 | 0.7 | 0.0 | 0.0 | 0.0 | 249.20 | 601.3 | 249.20 µs stall: softirq TIMER |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-vs-uring-sqpoll-burst50

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| md1 | 7794600 | 3.13 | 7.12 | 12.63 | 27.76 | 46.60 | 0.0 | 0.0 | 0.0 | 0.0 | 658.37 | 9686.7 | 658.37 µs stall: loop work (2700 items) |
| md2 | 7794606 | 3.80 | 10.16 | 15.95 | 26.52 | 41.27 | 0.0 | 0.0 | 0.0 | 0.0 | 622.89 | 12399.3 | 622.89 µs stall: loop work (2654 items) |

## Market-data burst, kernel arrival → decoded (every message), lab-recv-busy-vs-uring-sqpoll-single

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| md1 | 176000 | 3.09 | 4.90 | 6.11 | 18.93 | 23.83 | 0.0 | 0.0 | 0.0 | 0.0 | 71.36 | 188.7 | 71.36 µs stall: mostly unexplained (+softirq NET_RX) |
| md2 | 176000 | 3.47 | 6.24 | 7.77 | 20.97 | 34.31 | 0.0 | 0.0 | 0.0 | 0.0 | 48.96 | 458.0 | 48.96 µs stall: mostly unexplained (+preempted by kworker/2:1) |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-vs-recv-busy-burst50

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| md1 | 7794600 | 3.76 | 9.39 | 14.29 | 24.39 | 36.75 | 0.0 | 0.0 | 0.0 | 0.0 | 383.03 | 2.7 | 383.03 µs stall: loop work (2699 items) |
| md2 | 7794500 | 2.94 | 5.82 | 10.91 | 17.24 | 33.41 | 0.0 | 0.0 | 0.0 | 0.0 | 436.30 | 2.0 | 436.30 µs stall: loop work (2750 items) |

## Market-data burst, kernel arrival → decoded (every message), lab-uring-sqpoll-vs-recv-busy-single

| condition | n | p50 | p99 | p99.9 | p99.99 | max | >100 µs /min | >500 µs /min | >1 ms /min | >10 ms /min | longest loop iteration | loop stalls >20 µs /min | worst case |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| md1 | 176001 | 4.08 | 7.56 | 17.54 | 30.12 | 34.38 | 0.0 | 0.0 | 0.0 | 0.0 | 55.73 | 1.3 | 55.73 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |
| md2 | 176000 | 3.57 | 6.37 | 7.88 | 21.04 | 35.50 | 0.0 | 0.0 | 0.0 | 0.0 | 35.00 | 2.0 | 35.00 µs stall: unexplained: no kernel event (hypervisor or in-kernel time) |

## Interrupts per second on the loop CPU and the receiving CPU

From `/proc/interrupts` around each run (so including its start-up): local timer (LOC), irq_work (IWI: on a `nohz_full` CPU, mostly the tick being restarted), rescheduling (RES) and function-call (CAL) IPIs, and system-management interrupts (SMI, bare metal only: firmware pauses). The receiving CPU is the one that last took each connection's packets when that is not the loop's: the SQPOLL thread's, or a softirq's.

| run | condition | loop CPU | LOC | IWI | RES | CAL | SMI | receiving CPU | LOC | IWI | RES | CAL | SMI |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| lab-recv-busy-vs-epoll-spin-burst50 | md1 | 2 | 43 | 39 | 0 | 11 | 0 |  |  |  |  |  |  |
| lab-recv-busy-vs-epoll-spin-burst50 | md2 | 2 | 24 | 24 | 0 | 11 | 0 | 4 | 1008 | 0 | 0 | 2 | 0 |
| lab-recv-busy-vs-epoll-spin-single | md1 | 2 | 2 | 2 | 0 | 11 | 0 | 4 | 79 | 0 | 0 | 15 | 0 |
| lab-recv-busy-vs-epoll-spin-single | md2 | 2 | 2 | 2 | 0 | 11 | 0 | 0 | 1007 | 0 | 0 | 2 | 0 |
| lab-recv-busy-vs-recv-busy-burst50 | md1 | 2 | 25 | 25 | 0 | 11 | 0 | 3 | 25 | 25 | 0 | 11 | 0 |
| lab-recv-busy-vs-recv-busy-burst50 | md2 | 2 | 24 | 22 | 0 | 11 | 0 | 3 | 24 | 22 | 0 | 11 | 0 |
| lab-recv-busy-vs-recv-busy-single | md1 | 2 | 2 | 2 | 0 | 11 | 0 | 3 | 2 | 2 | 0 | 11 | 0 |
| lab-recv-busy-vs-recv-busy-single | md2 | 2 | 2 | 2 | 0 | 11 | 0 | 3 | 2 | 2 | 0 | 11 | 0 |
| lab-recv-busy-vs-uring-spin-burst50 | md1 | 2 | 20 | 20 | 0 | 11 | 0 | 0 | 1012 | 0 | 1 | 10 | 0 |
| lab-recv-busy-vs-uring-spin-burst50 | md2 | 2 | 26 | 25 | 0 | 11 | 0 |  |  |  |  |  |  |
| lab-recv-busy-vs-uring-spin-single | md1 | 2 | 6 | 6 | 0 | 11 | 0 |  |  |  |  |  |  |
| lab-recv-busy-vs-uring-spin-single | md2 | 2 | 3 | 4 | 0 | 11 | 0 | 4 | 78 | 0 | 0 | 9 | 0 |
| lab-recv-busy-vs-uring-sqpoll-burst50 | md1 | 2 | 24 | 24 | 0 | 11 | 0 | 4 | 605 | 0 | 0 | 5 | 0 |
| lab-recv-busy-vs-uring-sqpoll-burst50 | md2 | 2 | 32 | 31 | 0 | 11 | 0 | 1 | 60 | 29 | 0 | 14 | 0 |
| lab-recv-busy-vs-uring-sqpoll-single | md1 | 2 | 2 | 2 | 0 | 11 | 0 | 1 | 15 | 4 | 2 | 11 | 0 |
| lab-recv-busy-vs-uring-sqpoll-single | md2 | 2 | 2 | 2 | 0 | 11 | 0 | 1 | 15 | 4 | 2 | 11 | 0 |
| lab-uring-sqpoll-vs-recv-busy-burst50 | md1 | 2 | 0 | 0 | 0 | 11 | 0 | 3 | 31 | 31 | 0 | 11 | 0 |
| lab-uring-sqpoll-vs-recv-busy-burst50 | md2 | 2 | 0 | 0 | 0 | 11 | 0 | 1 | 61 | 28 | 1 | 14 | 0 |
| lab-uring-sqpoll-vs-recv-busy-single | md1 | 2 | 0 | 0 | 0 | 11 | 0 | 1 | 14 | 4 | 2 | 11 | 0 |
| lab-uring-sqpoll-vs-recv-busy-single | md2 | 2 | 0 | 0 | 0 | 11 | 0 | 1 | 15 | 4 | 2 | 11 | 0 |

## Worst events

The five slowest samples of every run (noise: longest gaps): value in µs, seconds into the run, and cause.

### lab-recv-busy-vs-epoll-spin-burst50

| condition | worst events |
|---|---|
| md1 | 45.25 at 48.8 s: client: loop work (6 items); 45.09 at 48.8 s: client: loop work (6 items); 44.88 at 48.8 s: client: loop work (6 items); 44.37 at 45.7 s: client: softirq NET_RX; 44.21 at 45.7 s: client: softirq NET_RX |
| md2 | 58.02 at 77.8 s: neither loop stalled long enough: network, hypervisor or untraced; 57.83 at 77.8 s: neither loop stalled long enough: network, hypervisor or untraced; 57.64 at 77.8 s: neither loop stalled long enough: network, hypervisor or untraced; 57.47 at 77.8 s: neither loop stalled long enough: network, hypervisor or untraced; 57.30 at 77.8 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-recv-busy-vs-epoll-spin-single

| condition | worst events |
|---|---|
| md1 | 41.87 at 39.6 s: neither loop stalled long enough: network, hypervisor or untraced; 39.73 at 49.7 s: neither loop stalled long enough: network, hypervisor or untraced; 35.75 at 50.2 s: neither loop stalled long enough: network, hypervisor or untraced; 33.43 at 49.7 s: client: mostly unexplained (+preempted by kworker/2:1); 33.40 at 81.0 s: neither loop stalled long enough: network, hypervisor or untraced |
| md2 | 40.42 at 73.4 s: neither loop stalled long enough: network, hypervisor or untraced; 37.30 at 63.5 s: neither loop stalled long enough: network, hypervisor or untraced; 37.14 at 36.1 s: neither loop stalled long enough: network, hypervisor or untraced; 36.28 at 34.2 s: neither loop stalled long enough: network, hypervisor or untraced; 36.05 at 4.2 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-recv-busy-vs-recv-busy-burst50

| condition | worst events |
|---|---|
| md1 | 51.46 at 63.4 s: client: loop work (13 items); 51.46 at 43.9 s: client: softirq NET_RX; 51.33 at 63.4 s: client: loop work (13 items); 51.32 at 43.9 s: client: softirq NET_RX; 51.19 at 63.4 s: client: loop work (13 items) |
| md2 | 53.06 at 6.8 s: neither loop stalled long enough: network, hypervisor or untraced; 48.68 at 31.6 s: client: loop work (4 items); 48.53 at 31.6 s: client: loop work (4 items); 48.39 at 31.6 s: client: loop work (4 items); 48.25 at 31.6 s: client: loop work (4 items) |

### lab-recv-busy-vs-recv-busy-single

| condition | worst events |
|---|---|
| md1 | 65.90 at 35.4 s: client: softirq TIMER; 39.04 at 30.7 s: client: mostly unexplained (+preempted by kworker/2:1); 38.11 at 62.5 s: client: mostly unexplained (+preempted by kworker/2:1); 37.92 at 37.2 s: client: mostly unexplained (+preempted by kworker/2:1); 37.60 at 43.4 s: client: mostly unexplained (+softirq NET_RX) |
| md2 | 27.50 at 85.1 s: neither loop stalled long enough: network, hypervisor or untraced; 22.33 at 60.5 s: client: mostly unexplained (+preempted by kworker/2:1); 21.61 at 59.0 s: client: mostly unexplained (+preempted by kworker/2:1); 20.86 at 85.3 s: neither loop stalled long enough: network, hypervisor or untraced; 18.60 at 24.3 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-recv-busy-vs-uring-spin-burst50

| condition | worst events |
|---|---|
| md1 | 47.74 at 4.8 s: neither loop stalled long enough: network, hypervisor or untraced; 47.58 at 4.8 s: neither loop stalled long enough: network, hypervisor or untraced; 47.42 at 4.8 s: neither loop stalled long enough: network, hypervisor or untraced; 47.25 at 4.8 s: neither loop stalled long enough: network, hypervisor or untraced; 47.09 at 4.8 s: neither loop stalled long enough: network, hypervisor or untraced |
| md2 | 52.24 at 48.4 s: client: loop work (8 items); 52.10 at 48.4 s: client: loop work (8 items); 51.95 at 48.4 s: client: loop work (8 items); 51.81 at 48.4 s: client: loop work (8 items); 51.67 at 48.4 s: client: loop work (8 items) |

### lab-recv-busy-vs-uring-spin-single

| condition | worst events |
|---|---|
| md1 | 45.49 at 55.1 s: client: mostly unexplained (+softirq NET_RX); 45.35 at 36.2 s: client: mostly unexplained (+softirq NET_RX); 44.89 at 67.4 s: client: mostly unexplained (+softirq NET_RX); 44.08 at 72.1 s: client: mostly unexplained (+softirq NET_RX); 43.83 at 71.6 s: client: mostly unexplained (+softirq NET_RX) |
| md2 | 131.75 at 80.9 s: client: softirq TIMER; 81.24 at 80.9 s: client: softirq TIMER; 30.96 at 68.0 s: client: mostly unexplained (+preempted by kworker/2:1); 30.90 at 4.6 s: client: mostly unexplained (+preempted by kworker/2:1); 30.51 at 14.9 s: client: mostly unexplained (+preempted by kworker/2:1) |

### lab-recv-busy-vs-uring-sqpoll-burst50

| condition | worst events |
|---|---|
| md1 | 46.60 at 70.7 s: client: loop work (7 items); 46.41 at 70.7 s: client: loop work (7 items); 46.23 at 70.7 s: client: loop work (7 items); 46.05 at 70.7 s: client: loop work (7 items); 45.85 at 70.7 s: client: loop work (7 items) |
| md2 | 41.27 at 70.0 s: client: mostly unexplained (+softirq NET_RX); 40.28 at 11.4 s: client: unexplained: no kernel event (hypervisor or in-kernel time); 40.18 at 35.5 s: client: loop work (13 items); 39.86 at 35.5 s: client: loop work (13 items); 38.80 at 62.7 s: client: mostly unexplained (+local timer tick) |

### lab-recv-busy-vs-uring-sqpoll-single

| condition | worst events |
|---|---|
| md1 | 23.83 at 18.1 s: client: mostly unexplained (+preempted by kworker/2:1); 22.44 at 3.3 s: client: softirq NET_RX; 22.41 at 88.1 s: neither loop stalled long enough: network, hypervisor or untraced; 21.84 at 68.6 s: client: softirq NET_RX; 21.23 at 32.5 s: client: mostly unexplained (+preempted by kworker/2:1) |
| md2 | 34.31 at 7.5 s: client: softirq NET_RX; 31.51 at 41.3 s: client: mostly unexplained (+preempted by kworker/2:1); 31.16 at 66.9 s: client: mostly unexplained (+preempted by kworker/2:1); 29.65 at 9.7 s: client: mostly unexplained (+preempted by kworker/2:1); 28.79 at 86.4 s: client: mostly unexplained (+preempted by kworker/2:1) |

### lab-uring-sqpoll-vs-recv-busy-burst50

| condition | worst events |
|---|---|
| md1 | 36.75 at 24.3 s: neither loop stalled long enough: network, hypervisor or untraced; 36.14 at 44.5 s: neither loop stalled long enough: network, hypervisor or untraced; 35.96 at 44.5 s: neither loop stalled long enough: network, hypervisor or untraced; 35.77 at 44.5 s: neither loop stalled long enough: network, hypervisor or untraced; 35.55 at 44.5 s: neither loop stalled long enough: network, hypervisor or untraced |
| md2 | 33.41 at 2.3 s: neither loop stalled long enough: network, hypervisor or untraced; 32.98 at 27.7 s: neither loop stalled long enough: network, hypervisor or untraced; 30.72 at 5.6 s: neither loop stalled long enough: network, hypervisor or untraced; 29.91 at 6.0 s: neither loop stalled long enough: network, hypervisor or untraced; 29.72 at 6.0 s: neither loop stalled long enough: network, hypervisor or untraced |

### lab-uring-sqpoll-vs-recv-busy-single

| condition | worst events |
|---|---|
| md1 | 34.38 at 81.2 s: neither loop stalled long enough: network, hypervisor or untraced; 34.09 at 86.4 s: neither loop stalled long enough: network, hypervisor or untraced; 34.03 at 34.9 s: neither loop stalled long enough: network, hypervisor or untraced; 33.48 at 84.6 s: neither loop stalled long enough: network, hypervisor or untraced; 33.23 at 81.4 s: neither loop stalled long enough: network, hypervisor or untraced |
| md2 | 35.50 at 82.1 s: neither loop stalled long enough: network, hypervisor or untraced; 35.40 at 45.2 s: neither loop stalled long enough: network, hypervisor or untraced; 32.09 at 39.1 s: neither loop stalled long enough: network, hypervisor or untraced; 32.01 at 63.1 s: neither loop stalled long enough: network, hypervisor or untraced; 30.14 at 38.0 s: neither loop stalled long enough: network, hypervisor or untraced |

## Stall time by cause (all runs of a condition, loop iterations over 20 µs)

| condition | causes (total µs) |
|---|---|
| md1 | loop work 2083626.33; unexplained: no kernel event (hypervisor or in-kernel time) 173517.15; mostly unexplained (+preempted by kworker/2:1) 51075.63; softirq NET_RX 45541.58; mostly unexplained (+softirq NET_RX) 27789.52 |
| md2 | loop work 1875307.48; unexplained: no kernel event (hypervisor or in-kernel time) 154100.32; softirq NET_RX 43861.47; mostly unexplained (+preempted by kworker/2:1) 41206.00; mostly unexplained (+softirq NET_RX) 20920.94 |

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
| cpumap-sender/cpu1 | counter-only spinner | 2 | :07 | 1 (50%) | 1 of 2 | 427.16 | no |
| cpumap-sender/cpu3 | counter-only spinner | 2 | :07 | 1 (50%) | 1 of 2 | 423.99 | no |
| lab-recv-busy-vs-epoll-spin-burst50 | receiver loop | 0 | | | | | |
| lab-recv-busy-vs-epoll-spin-burst50 | slow samples | 0 | | | | | |
| lab-recv-busy-vs-epoll-spin-single | receiver loop | 0 | | | | | |
| lab-recv-busy-vs-epoll-spin-single | slow samples | 0 | | | | | |
| lab-recv-busy-vs-recv-busy-burst50 | receiver loop | 0 | | | | | |
| lab-recv-busy-vs-recv-busy-burst50 | slow samples | 0 | | | | | |
| lab-recv-busy-vs-recv-busy-single | receiver loop | 1 | :19 | 1 (100%) | 1 of 2 | 342.28 | no |
| lab-recv-busy-vs-recv-busy-single | slow samples | 0 | | | | | |
| lab-recv-busy-vs-uring-spin-burst50 | receiver loop | 0 | | | | | |
| lab-recv-busy-vs-uring-spin-burst50 | slow samples | 0 | | | | | |
| lab-recv-busy-vs-uring-spin-single | receiver loop | 0 | | | | | |
| lab-recv-busy-vs-uring-spin-single | slow samples | 0 | | | | | |
| lab-recv-busy-vs-uring-sqpoll-burst50 | receiver loop | 0 | | | | | |
| lab-recv-busy-vs-uring-sqpoll-burst50 | slow samples | 0 | | | | | |
| lab-recv-busy-vs-uring-sqpoll-single | receiver loop | 0 | | | | | |
| lab-recv-busy-vs-uring-sqpoll-single | slow samples | 0 | | | | | |
| lab-uring-sqpoll-vs-recv-busy-burst50 | receiver loop | 1 | :47 | 1 (100%) | 1 of 3 | 368.86 | no |
| lab-uring-sqpoll-vs-recv-busy-burst50 | slow samples | 0 | | | | | |
| lab-uring-sqpoll-vs-recv-busy-single | receiver loop | 0 | | | | | |
| lab-uring-sqpoll-vs-recv-busy-single | slow samples | 0 | | | | | |

# Every run in detail

## Summary: paired runs (µs; Δ = B − A, negative means B decoded first)

| run | A | B | paired msgs | Δuser p50 | Δuser p90 | Δuser p99 | Δuser p99.9 | Δuser max | Δrx p50 | Δstack p50 | Δstack p99 | A syscalls/s | B syscalls/s | A kernel CPU % | B kernel CPU % | shared RX queue |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| md1/lab-recv-busy-vs-epoll-spin-burst50 | recv-busy | epoll-spin | 3897250 | -0.34 | 9.22 | 19.19 | 33.52 | 253.03 | 0.06 | 0.66 | 4.77 | 1634198 | 3924644 | 68.7 | 36.2 | yes |
| md1/lab-recv-busy-vs-epoll-spin-single | recv-busy | epoll-spin | 88000 | 7.20 | 15.30 | 24.27 | 57.76 | 174.36 | 7.73 | 1.01 | 10.50 | 1912388 | 4184970 | 64.6 | 33.4 | no |
| md1/lab-recv-busy-vs-recv-busy-burst50 | recv-busy | recv-busy | 3897250 | 0.96 | 13.37 | 28.32 | 49.68 | 172.14 | 0.13 | -0.34 | 4.16 | 1466806 | 1487159 | 72.0 | 71.9 | yes |
| md1/lab-recv-busy-vs-recv-busy-single | recv-busy | recv-busy | 88000 | 0.42 | 2.30 | 5.82 | 56.99 | 103.84 | 0.65 | -0.15 | 2.07 | 1914337 | 1917779 | 64.8 | 64.9 | no |
| md1/lab-recv-busy-vs-uring-spin-burst50 | recv-busy | uring-spin | 3897350 | 29.14 | 50.82 | 65.91 | 82.26 | 1011.75 | 21.07 | 3.59 | 11.59 | 1764122 | 22796 | 66.7 | 4.6 | no |
| md1/lab-recv-busy-vs-uring-spin-single | recv-busy | uring-spin | 88000 | 0.92 | 2.45 | 15.39 | 32.92 | 71.43 | -0.03 | 1.15 | 10.98 | 1893238 | 2000 | 64.9 | 0.3 | yes |
| md1/lab-recv-busy-vs-uring-sqpoll-burst50 | recv-busy | uring-sqpoll | 3897300 | 0.80 | 15.09 | 34.43 | 45.40 | 10494.18 | 0.12 | 0.18 | 3.07 | 1784441 | 0 | 66.2 | 0.0 | no |
| md1/lab-recv-busy-vs-uring-sqpoll-single | recv-busy | uring-sqpoll | 88000 | 0.40 | 2.86 | 5.21 | 58.45 | 83.96 | 0.21 | 0.33 | 1.51 | 1916846 | 0 | 64.6 | 0.0 | no |
| md1/lab-uring-sqpoll-vs-recv-busy-burst50 | uring-sqpoll | recv-busy | 3897300 | -4.94 | 3.94 | 9.96 | 27.50 | 101.46 | -3.98 | -0.80 | 4.12 | 0 | 1065596 | 0.0 | 80.0 | yes |
| md1/lab-uring-sqpoll-vs-recv-busy-single | uring-sqpoll | recv-busy | 88000 | -0.79 | 1.43 | 5.18 | 26.52 | 77.87 | 0.05 | -0.86 | 1.62 | 0 | 1914307 | 0.0 | 65.0 | no |
| md2/lab-recv-busy-vs-epoll-spin-burst50 | recv-busy | epoll-spin | 3897300 | 25.86 | 51.28 | 75.72 | 91.95 | 201.18 | 19.93 | 2.92 | 9.60 | 1751676 | 4020809 | 66.7 | 35.3 | no |
| md2/lab-recv-busy-vs-epoll-spin-single | recv-busy | epoll-spin | 87999 | 70.68 | 174.68 | 220.74 | 259.56 | 379.80 | 108.85 | 9.94 | 25.89 | 1913249 | 4172856 | 64.5 | 33.7 | no |
| md2/lab-recv-busy-vs-recv-busy-burst50 | recv-busy | recv-busy | 3897342 | -0.29 | 10.02 | 29.32 | 41.74 | 119.20 | -0.27 | 0.01 | 3.24 | 1782324 | 1783593 | 66.4 | 66.5 | no |
| md2/lab-recv-busy-vs-recv-busy-single | recv-busy | recv-busy | 88000 | -0.44 | 1.83 | 4.35 | 52.80 | 73.39 | -0.16 | -0.28 | 0.93 | 1927942 | 1928306 | 64.6 | 64.7 | no |
| md2/lab-recv-busy-vs-uring-spin-burst50 | recv-busy | uring-spin | 3897300 | -1.11 | 10.76 | 24.47 | 37.00 | 117.29 | -2.41 | 1.35 | 4.28 | 1737302 | 27275 | 67.1 | 3.6 | yes |
| md2/lab-recv-busy-vs-uring-spin-single | recv-busy | uring-spin | 88000 | 7.22 | 9.82 | 21.45 | 37.60 | 70.72 | 5.88 | 1.52 | 7.52 | 1909489 | 2000 | 64.7 | 0.3 | no |
| md2/lab-recv-busy-vs-uring-sqpoll-burst50 | recv-busy | uring-sqpoll | 3897303 | 0.14 | 8.62 | 17.35 | 33.17 | 126.16 | -0.03 | 0.57 | 5.45 | 1227952 | 0 | 76.5 | 0.0 | yes |
| md2/lab-recv-busy-vs-uring-sqpoll-single | recv-busy | uring-sqpoll | 88000 | 0.66 | 2.16 | 11.35 | 28.20 | 112.17 | -0.19 | 0.85 | 2.33 | 1908948 | 0 | 64.8 | 0.0 | no |
| md2/lab-uring-sqpoll-vs-recv-busy-burst50 | uring-sqpoll | recv-busy | 3897250 | 0.09 | 14.56 | 33.23 | 42.27 | 124.63 | 1.33 | -0.94 | 1.07 | 0 | 1790354 | 0.0 | 66.4 | no |
| md2/lab-uring-sqpoll-vs-recv-busy-single | uring-sqpoll | recv-busy | 88000 | -0.53 | 1.44 | 3.99 | 47.65 | 71.73 | 0.05 | -0.67 | 1.28 | 0 | 1913250 | 0.0 | 64.9 | no |

## md1/lab-recv-busy-vs-epoll-spin-burst50: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2792934 clocksource=tsc

### Loop counters (md1/lab-recv-busy-vs-epoll-spin-burst50)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4502750 | 1932993 | 2.33 | 147077824 | 32.66 | 1634198 | 1634198 | 98.7 | 68.7 | true | 0 | 3512 | 854.3 | 907301 / 34461 / 1 | 502750 | 0 |
| epoll-spin | 4502740 | 2130498 | 2.11 | 353217922 | 78.45 | 3924644 | 3900971 | 0.0 | 36.2 | false | 0 | 0 | 882.0 | 63789 / 42 / 1 | 502740 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 2 |
| epoll-spin | 0 | 8193 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1008616000 | 0 | 0 | 0 | 0.000 |
| epoll-spin | 1008613760 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (555525 of 3897250 in 1-msg reads) | 555525 | 3.33 | 2.53 | 5.37 | 10.13 | 22.81 | 33.31 | 43.09 |
| epoll-spin (719473 of 3897250 in 1-msg reads) | 719473 | 3.98 | 3.24 | 6.21 | 10.24 | 17.76 | 22.78 | 34.87 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **epoll-spin Δuser (write-order corrected)** | 3897250 | 1.04 | -0.34 | 9.22 | 19.19 | 33.52 | 78.71 | 253.03 |
| epoll-spin Δuser, even seq | 1948625 | 2.12 | 0.33 | 10.65 | 20.98 | 35.18 | 80.23 | 253.70 |
| epoll-spin Δuser, odd seq | 1948625 | -0.05 | -1.00 | 7.83 | 17.10 | 31.66 | 77.05 | 248.42 |
| epoll-spin Δrx | 3897250 | 0.88 | 0.06 | 8.68 | 18.28 | 31.91 | 75.05 | 246.65 |
| epoll-spin Δstack (1-msg reads) | 288242 | 0.31 | 0.66 | 2.35 | 4.77 | 11.83 | 17.02 | 31.39 |

epoll-spin: 3897250 paired messages; decoded before recv-busy on 49.6 % (uncorrected).

#### Stalls: md1/lab-recv-busy-vs-epoll-spin-burst50, loop 0 on CPU 2

147077824 iterations, longest 854.3 µs; over 5 / 20 / 100 µs: 907301 / 34461 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 30302 | 949878.0 | 854.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2513 | 62316.7 | 94.5 |
| mostly unexplained (+preempted by kworker/2:1) | 703 | 18319.4 | 57.4 |
| softirq NET_RX | 595 | 16425.9 | 59.0 |
| mostly unexplained (+softirq NET_RX) | 228 | 5923.6 | 44.7 |
| mostly unexplained (+local timer tick) | 113 | 3012.8 | 45.3 |
| mostly unexplained (+irq work (IPI/vector)) | 7 | 166.9 | 31.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 854.3 | 2750 | loop work (2750 items) |  |
| 2 | 98.6 | 26 | loop work (26 items) | softirq NET_RX 18, local timer tick 5, timer tick_nohz_handler 2 |
| 3 | 94.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 85.1 | 17 | loop work (17 items) | softirq NET_RX 16, local timer tick 3, preempted by kworker/2:1 1, timer tick_nohz_handler 1 |
| 5 | 83.7 | 20 | loop work (20 items) | softirq NET_RX 25, local timer tick 4, preempted by kworker/2:1 1, timer tick_nohz_handler 1 |
| 6 | 80.3 | 8 | loop work (8 items) | softirq NET_RX 21, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 78.7 | 21 | loop work (21 items) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 78.6 | 20 | loop work (20 items) | softirq NET_RX 12, local timer tick 4, timer tick_nohz_handler 2, preempted by kworker/2:1 1 |
| 9 | 77.0 | 10 | loop work (10 items) | softirq NET_RX 38, local timer tick 4, timer tick_nohz_handler 1 |
| 10 | 76.3 | 17 | loop work (17 items) | softirq NET_RX 12, local timer tick 4, preempted by kworker/2:1 1, timer tick_nohz_handler 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 3852 |
| IWI IRQ work interrupts | 3527 |
| CAL Function call interrupts | 995 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md1/lab-recv-busy-vs-epoll-spin-burst50, loop 1 on CPU 3

351087424 iterations, longest 882.0 µs; over 5 / 20 / 100 µs: 63789 / 42 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 27 | 1449.4 | 882.0 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 15 | 394.8 | 70.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 882.0 | 2743 | loop work (2743 items) |  |
| 2 | 70.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 34.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 28.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 28.0 | 2 | loop work (2 items) |  |
| 6 | 25.9 | 3 | loop work (3 items) |  |
| 7 | 25.6 | 4 | loop work (4 items) |  |
| 8 | 24.7 | 3 | loop work (3 items) |  |
| 9 | 23.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 23.2 | 3 | loop work (3 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## md1/lab-recv-busy-vs-epoll-spin-single: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=79324 clocksource=tsc

### Loop counters (md1/lab-recv-busy-vs-epoll-spin-single)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 90054 | 90001 | 1.00 | 172114944 | 1911.24 | 1912388 | 1912388 | 99.9 | 64.6 | true | 0 | 195 | 69.2 | 22610 / 411 / 0 | 0 | 0 |
| epoll-spin | 90054 | 90001 | 1.00 | 376647313 | 4182.46 | 4184970 | 4183970 | 0.0 | 33.4 | false | 0 | 0 | 43.0 | 9237 / 22 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 2 |
| epoll-spin | 0 | 8198 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 20172096 | 0 | 0 | 0 | 0.000 |
| epoll-spin | 20172096 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (88000 of 88000 in 1-msg reads) | 88000 | 3.29 | 2.96 | 4.73 | 6.50 | 9.42 | 30.80 | 33.43 |
| epoll-spin (88000 of 88000 in 1-msg reads) | 88000 | 4.95 | 4.16 | 8.17 | 13.71 | 16.15 | 31.43 | 41.87 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **epoll-spin Δuser (write-order corrected)** | 88000 | 8.20 | 7.20 | 15.30 | 24.27 | 57.76 | 164.18 | 174.36 |
| epoll-spin Δuser, even seq | 44000 | 5.83 | 4.67 | 17.26 | 21.77 | 42.28 | 142.11 | 168.51 |
| epoll-spin Δuser, odd seq | 44000 | 10.57 | 9.72 | 13.41 | 26.66 | 121.45 | 169.47 | 176.88 |
| epoll-spin Δrx | 88000 | 6.54 | 7.73 | 10.23 | 19.14 | 50.09 | 156.39 | 164.83 |
| epoll-spin Δstack (1-msg reads) | 88000 | 1.66 | 1.01 | 5.08 | 10.50 | 11.86 | 27.92 | 38.29 |

epoll-spin: 88000 paired messages; decoded before recv-busy on 4.2 % (uncorrected).

#### Stalls: md1/lab-recv-busy-vs-epoll-spin-single, loop 0 on CPU 2

172114944 iterations, longest 69.2 µs; over 5 / 20 / 100 µs: 22610 / 411 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 218 | 11634.5 | 67.5 |
| mostly unexplained (+preempted by kworker/2:1) | 116 | 2977.3 | 38.0 |
| mostly unexplained (+softirq NET_RX) | 44 | 2134.1 | 69.2 |
| softirq NET_RX | 29 | 683.9 | 33.7 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 86.6 | 34.3 |
| loop work | 1 | 30.4 | 30.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 69.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 67.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 67.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 66.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 66.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 66.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 66.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 65.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 65.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 65.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 997 |
| IWI IRQ work interrupts | 214 |
| LOC Local timer interrupts | 197 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Stalls: md1/lab-recv-busy-vs-epoll-spin-single, loop 1 on CPU 3

376557312 iterations, longest 43.0 µs; over 5 / 20 / 100 µs: 9237 / 22 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 21 | 524.4 | 38.5 |
| loop work | 1 | 43.0 | 43.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 43.0 | 54 | loop work (54 items) |  |
| 2 | 38.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 34.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 31.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 26.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 26.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 25.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 25.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 24.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 24.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| LOC Local timer interrupts | 5 |
| IWI IRQ work interrupts | 3 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

## md1/lab-recv-busy-vs-recv-busy-burst50: lab (A/A control), 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1979386 clocksource=tsc

### Loop counters (md1/lab-recv-busy-vs-recv-busy-burst50)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4502700 | 1452424 | 3.10 | 132012544 | 29.32 | 1466806 | 1466806 | 98.9 | 72.0 | true | 0 | 2228 | 729.8 | 475710 / 15307 / 2 | 502700 | 0 |
| recv-busy | 4502735 | 1519120 | 2.96 | 133844288 | 29.73 | 1487159 | 1487159 | 98.9 | 71.9 | true | 0 | 2239 | 656.8 | 455686 / 14610 / 1 | 502735 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 3 |
| recv-busy | 0 | 8199 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1008604800 | 0 | 0 | 0 | 0.000 |
| recv-busy | 1008612640 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy #0 (321825 of 3897250 in 1-msg reads) | 321825 | 4.00 | 3.27 | 6.01 | 9.74 | 13.81 | 32.83 | 47.86 |
| recv-busy #1 (395845 of 3897250 in 1-msg reads) | 395845 | 3.91 | 3.48 | 5.96 | 8.83 | 14.37 | 31.03 | 50.26 |

### Paired by message, relative to recv-busy (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **recv-busy #1 Δuser (write-order corrected)** | 3897250 | 0.96 | 0.96 | 13.37 | 28.32 | 49.68 | 73.89 | 172.14 |
| recv-busy #1 Δuser, even seq | 1948625 | -0.02 | -0.05 | 12.57 | 26.82 | 48.55 | 71.70 | 171.12 |
| recv-busy #1 Δuser, odd seq | 1948625 | 1.93 | 1.98 | 14.15 | 29.86 | 50.74 | 75.83 | 165.76 |
| recv-busy #1 Δrx | 3897250 | 0.95 | 0.13 | 13.03 | 27.34 | 46.36 | 69.79 | 170.36 |
| recv-busy #1 Δstack (1-msg reads) | 148684 | -0.50 | -0.34 | 1.73 | 4.16 | 11.39 | 29.69 | 45.14 |

recv-busy #1: 3897250 paired messages; decoded before recv-busy on 46.2 % (uncorrected).

#### Stalls: md1/lab-recv-busy-vs-recv-busy-burst50, loop 0 on CPU 2

132012544 iterations, longest 729.8 µs; over 5 / 20 / 100 µs: 475710 / 15307 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 13471 | 507295.0 | 729.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 971 | 28304.8 | 47.6 |
| softirq NET_RX | 540 | 17344.5 | 80.5 |
| mostly unexplained (+preempted by kworker/2:1) | 224 | 5878.8 | 38.4 |
| mostly unexplained (+softirq NET_RX) | 85 | 2746.9 | 60.8 |
| mostly unexplained (+local timer tick) | 11 | 310.4 | 48.6 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 68.3 | 34.5 |
| local timer tick | 3 | 62.7 | 21.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 729.8 | 2703 | loop work (2703 items) |  |
| 2 | 106.6 | 15 | loop work (15 items) | softirq NET_RX 27, local timer tick 8, timer tick_nohz_handler 6, preempted by kworker/2:1 5 |
| 3 | 90.8 | 25 | loop work (25 items) | softirq NET_RX 9 |
| 4 | 89.7 | 17 | loop work (17 items) | softirq NET_RX 22, preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 88.9 | 14 | loop work (14 items) | softirq NET_RX 25, local timer tick 4, preempted by kworker/2:1 4, timer tick_nohz_handler 2 |
| 6 | 88.5 | 23 | loop work (23 items) | softirq NET_RX 21, preempted by ksoftirqd/2 5, local timer tick 4, timer tick_nohz_handler 2 |
| 7 | 86.2 | 20 | loop work (20 items) | softirq NET_RX 8 |
| 8 | 85.0 | 20 | loop work (20 items) | softirq NET_RX 21, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 83.2 | 13 | loop work (13 items) | softirq NET_RX 15 |
| 10 | 82.8 | 13 | loop work (13 items) | softirq NET_RX 34, preempted by ksoftirqd/2 9, local timer tick 7, timer tick_nohz_handler 5 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2282 |
| IWI IRQ work interrupts | 2236 |
| CAL Function call interrupts | 994 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Stalls: md1/lab-recv-busy-vs-recv-busy-burst50, loop 1 on CPU 3

133844288 iterations, longest 656.8 µs; over 5 / 20 / 100 µs: 455686 / 14610 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 13004 | 475865.0 | 656.8 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 862 | 24605.3 | 48.1 |
| softirq NET_RX | 465 | 14524.5 | 63.1 |
| mostly unexplained (+preempted by kworker/3:1) | 200 | 5320.6 | 59.6 |
| mostly unexplained (+softirq NET_RX) | 76 | 2401.2 | 46.9 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 45.8 | 24.4 |
| mostly unexplained (+local timer tick) | 1 | 21.0 | 21.0 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 656.8 | 2742 | loop work (2742 items) |  |
| 2 | 86.3 | 20 | loop work (20 items) | softirq NET_RX 31, local timer tick 5, preempted by kworker/3:1 4, timer tick_nohz_handler 3 |
| 3 | 85.1 | 14 | loop work (14 items) | softirq NET_RX 27, preempted by kworker/3:1 8, local timer tick 4, timer tick_nohz_handler 2 |
| 4 | 81.0 | 18 | loop work (18 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 80.3 | 16 | loop work (16 items) | softirq NET_RX 26, local timer tick 6, preempted by kworker/3:1 5, timer tick_nohz_handler 3 |
| 6 | 79.6 | 16 | loop work (16 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 79.1 | 27 | loop work (27 items) | preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 78.9 | 14 | loop work (14 items) | softirq NET_RX 36, local timer tick 7, preempted by kworker/3:1 4, timer tick_nohz_handler 4 |
| 9 | 78.7 | 16 | loop work (16 items) | local timer tick 5, softirq NET_RX 2, timer tick_nohz_handler 2 |
| 10 | 78.4 | 22 | loop work (22 items) | softirq NET_RX 25, softirq NET_TX 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2284 |
| IWI IRQ work interrupts | 2251 |
| CAL Function call interrupts | 996 |
| TLB TLB shootdowns | 4 |
| RES Rescheduling interrupts | 1 |

## md1/lab-recv-busy-vs-recv-busy-single: lab (A/A control), 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=158332 clocksource=tsc

### Loop counters (md1/lab-recv-busy-vs-recv-busy-single)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 90054 | 90001 | 1.00 | 172290368 | 1913.19 | 1914337 | 1914337 | 99.9 | 64.8 | true | 0 | 144 | 342.3 | 45324 / 386 / 1 | 0 | 0 |
| recv-busy | 90054 | 90001 | 1.00 | 172600128 | 1916.63 | 1917779 | 1917779 | 99.9 | 64.9 | true | 0 | 154 | 250.8 | 39405 / 430 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 2 |
| recv-busy | 0 | 8194 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 20172096 | 0 | 0 | 0 | 0.000 |
| recv-busy | 20172096 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy #0 (88000 of 88000 in 1-msg reads) | 88000 | 4.07 | 3.85 | 5.59 | 7.19 | 9.26 | 34.03 | 65.90 |
| recv-busy #1 (88000 of 88000 in 1-msg reads) | 88000 | 3.92 | 3.65 | 5.55 | 7.22 | 8.60 | 25.77 | 37.45 |

### Paired by message, relative to recv-busy (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **recv-busy #1 Δuser (write-order corrected)** | 88000 | 0.43 | 0.42 | 2.30 | 5.82 | 56.99 | 79.13 | 103.84 |
| recv-busy #1 Δuser, even seq | 44000 | -1.48 | -1.30 | 0.08 | 2.59 | 54.09 | 65.94 | 101.15 |
| recv-busy #1 Δuser, odd seq | 44000 | 2.34 | 2.15 | 4.66 | 8.19 | 59.77 | 83.07 | 105.57 |
| recv-busy #1 Δrx | 88000 | 0.58 | 0.65 | 4.66 | 8.23 | 56.18 | 67.26 | 105.99 |
| recv-busy #1 Δstack (1-msg reads) | 88000 | -0.15 | -0.15 | 1.29 | 2.07 | 2.82 | 21.53 | 30.50 |

recv-busy #1: 88000 paired messages; decoded before recv-busy on 45.4 % (uncorrected).

#### Stalls: md1/lab-recv-busy-vs-recv-busy-single, loop 0 on CPU 2

172290368 iterations, longest 342.3 µs; over 5 / 20 / 100 µs: 45324 / 386 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 220 | 12071.0 | 69.2 |
| mostly unexplained (+softirq NET_RX) | 57 | 2583.5 | 69.1 |
| mostly unexplained (+preempted by kworker/2:1) | 86 | 2472.6 | 40.2 |
| softirq NET_RX | 19 | 476.4 | 29.4 |
| softirq TIMER | 1 | 342.3 | 342.3 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 64.3 | 37.7 |
| loop work | 1 | 25.1 | 25.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 342.3 | 1 | softirq TIMER | softirq TIMER 321, local timer tick 8, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 69.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 69.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 2 |
| 4 | 68.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 67.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 67.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 67.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 67.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 67.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 67.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 995 |
| IWI IRQ work interrupts | 155 |
| LOC Local timer interrupts | 146 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

#### Stalls: md1/lab-recv-busy-vs-recv-busy-single, loop 1 on CPU 3

172600128 iterations, longest 250.8 µs; over 5 / 20 / 100 µs: 39405 / 430 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 253 | 12451.5 | 69.4 |
| mostly unexplained (+softirq NET_RX) | 78 | 3051.1 | 82.9 |
| mostly unexplained (+preempted by kworker/3:1) | 75 | 2168.1 | 91.1 |
| softirq NET_RX | 22 | 566.6 | 33.4 |
| softirq TIMER | 1 | 250.8 | 250.8 |
| loop work | 1 | 31.3 | 31.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 250.8 | 0 | softirq TIMER | softirq TIMER 234, local timer tick 9, timer tick_nohz_handler 5, irq work (IPI/vector) 1 |
| 2 | 91.1 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 90.6 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 87.6 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 82.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 69.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 7 | 69.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 69.1 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 9 | 68.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 67.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| IWI IRQ work interrupts | 160 |
| LOC Local timer interrupts | 157 |
| TLB TLB shootdowns | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## md1/lab-recv-busy-vs-uring-spin-burst50: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2130237 clocksource=tsc

### Loop counters (md1/lab-recv-busy-vs-uring-spin-burst50)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4502650 | 2419976 | 1.86 | 158771008 | 35.26 | 1764122 | 1764122 | 98.5 | 66.7 | true | 0 | 1811 | 841.6 | 130042 / 9995 / 1 | 502650 | 0 |
| uring-spin | 4502650 | 1297309 | 3.47 | 2051608 | 0.46 | 22796 | 50096635 | 0.0 | 4.6 | false | 0 | 0 | 377.3 | 271363 / 107 / 2 | 502650 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 2 |
| uring-spin | 0 | 8193 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1008593600 | 0 | 0 | 0 | 0.000 |
| uring-spin | 1008593600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (861713 of 3897350 in 1-msg reads) | 861713 | 2.42 | 2.29 | 2.82 | 4.28 | 8.58 | 20.95 | 34.85 |
| uring-spin (319308 of 3897350 in 1-msg reads) | 319308 | 6.75 | 6.46 | 9.96 | 14.23 | 17.21 | 30.48 | 42.60 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **uring-spin Δuser (write-order corrected)** | 3897350 | 29.45 | 29.14 | 50.82 | 65.91 | 82.26 | 127.50 | 1011.75 |
| uring-spin Δuser, even seq | 1948675 | 31.30 | 30.89 | 52.13 | 67.52 | 83.76 | 126.27 | 1009.99 |
| uring-spin Δuser, odd seq | 1948675 | 27.60 | 27.39 | 49.53 | 64.30 | 80.83 | 128.26 | 1010.00 |
| uring-spin Δrx | 3897350 | 22.48 | 21.07 | 44.50 | 60.25 | 76.99 | 121.52 | 1004.82 |
| uring-spin Δstack (1-msg reads) | 203941 | 3.91 | 3.59 | 7.43 | 11.59 | 15.00 | 28.54 | 37.27 |

uring-spin: 3897350 paired messages; decoded before recv-busy on 1.9 % (uncorrected).

#### Stalls: md1/lab-recv-busy-vs-uring-spin-burst50, loop 0 on CPU 2

158771008 iterations, longest 841.6 µs; over 5 / 20 / 100 µs: 130042 / 9995 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 9551 | 230275.5 | 841.6 |
| mostly unexplained (+preempted by kworker/2:1) | 188 | 4580.2 | 37.7 |
| softirq NET_RX | 127 | 3072.9 | 43.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 119 | 2695.2 | 63.9 |
| mostly unexplained (+softirq NET_RX) | 7 | 167.9 | 27.0 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 74.4 | 29.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 841.6 | 2650 | loop work (2650 items) |  |
| 2 | 63.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 60.9 | 4 | loop work (4 items) | softirq NET_RX 19, preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 56.5 | 5 | loop work (5 items) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 54.6 | 5 | loop work (5 items) | preempted by kworker/2:1 5, irq work (IPI/vector) 2, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 53.8 | 3 | loop work (3 items) | softirq NET_RX 19, preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 52.8 | 6 | loop work (6 items) | softirq NET_RX 22, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 51.7 | 5 | loop work (5 items) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 50.4 | 4 | loop work (4 items) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 50.2 | 6 | loop work (6 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 1826 |
| LOC Local timer interrupts | 1813 |
| CAL Function call interrupts | 999 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md1/lab-recv-busy-vs-uring-spin-burst50, loop 1 on CPU 3

4508697152 iterations, longest 377.3 µs; over 5 / 20 / 100 µs: 271363 / 107 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 103 | 2922.9 | 299.2 |
| loop work | 4 | 467.7 | 377.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 377.3 | 2650 | loop work (2650 items) |  |
| 2 | 299.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 39.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 37.7 | 3 | loop work (3 items) |  |
| 5 | 36.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 35.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 34.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 33.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 32.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 32.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## md1/lab-recv-busy-vs-uring-spin-single: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=127862 clocksource=tsc

### Loop counters (md1/lab-recv-busy-vs-uring-spin-single)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 90053 | 90001 | 1.00 | 170391424 | 1892.12 | 1893238 | 1893238 | 99.9 | 64.9 | true | 0 | 501 | 65.1 | 44664 / 1673 / 0 | 0 | 0 |
| uring-spin | 90053 | 90001 | 1.00 | 180002 | 2.00 | 2000 | 52352890 | 0.0 | 0.3 | false | 0 | 0 | 202.8 | 1799 / 52 / 1 | 0 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8193 | 2 |
| uring-spin | 0 | 8193 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 20171872 | 0 | 0 | 0 | 0.000 |
| uring-spin | 20171872 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (88000 of 88000 in 1-msg reads) | 88000 | 3.77 | 3.00 | 6.27 | 10.39 | 36.34 | 42.67 | 45.49 |
| uring-spin (88000 of 88000 in 1-msg reads) | 88000 | 4.43 | 4.02 | 5.40 | 13.70 | 21.52 | 35.76 | 40.51 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **uring-spin Δuser (write-order corrected)** | 88000 | 0.52 | 0.92 | 2.45 | 15.39 | 32.92 | 56.82 | 71.43 |
| uring-spin Δuser, even seq | 44000 | 4.23 | 4.52 | 7.00 | 22.23 | 40.54 | 63.25 | 75.03 |
| uring-spin Δuser, odd seq | 44000 | -3.18 | -2.68 | -1.37 | -0.70 | 4.32 | 14.08 | 46.23 |
| uring-spin Δrx | 88000 | -0.14 | -0.03 | 5.16 | 7.96 | 29.36 | 51.04 | 71.43 |
| uring-spin Δstack (1-msg reads) | 88000 | 0.66 | 1.15 | 2.07 | 10.98 | 17.53 | 32.02 | 37.37 |

uring-spin: 88000 paired messages; decoded before recv-busy on 57.5 % (uncorrected).

#### Stalls: md1/lab-recv-busy-vs-uring-spin-single, loop 0 on CPU 2

170391424 iterations, longest 65.1 µs; over 5 / 20 / 100 µs: 44664 / 1673 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 1034 | 28975.4 | 54.1 |
| mostly unexplained (+softirq NET_RX) | 388 | 12106.3 | 53.6 |
| mostly unexplained (+preempted by kworker/2:1) | 199 | 5745.7 | 52.1 |
| mostly unexplained (+local timer tick) | 20 | 767.1 | 61.3 |
| softirq NET_RX | 28 | 749.9 | 36.6 |
| softirq TIMER | 1 | 65.1 | 65.1 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 57.5 | 31.0 |
| loop work | 1 | 35.4 | 35.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 65.1 | 0 | softirq TIMER | softirq TIMER 56, local timer tick 4, irq work (IPI/vector) 1, timer tick_nohz_handler 1 |
| 2 | 61.3 | 0 | mostly unexplained (+local timer tick) | local timer tick 13, timer tick_nohz_handler 9, preempted by kworker/2:1 8, irq work (IPI/vector) 1 |
| 3 | 54.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 53.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 16, preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 52.1 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 50.0 | 1 | mostly unexplained (+local timer tick) | local timer tick 5, preempted by kworker/2:1 3, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 7 | 48.4 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 48.0 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 46.4 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 6, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 10 | 46.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 16, preempted by kworker/2:1 12, local timer tick 5, timer tick_nohz_handler 2 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 998 |
| LOC Local timer interrupts | 555 |
| IWI IRQ work interrupts | 522 |
| TLB TLB shootdowns | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md1/lab-recv-busy-vs-uring-spin-single, loop 1 on CPU 3

4711760064 iterations, longest 202.8 µs; over 5 / 20 / 100 µs: 1799 / 52 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 51 | 1103.8 | 37.5 |
| softirq TIMER | 1 | 202.8 | 202.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 202.8 | 0 | softirq TIMER | softirq TIMER 181, local timer tick 13, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 2 | 37.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 31.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 27.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 24.5 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 22.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 22.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 22.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 22.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 22.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## md1/lab-recv-busy-vs-uring-sqpoll-burst50: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2770168 clocksource=tsc

### Loop counters (md1/lab-recv-busy-vs-uring-sqpoll-burst50)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4502700 | 1685404 | 2.67 | 160599680 | 35.67 | 1784441 | 1784441 | 99.0 | 66.2 | true | 0 | 2116 | 658.4 | 219103 / 14530 / 1 | 502700 | 0 |
| uring-sqpoll | 4502700 | 1482111 | 3.04 | 1 | 0.00 | 0 | 34576904 | 0.0 | 0.0 | true | 0 | 0 | 385.2 | 1406 / 4 / 1 | 502700 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 4 |
| uring-sqpoll | 0 | 8196 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1008604800 | 0 | 0 | 0 | 0.000 |
| uring-sqpoll | 1008604800 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (519001 of 3897300 in 1-msg reads) | 519001 | 2.67 | 2.34 | 3.74 | 6.42 | 11.08 | 27.41 | 33.94 |
| uring-sqpoll (394812 of 3897300 in 1-msg reads) | 394812 | 3.13 | 2.90 | 3.88 | 7.73 | 12.15 | 18.01 | 25.49 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **uring-sqpoll Δuser (write-order corrected)** | 3897300 | -0.04 | 0.80 | 15.09 | 34.43 | 45.40 | 60.76 | 10494.18 |
| uring-sqpoll Δuser, even seq | 1948650 | -0.82 | 0.16 | 14.24 | 33.89 | 44.52 | 59.98 | 89.71 |
| uring-sqpoll Δuser, odd seq | 1948650 | 0.73 | 1.43 | 15.91 | 34.98 | 46.31 | 61.63 | 10494.82 |
| uring-sqpoll Δrx | 3897300 | -0.62 | 0.12 | 14.23 | 33.37 | 44.01 | 58.96 | 10485.98 |
| uring-sqpoll Δstack (1-msg reads) | 159714 | 0.11 | 0.18 | 0.81 | 3.07 | 8.55 | 14.76 | 22.55 |

uring-sqpoll: 3897300 paired messages; decoded before recv-busy on 45.0 % (uncorrected).

#### Stalls: md1/lab-recv-busy-vs-uring-sqpoll-burst50, loop 0 on CPU 2

160599680 iterations, longest 658.4 µs; over 5 / 20 / 100 µs: 219103 / 14530 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 13254 | 395679.7 | 658.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 560 | 14250.5 | 41.3 |
| mostly unexplained (+preempted by kworker/2:1) | 420 | 10546.8 | 42.9 |
| softirq NET_RX | 256 | 6581.9 | 37.1 |
| mostly unexplained (+softirq NET_RX) | 32 | 863.7 | 35.5 |
| mostly unexplained (+irq work (IPI/vector)) | 8 | 245.9 | 41.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 658.4 | 2700 | loop work (2700 items) |  |
| 2 | 79.5 | 12 | loop work (12 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 78.9 | 7 | loop work (7 items) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 78.9 | 9 | loop work (9 items) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 68.2 | 6 | loop work (6 items) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 68.1 | 9 | loop work (9 items) | softirq NET_RX 22, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 66.4 | 7 | loop work (7 items) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 65.6 | 11 | loop work (11 items) | preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 64.9 | 10 | loop work (10 items) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 63.7 | 4 | loop work (4 items) | softirq NET_RX 23, preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2141 |
| LOC Local timer interrupts | 2118 |
| CAL Function call interrupts | 1000 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md1/lab-recv-busy-vs-uring-sqpoll-burst50, loop 1 on CPU 3

3111921344 iterations, longest 385.2 µs; over 5 / 20 / 100 µs: 1406 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 2 | 406.1 | 385.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 65.5 | 34.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 385.2 | 2700 | loop work (2700 items) |  |
| 2 | 34.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 30.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 21.0 | 4 | loop work (4 items) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 6 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## md1/lab-recv-busy-vs-uring-sqpoll-single: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=155336 clocksource=tsc

### Loop counters (md1/lab-recv-busy-vs-uring-sqpoll-single)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 90053 | 89993 | 1.00 | 172516160 | 1915.72 | 1916846 | 1916846 | 99.9 | 64.6 | true | 0 | 153 | 71.4 | 7702 / 283 / 0 | 0 | 0 |
| uring-sqpoll | 90053 | 89993 | 1.00 | 1 | 0.00 | 0 | 34478643 | 0.0 | 0.0 | true | 0 | 0 | 33.1 | 279 / 3 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8194 | 2 |
| uring-sqpoll | 0 | 8200 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 20171872 | 0 | 0 | 0 | 0.000 |
| uring-sqpoll | 20171872 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (87984 of 88000 in 1-msg reads) | 87984 | 3.00 | 2.87 | 3.61 | 4.77 | 6.13 | 20.14 | 23.83 |
| uring-sqpoll (87984 of 88000 in 1-msg reads) | 87984 | 3.34 | 3.26 | 3.93 | 4.97 | 6.05 | 14.98 | 22.41 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **uring-sqpoll Δuser (write-order corrected)** | 88000 | 0.50 | 0.40 | 2.86 | 5.21 | 58.45 | 65.83 | 83.96 |
| uring-sqpoll Δuser, even seq | 44000 | 3.82 | 3.66 | 6.17 | 8.54 | 61.72 | 67.65 | 87.22 |
| uring-sqpoll Δuser, odd seq | 44000 | -2.82 | -2.86 | -0.45 | 1.84 | 54.90 | 62.58 | 76.49 |
| uring-sqpoll Δrx | 88000 | 0.16 | 0.21 | 5.04 | 7.58 | 58.35 | 63.38 | 87.14 |
| uring-sqpoll Δstack (1-msg reads) | 87978 | 0.34 | 0.33 | 0.89 | 1.51 | 2.12 | 11.82 | 18.24 |

uring-sqpoll: 88000 paired messages; decoded before recv-busy on 46.9 % (uncorrected).

#### Stalls: md1/lab-recv-busy-vs-uring-sqpoll-single, loop 0 on CPU 2

172516160 iterations, longest 71.4 µs; over 5 / 20 / 100 µs: 7702 / 283 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 225 | 13033.4 | 66.9 |
| mostly unexplained (+softirq NET_RX) | 21 | 1263.5 | 71.4 |
| mostly unexplained (+preempted by kworker/2:1) | 26 | 554.9 | 24.5 |
| softirq NET_RX | 9 | 206.1 | 28.7 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 29.7 | 29.7 |
| loop work | 1 | 24.2 | 24.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 71.4 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 2 | 66.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 66.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 65.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 65.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 65.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 65.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 65.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 65.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 64.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1000 |
| IWI IRQ work interrupts | 162 |
| LOC Local timer interrupts | 156 |
| TLB TLB shootdowns | 5 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md1/lab-recv-busy-vs-uring-sqpoll-single, loop 1 on CPU 3

3103077888 iterations, longest 33.1 µs; over 5 / 20 / 100 µs: 279 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 90.3 | 33.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 33.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 29.2 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 28.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| TLB TLB shootdowns | 4 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |

## md1/lab-uring-sqpoll-vs-recv-busy-burst50: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2344641 clocksource=tsc

### Loop counters (md1/lab-uring-sqpoll-vs-recv-busy-burst50)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4502700 | 1454177 | 3.10 | 1 | 0.00 | 0 | 42887570 | 0.0 | 0.0 | true | 0 | 0 | 383.0 | 386 / 4 / 3 | 502700 | 0 |
| recv-busy | 4502700 | 2256826 | 2.00 | 95903680 | 21.30 | 1065596 | 1065596 | 97.6 | 80.0 | true | 0 | 2748 | 856.2 | 468967 / 14860 / 2 | 502700 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8196 | 3 |
| recv-busy | 0 | 8196 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1008604576 | 0 | 0 | 0 | 0.000 |
| recv-busy | 1008604800 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (308773 of 3897300 in 1-msg reads) | 308773 | 4.34 | 3.92 | 6.24 | 9.32 | 13.75 | 25.06 | 34.12 |
| recv-busy (729252 of 3897300 in 1-msg reads) | 729252 | 3.41 | 3.14 | 4.82 | 7.49 | 11.39 | 22.39 | 36.75 |

### Paired by message, relative to uring-sqpoll

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **recv-busy Δuser (write-order corrected)** | 3897300 | -5.59 | -4.94 | 3.94 | 9.96 | 27.50 | 47.38 | 101.46 |
| recv-busy Δuser, even seq | 1948650 | -6.40 | -5.61 | 3.32 | 8.96 | 26.32 | 46.79 | 93.75 |
| recv-busy Δuser, odd seq | 1948650 | -4.78 | -4.27 | 4.55 | 11.00 | 28.59 | 47.89 | 102.13 |
| recv-busy Δrx | 3897300 | -4.54 | -3.98 | 4.94 | 10.86 | 25.82 | 44.50 | 101.04 |
| recv-busy Δstack (1-msg reads) | 192518 | -0.88 | -0.80 | 0.70 | 4.12 | 7.87 | 21.26 | 32.58 |

recv-busy: 3897300 paired messages; decoded before uring-sqpoll on 73.3 % (uncorrected).

#### Stalls: md1/lab-uring-sqpoll-vs-recv-busy-burst50, loop 0 on CPU 2

3859881280 iterations, longest 383.0 µs; over 5 / 20 / 100 µs: 386 / 4 / 3

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 383.0 | 383.0 |
| softirq TIMER | 1 | 153.3 | 153.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 145.5 | 116.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 383.0 | 2699 | loop work (2699 items) |  |
| 2 | 153.3 | 0 | softirq TIMER | softirq TIMER 133, local timer tick 13, timer tick_nohz_handler 7, irq work (IPI/vector) 1 |
| 3 | 116.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 29.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

#### Stalls: md1/lab-uring-sqpoll-vs-recv-busy-burst50, loop 1 on CPU 3

95903680 iterations, longest 856.2 µs; over 5 / 20 / 100 µs: 468967 / 14860 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 13071 | 387892.8 | 856.2 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1084 | 26194.8 | 105.4 |
| softirq NET_RX | 551 | 14269.0 | 42.5 |
| mostly unexplained (+softirq NET_RX) | 93 | 2252.0 | 34.5 |
| mostly unexplained (+preempted by kworker/3:1) | 60 | 1402.3 | 34.6 |
| softirq TIMER | 1 | 64.2 | 64.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 856.2 | 2700 | loop work (2700 items) |  |
| 2 | 105.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 99.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 68.8 | 12 | loop work (12 items) | softirq NET_RX 25, preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 64.7 | 13 | loop work (13 items) | softirq NET_RX 23 |
| 6 | 64.2 | 0 | softirq TIMER | softirq TIMER 53, local timer tick 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 7 | 62.3 | 14 | loop work (14 items) | softirq NET_RX 9 |
| 8 | 62.3 | 11 | loop work (11 items) | softirq NET_RX 14 |
| 9 | 62.3 | 12 | loop work (12 items) | softirq NET_RX 18, preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 62.2 | 8 | loop work (8 items) | softirq NET_RX 26, preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2754 |
| LOC Local timer interrupts | 2753 |
| CAL Function call interrupts | 1000 |
| TLB TLB shootdowns | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## md1/lab-uring-sqpoll-vs-recv-busy-single: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=154536 clocksource=tsc

### Loop counters (md1/lab-uring-sqpoll-vs-recv-busy-single)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 90054 | 90001 | 1.00 | 1 | 0.00 | 0 | 37342262 | 0.0 | 0.0 | true | 0 | 0 | 55.7 | 382 / 2 / 0 | 0 | 0 |
| recv-busy | 90055 | 90002 | 1.00 | 172287616 | 1913.14 | 1914307 | 1914307 | 99.9 | 65.0 | true | 0 | 248 | 59.9 | 36826 / 856 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8194 | 1 |
| recv-busy | 0 | 8198 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 20171872 | 0 | 0 | 0 | 0.000 |
| recv-busy | 20172320 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (88000 of 88000 in 1-msg reads) | 88000 | 4.63 | 4.41 | 5.96 | 7.64 | 15.71 | 21.60 | 23.64 |
| recv-busy (88001 of 88001 in 1-msg reads) | 88001 | 3.88 | 3.60 | 5.49 | 7.43 | 23.43 | 32.45 | 34.38 |

### Paired by message, relative to uring-sqpoll

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **recv-busy Δuser (write-order corrected)** | 88000 | -0.69 | -0.79 | 1.43 | 5.18 | 26.52 | 44.46 | 77.87 |
| recv-busy Δuser, even seq | 44000 | -2.81 | -2.83 | -0.41 | 1.07 | 24.00 | 46.70 | 75.82 |
| recv-busy Δuser, odd seq | 44000 | 1.43 | 1.26 | 3.13 | 16.09 | 28.77 | 45.85 | 59.34 |
| recv-busy Δrx | 88000 | 0.06 | 0.05 | 3.99 | 7.38 | 27.00 | 35.58 | 76.89 |
| recv-busy Δstack (1-msg reads) | 88000 | -0.75 | -0.86 | 0.55 | 1.62 | 18.54 | 25.55 | 29.21 |

recv-busy: 88000 paired messages; decoded before uring-sqpoll on 56.2 % (uncorrected).

#### Stalls: md1/lab-uring-sqpoll-vs-recv-busy-single, loop 0 on CPU 2

3360803584 iterations, longest 55.7 µs; over 5 / 20 / 100 µs: 382 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 90.1 | 55.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 55.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 34.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Stalls: md1/lab-uring-sqpoll-vs-recv-busy-single, loop 1 on CPU 3

172287616 iterations, longest 59.9 µs; over 5 / 20 / 100 µs: 36826 / 856 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 609 | 16030.5 | 40.2 |
| mostly unexplained (+preempted by kworker/3:1) | 142 | 3867.2 | 55.8 |
| mostly unexplained (+softirq NET_RX) | 68 | 1923.1 | 59.9 |
| softirq NET_RX | 32 | 793.0 | 34.0 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 129.9 | 46.9 |
| loop work | 1 | 43.5 | 43.5 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 59.9 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/3:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 2 | 55.8 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 53.6 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 50.8 | 0 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 8, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 49.3 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 49.3 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 46.9 | 0 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 2, ipi_send_cpu 0 |
| 8 | 44.9 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 43.5 | 54 | loop work (54 items) |  |
| 10 | 42.5 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 263 |
| LOC Local timer interrupts | 250 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

## md2/lab-recv-busy-vs-epoll-spin-burst50: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1991833 clocksource=tsc

### Loop counters (md2/lab-recv-busy-vs-epoll-spin-burst50)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4502694 | 2268808 | 1.98 | 157650880 | 35.01 | 1751676 | 1751676 | 98.6 | 66.7 | true | 0 | 2139 | 617.1 | 185399 / 17295 / 1 | 502694 | 0 |
| epoll-spin | 4502653 | 1225784 | 3.67 | 361872824 | 80.37 | 4020809 | 4007189 | 0.0 | 35.3 | false | 0 | 0 | 653.9 | 253079 / 591 / 1 | 502653 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 4 |
| epoll-spin | 0 | 8193 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1008603456 | 0 | 0 | 0 | 0.000 |
| epoll-spin | 1008594272 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (844196 of 3897300 in 1-msg reads) | 844196 | 2.52 | 2.29 | 3.14 | 6.35 | 11.05 | 25.96 | 39.92 |
| epoll-spin (293614 of 3897300 in 1-msg reads) | 293614 | 6.04 | 6.13 | 8.00 | 11.99 | 27.12 | 36.87 | 55.16 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **epoll-spin Δuser (write-order corrected)** | 3897300 | 27.76 | 25.86 | 51.28 | 75.72 | 91.95 | 120.84 | 201.18 |
| epoll-spin Δuser, even seq | 1948650 | 29.36 | 27.61 | 52.48 | 76.38 | 93.32 | 120.21 | 202.93 |
| epoll-spin Δuser, odd seq | 1948650 | 26.15 | 24.11 | 50.01 | 74.86 | 90.70 | 121.54 | 163.89 |
| epoll-spin Δrx | 3897300 | 22.66 | 19.93 | 46.91 | 72.12 | 88.63 | 116.01 | 202.81 |
| epoll-spin Δstack (1-msg reads) | 191231 | 3.00 | 2.92 | 5.31 | 9.60 | 24.68 | 33.42 | 52.78 |

epoll-spin: 3897300 paired messages; decoded before recv-busy on 3.6 % (uncorrected).

#### Stalls: md2/lab-recv-busy-vs-epoll-spin-burst50, loop 0 on CPU 2

157650880 iterations, longest 617.1 µs; over 5 / 20 / 100 µs: 185399 / 17295 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 16182 | 441525.0 | 617.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 520 | 12047.4 | 38.1 |
| mostly unexplained (+preempted by kworker/2:1) | 369 | 9479.5 | 52.6 |
| softirq NET_RX | 191 | 4914.3 | 38.9 |
| mostly unexplained (+softirq NET_RX) | 30 | 760.4 | 33.1 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 62.1 | 40.5 |
| softirq TIMER | 1 | 22.6 | 22.6 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 617.1 | 2700 | loop work (2700 items) |  |
| 2 | 68.6 | 10 | loop work (10 items) | softirq NET_RX 22, preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 67.7 | 10 | loop work (10 items) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 66.5 | 9 | loop work (9 items) | softirq NET_RX 19, preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 64.8 | 10 | loop work (10 items) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 61.7 | 6 | loop work (6 items) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 61.4 | 9 | loop work (9 items) | softirq NET_RX 23, preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 8 | 61.0 | 6 | loop work (6 items) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 60.7 | 9 | loop work (9 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 60.6 | 5 | loop work (5 items) | preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2149 |
| LOC Local timer interrupts | 2142 |
| CAL Function call interrupts | 995 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md2/lab-recv-busy-vs-epoll-spin-burst50, loop 1 on CPU 3

360647040 iterations, longest 653.9 µs; over 5 / 20 / 100 µs: 253079 / 591 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 304 | 8434.6 | 653.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 287 | 7431.3 | 70.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 653.9 | 2671 | loop work (2671 items) |  |
| 2 | 70.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 51.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 48.2 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 45.3 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 45.1 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 42.5 | 2 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 39.8 | 3 | loop work (3 items) |  |
| 9 | 38.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 38.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 2 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## md2/lab-recv-busy-vs-epoll-spin-single: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=79301 clocksource=tsc

### Loop counters (md2/lab-recv-busy-vs-epoll-spin-single)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 90054 | 90001 | 1.00 | 172192384 | 1912.10 | 1913249 | 1913249 | 99.9 | 64.5 | true | 0 | 170 | 50.5 | 7291 / 592 / 0 | 0 | 0 |
| epoll-spin | 90053 | 90001 | 1.00 | 375557073 | 4170.40 | 4172856 | 4171856 | 0.0 | 33.7 | false | 0 | 0 | 38.3 | 66591 / 908 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 2 |
| epoll-spin | 0 | 8199 | 0 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 20172096 | 0 | 0 | 0 | 0.000 |
| epoll-spin | 20171872 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (88000 of 88000 in 1-msg reads) | 88000 | 2.72 | 2.65 | 3.25 | 3.94 | 5.12 | 7.73 | 24.61 |
| epoll-spin (88000 of 88000 in 1-msg reads) | 88000 | 10.90 | 12.60 | 13.54 | 28.61 | 31.86 | 34.43 | 40.42 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **epoll-spin Δuser (write-order corrected)** | 87999 | 96.47 | 70.68 | 174.68 | 220.74 | 259.56 | 298.57 | 379.80 |
| epoll-spin Δuser, even seq | 43999 | 126.99 | 123.94 | 166.31 | 211.01 | 249.28 | 263.82 | 372.72 |
| epoll-spin Δuser, odd seq | 44000 | 65.94 | 17.42 | 139.34 | 172.85 | 222.73 | 250.73 | 326.54 |
| epoll-spin Δrx | 87999 | 88.28 | 108.85 | 148.40 | 173.73 | 235.36 | 254.12 | 362.87 |
| epoll-spin Δstack (1-msg reads) | 87999 | 8.18 | 9.94 | 10.63 | 25.89 | 28.87 | 31.39 | 38.22 |

epoll-spin: 87999 paired messages; decoded before recv-busy on 0.4 % (uncorrected).

#### Stalls: md2/lab-recv-busy-vs-epoll-spin-single, loop 0 on CPU 2

172192384 iterations, longest 50.5 µs; over 5 / 20 / 100 µs: 7291 / 592 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 496 | 13758.3 | 50.5 |
| mostly unexplained (+preempted by kworker/2:1) | 47 | 1178.1 | 46.1 |
| mostly unexplained (+softirq NET_RX) | 22 | 582.6 | 35.7 |
| softirq NET_RX | 24 | 579.3 | 29.0 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 52.9 | 32.2 |
| loop work | 1 | 28.8 | 28.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 50.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 46.1 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 44.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 43.7 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 43.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 43.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 43.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 43.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 42.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 42.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 996 |
| IWI IRQ work interrupts | 185 |
| LOC Local timer interrupts | 181 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md2/lab-recv-busy-vs-epoll-spin-single, loop 1 on CPU 3

375467072 iterations, longest 38.3 µs; over 5 / 20 / 100 µs: 66591 / 908 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 906 | 21225.4 | 38.3 |
| loop work | 2 | 48.8 | 28.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 38.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 34.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 32.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 30.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 30.1 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 28.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 28.8 | 53 | loop work (53 items) |  |
| 8 | 28.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 27.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 27.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 4 |
| TLB TLB shootdowns | 1 |

## md2/lab-recv-busy-vs-recv-busy-burst50: lab (A/A control), 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3276757 clocksource=tsc

### Loop counters (md2/lab-recv-busy-vs-recv-busy-burst50)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4502650 | 1810952 | 2.49 | 160409152 | 35.63 | 1782324 | 1782324 | 98.9 | 66.4 | true | 0 | 1981 | 760.3 | 180283 / 11378 / 1 | 502650 | 0 |
| recv-busy | 4502650 | 1957625 | 2.30 | 160523392 | 35.65 | 1783593 | 1783593 | 98.8 | 66.5 | true | 0 | 1980 | 731.7 | 152510 / 10138 / 1 | 502650 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 2 |
| recv-busy | 0 | 8198 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1008593600 | 0 | 0 | 0 | 0.000 |
| recv-busy | 1008593600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy #0 (460191 of 3897342 in 1-msg reads) | 460191 | 2.65 | 2.42 | 3.17 | 6.91 | 16.64 | 26.84 | 46.83 |
| recv-busy #1 (661146 of 3897342 in 1-msg reads) | 661146 | 2.57 | 2.35 | 3.12 | 6.27 | 15.22 | 25.04 | 53.06 |

### Paired by message, relative to recv-busy (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **recv-busy #1 Δuser (write-order corrected)** | 3897342 | 0.37 | -0.29 | 10.02 | 29.32 | 41.74 | 57.77 | 119.20 |
| recv-busy #1 Δuser, even seq | 1948671 | 1.34 | 0.64 | 10.99 | 29.64 | 42.49 | 58.42 | 118.51 |
| recv-busy #1 Δuser, odd seq | 1948671 | -0.61 | -1.23 | 9.04 | 28.97 | 40.92 | 57.29 | 118.27 |
| recv-busy #1 Δrx | 3897342 | 0.44 | -0.27 | 10.13 | 28.83 | 40.65 | 55.55 | 118.30 |
| recv-busy #1 Δstack (1-msg reads) | 240621 | 0.00 | 0.01 | 0.58 | 3.24 | 12.67 | 22.57 | 50.63 |

recv-busy #1: 3897342 paired messages; decoded before recv-busy on 52.0 % (uncorrected).

#### Stalls: md2/lab-recv-busy-vs-recv-busy-burst50, loop 0 on CPU 2

160409152 iterations, longest 760.3 µs; over 5 / 20 / 100 µs: 180283 / 11378 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 10881 | 278720.2 | 760.3 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 201 | 4748.2 | 43.1 |
| softirq NET_RX | 111 | 2650.7 | 37.7 |
| mostly unexplained (+preempted by kworker/2:1) | 114 | 2646.0 | 44.4 |
| mostly unexplained (+local timer tick) | 55 | 1335.1 | 36.1 |
| mostly unexplained (+softirq NET_RX) | 14 | 320.3 | 29.2 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 48.4 | 28.3 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 760.3 | 2650 | loop work (2650 items) |  |
| 2 | 58.2 | 7 | loop work (7 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 54.4 | 9 | loop work (9 items) | softirq NET_RX 15, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 53.3 | 6 | loop work (6 items) | preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 51.9 | 6 | loop work (6 items) | softirq NET_RX 18, local timer tick 5, preempted by kworker/2:1 2, timer tick_nohz_handler 2 |
| 6 | 50.5 | 10 | loop work (10 items) | local timer tick 4, softirq NET_RX 4, timer tick_nohz_handler 2 |
| 7 | 49.7 | 8 | loop work (8 items) | local timer tick 4, preempted by kworker/2:1 2, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 8 | 48.9 | 6 | loop work (6 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 48.6 | 6 | loop work (6 items) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 48.5 | 4 | loop work (4 items) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2167 |
| IWI IRQ work interrupts | 1989 |
| CAL Function call interrupts | 995 |
| TLB TLB shootdowns | 4 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md2/lab-recv-busy-vs-recv-busy-burst50, loop 1 on CPU 3

160523392 iterations, longest 731.7 µs; over 5 / 20 / 100 µs: 152510 / 10138 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 9606 | 245917.8 | 731.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 251 | 6074.6 | 42.9 |
| mostly unexplained (+preempted by kworker/3:1) | 134 | 3107.9 | 39.7 |
| softirq NET_RX | 91 | 2172.2 | 37.3 |
| mostly unexplained (+local timer tick) | 39 | 939.2 | 37.5 |
| mostly unexplained (+softirq NET_RX) | 16 | 375.3 | 31.1 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 24.9 | 24.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 731.7 | 2650 | loop work (2650 items) |  |
| 2 | 64.1 | 5 | loop work (5 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 56.0 | 10 | loop work (10 items) | softirq NET_RX 11, preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 54.7 | 7 | loop work (7 items) | softirq NET_RX 24 |
| 5 | 54.5 | 8 | loop work (8 items) | softirq NET_RX 10 |
| 6 | 53.9 | 6 | loop work (6 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 53.4 | 7 | loop work (7 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 53.1 | 9 | loop work (9 items) | local timer tick 3, timer tick_nohz_handler 2 |
| 9 | 50.9 | 6 | loop work (6 items) | preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 50.1 | 10 | loop work (10 items) | softirq NET_RX 11 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2179 |
| IWI IRQ work interrupts | 1998 |
| CAL Function call interrupts | 995 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## md2/lab-recv-busy-vs-recv-busy-single: lab (A/A control), 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=157527 clocksource=tsc

### Loop counters (md2/lab-recv-busy-vs-recv-busy-single)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 90053 | 90001 | 1.00 | 173514816 | 1926.81 | 1927942 | 1927942 | 99.9 | 64.6 | true | 0 | 166 | 64.4 | 7536 / 290 / 0 | 0 | 0 |
| recv-busy | 90053 | 90001 | 1.00 | 173547584 | 1927.17 | 1928306 | 1928306 | 99.9 | 64.7 | true | 0 | 149 | 70.3 | 6889 / 288 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8195 | 2 |
| recv-busy | 0 | 8194 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 20171872 | 0 | 0 | 0 | 0.000 |
| recv-busy | 20171872 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy #0 (88000 of 88000 in 1-msg reads) | 88000 | 3.04 | 2.93 | 3.67 | 4.76 | 5.93 | 7.50 | 22.33 |
| recv-busy #1 (88000 of 88000 in 1-msg reads) | 88000 | 2.78 | 2.65 | 3.45 | 4.78 | 5.98 | 8.17 | 27.50 |

### Paired by message, relative to recv-busy (A/A: any difference is noise, core or connection bias)

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **recv-busy #1 Δuser (write-order corrected)** | 88000 | -0.36 | -0.44 | 1.83 | 4.35 | 52.80 | 58.65 | 73.39 |
| recv-busy #1 Δuser, even seq | 44000 | 2.77 | 2.65 | 4.85 | 7.67 | 56.46 | 62.22 | 76.48 |
| recv-busy #1 Δuser, odd seq | 44000 | -3.49 | -3.53 | -1.21 | 0.99 | 48.98 | 54.89 | 56.01 |
| recv-busy #1 Δrx | 88000 | -0.10 | -0.16 | 4.46 | 7.10 | 52.01 | 61.03 | 77.14 |
| recv-busy #1 Δstack (1-msg reads) | 88000 | -0.26 | -0.28 | 0.22 | 0.93 | 1.66 | 2.73 | 22.58 |

recv-busy #1: 88000 paired messages; decoded before recv-busy on 50.3 % (uncorrected).

#### Stalls: md2/lab-recv-busy-vs-recv-busy-single, loop 0 on CPU 2

173514816 iterations, longest 64.4 µs; over 5 / 20 / 100 µs: 7536 / 290 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 243 | 12137.0 | 64.4 |
| mostly unexplained (+preempted by kworker/2:1) | 25 | 564.2 | 33.1 |
| mostly unexplained (+softirq NET_RX) | 7 | 322.0 | 58.2 |
| softirq NET_RX | 13 | 287.5 | 26.3 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 29.6 | 29.6 |
| loop work | 1 | 28.1 | 28.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 64.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 60.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 60.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 60.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 59.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 59.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 59.4 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 59.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 58.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 58.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 999 |
| IWI IRQ work interrupts | 177 |
| LOC Local timer interrupts | 169 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md2/lab-recv-busy-vs-recv-busy-single, loop 1 on CPU 3

173547584 iterations, longest 70.3 µs; over 5 / 20 / 100 µs: 6889 / 288 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 248 | 12190.4 | 70.3 |
| mostly unexplained (+softirq NET_RX) | 10 | 454.0 | 63.0 |
| mostly unexplained (+preempted by kworker/3:1) | 14 | 307.9 | 28.5 |
| softirq NET_RX | 12 | 274.7 | 29.2 |
| mostly unexplained (+irq work (IPI/vector)) | 3 | 126.7 | 68.2 |
| loop work | 1 | 30.8 | 30.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 70.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 68.2 | 1 | mostly unexplained (+irq work (IPI/vector)) | irq work (IPI/vector) 1, preempted by kworker/3:1 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 63.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 63.0 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 5 | 62.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 62.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 62.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 61.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 61.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 61.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 161 |
| LOC Local timer interrupts | 149 |
| TLB TLB shootdowns | 1 |

## md2/lab-recv-busy-vs-uring-spin-burst50: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=1717401 clocksource=tsc

### Loop counters (md2/lab-recv-busy-vs-uring-spin-burst50)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4502700 | 1167108 | 3.86 | 156357184 | 34.73 | 1737302 | 1737302 | 99.3 | 67.1 | true | 0 | 2258 | 568.4 | 544128 / 22453 / 1 | 502700 | 0 |
| uring-spin | 4502656 | 1266879 | 3.55 | 2454721 | 0.55 | 27275 | 50276674 | 0.0 | 3.6 | false | 0 | 0 | 381.7 | 12982 / 5 / 2 | 502656 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8197 | 2 |
| uring-spin | 0 | 8197 | 2 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1008604800 | 0 | 0 | 0 | 0.000 |
| uring-spin | 1008594944 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (261979 of 3897300 in 1-msg reads) | 261979 | 3.25 | 2.51 | 5.19 | 10.93 | 27.27 | 35.80 | 45.22 |
| uring-spin (273776 of 3897300 in 1-msg reads) | 273776 | 4.26 | 3.79 | 5.86 | 9.53 | 13.11 | 18.89 | 26.76 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **uring-spin Δuser (write-order corrected)** | 3897300 | -0.02 | -1.11 | 10.76 | 24.47 | 37.00 | 52.34 | 117.29 |
| uring-spin Δuser, even seq | 1948650 | 0.73 | -0.66 | 11.37 | 26.12 | 37.66 | 53.48 | 117.74 |
| uring-spin Δuser, odd seq | 1948650 | -0.77 | -1.56 | 10.13 | 22.75 | 36.30 | 51.15 | 112.22 |
| uring-spin Δrx | 3897300 | -0.52 | -2.41 | 10.10 | 23.65 | 35.70 | 50.44 | 109.39 |
| uring-spin Δstack (1-msg reads) | 126360 | 0.80 | 1.35 | 2.44 | 4.28 | 8.16 | 11.32 | 16.44 |

uring-spin: 3897300 paired messages; decoded before recv-busy on 57.2 % (uncorrected).

#### Stalls: md2/lab-recv-busy-vs-uring-spin-burst50, loop 0 on CPU 2

156357184 iterations, longest 568.4 µs; over 5 / 20 / 100 µs: 544128 / 22453 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 19187 | 665160.8 | 568.4 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1476 | 40856.4 | 41.8 |
| mostly unexplained (+preempted by kworker/2:1) | 638 | 18154.1 | 60.9 |
| softirq NET_RX | 584 | 17141.4 | 58.8 |
| mostly unexplained (+softirq NET_RX) | 312 | 9378.9 | 69.0 |
| mostly unexplained (+local timer tick) | 250 | 5806.4 | 46.4 |
| mostly unexplained (+irq work (IPI/vector)) | 5 | 128.2 | 32.9 |
| local timer tick | 1 | 20.2 | 20.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 568.4 | 2700 | loop work (2700 items) |  |
| 2 | 85.8 | 15 | loop work (15 items) | softirq NET_RX 24, preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 3 | 85.1 | 8 | loop work (8 items) | softirq NET_RX 23, preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 77.5 | 21 | loop work (21 items) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 75.5 | 10 | loop work (10 items) | softirq NET_RX 18, preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 6 | 75.2 | 17 | loop work (17 items) | softirq NET_RX 26, preempted by kworker/2:1 5, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 73.8 | 14 | loop work (14 items) | preempted by kworker/2:1 6, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 73.5 | 11 | loop work (11 items) | softirq NET_RX 25, preempted by kworker/2:1 8, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 9 | 72.6 | 12 | loop work (12 items) | softirq NET_RX 27, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 72.5 | 18 | loop work (18 items) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2380 |
| IWI IRQ work interrupts | 2273 |
| CAL Function call interrupts | 999 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md2/lab-recv-busy-vs-uring-spin-burst50, loop 1 on CPU 3

4524900672 iterations, longest 381.7 µs; over 5 / 20 / 100 µs: 12982 / 5 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 381.7 | 381.7 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 4 | 284.0 | 207.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 381.7 | 2674 | loop work (2674 items) |  |
| 2 | 207.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 30.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 26.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 20.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

## md2/lab-recv-busy-vs-uring-spin-single: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=79353 clocksource=tsc

### Loop counters (md2/lab-recv-busy-vs-uring-spin-single)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 90054 | 90001 | 1.00 | 171854016 | 1908.34 | 1909489 | 1909489 | 99.9 | 64.7 | true | 0 | 307 | 249.2 | 59013 / 902 / 1 | 0 | 0 |
| uring-spin | 90054 | 90001 | 1.00 | 180002 | 2.00 | 2000 | 52700612 | 0.0 | 0.3 | false | 0 | 0 | 192.1 | 16919 / 4 / 1 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8199 | 2 |
| uring-spin | 0 | 8198 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 20172096 | 0 | 0 | 0 | 0.000 |
| uring-spin | 20172096 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (88000 of 88000 in 1-msg reads) | 88000 | 4.44 | 4.32 | 5.94 | 8.08 | 25.39 | 29.35 | 131.75 |
| uring-spin (88000 of 88000 in 1-msg reads) | 88000 | 6.03 | 5.38 | 7.99 | 11.70 | 17.04 | 19.19 | 81.24 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **uring-spin Δuser (write-order corrected)** | 88000 | 7.22 | 7.22 | 9.82 | 21.45 | 37.60 | 47.94 | 70.72 |
| uring-spin Δuser, even seq | 44000 | 10.46 | 10.60 | 12.06 | 23.71 | 40.13 | 55.44 | 74.10 |
| uring-spin Δuser, odd seq | 44000 | 3.99 | 3.84 | 8.52 | 18.89 | 35.41 | 42.43 | 59.94 |
| uring-spin Δrx | 88000 | 5.63 | 5.88 | 9.93 | 18.88 | 38.10 | 45.89 | 74.70 |
| uring-spin Δstack (1-msg reads) | 88000 | 1.59 | 1.52 | 3.65 | 7.52 | 14.32 | 15.54 | 17.65 |

uring-spin: 88000 paired messages; decoded before recv-busy on 4.6 % (uncorrected).

#### Stalls: md2/lab-recv-busy-vs-uring-spin-single, loop 0 on CPU 2

171854016 iterations, longest 249.2 µs; over 5 / 20 / 100 µs: 59013 / 902 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 539 | 15041.1 | 43.2 |
| mostly unexplained (+preempted by kworker/2:1) | 237 | 6264.8 | 55.0 |
| mostly unexplained (+softirq NET_RX) | 95 | 2619.3 | 54.6 |
| softirq NET_RX | 25 | 556.2 | 27.9 |
| softirq TIMER | 1 | 249.2 | 249.2 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 115.5 | 35.6 |
| loop work | 1 | 34.9 | 34.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 249.2 | 0 | softirq TIMER | softirq TIMER 240, local timer tick 5, timer tick_nohz_handler 2, irq work (IPI/vector) 1 |
| 2 | 55.0 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 54.6 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 13, preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 4 | 53.3 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 51.5 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 50.2 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 7 | 46.9 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 7, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 8 | 44.2 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 43.7 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 10 | 43.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1000 |
| IWI IRQ work interrupts | 327 |
| LOC Local timer interrupts | 310 |
| TLB TLB shootdowns | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md2/lab-recv-busy-vs-uring-spin-single, loop 1 on CPU 3

4743055104 iterations, longest 192.1 µs; over 5 / 20 / 100 µs: 16919 / 4 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| softirq TIMER | 1 | 192.1 | 192.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 3 | 89.0 | 36.8 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 192.1 | 0 | softirq TIMER | softirq TIMER 167, local timer tick 13, timer tick_nohz_handler 7 |
| 2 | 36.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 29.8 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 4 | 22.3 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 1 |
| MCP Machine check polls | 1 |
| TLB TLB shootdowns | 1 |

## md2/lab-recv-busy-vs-uring-sqpoll-burst50: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=2815164 clocksource=tsc

### Loop counters (md2/lab-recv-busy-vs-uring-sqpoll-burst50)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 4502653 | 2213735 | 2.03 | 110515648 | 24.54 | 1227952 | 1227952 | 98.0 | 76.5 | true | 0 | 2799 | 622.9 | 638487 / 18599 / 1 | 502653 | 0 |
| uring-sqpoll | 4502650 | 2265053 | 1.99 | 1 | 0.00 | 0 | 45077053 | 0.0 | 0.0 | true | 0 | 0 | 380.1 | 482 / 3 / 1 | 502650 | 0 |

### NIC queues

The pair **shares** an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8196 | 1 |
| uring-sqpoll | 0 | 8196 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 1008594272 | 0 | 0 | 0 | 0.000 |
| uring-sqpoll | 1008593600 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (915566 of 3897303 in 1-msg reads) | 915566 | 3.61 | 3.23 | 5.44 | 8.37 | 12.54 | 24.03 | 41.27 |
| uring-sqpoll (770004 of 3897303 in 1-msg reads) | 770004 | 4.18 | 3.84 | 5.86 | 9.16 | 13.71 | 23.51 | 40.28 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **uring-sqpoll Δuser (write-order corrected)** | 3897303 | -0.84 | 0.14 | 8.62 | 17.35 | 33.17 | 50.46 | 126.16 |
| uring-sqpoll Δuser, even seq | 1948652 | 0.33 | 1.88 | 10.08 | 19.46 | 34.99 | 51.74 | 122.32 |
| uring-sqpoll Δuser, odd seq | 1948651 | -2.01 | -1.60 | 7.09 | 15.28 | 31.37 | 48.93 | 124.42 |
| uring-sqpoll Δrx | 3897303 | -1.35 | -0.03 | 7.80 | 16.70 | 29.81 | 46.65 | 118.20 |
| uring-sqpoll Δstack (1-msg reads) | 376902 | 0.58 | 0.57 | 2.55 | 5.45 | 9.51 | 20.93 | 32.26 |

uring-sqpoll: 3897303 paired messages; decoded before recv-busy on 48.3 % (uncorrected).

#### Stalls: md2/lab-recv-busy-vs-uring-sqpoll-burst50, loop 0 on CPU 2

110515648 iterations, longest 622.9 µs; over 5 / 20 / 100 µs: 638487 / 18599 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 15980 | 489342.4 | 622.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1729 | 41629.9 | 58.2 |
| softirq NET_RX | 615 | 16562.3 | 49.2 |
| mostly unexplained (+softirq NET_RX) | 194 | 5190.1 | 54.0 |
| mostly unexplained (+preempted by kworker/2:1) | 64 | 1502.0 | 39.6 |
| mostly unexplained (+local timer tick) | 16 | 436.9 | 41.5 |
| mostly unexplained (+irq work (IPI/vector)) | 1 | 20.4 | 20.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 622.9 | 2654 | loop work (2654 items) |  |
| 2 | 82.6 | 16 | loop work (16 items) | softirq NET_RX 11 |
| 3 | 78.3 | 12 | loop work (12 items) | softirq NET_RX 27 |
| 4 | 76.4 | 13 | loop work (13 items) | softirq NET_RX 30, preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 5 | 76.2 | 12 | loop work (12 items) | softirq NET_RX 16, local timer tick 3, timer tick_nohz_handler 1 |
| 6 | 74.4 | 15 | loop work (15 items) | softirq NET_RX 16 |
| 7 | 73.9 | 12 | loop work (12 items) | softirq NET_RX 12 |
| 8 | 73.1 | 11 | loop work (11 items) | softirq NET_RX 13 |
| 9 | 72.0 | 11 | loop work (11 items) | softirq NET_RX 16 |
| 10 | 71.7 | 12 | loop work (12 items) | softirq NET_RX 15 |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| LOC Local timer interrupts | 2898 |
| IWI IRQ work interrupts | 2807 |
| CAL Function call interrupts | 1001 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md2/lab-recv-busy-vs-uring-sqpoll-burst50, loop 1 on CPU 3

4056934784 iterations, longest 380.1 µs; over 5 / 20 / 100 µs: 482 / 3 / 1

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 380.1 | 380.1 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 51.7 | 27.1 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 380.1 | 2661 | loop work (2661 items) |  |
| 2 | 27.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 24.6 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## md2/lab-recv-busy-vs-uring-sqpoll-single: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=155048 clocksource=tsc

### Loop counters (md2/lab-recv-busy-vs-uring-sqpoll-single)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| recv-busy | 90053 | 89997 | 1.00 | 171805312 | 1907.82 | 1908948 | 1908948 | 99.9 | 64.8 | true | 0 | 162 | 49.0 | 18297 / 687 / 0 | 0 | 0 |
| uring-sqpoll | 90053 | 89997 | 1.00 | 1 | 0.00 | 0 | 41128801 | 0.0 | 0.0 | true | 0 | 0 | 29.9 | 253 / 2 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| recv-busy | 0 | 8198 | 2 |
| uring-sqpoll | 0 | 8199 | 1 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| recv-busy | 20171872 | 0 | 0 | 0 | 0.000 |
| uring-sqpoll | 20171872 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| recv-busy (87992 of 88000 in 1-msg reads) | 87992 | 3.17 | 3.02 | 4.18 | 5.50 | 6.80 | 27.07 | 34.31 |
| uring-sqpoll (87993 of 88000 in 1-msg reads) | 87993 | 4.00 | 3.80 | 5.16 | 6.55 | 8.98 | 18.79 | 20.24 |

### Paired by message, relative to recv-busy

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **uring-sqpoll Δuser (write-order corrected)** | 88000 | 0.51 | 0.66 | 2.16 | 11.35 | 28.20 | 33.47 | 112.17 |
| uring-sqpoll Δuser, even seq | 44000 | 2.72 | 2.32 | 4.60 | 13.52 | 30.27 | 35.57 | 113.82 |
| uring-sqpoll Δuser, odd seq | 44000 | -1.71 | -0.99 | 0.16 | 1.47 | 25.78 | 30.98 | 33.63 |
| uring-sqpoll Δrx | 88000 | -0.32 | -0.19 | 3.05 | 12.25 | 28.11 | 33.44 | 113.94 |
| uring-sqpoll Δstack (1-msg reads) | 87990 | 0.83 | 0.85 | 1.61 | 2.33 | 3.81 | 14.60 | 16.67 |

uring-sqpoll: 88000 paired messages; decoded before recv-busy on 43.6 % (uncorrected).

#### Stalls: md2/lab-recv-busy-vs-uring-sqpoll-single, loop 0 on CPU 2

171805312 iterations, longest 49.0 µs; over 5 / 20 / 100 µs: 18297 / 687 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 516 | 13779.0 | 39.9 |
| mostly unexplained (+softirq NET_RX) | 65 | 1747.4 | 37.2 |
| mostly unexplained (+preempted by kworker/2:1) | 55 | 1417.2 | 49.0 |
| softirq NET_RX | 47 | 1169.7 | 35.1 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 58.5 | 33.1 |
| loop work | 1 | 31.0 | 31.0 |
| mostly unexplained (+ipi_send_cpu) | 1 | 30.9 | 30.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 49.0 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 2 | 43.9 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 3 | 43.3 | 1 | mostly unexplained (+preempted by kworker/2:1) | preempted by kworker/2:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 39.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 37.3 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 6 | 37.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 37.2 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 8 | 36.9 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 36.8 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 36.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1001 |
| IWI IRQ work interrupts | 172 |
| LOC Local timer interrupts | 165 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

#### Stalls: md2/lab-recv-busy-vs-uring-sqpoll-single, loop 1 on CPU 3

3701592128 iterations, longest 29.9 µs; over 5 / 20 / 100 µs: 253 / 2 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 51.0 | 29.9 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 29.9 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 21.1 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 995 |
| IWI IRQ work interrupts | 3 |
| LOC Local timer interrupts | 2 |
| TLB TLB shootdowns | 1 |

## md2/lab-uring-sqpoll-vs-recv-busy-burst50: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=3743280 clocksource=tsc

### Loop counters (md2/lab-uring-sqpoll-vs-recv-busy-burst50)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 4502707 | 2378031 | 1.89 | 1 | 0.00 | 0 | 35338964 | 0.0 | 0.0 | true | 0 | 0 | 436.3 | 729 / 3 / 2 | 502707 | 0 |
| recv-busy | 4502736 | 2028301 | 2.22 | 161131840 | 35.79 | 1790354 | 1790354 | 98.7 | 66.4 | true | 0 | 2376 | 740.6 | 94781 / 7041 / 2 | 502736 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8196 | 1 |
| recv-busy | 0 | 8198 | 4 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 1008606368 | 0 | 0 | 0 | 0.000 |
| recv-busy | 1008612864 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (892760 of 3897250 in 1-msg reads) | 892760 | 3.16 | 3.01 | 3.63 | 6.24 | 10.63 | 14.04 | 32.98 |
| recv-busy (789799 of 3897250 in 1-msg reads) | 789799 | 2.23 | 2.08 | 2.53 | 4.56 | 9.73 | 17.86 | 33.41 |

### Paired by message, relative to uring-sqpoll

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **recv-busy Δuser (write-order corrected)** | 3897250 | 1.92 | 0.09 | 14.56 | 33.23 | 42.27 | 59.35 | 124.63 |
| recv-busy Δuser, even seq | 1948625 | 0.47 | -1.33 | 13.24 | 31.36 | 40.43 | 58.61 | 123.21 |
| recv-busy Δuser, odd seq | 1948625 | 3.37 | 1.51 | 15.90 | 35.08 | 44.05 | 60.45 | 123.15 |
| recv-busy Δrx | 3897250 | 2.83 | 1.33 | 15.25 | 34.17 | 43.36 | 59.01 | 123.65 |
| recv-busy Δstack (1-msg reads) | 414102 | -0.97 | -0.94 | -0.58 | 1.07 | 5.71 | 14.80 | 29.59 |

recv-busy: 3897250 paired messages; decoded before uring-sqpoll on 48.2 % (uncorrected).

#### Stalls: md2/lab-uring-sqpoll-vs-recv-busy-burst50, loop 0 on CPU 2

3180506752 iterations, longest 436.3 µs; over 5 / 20 / 100 µs: 729 / 3 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 1 | 436.3 | 436.3 |
| softirq TIMER | 1 | 368.9 | 368.9 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 1 | 38.7 | 38.7 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 436.3 | 2750 | loop work (2750 items) |  |
| 2 | 368.9 | 0 | softirq TIMER | softirq TIMER 347, local timer tick 10, timer tick_nohz_handler 4 |
| 3 | 38.7 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 994 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 4 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |
| TLB TLB shootdowns | 1 |

#### Stalls: md2/lab-uring-sqpoll-vs-recv-busy-burst50, loop 1 on CPU 3

161131840 iterations, longest 740.6 µs; over 5 / 20 / 100 µs: 94781 / 7041 / 2

| cause | stalls | total µs | max µs |
|---|---|---|---|
| loop work | 6720 | 163236.4 | 740.6 |
| unexplained: no kernel event (hypervisor or in-kernel time) | 172 | 3981.9 | 46.8 |
| mostly unexplained (+preempted by kworker/3:1) | 75 | 1734.2 | 34.1 |
| softirq NET_RX | 59 | 1384.1 | 51.7 |
| mostly unexplained (+softirq NET_RX) | 12 | 270.5 | 27.0 |
| softirq TIMER | 1 | 116.6 | 116.6 |
| mostly unexplained (+irq work (IPI/vector)) | 2 | 53.1 | 27.2 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 740.6 | 2748 | loop work (2748 items) |  |
| 2 | 116.6 | 0 | softirq TIMER | softirq TIMER 106, local timer tick 6, timer tick_nohz_handler 3, irq work (IPI/vector) 1 |
| 3 | 57.2 | 6 | loop work (6 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 4 | 52.2 | 6 | loop work (6 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 5 | 52.2 | 16 | loop work (16 items) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 6 | 51.7 | 0 | softirq NET_RX | softirq NET_RX 30, preempted by kworker/3:1 9, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 7 | 50.1 | 10 | loop work (10 items) |  |
| 8 | 49.8 | 5 | loop work (5 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 9 | 47.1 | 8 | loop work (8 items) | softirq NET_RX 16, preempted by kworker/3:1 2, irq work (IPI/vector) 1, workqueue [ena] 1 |
| 10 | 46.9 | 4 | loop work (4 items) | preempted by kworker/3:1 4, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| IWI IRQ work interrupts | 2387 |
| LOC Local timer interrupts | 2379 |
| CAL Function call interrupts | 1000 |
| TLB TLB shootdowns | 3 |
| MCP Machine check polls | 1 |
| RES Rescheduling interrupts | 1 |

## md2/lab-uring-sqpoll-vs-recv-busy-single: lab, 90 s (first 2 s dropped), 1 connection(s) per variant, kernel 7.0.0-1014-aws

busy_poll_us=1 wait_us=0 mlockall=true BusyPollRxPackets during run=156378 clocksource=tsc

### Loop counters (md2/lab-uring-sqpoll-vs-recv-busy-single)

| variant | messages | reads | msgs/read | syscalls | syscalls/msg | syscalls/s | spins/s | recv EAGAIN % | kernel CPU % | NIC polling set up | sleeps (vol. ctx sw) | preempted | longest iteration µs | iterations >5 / >20 / >100 µs | dropped | unparsed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| uring-sqpoll | 90054 | 90001 | 1.00 | 1 | 0.00 | 0 | 35262127 | 0.0 | 0.0 | true | 0 | 0 | 35.0 | 483 / 3 / 0 | 0 | 0 |
| recv-busy | 90054 | 90001 | 1.00 | 172192512 | 1912.10 | 1913250 | 1913250 | 99.9 | 64.9 | true | 0 | 171 | 67.1 | 25954 / 393 / 0 | 0 | 0 |

### NIC queues

The pair does not share an RX queue.

| variant | conn | NAPI id | last rx CPU |
|---|---|---|---|
| uring-sqpoll | 0 | 8197 | 1 |
| recv-busy | 0 | 8200 | 3 |

### Copies beyond the single kernel→buffer copy

| variant | bytes received | span-copy bytes | compaction bytes | fragment bytes | extra copied % |
|---|---|---|---|---|---|
| uring-sqpoll | 20172096 | 0 | 0 | 0 | 0.000 |
| recv-busy | 20172096 | 0 | 0 | 0 | 0.000 |

### Receive stack: kernel rx timestamp → message decoded (`user − rx_sw`), single-message reads only

The kernel stamps a read with its newest segment, so multi-message reads are excluded. Interrupt-driven and polled variants take the rx timestamp on different CPUs, so compare this only within a regime.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| uring-sqpoll (88000 of 88000 in 1-msg reads) | 88000 | 4.05 | 3.83 | 5.22 | 6.48 | 7.95 | 19.09 | 21.71 |
| recv-busy (88000 of 88000 in 1-msg reads) | 88000 | 3.43 | 3.16 | 4.81 | 6.23 | 7.82 | 26.63 | 35.50 |

### Paired by message, relative to uring-sqpoll

**Δuser** = decoded-time difference for the same message (negative: this variant decoded first). The replay server writes each message to both connections back to back, alternating which goes first; the corrected row removes that ±offset (half the gap between the even- and odd-sequence medians).
Diagnostics: Δrx = kernel-arrival difference; Δstack = Δuser − Δrx over messages that were alone in their read on both sides.

| | n | mean µs | p50 µs | p90 µs | p99 µs | p99.9 µs | p99.99 µs | max µs |
|---|---|---|---|---|---|---|---|---|
| **recv-busy Δuser (write-order corrected)** | 88000 | -0.55 | -0.53 | 1.44 | 3.99 | 47.65 | 57.72 | 71.73 |
| recv-busy Δuser, even seq | 44000 | -2.69 | -2.63 | -0.64 | 1.32 | 44.09 | 55.70 | 69.63 |
| recv-busy Δuser, odd seq | 44000 | 1.59 | 1.57 | 3.51 | 6.47 | 50.72 | 59.34 | 63.53 |
| recv-busy Δrx | 88000 | 0.07 | 0.05 | 4.02 | 6.75 | 48.84 | 59.64 | 70.04 |
| recv-busy Δstack (1-msg reads) | 88000 | -0.62 | -0.67 | 0.44 | 1.28 | 2.00 | 21.31 | 30.11 |

recv-busy: 88000 paired messages; decoded before uring-sqpoll on 55.2 % (uncorrected).

#### Stalls: md2/lab-uring-sqpoll-vs-recv-busy-single, loop 0 on CPU 2

3173591424 iterations, longest 35.0 µs; over 5 / 20 / 100 µs: 483 / 3 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 2 | 64.4 | 35.0 |
| local timer tick | 1 | 31.4 | 31.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 35.0 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 2 | 31.4 | 0 | local timer tick | local timer tick 16, timer tick_nohz_handler 8, softirq TIMER 4 |
| 3 | 29.4 | 0 | unexplained: no kernel event (hypervisor or in-kernel time) |  |

Interrupts on CPU 2 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 993 |
| IWI IRQ work interrupts | 4 |
| LOC Local timer interrupts | 3 |
| TLB TLB shootdowns | 1 |

#### Stalls: md2/lab-uring-sqpoll-vs-recv-busy-single, loop 1 on CPU 3

172192512 iterations, longest 67.1 µs; over 5 / 20 / 100 µs: 25954 / 393 / 0

| cause | stalls | total µs | max µs |
|---|---|---|---|
| unexplained: no kernel event (hypervisor or in-kernel time) | 230 | 11135.3 | 65.2 |
| mostly unexplained (+preempted by kworker/3:1) | 90 | 2280.9 | 67.1 |
| mostly unexplained (+softirq NET_RX) | 33 | 1542.0 | 64.5 |
| softirq NET_RX | 35 | 873.9 | 31.6 |
| mostly unexplained (+irq work (IPI/vector)) | 4 | 107.1 | 31.9 |
| loop work | 1 | 28.4 | 28.4 |

| longest | µs | work | cause | kernel time inside (µs) |
|---|---|---|---|---|
| 1 | 67.1 | 1 | mostly unexplained (+preempted by kworker/3:1) | preempted by kworker/3:1 3, irq work (IPI/vector) 1, workqueue [ena] 1, ipi_send_cpu 0 |
| 2 | 65.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 3 | 64.5 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 4 | 64.5 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 5 | 62.8 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |
| 6 | 62.6 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 7 | 62.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 8 | 62.0 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 9 | 61.2 | 1 | unexplained: no kernel event (hypervisor or in-kernel time) |  |
| 10 | 60.7 | 1 | mostly unexplained (+softirq NET_RX) | softirq NET_RX 1 |

Interrupts on CPU 3 during the run (`/proc/interrupts`):

| interrupt | count |
|---|---|
| CAL Function call interrupts | 1001 |
| IWI IRQ work interrupts | 190 |
| LOC Local timer interrupts | 175 |
| TLB TLB shootdowns | 3 |
| RES Rescheduling interrupts | 1 |

